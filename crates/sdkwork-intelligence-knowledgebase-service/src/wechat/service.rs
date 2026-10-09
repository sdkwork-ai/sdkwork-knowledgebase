use crate::ports::knowledge_drive_storage::KnowledgeDriveStorage;
use crate::wechat::api_client::{WechatApiClient, WechatApiClientError, WechatDraftArticle};
use crate::wechat::config_store::WechatConfigStore;
use base64::engine::general_purpose::STANDARD as BASE64_STANDARD;
use base64::Engine;
use sdkwork_knowledgebase_contract::wechat::{
    KnowledgeWechatApplet, KnowledgeWechatArticle, KnowledgeWechatArticlesPreviewRequest,
    KnowledgeWechatArticlesPublishRequest, KnowledgeWechatFanTag, KnowledgeWechatFanTagList,
    KnowledgeWechatOfficialAccount, KnowledgeWechatOperationResult,
};
use sdkwork_utils_rust::is_blank;
use std::sync::Arc;
use thiserror::Error;

const MAX_WECHAT_PUBLISH_ACCOUNTS: usize = 20;
const MAX_WECHAT_ARTICLES_PER_OPERATION: usize = 8;
const MAX_WECHAT_PREVIEW_RECIPIENTS: usize = 20;
const MAX_WECHAT_ARTICLE_CONTENT_BYTES: usize = 2 * 1024 * 1024;
/// WeChat article field limits enforced locally so payloads fail with typed errors
/// instead of an upstream errcode round trip.
const MAX_WECHAT_ARTICLE_TITLE_CHARS: usize = 64;
const MAX_WECHAT_ARTICLE_DIGEST_CHARS: usize = 120;
const MAX_WECHAT_ARTICLE_CONTENT_CHARS: usize = 20_000;
/// Cover images arrive embedded in the request as base64; this bounds the decoded size
/// accepted for a permanent image material upload.
const MAX_WECHAT_ARTICLE_COVER_BYTES: usize = 5 * 1024 * 1024;
const WECHAT_ARTICLE_COVER_DATA_URI_PREFIX: &str = "data:image/";

/// An article cover classified out of the request's string-valued `cover` field.
#[derive(Debug)]
enum CoverSource {
    /// Base64 image data embedded in the request; uploaded as a permanent material per
    /// account before the draft is created (media ids are app-scoped on WeChat).
    Embedded(EmbeddedCoverImage),
    /// An already-uploaded app-scoped WeChat media id, passed through to the draft.
    MediaId(String),
}

#[derive(Debug)]
struct EmbeddedCoverImage {
    file_name: &'static str,
    content_type: String,
    bytes: Vec<u8>,
}

/// One article prepared for the draft API: the client payload plus any cover bytes that
/// still need an upload. Built once per request, before any account loop runs.
struct PreparedDraftArticle {
    draft: WechatDraftArticle,
    embedded_cover: Option<EmbeddedCoverImage>,
}

pub struct KnowledgeWechatService<'a> {
    config_store: WechatConfigStore<'a>,
    api_client: Arc<WechatApiClient>,
}

impl<'a> KnowledgeWechatService<'a> {
    /// Callers must inject a long-lived shared client: the api client owns the 7,000 s access
    /// token cache, so a per-request instance would re-fetch a token for every operation.
    pub fn new(
        drive: &'a dyn KnowledgeDriveStorage,
        tenant_id: &str,
        api_client: Arc<WechatApiClient>,
    ) -> Self {
        Self {
            config_store: WechatConfigStore::new(drive, tenant_id),
            api_client,
        }
    }

    pub async fn list_official_accounts(
        &self,
    ) -> Result<Vec<KnowledgeWechatOfficialAccount>, KnowledgeWechatServiceError> {
        self.config_store
            .load_official_accounts()
            .await
            .map_err(KnowledgeWechatServiceError::Storage)
    }

    pub async fn replace_official_accounts(
        &self,
        accounts: Vec<KnowledgeWechatOfficialAccount>,
    ) -> Result<Vec<KnowledgeWechatOfficialAccount>, KnowledgeWechatServiceError> {
        self.config_store
            .replace_official_accounts(accounts)
            .await
            .map_err(KnowledgeWechatServiceError::Storage)
    }

    pub async fn list_applets(
        &self,
    ) -> Result<Vec<KnowledgeWechatApplet>, KnowledgeWechatServiceError> {
        self.config_store
            .load_applets()
            .await
            .map_err(KnowledgeWechatServiceError::Storage)
    }

    pub async fn replace_applets(
        &self,
        applets: Vec<KnowledgeWechatApplet>,
    ) -> Result<Vec<KnowledgeWechatApplet>, KnowledgeWechatServiceError> {
        self.config_store
            .replace_applets(applets)
            .await
            .map_err(KnowledgeWechatServiceError::Storage)
    }

