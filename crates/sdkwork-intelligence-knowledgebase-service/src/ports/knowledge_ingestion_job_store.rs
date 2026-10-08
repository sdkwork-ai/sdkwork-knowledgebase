use async_trait::async_trait;
use sdkwork_knowledgebase_contract::drive::KnowledgeDriveObjectRef;
use sdkwork_knowledgebase_contract::ingest::{IngestionJob, IngestionJobState};
use thiserror::Error;
use time::{Duration, OffsetDateTime};

use crate::ports::knowledge_chunk_store::CreateKnowledgeChunkRecord;
use crate::ports::knowledge_outbox_store::AppendOutboxEventRecord;
use crate::tenant_quota::TenantQuotaExceeded;

pub const KNOWLEDGE_UPLOAD_SESSION_TTL: Duration = Duration::hours(24);
pub const STALE_UPLOAD_SESSION_RECOVERY_BATCH_SIZE: u32 = 100;
pub const DEFAULT_INGESTION_JOB_LEASE: Duration = Duration::minutes(5);
pub const MAX_INGESTION_JOB_LEASE: Duration = Duration::hours(1);
/// Age after which a never-leased (`claim_token IS NULL`) `api` job in a
/// non-terminal state stops counting against the tenant ingest-concurrency
/// quota. `api` jobs run synchronously inside the caller's request without a
/// claim lease, so a worker crash between `mark_running` and completion leaves
/// them `Running` forever; without this horizon every such zombie would
/// permanently consume quota. The job row itself is left untouched so the
/// idempotency key keeps answering with the original job.
pub const KNOWLEDGE_UNLEASED_API_JOB_QUOTA_TTL: Duration = Duration::hours(1);

#[async_trait]
pub trait IngestionJobStore: Send + Sync {
    async fn create_or_get_job(
        &self,
        record: CreateIngestionJobRecord,
    ) -> Result<CreateOrGetIngestionJobResult, IngestionJobStoreError>;

    async fn get_job(&self, job_id: u64) -> Result<IngestionJob, IngestionJobStoreError>;

    async fn get_job_lifecycle(
        &self,
        job_id: u64,
    ) -> Result<IngestionJobLifecycle, IngestionJobStoreError> {
        Err(IngestionJobStoreError::Internal(format!(
            "ingestion job store does not expose lifecycle timestamps for job {job_id}"
        )))
    }

    async fn update_job_state(
        &self,
        job_id: u64,
        expected_state: IngestionJobState,
        state: IngestionJobState,
        error_message: Option<String>,
    ) -> Result<IngestionJob, IngestionJobStoreError>;

    async fn list_jobs_by_state(
        &self,
        state: IngestionJobState,
        limit: u32,
    ) -> Result<Vec<IngestionJob>, IngestionJobStoreError>;

    async fn claim_ingestion_jobs(
        &self,
        _request: ClaimIngestionJobsRequest,
    ) -> Result<Vec<ClaimedIngestionJob>, IngestionJobStoreError> {
        Err(IngestionJobStoreError::Internal(
            "ingestion job store does not support leased claims".to_string(),
        ))
    }

    async fn renew_ingestion_job_lease(
        &self,
        _job_id: u64,
        _claim_token: &str,
        _lease_duration: Duration,
    ) -> Result<OffsetDateTime, IngestionJobStoreError> {
        Err(IngestionJobStoreError::Internal(
            "ingestion job store does not support lease renewal".to_string(),
        ))
    }

    async fn fail_claimed_ingestion_job(
        &self,
        _job_id: u64,
        _claim_token: &str,
        _error_message: String,
    ) -> Result<IngestionJob, IngestionJobStoreError> {
        Err(IngestionJobStoreError::Internal(
            "ingestion job store does not support fenced failure".to_string(),
        ))
    }

    async fn fail_expired_upload_sessions(
        &self,
        _expired_before: OffsetDateTime,
        _limit: u32,
    ) -> Result<u32, IngestionJobStoreError> {
        Err(IngestionJobStoreError::Internal(
            "ingestion job store does not support expired upload-session recovery".to_string(),
        ))
    }

    async fn attach_drive_import_linkage(
        &self,
        job_id: u64,
        linkage: DriveImportJobLinkage,
    ) -> Result<(), IngestionJobStoreError>;

    async fn get_drive_import_linkage(
        &self,
        job_id: u64,
    ) -> Result<Option<DriveImportJobLinkage>, IngestionJobStoreError>;

    async fn mark_running_job_succeeded_with_outbox(
        &self,
        job_id: u64,
        outbox: AppendOutboxEventRecord,
    ) -> Result<IngestionJob, IngestionJobStoreError>;

    async fn complete_running_ingestion_with_chunks_and_outbox(
        &self,
        record: CompleteRunningIngestionRecord,
    ) -> Result<CompletedIngestionResult, IngestionJobStoreError>;
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CompleteRunningIngestionRecord {
    pub job_id: u64,
    pub claim_token: Option<String>,
    pub document_version_id: u64,
    pub chunks: Vec<CreateKnowledgeChunkRecord>,
    pub outbox: AppendOutboxEventRecord,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ClaimIngestionJobsRequest {
    pub claim_owner: String,
    pub lease_duration: Duration,
    pub limit: u32,
    /// Maximum claim attempts before a repeatedly crashing or lease-losing job
    /// is failed permanently instead of being retried forever.
    pub max_attempts: u32,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ClaimedIngestionJob {
    pub job: IngestionJob,
    pub claim_token: String,
    pub lease_expires_at: OffsetDateTime,
    pub attempt_count: u32,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct IngestionJobLifecycle {
    pub job: IngestionJob,
    pub created_at: OffsetDateTime,
    pub updated_at: OffsetDateTime,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DriveImportJobLinkage {
    pub source_id: u64,
    pub document_id: u64,
    pub document_version_id: u64,
    pub original_object_ref: KnowledgeDriveObjectRef,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CreateIngestionJobRecord {
    pub space_id: u64,
    pub source_type: String,
    pub idempotency_key: String,
    pub idempotency_fingerprint_sha256_hex: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CreateOrGetIngestionJobResult {
    pub job: IngestionJob,
    pub created: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CompletedIngestionResult {
    pub job: IngestionJob,
    pub chunk_count: usize,
}

#[derive(Debug, Error, Clone, PartialEq, Eq)]
pub enum IngestionJobStoreError {
    #[error("ingestion job not found: {0}")]
    NotFound(u64),
    #[error("ingestion job conflict: {0}")]
    Conflict(String),
    #[error(transparent)]
    QuotaExceeded(#[from] TenantQuotaExceeded),
    #[error("ingestion job store internal error: {0}")]
    Internal(String),
}
