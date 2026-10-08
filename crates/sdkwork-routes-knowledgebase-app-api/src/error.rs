use axum::{
    http::StatusCode,
    response::{IntoResponse, Response},
};
use sdkwork_intelligence_knowledgebase_service::ports::knowledge_drive_storage::KnowledgeStorageError;
use sdkwork_intelligence_knowledgebase_service::{
    agent::KnowledgeAgentServiceError,
    agent_chat::KnowledgeAgentChatServiceError,
    browser::KnowledgeBrowserServiceError,
    context_binding::KnowledgeContextBindingServiceError,
    imports::{KnowledgeDriveImportServiceError, KnowledgeGitImportServiceError},
    ingest::{
        ApiMarkdownIngestPipelineError, KnowledgeApiMarkdownIndexServiceError,
        KnowledgeApiPayloadIngestServiceError, KnowledgeIngestionServiceError,
    },
    okf::OkfConceptServiceError,
    ports::{
        knowledge_agent_profile_store::KnowledgeAgentProfileStoreError,
        knowledge_access_control::KnowledgeAccessControlError,
        knowledge_context_binding_store::KnowledgeContextBindingStoreError,
        knowledge_document_store::KnowledgeDocumentStoreError,
        knowledge_ingestion_job_store::IngestionJobStoreError,
        knowledge_memory_context::KnowledgeMemoryContextProviderError,
        knowledge_retrieval_backend::KnowledgeRetrievalBackendError,
        knowledge_retrieval_trace_store::KnowledgeRetrievalTraceStoreError,
        knowledge_source_store::KnowledgeSourceStoreError,
        knowledge_space_store::KnowledgeSpaceStoreError,
    },
    retrieval::KnowledgeRetrievalServiceError,
    space::KnowledgeSpaceServiceError,
    wechat::KnowledgeWechatServiceError,
    wechat::WechatCallbackError,
};
use sdkwork_intelligence_knowledgebase_service::{
    group_space_access::GroupKnowledgeSpaceAccessAuthorizerError,
    ports::knowledge_group_space_binding_store::KnowledgeGroupSpaceBindingStoreError,
};
use sdkwork_knowledgebase_contract::ProblemDetails;

pub type ApiResult<T> = Result<T, ApiError>;

const INTERNAL_CLIENT_DETAIL: &str = "An internal error occurred. Please try again later.";

#[derive(Debug, Clone)]
pub struct ApiError {
    status: StatusCode,
    code: String,
    detail: String,
}

impl ApiError {
    pub fn new(status: StatusCode, code: impl Into<String>, detail: impl Into<String>) -> Self {
        Self {
            status,
            code: code.into(),
            detail: detail.into(),
        }
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

    fn gateway_timeout(code: impl Into<String>, internal_detail: impl Into<String>) -> Self {
        let code_value = code.into();
        tracing::error!(
            code = %code_value,
            error = %internal_detail.into(),
            "gateway timeout"
        );
        Self::new(
            StatusCode::GATEWAY_TIMEOUT,
            code_value,
            INTERNAL_CLIENT_DETAIL,
        )
    }

    fn service_unavailable(code: impl Into<String>, internal_detail: impl Into<String>) -> Self {
        let code_value = code.into();
        tracing::error!(
            code = %code_value,
            error = %internal_detail.into(),
            "service unavailable"
        );
        Self::new(
            StatusCode::SERVICE_UNAVAILABLE,
            code_value,
            INTERNAL_CLIENT_DETAIL,
        )
    }

    fn bad_gateway(code: impl Into<String>, internal_detail: impl Into<String>) -> Self {
        let code_value = code.into();
        tracing::error!(
            code = %code_value,
            error = %internal_detail.into(),
            "bad gateway"
        );
        Self::new(StatusCode::BAD_GATEWAY, code_value, INTERNAL_CLIENT_DETAIL)
    }

    pub fn invalid_request(code: impl Into<String>, detail: impl Into<String>) -> Self {
        Self::new(StatusCode::BAD_REQUEST, code, detail)
    }

    pub fn unauthorized(code: impl Into<String>, detail: impl Into<String>) -> Self {
        Self::new(StatusCode::UNAUTHORIZED, code, detail)
    }

    pub fn forbidden(code: impl Into<String>, detail: impl Into<String>) -> Self {
        Self::new(StatusCode::FORBIDDEN, code, detail)
    }

    pub fn not_found(code: impl Into<String>, detail: impl Into<String>) -> Self {
        Self::new(StatusCode::NOT_FOUND, code, detail)
    }

    pub fn conflict(code: impl Into<String>, detail: impl Into<String>) -> Self {
        Self::new(StatusCode::CONFLICT, code, detail)
    }

    pub fn quota_exceeded(detail: impl Into<String>) -> Self {
        Self::new(
            StatusCode::TOO_MANY_REQUESTS,
            "knowledge_tenant_quota_exceeded",
            detail,
        )
    }

    pub fn unsupported_operation(operation_id: &'static str) -> Self {
        Self::new(
            StatusCode::NOT_IMPLEMENTED,
            "operation_unsupported",
            format!("operation is unsupported by this runtime: {operation_id}"),
        )
    }

    pub fn to_open_api_error(self) -> sdkwork_routes_knowledgebase_open_api::ApiError {
        sdkwork_routes_knowledgebase_open_api::ApiError::new(self.status, self.code, self.detail)
    }

    pub fn to_backend_api_error(self) -> sdkwork_routes_knowledgebase_backend_api::BackendApiError {
        sdkwork_routes_knowledgebase_backend_api::BackendApiError::new(
            self.status,
            self.code,
            self.detail,
        )
    }
}

#[derive(Debug, Clone)]
pub struct ApiProblem {
    status: StatusCode,
    problem: Box<ProblemDetails>,
}

impl ApiProblem {
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
        Self::from(ApiError::sanitized_internal(code, internal_detail))
    }
}

