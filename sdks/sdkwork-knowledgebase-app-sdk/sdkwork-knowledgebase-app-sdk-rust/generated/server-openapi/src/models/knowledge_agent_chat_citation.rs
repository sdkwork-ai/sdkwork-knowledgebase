use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Debug, Clone, Default)]
pub struct KnowledgeAgentChatCitation {
    #[serde(rename = "documentId")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub document_id: Option<String>,

    #[serde(rename = "conceptId")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub concept_id: Option<String>,

    pub title: String,

    #[serde(rename = "sourceUri")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub source_uri: Option<String>,

    #[serde(rename = "logicalPath")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub logical_path: Option<String>,

    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub locator: Option<String>,

    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub score: Option<f64>,

    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub snippet: Option<String>,
}
