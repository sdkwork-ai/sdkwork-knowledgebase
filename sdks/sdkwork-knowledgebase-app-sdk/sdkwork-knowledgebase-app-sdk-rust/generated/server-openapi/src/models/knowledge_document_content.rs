use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Debug, Clone, Default)]
pub struct KnowledgeDocumentContent {
    #[serde(rename = "documentId")]
    pub document_id: String,

    #[serde(rename = "contentMarkdown")]
    pub content_markdown: String,

    #[serde(rename = "contentSource")]
    pub content_source: String,

    #[serde(rename = "contentVersion")]
    pub content_version: String,
}