impl From<ApiError> for ApiProblem {
    fn from(error: ApiError) -> Self {
        Self::new(error.status, error.code, error.detail)
    }
}

impl IntoResponse for ApiProblem {
    fn into_response(self) -> Response {
        sdkwork_knowledgebase_observability::request_correlation::problem_json_response(
            self.status,
            *self.problem,
        )
    }
}

impl From<KnowledgeRetrievalServiceError> for ApiError {
    fn from(error: KnowledgeRetrievalServiceError) -> Self {
        match error {
            KnowledgeRetrievalServiceError::InvalidRequest(detail) => {
                Self::invalid_request("invalid_knowledge_retrieval_request", detail)
            }
            KnowledgeRetrievalServiceError::Backend(
                KnowledgeRetrievalBackendError::TenantMismatch,
            ) => Self::new(
                StatusCode::FORBIDDEN,
                "tenant_id_mismatch",
                "retrieval tenantId does not match backend tenant scope",
            ),
            KnowledgeRetrievalServiceError::Backend(
                KnowledgeRetrievalBackendError::UnsupportedMethod(method),
            ) => Self::invalid_request(
                "unsupported_retrieval_method",
                format!("retrieval method is not supported by the configured backend: {method:?}"),
            ),
            KnowledgeRetrievalServiceError::Backend(
                KnowledgeRetrievalBackendError::QueueSaturated { capacity },
            ) => Self::service_unavailable(
                "knowledge_retrieval_queue_saturated",
                format!("knowledge retrieval embedding queue is saturated at capacity {capacity}"),
            ),
            KnowledgeRetrievalServiceError::Backend(KnowledgeRetrievalBackendError::TimedOut {
                timeout,
            }) => Self::gateway_timeout(
                "knowledge_retrieval_timed_out",
                format!("knowledge retrieval embedding timed out after {timeout:?}"),
            ),
            KnowledgeRetrievalServiceError::TraceStore(
                KnowledgeRetrievalTraceStoreError::NotFound(retrieval_id),
            ) => Self::not_found(
                "knowledge_retrieval_not_found",
                format!("knowledge retrieval trace was not found: {retrieval_id}"),
            ),
            KnowledgeRetrievalServiceError::Backend(KnowledgeRetrievalBackendError::Internal(
                detail,
            ))
            | KnowledgeRetrievalServiceError::MemoryProvider(
                KnowledgeMemoryContextProviderError::Upstream(detail)
                | KnowledgeMemoryContextProviderError::Internal(detail),
            )
            | KnowledgeRetrievalServiceError::TraceStore(
                KnowledgeRetrievalTraceStoreError::Internal(detail),
            ) => Self::internal("knowledge_retrieval_failed", detail),
            KnowledgeRetrievalServiceError::MemoryProvider(
                KnowledgeMemoryContextProviderError::InvalidRequest(detail),
            ) => Self::invalid_request("invalid_knowledge_memory_context_request", detail),
        }
    }
}

impl From<KnowledgeAgentServiceError> for ApiError {
    fn from(error: KnowledgeAgentServiceError) -> Self {
        match error {
            KnowledgeAgentServiceError::InvalidRequest(detail) => {
                Self::invalid_request("invalid_knowledge_agent_request", detail)
            }
            KnowledgeAgentServiceError::Store(KnowledgeAgentProfileStoreError::NotFound(id)) => {
                Self::not_found(
                    "knowledge_agent_profile_not_found",
                    format!("knowledge agent resource was not found: {id}"),
                )
            }
            KnowledgeAgentServiceError::Store(KnowledgeAgentProfileStoreError::Conflict(
                detail,
            )) => Self::conflict("knowledge_agent_profile_conflict", detail),
            KnowledgeAgentServiceError::Store(KnowledgeAgentProfileStoreError::Unsupported(
                detail,
            )) => Self::invalid_request("knowledge_agent_profile_store_unsupported", detail),
            KnowledgeAgentServiceError::Store(KnowledgeAgentProfileStoreError::Internal(
                detail,
            )) => Self::internal("knowledge_agent_profile_store_failed", detail),
            KnowledgeAgentServiceError::Retrieval(error) => Self::from(error),
        }
    }
}

impl From<KnowledgeAgentChatServiceError> for ApiError {
    fn from(error: KnowledgeAgentChatServiceError) -> Self {
        match error {
            KnowledgeAgentChatServiceError::InvalidRequest(detail) => {
                Self::invalid_request("invalid_knowledge_agent_chat_request", detail)
            }
            KnowledgeAgentChatServiceError::QueueSaturated { capacity } => {
                Self::service_unavailable(
                    "knowledge_agent_chat_queue_saturated",
                    format!(
                        "knowledge agent chat execution queue is saturated at capacity {capacity}"
                    ),
                )
            }
            KnowledgeAgentChatServiceError::TimedOut { timeout } => Self::gateway_timeout(
                "knowledge_agent_chat_timed_out",
                format!("knowledge agent chat exceeded its {timeout:?} execution budget"),
            ),
            KnowledgeAgentChatServiceError::Retrieval(error) => Self::from(error),
            KnowledgeAgentChatServiceError::KnowledgeProvider(detail) => {
                if detail.contains("capability unsupported") {
                    Self::invalid_request("knowledge_agent_chat_provider_unsupported", detail)
                } else {
                    Self::internal("knowledge_agent_chat_provider_failed", detail)
                }
            }
            KnowledgeAgentChatServiceError::Runtime(detail) => {
                Self::internal("knowledge_agent_chat_runtime_failed", detail)
            }
            KnowledgeAgentChatServiceError::AgentKernel(detail) => {
                Self::internal("knowledge_agent_chat_kernel_failed", detail)
            }
        }
    }
}

