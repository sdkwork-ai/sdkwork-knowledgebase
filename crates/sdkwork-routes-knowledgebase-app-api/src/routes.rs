use axum::{
    extract::{
        rejection::JsonRejection, FromRequestParts, OriginalUri, Path, Query, State,
    },
    http::{request::Parts, StatusCode},
    response::Response,
    routing::{delete, get, patch, post, put},
    Json, Router,
};
use serde::de::DeserializeOwned;
use sdkwork_knowledgebase_contract::{
    context_binding::{
        CreateKnowledgeSpaceContextBindingRequest, UpdateKnowledgeSpaceContextBindingRequest,
    },
    ChangeKnowledgeWikiSourceFileVisibilityRequest, CreateKnowledgeDocumentRequest,
    CreateKnowledgeDocumentVersionRequest, CreateKnowledgeSpaceRequest,
    GrantKnowledgeSpaceMemberRequest, KnowledgeAgentBindingRequest, KnowledgeAgentChatRequest,
    KnowledgeAgentProfileRequest, KnowledgeBrowserListData, KnowledgeBrowserView,
    KnowledgeContextPackRequest, KnowledgeDriveImportRequest, KnowledgeGitImportRequest,
    KnowledgeGitSyncRequest, KnowledgeIngestRequest, KnowledgeMarketSubscriptionRequest,
    KnowledgeMediaTaskRequest, KnowledgeRetrievalRequest, KnowledgeSpaceMemberSubjectType,
    KnowledgeWechatArticlesPreviewRequest, KnowledgeWechatArticlesPublishRequest,
    KnowledgeWechatReplaceAppletsRequest, KnowledgeWechatReplaceOfficialAccountsRequest,
    KnowledgeWikiPublicationVersionCommandRequest, KnowledgeWikiSourceFileVersionCommandRequest,
    ListKnowledgeBrowserRequest, ListOkfConceptsQuery, OkfBundleExportRequest,
    OkfBundleImportRequest, OkfConceptUpsertRequest, OkfContextPackRequest, OkfFileAnswerRequest,
    OkfQualityRunRequest, OkfQueryRequest, PublishKnowledgeWikiSourceFileRequest,
    UpdateKnowledgeSpaceRequest,
};
use serde::{Deserialize, Serialize};
use std::sync::Arc;

use crate::{
    adapters::{
        AgentAndRetrievalAppApi, AgentOnlyAppApi, BrowserOnlyAppApi, FullAppApi,
        RetrievalOnlyAppApi,
    },
    auth::{require_app_context, RequiredAppContext},
    paths, ApiError, ApiProblem, ApiResult, KnowledgeAgentAppService, KnowledgeAppApi,
    KnowledgeBrowserApi, KnowledgeCommerceAppService, KnowledgeDocumentAppService,
    KnowledgeDriveImportAppService, KnowledgeGitImportAppService, KnowledgeGroupLaunchAppService,
    KnowledgeIngestAppService, KnowledgeOkfAppService, KnowledgeRetrievalAppService,
    KnowledgeSpaceAppService, KnowledgeWikiPublicationAppService,
};
use sdkwork_routes_knowledgebase_backend_api::KnowledgebaseReadinessCheck;

pub use sdkwork_routes_knowledgebase_backend_api::KnowledgebaseReadinessCheck as ReadinessCheck;

#[derive(Clone)]
struct AppState {
    api: Arc<dyn KnowledgeAppApi>,
}

pub fn build_router_with_browser<B>(browser: B) -> Router
where
    B: KnowledgeBrowserApi,
{
    build_router_with_shared_browser(Arc::new(browser))
}

pub fn build_router_with_shared_browser(browser: Arc<dyn KnowledgeBrowserApi>) -> Router {
    build_router_with_shared_app_api(Arc::new(BrowserOnlyAppApi::new(browser)))
}

pub fn build_router_with_retrieval_service<R>(retrieval: R) -> Router
where
    R: KnowledgeRetrievalAppService,
{
    build_router_with_shared_retrieval_service(Arc::new(retrieval))
}

pub fn build_router_with_shared_retrieval_service(
    retrieval: Arc<dyn KnowledgeRetrievalAppService>,
) -> Router {
    build_router_with_shared_app_api(Arc::new(RetrievalOnlyAppApi::new(retrieval)))
}

pub fn build_router_with_agent_service<A>(agent: A) -> Router
where
    A: KnowledgeAgentAppService,
{
    build_router_with_shared_agent_service(Arc::new(agent))
}

pub fn build_router_with_shared_agent_service(agent: Arc<dyn KnowledgeAgentAppService>) -> Router {
    build_router_with_shared_app_api(Arc::new(AgentOnlyAppApi::new(agent)))
}

pub fn build_router_with_agent_and_retrieval_services<A, R>(agent: A, retrieval: R) -> Router
where
    A: KnowledgeAgentAppService,
    R: KnowledgeRetrievalAppService,
{
    build_router_with_shared_agent_and_retrieval_services(Arc::new(agent), Arc::new(retrieval))
}

pub fn build_router_with_shared_agent_and_retrieval_services(
    agent: Arc<dyn KnowledgeAgentAppService>,
    retrieval: Arc<dyn KnowledgeRetrievalAppService>,
) -> Router {
    build_router_with_shared_app_api(Arc::new(AgentAndRetrievalAppApi::new(agent, retrieval)))
}

pub fn build_router_with_app_api<A>(api: A) -> Router
where
    A: KnowledgeAppApi,
{
    build_router_with_shared_app_api(Arc::new(api))
}

#[allow(clippy::too_many_arguments)]
pub fn build_router_with_full_app_api(
    space: Arc<dyn KnowledgeSpaceAppService>,
    wiki: Arc<dyn KnowledgeWikiPublicationAppService>,
    group_launch: Arc<dyn KnowledgeGroupLaunchAppService>,
    drive_import: Arc<dyn KnowledgeDriveImportAppService>,
    git_import: Arc<dyn KnowledgeGitImportAppService>,
    ingest: Arc<dyn KnowledgeIngestAppService>,
    document: Arc<dyn KnowledgeDocumentAppService>,
    okf: Arc<dyn KnowledgeOkfAppService>,
    browser: Arc<dyn KnowledgeBrowserApi>,
    retrieval: Arc<dyn KnowledgeRetrievalAppService>,
    agent: Arc<dyn KnowledgeAgentAppService>,
    context_binding: Arc<dyn crate::KnowledgeContextBindingAppService>,
    wechat: Arc<dyn crate::KnowledgeWechatAppService>,
    commerce: Arc<dyn KnowledgeCommerceAppService>,
) -> Router {
    build_router_with_shared_app_api(Arc::new(FullAppApi::new(
        space,
        wiki,
        group_launch,
        drive_import,
        git_import,
        ingest,
        document,
        okf,
        browser,
        retrieval,
        agent,
        context_binding,
        wechat,
        commerce,
    )))
}

