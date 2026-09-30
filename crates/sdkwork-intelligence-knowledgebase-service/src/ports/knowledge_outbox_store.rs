use async_trait::async_trait;
use thiserror::Error;

pub const MAX_KNOWLEDGE_OUTBOX_PAYLOAD_BYTES: usize = 64 * 1024;

#[async_trait]
pub trait KnowledgeOutboxStore: Send + Sync {
    async fn append_event(
        &self,
        record: AppendOutboxEventRecord,
    ) -> Result<(), KnowledgeOutboxStoreError>;

    async fn list_pending_events(
        &self,
        limit: u32,
    ) -> Result<Vec<PendingOutboxEvent>, KnowledgeOutboxStoreError>;

    async fn claim_pending_events(
        &self,
        limit: u32,
    ) -> Result<Vec<ClaimedOutboxEvent>, KnowledgeOutboxStoreError>;

    /// Releases claims that have been held longer than `stale_after_secs`
    /// (typically after a worker crash).
    ///
    /// A stale release counts as a failed delivery attempt: `retry_count` is
    /// incremented and the event is rescheduled with exponential backoff.
    /// Events whose retry budget (`max_retry_count`) is exhausted by the
    /// increment move directly to the dead-letter status, so a worker that
    /// crashes before `mark_failed` cannot loop an event forever.
    async fn release_stale_claimed_events(
        &self,
        stale_after_secs: u64,
        max_retry_count: u32,
    ) -> Result<OutboxStaleReleaseResult, KnowledgeOutboxStoreError>;

    async fn mark_published(
        &self,
        claimed: &ClaimedOutboxEvent,
    ) -> Result<(), KnowledgeOutboxStoreError>;

    async fn mark_failed(
        &self,
        claimed: &ClaimedOutboxEvent,
        error_message: &str,
    ) -> Result<(), KnowledgeOutboxStoreError>;

    async fn requeue_failed_events(
        &self,
        limit: u32,
        max_retry_count: u32,
    ) -> Result<OutboxRequeueResult, KnowledgeOutboxStoreError>;
}

/// Outcome of a stale-claim release sweep.
///
/// `requeued` events became pending again (with backoff applied); events whose
/// retry budget was exhausted by the release were `dead_lettered`.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct OutboxStaleReleaseResult {
    pub requeued: usize,
    pub dead_lettered: usize,
}

/// Outcome of a failed-event requeue sweep.
///
/// `requeued` events became pending again for a later claim (honoring their
/// exponential backoff); `dead_lettered` events exhausted their retry budget and
/// were permanently moved to the dead-letter status.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct OutboxRequeueResult {
    pub requeued: usize,
    pub dead_lettered: usize,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AppendOutboxEventRecord {
    pub aggregate_type: String,
    pub aggregate_id: u64,
    pub event_type: String,
    pub payload_json: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PendingOutboxEvent {
    pub id: u64,
    pub event_uuid: String,
    pub event_type: String,
    pub aggregate_type: String,
    pub aggregate_id: u64,
    pub retry_count: u32,
    pub payload_json: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OutboxClaim {
    pub owner: String,
    pub token: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ClaimedOutboxEvent {
    pub event: PendingOutboxEvent,
    pub claim: OutboxClaim,
}

#[derive(Debug, Error, Clone, PartialEq, Eq)]
pub enum KnowledgeOutboxStoreError {
    #[error("invalid outbox event: {0}")]
    InvalidRequest(String),
    #[error("outbox store internal error: {0}")]
    Internal(String),
}
