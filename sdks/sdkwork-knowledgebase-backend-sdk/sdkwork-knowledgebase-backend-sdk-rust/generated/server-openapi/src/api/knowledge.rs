use std::sync::Arc;

use crate::api::paths::backend_path;
use crate::api::paths::append_query_string;
use crate::http::{SdkworkError, SdkworkHttpClient};
use crate::models::{AnonymizeKnowledgeAuditSubjectRequest, AnonymizeKnowledgeAuditSubjectResult, CreateKnowledgeEngineProviderBindingRequest, CreateKnowledgeEngineProviderCredentialReferenceRequest, CreateKnowledgeEngineProviderMigrationOperationRequest, CreateKnowledgeSourceRequest, ExportKnowledgeAuditEventsRequest, GroupKnowledgebaseLaunchCapability, IngestionJob, KnowledgeAuditEventExport, KnowledgeEngineProviderBinding, KnowledgeEngineProviderBindingPage, KnowledgeEngineProviderCredentialReference, KnowledgeEngineProviderCredentialReferencePage, KnowledgeEngineProviderMigrationOperation, KnowledgeEngineProviderMigrationOperationPage, KnowledgeIndex, KnowledgeIndexRequest, KnowledgeOkfBundleFile, KnowledgeOkfProfileRequest, KnowledgeProviderHealth, KnowledgeRetrievalProfile, KnowledgeRetrievalProfileRequest, KnowledgeRetrievalTrace, KnowledgeSource, KnowledgeTenantStatus, OkfBundleExportRequest, OkfBundleImportRequest, OkfBundleImportResult, OkfBundleIndexRebuildRequest, OkfCandidateResult, OkfCandidateReviewRequest, OkfCompileJobRequest, OkfConceptPublishRequest, OkfConceptSummary, OkfIndexDocument, OkfLogEntry, OkfQualityRun, OkfQualityRunRequest, ProviderBindingVersionCommandRequest, ProviderMigrationVersionCommandRequest, RevokeKnowledgeEngineProviderCredentialReferenceRequest, RotateKnowledgeEngineProviderCredentialReferenceRequest, SdkWorkCommandData, UpdateKnowledgeEngineProviderBindingRequest};

#[derive(Clone)]
pub struct KnowledgeApi {
    client: Arc<SdkworkHttpClient>,
}

impl KnowledgeApi {
    pub fn new(client: Arc<SdkworkHttpClient>) -> Self {
        Self { client }
    }

    /// Anonymize audit events for a subject
    pub async fn compliance_audit_events_anonymize_actor_create(&self, body: &AnonymizeKnowledgeAuditSubjectRequest) -> Result<AnonymizeKnowledgeAuditSubjectResult, SdkworkError> {
        let path = backend_path(&"/knowledge/compliance/audit_events/anonymize_actor".to_string());
        self.client.post(&path, Some(body), None, None, Some("application/json")).await
    }

    /// Export knowledge audit events for a subject
    pub async fn compliance_audit_events_export_create(&self, body: &ExportKnowledgeAuditEventsRequest) -> Result<KnowledgeAuditEventExport, SdkworkError> {
        let path = backend_path(&"/knowledge/compliance/audit_events/export".to_string());
        self.client.post(&path, Some(body), None, None, Some("application/json")).await
    }

    /// Retrieve the group knowledgebase launch capability state for the runtime deployment
    pub async fn group_launch_capability(&self) -> Result<GroupKnowledgebaseLaunchCapability, SdkworkError> {
        let path = backend_path(&"/knowledge/group_launch_capability".to_string());
        self.client.get(&path, None, None).await
    }

    /// Create a knowledge index
    pub async fn indexes_create(&self, body: &KnowledgeIndexRequest) -> Result<KnowledgeIndex, SdkworkError> {
        let path = backend_path(&"/knowledge/indexes".to_string());
        self.client.post(&path, Some(body), None, None, Some("application/json")).await
    }

    /// List knowledge indexes
    pub async fn indexes_list(&self, cursor: Option<&str>, page_size: Option<i64>) -> Result<serde_json::Value, SdkworkError> {
        let query = build_query_string(&[
            QueryParameterSpec::new("cursor", cursor, "form", true, false, None),
            QueryParameterSpec::new("page_size", page_size, "form", true, false, None),
        ]);
        let path = append_query_string(backend_path(&"/knowledge/indexes".to_string()), &query);
        self.client.get(&path, None, None).await
    }

