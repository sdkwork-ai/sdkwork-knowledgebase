use serde::{Deserialize, Serialize};

/// Optimistic Wiki source-file publication command.
#[derive(Serialize, Deserialize, Debug, Clone, Default)]
pub struct KnowledgeWikiSourceFileVersionCommandRequest {
    #[serde(rename = "expectedPublicationVersion")]
    pub expected_publication_version: String,

    #[serde(rename = "expectedPageVersion")]
    pub expected_page_version: String,
}
