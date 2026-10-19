use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Debug, Clone, Default)]
pub struct OkfFileAnswerRequest {
    #[serde(rename = "spaceId")]
    pub space_id: String,

    pub title: String,

    #[serde(rename = "answerMarkdown")]
    pub answer_markdown: String,
}