    /// Retrieve a knowledge index
    pub async fn indexes_retrieve(&self, index_id: &str) -> Result<KnowledgeIndex, SdkworkError> {
        let path = backend_path(&format!("/knowledge/indexes/{}", serialize_path_parameter(index_id, PathParameterSpec::new("indexId", "simple", false))));
        self.client.get(&path, None, None).await
    }

    /// Rebuild a knowledge index
    pub async fn indexes_rebuild(&self, index_id: &str, body: &OkfBundleIndexRebuildRequest) -> Result<OkfIndexDocument, SdkworkError> {
        let path = backend_path(&format!("/knowledge/indexes/{}/rebuild", serialize_path_parameter(index_id, PathParameterSpec::new("indexId", "simple", false))));
        self.client.post(&path, Some(body), None, None, Some("application/json")).await
    }

    /// List OKF bundle files
    pub async fn okf_bundle_files_list(&self, cursor: Option<&str>, page_size: Option<i64>) -> Result<serde_json::Value, SdkworkError> {
        let query = build_query_string(&[
            QueryParameterSpec::new("cursor", cursor, "form", true, false, None),
            QueryParameterSpec::new("page_size", page_size, "form", true, false, None),
        ]);
        let path = append_query_string(backend_path(&"/knowledge/okf/bundle/files".to_string()), &query);
        self.client.get(&path, None, None).await
    }

    /// List OKF candidates
    pub async fn okf_candidates_list(&self, space_id: i64, cursor: Option<&str>, page_size: Option<i64>) -> Result<serde_json::Value, SdkworkError> {
        let query = build_query_string(&[
            QueryParameterSpec::new("spaceId", space_id, "form", true, false, None),
            QueryParameterSpec::new("cursor", cursor, "form", true, false, None),
            QueryParameterSpec::new("page_size", page_size, "form", true, false, None),
        ]);
        let path = append_query_string(backend_path(&"/knowledge/okf/candidates".to_string()), &query);
        self.client.get(&path, None, None).await
    }

    /// Approve an OKF candidate
    pub async fn okf_candidates_approve(&self, candidate_id: i64, body: &OkfCandidateReviewRequest) -> Result<OkfCandidateResult, SdkworkError> {
        let path = backend_path(&format!("/knowledge/okf/candidates/{}/approve", serialize_path_parameter(candidate_id, PathParameterSpec::new("candidateId", "simple", false))));
        self.client.post(&path, Some(body), None, None, Some("application/json")).await
    }

    /// Reject an OKF candidate
    pub async fn okf_candidates_reject(&self, candidate_id: i64, body: &OkfCandidateReviewRequest) -> Result<OkfCandidateResult, SdkworkError> {
        let path = backend_path(&format!("/knowledge/okf/candidates/{}/reject", serialize_path_parameter(candidate_id, PathParameterSpec::new("candidateId", "simple", false))));
        self.client.post(&path, Some(body), None, None, Some("application/json")).await
    }

    /// Create an OKF compile job
    pub async fn okf_compile_jobs_create(&self, body: &OkfCompileJobRequest) -> Result<IngestionJob, SdkworkError> {
        let path = backend_path(&"/knowledge/okf/compile_jobs".to_string());
        self.client.post(&path, Some(body), None, None, Some("application/json")).await
    }

    /// Publish an OKF concept
    pub async fn okf_concepts_publish(&self, concept_id: i64, body: &OkfConceptPublishRequest) -> Result<OkfConceptSummary, SdkworkError> {
        let path = backend_path(&format!("/knowledge/okf/concepts/{}/publish", serialize_path_parameter(concept_id, PathParameterSpec::new("conceptId", "simple", false))));
        self.client.post(&path, Some(body), None, None, Some("application/json")).await
    }

    /// Create an OKF eval run
    pub async fn okf_eval_runs_create(&self, body: &OkfQualityRunRequest) -> Result<OkfQualityRun, SdkworkError> {
        let path = backend_path(&"/knowledge/okf/eval_runs".to_string());
        self.client.post(&path, Some(body), None, None, Some("application/json")).await
    }