impl From<KnowledgeSpaceServiceError> for ApiError {
    fn from(error: KnowledgeSpaceServiceError) -> Self {
        match error {
            KnowledgeSpaceServiceError::InvalidRequest(detail) => {
                Self::invalid_request("invalid_knowledge_space_request", detail)
            }
            KnowledgeSpaceServiceError::AccessDenied(detail) => Self::new(
                StatusCode::FORBIDDEN,
                "knowledge_space_access_denied",
                detail,
            ),
            KnowledgeSpaceServiceError::Store(error) => Self::from(error),
            KnowledgeSpaceServiceError::OkfBundleInitializer(error) => Self::internal(
                "knowledge_space_okf_initialization_failed",
                error.to_string(),
            ),
            KnowledgeSpaceServiceError::DriveSpaceProvisioner(error) => Self::internal(
                "knowledge_space_drive_provisioning_failed",
                error.to_string(),
            ),
            KnowledgeSpaceServiceError::AccessControl(error) => match error {
                KnowledgeAccessControlError::NotFound(message) => Self::not_found(
                    "knowledge_space_drive_anchor_missing",
                    message,
                ),
                other => Self::internal(
                    "knowledge_space_access_control_failed",
                    other.to_string(),
                ),
            },
            KnowledgeSpaceServiceError::InitializationCleanup { original, .. }
            | KnowledgeSpaceServiceError::DriveSpaceCleanup { original, .. } => {
                Self::internal("knowledge_space_initialization_failed", original)
            }
        }
    }
}

impl From<KnowledgeSpaceStoreError> for ApiError {
    fn from(error: KnowledgeSpaceStoreError) -> Self {
        match error {
            KnowledgeSpaceStoreError::Conflict(detail) => {
                Self::conflict("knowledge_space_conflict", detail)
            }
            KnowledgeSpaceStoreError::NotFound(detail) => {
                Self::not_found("knowledge_space_not_found", detail)
            }
            KnowledgeSpaceStoreError::Internal(detail) => {
                Self::internal("knowledge_space_store_failed", detail)
            }
        }
    }
}

impl From<KnowledgeIngestionServiceError> for ApiError {
    fn from(error: KnowledgeIngestionServiceError) -> Self {
        match error {
            KnowledgeIngestionServiceError::InvalidRequest(detail) => {
                Self::invalid_request("invalid_ingestion_job_request", detail)
            }
            KnowledgeIngestionServiceError::InvalidTransition { from, to } => Self::conflict(
                "invalid_ingestion_job_transition",
                format!("invalid ingestion job transition: {from:?} -> {to:?}"),
            ),
            KnowledgeIngestionServiceError::Store(error) => Self::from(error),
        }
    }
}

impl From<KnowledgeApiPayloadIngestServiceError> for ApiError {
    fn from(error: KnowledgeApiPayloadIngestServiceError) -> Self {
        match error {
            KnowledgeApiPayloadIngestServiceError::InvalidRequest(detail) => {
                Self::invalid_request("invalid_knowledge_ingest_request", detail)
            }
            KnowledgeApiPayloadIngestServiceError::WebLink(error) => match error {
                sdkwork_intelligence_knowledgebase_service::ingest::WebLinkFetchError::InvalidRequest(
                    detail,
                ) => Self::invalid_request("invalid_knowledge_ingest_source_url", detail),
                sdkwork_intelligence_knowledgebase_service::ingest::WebLinkFetchError::Upstream(
                    detail,
                ) => Self::internal("knowledge_ingest_source_url_fetch_failed", detail),
            },
            KnowledgeApiPayloadIngestServiceError::Store(error) => Self::from(error),
            KnowledgeApiPayloadIngestServiceError::Storage(error) => {
                Self::internal("knowledge_ingest_drive_failed", error.to_string())
            }
        }
    }
}

impl From<IngestionJobStoreError> for ApiError {
    fn from(error: IngestionJobStoreError) -> Self {
        match error {
            IngestionJobStoreError::NotFound(job_id) => Self::not_found(
                "ingestion_job_not_found",
                format!("ingestion job was not found: {job_id}"),
            ),
            IngestionJobStoreError::Conflict(detail) => {
                Self::conflict("ingestion_job_conflict", detail)
            }
            IngestionJobStoreError::QuotaExceeded(error) => {
                crate::tenant_quota_enforcement::map_tenant_quota_error(error)
            }
            IngestionJobStoreError::Internal(detail) => {
                Self::internal("ingestion_job_store_failed", detail)
            }
        }
    }
}

impl From<KnowledgeWechatServiceError> for ApiError {
    fn from(error: KnowledgeWechatServiceError) -> Self {
        match error {
            KnowledgeWechatServiceError::InvalidRequest(detail) => {
                Self::invalid_request("invalid_wechat_request", detail)
            }
            KnowledgeWechatServiceError::UnsupportedOperation(operation_id) => {
                Self::unsupported_operation(operation_id)
            }
            KnowledgeWechatServiceError::Storage(error) => error.into(),
            KnowledgeWechatServiceError::Api(error) => {
                Self::internal("wechat_upstream_failed", error.to_string())
            }
        }
    }
}

impl From<WechatCallbackError> for ApiError {
    fn from(error: WechatCallbackError) -> Self {
        match error {
            WechatCallbackError::InvalidRequest(detail) => {
                // Detail strings are validation messages only; they never include
                // credential material or decrypted payload content.
                Self::invalid_request("invalid_wechat_callback", detail)
            }
            // A failed signature or receive-id check is an authentication failure for
            // an unauthenticated external caller; WeChat retries non-2xx deliveries.
            WechatCallbackError::VerificationFailed => Self::unauthorized(
                "wechat_callback_verification_failed",
                "wechat callback signature verification failed",
            ),
            WechatCallbackError::NotConfigured(detail) => {
                Self::invalid_request("wechat_callback_not_configured", detail)
            }
            WechatCallbackError::Crypto(error) => {
                Self::invalid_request("invalid_wechat_callback_payload", error.to_string())
            }
            WechatCallbackError::Storage(error) => error.into(),
        }
    }
}

