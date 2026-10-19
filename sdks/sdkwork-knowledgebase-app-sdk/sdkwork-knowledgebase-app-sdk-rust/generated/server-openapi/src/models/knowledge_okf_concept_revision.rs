use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Debug, Clone, Default)]
pub struct KnowledgeOkfConceptRevision {
    pub id: String,

    #[serde(rename = "conceptRowId")]
    pub concept_row_id: String,

    #[serde(rename = "revisionNo")]
    pub revision_no: String,

    #[serde(rename = "markdownObjectRefId")]
    pub markdown_object_ref_id: String,

    #[serde(rename = "contentHash")]
    pub content_hash: String,

    #[serde(rename = "reviewState")]
    pub review_state: String,

    #[serde(rename = "createdAt")]
    pub created_at: String,
}