    pub async fn list_fan_tags(
        &self,
        account_id: &str,
    ) -> Result<KnowledgeWechatFanTagList, KnowledgeWechatServiceError> {
        if is_blank(Some(account_id)) {
            return Err(KnowledgeWechatServiceError::InvalidRequest(
                "accountId is required".to_string(),
            ));
        }
        let access_token = self.resolve_account_access_token(account_id).await?;
        let tags = self.api_client.list_user_tags(&access_token).await?;
        Ok(KnowledgeWechatFanTagList {
            tags: tags
                .into_iter()
                .map(|tag| KnowledgeWechatFanTag {
                    id: tag.id.to_string(),
                    name: tag.name,
                    fan_count: tag.count,
                })
                .collect(),
        })
    }

    /// Publishes (or, with `sendNotification: false`, drafts) the articles for
    /// every configured official account: permanent-material cover upload
    /// (only for covers carrying embedded image data) and `draft/add`, then —
    /// for publishing — `freepublish/submit`. A draft-only request stops after
    /// `draft/add` so the articles land in the WeChat draft box without being
    /// pushed to followers; `sendNotification: false` is the documented
    /// draft-only trigger and must never behave as a silent full publish.
    /// Publication completes asynchronously upstream; the upstream `publish_id`
    /// cannot be carried in the operation result contract (`accepted`/`status`
    /// only), so it is logged for traceability.
    pub async fn publish_articles(
        &self,
        request: KnowledgeWechatArticlesPublishRequest,
    ) -> Result<KnowledgeWechatOperationResult, KnowledgeWechatServiceError> {
        validate_publish_request(&request)?;
        if !is_blank(request.schedule_time.as_deref()) {
            return Err(KnowledgeWechatServiceError::InvalidRequest(
                "scheduleTime is not supported; publish immediately or save drafts without scheduling"
                    .to_string(),
            ));
        }
        // Fail closed on fan-tag targeting: the draft/freepublish pipeline has
        // no per-tag delivery, so silently ignoring this flag would publish to
        // ALL followers while the caller believes they targeted one group.
        // (A non-targeted `selectedGroupId` is the "all followers" sentinel,
        // so it may not reject here.)
        if request.group_notification.unwrap_or(false) {
            return Err(KnowledgeWechatServiceError::InvalidRequest(
                "groupNotification (fan-tag targeted publish) is not supported; articles publish to all followers"
                    .to_string(),
            ));
        }
        let draft_only = request.send_notification == Some(false);
        validate_articles(&request.articles)?;
        let prepared = prepare_draft_articles(&request.articles)?;

        for account_id in &request.account_ids {
            let account = self
                .config_store
                .find_official_account(account_id)
                .await
                .map_err(KnowledgeWechatServiceError::Storage)?
                .ok_or_else(|| {
                    KnowledgeWechatServiceError::InvalidRequest(format!(
                        "official account was not found: {account_id}"
                    ))
                })?;
            let app_secret = account.app_secret.as_deref().ok_or_else(|| {
                KnowledgeWechatServiceError::InvalidRequest(format!(
                    "official account {account_id} is missing appSecret"
                ))
            })?;
            let draft_articles = self
                .upload_draft_covers(&account.app_id, app_secret, &prepared)
                .await?;
            let draft_media_id = self
                .api_client
                .add_draft(&account.app_id, app_secret, draft_articles)
                .await?;
            if draft_only {
                tracing::info!(
                    account_id = %account_id,
                    draft_media_id = %draft_media_id,
                    "wechat draft saved (sendNotification=false); no freepublish submitted"
                );
                continue;
            }
            let publish_id = self
                .api_client
                .submit_freepublish(&account.app_id, app_secret, &draft_media_id)
                .await?;
            tracing::info!(
                account_id = %account_id,
                draft_media_id = %draft_media_id,
                publish_id = %publish_id,
                "wechat articles publish submitted"
            );
        }

        Ok(KnowledgeWechatOperationResult {
            accepted: true,
            status: "accepted".to_string(),
        })
    }

