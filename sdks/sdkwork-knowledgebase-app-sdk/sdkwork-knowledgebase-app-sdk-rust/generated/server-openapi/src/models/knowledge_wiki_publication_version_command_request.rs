use serde::{Deserialize, Serialize};

/// Optimistic Wiki publication status command.
#[derive(Serialize, Deserialize, Debug, Clone, Default)]
pub struct KnowledgeWikiPublicationVersionCommandRequest {
    #[serde(rename = "expectedVersion")]
    pub expected_version: String,
}
