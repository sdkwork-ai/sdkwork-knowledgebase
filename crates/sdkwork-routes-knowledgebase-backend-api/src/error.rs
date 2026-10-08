use axum::{
    http::StatusCode,
    response::{IntoResponse, Response},
};
use sdkwork_knowledgebase_contract::ProblemDetails;

pub type BackendApiResult<T> = Result<T, BackendApiError>;

const INTERNAL_CLIENT_DETAIL: &str = "An internal error occurred. Please try again later.";

#[derive(Debug, Clone)]
pub struct BackendApiError {
    status: StatusCode,
    code: String,
    detail: String,
}

impl BackendApiError {
    pub fn new(status: StatusCode, code: impl Into<String>, detail: impl Into<String>) -> Self {
        Self {
            status,
            code: code.into(),
            detail: detail.into(),
        }
    }

    pub fn unsupported_operation(operation_id: &'static str) -> Self {
        Self::new(
            StatusCode::NOT_IMPLEMENTED,
            "operation_unsupported",
            format!("operation is unsupported by this runtime: {operation_id}"),
        )
    }

    pub fn internal(code: impl Into<String>, detail: impl Into<String>) -> Self {
        Self::sanitized_internal(code, detail)
    }

    pub fn sanitized_internal(code: impl Into<String>, internal_detail: impl Into<String>) -> Self {
        let code_value = code.into();
        tracing::error!(
            code = %code_value,
            error = %internal_detail.into(),
            "internal error"
        );
        Self::new(
            StatusCode::INTERNAL_SERVER_ERROR,
            code_value,
            INTERNAL_CLIENT_DETAIL,
        )
    }
}

#[derive(Debug, Clone)]
pub struct BackendApiProblem {
    status: StatusCode,
    problem: Box<ProblemDetails>,
}

impl BackendApiProblem {
    pub fn new(status: StatusCode, code: impl Into<String>, detail: impl Into<String>) -> Self {
        let client_detail = if status.is_server_error() {
            INTERNAL_CLIENT_DETAIL.to_string()
        } else {
            detail.into()
        };
        Self {
            status,
            problem: Box::new(ProblemDetails::pending_trace(status, code, client_detail)),
        }
    }

    pub fn from_internal(code: impl Into<String>, internal_detail: impl Into<String>) -> Self {
        Self::from(BackendApiError::sanitized_internal(code, internal_detail))
    }
}

impl From<BackendApiError> for BackendApiProblem {
    fn from(error: BackendApiError) -> Self {
        Self::new(error.status, error.code, error.detail)
    }
}

impl IntoResponse for BackendApiProblem {
    fn into_response(self) -> Response {
        sdkwork_knowledgebase_observability::request_correlation::problem_json_response(
            self.status,
            *self.problem,
        )
    }
}