impl From<KnowledgeDriveImportServiceError> for ApiError {
    fn from(error: KnowledgeDriveImportServiceError) -> Self {
        match error {
            KnowledgeDriveImportServiceError::InvalidRequest(detail) => {
                Self::invalid_request("invalid_knowledge_drive_import_request", detail)
            }
            KnowledgeDriveImportServiceError::Storage(error) => Self::from(error),
            KnowledgeDriveImportServiceError::Metadata(error) => match error {
                sdkwork_intelligence_knowledgebase_service::ports::drive_import_metadata_store::DriveImportMetadataStoreError::InvalidRequest(detail) => {
                    Self::invalid_request("invalid_knowledge_drive_import_request", detail)
                }
                sdkwork_intelligence_knowledgebase_service::ports::drive_import_metadata_store::DriveImportMetadataStoreError::Conflict(detail) => {
                    Self::conflict("knowledge_drive_import_conflict", detail)
                }
                sdkwork_intelligence_knowledgebase_service::ports::drive_import_metadata_store::DriveImportMetadataStoreError::QuotaExceeded(error) => {
                    crate::tenant_quota_enforcement::map_tenant_quota_error(error)
                }
                sdkwork_intelligence_knowledgebase_service::ports::drive_import_metadata_store::DriveImportMetadataStoreError::Internal(detail) => {
                    Self::internal("drive_import_metadata_store_failed", detail)
                }
            },
        }
    }
}

impl From<ApiMarkdownIngestPipelineError> for ApiError {
    fn from(error: ApiMarkdownIngestPipelineError) -> Self {
        match error {
            ApiMarkdownIngestPipelineError::Payload(error) => error.into(),
            ApiMarkdownIngestPipelineError::Ingestion(error) => error.into(),
            ApiMarkdownIngestPipelineError::Index(error) => error.into(),
            ApiMarkdownIngestPipelineError::Store(error) => error.into(),
            ApiMarkdownIngestPipelineError::Storage(error) => error.into(),
        }
    }
}

impl From<KnowledgeGitImportServiceError> for ApiError {
    fn from(error: KnowledgeGitImportServiceError) -> Self {
        match error {
            KnowledgeGitImportServiceError::InvalidRequest(detail) => {
                Self::invalid_request("invalid_knowledge_git_import_request", detail)
            }
            KnowledgeGitImportServiceError::GitHub(
                sdkwork_intelligence_knowledgebase_service::imports::GitHubApiError::InvalidRequest(
                    detail,
                ),
            ) => Self::invalid_request("invalid_knowledge_git_import_request", detail),
            KnowledgeGitImportServiceError::GitHub(error) => {
                Self::internal("knowledge_git_import_upstream_failed", error.to_string())
            }
            KnowledgeGitImportServiceError::Pipeline(error) => error.into(),
        }
    }
}

impl From<sdkwork_intelligence_knowledgebase_service::imports::KnowledgeGitSyncServiceError>
    for ApiError
{
    fn from(
        error: sdkwork_intelligence_knowledgebase_service::imports::KnowledgeGitSyncServiceError,
    ) -> Self {
        use sdkwork_intelligence_knowledgebase_service::imports::KnowledgeGitSyncServiceError;
        use sdkwork_intelligence_knowledgebase_service::ports::knowledge_document_store::KnowledgeDocumentStoreError;
        match error {
            KnowledgeGitSyncServiceError::InvalidRequest(detail) => {
                Self::invalid_request("invalid_knowledge_git_sync_request", detail)
            }
            KnowledgeGitSyncServiceError::GitHub(
                sdkwork_intelligence_knowledgebase_service::imports::GitHubApiError::InvalidRequest(
                    detail,
                ),
            ) => Self::invalid_request("invalid_knowledge_git_sync_request", detail),
            KnowledgeGitSyncServiceError::GitHub(error) => {
                Self::internal("knowledge_git_sync_upstream_failed", error.to_string())
            }
            KnowledgeGitSyncServiceError::DocumentContent(detail) => {
                Self::invalid_request("invalid_knowledge_git_sync_request", detail)
            }
            KnowledgeGitSyncServiceError::DocumentStore(
                KnowledgeDocumentStoreError::InvalidRecord(detail),
            ) => Self::invalid_request("invalid_knowledge_git_sync_request", detail),
            KnowledgeGitSyncServiceError::DocumentStore(error) => Self::internal(
                "knowledge_git_sync_document_store_failed",
                error.to_string(),
            ),
        }
    }
}

impl From<KnowledgeBrowserServiceError> for ApiError {
    fn from(error: KnowledgeBrowserServiceError) -> Self {
        match error {
            KnowledgeBrowserServiceError::InvalidRequest(detail) => {
                Self::invalid_request("invalid_knowledge_browser_request", detail)
            }
            KnowledgeBrowserServiceError::AccessDenied(detail) => Self::new(
                axum::http::StatusCode::FORBIDDEN,
                "knowledge_browser_access_denied",
                detail,
            ),
            KnowledgeBrowserServiceError::AccessControlNotConfigured => Self::internal(
                "knowledge_browser_access_control_not_configured",
                "knowledge browser access control is not configured",
            ),
            KnowledgeBrowserServiceError::SpaceStore(error) => Self::from(error),
            KnowledgeBrowserServiceError::DriveTree(error) => {
                Self::internal("knowledge_browser_drive_tree_failed", error.to_string())
            }
            KnowledgeBrowserServiceError::ProjectionStore(error) => {
                Self::internal("knowledge_browser_projection_failed", error.to_string())
            }
            KnowledgeBrowserServiceError::AccessControl(error) => match error {
                KnowledgeAccessControlError::NotFound(message) => Self::not_found(
                    "knowledge_browser_drive_anchor_missing",
                    message,
                ),
                other => Self::internal(
                    "knowledge_browser_access_control_failed",
                    other.to_string(),
                ),
            },
        }
    }
}

