use crate::ports::knowledge_outbox_dispatcher::KnowledgeOutboxDispatcher;
use crate::ports::knowledge_outbox_store::{KnowledgeOutboxStore, KnowledgeOutboxStoreError};
use futures::{stream, StreamExt};
use thiserror::Error;

/// Bounded in-batch dispatch concurrency: a slow webhook endpoint delays only the events
/// sharing its batch window, never the whole outbox domain for the batch duration.
const OUTBOX_PUBLISH_CONCURRENCY: usize = 4;

pub struct KnowledgeOutboxPublisherService<'a> {
    outbox: &'a dyn KnowledgeOutboxStore,
    dispatcher: &'a dyn KnowledgeOutboxDispatcher,
    tenant_id: u64,
}

impl<'a> KnowledgeOutboxPublisherService<'a> {
    pub fn new(
        tenant_id: u64,
        outbox: &'a dyn KnowledgeOutboxStore,
        dispatcher: &'a dyn KnowledgeOutboxDispatcher,
    ) -> Self {
        Self {
            outbox,
            dispatcher,
            tenant_id,
        }
    }

    pub async fn publish_pending(
        &self,
        limit: u32,
    ) -> Result<OutboxPublishBatchResult, KnowledgeOutboxPublisherServiceError> {
        let pending = self
            .outbox
            .claim_pending_events(limit)
            .await
            .map_err(KnowledgeOutboxPublisherServiceError::Store)?;

        let mut published = 0usize;
        let mut failed = 0usize;
        let outcomes = stream::iter(pending)
            .map(|claimed| self.dispatch_one(claimed))
            .buffer_unordered(OUTBOX_PUBLISH_CONCURRENCY);
        tokio::pin!(outcomes);
        while let Some(outcome) = outcomes.next().await {
            match outcome {
                DispatchOutcome::Published => published += 1,
                DispatchOutcome::Failed => failed += 1,
                DispatchOutcome::Unresolved => {}
            }
        }

        Ok(OutboxPublishBatchResult {
            requeued: 0,
            published,
            failed,
            dead_lettered: 0,
        })
    }

    async fn dispatch_one(
        &self,
        claimed: crate::ports::knowledge_outbox_store::ClaimedOutboxEvent,
    ) -> DispatchOutcome {
        let event = &claimed.event;
        tracing::info!(
            event_id = event.id,
            event_type = %event.event_type,
            aggregate_type = %event.aggregate_type,
            aggregate_id = event.aggregate_id,
            "dispatching knowledgebase outbox event"
        );
        match self.dispatcher.dispatch(self.tenant_id, event).await {
            Ok(()) => {
                // A stale/fenced completion must not abort the rest of the
                // batch: the event is left CLAIMED and will be released by
                // the stale-claim sweep (at-least-once redelivery), while
                // the remaining claimed events keep being processed.
                if let Err(error) = self.outbox.mark_published(&claimed).await {
                    tracing::error!(
                        event_id = event.id,
                        error = %error,
                        "knowledgebase outbox mark_published failed; event remains claimed and will be released by the stale-claim sweep"
                    );
                    DispatchOutcome::Unresolved
                } else {
                    DispatchOutcome::Published
                }
            }
            Err(error) => {
                tracing::warn!(
                    event_id = event.id,
                    error = %error,
                    "knowledgebase outbox dispatch failed"
                );
                if let Err(mark_error) = self.outbox.mark_failed(&claimed, &error.to_string()).await
                {
                    tracing::error!(
                        event_id = event.id,
                        error = %mark_error,
                        "knowledgebase outbox mark_failed failed; event remains claimed and will be released by the stale-claim sweep"
                    );
                    DispatchOutcome::Unresolved
                } else {
                    DispatchOutcome::Failed
                }
            }
        }
    }
}

