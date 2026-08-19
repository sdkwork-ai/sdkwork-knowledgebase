//! Request-scoped repository stores derived from authenticated token context.

use sdkwork_intelligence_knowledgebase_repository_sqlx::{
    PostgresDriveImportMetadataStore, PostgresIngestionJobStore,
    PostgresKnowledgeBrowserProjectionStore, PostgresKnowledgeDocumentStore,
    PostgresKnowledgeDriveObjectRefStore, PostgresKnowledgeOkfConceptStore,
    PostgresKnowledgeSourceStore, PostgresKnowledgeSpaceStore,
    PostgresMarkdownIndexMetadataStore,
};
use sdkwork_routes_knowledgebase_backend_api::{knowledge_data_scope, KnowledgeDataScope};

use crate::{runtime::KnowledgebaseRuntime, KnowledgeAppRequestContext};

pub(crate) fn request_data_scope(context: &KnowledgeAppRequestContext) -> KnowledgeDataScope {
    knowledge_data_scope(context.tenant_id, context.organization_id)
}

pub(crate) struct RequestScopedStores {
    pub scope: KnowledgeDataScope,
    pub space_store: PostgresKnowledgeSpaceStore,
    pub document_store: PostgresKnowledgeDocumentStore,
    pub ingestion_job_store: PostgresIngestionJobStore,
    pub okf_concept_store: PostgresKnowledgeOkfConceptStore,
    pub source_store: PostgresKnowledgeSourceStore,
    pub object_ref_store: PostgresKnowledgeDriveObjectRefStore,
    pub markdown_index_metadata_store: PostgresMarkdownIndexMetadataStore,
    pub drive_import_metadata_store: PostgresDriveImportMetadataStore,
    pub browser_projection_store: PostgresKnowledgeBrowserProjectionStore,
}

impl RequestScopedStores {
    pub(crate) fn new(runtime: &KnowledgebaseRuntime, context: &KnowledgeAppRequestContext) -> Self {
        let scope = request_data_scope(context);
        Self {
            scope,
            space_store: runtime.space_store_for(scope),
            document_store: runtime.document_store_for(scope),
            ingestion_job_store: runtime.ingestion_job_store_for(scope),
            okf_concept_store: runtime.okf_concept_store_for(scope),
            source_store: runtime.source_store_for(scope),
            object_ref_store: runtime.object_ref_store_for(scope),
            markdown_index_metadata_store: runtime.markdown_index_metadata_store_for(scope),
            drive_import_metadata_store: runtime.drive_import_metadata_store_for(scope),
            browser_projection_store: runtime.browser_projection_store_for(scope),
        }
    }
}
