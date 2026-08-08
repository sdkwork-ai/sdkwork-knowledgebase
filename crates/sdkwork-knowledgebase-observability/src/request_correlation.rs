//! Request correlation identifiers for logs, traces, and problem responses.
//!
//! Provides canonical `SdkWorkApiResponse` success envelope wrapping and
//! `application/problem+json` error response building per `API_SPEC.md` §15.

use axum::http::{header, HeaderName, HeaderValue, StatusCode};
use axum::response::{IntoResponse, Response};
use axum::Json;
use sdkwork_knowledgebase_contract::{KnowledgeBrowserListData, ProblemDetails};
use sdkwork_utils_rust::{
    SdkWorkApiResponse, SdkWorkCommandData, SdkWorkPageData, SdkWorkResourceData,
    SDKWORK_TRACE_ID_HEADER,
};
use serde::Serialize;
use std::future::Future;

tokio::task_local! {
    static REQUEST_ID: String;
}

/// Run a request-scoped future with the active correlation id.
pub async fn request_id_scope<Fut>(request_id: String, future: Fut) -> Fut::Output
where
    Fut: Future,
{
    REQUEST_ID.scope(request_id, future).await
}

/// Returns the active request correlation id when called inside [`request_id_scope`].
pub fn current_request_id() -> Option<String> {
    REQUEST_ID.try_with(|request_id| request_id.clone()).ok()
}

/// Resolve the trace id for the current request scope.
///
/// Falls back to a freshly generated UUID v4 when no request-scoped id is
/// available (e.g. during tests or background tasks).
pub fn resolve_trace_id() -> String {
    current_request_id().unwrap_or_else(|| uuid::Uuid::new_v4().to_string())
}

/// Attach the active request id to problem details when the trace id is empty.
pub fn enrich_problem_trace_id(mut problem: ProblemDetails) -> ProblemDetails {
    if problem.trace_id.is_empty() {
        problem.trace_id = resolve_trace_id();
    }
    problem
}

/// Build a RFC 7807 problem response with correlation metadata.
pub fn problem_json_response(status: StatusCode, problem: ProblemDetails) -> Response {
    let problem = enrich_problem_trace_id(problem);
    let trace_id = problem.trace_id.clone();
    let mut response = (status, Json(problem)).into_response();
    response.headers_mut().insert(
        header::CONTENT_TYPE,
        HeaderValue::from_static("application/problem+json"),
    );
    attach_trace_header(&mut response, &trace_id);
    response
}

/// Build a success list response wrapping `SdkWorkPageData` directly in
/// `SdkWorkApiResponse.data` (`API_SPEC.md` §16).
///
/// Wire body: `{ "code": 0, "data": { "items": [...], "pageInfo": {...} }, "traceId": "..." }`.
pub fn success_list_json_response<T: Serialize>(
    status: StatusCode,
    data: SdkWorkPageData<T>,
) -> Response {
    let trace_id = resolve_trace_id();
    let envelope = SdkWorkApiResponse::success(data, trace_id.clone());
    let mut response = (status, Json(envelope)).into_response();
    attach_trace_header(&mut response, &trace_id);
    response
}

/// Build a knowledge browser list response with standard list fields and
/// browser view context directly in `SdkWorkApiResponse.data`.
pub fn success_browser_list_json_response(
    status: StatusCode,
    data: KnowledgeBrowserListData,
) -> Response {
    let trace_id = resolve_trace_id();
    let envelope = SdkWorkApiResponse::success(data, trace_id.clone());
    let mut response = (status, Json(envelope)).into_response();
    attach_trace_header(&mut response, &trace_id);
    response
}

/// Build a success response wrapping `data` in the canonical
/// `SdkWorkApiResponse` envelope (`API_SPEC.md` §15.1).
///
/// The domain object is wrapped in `SdkWorkResourceData { item }` so the
/// wire body is `{ "code": 0, "data": { "item": <payload> }, "traceId": "..." }`.
pub fn success_json_response<T: Serialize>(status: StatusCode, data: T) -> Response {
    let trace_id = resolve_trace_id();
    let envelope =
        SdkWorkApiResponse::success(SdkWorkResourceData { item: data }, trace_id.clone());
    let mut response = (status, Json(envelope)).into_response();
    attach_trace_header(&mut response, &trace_id);
    response
}

/// Build a success command response with `SdkWorkCommandData` directly in
/// `SdkWorkApiResponse.data`.
pub fn success_command_json_response(status: StatusCode, data: SdkWorkCommandData) -> Response {
    let trace_id = resolve_trace_id();
    let envelope = SdkWorkApiResponse::success(data, trace_id.clone());
    let mut response = (status, Json(envelope)).into_response();
    attach_trace_header(&mut response, &trace_id);
    response
}

/// Build a 204 No Content response with the trace header.
pub fn no_content_response() -> Response {
    let trace_id = resolve_trace_id();
    let mut response = StatusCode::NO_CONTENT.into_response();
    attach_trace_header(&mut response, &trace_id);
    response
}

