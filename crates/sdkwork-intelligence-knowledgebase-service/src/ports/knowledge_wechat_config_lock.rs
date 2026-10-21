use crate::ports::knowledge_drive_storage::KnowledgeStorageError;
use async_trait::async_trait;

/// Cross-process mutual exclusion for WeChat tenant configuration writes.
///
/// The Drive object is the configuration authority, and its `put_object` has no
/// compare-and-swap precondition. A process-local mutex is therefore insufficient when two
/// replicas serve the same tenant: both read the same document and the last writer silently
/// drops the other side's entries. Hosts inject a database- or provider-backed lock; the
/// store holds it across the load-merge-save critical section.
#[async_trait]
pub trait KnowledgeWechatConfigLock: Send + Sync {
    /// Acquires the named lock, waiting for a bounded period. `key` identifies the tenant and
    /// the configuration kind (for example `wechat:official_accounts`).
    async fn acquire(
        &self,
        key: &str,
    ) -> Result<Box<dyn KnowledgeWechatConfigLockGuard>, KnowledgeStorageError>;
}

/// Held lock guard. Releasing is explicit (not `Drop`) because the release performs I/O and
/// its failure must be observable to the caller.
#[async_trait]
pub trait KnowledgeWechatConfigLockGuard: Send + Sync {
    async fn release(&self) -> Result<(), KnowledgeStorageError>;
}
