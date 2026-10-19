use serde::{Deserialize, Serialize};

/// Publish the exact current Drive version as PUBLIC or UNLISTED.
#[derive(Serialize, Deserialize, Debug, Clone, Default)]
pub struct PublishKnowledgeWikiSourceFileRequest {
    pub visibility: String,

    #[serde(rename = "expectedPublicationVersion")]
    pub expected_publication_version: String,

    #[serde(rename = "expectedPageVersion")]
    pub expected_page_version: String,
}