    /// Dry run for the publish payload: resolves the configured account locally and
    /// enforces every limit publish would enforce, but makes no outbound WeChat calls.
    pub async fn preview_articles(
        &self,
        request: KnowledgeWechatArticlesPreviewRequest,
    ) -> Result<KnowledgeWechatOperationResult, KnowledgeWechatServiceError> {
        if is_blank(Some(request.account_id.as_str())) || request.wechat_ids.is_empty() {
            return Err(KnowledgeWechatServiceError::InvalidRequest(
                "accountId and wechatIds are required".to_string(),
            ));
        }
        if request.wechat_ids.len() > MAX_WECHAT_PREVIEW_RECIPIENTS
            || request
                .wechat_ids
                .iter()
                .any(|recipient| is_blank(Some(recipient.as_str())))
        {
            return Err(KnowledgeWechatServiceError::InvalidRequest(format!(
                "wechatIds must contain 1 to {MAX_WECHAT_PREVIEW_RECIPIENTS} non-empty values"
            )));
        }
        if request.articles.is_empty() {
            return Err(KnowledgeWechatServiceError::InvalidRequest(
                "at least one article is required".to_string(),
            ));
        }
        validate_articles(&request.articles)?;
        prepare_draft_articles(&request.articles)?;
        let account = self
            .config_store
            .find_official_account(&request.account_id)
            .await
            .map_err(KnowledgeWechatServiceError::Storage)?
            .ok_or_else(|| {
                KnowledgeWechatServiceError::InvalidRequest(format!(
                    "official account was not found: {}",
                    request.account_id
                ))
            })?;
        if account
            .app_secret
            .as_deref()
            .map_or(true, |secret| is_blank(Some(secret)))
        {
            return Err(KnowledgeWechatServiceError::InvalidRequest(format!(
                "official account {} is missing appSecret",
                request.account_id
            )));
        }
        Ok(KnowledgeWechatOperationResult {
            accepted: true,
            status: "validated".to_string(),
        })
    }

    /// Uploads each embedded cover image for `app_id` (WeChat media ids are app-scoped,
    /// so the same cover bytes are uploaded once per account) and assembles the final
    /// draft payload.
    async fn upload_draft_covers(
        &self,
        app_id: &str,
        app_secret: &str,
        prepared: &[PreparedDraftArticle],
    ) -> Result<Vec<WechatDraftArticle>, KnowledgeWechatServiceError> {
        let mut draft_articles = Vec::with_capacity(prepared.len());
        for article in prepared {
            let mut draft = article.draft.clone();
            if let Some(cover) = &article.embedded_cover {
                let material = self
                    .api_client
                    .upload_permanent_image(
                        app_id,
                        app_secret,
                        cover.file_name,
                        &cover.content_type,
                        &cover.bytes,
                    )
                    .await?;
                draft.thumb_media_id = Some(material.media_id);
            }
            draft_articles.push(draft);
        }
        Ok(draft_articles)
    }

    async fn resolve_account_access_token(
        &self,
        account_id: &str,
    ) -> Result<String, KnowledgeWechatServiceError> {
        let account = self
            .config_store
            .find_official_account(account_id)
            .await
            .map_err(KnowledgeWechatServiceError::Storage)?
            .ok_or_else(|| {
                KnowledgeWechatServiceError::InvalidRequest(format!(
                    "official account was not found: {account_id}"
                ))
            })?;
        let app_secret = account.app_secret.as_deref().ok_or_else(|| {
            KnowledgeWechatServiceError::InvalidRequest(format!(
                "official account {account_id} is missing appSecret"
            ))
        })?;
        self.api_client
            .fetch_access_token(&account.app_id, app_secret)
            .await
            .map_err(KnowledgeWechatServiceError::from)
    }
}

fn validate_articles(
    articles: &[sdkwork_knowledgebase_contract::wechat::KnowledgeWechatArticle],
) -> Result<(), KnowledgeWechatServiceError> {
    if articles.is_empty() || articles.len() > MAX_WECHAT_ARTICLES_PER_OPERATION {
        return Err(KnowledgeWechatServiceError::InvalidRequest(format!(
            "articles must contain 1 to {MAX_WECHAT_ARTICLES_PER_OPERATION} items"
        )));
    }
    for article in articles {
        let content = article.content.as_deref().unwrap_or_default();
        if is_blank(Some(article.title.as_str())) || is_blank(Some(content)) {
            return Err(KnowledgeWechatServiceError::InvalidRequest(
                "article title and content must not be empty".to_string(),
            ));
        }
        if content.len() > MAX_WECHAT_ARTICLE_CONTENT_BYTES {
            return Err(KnowledgeWechatServiceError::InvalidRequest(format!(
                "article content exceeds {MAX_WECHAT_ARTICLE_CONTENT_BYTES} bytes"
            )));
        }
        if article.title.chars().count() > MAX_WECHAT_ARTICLE_TITLE_CHARS {
            return Err(KnowledgeWechatServiceError::InvalidRequest(format!(
                "article title exceeds {MAX_WECHAT_ARTICLE_TITLE_CHARS} characters"
            )));
        }
        if content.chars().count() > MAX_WECHAT_ARTICLE_CONTENT_CHARS {
            return Err(KnowledgeWechatServiceError::InvalidRequest(format!(
                "article content exceeds {MAX_WECHAT_ARTICLE_CONTENT_CHARS} characters"
            )));
        }
        if let Some(digest) = article.r#abstract.as_deref() {
            if digest.chars().count() > MAX_WECHAT_ARTICLE_DIGEST_CHARS {
                return Err(KnowledgeWechatServiceError::InvalidRequest(format!(
                    "article digest exceeds {MAX_WECHAT_ARTICLE_DIGEST_CHARS} characters"
                )));
            }
        }
    }
    Ok(())
}

