use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Debug, Clone, Default)]
pub struct OkfCandidateResult {
    pub id: String,

    pub state: String,
}
