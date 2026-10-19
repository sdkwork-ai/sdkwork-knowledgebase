use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Debug, Clone, Default)]
pub struct OkfProfileDocument {
    #[serde(rename = "agentsMarkdown")]
    pub agents_markdown: String,

    #[serde(rename = "schemaYaml")]
    pub schema_yaml: String,
}
