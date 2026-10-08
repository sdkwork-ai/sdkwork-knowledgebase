use axum::{
    extract::{FromRequestParts, Path, Query, State},
    http::{request::Parts, StatusCode},
    response::Response,
    routing::{get, post},
    Json, Router,
};
use sdkwork_knowledgebase_contract::{
    KnowledgeBrowserListData, KnowledgeBrowserView, KnowledgeContextPackRequest,
    KnowledgeIngestRequest, KnowledgeRetrievalRequest, ListKnowledgeBrowserRequest,
};
use sdkwork_routes_knowledgebase_backend_api::{health, KnowledgebaseReadinessCheck};
use serde::de::DeserializeOwned;
use serde::{Deserialize, Serialize};
use std::sync::Arc;

use crate::{
    auth::{require_context, RequiredOpenContext},
    paths, ApiProblem, ApiResult, KnowledgeOpenApi,
};

#[derive(Clone)]
struct OpenState {
    api: Arc<dyn KnowledgeOpenApi>,
}

pub fn build_router_with_open_api<A>(api: A) -> Router
where
    A: KnowledgeOpenApi,
{
    build_router_with_shared_open_api(Arc::new(api))
}

pub fn build_router_with_shared_open_api(api: Arc<dyn KnowledgeOpenApi>) -> Router {
    build_router_with_shared_open_api_and_readiness(api, None)
}

pub fn build_router_with_shared_open_api_and_readiness(
    api: Arc<dyn KnowledgeOpenApi>,
    readiness: Option<KnowledgebaseReadinessCheck>,
) -> Router {
    health::mount_knowledgebase_infra_routes(
        build_business_router_with_shared_open_api(api),
        health::knowledgebase_service_router_config(readiness),
    )
}

pub fn build_business_router_with_shared_open_api(api: Arc<dyn KnowledgeOpenApi>) -> Router {
    Router::new()
        .route(paths::RETRIEVALS, post(create_retrieval))
        .route(paths::RETRIEVAL, get(retrieve_retrieval))
        .route(paths::CONTEXT_PACKS, post(create_context_pack))
        .route(paths::INGESTS, post(create_ingest))
        .route(paths::INGEST, get(retrieve_ingest))
        .route(paths::DOCUMENTS, get(list_documents))
        .route(paths::DOCUMENT, get(retrieve_document))
        .route(paths::SPACE_BROWSER, get(list_browser))
        // Explicit transport-level request body bound: the largest Open API payload is the
        // 512 KiB markdown ingest envelope, so 1 MiB leaves headroom while rejecting
        // oversized bodies before JSON deserialization.
        .layer(axum::extract::DefaultBodyLimit::max(1_048_576))
        .with_state(OpenState { api })
}

pub fn gateway_mount_business(api: Arc<dyn KnowledgeOpenApi>) -> Router {
    build_business_router_with_shared_open_api(api)
}

async fn create_retrieval(
    State(state): State<OpenState>,
    context: RequiredOpenContext,
    Json(request): Json<KnowledgeRetrievalRequest>,
) -> Result<Response, ApiProblem> {
    let context = require_context(context)?;
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
    State(state): State<OpenState>,
    context: RequiredOpenContext,
    Path(retrieval_id): Path<u64>,
) -> Result<Response, ApiProblem> {
    let context = require_context(context)?;
    ok_json(state.api.retrieve_retrieval(context, retrieval_id).await)
}

async fn create_context_pack(
    State(state): State<OpenState>,
    context: RequiredOpenContext,
    Json(request): Json<KnowledgeContextPackRequest>,
) -> Result<Response, ApiProblem> {
    let context = require_context(context)?;
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

async fn create_ingest(
    State(state): State<OpenState>,
    context: RequiredOpenContext,
    Json(request): Json<KnowledgeIngestRequest>,
) -> Result<Response, ApiProblem> {
    let context = require_context(context)?;
    created_json(state.api.create_ingest(context, request).await)
}

async fn retrieve_ingest(
    State(state): State<OpenState>,
    context: RequiredOpenContext,
    Path(ingest_id): Path<u64>,
) -> Result<Response, ApiProblem> {
    let context = require_context(context)?;
    ok_json(state.api.retrieve_ingest(context, ingest_id).await)
}

async fn list_documents(
    State(state): State<OpenState>,
    context: RequiredOpenContext,
    CheckedQuery(query): CheckedQuery<ListDocumentsQuery>,
) -> Result<Response, ApiProblem> {
    let context = require_context(context)?;
    ok_list_json(
        state
            .api
            .list_documents(context, query.space_id, query.cursor, query.page_size)
            .await,
    )
}

async fn retrieve_document(
    State(state): State<OpenState>,
    context: RequiredOpenContext,
    Path(document_id): Path<u64>,
) -> Result<Response, ApiProblem> {
    let context = require_context(context)?;
    ok_json(state.api.retrieve_document(context, document_id).await)
}

async fn list_browser(
    State(state): State<OpenState>,
    context: RequiredOpenContext,
    Path(space_id): Path<u64>,
    CheckedQuery(query): CheckedQuery<ListBrowserQuery>,
) -> Result<Response, ApiProblem> {
    let context = require_context(context)?;
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

// Query parameter names follow the API_SPEC §13 lower_snake_case canonical form.
#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct ListDocumentsQuery {
    space_id: u64,
    cursor: Option<String>,
    #[serde(rename = "page_size")]
    page_size: Option<u32>,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct ListBrowserQuery {
    view: Option<String>,
    parent_id: Option<String>,
    cursor: Option<String>,
    #[serde(rename = "page_size")]
    page_size: Option<u32>,
}

/// Query extractor that rejects forbidden pagination aliases (PAGINATION_SPEC
/// §3.1) BEFORE serde deserialization, and converts deserialization failures
/// (including unknown parameters rejected by `deny_unknown_fields`) into the
/// standard problem+json envelope instead of axum's plain-text 400, matching
/// the app-api `CheckedQuery` error contract.
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

const FORBIDDEN_PAGINATION_QUERY_ALIASES: &[(&str, &str)] = &[
    ("pageSize", "page_size"),
    ("limit", "page_size"),
    ("page_no", "page"),
    ("pageNo", "page"),
    ("per_page", "page_size"),
    ("size", "page_size"),
];

fn reject_forbidden_pagination_aliases(query: Option<&str>) -> Result<(), ApiProblem> {
    let Some(query) = query else {
        return Ok(());
    };
    for pair in query.split('&') {
        let key = pair.split_once('=').map_or(pair, |(key, _)| key);
        if let Some((alias, canonical)) = FORBIDDEN_PAGINATION_QUERY_ALIASES
            .iter()
            .find(|(alias, _)| key == *alias)
        {
            return Err(ApiProblem::new(
                StatusCode::BAD_REQUEST,
                "pagination_parameter_alias_forbidden",
                format!("HTTP query parameter {alias} is forbidden; use {canonical}"),
            ));
        }
    }
    Ok(())
}