/// Validates each cover and assembles the draft payload that `draft/add` consumes.
/// Pure function: no storage or WeChat IO, so preview can run it as part of its dry run.
fn prepare_draft_articles(
    articles: &[KnowledgeWechatArticle],
) -> Result<Vec<PreparedDraftArticle>, KnowledgeWechatServiceError> {
    articles
        .iter()
        .map(|article| {
            let cover_source = classify_article_cover(article)?;
            let draft = WechatDraftArticle {
                title: article.title.clone(),
                author: article.author.clone(),
                digest: article
                    .r#abstract
                    .clone()
                    .filter(|digest| !is_blank(Some(digest))),
                content: article.content.clone().unwrap_or_default(),
                content_source_url: None,
                thumb_media_id: match &cover_source {
                    Some(CoverSource::MediaId(media_id)) => Some(media_id.clone()),
                    _ => None,
                },
            };
            Ok(PreparedDraftArticle {
                embedded_cover: match cover_source {
                    Some(CoverSource::Embedded(cover)) => Some(cover),
                    _ => None,
                },
                draft,
            })
        })
        .collect()
}

/// Classifies one article cover. The publish contract carries no binary media field, so
/// a cover is either base64 image data embedded in the request (uploaded per account as
/// a permanent material) or an already-uploaded app-scoped WeChat media id passed
/// through verbatim. Remote URLs are rejected: the server never fetches caller-supplied
/// cover links.
fn classify_article_cover(
    article: &KnowledgeWechatArticle,
) -> Result<Option<CoverSource>, KnowledgeWechatServiceError> {
    let Some(cover) = article
        .cover
        .as_deref()
        .filter(|value| !is_blank(Some(value)))
    else {
        return Ok(None);
    };
    if cover.starts_with("http://") || cover.starts_with("https://") {
        return Err(KnowledgeWechatServiceError::InvalidRequest(
            "article cover must be base64 image data (a data:image/...;base64,... URI) or an already uploaded WeChat media id; remote cover URLs are not fetched"
                .to_string(),
        ));
    }
    match cover.strip_prefix(WECHAT_ARTICLE_COVER_DATA_URI_PREFIX) {
        Some(payload) => parse_embedded_cover(payload).map(|cover| Some(CoverSource::Embedded(cover))),
        None => Ok(Some(CoverSource::MediaId(cover.to_string()))),
    }
}

fn parse_embedded_cover(
    payload: &str,
) -> Result<EmbeddedCoverImage, KnowledgeWechatServiceError> {
    let (subtype, encoded) = payload.split_once(";base64,").ok_or_else(|| {
        KnowledgeWechatServiceError::InvalidRequest(
            "article cover data URI must carry base64 image data (data:image/...;base64,...)"
                .to_string(),
        )
    })?;
    let content_type = format!("image/{subtype}");
    let file_name = cover_file_name(&content_type).ok_or_else(|| {
        KnowledgeWechatServiceError::InvalidRequest(
            "article cover image must be png, jpeg, gif, or bmp data".to_string(),
        )
    })?;
    let bytes = BASE64_STANDARD.decode(encoded.trim()).map_err(|_| {
        KnowledgeWechatServiceError::InvalidRequest(
            "article cover data URI payload is not valid base64".to_string(),
        )
    })?;
    if bytes.is_empty() {
        return Err(KnowledgeWechatServiceError::InvalidRequest(
            "article cover image data must not be empty".to_string(),
        ));
    }
    if bytes.len() > MAX_WECHAT_ARTICLE_COVER_BYTES {
        return Err(KnowledgeWechatServiceError::InvalidRequest(format!(
            "article cover image exceeds {MAX_WECHAT_ARTICLE_COVER_BYTES} bytes"
        )));
    }
    Ok(EmbeddedCoverImage {
        file_name,
        content_type,
        bytes,
    })
}

/// WeChat detects the material type from the file name extension, so the upload needs a
/// conventional name matching the declared content type.
fn cover_file_name(content_type: &str) -> Option<&'static str> {
    match content_type {
        "image/png" => Some("cover.png"),
        "image/jpeg" | "image/jpg" => Some("cover.jpg"),
        "image/gif" => Some("cover.gif"),
        "image/bmp" => Some("cover.bmp"),
        _ => None,
    }
}

fn validate_publish_request(
    request: &KnowledgeWechatArticlesPublishRequest,
) -> Result<(), KnowledgeWechatServiceError> {
    if request.account_ids.is_empty()
        || request.account_ids.len() > MAX_WECHAT_PUBLISH_ACCOUNTS
        || request
            .account_ids
            .iter()
            .any(|account_id| is_blank(Some(account_id.as_str())))
        || request.articles.is_empty()
    {
        return Err(KnowledgeWechatServiceError::InvalidRequest(
            format!(
                "accountIds must contain 1 to {MAX_WECHAT_PUBLISH_ACCOUNTS} non-empty values and articles are required"
            ),
        ));
    }
    Ok(())
}

