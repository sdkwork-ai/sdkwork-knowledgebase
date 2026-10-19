use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Debug, Clone, Default)]
pub struct OkfConceptUpsertRequest {
    #[serde(rename = "spaceId")]
    pub space_id: String,

    #[serde(rename = "conceptId")]
    pub concept_id: String,

    pub markdown: String,

    pub actor: String,

    pub publish: bool,
}