enum DispatchOutcome {
    Published,
    Failed,
    Unresolved,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OutboxPublishBatchResult {
    pub requeued: usize,
    pub published: usize,
    pub failed: usize,
    /// Events moved to the dead-letter status by the requeue sweep. The
    /// publisher itself never dead-letters; the runtime fills this from the
    /// requeue store result.
    pub dead_lettered: usize,
}

#[derive(Debug, Error)]
pub enum KnowledgeOutboxPublisherServiceError {
    #[error(transparent)]
    Store(#[from] KnowledgeOutboxStoreError),
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use async_trait::async_trait;

    use super::*;
    use crate::ports::knowledge_outbox_dispatcher::KnowledgeOutboxDispatchError;
    use crate::ports::knowledge_outbox_store::{
        AppendOutboxEventRecord, ClaimedOutboxEvent, OutboxClaim, OutboxRequeueResult,
        OutboxStaleReleaseResult, PendingOutboxEvent,
    };

    struct InMemoryOutboxStore {
        pending: tokio::sync::Mutex<Vec<PendingOutboxEvent>>,
        published: tokio::sync::Mutex<Vec<u64>>,
        failed: tokio::sync::Mutex<Vec<(u64, String)>>,
        fail_claim: bool,
    }

    impl InMemoryOutboxStore {
        fn new() -> Self {
            Self {
                pending: tokio::sync::Mutex::new(Vec::new()),
                published: tokio::sync::Mutex::new(Vec::new()),
                failed: tokio::sync::Mutex::new(Vec::new()),
                fail_claim: false,
            }
        }

        fn failing_claim() -> Self {
            Self {
                fail_claim: true,
                ..Self::new()
            }
        }
    }

    #[async_trait]
    impl KnowledgeOutboxStore for InMemoryOutboxStore {
        async fn append_event(
            &self,
            record: AppendOutboxEventRecord,
        ) -> Result<(), KnowledgeOutboxStoreError> {
            let mut pending = self.pending.lock().await;
            let id = pending.len() as u64 + 1;
            pending.push(PendingOutboxEvent {
                id,
                event_uuid: format!("event-{id}"),
                event_type: record.event_type,
                aggregate_type: record.aggregate_type,
                aggregate_id: record.aggregate_id,
                retry_count: 0,
                payload_json: record.payload_json,
            });
            Ok(())
        }

        async fn list_pending_events(
            &self,
            limit: u32,
        ) -> Result<Vec<PendingOutboxEvent>, KnowledgeOutboxStoreError> {
            let pending = self.pending.lock().await;
            Ok(pending.iter().take(limit as usize).cloned().collect())
        }

        async fn claim_pending_events(
            &self,
            limit: u32,
        ) -> Result<Vec<ClaimedOutboxEvent>, KnowledgeOutboxStoreError> {
            if self.fail_claim {
                return Err(KnowledgeOutboxStoreError::Internal(
                    "simulated claim failure".to_string(),
                ));
            }
            let mut pending = self.pending.lock().await;
            let claimed: Vec<ClaimedOutboxEvent> = pending
                .iter()
                .take(limit as usize)
                .cloned()
                .map(|event| ClaimedOutboxEvent {
                    event,
                    claim: OutboxClaim {
                        owner: "test-worker".to_string(),
                        token: "test-claim".to_string(),
                    },
                })
                .collect();
            pending.drain(0..claimed.len());
            Ok(claimed)
        }

        async fn release_stale_claimed_events(
            &self,
            _stale_after_secs: u64,
            _max_retry_count: u32,
        ) -> Result<OutboxStaleReleaseResult, KnowledgeOutboxStoreError> {
            Ok(OutboxStaleReleaseResult::default())
        }

        async fn mark_published(
            &self,
            claimed: &ClaimedOutboxEvent,
        ) -> Result<(), KnowledgeOutboxStoreError> {
            let event_id = claimed.event.id;
            let mut pending = self.pending.lock().await;
            pending.retain(|event| event.id != event_id);
            self.published.lock().await.push(event_id);
            Ok(())
        }

        async fn mark_failed(
            &self,
            claimed: &ClaimedOutboxEvent,
            error_message: &str,
        ) -> Result<(), KnowledgeOutboxStoreError> {
            let event_id = claimed.event.id;
            let mut pending = self.pending.lock().await;
            pending.retain(|event| event.id != event_id);
            self.failed
                .lock()
                .await
                .push((event_id, error_message.to_string()));
            Ok(())
        }

        async fn requeue_failed_events(
            &self,
            _limit: u32,
            _max_retry_count: u32,
        ) -> Result<OutboxRequeueResult, KnowledgeOutboxStoreError> {
            Ok(OutboxRequeueResult::default())
        }
    }

    struct AlwaysFailDispatcher;

    struct AlwaysSucceedDispatcher;

    #[async_trait]
    impl KnowledgeOutboxDispatcher for AlwaysFailDispatcher {
        async fn dispatch(
            &self,
            _tenant_id: u64,
            _event: &PendingOutboxEvent,
        ) -> Result<(), KnowledgeOutboxDispatchError> {
            Err(KnowledgeOutboxDispatchError::DeliveryFailed(
                "simulated failure".to_string(),
            ))
        }
    }

    #[async_trait]
    impl KnowledgeOutboxDispatcher for AlwaysSucceedDispatcher {
        async fn dispatch(
            &self,
            _tenant_id: u64,
            _event: &PendingOutboxEvent,
        ) -> Result<(), KnowledgeOutboxDispatchError> {
            Ok(())
        }
    }

    #[tokio::test]
    async fn publish_pending_marks_failed_without_publishing_on_dispatch_error() {
        let store = Arc::new(InMemoryOutboxStore::new());
        store
            .append_event(AppendOutboxEventRecord {
                aggregate_type: "ingestion_job".to_string(),
                aggregate_id: 1,
                event_type: "knowledge.ingest.succeeded".to_string(),
                payload_json: r#"{"spaceId":1}"#.to_string(),
            })
            .await
            .expect("append");

        let result = KnowledgeOutboxPublisherService::new(1, store.as_ref(), &AlwaysFailDispatcher)
            .publish_pending(10)
            .await
            .expect("publish batch");

        assert_eq!(result.published, 0);
        assert_eq!(result.failed, 1);
        assert_eq!(result.requeued, 0);
        assert!(store.published.lock().await.is_empty());
        assert_eq!(store.failed.lock().await.len(), 1);
    }

    #[tokio::test]
    async fn publish_pending_reports_successful_dispatches() {
        let store = Arc::new(InMemoryOutboxStore::new());
        for aggregate_id in 1..=2 {
            store
                .append_event(AppendOutboxEventRecord {
                    aggregate_type: "ingestion_job".to_string(),
                    aggregate_id,
                    event_type: "knowledge.ingest.succeeded".to_string(),
                    payload_json: format!(r#"{{"spaceId":{aggregate_id}}}"#),
                })
                .await
                .expect("append");
        }

        let result =
            KnowledgeOutboxPublisherService::new(1, store.as_ref(), &AlwaysSucceedDispatcher)
                .publish_pending(10)
                .await
                .expect("publish batch");

        assert_eq!(result.published, 2);
        assert_eq!(result.failed, 0);
        assert_eq!(result.requeued, 0);
        assert_eq!(store.published.lock().await.as_slice(), &[1, 2]);
    }

    #[tokio::test]
    async fn publish_pending_propagates_claim_failure() {
        let store = InMemoryOutboxStore::failing_claim();

        let error = KnowledgeOutboxPublisherService::new(1, &store, &AlwaysSucceedDispatcher)
            .publish_pending(10)
            .await
            .expect_err("claim failure must fail the batch");

        assert!(matches!(
            error,
            KnowledgeOutboxPublisherServiceError::Store(KnowledgeOutboxStoreError::Internal(_))
        ));
    }
}