#[derive(Debug, Error)]
pub enum KnowledgeWechatServiceError {
    #[error("invalid wechat request: {0}")]
    InvalidRequest(String),
    #[error("unsupported wechat operation: {0}")]
    UnsupportedOperation(&'static str),
    #[error(transparent)]
    Storage(#[from] crate::ports::knowledge_drive_storage::KnowledgeStorageError),
    #[error(transparent)]
    Api(#[from] WechatApiClientError),
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ports::knowledge_drive_storage::{
        HeadKnowledgeObjectRequest, KnowledgeObjectRef, KnowledgeStorageError,
        PutKnowledgeObjectRequest,
    };
    use serde_json::json;
    use std::collections::HashMap;
    use wiremock::matchers::{any, method, path};
    use wiremock::{Mock, MockServer, Request, ResponseTemplate};

    fn article() -> KnowledgeWechatArticle {
        KnowledgeWechatArticle {
            id: "article-1".to_string(),
            title: "Title".to_string(),
            author: "Author".to_string(),
            content: Some("<p>Body</p>".to_string()),
            cover: None,
            r#abstract: Some("Digest".to_string()),
        }
    }

    #[test]
    fn publish_capability_reports_the_canonical_unsupported_operation() {
        let error = KnowledgeWechatServiceError::UnsupportedOperation("wechat.articles.publish");

        assert_eq!(
            error.to_string(),
            "unsupported wechat operation: wechat.articles.publish"
        );
    }

    #[test]
    fn article_validation_bounds_article_count_and_content_bytes() {
        let too_many = vec![article(); MAX_WECHAT_ARTICLES_PER_OPERATION + 1];
        assert!(validate_articles(&too_many).is_err());

        let mut oversize = article();
        oversize.content = Some("x".repeat(MAX_WECHAT_ARTICLE_CONTENT_BYTES + 1));
        assert!(validate_articles(&[oversize]).is_err());
    }

    #[test]
    fn article_validation_enforces_wechat_field_limits() {
        let mut over_title = article();
        over_title.title = "题".repeat(MAX_WECHAT_ARTICLE_TITLE_CHARS + 1);
        assert!(validate_articles(&[over_title]).is_err());

        let mut over_digest = article();
        over_digest.r#abstract = Some("d".repeat(MAX_WECHAT_ARTICLE_DIGEST_CHARS + 1));
        assert!(validate_articles(&[over_digest]).is_err());

        let mut over_content = article();
        over_content.content = Some("x".repeat(MAX_WECHAT_ARTICLE_CONTENT_CHARS + 1));
        assert!(validate_articles(&[over_content]).is_err());
    }

    #[test]
    fn cover_classification_accepts_embedded_data_and_media_ids_only() {
        let mut embedded = article();
        embedded.cover = Some("data:image/png;base64,cG5nLWJ5dGVz".to_string());
        match classify_article_cover(&embedded).expect("embedded cover") {
            Some(CoverSource::Embedded(cover)) => {
                assert_eq!(cover.file_name, "cover.png");
                assert_eq!(cover.content_type, "image/png");
                assert_eq!(cover.bytes, b"png-bytes".to_vec());
            }
            other => panic!("expected embedded cover, got {other:?}"),
        }

        let mut media_id_cover = article();
        media_id_cover.cover = Some("MEDIA_ID_FROM_PRIOR_UPLOAD".to_string());
        assert!(matches!(
            classify_article_cover(&media_id_cover),
            Ok(Some(CoverSource::MediaId(_)))
        ));

        let blank_cover = article();
        assert!(matches!(classify_article_cover(&blank_cover), Ok(None)));

        let mut remote_url = article();
        remote_url.cover = Some("https://cdn.example.com/cover.png".to_string());
        assert!(matches!(
            classify_article_cover(&remote_url),
            Err(KnowledgeWechatServiceError::InvalidRequest(_))
        ));

        let mut broken_base64 = article();
        broken_base64.cover = Some("data:image/png;base64,!!!".to_string());
        assert!(matches!(
            classify_article_cover(&broken_base64),
            Err(KnowledgeWechatServiceError::InvalidRequest(_))
        ));

        let mut unsupported_type = article();
        unsupported_type.cover = Some("data:image/svg+xml;base64,c3Zn".to_string());
        assert!(matches!(
            classify_article_cover(&unsupported_type),
            Err(KnowledgeWechatServiceError::InvalidRequest(_))
        ));
    }

    /// Minimal in-memory Drive double so the wechat config store can round-trip the
    /// tenant config without a real storage backend.
    struct InMemoryWechatConfigDrive {
        objects: std::sync::Mutex<HashMap<String, Vec<u8>>>,
    }

    impl InMemoryWechatConfigDrive {
        fn new() -> Self {
            Self {
                objects: std::sync::Mutex::new(HashMap::new()),
            }
        }

        fn locked_objects(&self) -> std::sync::MutexGuard<'_, HashMap<String, Vec<u8>>> {
            self.objects
                .lock()
                .unwrap_or_else(|poisoned| poisoned.into_inner())
        }
    }

    #[async_trait::async_trait]
    impl KnowledgeDriveStorage for InMemoryWechatConfigDrive {
        async fn put_object(
            &self,
            request: PutKnowledgeObjectRequest,
        ) -> Result<KnowledgeObjectRef, KnowledgeStorageError> {
            let object_key = request.logical_path.clone();
            let object_role = request.object_role.clone();
            let content_type = request.content_type.clone();
            let size_bytes = request.body.len() as u64;
            self.locked_objects().insert(object_key.clone(), request.body);
            Ok(KnowledgeObjectRef {
                storage_provider_id: "in-memory".to_string(),
                bucket: "in-memory".to_string(),
                object_key: object_key.clone(),
                logical_path: object_key,
                object_role,
                content_type,
                size_bytes,
                checksum_sha256_hex: None,
                etag: None,
                version_id: None,
            })
        }

        async fn head_object(
            &self,
            request: HeadKnowledgeObjectRequest,
        ) -> Result<KnowledgeObjectRef, KnowledgeStorageError> {
            let size_bytes = self
                .locked_objects()
                .get(&request.object_key)
                .map(|body| body.len() as u64)
                .ok_or_else(|| {
                    KnowledgeStorageError::NotFound("test object is absent".to_string())
                })?;
            Ok(KnowledgeObjectRef {
                storage_provider_id: "in-memory".to_string(),
                bucket: "in-memory".to_string(),
                object_key: request.object_key.clone(),
                logical_path: request.object_key,
                object_role: request.object_role,
                content_type: "application/json; charset=utf-8".to_string(),
                size_bytes,
                checksum_sha256_hex: None,
                etag: None,
                version_id: None,
            })
        }

        async fn get_object_text(
            &self,
            object_ref: &KnowledgeObjectRef,
        ) -> Result<String, KnowledgeStorageError> {
            let objects = self.locked_objects();
            let body = objects
                .get(&object_ref.object_key)
                .ok_or_else(|| {
                    KnowledgeStorageError::NotFound("test object is absent".to_string())
                })?;
            String::from_utf8(body.clone())
                .map_err(|error| KnowledgeStorageError::Internal(error.to_string()))
        }
    }

    fn configured_account(id: &str) -> KnowledgeWechatOfficialAccount {
        KnowledgeWechatOfficialAccount {
            id: id.to_string(),
            name: "Account".to_string(),
            account_type: "subscription".to_string(),
            avatar: "OA".to_string(),
            description: None,
            app_id: format!("wx-{id}"),
            app_secret: Some("account-secret".to_string()),
            server_url: None,
            token: None,
            encoding_aes_key: None,
            encrypt_mode: Some("safe".to_string()),
            domain_verify_file_name: None,
            domain_verify_file_content: None,
            js_secure_domains: None,
            web_auth_domains: None,
            business_domains: None,
            group: None,
        }
    }

    fn publish_request(articles: Vec<KnowledgeWechatArticle>) -> KnowledgeWechatArticlesPublishRequest {
        KnowledgeWechatArticlesPublishRequest {
            account_ids: vec!["acct-1".to_string()],
            articles,
            send_notification: None,
            group_notification: None,
            selected_group_id: None,
            schedule_time: None,
        }
    }

    fn preview_request(articles: Vec<KnowledgeWechatArticle>) -> KnowledgeWechatArticlesPreviewRequest {
        KnowledgeWechatArticlesPreviewRequest {
            account_id: "acct-1".to_string(),
            wechat_ids: vec!["openid-1".to_string()],
            articles,
        }
    }

    fn mock_api_client(server_uri: &str) -> WechatApiClient {
        WechatApiClient::with_api_base_override(server_uri.parse().expect("mock api base url"))
    }

    async fn seeded_service<'a>(
        drive: &'a InMemoryWechatConfigDrive,
        api_client: Arc<WechatApiClient>,
    ) -> KnowledgeWechatService<'a> {
        let service = KnowledgeWechatService::new(drive, "tenant-1", api_client);
        service
            .replace_official_accounts(vec![configured_account("acct-1")])
            .await
            .expect("seed official account config");
        service
    }

