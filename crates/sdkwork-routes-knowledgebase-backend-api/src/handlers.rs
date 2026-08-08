use axum::{
    extract::{Path, Query, State},
    response::Response,
    Json,
};
use sdkwork_knowledgebase_contract::provider_binding::{
    CreateKnowledgeEngineProviderBindingRequest,
    CreateKnowledgeEngineProviderCredentialReferenceRequest,
    CreateKnowledgeEngineProviderMigrationOperationRequest, KnowledgeEngineProviderBindingState,
    KnowledgeEngineProviderCredentialRotationState, KnowledgeEngineProviderMigrationState,
    ProviderBindingVersionCommandRequest, ProviderMigrationVersionCommandRequest,
    RevokeKnowledgeEngineProviderCredentialReferenceRequest,
    RotateKnowledgeEngineProviderCredentialReferenceRequest,
    UpdateKnowledgeEngineProviderBindingRequest,
};
use sdkwork_knowledgebase_contract::{
    AnonymizeKnowledgeAuditSubjectRequest, CreateKnowledgeSourceRequest,
    ExportKnowledgeAuditEventsRequest, KnowledgeIndexRequest, KnowledgeOkfProfileRequest,
    KnowledgeRetrievalProfileRequest, OkfBundleExportRequest, OkfBundleImportRequest,
    OkfCandidateReviewRequest, OkfCompileJobRequest, OkfConceptPublishRequest,
    OkfIndexRebuildRequest, OkfLogEntry, OkfQualityRunRequest,
};
use serde::Deserialize;

use crate::{
    auth::{require_backend_context, require_backend_mutation_context, RequiredBackendContext},
    error::{BackendApiProblem, BackendApiResult},
    ports::KnowledgeBackendRequestContext,
    response::{command_json, created_json, ok_json, ok_list_json},
    routes::BackendState,
};