    /// Create an OKF bundle export
    pub async fn okf_bundle_export_create(&self, body: &OkfBundleExportRequest) -> Result<KnowledgeOkfBundleFile, SdkworkError> {
        let path = backend_path(&"/knowledge/okf/exports".to_string());
        self.client.post(&path, Some(body), None, None, Some("application/json")).await
    }

    /// Retrieve an OKF bundle export
    pub async fn okf_bundle_export_retrieve(&self, export_id: i64) -> Result<KnowledgeOkfBundleFile, SdkworkError> {
        let path = backend_path(&format!("/knowledge/okf/exports/{}", serialize_path_parameter(export_id, PathParameterSpec::new("exportId", "simple", false))));
        self.client.get(&path, None, None).await
    }

    /// Import an OKF bundle from drive staging
    pub async fn okf_bundle_import_create(&self, body: &OkfBundleImportRequest) -> Result<OkfBundleImportResult, SdkworkError> {
        let path = backend_path(&"/knowledge/okf/imports".to_string());
        self.client.post(&path, Some(body), None, None, Some("application/json")).await
    }

    /// Rebuild the OKF bundle index
    pub async fn okf_bundle_index_rebuild(&self, body: &OkfBundleIndexRebuildRequest) -> Result<OkfIndexDocument, SdkworkError> {
        let path = backend_path(&"/knowledge/okf/index/rebuild".to_string());
        self.client.post(&path, Some(body), None, None, Some("application/json")).await
    }

    /// Create an OKF lint run
    pub async fn okf_lint_runs_create(&self, body: &OkfQualityRunRequest) -> Result<OkfQualityRun, SdkworkError> {
        let path = backend_path(&"/knowledge/okf/lint_runs".to_string());
        self.client.post(&path, Some(body), None, None, Some("application/json")).await
    }

    /// Create an OKF log entry
    pub async fn okf_log_entries_create(&self, body: &OkfLogEntry) -> Result<OkfLogEntry, SdkworkError> {
        let path = backend_path(&"/knowledge/okf/log_entries".to_string());
        self.client.post(&path, Some(body), None, None, Some("application/json")).await
    }

    /// Create an OKF profile
    pub async fn okf_profile_create(&self, body: &KnowledgeOkfProfileRequest) -> Result<KnowledgeOkfBundleFile, SdkworkError> {
        let path = backend_path(&"/knowledge/okf/profile".to_string());
        self.client.post(&path, Some(body), None, None, Some("application/json")).await
    }

    /// Update an OKF profile
    pub async fn okf_profile_update(&self, profile_id: i64, body: &KnowledgeOkfProfileRequest) -> Result<KnowledgeOkfBundleFile, SdkworkError> {
        let path = backend_path(&format!("/knowledge/okf/profile/{}", serialize_path_parameter(profile_id, PathParameterSpec::new("profileId", "simple", false))));
        self.client.patch(&path, Some(body), None, None, Some("application/json")).await
    }

    /// List Provider credential references
    pub async fn provider_credential_references_list(&self, implementation_id: Option<&str>, rotation_state: Option<&str>, cursor: Option<&str>, page_size: Option<i64>) -> Result<KnowledgeEngineProviderCredentialReferencePage, SdkworkError> {
        let query = build_query_string(&[
            QueryParameterSpec::new("implementation_id", implementation_id, "form", true, false, None),
            QueryParameterSpec::new("rotation_state", rotation_state, "form", true, false, None),
            QueryParameterSpec::new("cursor", cursor, "form", true, false, None),
            QueryParameterSpec::new("page_size", page_size, "form", true, false, None),
        ]);
        let path = append_query_string(backend_path(&"/knowledge/provider_credential_references".to_string()), &query);
        self.client.get(&path, None, None).await
    }

    /// Create a Provider credential reference
    pub async fn provider_credential_references_create(&self, body: &CreateKnowledgeEngineProviderCredentialReferenceRequest) -> Result<KnowledgeEngineProviderCredentialReference, SdkworkError> {
        let path = backend_path(&"/knowledge/provider_credential_references".to_string());
        self.client.post(&path, Some(body), None, None, Some("application/json")).await
    }

