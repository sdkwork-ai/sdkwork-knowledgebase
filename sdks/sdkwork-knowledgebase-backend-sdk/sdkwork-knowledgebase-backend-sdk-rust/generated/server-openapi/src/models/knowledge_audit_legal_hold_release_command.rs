use serde::{Deserialize, Serialize};

/// Legal-hold release command. The hold is identified by the path; no fields are required.
#[derive(Serialize, Deserialize, Debug, Clone, Default)]
pub struct KnowledgeAuditLegalHoldReleaseCommand {
    #[serde(flatten)]
    pub additional_properties: std::collections::HashMap<String, serde_json::Value>,
}
