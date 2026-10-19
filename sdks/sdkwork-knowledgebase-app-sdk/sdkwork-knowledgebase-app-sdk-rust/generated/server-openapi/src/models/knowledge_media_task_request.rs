use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Debug, Clone, Default)]
pub struct KnowledgeMediaTaskRequest {
    #[serde(rename = "spaceId")]
    pub space_id: String,

    #[serde(rename = "taskType")]
    pub task_type: String,

    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub prompt: Option<String>,

    #[serde(rename = "aspectMode")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub aspect_mode: Option<String>,

    #[serde(rename = "styleMode")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub style_mode: Option<String>,

    #[serde(rename = "sourceUrl")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub source_url: Option<String>,

    #[serde(rename = "documentId")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub document_id: Option<String>,
}