macro_rules! backend_handler {
    ($name:ident, $body:expr) => {
        pub(crate) async fn $name(
            State(state): State<BackendState>,
            context: RequiredBackendContext,
        ) -> Result<Response, BackendApiProblem> {
            require_backend_context(&state, context)?;
            $body(state).await
        }
    };
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct ListOkfCandidatesQuery {
    pub space_id: u64,
    pub cursor: Option<String>,
    #[serde(rename = "page_size")]
    pub page_size: Option<u32>,
}

fn parse_cursor(cursor: Option<&str>) -> Result<Option<u64>, BackendApiProblem> {
    crate::pagination::parse_u64_cursor(cursor).map_err(|_| {
        BackendApiProblem::from(crate::error::BackendApiError::new(
            axum::http::StatusCode::BAD_REQUEST,
            "invalid_parameter",
            "cursor must be a valid cursor token",
        ))
    })
}

async fn audit_backend_mutation<T>(
    context: &KnowledgeBackendRequestContext,
    operation: &str,
    result: BackendApiResult<T>,
) -> Result<BackendApiResult<T>, BackendApiProblem> {
    if result.is_ok() {
        let operator_id = backend_audit_operator_id(context)?;
        sdkwork_knowledgebase_observability::record_backend_admin_operation(
            operation,
            context.tenant_id,
            operator_id,
        )
        .await
        .map_err(|error| {
            BackendApiProblem::from_internal(
                "knowledge_audit_persistence_failed",
                error.to_string(),
            )
        })?;
    }
    Ok(result)
}

fn backend_audit_operator_id(
    context: &KnowledgeBackendRequestContext,
) -> Result<u64, BackendApiProblem> {
    context.operator_id.ok_or_else(|| {
        BackendApiProblem::from_internal(
            "knowledge_audit_actor_missing",
            "authenticated backend operator is required for mutation audit",
        )
    })
}

async fn audit_provider_mutation<T, F>(
    context: &KnowledgeBackendRequestContext,
    operation: &str,
    result: BackendApiResult<T>,
    metadata: F,
) -> Result<BackendApiResult<T>, BackendApiProblem>
where
    F: FnOnce(&T) -> sdkwork_knowledgebase_observability::BackendAdminResourceAudit,
{
    if let Ok(value) = &result {
        sdkwork_knowledgebase_observability::record_backend_admin_resource_operation(
            operation,
            context.tenant_id,
            backend_audit_operator_id(context)?,
            metadata(value),
        )
        .await
        .map_err(|error| {
            BackendApiProblem::from_internal(
                "knowledge_audit_persistence_failed",
                error.to_string(),
            )
        })?;
    }
    Ok(result)
}

fn provider_audit_metadata(
    resource_type: &str,
    resource_id: u64,
    space_id: Option<u64>,
    expected_version: Option<u64>,
    result_version: Option<u64>,
    result_status: Option<String>,
) -> sdkwork_knowledgebase_observability::BackendAdminResourceAudit {
    sdkwork_knowledgebase_observability::BackendAdminResourceAudit {
        resource_type: resource_type.to_string(),
        resource_id: Some(resource_id),
        space_id,
        expected_version,
        result_version,
        result_status,
    }
}

pub(crate) async fn list_sources(
    State(state): State<BackendState>,
    context: RequiredBackendContext,
    Query(query): Query<ListSpacesQuery>,
) -> Result<Response, BackendApiProblem> {
    require_backend_context(&state, context)?;
    let cursor = parse_cursor(query.cursor.as_deref())?;
    ok_list_json(
        state
            .api
            .list_sources_page(cursor.map(|id| id.to_string()), query.page_size)
            .await,
    )
}

pub(crate) async fn create_source(
    State(state): State<BackendState>,
    context: RequiredBackendContext,
    Json(request): Json<CreateKnowledgeSourceRequest>,
) -> Result<Response, BackendApiProblem> {
    let context = require_backend_mutation_context(&state, context, "sources.create")?;
    let result = state.api.create_source(request).await;
    created_json(audit_backend_mutation(&context, "sources.create", result).await?)
}

pub(crate) async fn create_okf_compile_job(
    State(state): State<BackendState>,
    context: RequiredBackendContext,
    Json(request): Json<OkfCompileJobRequest>,
) -> Result<Response, BackendApiProblem> {
    let context = require_backend_mutation_context(&state, context, "okf.compileJobs.create")?;
    let result = state.api.create_okf_compile_job(request).await;
    created_json(audit_backend_mutation(&context, "okf.compileJobs.create", result).await?)
}

pub(crate) async fn list_okf_candidates(
    State(state): State<BackendState>,
    context: RequiredBackendContext,
    Query(query): Query<ListOkfCandidatesQuery>,
) -> Result<Response, BackendApiProblem> {
    require_backend_context(&state, context)?;
    let cursor = parse_cursor(query.cursor.as_deref())?;
    ok_list_json(
        state
            .api
            .list_okf_candidates_page(
                query.space_id,
                cursor.map(|id| id.to_string()),
                query.page_size,
            )
            .await,
    )
}

pub(crate) async fn approve_okf_candidate(
    State(state): State<BackendState>,
    context: RequiredBackendContext,
    Path(candidate_id): Path<u64>,
    Json(request): Json<OkfCandidateReviewRequest>,
) -> Result<Response, BackendApiProblem> {
    let context = require_backend_mutation_context(&state, context, "okf.candidates.approve")?;
    let result = state.api.approve_okf_candidate(candidate_id, request).await;
    ok_json(audit_backend_mutation(&context, "okf.candidates.approve", result).await?)
}

pub(crate) async fn reject_okf_candidate(
    State(state): State<BackendState>,
    context: RequiredBackendContext,
    Path(candidate_id): Path<u64>,
    Json(request): Json<OkfCandidateReviewRequest>,
) -> Result<Response, BackendApiProblem> {
    let context = require_backend_mutation_context(&state, context, "okf.candidates.reject")?;
    let result = state.api.reject_okf_candidate(candidate_id, request).await;
    ok_json(audit_backend_mutation(&context, "okf.candidates.reject", result).await?)
}

pub(crate) async fn publish_okf_concept(
    State(state): State<BackendState>,
    context: RequiredBackendContext,
    Path(concept_id): Path<u64>,
    Json(request): Json<OkfConceptPublishRequest>,
) -> Result<Response, BackendApiProblem> {
    let context = require_backend_mutation_context(&state, context, "okf.concepts.publish")?;
    let result = state.api.publish_okf_concept(concept_id, request).await;
    ok_json(audit_backend_mutation(&context, "okf.concepts.publish", result).await?)
}

pub(crate) async fn create_okf_profile(
    State(state): State<BackendState>,
    context: RequiredBackendContext,
    Json(request): Json<KnowledgeOkfProfileRequest>,
) -> Result<Response, BackendApiProblem> {
    let context = require_backend_mutation_context(&state, context, "okf.profile.create")?;
    let result = state.api.create_okf_profile(request).await;
    created_json(audit_backend_mutation(&context, "okf.profile.create", result).await?)
}

pub(crate) async fn update_okf_profile(
    State(state): State<BackendState>,
    context: RequiredBackendContext,
    Path(profile_id): Path<u64>,
    Json(request): Json<KnowledgeOkfProfileRequest>,
) -> Result<Response, BackendApiProblem> {
    let context = require_backend_mutation_context(&state, context, "okf.profile.update")?;
    let result = state.api.update_okf_profile(profile_id, request).await;
    ok_json(audit_backend_mutation(&context, "okf.profile.update", result).await?)
}

pub(crate) async fn rebuild_okf_index(
    State(state): State<BackendState>,
    context: RequiredBackendContext,
    Json(request): Json<OkfIndexRebuildRequest>,
) -> Result<Response, BackendApiProblem> {
    let context = require_backend_mutation_context(&state, context, "okf.bundle.index.rebuild")?;
    let result = state.api.rebuild_okf_index(request).await;
    ok_json(audit_backend_mutation(&context, "okf.bundle.index.rebuild", result).await?)
}

pub(crate) async fn create_okf_log_entry(
    State(state): State<BackendState>,
    context: RequiredBackendContext,
    Json(request): Json<OkfLogEntry>,
) -> Result<Response, BackendApiProblem> {
    let context = require_backend_mutation_context(&state, context, "okf.log.entries.create")?;
    let result = state.api.create_okf_log_entry(request).await;
    created_json(audit_backend_mutation(&context, "okf.log.entries.create", result).await?)
}

pub(crate) async fn create_okf_export(
    State(state): State<BackendState>,
    context: RequiredBackendContext,
    Json(request): Json<OkfBundleExportRequest>,
) -> Result<Response, BackendApiProblem> {
    let context = require_backend_mutation_context(&state, context, "okf.bundle.export.create")?;
    let result = state.api.create_okf_export(request).await;
    created_json(audit_backend_mutation(&context, "okf.bundle.export.create", result).await?)
}

pub(crate) async fn create_okf_import(
    State(state): State<BackendState>,
    context: RequiredBackendContext,
    Json(request): Json<OkfBundleImportRequest>,
) -> Result<Response, BackendApiProblem> {
    let context = require_backend_mutation_context(&state, context, "okf.bundle.import.create")?;
    let result = state.api.create_okf_import(request).await;
    created_json(audit_backend_mutation(&context, "okf.bundle.import.create", result).await?)
}

pub(crate) async fn retrieve_okf_export(
    State(state): State<BackendState>,
    context: RequiredBackendContext,
    Path(export_id): Path<u64>,
) -> Result<Response, BackendApiProblem> {
    require_backend_context(&state, context)?;
    ok_json(state.api.retrieve_okf_export(export_id).await)
}

pub(crate) async fn list_okf_bundle_files(
    State(state): State<BackendState>,
    context: RequiredBackendContext,
    Query(query): Query<ListSpacesQuery>,
) -> Result<Response, BackendApiProblem> {
    require_backend_context(&state, context)?;
    let cursor = parse_cursor(query.cursor.as_deref())?;
    ok_list_json(
        state
            .api
            .list_okf_bundle_files_page(cursor.map(|id| id.to_string()), query.page_size)
            .await,
    )
}

pub(crate) async fn create_okf_lint_run(
    State(state): State<BackendState>,
    context: RequiredBackendContext,
    Json(request): Json<OkfQualityRunRequest>,
) -> Result<Response, BackendApiProblem> {
    let context = require_backend_mutation_context(&state, context, "okf.lintRuns.create")?;
    let result = state.api.create_okf_lint_run(request).await;
    created_json(audit_backend_mutation(&context, "okf.lintRuns.create", result).await?)
}

pub(crate) async fn create_okf_eval_run(
    State(state): State<BackendState>,
    context: RequiredBackendContext,
    Json(request): Json<OkfQualityRunRequest>,
) -> Result<Response, BackendApiProblem> {
    let context = require_backend_mutation_context(&state, context, "okf.evalRuns.create")?;
    let result = state.api.create_okf_eval_run(request).await;
    created_json(audit_backend_mutation(&context, "okf.evalRuns.create", result).await?)
}

pub(crate) async fn list_indexes(
    State(state): State<BackendState>,
    context: RequiredBackendContext,
    Query(query): Query<ListSpacesQuery>,
) -> Result<Response, BackendApiProblem> {
    require_backend_context(&state, context)?;
    let cursor = parse_cursor(query.cursor.as_deref())?;
    ok_list_json(
        state
            .api
            .list_indexes_page(cursor.map(|id| id.to_string()), query.page_size)
            .await,
    )
}

pub(crate) async fn create_index(
    State(state): State<BackendState>,
    context: RequiredBackendContext,
    Json(request): Json<KnowledgeIndexRequest>,
) -> Result<Response, BackendApiProblem> {
    let context = require_backend_mutation_context(&state, context, "indexes.create")?;
    let result = state
        .api
        .create_index(request.with_tenant_id(context.tenant_id))
        .await;
    created_json(audit_backend_mutation(&context, "indexes.create", result).await?)
}

pub(crate) async fn retrieve_index(
    State(state): State<BackendState>,
    context: RequiredBackendContext,
    Path(index_id): Path<u64>,
) -> Result<Response, BackendApiProblem> {
    require_backend_context(&state, context)?;
    ok_json(state.api.retrieve_index(index_id).await)
}

pub(crate) async fn rebuild_index(
    State(state): State<BackendState>,
    context: RequiredBackendContext,
    Path(index_id): Path<u64>,
    Json(request): Json<OkfIndexRebuildRequest>,
) -> Result<Response, BackendApiProblem> {
    let context = require_backend_mutation_context(&state, context, "indexes.rebuild")?;
    let result = state.api.rebuild_index(index_id, request).await;
    ok_json(audit_backend_mutation(&context, "indexes.rebuild", result).await?)
}

pub(crate) async fn create_retrieval_profile(
    State(state): State<BackendState>,
    context: RequiredBackendContext,
    Json(request): Json<KnowledgeRetrievalProfileRequest>,
) -> Result<Response, BackendApiProblem> {
    let context = require_backend_mutation_context(&state, context, "retrievalProfiles.create")?;
    let result = state
        .api
        .create_retrieval_profile(request.with_tenant_id(context.tenant_id))
        .await;
    created_json(audit_backend_mutation(&context, "retrievalProfiles.create", result).await?)
}

pub(crate) async fn retrieve_retrieval_profile(
    State(state): State<BackendState>,
    context: RequiredBackendContext,
    Path(profile_id): Path<u64>,
) -> Result<Response, BackendApiProblem> {
    require_backend_context(&state, context)?;
    ok_json(state.api.retrieve_retrieval_profile(profile_id).await)
}

pub(crate) async fn update_retrieval_profile(
    State(state): State<BackendState>,
    context: RequiredBackendContext,
    Path(profile_id): Path<u64>,
    Json(request): Json<KnowledgeRetrievalProfileRequest>,
) -> Result<Response, BackendApiProblem> {
    let context = require_backend_mutation_context(&state, context, "retrievalProfiles.update")?;
    let result = state
        .api
        .update_retrieval_profile(profile_id, request.with_tenant_id(context.tenant_id))
        .await;
    ok_json(audit_backend_mutation(&context, "retrievalProfiles.update", result).await?)
}

pub(crate) async fn list_retrieval_traces(
    State(state): State<BackendState>,
    context: RequiredBackendContext,
    Query(query): Query<ListSpacesQuery>,
) -> Result<Response, BackendApiProblem> {
    require_backend_context(&state, context)?;
    let cursor = parse_cursor(query.cursor.as_deref())?;
    ok_list_json(
        state
            .api
            .list_retrieval_traces_page(cursor.map(|id| id.to_string()), query.page_size)
            .await,
    )
}

pub(crate) async fn retrieve_retrieval_trace(
    State(state): State<BackendState>,
    context: RequiredBackendContext,
    Path(trace_id): Path<u64>,
) -> Result<Response, BackendApiProblem> {
    require_backend_context(&state, context)?;
    ok_json(state.api.retrieve_retrieval_trace(trace_id).await)
}

pub(crate) async fn retrieve_provider_health(
    State(state): State<BackendState>,
    context: RequiredBackendContext,
) -> Result<Response, BackendApiProblem> {
    let context = require_backend_context(&state, context)?;
    ok_json(state.api.retrieve_provider_health(&context).await)
}

#[derive(Debug, Deserialize)]
pub(crate) struct ListProviderCredentialReferencesQuery {
    pub implementation_id: Option<String>,
    pub rotation_state: Option<KnowledgeEngineProviderCredentialRotationState>,
    pub cursor: Option<String>,
    pub page_size: Option<u32>,
}

#[derive(Debug, Deserialize)]
pub(crate) struct ListProviderBindingsQuery {
    pub lifecycle_state: Option<KnowledgeEngineProviderBindingState>,
    pub cursor: Option<String>,
    pub page_size: Option<u32>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct CreateProviderBindingBody {
    pub implementation_id: String,
    pub remote_resource_type: String,
    pub remote_resource_id: String,
    #[serde(default, with = "sdkwork_utils_rust::serde_uint64::option")]
    pub credential_reference_id: Option<u64>,
}

#[derive(Debug, Deserialize)]
pub(crate) struct ListProviderMigrationsQuery {
    pub operation_state: Option<KnowledgeEngineProviderMigrationState>,
    pub cursor: Option<String>,
    pub page_size: Option<u32>,
}

pub(crate) async fn create_provider_credential_reference(
    State(state): State<BackendState>,
    context: RequiredBackendContext,
    Json(request): Json<CreateKnowledgeEngineProviderCredentialReferenceRequest>,
) -> Result<Response, BackendApiProblem> {
    let context =
        require_backend_mutation_context(&state, context, "providerCredentialReferences.create")?;
    let result = state
        .api
        .create_provider_credential_reference(&context, request)
        .await;
    created_json(
        audit_provider_mutation(
            &context,
            "providerCredentialReferences.create",
            result,
            |credential| {
                provider_audit_metadata(
                    "provider_credential_reference",
                    credential.id,
                    None,
                    None,
                    Some(credential.version),
                    Some(credential.rotation_state.as_str().to_string()),
                )
            },
        )
        .await?,
    )
}

pub(crate) async fn list_provider_credential_references(
    State(state): State<BackendState>,
    context: RequiredBackendContext,
    Query(query): Query<ListProviderCredentialReferencesQuery>,
) -> Result<Response, BackendApiProblem> {
    let context = require_backend_context(&state, context)?;
    ok_list_json(
        state
            .api
            .list_provider_credential_references(
                &context,
                query.implementation_id,
                query.rotation_state,
                query.cursor,
                query.page_size,
            )
            .await,
    )
}

pub(crate) async fn retrieve_provider_credential_reference(
    State(state): State<BackendState>,
    context: RequiredBackendContext,
    Path(credential_reference_id): Path<u64>,
) -> Result<Response, BackendApiProblem> {
    let context = require_backend_context(&state, context)?;
    ok_json(
        state
            .api
            .retrieve_provider_credential_reference(&context, credential_reference_id)
            .await,
    )
}

pub(crate) async fn rotate_provider_credential_reference(
    State(state): State<BackendState>,
    context: RequiredBackendContext,
    Path(credential_reference_id): Path<u64>,
    Json(request): Json<RotateKnowledgeEngineProviderCredentialReferenceRequest>,
) -> Result<Response, BackendApiProblem> {
    let context =
        require_backend_mutation_context(&state, context, "providerCredentialReferences.rotate")?;
    let expected_version = request.expected_version;
    let result = state
        .api
        .rotate_provider_credential_reference(&context, credential_reference_id, request)
        .await;
    command_json(
        audit_provider_mutation(
            &context,
            "providerCredentialReferences.rotate",
            result,
            |command| {
                provider_audit_metadata(
                    "provider_credential_reference",
                    credential_reference_id,
                    None,
                    Some(expected_version),
                    None,
                    command.status.clone(),
                )
            },
        )
        .await?,
    )
}

pub(crate) async fn revoke_provider_credential_reference(
    State(state): State<BackendState>,
    context: RequiredBackendContext,
    Path(credential_reference_id): Path<u64>,
    Json(request): Json<RevokeKnowledgeEngineProviderCredentialReferenceRequest>,
) -> Result<Response, BackendApiProblem> {
    let context =
        require_backend_mutation_context(&state, context, "providerCredentialReferences.revoke")?;
    let expected_version = request.expected_version;
    let result = state
        .api
        .revoke_provider_credential_reference(&context, credential_reference_id, request)
        .await;
    command_json(
        audit_provider_mutation(
            &context,
            "providerCredentialReferences.revoke",
            result,
            |command| {
                provider_audit_metadata(
                    "provider_credential_reference",
                    credential_reference_id,
                    None,
                    Some(expected_version),
                    None,
                    command.status.clone(),
                )
            },
        )
        .await?,
    )
}

pub(crate) async fn list_provider_bindings(
    State(state): State<BackendState>,
    context: RequiredBackendContext,
    Path(space_id): Path<u64>,
    Query(query): Query<ListProviderBindingsQuery>,
) -> Result<Response, BackendApiProblem> {
    let context = require_backend_context(&state, context)?;
    ok_list_json(
        state
            .api
            .list_provider_bindings(
                &context,
                space_id,
                query.lifecycle_state,
                query.cursor,
                query.page_size,
            )
            .await,
    )
}

pub(crate) async fn create_provider_binding(
    State(state): State<BackendState>,
    context: RequiredBackendContext,
    Path(space_id): Path<u64>,
    Json(request): Json<CreateProviderBindingBody>,
) -> Result<Response, BackendApiProblem> {
    let context =
        require_backend_mutation_context(&state, context, "spaces.providerBindings.create")?;
    let result = state
        .api
        .create_provider_binding(
            &context,
            CreateKnowledgeEngineProviderBindingRequest {
                space_id,
                implementation_id: request.implementation_id,
                remote_resource_type: request.remote_resource_type,
                remote_resource_id: request.remote_resource_id,
                credential_reference_id: request.credential_reference_id,
            },
        )
        .await;
    created_json(
        audit_provider_mutation(
            &context,
            "spaces.providerBindings.create",
            result,
            |binding| {
                provider_audit_metadata(
                    "provider_binding",
                    binding.id,
                    Some(space_id),
                    None,
                    Some(binding.version),
                    Some(binding.lifecycle_state.as_str().to_string()),
                )
            },
        )
        .await?,
    )
}

pub(crate) async fn retrieve_provider_binding(
    State(state): State<BackendState>,
    context: RequiredBackendContext,
    Path((space_id, binding_id)): Path<(u64, u64)>,
) -> Result<Response, BackendApiProblem> {
    let context = require_backend_context(&state, context)?;
    ok_json(
        state
            .api
            .retrieve_provider_binding(&context, space_id, binding_id)
            .await,
    )
}

pub(crate) async fn update_provider_binding(
    State(state): State<BackendState>,
    context: RequiredBackendContext,
    Path((space_id, binding_id)): Path<(u64, u64)>,
    Json(request): Json<UpdateKnowledgeEngineProviderBindingRequest>,
) -> Result<Response, BackendApiProblem> {
    let context =
        require_backend_mutation_context(&state, context, "spaces.providerBindings.update")?;
    let expected_version = request.expected_version;
    let result = state
        .api
        .update_provider_binding(&context, space_id, binding_id, request)
        .await;
    ok_json(
        audit_provider_mutation(
            &context,
            "spaces.providerBindings.update",
            result,
            |binding| {
                provider_audit_metadata(
                    "provider_binding",
                    binding_id,
                    Some(space_id),
                    Some(expected_version),
                    Some(binding.version),
                    Some(binding.lifecycle_state.as_str().to_string()),
                )
            },
        )
        .await?,
    )
}

macro_rules! provider_binding_command_handler {
    ($name:ident, $method:ident, $operation:literal) => {
        pub(crate) async fn $name(
            State(state): State<BackendState>,
            context: RequiredBackendContext,
            Path((space_id, binding_id)): Path<(u64, u64)>,
            Json(request): Json<ProviderBindingVersionCommandRequest>,
        ) -> Result<Response, BackendApiProblem> {
            let context = require_backend_mutation_context(&state, context, $operation)?;
            let expected_version = request.expected_version;
            let result = state
                .api
                .$method(&context, space_id, binding_id, expected_version)
                .await;
            command_json(
                audit_provider_mutation(&context, $operation, result, |command| {
                    provider_audit_metadata(
                        "provider_binding",
                        binding_id,
                        Some(space_id),
                        Some(expected_version),
                        None,
                        command.status.clone(),
                    )
                })
                .await?,
            )
        }
    };
}

provider_binding_command_handler!(
    test_provider_binding,
    test_provider_binding,
    "spaces.providerBindings.test"
);

pub(crate) async fn list_provider_migrations(
    State(state): State<BackendState>,
    context: RequiredBackendContext,
    Path(space_id): Path<u64>,
    Query(query): Query<ListProviderMigrationsQuery>,
) -> Result<Response, BackendApiProblem> {
    let context = require_backend_context(&state, context)?;
    ok_list_json(
        state
            .api
            .list_provider_migrations(
                &context,
                space_id,
                query.operation_state,
                query.cursor,
                query.page_size,
            )
            .await,
    )
}

pub(crate) async fn create_provider_migration(
    State(state): State<BackendState>,
    context: RequiredBackendContext,
    Path(space_id): Path<u64>,
    Json(request): Json<CreateKnowledgeEngineProviderMigrationOperationRequest>,
) -> Result<Response, BackendApiProblem> {
    let context =
        require_backend_mutation_context(&state, context, "spaces.providerMigrations.create")?;
    let result = state
        .api
        .create_provider_migration(&context, space_id, request)
        .await;
    created_json(
        audit_provider_mutation(
            &context,
            "spaces.providerMigrations.create",
            result,
            |operation| {
                provider_audit_metadata(
                    "provider_migration_operation",
                    operation.id,
                    Some(space_id),
                    None,
                    Some(operation.version),
                    Some(operation.operation_state.as_str().to_string()),
                )
            },
        )
        .await?,
    )
}

pub(crate) async fn retrieve_provider_migration(
    State(state): State<BackendState>,
    context: RequiredBackendContext,
    Path((space_id, migration_operation_id)): Path<(u64, u64)>,
) -> Result<Response, BackendApiProblem> {
    let context = require_backend_context(&state, context)?;
    ok_json(
        state
            .api
            .retrieve_provider_migration(&context, space_id, migration_operation_id)
            .await,
    )
}

pub(crate) async fn rollback_provider_migration(
    State(state): State<BackendState>,
    context: RequiredBackendContext,
    Path((space_id, migration_operation_id)): Path<(u64, u64)>,
    Json(request): Json<ProviderMigrationVersionCommandRequest>,
) -> Result<Response, BackendApiProblem> {
    let context =
        require_backend_mutation_context(&state, context, "spaces.providerMigrations.rollback")?;
    let expected_version = request.expected_version;
    let result = state
        .api
        .rollback_provider_migration(&context, space_id, migration_operation_id, expected_version)
        .await;
    command_json(
        audit_provider_mutation(
            &context,
            "spaces.providerMigrations.rollback",
            result,
            |command| {
                provider_audit_metadata(
                    "provider_migration_operation",
                    migration_operation_id,
                    Some(space_id),
                    Some(expected_version),
                    None,
                    command.status.clone(),
                )
            },
        )
        .await?,
    )
}
provider_binding_command_handler!(
    activate_provider_binding,
    activate_provider_binding,
    "spaces.providerBindings.activate"
);
provider_binding_command_handler!(
    disable_provider_binding,
    disable_provider_binding,
    "spaces.providerBindings.disable"
);

backend_handler!(
    retrieve_group_launch_capability,
    |state: BackendState| async move { ok_json(state.api.retrieve_group_launch_capability().await) }
);

// ============================================================================
// Tenant status handler
// ============================================================================

// Retrieves the caller's own tenant knowledgebase status.
//
// Security: The tenant is identified by the authenticated principal's token claims.
// Returns space count, document count, and status for the current tenant.
backend_handler!(retrieve_current_tenant, |state: BackendState| async move {
    ok_json(state.api.retrieve_current_tenant().await)
});

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct ListSpaceMembersQuery {
    pub cursor: Option<String>,
    #[serde(rename = "page_size")]
    pub page_size: Option<u32>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct ListSpacesQuery {
    pub cursor: Option<String>,
    #[serde(rename = "page_size")]
    pub page_size: Option<u32>,
}

pub(crate) async fn list_spaces(
    State(state): State<BackendState>,
    context: RequiredBackendContext,
    Query(query): Query<ListSpacesQuery>,
) -> Result<Response, BackendApiProblem> {
    require_backend_context(&state, context)?;
    ok_list_json(state.api.list_spaces(query.cursor, query.page_size).await)
}

pub(crate) async fn list_space_members(
    State(state): State<BackendState>,
    context: RequiredBackendContext,
    Path(space_id): Path<u64>,
    Query(query): Query<ListSpaceMembersQuery>,
) -> Result<Response, BackendApiProblem> {
    require_backend_context(&state, context)?;
    ok_list_json(
        state
            .api
            .list_space_members(space_id, query.cursor, query.page_size)
            .await,
    )
}

pub(crate) async fn export_audit_events(
    State(state): State<BackendState>,
    context: RequiredBackendContext,
    Json(request): Json<ExportKnowledgeAuditEventsRequest>,
) -> Result<Response, BackendApiProblem> {
    let context =
        require_backend_mutation_context(&state, context, "compliance.auditEvents.export.create")?;
    let result = state.api.export_audit_events(request).await;
    created_json(
        audit_backend_mutation(&context, "compliance.auditEvents.export.create", result).await?,
    )
}

pub(crate) async fn anonymize_audit_subject(
    State(state): State<BackendState>,
    context: RequiredBackendContext,
    Json(request): Json<AnonymizeKnowledgeAuditSubjectRequest>,
) -> Result<Response, BackendApiProblem> {
    let context = require_backend_mutation_context(
        &state,
        context,
        "compliance.auditEvents.anonymizeActor.create",
    )?;
    let result = state.api.anonymize_audit_subject(request).await;
    created_json(
        audit_backend_mutation(
            &context,
            "compliance.auditEvents.anonymizeActor.create",
            result,
        )
        .await?,
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::BackendApiError;
    use std::sync::{
        atomic::{AtomicBool, Ordering},
        Arc,
    };

    async fn audit_test_lock() -> tokio::sync::MutexGuard<'static, ()> {
        static LOCK: tokio::sync::Mutex<()> = tokio::sync::Mutex::const_new(());
        LOCK.lock().await
    }

    fn admin_context() -> KnowledgeBackendRequestContext {
        KnowledgeBackendRequestContext {
            tenant_id: 100_001,
            operator_id: Some(99),
            organization_id: Some(7),
            permission_scope: vec![],
            trace_id: "trace-handler-test".to_string(),
        }
    }

    #[tokio::test]
    async fn mutation_audit_never_reports_failed_business_result_as_success() {
        let _guard = audit_test_lock().await;
        let called = Arc::new(AtomicBool::new(false));
        let called_by_handler = Arc::clone(&called);
        sdkwork_knowledgebase_observability::install_audit_persistence(move |_event| {
            let called = Arc::clone(&called_by_handler);
            async move {
                called.store(true, Ordering::SeqCst);
                Ok(())
            }
        });

        let business_failure: BackendApiResult<()> =
            Err(BackendApiError::unsupported_operation("sources.create"));
        let result = audit_backend_mutation(&admin_context(), "sources.create", business_failure)
            .await
            .expect("business result");

        assert!(result.is_err());
        assert!(!called.load(Ordering::SeqCst));
    }

    #[tokio::test]
    async fn mutation_audit_failure_prevents_success_response() {
        let _guard = audit_test_lock().await;
        sdkwork_knowledgebase_observability::install_audit_persistence(|_event| async {
            Err(
                sdkwork_knowledgebase_observability::AuditPersistenceError::write_failed(
                    "database unavailable",
                ),
            )
        });

        let result = audit_backend_mutation(
            &admin_context(),
            "sources.create",
            Ok::<(), BackendApiError>(()),
        )
        .await;

        assert!(result.is_err());
    }
}