    /// Retrieve a Provider credential reference
    pub async fn provider_credential_references_retrieve(&self, credential_reference_id: &str) -> Result<KnowledgeEngineProviderCredentialReference, SdkworkError> {
        let path = backend_path(&format!("/knowledge/provider_credential_references/{}", serialize_path_parameter(credential_reference_id, PathParameterSpec::new("credentialReferenceId", "simple", false))));
        self.client.get(&path, None, None).await
    }

    /// Revoke a Provider credential reference
    pub async fn provider_credential_references_revoke(&self, credential_reference_id: &str, body: &RevokeKnowledgeEngineProviderCredentialReferenceRequest) -> Result<SdkWorkCommandData, SdkworkError> {
        let path = backend_path(&format!("/knowledge/provider_credential_references/{}/revoke", serialize_path_parameter(credential_reference_id, PathParameterSpec::new("credentialReferenceId", "simple", false))));
        self.client.post(&path, Some(body), None, None, Some("application/json")).await
    }

    /// Rotate a Provider credential reference
    pub async fn provider_credential_references_rotate(&self, credential_reference_id: &str, body: &RotateKnowledgeEngineProviderCredentialReferenceRequest) -> Result<SdkWorkCommandData, SdkworkError> {
        let path = backend_path(&format!("/knowledge/provider_credential_references/{}/rotate", serialize_path_parameter(credential_reference_id, PathParameterSpec::new("credentialReferenceId", "simple", false))));
        self.client.post(&path, Some(body), None, None, Some("application/json")).await
    }

    /// Retrieve provider health status
    pub async fn provider_health_list(&self) -> Result<KnowledgeProviderHealth, SdkworkError> {
        let path = backend_path(&"/knowledge/provider_health".to_string());
        self.client.get(&path, None, None).await
    }

    /// Create a retrieval profile
    pub async fn retrieval_profiles_create(&self, body: &KnowledgeRetrievalProfileRequest) -> Result<KnowledgeRetrievalProfile, SdkworkError> {
        let path = backend_path(&"/knowledge/retrieval_profiles".to_string());
        self.client.post(&path, Some(body), None, None, Some("application/json")).await
    }

    /// Retrieve a retrieval profile
    pub async fn retrieval_profiles_retrieve(&self, profile_id: &str) -> Result<KnowledgeRetrievalProfile, SdkworkError> {
        let path = backend_path(&format!("/knowledge/retrieval_profiles/{}", serialize_path_parameter(profile_id, PathParameterSpec::new("profileId", "simple", false))));
        self.client.get(&path, None, None).await
    }

    /// Update a retrieval profile
    pub async fn retrieval_profiles_update(&self, profile_id: &str, body: &KnowledgeRetrievalProfileRequest) -> Result<KnowledgeRetrievalProfile, SdkworkError> {
        let path = backend_path(&format!("/knowledge/retrieval_profiles/{}", serialize_path_parameter(profile_id, PathParameterSpec::new("profileId", "simple", false))));
        self.client.patch(&path, Some(body), None, None, Some("application/json")).await
    }

    /// List retrieval traces
    pub async fn retrieval_traces_list(&self, cursor: Option<&str>, page_size: Option<i64>) -> Result<serde_json::Value, SdkworkError> {
        let query = build_query_string(&[
            QueryParameterSpec::new("cursor", cursor, "form", true, false, None),
            QueryParameterSpec::new("page_size", page_size, "form", true, false, None),
        ]);
        let path = append_query_string(backend_path(&"/knowledge/retrieval_traces".to_string()), &query);
        self.client.get(&path, None, None).await
    }

    /// Retrieve a retrieval trace
    pub async fn retrieval_traces_retrieve(&self, trace_id: &str) -> Result<KnowledgeRetrievalTrace, SdkworkError> {
        let path = backend_path(&format!("/knowledge/retrieval_traces/{}", serialize_path_parameter(trace_id, PathParameterSpec::new("traceId", "simple", false))));
        self.client.get(&path, None, None).await
    }

    /// List knowledge sources
    pub async fn sources_list(&self, cursor: Option<&str>, page_size: Option<i64>) -> Result<serde_json::Value, SdkworkError> {
        let query = build_query_string(&[
            QueryParameterSpec::new("cursor", cursor, "form", true, false, None),
            QueryParameterSpec::new("page_size", page_size, "form", true, false, None),
        ]);
        let path = append_query_string(backend_path(&"/knowledge/sources".to_string()), &query);
        self.client.get(&path, None, None).await
    }

