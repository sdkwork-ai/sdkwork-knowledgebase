use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Debug, Clone, Default)]
pub struct ConsumeGroupKnowledgebaseLaunchTicketRequest {
    /// Opaque one-time IM launch ticket. It must never be persisted, logged, or copied into a URL query parameter.
    pub ticket: String,
}
