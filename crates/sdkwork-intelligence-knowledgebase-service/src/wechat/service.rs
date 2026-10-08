use crate::ports::knowledge_drive_storage::KnowledgeDriveStorage;
use crate::wechat::api_client::{WechatApiClient, WechatApiClientError};
use crate::wechat::config_store::WechatConfigStore;
use sdkwork_knowledgebase_contract::wechat::{
    KnowledgeWechatApplet, KnowledgeWechatArticlesPreviewRequest,
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

        let group_notification = request.group_notification.unwrap_or(false);
        resolve_fan_tag_id(request.selected_group_id.as_deref(), group_notification)?;
        validate_articles(&request.articles)?;
        Err(KnowledgeWechatServiceError::UnsupportedOperation(
            "wechat.articles.publish",
        ))
    }

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
        Err(KnowledgeWechatServiceError::UnsupportedOperation(
            "wechat.articles.preview",
        ))
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
    }
    Ok(())
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

fn resolve_fan_tag_id(
    selected_group_id: Option<&str>,
    group_notification: bool,
) -> Result<Option<i64>, KnowledgeWechatServiceError> {
    if !group_notification {
        return Ok(None);
    }
    let Some(group_id) = selected_group_id.filter(|value| !is_blank(Some(value))) else {
        return Err(KnowledgeWechatServiceError::InvalidRequest(
            "selectedGroupId is required when groupNotification is enabled".to_string(),
        ));
    };
    if group_id.eq_ignore_ascii_case("all") {
        return Ok(None);
    }
    group_id.parse::<i64>().map(Some).map_err(|_| {
        KnowledgeWechatServiceError::InvalidRequest(format!(
            "selectedGroupId must be 'all' or a numeric WeChat tag id, got {group_id}"
        ))
    })
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
    use sdkwork_knowledgebase_contract::wechat::KnowledgeWechatArticle;

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
}