    async fn mount_token_endpoint(server: &MockServer, expected_calls: u64) {
        Mock::given(method("GET"))
            .and(path("/cgi-bin/token"))
            .respond_with(ResponseTemplate::new(200).set_body_json(json!({
                "access_token": "token-1",
                "expires_in": 7200
            })))
            .expect(expected_calls)
            .mount(server)
            .await;
    }

    #[tokio::test]
    async fn preview_validates_payload_and_makes_no_outbound_calls() {
        let _cipher_guard =
            crate::wechat::secret_cipher::test_support::TestEncryptionKeyGuard::with_key(
                "service-preview-test-key",
            );
        let server = MockServer::start().await;
        Mock::given(any())
            .respond_with(ResponseTemplate::new(200))
            .expect(0)
            .mount(&server)
            .await;
        let drive = InMemoryWechatConfigDrive::new();
        let service = seeded_service(&drive, Arc::new(mock_api_client(&server.uri()))).await;

        let result = service
            .preview_articles(preview_request(vec![article()]))
            .await
            .expect("preview dry run");

        assert!(result.accepted);
        assert_eq!(result.status, "validated");
    }

    #[tokio::test]
    async fn preview_rejects_account_without_app_secret_without_outbound_calls() {
        let _cipher_guard =
            crate::wechat::secret_cipher::test_support::TestEncryptionKeyGuard::with_key(
                "service-preview-no-secret-test-key",
            );
        let server = MockServer::start().await;
        Mock::given(any())
            .respond_with(ResponseTemplate::new(200))
            .expect(0)
            .mount(&server)
            .await;
        let drive = InMemoryWechatConfigDrive::new();
        let service = KnowledgeWechatService::new(
            &drive,
            "tenant-1",
            Arc::new(mock_api_client(&server.uri())),
        );
        let mut account = configured_account("acct-1");
        account.app_secret = None;
        service
            .replace_official_accounts(vec![account])
            .await
            .expect("seed official account config");

        let error = service
            .preview_articles(preview_request(vec![article()]))
            .await
            .expect_err("preview must reject an account without appSecret");

        assert!(matches!(
            error,
            KnowledgeWechatServiceError::InvalidRequest(_)
        ));
    }