impl From<KnowledgeSourceStoreError> for ApiError {
    fn from(error: KnowledgeSourceStoreError) -> Self {
        match error {
            KnowledgeSourceStoreError::Unsupported(detail) => {
                Self::internal("knowledge_source_store_unsupported", detail)
            }
            KnowledgeSourceStoreError::Internal(detail) => {
                Self::internal("knowledge_source_store_failed", detail)
            }
        }
    }
}

impl From<KnowledgeDocumentStoreError> for ApiError {
    fn from(error: KnowledgeDocumentStoreError) -> Self {
        match error {
            KnowledgeDocumentStoreError::InvalidRecord(detail) => {
                Self::invalid_request("invalid_knowledge_document_record", detail)
            }
            KnowledgeDocumentStoreError::Unsupported(detail) => {
                Self::invalid_request("knowledge_document_store_unsupported", detail)
            }
            KnowledgeDocumentStoreError::QuotaExceeded(error) => {
                crate::tenant_quota_enforcement::map_tenant_quota_error(error)
            }
            KnowledgeDocumentStoreError::NotFound(detail) => {
                Self::not_found("knowledge_document_not_found", detail)
            }
            KnowledgeDocumentStoreError::Internal(detail) => {
                Self::internal("knowledge_document_store_failed", detail)
            }
        }
    }
}

impl From<KnowledgeStorageError> for ApiError {
    fn from(error: KnowledgeStorageError) -> Self {
        match error {
            KnowledgeStorageError::NotFound(detail) => {
                Self::not_found("knowledge_storage_not_found", detail)
            }
            KnowledgeStorageError::InvalidRequest(detail) => {
                Self::invalid_request("invalid_knowledge_storage_request", detail)
            }
            KnowledgeStorageError::IntegrityFailed(detail) => {
                Self::internal("knowledge_storage_integrity_failed", detail)
            }
            KnowledgeStorageError::PermissionDenied(detail) => {
                Self::forbidden("knowledge_storage_permission_denied", detail)
            }
            KnowledgeStorageError::Conflict(detail) => {
                Self::conflict("knowledge_storage_conflict", detail)
            }
            KnowledgeStorageError::Upstream(detail) => {
                Self::internal("knowledge_storage_upstream_failed", detail)
            }
            KnowledgeStorageError::Internal(detail) => {
                Self::internal("knowledge_storage_failed", detail)
            }
        }
    }
}

impl From<sdkwork_knowledgebase_observability::AuditPersistenceError> for ApiError {
    fn from(error: sdkwork_knowledgebase_observability::AuditPersistenceError) -> Self {
        Self::internal("knowledge_audit_persistence_failed", error.to_string())
    }
}

impl From<sdkwork_intelligence_knowledgebase_service::okf::OkfBundleWorkflowError> for ApiError {
    fn from(
        error: sdkwork_intelligence_knowledgebase_service::okf::OkfBundleWorkflowError,
    ) -> Self {
        use sdkwork_intelligence_knowledgebase_service::okf::OkfBundleWorkflowError;
        match error {
            OkfBundleWorkflowError::InvalidRequest(detail) => {
                Self::invalid_request("okf_bundle_workflow_invalid_request", detail)
            }
            OkfBundleWorkflowError::SpaceStore(store_error) => store_error.into(),
            OkfBundleWorkflowError::ConceptStore(store_error) => {
                Self::internal("okf_bundle_workflow_failed", store_error.to_string())
            }
            OkfBundleWorkflowError::SourceStore(store_error) => store_error.into(),
            OkfBundleWorkflowError::IndexRebuild(rebuild_error) => {
                Self::internal("okf_bundle_workflow_failed", rebuild_error.to_string())
            }
            OkfBundleWorkflowError::Linter(linter_error) => {
                Self::internal("okf_bundle_workflow_failed", linter_error.to_string())
            }
            OkfBundleWorkflowError::Storage(storage_error) => storage_error.into(),
            OkfBundleWorkflowError::BundleFileStore(store_error) => {
                Self::internal("okf_bundle_workflow_failed", store_error.to_string())
            }
            OkfBundleWorkflowError::BundleFileRegistry(registry_error) => {
                Self::internal("okf_bundle_workflow_failed", registry_error.to_string())
            }
            OkfBundleWorkflowError::CatalogSync(catalog_sync_error) => {
                use sdkwork_intelligence_knowledgebase_service::okf::StandardBundleCatalogSyncError;
                match catalog_sync_error {
                    StandardBundleCatalogSyncError::Registry(registry_error) => {
                        Self::internal("okf_bundle_workflow_failed", registry_error.to_string())
                    }
                    StandardBundleCatalogSyncError::DriveWorkspace(workspace_error) => {
                        Self::internal("okf_bundle_workflow_failed", workspace_error.to_string())
                    }
                }
            }
            OkfBundleWorkflowError::Engine(engine_error) => engine_error.into(),
        }
    }
}

