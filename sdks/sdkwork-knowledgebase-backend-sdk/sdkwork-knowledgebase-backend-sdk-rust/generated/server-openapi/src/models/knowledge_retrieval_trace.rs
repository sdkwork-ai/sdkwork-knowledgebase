use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Debug, Clone, Default)]
pub struct KnowledgeRetrievalTrace {
    #[serde(rename = "retrievalTraceId")]
    pub retrieval_trace_id: String,

    pub status: String,

    #[serde(rename = "latencyMs")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub latency_ms: Option<String>,

    #[serde(rename = "resultCount")]
    pub result_count: i64,
}