    #[tokio::test]
    async fn publish_maps_upstream_errcode_to_safe_typed_error() {
        let _cipher_guard =
            crate::wechat::secret_cipher::test_support::TestEncryptionKeyGuard::with_key(
                "service-publish-errcode-test-key",
            );
        let server = MockServer::start().await;
        mount_token_endpoint(&server, 1).await;
        Mock::given(method("POST"))
            .and(path("/cgi-bin/draft/add"))
            .respond_with(|_request: &Request| {
                ResponseTemplate::new(200).set_body_json(json!({
                    "errcode": 53401,
                    "errmsg": "secret upstream detail text"
                }))
            })
            .mount(&server)
            .await;
        let drive = InMemoryWechatConfigDrive::new();
        let service = seeded_service(&drive, Arc::new(mock_api_client(&server.uri()))).await;

        let error = service
            .publish_articles(publish_request(vec![article()]))
            .await
            .expect_err("publish must fail on upstream errcode");

        match error {
            KnowledgeWechatServiceError::Api(WechatApiClientError::Api(detail)) => {
                assert!(detail.contains("53401"), "detail: {detail}");
                assert!(
                    !detail.contains("secret upstream detail"),
                    "raw upstream text leaked: {detail}"
                );
            }
            other => panic!("expected Api error, got {other:?}"),
        }
    }