impl From<sdkwork_knowledgebase_contract::knowledge_engine::KnowledgeEngineError> for ApiError {
    fn from(error: sdkwork_knowledgebase_contract::knowledge_engine::KnowledgeEngineError) -> Self {
        use sdkwork_knowledgebase_contract::knowledge_engine::KnowledgeEngineError;
        match error {
            KnowledgeEngineError::Validation(detail) => {
                Self::invalid_request("knowledge_engine_validation_failed", detail)
            }
            KnowledgeEngineError::NotFound(detail) => {
                Self::not_found("knowledge_engine_not_found", detail)
            }
            KnowledgeEngineError::Unsupported(detail) => {
                Self::invalid_request("knowledge_engine_unsupported", detail)
            }
            KnowledgeEngineError::PermissionDenied(detail) => {
                Self::forbidden("knowledge_engine_permission_denied", detail)
            }
            KnowledgeEngineError::Provider(failure) => {
                use sdkwork_knowledgebase_contract::knowledge_engine::KnowledgeEngineProviderErrorCategory as Category;
                let detail = format!(
                    "implementation_id={} binding_id={:?} operation={} category={:?} status={:?}: {}",
                    failure.implementation_id,
                    failure.binding_id,
                    failure.operation,
                    failure.category,
                    failure.status_code,
                    failure.safe_message
                );
                match failure.category {
                    Category::NotFound => Self::not_found(
                        "knowledge_provider_resource_not_found",
                        failure.safe_message,
                    ),
                    Category::Validation | Category::Unsupported => Self::invalid_request(
                        "knowledge_provider_request_invalid",
                        failure.safe_message,
                    ),
                    Category::Timeout => {
                        Self::gateway_timeout("knowledge_provider_timeout", detail)
                    }
                    Category::RateLimited
                    | Category::Unavailable
                    | Category::CircuitOpen
                    | Category::BulkheadSaturated => {
                        Self::service_unavailable("knowledge_provider_unavailable", detail)
                    }
                    Category::Authentication
                    | Category::PermissionDenied
                    | Category::InvalidResponse
                    | Category::ResponseTooLarge
                    | Category::InvalidTarget
                    | Category::Internal => {
                        Self::bad_gateway("knowledge_provider_gateway_failed", detail)
                    }
                }
            }
            KnowledgeEngineError::Internal(detail) => {
                Self::internal("knowledge_engine_failed", detail)
            }
        }
    }
}

impl From<
        sdkwork_intelligence_knowledgebase_service::provider_binding::KnowledgeEngineProviderBindingServiceError,
    > for ApiError
{
    fn from(
        error: sdkwork_intelligence_knowledgebase_service::provider_binding::KnowledgeEngineProviderBindingServiceError,
    ) -> Self {
        use sdkwork_intelligence_knowledgebase_service::{
            ports::{
                knowledge_provider_binding_store::KnowledgeEngineProviderBindingStoreError,
                knowledge_provider_credential_resolver::KnowledgeEngineProviderCredentialError,
            },
            provider_binding::KnowledgeEngineProviderBindingServiceError,
        };
        match error {
            KnowledgeEngineProviderBindingServiceError::InvalidRequest(detail) => {
                Self::invalid_request("invalid_provider_management_request", detail)
            }
            KnowledgeEngineProviderBindingServiceError::PermissionDenied(detail) => {
                Self::forbidden("provider_management_permission_denied", detail)
            }
            KnowledgeEngineProviderBindingServiceError::DeadlineExceeded => {
                Self::gateway_timeout(
                    "provider_management_deadline_exceeded",
                    "Provider management deadline exceeded",
                )
            }
            KnowledgeEngineProviderBindingServiceError::InvalidLifecycle(detail) => {
                Self::conflict("provider_lifecycle_conflict", detail)
            }
            KnowledgeEngineProviderBindingServiceError::Store(store_error) => match store_error {
                KnowledgeEngineProviderBindingStoreError::InvalidRequest(detail) => {
                    Self::invalid_request("invalid_provider_management_request", detail)
                }
                KnowledgeEngineProviderBindingStoreError::NotFound(resource_id) => {
                    Self::not_found(
                        "provider_management_resource_not_found",
                        format!("Provider management resource was not found: {resource_id}"),
                    )
                }
                KnowledgeEngineProviderBindingStoreError::Conflict(detail) => {
                    Self::conflict("provider_version_conflict", detail)
                }
                KnowledgeEngineProviderBindingStoreError::InvalidLifecycle(detail) => {
                    Self::conflict("provider_lifecycle_conflict", detail)
                }
                KnowledgeEngineProviderBindingStoreError::CredentialUnavailable(reference_id) => {
                    Self::service_unavailable(
                        "provider_credential_unavailable",
                        format!("Provider credential reference is unavailable: {reference_id}"),
                    )
                }
                KnowledgeEngineProviderBindingStoreError::Internal(detail) => {
                    Self::internal("provider_management_store_failed", detail)
                }
            },
            KnowledgeEngineProviderBindingServiceError::Engine(engine_error) => {
                engine_error.into()
            }
            KnowledgeEngineProviderBindingServiceError::Credential(credential_error) => {
                match credential_error {
                    KnowledgeEngineProviderCredentialError::InvalidReference => {
                        Self::invalid_request(
                            "invalid_provider_credential_reference",
                            "Provider credential reference is invalid",
                        )
                    }
                    KnowledgeEngineProviderCredentialError::AccessDenied => Self::forbidden(
                        "provider_credential_access_denied",
                        "Provider credential access is denied",
                    ),
                    KnowledgeEngineProviderCredentialError::Unavailable => {
                        Self::service_unavailable(
                            "provider_credential_unavailable",
                            "Provider credential is unavailable",
                        )
                    }
                    KnowledgeEngineProviderCredentialError::ResponseTooLarge => {
                        Self::service_unavailable(
                            "provider_credential_too_large",
                            "Provider credential exceeds the permitted size",
                        )
                    }
                    KnowledgeEngineProviderCredentialError::Internal => {
                        Self::internal(
                            "provider_credential_resolution_failed",
                            "Provider credential resolution failed",
                        )
                    }
                }
            }
            KnowledgeEngineProviderBindingServiceError::Internal(detail) => {
                Self::internal("provider_management_failed", detail)
            }
        }
    }
}