    /// Create a knowledge source
    pub async fn sources_create(&self, body: &CreateKnowledgeSourceRequest) -> Result<KnowledgeSource, SdkworkError> {
        let path = backend_path(&"/knowledge/sources".to_string());
        self.client.post(&path, Some(body), None, None, Some("application/json")).await
    }

    /// List knowledge spaces
    pub async fn spaces_list(&self, cursor: Option<&str>, page_size: Option<i64>) -> Result<serde_json::Value, SdkworkError> {
        let query = build_query_string(&[
            QueryParameterSpec::new("cursor", cursor, "form", true, false, None),
            QueryParameterSpec::new("page_size", page_size, "form", true, false, None),
        ]);
        let path = append_query_string(backend_path(&"/knowledge/spaces".to_string()), &query);
        self.client.get(&path, None, None).await
    }

    /// List knowledge space members
    pub async fn spaces_members_list(&self, space_id: &str, cursor: Option<&str>, page_size: Option<i64>) -> Result<serde_json::Value, SdkworkError> {
        let query = build_query_string(&[
            QueryParameterSpec::new("cursor", cursor, "form", true, false, None),
            QueryParameterSpec::new("page_size", page_size, "form", true, false, None),
        ]);
        let path = append_query_string(backend_path(&format!("/knowledge/spaces/{}/members", serialize_path_parameter(space_id, PathParameterSpec::new("spaceId", "simple", false)))), &query);
        self.client.get(&path, None, None).await
    }

    /// List Provider bindings for a knowledge space
    pub async fn spaces_provider_bindings_list(&self, space_id: &str, lifecycle_state: Option<&str>, cursor: Option<&str>, page_size: Option<i64>) -> Result<KnowledgeEngineProviderBindingPage, SdkworkError> {
        let query = build_query_string(&[
            QueryParameterSpec::new("lifecycle_state", lifecycle_state, "form", true, false, None),
            QueryParameterSpec::new("cursor", cursor, "form", true, false, None),
            QueryParameterSpec::new("page_size", page_size, "form", true, false, None),
        ]);
        let path = append_query_string(backend_path(&format!("/knowledge/spaces/{}/provider_bindings", serialize_path_parameter(space_id, PathParameterSpec::new("spaceId", "simple", false)))), &query);
        self.client.get(&path, None, None).await
    }

    /// Create a Provider binding for a knowledge space
    pub async fn spaces_provider_bindings_create(&self, space_id: &str, body: &CreateKnowledgeEngineProviderBindingRequest) -> Result<KnowledgeEngineProviderBinding, SdkworkError> {
        let path = backend_path(&format!("/knowledge/spaces/{}/provider_bindings", serialize_path_parameter(space_id, PathParameterSpec::new("spaceId", "simple", false))));
        self.client.post(&path, Some(body), None, None, Some("application/json")).await
    }

    /// Retrieve a Provider binding
    pub async fn spaces_provider_bindings_retrieve(&self, space_id: &str, binding_id: &str) -> Result<KnowledgeEngineProviderBinding, SdkworkError> {
        let path = backend_path(&format!("/knowledge/spaces/{}/provider_bindings/{}", serialize_path_parameter(space_id, PathParameterSpec::new("spaceId", "simple", false)), serialize_path_parameter(binding_id, PathParameterSpec::new("bindingId", "simple", false))));
        self.client.get(&path, None, None).await
    }

    /// Update a draft Provider binding
    pub async fn spaces_provider_bindings_update(&self, space_id: &str, binding_id: &str, body: &UpdateKnowledgeEngineProviderBindingRequest) -> Result<KnowledgeEngineProviderBinding, SdkworkError> {
        let path = backend_path(&format!("/knowledge/spaces/{}/provider_bindings/{}", serialize_path_parameter(space_id, PathParameterSpec::new("spaceId", "simple", false)), serialize_path_parameter(binding_id, PathParameterSpec::new("bindingId", "simple", false))));
        self.client.patch(&path, Some(body), None, None, Some("application/json")).await
    }

