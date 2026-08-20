use async_trait::async_trait;
use axum::{
    body::{to_bytes, Body},
    http::{header, Request, StatusCode},
    response::Response,
};
use sdkwork_knowledgebase_contract::{
    ChangeKnowledgeWikiSourceFileVisibilityRequest, KnowledgeWikiIndexState,
    KnowledgeWikiPagePublicationState, KnowledgeWikiPublication, KnowledgeWikiPublicationMode,
    KnowledgeWikiPublicationStatus, KnowledgeWikiPublicationVersionCommandRequest,
    KnowledgeWikiSourceFile, KnowledgeWikiSourceFileCommandResult, KnowledgeWikiSourceFileKind,
    KnowledgeWikiSourceFileVersionCommandRequest, KnowledgeWikiSourceState,
    KnowledgeWikiUpdatePolicy, KnowledgeWikiVisibility, PublishKnowledgeWikiSourceFileRequest,
};
use sdkwork_routes_knowledgebase_app_api::{
    app_route_manifest, build_router_with_app_api, ApiError, ApiResult, KnowledgeAppApi,
    KnowledgeAppRequestContext,
};
use sdkwork_web_core::RateLimitTier;
use serde_json::{json, Value};
use std::sync::{Arc, Mutex};
use tower::ServiceExt;

const TENANT_ID: u64 = 100_001;
const ORGANIZATION_ID: u64 = 200_001;
const ACTOR_ID: u64 = 300_001;
const SPACE_ID: u64 = 7;
const SOURCE_FILE_UUID: &str = "source-file-001";

fn request_context() -> KnowledgeAppRequestContext {
    KnowledgeAppRequestContext {
        tenant_id: TENANT_ID,
        actor_id: Some(ACTOR_ID),
        organization_id: Some(ORGANIZATION_ID),
        session_id: Some("session-wiki-routes".to_string()),
        request_id: "request-wiki-routes".to_string(),
        trace_id: Some("trace-wiki-routes".to_string()),
        idempotency_key: Some("wiki-command-001".to_string()),
    }
}

#[tokio::test]
async fn wiki_routes_parse_string_versions_and_forward_authenticated_context() {
    let api = RecordingWikiApi::default();
    let app = build_router_with_app_api(api.clone());

    let cases = [
        (
            "GET",
            format!("/app/v3/api/knowledge/spaces/{SPACE_ID}/wiki_publication"),
            None,
            "retrieve",
        ),
        (
            "POST",
            format!(
                "/app/v3/api/knowledge/spaces/{SPACE_ID}/wiki_publication/activate"
            ),
            Some(json!({"expectedVersion": "11"})),
            "activate",
        ),
        (
            "POST",
            format!("/app/v3/api/knowledge/spaces/{SPACE_ID}/wiki_publication/pause"),
            Some(json!({"expectedVersion": "12"})),
            "pause",
        ),
        (
            "POST",
            format!(
                "/app/v3/api/knowledge/spaces/{SPACE_ID}/wiki_source_files/{SOURCE_FILE_UUID}/publish"
            ),
            Some(json!({
                "visibility": "public",
                "expectedPublicationVersion": "13",
                "expectedPageVersion": "21"
            })),
            "publish",
        ),
        (
            "POST",
            format!(
                "/app/v3/api/knowledge/spaces/{SPACE_ID}/wiki_source_files/{SOURCE_FILE_UUID}/unpublish"
            ),
            Some(json!({
                "expectedPublicationVersion": "14",
                "expectedPageVersion": "22"
            })),
            "unpublish",
        ),
        (
            "PATCH",
            format!(
                "/app/v3/api/knowledge/spaces/{SPACE_ID}/wiki_source_files/{SOURCE_FILE_UUID}/visibility"
            ),
            Some(json!({
                "visibility": "unlisted",
                "expectedPublicationVersion": "15",
                "expectedPageVersion": "23"
            })),
            "visibility",
        ),
    ];

    for (method, uri, body, expected_operation) in cases {
        let response = send(app.clone(), method, &uri, body, request_context()).await;
        assert_eq!(response.status(), StatusCode::OK, "{expected_operation}");
        let payload = response_json(response).await;
        assert_eq!(payload["code"], 0, "{expected_operation}");
        let publication = payload["data"]["item"]
            .get("publication")
            .unwrap_or(&payload["data"]["item"]);
        assert_eq!(publication["spaceId"], "7", "{expected_operation}");
        assert_eq!(publication["version"], "16", "{expected_operation}");
    }

    let calls = api.calls();
    assert_eq!(calls.len(), 6);
    assert!(calls.iter().all(|call| call.context == request_context()));
    assert!(calls.iter().all(|call| call.space_id == SPACE_ID));
    assert_eq!(calls[0].operation, "retrieve");
    assert_eq!(calls[1].request, json!({"expectedVersion": "11"}));
    assert_eq!(calls[2].request, json!({"expectedVersion": "12"}));
    assert_eq!(calls[3].source_file_uuid.as_deref(), Some(SOURCE_FILE_UUID));
    assert_eq!(calls[3].request["expectedPublicationVersion"], "13");
    assert_eq!(calls[3].request["expectedPageVersion"], "21");
    assert_eq!(calls[4].request["expectedPublicationVersion"], "14");
    assert_eq!(calls[4].request["expectedPageVersion"], "22");
    assert_eq!(calls[5].request["visibility"], "unlisted");
    assert_eq!(calls[5].request["expectedPublicationVersion"], "15");
    assert_eq!(calls[5].request["expectedPageVersion"], "23");
}