impl From<OkfConceptServiceError> for ApiError {
    fn from(error: OkfConceptServiceError) -> Self {
        match error {
            OkfConceptServiceError::InvalidRequest(detail) => {
                Self::invalid_request("invalid_knowledge_okf_concept_request", detail)
            }
            OkfConceptServiceError::RevisionMetadata(
                sdkwork_intelligence_knowledgebase_service::ports::okf_concept_revision_metadata_store::OkfConceptRevisionMetadataStoreError::QuotaExceeded(error),
            ) => crate::tenant_quota_enforcement::map_tenant_quota_error(error),
            other => Self::internal("knowledge_okf_concept_service_failed", other.to_string()),
        }
    }
}

impl From<sdkwork_intelligence_knowledgebase_service::okf::OkfBundleImporterError> for ApiError {
    fn from(
        error: sdkwork_intelligence_knowledgebase_service::okf::OkfBundleImporterError,
    ) -> Self {
        use sdkwork_intelligence_knowledgebase_service::okf::OkfBundleImporterError;
        match error {
            OkfBundleImporterError::InvalidRequest(detail) => {
                Self::invalid_request("invalid_okf_bundle_import_request", detail)
            }
            OkfBundleImporterError::Conformance(detail) => {
                Self::invalid_request("okf_bundle_import_conformance_failed", detail)
            }
            OkfBundleImporterError::Storage(storage_error) => storage_error.into(),
            OkfBundleImporterError::ConceptService(service_error) => service_error.into(),
        }
    }
}

impl From<KnowledgeContextBindingServiceError> for ApiError {
    fn from(error: KnowledgeContextBindingServiceError) -> Self {
        match error {
            KnowledgeContextBindingServiceError::InvalidRequest(detail) => {
                Self::invalid_request("invalid_knowledge_context_binding_request", detail)
            }
            KnowledgeContextBindingServiceError::Store(store_error) => store_error.into(),
            KnowledgeContextBindingServiceError::DrivePermission(detail) => Self::internal(
                "knowledge_context_binding_drive_permission_failed",
                detail.to_string(),
            ),
        }
    }
}

impl From<KnowledgeContextBindingStoreError> for ApiError {
    fn from(error: KnowledgeContextBindingStoreError) -> Self {
        match error {
            KnowledgeContextBindingStoreError::InvalidRequest(detail) => {
                Self::invalid_request("invalid_knowledge_context_binding_request", detail)
            }
            KnowledgeContextBindingStoreError::NotFound(binding_id) => Self::not_found(
                "knowledge_context_binding_not_found",
                format!("knowledge context binding was not found: {binding_id}"),
            ),
            KnowledgeContextBindingStoreError::Conflict(detail) => {
                Self::conflict("knowledge_context_binding_conflict", detail)
            }
            KnowledgeContextBindingStoreError::Internal(detail) => {
                Self::internal("knowledge_context_binding_store_failed", detail)
            }
        }
    }
}

impl From<KnowledgeGroupSpaceBindingStoreError> for ApiError {
    fn from(error: KnowledgeGroupSpaceBindingStoreError) -> Self {
        match error {
            KnowledgeGroupSpaceBindingStoreError::InvalidRequest(detail) => {
                Self::invalid_request("invalid_group_knowledge_space_request", detail)
            }
            KnowledgeGroupSpaceBindingStoreError::NotFound(identifier) => Self::not_found(
                "group_knowledge_space_not_found",
                format!("group knowledge space was not found: {identifier}"),
            ),
            KnowledgeGroupSpaceBindingStoreError::Conflict(detail) => {
                Self::conflict("group_knowledge_space_conflict", detail)
            }
            KnowledgeGroupSpaceBindingStoreError::InvalidLifecycle(detail) => {
                Self::conflict("group_knowledge_space_invalid_lifecycle", detail)
            }
            KnowledgeGroupSpaceBindingStoreError::Internal(detail) => {
                Self::internal("group_knowledge_space_store_failed", detail)
            }
        }
    }
}

impl From<GroupKnowledgeSpaceAccessAuthorizerError> for ApiError {
    fn from(error: GroupKnowledgeSpaceAccessAuthorizerError) -> Self {
        match error {
            GroupKnowledgeSpaceAccessAuthorizerError::InvalidRequest(detail) => {
                Self::invalid_request("invalid_group_knowledge_space_access_request", detail)
            }
            GroupKnowledgeSpaceAccessAuthorizerError::Denied(detail) => Self::new(
                StatusCode::FORBIDDEN,
                "group_knowledge_space_access_denied",
                detail,
            ),
            GroupKnowledgeSpaceAccessAuthorizerError::Store(error) => error.into(),
        }
    }
}

impl From<KnowledgeApiMarkdownIndexServiceError> for ApiError {
    fn from(error: KnowledgeApiMarkdownIndexServiceError) -> Self {
        match error {
            KnowledgeApiMarkdownIndexServiceError::InvalidRequest(detail) => {
                Self::invalid_request("invalid_knowledge_markdown_index_request", detail)
            }
            KnowledgeApiMarkdownIndexServiceError::Metadata(error) => match error {
                sdkwork_intelligence_knowledgebase_service::ports::markdown_index_metadata_store::MarkdownIndexMetadataStoreError::InvalidRequest(detail) => {
                    Self::invalid_request("invalid_knowledge_markdown_index_request", detail)
                }
                sdkwork_intelligence_knowledgebase_service::ports::markdown_index_metadata_store::MarkdownIndexMetadataStoreError::Conflict(detail) => {
                    Self::conflict("markdown_index_metadata_conflict", detail)
                }
                sdkwork_intelligence_knowledgebase_service::ports::markdown_index_metadata_store::MarkdownIndexMetadataStoreError::QuotaExceeded(error) => {
                    crate::tenant_quota_enforcement::map_tenant_quota_error(error)
                }
                sdkwork_intelligence_knowledgebase_service::ports::markdown_index_metadata_store::MarkdownIndexMetadataStoreError::Internal(detail) => {
                    Self::internal("markdown_index_metadata_store_failed", detail)
                }
            },
        }
    }
}

impl std::fmt::Display for ApiError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.detail)
    }
}