    /// Activate a Provider binding
    pub async fn spaces_provider_bindings_activate(&self, space_id: &str, binding_id: &str, body: &ProviderBindingVersionCommandRequest) -> Result<SdkWorkCommandData, SdkworkError> {
        let path = backend_path(&format!("/knowledge/spaces/{}/provider_bindings/{}/activate", serialize_path_parameter(space_id, PathParameterSpec::new("spaceId", "simple", false)), serialize_path_parameter(binding_id, PathParameterSpec::new("bindingId", "simple", false))));
        self.client.post(&path, Some(body), None, None, Some("application/json")).await
    }

    /// Disable a Provider binding
    pub async fn spaces_provider_bindings_disable(&self, space_id: &str, binding_id: &str, body: &ProviderBindingVersionCommandRequest) -> Result<SdkWorkCommandData, SdkworkError> {
        let path = backend_path(&format!("/knowledge/spaces/{}/provider_bindings/{}/disable", serialize_path_parameter(space_id, PathParameterSpec::new("spaceId", "simple", false)), serialize_path_parameter(binding_id, PathParameterSpec::new("bindingId", "simple", false))));
        self.client.post(&path, Some(body), None, None, Some("application/json")).await
    }

    /// Test a Provider binding
    pub async fn spaces_provider_bindings_test(&self, space_id: &str, binding_id: &str, body: &ProviderBindingVersionCommandRequest) -> Result<SdkWorkCommandData, SdkworkError> {
        let path = backend_path(&format!("/knowledge/spaces/{}/provider_bindings/{}/test", serialize_path_parameter(space_id, PathParameterSpec::new("spaceId", "simple", false)), serialize_path_parameter(binding_id, PathParameterSpec::new("bindingId", "simple", false))));
        self.client.post(&path, Some(body), None, None, Some("application/json")).await
    }

    /// List Provider migration operations for a knowledge space
    pub async fn spaces_provider_migrations_list(&self, space_id: &str, operation_state: Option<&str>, cursor: Option<&str>, page_size: Option<i64>) -> Result<KnowledgeEngineProviderMigrationOperationPage, SdkworkError> {
        let query = build_query_string(&[
            QueryParameterSpec::new("operation_state", operation_state, "form", true, false, None),
            QueryParameterSpec::new("cursor", cursor, "form", true, false, None),
            QueryParameterSpec::new("page_size", page_size, "form", true, false, None),
        ]);
        let path = append_query_string(backend_path(&format!("/knowledge/spaces/{}/provider_migrations", serialize_path_parameter(space_id, PathParameterSpec::new("spaceId", "simple", false)))), &query);
        self.client.get(&path, None, None).await
    }

    /// Create a recoverable Provider migration operation
    pub async fn spaces_provider_migrations_create(&self, space_id: &str, body: &CreateKnowledgeEngineProviderMigrationOperationRequest) -> Result<KnowledgeEngineProviderMigrationOperation, SdkworkError> {
        let path = backend_path(&format!("/knowledge/spaces/{}/provider_migrations", serialize_path_parameter(space_id, PathParameterSpec::new("spaceId", "simple", false))));
        self.client.post(&path, Some(body), None, None, Some("application/json")).await
    }

    /// Retrieve a Provider migration operation
    pub async fn spaces_provider_migrations_retrieve(&self, space_id: &str, migration_operation_id: &str) -> Result<KnowledgeEngineProviderMigrationOperation, SdkworkError> {
        let path = backend_path(&format!("/knowledge/spaces/{}/provider_migrations/{}", serialize_path_parameter(space_id, PathParameterSpec::new("spaceId", "simple", false)), serialize_path_parameter(migration_operation_id, PathParameterSpec::new("migrationOperationId", "simple", false))));
        self.client.get(&path, None, None).await
    }

    /// Request rollback of a Provider migration operation
    pub async fn spaces_provider_migrations_rollback(&self, space_id: &str, migration_operation_id: &str, body: &ProviderMigrationVersionCommandRequest) -> Result<SdkWorkCommandData, SdkworkError> {
        let path = backend_path(&format!("/knowledge/spaces/{}/provider_migrations/{}/rollback", serialize_path_parameter(space_id, PathParameterSpec::new("spaceId", "simple", false)), serialize_path_parameter(migration_operation_id, PathParameterSpec::new("migrationOperationId", "simple", false))));
        self.client.post(&path, Some(body), None, None, Some("application/json")).await
    }

