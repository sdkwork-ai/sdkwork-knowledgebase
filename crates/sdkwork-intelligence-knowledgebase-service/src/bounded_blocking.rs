use std::sync::{Arc, OnceLock};
use std::time::Duration;
use thiserror::Error;

pub(crate) const BLOCKING_OPERATION_CAPACITY: usize = 64;
pub(crate) const BLOCKING_OPERATION_TIMEOUT: Duration = Duration::from_secs(30);

#[derive(Clone)]
pub(crate) struct BoundedBlockingExecutor {
    capacity: usize,
    admission: Arc<tokio::sync::Semaphore>,
}

impl BoundedBlockingExecutor {
    pub(crate) fn new(capacity: usize) -> Result<Self, BoundedBlockingError> {
        if capacity == 0 || capacity > tokio::sync::Semaphore::MAX_PERMITS {
            return Err(BoundedBlockingError::InvalidCapacity);
        }
        Ok(Self {
            capacity,
            admission: Arc::new(tokio::sync::Semaphore::new(capacity)),
        })
    }

    pub(crate) async fn run<F, T>(
        &self,
        timeout: Duration,
        operation: F,
    ) -> Result<T, BoundedBlockingError>
    where
        F: FnOnce() -> T + Send + 'static,
        T: Send + 'static,
    {
        let permit = Arc::clone(&self.admission)
            .try_acquire_owned()
            .map_err(|_| BoundedBlockingError::QueueSaturated {
                capacity: self.capacity,
            })?;
        // The permit lives on the caller side of the JoinHandle: dropping it on timeout
        // returns capacity immediately even though the blocking task keeps running
        // (`spawn_blocking` cannot be cancelled). Normal completions hold the permit for
        // the whole task duration, so the concurrency cap stays strict except for
        // evicted (zombie) tasks, which is exactly the intended eviction semantics.
        let task = tokio::task::spawn_blocking(operation);
        match tokio::time::timeout(timeout, task).await {
            Ok(Ok(result)) => {
                drop(permit);
                Ok(result)
            }
            Ok(Err(error)) if error.is_panic() => {
                drop(permit);
                Err(BoundedBlockingError::TaskPanicked)
            }
            Ok(Err(_)) => {
                drop(permit);
                Err(BoundedBlockingError::TaskCancelled)
            }
            Err(_) => {
                drop(permit);
                Err(BoundedBlockingError::TimedOut { timeout })
            }
        }
    }
}

#[derive(Clone, Debug, Error, Eq, PartialEq)]
pub(crate) enum BoundedBlockingError {
    #[error("bounded blocking capacity must be greater than zero")]
    InvalidCapacity,
    #[error("bounded blocking queue is saturated at capacity {capacity}")]
    QueueSaturated { capacity: usize },
    #[error("bounded blocking operation timed out after {timeout:?}")]
    TimedOut { timeout: Duration },
    #[error("bounded blocking operation panicked")]
    TaskPanicked,
    #[error("bounded blocking operation was cancelled")]
    TaskCancelled,
}

static BLOCKING_EXECUTOR: OnceLock<Result<BoundedBlockingExecutor, BoundedBlockingError>> =
    OnceLock::new();

pub(crate) async fn run_bounded_blocking<F, T>(operation: F) -> Result<T, BoundedBlockingError>
where
    F: FnOnce() -> T + Send + 'static,
    T: Send + 'static,
{
    run_bounded_blocking_with_timeout(BLOCKING_OPERATION_TIMEOUT, operation).await
}

pub(crate) async fn run_bounded_blocking_with_timeout<F, T>(
    timeout: Duration,
    operation: F,
) -> Result<T, BoundedBlockingError>
where
    F: FnOnce() -> T + Send + 'static,
    T: Send + 'static,
{
    let executor = match BLOCKING_EXECUTOR
        .get_or_init(|| BoundedBlockingExecutor::new(BLOCKING_OPERATION_CAPACITY))
    {
        Ok(executor) => executor,
        Err(error) => return Err(error.clone()),
    };
    executor.run(timeout, operation).await
}

#[cfg(test)]
mod tests {
    use super::{BoundedBlockingError, BoundedBlockingExecutor};
    use std::sync::mpsc;
    use std::time::Duration;

    #[tokio::test]
    async fn rejects_work_when_all_blocking_permits_are_in_use() {
        let executor = BoundedBlockingExecutor::new(1).expect("create executor");
        let (started_tx, started_rx) = tokio::sync::oneshot::channel();
        let (release_tx, release_rx) = mpsc::sync_channel(1);
        let active_executor = executor.clone();
        let active = tokio::spawn(async move {
            active_executor
                .run(Duration::from_secs(1), move || {
                    let _ = started_tx.send(());
                    let _ = release_rx.recv();
                    1_u8
                })
                .await
        });
        started_rx.await.expect("blocking task started");

        assert_eq!(
            executor.run(Duration::from_secs(1), || 2_u8).await,
            Err(BoundedBlockingError::QueueSaturated { capacity: 1 })
        );

        release_tx.send(()).expect("release active task");
        assert_eq!(active.await.expect("join active caller"), Ok(1_u8));
    }

    #[tokio::test]
    async fn timeout_evicts_permit_so_queue_recovers_without_overcapacity() {
        let executor = BoundedBlockingExecutor::new(1).expect("create executor");
        let (started_tx, started_rx) = tokio::sync::oneshot::channel();
        let (release_tx, release_rx) = mpsc::sync_channel(1);
        let (finished_tx, finished_rx) = tokio::sync::oneshot::channel();
        let timeout = Duration::from_millis(25);
        let timed_executor = executor.clone();
        let timed = tokio::spawn(async move {
            timed_executor
                .run(timeout, move || {
                    let _ = started_tx.send(());
                    let _ = release_rx.recv();
                    let _ = finished_tx.send(());
                    1_u8
                })
                .await
        });
        started_rx.await.expect("blocking task started");

        assert_eq!(
            timed.await.expect("join timed caller"),
            Err(BoundedBlockingError::TimedOut { timeout })
        );
        // The timed-out call evicted its permit, so fresh work is admitted immediately
        // even though the stuck task is still running in the background.
        assert_eq!(
            executor.run(Duration::from_secs(1), || 2_u8).await,
            Ok(2_u8)
        );

        // Releasing the stuck task lets the executor return to a quiescent state where
        // exactly one permit is available and a new call still succeeds.
        release_tx.send(()).expect("release timed out task");
        finished_rx.await.expect("timed out task finished");
        assert_eq!(
            executor.run(Duration::from_secs(1), || 3_u8).await,
            Ok(3_u8)
        );
        assert_eq!(executor.admission.available_permits(), 1);
    }
}