#[tokio::test]
async fn wiki_mutation_conflicts_use_problem_details_and_http_409() {
    let app = build_router_with_app_api(RecordingWikiApi::with_pause_conflict());
    let response = send(
        app,
        "POST",
        &format!("/app/v3/api/knowledge/spaces/{SPACE_ID}/wiki_publication/pause"),
        Some(json!({"expectedVersion": "9"})),
        request_context(),
    )
    .await;

    assert_eq!(response.status(), StatusCode::CONFLICT);
    assert_eq!(
        response.headers().get(header::CONTENT_TYPE).unwrap(),
        "application/problem+json"
    );
    let problem = response_json(response).await;
    assert_eq!(problem["status"], 409);
    assert_eq!(problem["code"], 40901);
}

#[tokio::test]
async fn wiki_routes_preserve_tenant_and_organization_denial() {
    let app = build_router_with_app_api(RecordingWikiApi::default());
    let mut context = request_context();
    context.organization_id = Some(ORGANIZATION_ID + 1);

    let response = send(
        app,
        "GET",
        &format!("/app/v3/api/knowledge/spaces/{SPACE_ID}/wiki_publication"),
        None,
        context,
    )
    .await;

    assert_eq!(response.status(), StatusCode::FORBIDDEN);
    let problem = response_json(response).await;
    assert_eq!(problem["status"], 403);
    assert_eq!(problem["code"], 40301);
}