    /// Retrieve current tenant knowledgebase status
    pub async fn tenants_current_list(&self) -> Result<KnowledgeTenantStatus, SdkworkError> {
        let path = backend_path(&"/knowledge/tenants/current".to_string());
        self.client.get(&path, None, None).await
    }

}

struct PathParameterSpec<'a> {
    name: &'a str,
    style: &'a str,
    explode: bool,
}

impl<'a> PathParameterSpec<'a> {
    fn new(name: &'a str, style: &'a str, explode: bool) -> Self {
        Self { name, style, explode }
    }
}

fn serialize_path_parameter<T: serde::Serialize>(value: T, spec: PathParameterSpec<'_>) -> String {
    let value = serde_json::to_value(value).unwrap_or(serde_json::Value::Null);
    if value.is_null() {
        return String::new();
    }
    let style = if spec.style.is_empty() { "simple" } else { spec.style };
    match value {
        serde_json::Value::Array(values) => serialize_path_array(spec.name, &values, style, spec.explode),
        serde_json::Value::Object(values) => serialize_path_object(spec.name, &values, style, spec.explode),
        value => format!("{}{}", path_primitive_prefix(spec.name, style), percent_encode(&primitive_to_string(&value))),
    }
}

fn serialize_path_array(name: &str, values: &[serde_json::Value], style: &str, explode: bool) -> String {
    let serialized = values
        .iter()
        .filter(|value| !value.is_null())
        .map(|value| percent_encode(&primitive_to_string(value)))
        .collect::<Vec<_>>();
    if serialized.is_empty() {
        return path_prefix(name, style);
    }
    if style == "matrix" {
        if explode {
            return serialized.iter().map(|item| format!(";{}={}", name, item)).collect::<Vec<_>>().join("");
        }
        return format!(";{}={}", name, serialized.join(","));
    }
    let separator = if explode { "." } else { "," };
    format!("{}{}", path_prefix(name, style), serialized.join(separator))
}

fn serialize_path_object(
    name: &str,
    values: &serde_json::Map<String, serde_json::Value>,
    style: &str,
    explode: bool,
) -> String {
    let mut entries = Vec::new();
    let mut exploded = Vec::new();
    for (key, value) in values {
        if value.is_null() {
            continue;
        }
        let escaped_key = percent_encode(key);
        let escaped_value = percent_encode(&primitive_to_string(value));
        if explode {
            if style == "matrix" {
                exploded.push(format!(";{}={}", escaped_key, escaped_value));
            } else {
                exploded.push(format!("{}={}", escaped_key, escaped_value));
            }
        } else {
            entries.push(escaped_key);
            entries.push(escaped_value);
        }
    }
    if style == "matrix" {
        if explode {
            return exploded.join("");
        }
        return format!(";{}={}", name, entries.join(","));
    }
    if explode {
        let separator = if style == "label" { "." } else { "," };
        return format!("{}{}", path_prefix(name, style), exploded.join(separator));
    }
    format!("{}{}", path_prefix(name, style), entries.join(","))
}

fn path_prefix(name: &str, style: &str) -> String {
    match style {
        "label" => ".".to_string(),
        "matrix" => format!(";{}", name),
        _ => String::new(),
    }
}

fn path_primitive_prefix(name: &str, style: &str) -> String {
    if style == "matrix" {
        format!(";{}=", name)
    } else {
        path_prefix(name, style)
    }
}


struct QueryParameterSpec<'a> {
    name: &'a str,
    value: serde_json::Value,
    style: &'a str,
    explode: bool,
    allow_reserved: bool,
    content_type: Option<&'a str>,
}

impl<'a> QueryParameterSpec<'a> {
    fn new<T: serde::Serialize>(
        name: &'a str,
        value: T,
        style: &'a str,
        explode: bool,
        allow_reserved: bool,
        content_type: Option<&'a str>,
    ) -> Self {
        Self {
            name,
            value: serde_json::to_value(value).unwrap_or(serde_json::Value::Null),
            style,
            explode,
            allow_reserved,
            content_type,
        }
    }
}

