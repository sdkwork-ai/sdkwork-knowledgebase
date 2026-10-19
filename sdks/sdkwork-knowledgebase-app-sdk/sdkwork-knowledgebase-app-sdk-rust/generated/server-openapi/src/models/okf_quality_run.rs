use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Debug, Clone, Default)]
pub struct OkfQualityRun {
    pub id: String,

    pub state: String,
}
