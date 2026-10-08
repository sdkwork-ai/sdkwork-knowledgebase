use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Debug, Clone, Default)]
pub struct KnowledgeTenantQuotaStatus {
    #[serde(rename = "maxDocuments")]
    pub max_documents: String,

    #[serde(rename = "documentCount")]
    pub document_count: String,

    #[serde(rename = "maxConcurrentIngestJobs")]
    pub max_concurrent_ingest_jobs: i64,

    #[serde(rename = "inflightIngestJobs")]
    pub inflight_ingest_jobs: i64,

    #[serde(rename = "maxRetrievalsPerMinute")]
    pub max_retrievals_per_minute: i64,

    #[serde(rename = "maxStorageBytes")]
    pub max_storage_bytes: String,

    #[serde(rename = "storageBytesUsed")]
    pub storage_bytes_used: String,
}
