pub const PREFIX: &str = "/backend/v3/api";
pub const LIVEZ: &str = "/livez";
pub const READYZ: &str = "/readyz";
pub const HEALTHZ: &str = "/healthz";
pub const SOURCES: &str = "/backend/v3/api/knowledge/sources";
pub const OKF_COMPILE_JOBS: &str = "/backend/v3/api/knowledge/okf/compile_jobs";
pub const OKF_CANDIDATES: &str = "/backend/v3/api/knowledge/okf/candidates";
pub const OKF_CANDIDATE_APPROVE: &str =
    "/backend/v3/api/knowledge/okf/candidates/{candidate_id}/approve";
pub const OKF_CANDIDATE_REJECT: &str =
    "/backend/v3/api/knowledge/okf/candidates/{candidate_id}/reject";
pub const OKF_CONCEPT_PUBLISH: &str = "/backend/v3/api/knowledge/okf/concepts/{concept_id}/publish";
pub const OKF_PROFILES: &str = "/backend/v3/api/knowledge/okf/profile";
pub const OKF_PROFILE: &str = "/backend/v3/api/knowledge/okf/profile/{profile_id}";
pub const OKF_INDEX_REBUILD: &str = "/backend/v3/api/knowledge/okf/index/rebuild";
pub const OKF_LOG_ENTRIES: &str = "/backend/v3/api/knowledge/okf/log_entries";
pub const OKF_EXPORTS: &str = "/backend/v3/api/knowledge/okf/exports";
pub const OKF_EXPORT: &str = "/backend/v3/api/knowledge/okf/exports/{export_id}";
pub const OKF_IMPORTS: &str = "/backend/v3/api/knowledge/okf/imports";
pub const OKF_BUNDLE_FILES: &str = "/backend/v3/api/knowledge/okf/bundle/files";
pub const OKF_LINT_RUNS: &str = "/backend/v3/api/knowledge/okf/lint_runs";
pub const OKF_EVAL_RUNS: &str = "/backend/v3/api/knowledge/okf/eval_runs";
pub const INDEXES: &str = "/backend/v3/api/knowledge/indexes";
pub const INDEX: &str = "/backend/v3/api/knowledge/indexes/{index_id}";
pub const INDEX_REBUILD: &str = "/backend/v3/api/knowledge/indexes/{index_id}/rebuild";
pub const RETRIEVAL_PROFILES: &str = "/backend/v3/api/knowledge/retrieval_profiles";
pub const RETRIEVAL_PROFILE: &str = "/backend/v3/api/knowledge/retrieval_profiles/{profile_id}";
pub const RETRIEVAL_TRACES: &str = "/backend/v3/api/knowledge/retrieval_traces";
pub const RETRIEVAL_TRACE: &str = "/backend/v3/api/knowledge/retrieval_traces/{trace_id}";
pub const PROVIDER_HEALTH: &str = "/backend/v3/api/knowledge/provider_health";
pub const PROVIDER_CREDENTIAL_REFERENCES: &str =
    "/backend/v3/api/knowledge/provider_credential_references";
pub const PROVIDER_CREDENTIAL_REFERENCE: &str =
    "/backend/v3/api/knowledge/provider_credential_references/{credential_reference_id}";
pub const PROVIDER_CREDENTIAL_REFERENCE_ROTATE: &str =
    "/backend/v3/api/knowledge/provider_credential_references/{credential_reference_id}/rotate";
pub const PROVIDER_CREDENTIAL_REFERENCE_REVOKE: &str =
    "/backend/v3/api/knowledge/provider_credential_references/{credential_reference_id}/revoke";
pub const SPACE_PROVIDER_BINDINGS: &str =
    "/backend/v3/api/knowledge/spaces/{space_id}/provider_bindings";
pub const SPACE_PROVIDER_BINDING: &str =
    "/backend/v3/api/knowledge/spaces/{space_id}/provider_bindings/{binding_id}";
pub const SPACE_PROVIDER_BINDING_TEST: &str =
    "/backend/v3/api/knowledge/spaces/{space_id}/provider_bindings/{binding_id}/test";
pub const SPACE_PROVIDER_BINDING_ACTIVATE: &str =
    "/backend/v3/api/knowledge/spaces/{space_id}/provider_bindings/{binding_id}/activate";
pub const SPACE_PROVIDER_BINDING_DISABLE: &str =
    "/backend/v3/api/knowledge/spaces/{space_id}/provider_bindings/{binding_id}/disable";
pub const SPACE_PROVIDER_MIGRATIONS: &str =
    "/backend/v3/api/knowledge/spaces/{space_id}/provider_migrations";
pub const SPACE_PROVIDER_MIGRATION: &str =
    "/backend/v3/api/knowledge/spaces/{space_id}/provider_migrations/{migration_operation_id}";
pub const SPACE_PROVIDER_MIGRATION_ROLLBACK: &str =
    "/backend/v3/api/knowledge/spaces/{space_id}/provider_migrations/{migration_operation_id}/rollback";
pub const GROUP_LAUNCH_CAPABILITY: &str = "/backend/v3/api/knowledge/group_launch_capability";
pub const TENANT_LANDING: &str = "/backend/v3/api/knowledge/tenants/current";
pub const SPACES: &str = "/backend/v3/api/knowledge/spaces";
pub const SPACE_MEMBERS: &str = "/backend/v3/api/knowledge/spaces/{space_id}/members";
pub const COMPLIANCE_AUDIT_EVENTS_EXPORT: &str =
    "/backend/v3/api/knowledge/compliance/audit_events/export";
pub const COMPLIANCE_AUDIT_EVENTS_ANONYMIZE: &str =
    "/backend/v3/api/knowledge/compliance/audit_events/anonymize_actor";
pub const COMPLIANCE_AUDIT_LEGAL_HOLDS: &str =
    "/backend/v3/api/knowledge/compliance/audit_legal_holds";
pub const COMPLIANCE_AUDIT_LEGAL_HOLD_RELEASE: &str =
    "/backend/v3/api/knowledge/compliance/audit_legal_holds/{hold_id}/release";