#[test]
fn wiki_route_manifest_declares_permissions_rate_limits_and_idempotency() {
    let manifest = app_route_manifest();
    let routes = [
        (
            "GET",
            "/app/v3/api/knowledge/spaces/7/wiki_publication",
            "wikiPublications.retrieve",
            "knowledge.spaces.read",
            false,
        ),
        (
            "POST",
            "/app/v3/api/knowledge/spaces/7/wiki_publication/activate",
            "wikiPublications.activate",
            "knowledge.spaces.write",
            true,
        ),
        (
            "POST",
            "/app/v3/api/knowledge/spaces/7/wiki_publication/pause",
            "wikiPublications.pause",
            "knowledge.spaces.write",
            true,
        ),
        (
            "POST",
            "/app/v3/api/knowledge/spaces/7/wiki_source_files/source-file-001/publish",
            "wikiSourceFiles.publish",
            "knowledge.spaces.write",
            true,
        ),
        (
            "POST",
            "/app/v3/api/knowledge/spaces/7/wiki_source_files/source-file-001/unpublish",
            "wikiSourceFiles.unpublish",
            "knowledge.spaces.write",
            true,
        ),
        (
            "PATCH",
            "/app/v3/api/knowledge/spaces/7/wiki_source_files/source-file-001/visibility",
            "wikiSourceFiles.visibility.update",
            "knowledge.spaces.write",
            true,
        ),
    ];

    for (method, path, operation_id, permission, mutation) in routes {
        let route = manifest.match_route(method, path).expect(operation_id);
        assert_eq!(route.operation_id, operation_id);
        assert_eq!(route.required_permission, Some(permission));
        assert_eq!(route.idempotent, mutation);
        assert_eq!(
            route.rate_limit_tier,
            mutation.then_some(RateLimitTier::AuthCritical)
        );
    }
}

async fn send(
    app: axum::Router,
    method: &str,
    uri: &str,
    body: Option<Value>,
    context: KnowledgeAppRequestContext,
) -> Response {
    let mut request = Request::builder()
        .method(method)
        .uri(uri)
        .extension(context);
    let body = if let Some(body) = body {
        request = request.header(header::CONTENT_TYPE, "application/json");
        Body::from(serde_json::to_vec(&body).unwrap())
    } else {
        Body::empty()
    };
    app.oneshot(request.body(body).unwrap()).await.unwrap()
}

