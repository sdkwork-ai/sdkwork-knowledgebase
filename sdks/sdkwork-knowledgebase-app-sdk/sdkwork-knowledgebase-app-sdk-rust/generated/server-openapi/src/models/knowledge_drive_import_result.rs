use serde::{Deserialize, Serialize};

use crate::models::{IngestionJob, KnowledgeDocument, KnowledgeDocumentVersion, KnowledgeDriveObjectRef, KnowledgeSource};

#[derive(Serialize, Deserialize, Debug, Clone, Default)]
pub struct KnowledgeDriveImportResult {
    pub source: KnowledgeSource,

    pub document: KnowledgeDocument,

    pub version: KnowledgeDocumentVersion,

    #[serde(rename = "originalObjectRef")]
    pub original_object_ref: KnowledgeDriveObjectRef,

    pub job: IngestionJob,
}