fn build_query_string(parameters: &[QueryParameterSpec<'_>]) -> String {
    let mut pairs = Vec::new();
    for parameter in parameters {
        append_serialized_parameter(&mut pairs, parameter);
    }
    pairs.join("&")
}

fn append_serialized_parameter(pairs: &mut Vec<String>, parameter: &QueryParameterSpec<'_>) {
    if parameter.value.is_null() {
        return;
    }
    if parameter.content_type.is_some() {
        pairs.push(format!(
            "{}={}",
            percent_encode(parameter.name),
            encode_query_value(&parameter.value.to_string(), parameter.allow_reserved)
        ));
        return;
    }

    let style = if parameter.style.is_empty() { "form" } else { parameter.style };
    match &parameter.value {
        serde_json::Value::Array(values) => append_array_parameter(pairs, parameter.name, values, style, parameter.explode, parameter.allow_reserved),
        serde_json::Value::Object(values) if style == "deepObject" => append_deep_object_parameter(pairs, parameter.name, values, parameter.allow_reserved),
        serde_json::Value::Object(values) => append_object_parameter(pairs, parameter.name, values, style, parameter.explode, parameter.allow_reserved),
        value => pairs.push(format!("{}={}", percent_encode(parameter.name), encode_query_value(&primitive_to_string(value), parameter.allow_reserved))),
    }
}

fn append_array_parameter(
    pairs: &mut Vec<String>,
    name: &str,
    values: &[serde_json::Value],
    style: &str,
    explode: bool,
    allow_reserved: bool,
) {
    let serialized = values.iter().filter(|value| !value.is_null()).map(primitive_to_string).collect::<Vec<_>>();
    if serialized.is_empty() {
        return;
    }
    if style == "form" && explode {
        for item in serialized {
            pairs.push(format!("{}={}", percent_encode(name), encode_query_value(&item, allow_reserved)));
        }
        return;
    }
    pairs.push(format!("{}={}", percent_encode(name), encode_query_value(&serialized.join(","), allow_reserved)));
}

fn append_object_parameter(
    pairs: &mut Vec<String>,
    name: &str,
    values: &serde_json::Map<String, serde_json::Value>,
    style: &str,
    explode: bool,
    allow_reserved: bool,
) {
    let mut serialized = Vec::new();
    for (key, value) in values {
        if value.is_null() {
            continue;
        }
        if style == "form" && explode {
            pairs.push(format!("{}={}", percent_encode(key), encode_query_value(&primitive_to_string(value), allow_reserved)));
        } else {
            serialized.push(key.clone());
            serialized.push(primitive_to_string(value));
        }
    }
    if !serialized.is_empty() {
        pairs.push(format!("{}={}", percent_encode(name), encode_query_value(&serialized.join(","), allow_reserved)));
    }
}

fn append_deep_object_parameter(
    pairs: &mut Vec<String>,
    name: &str,
    values: &serde_json::Map<String, serde_json::Value>,
    allow_reserved: bool,
) {
    for (key, value) in values {
        if !value.is_null() {
            pairs.push(format!("{}={}", percent_encode(&format!("{}[{}]", name, key)), encode_query_value(&primitive_to_string(value), allow_reserved)));
        }
    }
}

fn encode_query_value(value: &str, allow_reserved: bool) -> String {
    let mut encoded = percent_encode(value);
    if !allow_reserved {
        return encoded;
    }
    for (escaped, reserved) in [
        ("%3A", ":"), ("%2F", "/"), ("%3F", "?"), ("%23", "#"),
        ("%5B", "["), ("%5D", "]"), ("%40", "@"), ("%21", "!"),
        ("%24", "$"), ("%26", "&"), ("%27", "'"), ("%28", "("),
        ("%29", ")"), ("%2A", "*"), ("%2B", "+"), ("%2C", ","),
        ("%3B", ";"), ("%3D", "="),
    ] {
        encoded = encoded.replace(escaped, reserved);
    }
    encoded
}

fn primitive_to_string(value: &serde_json::Value) -> String {
    match value {
        serde_json::Value::String(value) => value.clone(),
        serde_json::Value::Number(value) => value.to_string(),
        serde_json::Value::Bool(value) => value.to_string(),
        other => other.to_string(),
    }
}

fn percent_encode(value: &str) -> String {
    value
        .bytes()
        .flat_map(|byte| match byte {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' => {
                vec![byte as char]
            }
            _ => format!("%{:02X}", byte).chars().collect(),
        })
        .collect()
}