async fn response_json(response: Response) -> Value {
    let body = to_bytes(response.into_body(), usize::MAX).await.unwrap();
    serde_json::from_slice(&body).unwrap()
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct RecordedCall {
    operation: &'static str,
    context: KnowledgeAppRequestContext,
    space_id: u64,
    source_file_uuid: Option<String>,
    request: Value,
}

#[derive(Clone, Default)]
struct RecordingWikiApi {
    calls: Arc<Mutex<Vec<RecordedCall>>>,
    pause_conflict: bool,
}

impl RecordingWikiApi {
    fn with_pause_conflict() -> Self {
        Self {
            pause_conflict: true,
            ..Self::default()
        }
    }

    fn calls(&self) -> Vec<RecordedCall> {
        self.calls.lock().unwrap().clone()
    }

    fn record<T: serde::Serialize>(
        &self,
        operation: &'static str,
        context: KnowledgeAppRequestContext,
        space_id: u64,
        source_file_uuid: Option<String>,
        request: T,
    ) {
        self.calls.lock().unwrap().push(RecordedCall {
            operation,
            context,
            space_id,
            source_file_uuid,
            request: serde_json::to_value(request).unwrap(),
        });
    }

    fn validate_scope(context: &KnowledgeAppRequestContext) -> ApiResult<()> {
        if context.tenant_id != TENANT_ID || context.organization_id != Some(ORGANIZATION_ID) {
            return Err(ApiError::forbidden(
                "knowledge_space_access_denied",
                "knowledge space is outside the authenticated tenant or organization",
            ));
        }
        Ok(())
    }
}

#[async_trait]
impl KnowledgeAppApi for RecordingWikiApi {
    async fn retrieve_wiki_publication(
        &self,
        context: KnowledgeAppRequestContext,
        space_id: u64,
    ) -> ApiResult<KnowledgeWikiPublication> {
        Self::validate_scope(&context)?;
        self.record("retrieve", context, space_id, None, json!({}));
        Ok(publication())
    }

    async fn activate_wiki_publication(
        &self,
        context: KnowledgeAppRequestContext,
        space_id: u64,
        request: KnowledgeWikiPublicationVersionCommandRequest,
    ) -> ApiResult<KnowledgeWikiPublication> {
        Self::validate_scope(&context)?;
        self.record("activate", context, space_id, None, request);
        Ok(publication())
    }

    async fn pause_wiki_publication(
        &self,
        context: KnowledgeAppRequestContext,
        space_id: u64,
        request: KnowledgeWikiPublicationVersionCommandRequest,
    ) -> ApiResult<KnowledgeWikiPublication> {
        Self::validate_scope(&context)?;
        self.record("pause", context, space_id, None, request);
        if self.pause_conflict {
            return Err(ApiError::conflict(
                "wiki_publication_version_conflict",
                "Wiki publication state changed; refresh and retry",
            ));
        }
        Ok(publication())
    }

    async fn publish_wiki_source_file(
        &self,
        context: KnowledgeAppRequestContext,
        space_id: u64,
        source_file_uuid: String,
        request: PublishKnowledgeWikiSourceFileRequest,
    ) -> ApiResult<KnowledgeWikiSourceFileCommandResult> {
        Self::validate_scope(&context)?;
        self.record(
            "publish",
            context,
            space_id,
            Some(source_file_uuid),
            request,
        );
        Ok(page_result())
    }

    async fn unpublish_wiki_source_file(
        &self,
        context: KnowledgeAppRequestContext,
        space_id: u64,
        source_file_uuid: String,
        request: KnowledgeWikiSourceFileVersionCommandRequest,
    ) -> ApiResult<KnowledgeWikiSourceFileCommandResult> {
        Self::validate_scope(&context)?;
        self.record(
            "unpublish",
            context,
            space_id,
            Some(source_file_uuid),
            request,
        );
        Ok(page_result())
    }

    async fn change_wiki_source_file_visibility(
        &self,
        context: KnowledgeAppRequestContext,
        space_id: u64,
        source_file_uuid: String,
        request: ChangeKnowledgeWikiSourceFileVisibilityRequest,
    ) -> ApiResult<KnowledgeWikiSourceFileCommandResult> {
        Self::validate_scope(&context)?;
        self.record(
            "visibility",
            context,
            space_id,
            Some(source_file_uuid),
            request,
        );
        Ok(page_result())
    }
}

fn publication() -> KnowledgeWikiPublication {
    KnowledgeWikiPublication {
        uuid: "wiki-publication-001".to_string(),
        space_id: SPACE_ID,
        drive_space_uuid: "drive-space-001".to_string(),
        source_root_node_uuid: Some("sources-raw-001".to_string()),
        status: KnowledgeWikiPublicationStatus::Active,
        title: "Product Wiki".to_string(),
        homepage_source_path: "index.md".to_string(),
        publication_mode: KnowledgeWikiPublicationMode::ReviewRequired,
        default_visibility: KnowledgeWikiVisibility::Public,
        update_policy: KnowledgeWikiUpdatePolicy::KeepLastPublicUntilReady,
        provider_generation: 5,
        navigation_generation: 6,
        search_generation: 7,
        last_projected_drive_checkpoint: 8,
        version: 16,
    }
}

fn page_result() -> KnowledgeWikiSourceFileCommandResult {
    KnowledgeWikiSourceFileCommandResult {
        publication: publication(),
        source_file: KnowledgeWikiSourceFile {
            uuid: SOURCE_FILE_UUID.to_string(),
            drive_node_uuid: "drive-node-001".to_string(),
            drive_version_uuid: "drive-version-002".to_string(),
            source_path: "guides/getting-started.md".to_string(),
            canonical_route: Some("/guides/getting-started".to_string()),
            file_kind: KnowledgeWikiSourceFileKind::Page,
            media_type: "text/markdown".to_string(),
            size_bytes: 1024,
            content_sha256: "0123456789abcdef".to_string(),
            source_state: KnowledgeWikiSourceState::Ready,
            publication_state: KnowledgeWikiPagePublicationState::Published,
            visibility: KnowledgeWikiVisibility::Public,
            index_state: KnowledgeWikiIndexState::Ready,
            public_drive_version_uuid: Some("drive-version-002".to_string()),
            page_public_version: 3,
            version: 24,
        },
    }
}