impl From<sdkwork_intelligence_knowledgebase_service::imports::KnowledgeDriveImportPipelineServiceError>
    for ApiError
{
    fn from(
        error: sdkwork_intelligence_knowledgebase_service::imports::KnowledgeDriveImportPipelineServiceError,
    ) -> Self {
        match error {
            sdkwork_intelligence_knowledgebase_service::imports::KnowledgeDriveImportPipelineServiceError::Ingestion(
                error,
            ) => Self::from(error),
            sdkwork_intelligence_knowledgebase_service::imports::KnowledgeDriveImportPipelineServiceError::Storage(
                error,
            ) => Self::from(error),
            sdkwork_intelligence_knowledgebase_service::imports::KnowledgeDriveImportPipelineServiceError::PayloadLimit(
                error,
            ) => match error {
                sdkwork_intelligence_knowledgebase_service::ingest::PayloadLimitError::PayloadEmpty => {
                    Self::invalid_request(
                        "invalid_knowledge_drive_import_payload",
                        "drive import payload must not be empty",
                    )
                }
                sdkwork_intelligence_knowledgebase_service::ingest::PayloadLimitError::PayloadTooLarge {
                    max_bytes,
                } => Self::invalid_request(
                    "invalid_knowledge_drive_import_payload",
                    format!("drive import payload exceeds maximum allowed size of {max_bytes} bytes"),
                ),
            },
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::body::to_bytes;
    use std::time::Duration;

    #[tokio::test]
    async fn agent_chat_timeout_maps_to_sanitized_gateway_timeout_problem() {
        assert_sanitized_problem(
            ApiError::from(KnowledgeAgentChatServiceError::TimedOut {
                timeout: Duration::from_secs(30),
            }),
            StatusCode::GATEWAY_TIMEOUT,
            50401,
        )
        .await;
    }

    #[tokio::test]
    async fn agent_chat_saturation_maps_to_sanitized_service_unavailable_problem() {
        assert_sanitized_problem(
            ApiError::from(KnowledgeAgentChatServiceError::QueueSaturated { capacity: 64 }),
            StatusCode::SERVICE_UNAVAILABLE,
            50301,
        )
        .await;
    }

    #[tokio::test]
    async fn retrieval_timeout_maps_to_sanitized_gateway_timeout_problem() {
        assert_sanitized_problem(
            ApiError::from(KnowledgeRetrievalServiceError::Backend(
                KnowledgeRetrievalBackendError::TimedOut {
                    timeout: Duration::from_secs(30),
                },
            )),
            StatusCode::GATEWAY_TIMEOUT,
            50401,
        )
        .await;
    }

    #[tokio::test]
    async fn retrieval_saturation_maps_to_sanitized_service_unavailable_problem() {
        assert_sanitized_problem(
            ApiError::from(KnowledgeRetrievalServiceError::Backend(
                KnowledgeRetrievalBackendError::QueueSaturated { capacity: 64 },
            )),
            StatusCode::SERVICE_UNAVAILABLE,
            50301,
        )
        .await;
    }

    #[tokio::test]
    async fn provider_timeout_maps_to_sanitized_gateway_timeout_problem() {
        assert_sanitized_problem(
            provider_error(
                sdkwork_knowledgebase_contract::knowledge_engine::KnowledgeEngineProviderErrorCategory::Timeout,
            ),
            StatusCode::GATEWAY_TIMEOUT,
            50401,
        )
        .await;
    }

    #[tokio::test]
    async fn provider_upstream_auth_failure_maps_to_bad_gateway_not_user_unauthorized() {
        assert_sanitized_problem(
            provider_error(
                sdkwork_knowledgebase_contract::knowledge_engine::KnowledgeEngineProviderErrorCategory::Authentication,
            ),
            StatusCode::BAD_GATEWAY,
            50201,
        )
        .await;
    }

    #[tokio::test]
    async fn provider_rate_limit_maps_to_sanitized_service_unavailable_problem() {
        assert_sanitized_problem(
            provider_error(
                sdkwork_knowledgebase_contract::knowledge_engine::KnowledgeEngineProviderErrorCategory::RateLimited,
            ),
            StatusCode::SERVICE_UNAVAILABLE,
            50301,
        )
        .await;
    }

    fn provider_error(
        category: sdkwork_knowledgebase_contract::knowledge_engine::KnowledgeEngineProviderErrorCategory,
    ) -> ApiError {
        use sdkwork_knowledgebase_contract::knowledge_engine::{
            KnowledgeEngineError, KnowledgeEngineProviderFailure, KnowledgeEngineProviderOperation,
        };
        ApiError::from(KnowledgeEngineError::Provider(
            KnowledgeEngineProviderFailure {
                category,
                operation: KnowledgeEngineProviderOperation::Search,
                implementation_id: "engine.knowledge.external.dify".to_string(),
                binding_id: Some("binding-1".to_string()),
                status_code: Some(503),
                retryable: true,
                retry_after_ms: Some(100),
                safe_message: "provider request failed".to_string(),
            },
        ))
    }

    async fn assert_sanitized_problem(
        error: ApiError,
        expected_status: StatusCode,
        expected_code: i32,
    ) {
        assert_eq!(error.status, expected_status);

        let response = ApiProblem::from(error).into_response();
        assert_eq!(response.status(), expected_status);
        assert_eq!(
            response
                .headers()
                .get(axum::http::header::CONTENT_TYPE)
                .and_then(|value| value.to_str().ok()),
            Some("application/problem+json")
        );

        let body = to_bytes(response.into_body(), usize::MAX)
            .await
            .expect("read problem response body");
        let problem: ProblemDetails =
            serde_json::from_slice(&body).expect("deserialize problem response body");
        assert_eq!(problem.status, expected_status.as_u16());
        assert_eq!(problem.code, expected_code);
        assert!(problem.detail.is_none());
    }
}