fn attach_trace_header(response: &mut Response, trace_id: &str) {
    if let Ok(value) = HeaderValue::from_str(trace_id) {
        if let Ok(name) = HeaderName::try_from(SDKWORK_TRACE_ID_HEADER) {
            response.headers_mut().insert(name, value);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::body::to_bytes;

    #[tokio::test]
    async fn enrich_problem_trace_id_uses_scoped_request_id() {
        let problem = ProblemDetails::from_status(
            StatusCode::BAD_REQUEST,
            "invalid_request",
            "invalid",
            String::new(),
        );

        let enriched = request_id_scope("corr-123".to_string(), async {
            enrich_problem_trace_id(problem)
        })
        .await;

        assert_eq!(enriched.trace_id, "corr-123");
    }

    #[tokio::test]
    async fn success_list_json_response_wraps_page_data_directly() {
        use sdkwork_utils_rust::{PageInfo, PageMode, SdkWorkPageData};

        let page = SdkWorkPageData {
            items: vec![serde_json::json!({ "id": 1 })],
            page_info: PageInfo {
                mode: PageMode::Cursor,
                page: None,
                page_size: Some(20),
                total_items: None,
                total_pages: None,
                next_cursor: None,
                has_more: Some(false),
            },
        };
        let response = success_list_json_response(StatusCode::OK, page);
        let body = to_bytes(response.into_body(), usize::MAX)
            .await
            .expect("body");
        let payload: serde_json::Value = serde_json::from_slice(&body).expect("json");
        assert_eq!(0, payload["code"].as_i64().unwrap());
        assert_eq!(1, payload["data"]["items"][0]["id"].as_i64().unwrap());
        assert_eq!(
            "cursor",
            payload["data"]["pageInfo"]["mode"].as_str().unwrap()
        );
        assert!(payload.get("data").unwrap().get("item").is_none());
        assert!(payload["traceId"].as_str().is_some());
    }

    #[tokio::test]
    async fn success_browser_list_json_response_preserves_view_context() {
        use sdkwork_knowledgebase_contract::{
            KnowledgeBrowserListData, KnowledgeBrowserNode, KnowledgeBrowserView,
        };
        use sdkwork_utils_rust::{PageInfo, PageMode};

        let page = KnowledgeBrowserListData {
            space_id: 7,
            drive_space_id: "drv-kb-001".to_string(),
            parent_id: Some("node-raw".to_string()),
            view: KnowledgeBrowserView::Files,
            page_size: 20,
            items: Vec::<KnowledgeBrowserNode>::new(),
            page_info: PageInfo {
                mode: PageMode::Cursor,
                page: None,
                page_size: Some(20),
                total_items: None,
                total_pages: None,
                next_cursor: Some("next-1".to_string()),
                has_more: Some(true),
            },
        };

        let response = success_browser_list_json_response(StatusCode::OK, page);
        let body = to_bytes(response.into_body(), usize::MAX)
            .await
            .expect("body");
        let payload: serde_json::Value = serde_json::from_slice(&body).expect("json");

        assert_eq!(0, payload["code"].as_i64().unwrap());
        assert_eq!(7, payload["data"]["spaceId"].as_u64().unwrap());
        assert_eq!(
            "drv-kb-001",
            payload["data"]["driveSpaceId"].as_str().unwrap()
        );
        assert_eq!("node-raw", payload["data"]["parentId"].as_str().unwrap());
        assert_eq!("files", payload["data"]["view"].as_str().unwrap());
        assert_eq!(20, payload["data"]["pageSize"].as_u64().unwrap());
        assert!(payload["data"]["items"].as_array().unwrap().is_empty());
        assert_eq!(
            "cursor",
            payload["data"]["pageInfo"]["mode"].as_str().unwrap()
        );
        assert_eq!(
            "next-1",
            payload["data"]["pageInfo"]["nextCursor"].as_str().unwrap()
        );
        assert!(payload["data"].get("item").is_none());
    }

    #[tokio::test]
    async fn success_json_response_uses_envelope() {
        let response = success_json_response(StatusCode::OK, serde_json::json!({ "id": 42 }));
        let body = to_bytes(response.into_body(), usize::MAX)
            .await
            .expect("body");
        let payload: serde_json::Value = serde_json::from_slice(&body).expect("json");
        assert_eq!(0, payload["code"].as_i64().unwrap());
        assert_eq!(42, payload["data"]["item"]["id"].as_i64().unwrap());
        assert!(payload["traceId"].as_str().is_some());
    }

    #[tokio::test]
    async fn success_command_json_response_wraps_command_data_directly() {
        let response =
            success_command_json_response(StatusCode::OK, SdkWorkCommandData::accepted());
        let body = to_bytes(response.into_body(), usize::MAX)
            .await
            .expect("body");
        let payload: serde_json::Value = serde_json::from_slice(&body).expect("json");
        assert_eq!(0, payload["code"].as_i64().unwrap());
        assert!(payload["data"]["accepted"].as_bool().unwrap());
        assert!(payload["data"].get("item").is_none());
        assert!(payload["traceId"].as_str().is_some());
    }

    #[tokio::test]
    async fn problem_json_response_uses_numeric_code_and_trace() {
        let problem = ProblemDetails::from_status(
            StatusCode::NOT_FOUND,
            "knowledge_space_not_found",
            "space was not found",
            String::new(),
        );

        let response = problem_json_response(StatusCode::NOT_FOUND, problem);
        assert_eq!(
            "application/problem+json",
            response
                .headers()
                .get(header::CONTENT_TYPE)
                .and_then(|v| v.to_str().ok())
                .unwrap_or_default()
        );
        let body = to_bytes(response.into_body(), usize::MAX)
            .await
            .expect("body");
        let payload: serde_json::Value = serde_json::from_slice(&body).expect("json");
        assert_eq!(40401, payload["code"].as_i64().unwrap());
        assert_eq!(404, payload["status"].as_u64().unwrap());
        assert!(payload["traceId"].as_str().is_some());
    }

    #[tokio::test]
    async fn no_content_response_has_no_body() {
        let response = no_content_response();
        assert_eq!(StatusCode::NO_CONTENT, response.status());
        let body = to_bytes(response.into_body(), usize::MAX)
            .await
            .expect("body");
        assert!(body.is_empty());
    }
}
