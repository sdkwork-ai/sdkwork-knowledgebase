pub const PACKAGE_NAME: &str = "sdkwork-routes-knowledgebase-backend-api";
pub const SURFACE: &str = "backend-api";
pub const OWNER: &str = "sdkwork-knowledgebase";
pub const DOMAIN: &str = "intelligence";
pub const CAPABILITY: &str = "knowledgebase";
pub const API_AUTHORITY: &str = "sdkwork-knowledgebase-backend-api";
pub const SDK_FAMILY: &str = "sdkwork-knowledgebase-backend-sdk";
pub const PREFIX: &str = "/backend/v3/api";

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RouteManifestEntry {
    pub method: &'static str,
    pub path: &'static str,
    pub operation_id: &'static str,
}

pub const ROUTES: &[RouteManifestEntry] = &[
    RouteManifestEntry {
        method: "GET",
        path: "/backend/v3/api/knowledge/sources",
        operation_id: "sources.list",
    },
    RouteManifestEntry {
        method: "POST",
        path: "/backend/v3/api/knowledge/sources",
        operation_id: "sources.create",
    },
    RouteManifestEntry {
        method: "POST",
        path: "/backend/v3/api/knowledge/okf/compile_jobs",
        operation_id: "okf.compileJobs.create",
    },
    RouteManifestEntry {
        method: "GET",
        path: "/backend/v3/api/knowledge/okf/candidates",
        operation_id: "okf.candidates.list",
    },
    RouteManifestEntry {
        method: "POST",
        path: "/backend/v3/api/knowledge/okf/candidates/{candidateId}/approve",
        operation_id: "okf.candidates.approve",
    },
    RouteManifestEntry {
        method: "POST",
        path: "/backend/v3/api/knowledge/okf/candidates/{candidateId}/reject",
        operation_id: "okf.candidates.reject",
    },
    RouteManifestEntry {
        method: "POST",
        path: "/backend/v3/api/knowledge/okf/concepts/{conceptId}/publish",
        operation_id: "okf.concepts.publish",
    },
    RouteManifestEntry {
        method: "POST",
        path: "/backend/v3/api/knowledge/okf/profile",
        operation_id: "okf.profile.create",
    },
    RouteManifestEntry {
        method: "PATCH",
        path: "/backend/v3/api/knowledge/okf/profile/{profileId}",
        operation_id: "okf.profile.update",
    },
    RouteManifestEntry {
        method: "POST",
        path: "/backend/v3/api/knowledge/okf/index/rebuild",
        operation_id: "okf.bundle.index.rebuild",
    },
    RouteManifestEntry {
        method: "POST",
        path: "/backend/v3/api/knowledge/okf/log_entries",
        operation_id: "okf.log.entries.create",
    },
    RouteManifestEntry {
        method: "POST",
        path: "/backend/v3/api/knowledge/okf/exports",
        operation_id: "okf.bundle.export.create",
    },
    RouteManifestEntry {
        method: "GET",
        path: "/backend/v3/api/knowledge/okf/exports/{exportId}",
        operation_id: "okf.bundle.export.retrieve",
    },
    RouteManifestEntry {
        method: "POST",
        path: "/backend/v3/api/knowledge/okf/imports",
        operation_id: "okf.bundle.import.create",
    },
    RouteManifestEntry {
        method: "GET",
        path: "/backend/v3/api/knowledge/okf/bundle/files",
        operation_id: "okf.bundle.files.list",
    },
    RouteManifestEntry {
        method: "POST",
        path: "/backend/v3/api/knowledge/okf/lint_runs",
        operation_id: "okf.lintRuns.create",
    },
    RouteManifestEntry {
        method: "POST",
        path: "/backend/v3/api/knowledge/okf/eval_runs",
        operation_id: "okf.evalRuns.create",
    },
    RouteManifestEntry {
        method: "GET",
        path: "/backend/v3/api/knowledge/indexes",
        operation_id: "indexes.list",
    },
    RouteManifestEntry {
        method: "POST",
        path: "/backend/v3/api/knowledge/indexes",
        operation_id: "indexes.create",
    },
    RouteManifestEntry {
        method: "GET",
        path: "/backend/v3/api/knowledge/indexes/{indexId}",
        operation_id: "indexes.retrieve",
    },
    RouteManifestEntry {
        method: "POST",
        path: "/backend/v3/api/knowledge/indexes/{indexId}/rebuild",
        operation_id: "indexes.rebuild",
    },
    RouteManifestEntry {
        method: "POST",
        path: "/backend/v3/api/knowledge/retrieval_profiles",
        operation_id: "retrievalProfiles.create",
    },
    RouteManifestEntry {
        method: "GET",
        path: "/backend/v3/api/knowledge/retrieval_profiles/{profileId}",
        operation_id: "retrievalProfiles.retrieve",
    },
    RouteManifestEntry {
        method: "PATCH",
        path: "/backend/v3/api/knowledge/retrieval_profiles/{profileId}",
        operation_id: "retrievalProfiles.update",
    },
    RouteManifestEntry {
        method: "GET",
        path: "/backend/v3/api/knowledge/retrieval_traces",
        operation_id: "retrievalTraces.list",
    },
    RouteManifestEntry {
        method: "GET",
        path: "/backend/v3/api/knowledge/retrieval_traces/{traceId}",
        operation_id: "retrievalTraces.retrieve",
    },
    RouteManifestEntry {
        method: "GET",
        path: "/backend/v3/api/knowledge/provider_health",
        operation_id: "providerHealth.list",
    },
    RouteManifestEntry {
        method: "GET",
        path: "/backend/v3/api/knowledge/provider_credential_references",
        operation_id: "providerCredentialReferences.list",
    },
    RouteManifestEntry {
        method: "POST",
        path: "/backend/v3/api/knowledge/provider_credential_references",
        operation_id: "providerCredentialReferences.create",
    },
    RouteManifestEntry {
        method: "GET",
        path: "/backend/v3/api/knowledge/provider_credential_references/{credentialReferenceId}",
        operation_id: "providerCredentialReferences.retrieve",
    },
    RouteManifestEntry {
        method: "POST",
        path: "/backend/v3/api/knowledge/provider_credential_references/{credentialReferenceId}/rotate",
        operation_id: "providerCredentialReferences.rotate",
    },
    RouteManifestEntry {
        method: "POST",
        path: "/backend/v3/api/knowledge/provider_credential_references/{credentialReferenceId}/revoke",
        operation_id: "providerCredentialReferences.revoke",
    },
    RouteManifestEntry {
        method: "GET",
        path: "/backend/v3/api/knowledge/spaces/{spaceId}/provider_bindings",
        operation_id: "spaces.providerBindings.list",
    },
    RouteManifestEntry {
        method: "POST",
        path: "/backend/v3/api/knowledge/spaces/{spaceId}/provider_bindings",
        operation_id: "spaces.providerBindings.create",
    },
    RouteManifestEntry {
        method: "GET",
        path: "/backend/v3/api/knowledge/spaces/{spaceId}/provider_bindings/{bindingId}",
        operation_id: "spaces.providerBindings.retrieve",
    },
    RouteManifestEntry {
        method: "PATCH",
        path: "/backend/v3/api/knowledge/spaces/{spaceId}/provider_bindings/{bindingId}",
        operation_id: "spaces.providerBindings.update",
    },
    RouteManifestEntry {
        method: "POST",
        path: "/backend/v3/api/knowledge/spaces/{spaceId}/provider_bindings/{bindingId}/test",
        operation_id: "spaces.providerBindings.test",
    },
    RouteManifestEntry {
        method: "POST",
        path: "/backend/v3/api/knowledge/spaces/{spaceId}/provider_bindings/{bindingId}/activate",
        operation_id: "spaces.providerBindings.activate",
    },
    RouteManifestEntry {
        method: "POST",
        path: "/backend/v3/api/knowledge/spaces/{spaceId}/provider_bindings/{bindingId}/disable",
        operation_id: "spaces.providerBindings.disable",
    },
    RouteManifestEntry {
        method: "GET",
        path: "/backend/v3/api/knowledge/spaces/{spaceId}/provider_migrations",
        operation_id: "spaces.providerMigrations.list",
    },
    RouteManifestEntry {
        method: "POST",
        path: "/backend/v3/api/knowledge/spaces/{spaceId}/provider_migrations",
        operation_id: "spaces.providerMigrations.create",
    },
    RouteManifestEntry {
        method: "GET",
        path: "/backend/v3/api/knowledge/spaces/{spaceId}/provider_migrations/{migrationOperationId}",
        operation_id: "spaces.providerMigrations.retrieve",
    },
    RouteManifestEntry {
        method: "POST",
        path: "/backend/v3/api/knowledge/spaces/{spaceId}/provider_migrations/{migrationOperationId}/rollback",
        operation_id: "spaces.providerMigrations.rollback",
    },
    RouteManifestEntry {
        method: "GET",
        path: "/backend/v3/api/knowledge/group_launch_capability",
        operation_id: "group.launch.capability",
    },
    RouteManifestEntry {
        method: "GET",
        path: "/backend/v3/api/knowledge/tenants/current",
        operation_id: "tenants.current.list",
    },
    RouteManifestEntry {
        method: "GET",
        path: "/backend/v3/api/knowledge/spaces",
        operation_id: "spaces.list",
    },
    RouteManifestEntry {
        method: "GET",
        path: "/backend/v3/api/knowledge/spaces/{spaceId}/members",
        operation_id: "spaces.members.list",
    },
    RouteManifestEntry {
        method: "POST",
        path: "/backend/v3/api/knowledge/compliance/audit_events/export",
        operation_id: "compliance.auditEvents.export.create",
    },
    RouteManifestEntry {
        method: "POST",
        path: "/backend/v3/api/knowledge/compliance/audit_events/anonymize_actor",
        operation_id: "compliance.auditEvents.anonymizeActor.create",
    },
    RouteManifestEntry {
        method: "GET",
        path: "/backend/v3/api/knowledge/compliance/audit_legal_holds",
        operation_id: "compliance.auditLegalHolds.list",
    },
    RouteManifestEntry {
        method: "POST",
        path: "/backend/v3/api/knowledge/compliance/audit_legal_holds",
        operation_id: "compliance.auditLegalHolds.create",
    },
    RouteManifestEntry {
        method: "POST",
        path: "/backend/v3/api/knowledge/compliance/audit_legal_holds/{holdId}/release",
        operation_id: "compliance.auditLegalHolds.release.create",
    },
];