pub fn build_router_with_shared_app_api(api: Arc<dyn KnowledgeAppApi>) -> Router {
    build_business_router(api)
}

/// Builds the app-api business router. Readiness probes are mounted at gateway assembly
/// via `sdkwork_routes_knowledgebase_backend_api::health`, not on this router.
pub fn build_router_with_shared_app_api_and_readiness(
    api: Arc<dyn KnowledgeAppApi>,
    _readiness: Option<KnowledgebaseReadinessCheck>,
) -> Router {
    build_router_with_shared_app_api(api)
}

fn build_business_router(api: Arc<dyn KnowledgeAppApi>) -> Router {
    Router::new()
        .route(
            paths::GROUP_LAUNCHES_CONSUME,
            post(consume_group_launch_ticket),
        )
        .route(paths::SPACES, post(create_space))
        .route(
            paths::SPACE,
            get(retrieve_space).patch(update_space).delete(delete_space),
        )
        .route(paths::WIKI_PUBLICATION, get(retrieve_wiki_publication))
        .route(
            paths::WIKI_PUBLICATION_ACTIVATE,
            post(activate_wiki_publication),
        )
        .route(paths::WIKI_PUBLICATION_PAUSE, post(pause_wiki_publication))
        .route(
            paths::WIKI_SOURCE_FILE_PUBLISH,
            post(publish_wiki_source_file),
        )
        .route(
            paths::WIKI_SOURCE_FILE_UNPUBLISH,
            post(unpublish_wiki_source_file),
        )
        .route(
            paths::WIKI_SOURCE_FILE_VISIBILITY,
            patch(change_wiki_source_file_visibility),
        )
        .route(paths::DRIVE_IMPORTS, post(create_drive_import))
        .route(paths::GIT_IMPORTS, post(create_git_import))
        .route(paths::GIT_SYNCS, post(create_git_sync))
        .route(paths::INGESTS, post(create_ingest))
        .route(paths::INGEST, get(retrieve_ingest))
        .route(paths::DOCUMENTS, get(list_documents).post(create_document))
        .route(
            paths::DOCUMENT,
            get(retrieve_document)
                .patch(update_document)
                .delete(delete_document),
        )
        .route(
            paths::DOCUMENT_VERSIONS,
            get(list_document_versions).post(create_document_version),
        )
        .route(paths::DOCUMENT_CONTENT, get(retrieve_document_content))
        .route(paths::OKF_CONCEPTS, get(list_okf_concepts))
        .route(paths::OKF_CONCEPT_UPSERT, put(upsert_okf_concept))
        .route(
            paths::OKF_CONCEPT,
            get(retrieve_okf_concept).delete(delete_okf_concept),
        )
        .route(
            paths::OKF_CONCEPT_REVISIONS,
            get(list_okf_concept_revisions),
        )
        .route(paths::OKF_INDEX, get(retrieve_okf_index))
        .route(paths::OKF_LOG, get(retrieve_okf_log))
        .route(paths::OKF_PROFILE, get(retrieve_okf_schema))
        .route(paths::OKF_QUERIES, post(create_okf_query))
        .route(paths::OKF_QUERY_FILE_ANSWER, post(file_okf_query_answer))
        .route(paths::OKF_CONTEXT_PACKS, post(create_okf_context_pack))
        .route(paths::OKF_EXPORTS, post(create_okf_export))
        .route(paths::OKF_EXPORT, get(retrieve_okf_export))
        .route(paths::OKF_IMPORTS, post(create_okf_import))
        .route(paths::OKF_LINT_RUNS, post(create_okf_lint_run))
        .route(paths::SPACE_BROWSER, get(list_browser))
        .route(paths::RETRIEVALS, post(create_retrieval))
        .route(paths::RETRIEVAL, get(retrieve_retrieval))
        .route(paths::CONTEXT_PACKS, post(create_context_pack))
        .route(paths::AGENT_PROFILES, post(create_agent_profile))
        .route(
            paths::AGENT_PROFILE,
            get(retrieve_agent_profile)
                .patch(update_agent_profile)
                .delete(delete_agent_profile),
        )
        .route(
            paths::AGENT_PROFILE_BINDINGS,
            get(list_agent_profile_bindings).post(create_agent_profile_binding),
        )
        .route(
            paths::AGENT_PROFILE_BINDING,
            patch(update_agent_profile_binding).delete(delete_agent_profile_binding),
        )
        .route(
            paths::AGENT_PROFILE_RETRIEVAL_PREVIEW,
            post(create_agent_profile_retrieval_preview),
        )
        .route(paths::AGENT_PROFILE_CHAT, post(create_agent_profile_chat))
        .route(
            paths::SPACE_CONTEXT_BINDINGS,
            get(list_space_context_bindings).post(create_space_context_binding),
        )
        .route(
            paths::SPACE_MEMBERS,
            get(list_space_members)
                .post(grant_space_member)
                .delete(revoke_space_member),
        )
        .route(
            paths::CONTEXT_BINDING,
            get(retrieve_context_binding)
                .patch(update_context_binding)
                .delete(delete_context_binding),
        )
        .route(
            paths::WECHAT_OFFICIAL_ACCOUNTS,
            get(list_wechat_official_accounts).put(replace_wechat_official_accounts),
        )
        .route(
            paths::WECHAT_OFFICIAL_ACCOUNT_FAN_TAGS,
            get(list_wechat_official_account_fan_tags),
        )
        .route(
            paths::WECHAT_APPLETS,
            get(list_wechat_applets).put(replace_wechat_applets),
        )
        .route(
            paths::WECHAT_ARTICLES_PUBLISH,
            post(publish_wechat_articles),
        )
        .route(
            paths::WECHAT_ARTICLES_PREVIEW,
            post(preview_wechat_articles),
        )
        .route(paths::MARKET_LISTINGS, get(list_market_listings))
        .route(
            paths::MARKET_SUBSCRIPTIONS,
            post(create_market_subscription),
        )
        .route(
            paths::MARKET_SUBSCRIPTION,
            delete(delete_market_subscription),
        )
        .route(paths::MEDIA_TASKS, post(create_media_task))
        // Explicit transport-level request body bound: the largest business payload is the
        // 512 KiB markdown ingest envelope, so 1 MiB leaves envelope and encoding headroom
        // while rejecting oversized bodies before JSON deserialization.
        .layer(axum::extract::DefaultBodyLimit::max(1_048_576))
        .with_state(AppState { api })
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct ListDocumentsQuery {
    space_id: u64,
    cursor: Option<String>,
    #[serde(rename = "page_size")]
    page_size: Option<u32>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct ListDocumentVersionsQuery {
    cursor: Option<String>,
    #[serde(rename = "page_size")]
    page_size: Option<u32>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct ListContextBindingsQuery {
    cursor: Option<String>,
    #[serde(rename = "page_size")]
    page_size: Option<u32>,
    context_type: Option<sdkwork_knowledgebase_contract::context_binding::KnowledgeContextType>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct ListMarketListingsQuery {
    cursor: Option<String>,
    #[serde(rename = "page_size")]
    page_size: Option<u32>,
}

async fn create_space(
    State(state): State<AppState>,
    context: RequiredAppContext,
    Json(request): Json<CreateKnowledgeSpaceRequest>,
) -> Result<Response, ApiProblem> {
    let context = require_app_context(context)?;
    created_json(state.api.create_space(context, request).await)
}

async fn consume_group_launch_ticket(
    State(state): State<AppState>,
    context: RequiredAppContext,
    Json(request): Json<
        sdkwork_knowledgebase_contract::group_space::ConsumeGroupKnowledgebaseLaunchTicketRequest,
    >,
) -> Result<Response, ApiProblem> {
    let context = require_app_context(context)?;
    ok_json(
        state
            .api
            .consume_group_launch_ticket(context, request)
            .await,
    )
}

async fn retrieve_space(
    State(state): State<AppState>,
    context: RequiredAppContext,
    Path(space_id): Path<u64>,
) -> Result<Response, ApiProblem> {
    let context = require_app_context(context)?;
    ok_json(state.api.retrieve_space(context, space_id).await)
}

async fn update_space(
    State(state): State<AppState>,
    context: RequiredAppContext,
    Path(space_id): Path<u64>,
    Json(request): Json<UpdateKnowledgeSpaceRequest>,
) -> Result<Response, ApiProblem> {
    let context = require_app_context(context)?;
    ok_json(state.api.update_space(context, space_id, request).await)
}

async fn delete_space(
    State(state): State<AppState>,
    context: RequiredAppContext,
    Path(space_id): Path<u64>,
) -> Result<Response, ApiProblem> {
    let context = require_app_context(context)?;
    state
        .api
        .delete_space(context, space_id)
        .await
        .map_err(ApiProblem::from)?;
    Ok(sdkwork_knowledgebase_observability::request_correlation::no_content_response())
}

async fn retrieve_wiki_publication(
    State(state): State<AppState>,
    context: RequiredAppContext,
    Path(space_id): Path<u64>,
) -> Result<Response, ApiProblem> {
    let context = require_app_context(context)?;
    ok_json(state.api.retrieve_wiki_publication(context, space_id).await)
}

async fn activate_wiki_publication(
    State(state): State<AppState>,
    context: RequiredAppContext,
    Path(space_id): Path<u64>,
    Json(request): Json<KnowledgeWikiPublicationVersionCommandRequest>,
) -> Result<Response, ApiProblem> {
    let context = require_app_context(context)?;
    ok_json(
        state
            .api
            .activate_wiki_publication(context, space_id, request)
            .await,
    )
}

async fn pause_wiki_publication(
    State(state): State<AppState>,
    context: RequiredAppContext,
    Path(space_id): Path<u64>,
    Json(request): Json<KnowledgeWikiPublicationVersionCommandRequest>,
) -> Result<Response, ApiProblem> {
    let context = require_app_context(context)?;
    ok_json(
        state
            .api
            .pause_wiki_publication(context, space_id, request)
            .await,
    )
}

async fn publish_wiki_source_file(
    State(state): State<AppState>,
    context: RequiredAppContext,
    Path((space_id, source_file_uuid)): Path<(u64, String)>,
    Json(request): Json<PublishKnowledgeWikiSourceFileRequest>,
) -> Result<Response, ApiProblem> {
    let context = require_app_context(context)?;
    ok_json(
        state
            .api
            .publish_wiki_source_file(context, space_id, source_file_uuid, request)
            .await,
    )
}

async fn unpublish_wiki_source_file(
    State(state): State<AppState>,
    context: RequiredAppContext,
    Path((space_id, source_file_uuid)): Path<(u64, String)>,
    Json(request): Json<KnowledgeWikiSourceFileVersionCommandRequest>,
) -> Result<Response, ApiProblem> {
    let context = require_app_context(context)?;
    ok_json(
        state
            .api
            .unpublish_wiki_source_file(context, space_id, source_file_uuid, request)
            .await,
    )
}

async fn change_wiki_source_file_visibility(
    State(state): State<AppState>,
    context: RequiredAppContext,
    Path((space_id, source_file_uuid)): Path<(u64, String)>,
    Json(request): Json<ChangeKnowledgeWikiSourceFileVisibilityRequest>,
) -> Result<Response, ApiProblem> {
    let context = require_app_context(context)?;
    ok_json(
        state
            .api
            .change_wiki_source_file_visibility(context, space_id, source_file_uuid, request)
            .await,
    )
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct RevokeSpaceMemberQuery {
    subject_type: KnowledgeSpaceMemberSubjectType,
    subject_id: String,
}

async fn list_space_members(
    State(state): State<AppState>,
    context: RequiredAppContext,
    Path(space_id): Path<u64>,
    OriginalUri(uri): OriginalUri,
    CheckedQuery(query): CheckedQuery<ListSpaceMembersQuery>,
) -> Result<Response, ApiProblem> {
    let context = require_app_context(context)?;
    reject_forbidden_pagination_aliases(uri.query())?;
    ok_list_json(
        state
            .api
            .list_space_members(context, space_id, query.cursor, query.page_size)
            .await,
    )
}

async fn grant_space_member(
    State(state): State<AppState>,
    context: RequiredAppContext,
    Path(space_id): Path<u64>,
    Json(request): Json<GrantKnowledgeSpaceMemberRequest>,
) -> Result<Response, ApiProblem> {
    let context = require_app_context(context)?;
    state
        .api
        .grant_space_member(context, space_id, request)
        .await
        .map_err(ApiProblem::from)?;
    command_json()
}

async fn revoke_space_member(
    State(state): State<AppState>,
    context: RequiredAppContext,
    Path(space_id): Path<u64>,
    CheckedQuery(query): CheckedQuery<RevokeSpaceMemberQuery>,
) -> Result<Response, ApiProblem> {
    let context = require_app_context(context)?;
    state
        .api
        .revoke_space_member(context, space_id, query.subject_type, query.subject_id)
        .await
        .map_err(ApiProblem::from)?;
    Ok(sdkwork_knowledgebase_observability::request_correlation::no_content_response())
}

async fn create_drive_import(
    State(state): State<AppState>,
    context: RequiredAppContext,
    Json(request): Json<KnowledgeDriveImportRequest>,
) -> Result<Response, ApiProblem> {
    let context = require_app_context(context)?;
    created_json(state.api.create_drive_import(context, request).await)
}

async fn create_git_import(
    State(state): State<AppState>,
    context: RequiredAppContext,
    Json(request): Json<KnowledgeGitImportRequest>,
) -> Result<Response, ApiProblem> {
    let context = require_app_context(context)?;
    created_json(state.api.create_git_import(context, request).await)
}

async fn create_git_sync(
    State(state): State<AppState>,
    context: RequiredAppContext,
    Json(request): Json<KnowledgeGitSyncRequest>,
) -> Result<Response, ApiProblem> {
    let context = require_app_context(context)?;
    created_json(state.api.create_git_sync(context, request).await)
}

async fn list_wechat_official_accounts(
    State(state): State<AppState>,
    context: RequiredAppContext,
) -> Result<Response, ApiProblem> {
    let context = require_app_context(context)?;
    ok_json(state.api.list_wechat_official_accounts(context).await)
}

async fn replace_wechat_official_accounts(
    State(state): State<AppState>,
    context: RequiredAppContext,
    payload: Result<Json<KnowledgeWechatReplaceOfficialAccountsRequest>, JsonRejection>,
) -> Result<Response, ApiProblem> {
    let context = require_app_context(context)?;
    let request = parse_wechat_config_json(payload)?;
    ok_json(
        state
            .api
            .replace_wechat_official_accounts(context, request)
            .await,
    )
}

async fn list_wechat_official_account_fan_tags(
    State(state): State<AppState>,
    context: RequiredAppContext,
    Path(account_id): Path<String>,
) -> Result<Response, ApiProblem> {
    let context = require_app_context(context)?;
    ok_json(
        state
            .api
            .list_official_account_fan_tags(context, account_id)
            .await,
    )
}

async fn list_wechat_applets(
    State(state): State<AppState>,
    context: RequiredAppContext,
) -> Result<Response, ApiProblem> {
    let context = require_app_context(context)?;
    ok_json(state.api.list_wechat_applets(context).await)
}

async fn replace_wechat_applets(
    State(state): State<AppState>,
    context: RequiredAppContext,
    payload: Result<Json<KnowledgeWechatReplaceAppletsRequest>, JsonRejection>,
) -> Result<Response, ApiProblem> {
    let context = require_app_context(context)?;
    let request = parse_wechat_config_json(payload)?;
    ok_json(state.api.replace_wechat_applets(context, request).await)
}

fn parse_wechat_config_json<T>(payload: Result<Json<T>, JsonRejection>) -> Result<T, ApiProblem> {
    payload.map(|Json(value)| value).map_err(|_| {
        ApiError::invalid_request(
            "invalid_wechat_request",
            "request body must match the WeChat configuration schema",
        )
        .into()
    })
}

async fn publish_wechat_articles(
    State(state): State<AppState>,
    context: RequiredAppContext,
    Json(request): Json<KnowledgeWechatArticlesPublishRequest>,
) -> Result<Response, ApiProblem> {
    let context = require_app_context(context)?;
    ok_json(state.api.publish_wechat_articles(context, request).await)
}

async fn preview_wechat_articles(
    State(state): State<AppState>,
    context: RequiredAppContext,
    Json(request): Json<KnowledgeWechatArticlesPreviewRequest>,
) -> Result<Response, ApiProblem> {
    let context = require_app_context(context)?;
    ok_json(state.api.preview_wechat_articles(context, request).await)
}

async fn list_market_listings(
    State(state): State<AppState>,
    context: RequiredAppContext,
    OriginalUri(uri): OriginalUri,
    CheckedQuery(query): CheckedQuery<ListMarketListingsQuery>,
) -> Result<Response, ApiProblem> {
    let context = require_app_context(context)?;
    reject_forbidden_pagination_aliases(uri.query())?;
    ok_list_json(
        state
            .api
            .list_market_listings(context, query.cursor, query.page_size)
            .await,
    )
}

async fn create_market_subscription(
    State(state): State<AppState>,
    context: RequiredAppContext,
    Json(request): Json<KnowledgeMarketSubscriptionRequest>,
) -> Result<Response, ApiProblem> {
    let context = require_app_context(context)?;
    created_json(state.api.create_market_subscription(context, request).await)
}

async fn delete_market_subscription(
    State(state): State<AppState>,
    context: RequiredAppContext,
    Path(listing_id): Path<u64>,
) -> Result<Response, ApiProblem> {
    let context = require_app_context(context)?;
    state
        .api
        .delete_market_subscription(context, listing_id)
        .await
        .map_err(ApiProblem::from)?;
    Ok(sdkwork_knowledgebase_observability::request_correlation::no_content_response())
}

async fn create_media_task(
    State(state): State<AppState>,
    context: RequiredAppContext,
    Json(request): Json<KnowledgeMediaTaskRequest>,
) -> Result<Response, ApiProblem> {
    let context = require_app_context(context)?;
    created_json(state.api.create_media_task(context, request).await)
}

async fn create_ingest(
    State(state): State<AppState>,
    context: RequiredAppContext,
    Json(request): Json<KnowledgeIngestRequest>,
) -> Result<Response, ApiProblem> {
    let context = require_app_context(context)?;
    created_json(state.api.create_ingest(context, request).await)
}

async fn retrieve_ingest(
    State(state): State<AppState>,
    context: RequiredAppContext,
    Path(ingest_id): Path<u64>,
) -> Result<Response, ApiProblem> {
    let context = require_app_context(context)?;
    ok_json(state.api.retrieve_ingest(context, ingest_id).await)
}

async fn list_documents(
    State(state): State<AppState>,
    context: RequiredAppContext,
    OriginalUri(uri): OriginalUri,
    CheckedQuery(query): CheckedQuery<ListDocumentsQuery>,
) -> Result<Response, ApiProblem> {
    let context = require_app_context(context)?;
    reject_forbidden_pagination_aliases(uri.query())?;
    ok_list_json(
        state
            .api
            .list_documents(context, query.space_id, query.cursor, query.page_size)
            .await,
    )
}

async fn create_document(
    State(state): State<AppState>,
    context: RequiredAppContext,
    Json(request): Json<CreateKnowledgeDocumentRequest>,
) -> Result<Response, ApiProblem> {
    let context = require_app_context(context)?;
    created_json(state.api.create_document(context, request).await)
}

async fn retrieve_document(
    State(state): State<AppState>,
    context: RequiredAppContext,
    Path(document_id): Path<u64>,
) -> Result<Response, ApiProblem> {
    let context = require_app_context(context)?;
    ok_json(state.api.retrieve_document(context, document_id).await)
}

async fn retrieve_document_content(
    State(state): State<AppState>,
    context: RequiredAppContext,
    Path(document_id): Path<u64>,
) -> Result<Response, ApiProblem> {
    let context = require_app_context(context)?;
    ok_json(
        state
            .api
            .retrieve_document_content(context, document_id)
            .await,
    )
}

async fn update_document(
    State(state): State<AppState>,
    context: RequiredAppContext,
    Path(document_id): Path<u64>,
    Json(request): Json<CreateKnowledgeDocumentRequest>,
) -> Result<Response, ApiProblem> {
    let context = require_app_context(context)?;
    ok_json(
        state
            .api
            .update_document(context, document_id, request)
            .await,
    )
}

async fn delete_document(
    State(state): State<AppState>,
    context: RequiredAppContext,
    Path(document_id): Path<u64>,
) -> Result<Response, ApiProblem> {
    let context = require_app_context(context)?;
    state
        .api
        .delete_document(context, document_id)
        .await
        .map_err(ApiProblem::from)?;
    Ok(sdkwork_knowledgebase_observability::request_correlation::no_content_response())
}

async fn list_document_versions(
    State(state): State<AppState>,
    context: RequiredAppContext,
    Path(document_id): Path<u64>,
    OriginalUri(uri): OriginalUri,
    CheckedQuery(query): CheckedQuery<ListDocumentVersionsQuery>,
) -> Result<Response, ApiProblem> {
    let context = require_app_context(context)?;
    reject_forbidden_pagination_aliases(uri.query())?;
    ok_list_json(
        state
            .api
            .list_document_versions(context, document_id, query.cursor, query.page_size)
            .await,
    )
}

async fn create_document_version(
    State(state): State<AppState>,
    context: RequiredAppContext,
    Path(document_id): Path<u64>,
    Json(request): Json<CreateKnowledgeDocumentVersionRequest>,
) -> Result<Response, ApiProblem> {
    let context = require_app_context(context)?;
    if request.document_id != document_id {
        return Err(ApiProblem::new(
            StatusCode::BAD_REQUEST,
            "document_id_mismatch",
            "documentId in body must match documentId in path",
        ));
    }
    ok_json(
        state
            .api
            .create_document_version(context, document_id, request)
            .await,
    )
}

async fn list_okf_concepts(
    State(state): State<AppState>,
    context: RequiredAppContext,
    OriginalUri(uri): OriginalUri,
) -> Result<Response, ApiProblem> {
    let context = require_app_context(context)?;
    let query = parse_okf_concept_list_query(uri.query())?;
    ok_list_json(
        state
            .api
            .list_okf_concepts(context, query.space_id, query.cursor, query.page_size)
            .await,
    )
}

async fn upsert_okf_concept(
    State(state): State<AppState>,
    context: RequiredAppContext,
    Json(request): Json<OkfConceptUpsertRequest>,
) -> Result<Response, ApiProblem> {
    let context = require_app_context(context)?;
    ok_json(state.api.upsert_okf_concept(context, request).await)
}

async fn retrieve_okf_concept(
    State(state): State<AppState>,
    context: RequiredAppContext,
    Path(concept_row_id): Path<u64>,
) -> Result<Response, ApiProblem> {
    let context = require_app_context(context)?;
    ok_json(
        state
            .api
            .retrieve_okf_concept(context, concept_row_id)
            .await,
    )
}

async fn delete_okf_concept(
    State(state): State<AppState>,
    context: RequiredAppContext,
    Path(concept_row_id): Path<u64>,
) -> Result<Response, ApiProblem> {
    let context = require_app_context(context)?;
    state
        .api
        .delete_okf_concept(context, concept_row_id)
        .await
        .map_err(ApiProblem::from)?;
    Ok(sdkwork_knowledgebase_observability::request_correlation::no_content_response())
}

async fn list_okf_concept_revisions(
    State(state): State<AppState>,
    context: RequiredAppContext,
    Path(concept_row_id): Path<u64>,
    OriginalUri(uri): OriginalUri,
) -> Result<Response, ApiProblem> {
    let context = require_app_context(context)?;
    let query = parse_okf_revision_list_query(uri.query())?;
    ok_list_json(
        state
            .api
            .list_okf_concept_revisions(context, concept_row_id, query.cursor, query.page_size)
            .await,
    )
}

async fn retrieve_okf_index(
    State(state): State<AppState>,
    context: RequiredAppContext,
    CheckedQuery(query): CheckedQuery<ListOkfConceptsQuery>,
) -> Result<Response, ApiProblem> {
    let context = require_app_context(context)?;
    ok_json(state.api.retrieve_okf_index(context, query.space_id).await)
}

async fn retrieve_okf_log(
    State(state): State<AppState>,
    context: RequiredAppContext,
    CheckedQuery(query): CheckedQuery<ListOkfConceptsQuery>,
) -> Result<Response, ApiProblem> {
    let context = require_app_context(context)?;
    ok_json(state.api.retrieve_okf_log(context, query.space_id).await)
}

async fn retrieve_okf_schema(
    State(state): State<AppState>,
    context: RequiredAppContext,
    CheckedQuery(query): CheckedQuery<ListOkfConceptsQuery>,
) -> Result<Response, ApiProblem> {
    let context = require_app_context(context)?;
    ok_json(state.api.retrieve_okf_schema(context, query.space_id).await)
}

async fn create_okf_query(
    State(state): State<AppState>,
    context: RequiredAppContext,
    Json(request): Json<OkfQueryRequest>,
) -> Result<Response, ApiProblem> {
    let context = require_app_context(context)?;
    created_json(state.api.create_okf_query(context, request).await)
}

async fn file_okf_query_answer(
    State(state): State<AppState>,
    context: RequiredAppContext,
    Path(query_id): Path<u64>,
    Json(request): Json<OkfFileAnswerRequest>,
) -> Result<Response, ApiProblem> {
    let context = require_app_context(context)?;
    ok_json(
        state
            .api
            .file_okf_query_answer(context, query_id, request)
            .await,
    )
}

async fn create_okf_context_pack(
    State(state): State<AppState>,
    context: RequiredAppContext,
    Json(request): Json<OkfContextPackRequest>,
) -> Result<Response, ApiProblem> {
    let context = require_app_context(context)?;
    created_json(state.api.create_okf_context_pack(context, request).await)
}

async fn create_okf_export(
    State(state): State<AppState>,
    context: RequiredAppContext,
    Json(request): Json<OkfBundleExportRequest>,
) -> Result<Response, ApiProblem> {
    let context = require_app_context(context)?;
    created_json(state.api.create_okf_export(context, request).await)
}

async fn retrieve_okf_export(
    State(state): State<AppState>,
    context: RequiredAppContext,
    Path(export_id): Path<u64>,
) -> Result<Response, ApiProblem> {
    let context = require_app_context(context)?;
    ok_json(state.api.retrieve_okf_export(context, export_id).await)
}

async fn create_okf_import(
    State(state): State<AppState>,
    context: RequiredAppContext,
    Json(request): Json<OkfBundleImportRequest>,
) -> Result<Response, ApiProblem> {
    let context = require_app_context(context)?;
    created_json(state.api.create_okf_import(context, request).await)
}

async fn create_okf_lint_run(
    State(state): State<AppState>,
    context: RequiredAppContext,
    Json(request): Json<OkfQualityRunRequest>,
) -> Result<Response, ApiProblem> {
    let context = require_app_context(context)?;
    created_json(state.api.create_okf_lint_run(context, request).await)
}

async fn list_browser(
    State(state): State<AppState>,
    context: RequiredAppContext,
    Path(space_id): Path<u64>,
    OriginalUri(uri): OriginalUri,
    CheckedQuery(query): CheckedQuery<ListBrowserQuery>,
) -> Result<Response, ApiProblem> {
    let context = require_app_context(context)?;
    reject_forbidden_pagination_aliases(uri.query())?;
    let view = parse_view(query.view.as_deref())?;
    ok_browser_list_json(
        state
            .api
            .list_browser(
                context,
                ListKnowledgeBrowserRequest {
                    space_id,
                    parent_id: query.parent_id,
                    view,
                    cursor: query.cursor,
                    page_size: query.page_size,
                },
            )
            .await,
    )
}

async fn create_retrieval(
    State(state): State<AppState>,
    context: RequiredAppContext,
    Json(request): Json<KnowledgeRetrievalRequest>,
) -> Result<Response, ApiProblem> {
    let context = require_app_context(context)?;
    let tenant_id = context.tenant_id;
    let actor_id = context.actor_id;
    created_json(
        state
            .api
            .create_retrieval(
                context,
                request.with_tenant_id(tenant_id).with_actor_id(actor_id),
            )
            .await,
    )
}

async fn retrieve_retrieval(
    State(state): State<AppState>,
    Path(retrieval_id): Path<u64>,
    context: RequiredAppContext,
) -> Result<Response, ApiProblem> {
    let context = require_app_context(context)?;
    ok_json(state.api.retrieve_retrieval(context, retrieval_id).await)
}

async fn create_context_pack(
    State(state): State<AppState>,
    context: RequiredAppContext,
    Json(request): Json<KnowledgeContextPackRequest>,
) -> Result<Response, ApiProblem> {
    let context = require_app_context(context)?;
    let tenant_id = context.tenant_id;
    let actor_id = context.actor_id;
    created_json(
        state
            .api
            .create_context_pack(
                context,
                request.with_tenant_id(tenant_id).with_actor_id(actor_id),
            )
            .await,
    )
}

async fn create_agent_profile(
    State(state): State<AppState>,
    context: RequiredAppContext,
    Json(request): Json<KnowledgeAgentProfileRequest>,
) -> Result<Response, ApiProblem> {
    let context = require_app_context(context)?;
    let tenant_id = context.tenant_id;
    created_json(
        state
            .api
            .create_agent_profile(context, request.with_tenant_id(tenant_id))
            .await,
    )
}

async fn retrieve_agent_profile(
    State(state): State<AppState>,
    context: RequiredAppContext,
    Path(profile_id): Path<u64>,
) -> Result<Response, ApiProblem> {
    let context = require_app_context(context)?;
    ok_json(state.api.retrieve_agent_profile(context, profile_id).await)
}

async fn update_agent_profile(
    State(state): State<AppState>,
    context: RequiredAppContext,
    Path(profile_id): Path<u64>,
    Json(request): Json<KnowledgeAgentProfileRequest>,
) -> Result<Response, ApiProblem> {
    let context = require_app_context(context)?;
    let tenant_id = context.tenant_id;
    ok_json(
        state
            .api
            .update_agent_profile(context, profile_id, request.with_tenant_id(tenant_id))
            .await,
    )
}

async fn delete_agent_profile(
    State(state): State<AppState>,
    context: RequiredAppContext,
    Path(profile_id): Path<u64>,
) -> Result<Response, ApiProblem> {
    let context = require_app_context(context)?;
    state
        .api
        .delete_agent_profile(context, profile_id)
        .await
        .map_err(ApiProblem::from)?;
    Ok(sdkwork_knowledgebase_observability::request_correlation::no_content_response())
}

async fn list_agent_profile_bindings(
    State(state): State<AppState>,
    context: RequiredAppContext,
    Path(profile_id): Path<u64>,
) -> Result<Response, ApiProblem> {
    let context = require_app_context(context)?;
    ok_json(
        state
            .api
            .list_agent_profile_bindings(context, profile_id)
            .await,
    )
}

async fn create_agent_profile_binding(
    State(state): State<AppState>,
    context: RequiredAppContext,
    Path(profile_id): Path<u64>,
    Json(request): Json<KnowledgeAgentBindingRequest>,
) -> Result<Response, ApiProblem> {
    let context = require_app_context(context)?;
    let tenant_id = context.tenant_id;
    if request.profile_id != profile_id {
        return Err(ApiProblem::new(
            StatusCode::BAD_REQUEST,
            "profile_id_mismatch",
            "profileId in body must match profileId in path",
        ));
    }
    ok_json(
        state
            .api
            .create_agent_profile_binding(context, profile_id, request.with_tenant_id(tenant_id))
            .await,
    )
}

async fn update_agent_profile_binding(
    State(state): State<AppState>,
    context: RequiredAppContext,
    Path((profile_id, binding_id)): Path<(u64, u64)>,
    Json(request): Json<KnowledgeAgentBindingRequest>,
) -> Result<Response, ApiProblem> {
    let context = require_app_context(context)?;
    let tenant_id = context.tenant_id;
    if request.profile_id != profile_id {
        return Err(ApiProblem::new(
            StatusCode::BAD_REQUEST,
            "profile_id_mismatch",
            "profileId in body must match profileId in path",
        ));
    }
    ok_json(
        state
            .api
            .update_agent_profile_binding(
                context,
                profile_id,
                binding_id,
                request.with_tenant_id(tenant_id),
            )
            .await,
    )
}

async fn delete_agent_profile_binding(
    State(state): State<AppState>,
    context: RequiredAppContext,
    Path((profile_id, binding_id)): Path<(u64, u64)>,
) -> Result<Response, ApiProblem> {
    let context = require_app_context(context)?;
    state
        .api
        .delete_agent_profile_binding(context, profile_id, binding_id)
        .await
        .map_err(ApiProblem::from)?;
    Ok(sdkwork_knowledgebase_observability::request_correlation::no_content_response())
}

async fn create_agent_profile_retrieval_preview(
    State(state): State<AppState>,
    context: RequiredAppContext,
    Path(profile_id): Path<u64>,
    Json(request): Json<KnowledgeRetrievalRequest>,
) -> Result<Response, ApiProblem> {
    let context = require_app_context(context)?;
    let tenant_id = context.tenant_id;
    ok_json(
        state
            .api
            .create_agent_profile_retrieval_preview(
                context,
                profile_id,
                request.with_tenant_id(tenant_id),
            )
            .await,
    )
}

async fn create_agent_profile_chat(
    State(state): State<AppState>,
    context: RequiredAppContext,
    Path(profile_id): Path<u64>,
    Json(request): Json<KnowledgeAgentChatRequest>,
) -> Result<Response, ApiProblem> {
    let context = require_app_context(context)?;
    let tenant_id = context.tenant_id;
    ok_json(
        state
            .api
            .create_agent_chat(context, profile_id, request.with_tenant_id(tenant_id))
            .await,
    )
}

async fn list_space_context_bindings(
    State(state): State<AppState>,
    context: RequiredAppContext,
    Path(space_id): Path<u64>,
    OriginalUri(uri): OriginalUri,
    CheckedQuery(query): CheckedQuery<ListContextBindingsQuery>,
) -> Result<Response, ApiProblem> {
    let context = require_app_context(context)?;
    reject_forbidden_pagination_aliases(uri.query())?;
    ok_list_json(
        state
            .api
            .list_space_context_bindings(
                context,
                space_id,
                query.cursor,
                query.page_size,
                query.context_type,
            )
            .await,
    )
}

async fn create_space_context_binding(
    State(state): State<AppState>,
    context: RequiredAppContext,
    Path(space_id): Path<u64>,
    Json(request): Json<CreateKnowledgeSpaceContextBindingRequest>,
) -> Result<Response, ApiProblem> {
    let context = require_app_context(context)?;
    if request.space_id != space_id {
        return Err(ApiProblem::new(
            StatusCode::BAD_REQUEST,
            "space_id_mismatch",
            "spaceId in body must match spaceId in path",
        ));
    }
    ok_json(
        state
            .api
            .create_space_context_binding(context, space_id, request)
            .await,
    )
}

async fn retrieve_context_binding(
    State(state): State<AppState>,
    context: RequiredAppContext,
    Path(binding_id): Path<u64>,
) -> Result<Response, ApiProblem> {
    let context = require_app_context(context)?;
    ok_json(
        state
            .api
            .retrieve_context_binding(context, binding_id)
            .await,
    )
}

async fn update_context_binding(
    State(state): State<AppState>,
    context: RequiredAppContext,
    Path(binding_id): Path<u64>,
    Json(request): Json<UpdateKnowledgeSpaceContextBindingRequest>,
) -> Result<Response, ApiProblem> {
    let context = require_app_context(context)?;
    ok_json(
        state
            .api
            .update_context_binding(context, binding_id, request)
            .await,
    )
}

async fn delete_context_binding(
    State(state): State<AppState>,
    context: RequiredAppContext,
    Path(binding_id): Path<u64>,
) -> Result<Response, ApiProblem> {
    let context = require_app_context(context)?;
    state
        .api
        .delete_context_binding(context, binding_id)
        .await
        .map_err(ApiProblem::from)?;
    Ok(sdkwork_knowledgebase_observability::request_correlation::no_content_response())
}

fn ok_list_json<T>(
    result: ApiResult<sdkwork_utils_rust::SdkWorkPageData<T>>,
) -> Result<Response, ApiProblem>
where
    T: Serialize,
{
    result
        .map(|value| {
            sdkwork_knowledgebase_observability::request_correlation::success_list_json_response(
                StatusCode::OK,
                value,
            )
        })
        .map_err(ApiProblem::from)
}

fn ok_browser_list_json(
    result: ApiResult<KnowledgeBrowserListData>,
) -> Result<Response, ApiProblem> {
    result
        .map(|value| {
            sdkwork_knowledgebase_observability::request_correlation::success_browser_list_json_response(
                StatusCode::OK,
                value,
            )
        })
        .map_err(ApiProblem::from)
}

fn ok_json<T>(result: ApiResult<T>) -> Result<Response, ApiProblem>
where
    T: Serialize,
{
    result
        .map(|value| {
            sdkwork_knowledgebase_observability::request_correlation::success_json_response(
                StatusCode::OK,
                value,
            )
        })
        .map_err(ApiProblem::from)
}

fn created_json<T>(result: ApiResult<T>) -> Result<Response, ApiProblem>
where
    T: Serialize,
{
    result
        .map(|value| {
            sdkwork_knowledgebase_observability::request_correlation::success_json_response(
                StatusCode::CREATED,
                value,
            )
        })
        .map_err(ApiProblem::from)
}

fn command_json() -> Result<Response, ApiProblem> {
    Ok(
        sdkwork_knowledgebase_observability::request_correlation::success_command_json_response(
            StatusCode::OK,
            sdkwork_utils_rust::SdkWorkCommandData::accepted(),
        ),
    )
}

const FORBIDDEN_PAGINATION_QUERY_ALIASES: &[(&str, &str)] = &[
    ("pageSize", "page_size"),
    ("limit", "page_size"),
    ("page_no", "page"),
    ("pageNo", "page"),
    ("per_page", "page_size"),
    ("size", "page_size"),
];

/// Query extractor that rejects forbidden pagination aliases (PAGINATION_SPEC
/// §3.1) BEFORE serde deserialization, and converts deserialization failures
/// into the standard problem+json envelope instead of axum's plain-text 400,
/// so API consumers always receive the documented error contract.
struct CheckedQuery<T>(T);

impl<S, T> FromRequestParts<S> for CheckedQuery<T>
where
    S: Send + Sync,
    T: DeserializeOwned,
{
    type Rejection = ApiProblem;

    async fn from_request_parts(parts: &mut Parts, state: &S) -> Result<Self, Self::Rejection> {
        reject_forbidden_pagination_aliases(parts.uri.query())?;
        let query = Query::<T>::from_request_parts(parts, state)
            .await
            .map_err(|rejection| {
                ApiProblem::new(
                    StatusCode::BAD_REQUEST,
                    "invalid_parameter",
                    rejection.to_string(),
                )
            })?;
        Ok(Self(query.0))
    }
}

fn reject_forbidden_pagination_aliases(query: Option<&str>) -> Result<(), ApiProblem> {
    let Some(query) = query else {
        return Ok(());
    };
    for (key, _) in url::form_urlencoded::parse(query.as_bytes()) {
        if let Some((alias, canonical)) = FORBIDDEN_PAGINATION_QUERY_ALIASES
            .iter()
            .find(|(alias, _)| key == *alias)
        {
            return Err(ApiProblem::new(
                StatusCode::BAD_REQUEST,
                "invalid_parameter",
                format!("HTTP query parameter {alias} is forbidden; use {canonical}"),
            ));
        }
    }
    Ok(())
}

#[derive(Debug, Default)]
struct OkfRevisionListQuery {
    cursor: Option<String>,
    page_size: Option<u32>,
}

fn parse_okf_concept_list_query(query: Option<&str>) -> Result<ListOkfConceptsQuery, ApiProblem> {
    reject_forbidden_pagination_aliases(query)?;
    let mut space_id = None;
    let mut pagination = OkfRevisionListQuery::default();
    for (key, value) in url::form_urlencoded::parse(query.unwrap_or_default().as_bytes()) {
        match key.as_ref() {
            "spaceId" => {
                reject_duplicate_query_parameter(space_id.is_some(), "spaceId")?;
                let parsed = parse_positive_u64_query_parameter(&value, "spaceId")?;
                space_id = Some(parsed);
            }
            "cursor" => {
                reject_duplicate_query_parameter(pagination.cursor.is_some(), "cursor")?;
                pagination.cursor = Some(value.into_owned());
            }
            "page_size" => {
                reject_duplicate_query_parameter(pagination.page_size.is_some(), "page_size")?;
                pagination.page_size = Some(parse_page_size_query_parameter(&value)?);
            }
            _ => {
                return Err(invalid_query_parameter(format!(
                    "unknown query parameter: {key}"
                )))
            }
        }
    }
    Ok(ListOkfConceptsQuery {
        space_id: space_id.ok_or_else(|| invalid_query_parameter("spaceId is required"))?,
        cursor: pagination.cursor,
        page_size: pagination.page_size,
    })
}

fn parse_okf_revision_list_query(query: Option<&str>) -> Result<OkfRevisionListQuery, ApiProblem> {
    reject_forbidden_pagination_aliases(query)?;
    let mut parsed = OkfRevisionListQuery::default();
    for (key, value) in url::form_urlencoded::parse(query.unwrap_or_default().as_bytes()) {
        match key.as_ref() {
            "cursor" => {
                reject_duplicate_query_parameter(parsed.cursor.is_some(), "cursor")?;
                parsed.cursor = Some(value.into_owned());
            }
            "page_size" => {
                reject_duplicate_query_parameter(parsed.page_size.is_some(), "page_size")?;
                parsed.page_size = Some(parse_page_size_query_parameter(&value)?);
            }
            _ => {
                return Err(invalid_query_parameter(format!(
                    "unknown query parameter: {key}"
                )))
            }
        }
    }
    Ok(parsed)
}

fn parse_positive_u64_query_parameter(value: &str, name: &str) -> Result<u64, ApiProblem> {
    let value = value
        .parse::<u64>()
        .map_err(|_| invalid_query_parameter(format!("{name} must be a positive integer")))?;
    if value == 0 {
        return Err(invalid_query_parameter(format!(
            "{name} must be a positive integer"
        )));
    }
    Ok(value)
}

fn parse_page_size_query_parameter(value: &str) -> Result<u32, ApiProblem> {
    let page_size = value
        .parse::<u32>()
        .map_err(|_| invalid_query_parameter("page_size must be an integer between 1 and 200"))?;
    crate::pagination::normalize_page_size(Some(page_size))
        .map_err(|_| invalid_query_parameter("page_size must be between 1 and 200"))
}

fn reject_duplicate_query_parameter(duplicate: bool, name: &str) -> Result<(), ApiProblem> {
    if duplicate {
        return Err(invalid_query_parameter(format!(
            "query parameter {name} must appear at most once"
        )));
    }
    Ok(())
}

fn invalid_query_parameter(detail: impl Into<String>) -> ApiProblem {
    ApiProblem::new(StatusCode::BAD_REQUEST, "invalid_parameter", detail)
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct ListSpaceMembersQuery {
    cursor: Option<String>,
    #[serde(rename = "page_size")]
    page_size: Option<u32>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct ListBrowserQuery {
    view: Option<String>,
    parent_id: Option<String>,
    cursor: Option<String>,
    #[serde(rename = "page_size")]
    page_size: Option<u32>,
}

fn parse_view(value: Option<&str>) -> Result<KnowledgeBrowserView, ApiProblem> {
    match value.unwrap_or("files") {
        "files" => Ok(KnowledgeBrowserView::Files),
        "okf_bundle" => Ok(KnowledgeBrowserView::OkfBundle),
        "outputs" => Ok(KnowledgeBrowserView::Outputs),
        value => Err(ApiProblem::new(
            StatusCode::BAD_REQUEST,
            "invalid_browser_view",
            format!("unsupported browser view: {value}"),
        )),
    }
}