    #[tokio::test]
    async fn publish_uploads_embedded_cover_then_creates_draft_and_submits() {
        let _cipher_guard =
            crate::wechat::secret_cipher::test_support::TestEncryptionKeyGuard::with_key(
                "service-publish-happy-path-test-key",
            );
        let server = MockServer::start().await;
        mount_token_endpoint(&server, 1).await;

        let upload_payloads: Arc<std::sync::Mutex<Vec<String>>> =
            Arc::new(std::sync::Mutex::new(Vec::new()));
        let upload_capture = Arc::clone(&upload_payloads);
        Mock::given(method("POST"))
            .and(path("/cgi-bin/material/add_material"))
            .respond_with(move |request: &Request| {
                if let Ok(mut payloads) = upload_capture.lock() {
                    payloads.push(String::from_utf8_lossy(&request.body).to_string());
                }
                ResponseTemplate::new(200).set_body_json(json!({
                    "media_id": "MEDIA_ID_1",
                    "url": "https://mmbiz.example.com/cover.png"
                }))
            })
            .expect(1)
            .mount(&server)
            .await;

        let draft_payloads: Arc<std::sync::Mutex<Vec<serde_json::Value>>> =
            Arc::new(std::sync::Mutex::new(Vec::new()));
        let draft_capture = Arc::clone(&draft_payloads);
        Mock::given(method("POST"))
            .and(path("/cgi-bin/draft/add"))
            .respond_with(move |request: &Request| {
                if let Ok(mut payloads) = draft_capture.lock() {
                    payloads.push(
                        serde_json::from_slice(&request.body).unwrap_or(serde_json::Value::Null),
                    );
                }
                ResponseTemplate::new(200).set_body_json(json!({ "media_id": "DRAFT_ID_1" }))
            })
            .expect(1)
            .mount(&server)
            .await;

        Mock::given(method("POST"))
            .and(path("/cgi-bin/freepublish/submit"))
            .respond_with(|request: &Request| {
                let payload: serde_json::Value =
                    serde_json::from_slice(&request.body).unwrap_or(serde_json::Value::Null);
                assert_eq!(payload["media_id"], "DRAFT_ID_1");
                ResponseTemplate::new(200).set_body_json(json!({
                    "errcode": 0,
                    "errmsg": "ok",
                    "publish_id": "PUBLISH_ID_1"
                }))
            })
            .expect(1)
            .mount(&server)
            .await;

        let drive = InMemoryWechatConfigDrive::new();
        let service = seeded_service(&drive, Arc::new(mock_api_client(&server.uri()))).await;
        let mut covered_article = article();
        covered_article.cover = Some("data:image/png;base64,cG5nLWJ5dGVz".to_string());

        let result = service
            .publish_articles(publish_request(vec![covered_article]))
            .await
            .expect("publish happy path");

        assert!(result.accepted);
        assert_eq!(result.status, "accepted");

        let uploads = upload_payloads.lock().expect("upload capture lock");
        assert_eq!(uploads.len(), 1);
        assert!(uploads[0].contains("filename=\"cover.png\""));
        assert!(uploads[0].contains("png-bytes"));

        let drafts = draft_payloads.lock().expect("draft capture lock");
        assert_eq!(drafts.len(), 1);
        assert_eq!(drafts[0]["articles"][0]["thumb_media_id"], "MEDIA_ID_1");
        assert_eq!(drafts[0]["articles"][0]["title"], "Title");
    }

    #[tokio::test]
    async fn publish_rejects_account_without_app_secret_before_upstream_io() {
        let _cipher_guard =
            crate::wechat::secret_cipher::test_support::TestEncryptionKeyGuard::with_key(
                "service-publish-no-secret-test-key",
            );
        let server = MockServer::start().await;
        Mock::given(any())
            .respond_with(ResponseTemplate::new(200))
            .expect(0)
            .mount(&server)
            .await;
        let drive = InMemoryWechatConfigDrive::new();
        let service = KnowledgeWechatService::new(
            &drive,
            "tenant-1",
            Arc::new(mock_api_client(&server.uri())),
        );
        let mut account = configured_account("acct-1");
        account.app_secret = None;
        service
            .replace_official_accounts(vec![account])
            .await
            .expect("seed official account config");

        let error = service
            .publish_articles(publish_request(vec![article()]))
            .await
            .expect_err("publish must reject an account without appSecret");

        assert!(matches!(
            error,
            KnowledgeWechatServiceError::InvalidRequest(_)
        ));
    }

    #[tokio::test]
    async fn publish_supports_media_id_cover_without_material_upload() {
        let _cipher_guard =
            crate::wechat::secret_cipher::test_support::TestEncryptionKeyGuard::with_key(
                "service-publish-media-id-cover-test-key",
            );
        let server = MockServer::start().await;
        mount_token_endpoint(&server, 1).await;
        Mock::given(method("POST"))
            .and(path("/cgi-bin/material/add_material"))
            .respond_with(|_request: &Request| {
                panic!("a pass-through media id cover must not trigger a material upload")
            })
            .mount(&server)
            .await;
        Mock::given(method("POST"))
            .and(path("/cgi-bin/draft/add"))
            .respond_with(|request: &Request| {
                let payload: serde_json::Value =
                    serde_json::from_slice(&request.body).unwrap_or(serde_json::Value::Null);
                assert_eq!(payload["articles"][0]["thumb_media_id"], "PRIOR_MEDIA_ID");
                ResponseTemplate::new(200).set_body_json(json!({ "media_id": "DRAFT_ID_1" }))
            })
            .expect(1)
            .mount(&server)
            .await;
        Mock::given(method("POST"))
            .and(path("/cgi-bin/freepublish/submit"))
            .respond_with(ResponseTemplate::new(200).set_body_json(json!({
                "errcode": 0,
                "publish_id": "PUBLISH_ID_1"
            })))
            .expect(1)
            .mount(&server)
            .await;

        let drive = InMemoryWechatConfigDrive::new();
        let service = seeded_service(&drive, Arc::new(mock_api_client(&server.uri()))).await;
        let mut covered_article = article();
        covered_article.cover = Some("PRIOR_MEDIA_ID".to_string());

        let result = service
            .publish_articles(publish_request(vec![covered_article]))
            .await
            .expect("publish with pass-through media id cover");

        assert!(result.accepted);
        assert_eq!(result.status, "accepted");
    }
}
