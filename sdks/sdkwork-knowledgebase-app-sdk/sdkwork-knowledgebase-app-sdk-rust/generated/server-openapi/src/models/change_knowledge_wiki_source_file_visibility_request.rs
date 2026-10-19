use serde::{Deserialize, Serialize};

/// Change a published Wiki source file visibility with version fencing.
#[derive(Serialize, Deserialize, Debug, Clone, Default)]
pub struct ChangeKnowledgeWikiSourceFileVisibilityRequest {
    pub visibility: String,

    #[serde(rename = "expectedPublicationVersion")]
    pub expected_publication_version: String,

    #[serde(rename = "expectedPageVersion")]
    pub expected_page_version: String,
}
