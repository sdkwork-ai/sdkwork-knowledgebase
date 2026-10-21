//! PostgreSQL advisory lock for WeChat tenant configuration writes.
//!
//! The Drive configuration object has no compare-and-swap precondition, so a process-local
//! mutex is not enough when several replicas serve one tenant. The lock is a transaction-scoped
//! advisory lock: dropping the transaction (including on an error or panic) rolls back and
//! releases it, and `lock_timeout` bounds the wait instead of hanging a request.

use async_trait::async_trait;
use sdkwork_intelligence_knowledgebase_service::ports::knowledge_drive_storage::KnowledgeStorageError;
use sdkwork_intelligence_knowledgebase_service::ports::knowledge_wechat_config_lock::{
    KnowledgeWechatConfigLock, KnowledgeWechatConfigLockGuard,
};
use sqlx::{PgPool, Postgres, Transaction};
use tokio::sync::Mutex;

/// Advisory-lock namespace ("KBWC"); keeps WeChat config locks from colliding with the quota
/// and other single-key advisory locks this service uses.
const WECHAT_CONFIG_LOCK_NAMESPACE: i32 = 0x4B42_5743;
const WECHAT_CONFIG_LOCK_TIMEOUT: &str = "3s";

pub struct PostgresWechatConfigLock {
    pool: PgPool,
}

impl PostgresWechatConfigLock {
    #[must_use]
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }
}

#[async_trait]
impl KnowledgeWechatConfigLock for PostgresWechatConfigLock {
    async fn acquire(
        &self,
        key: &str,
    ) -> Result<Box<dyn KnowledgeWechatConfigLockGuard>, KnowledgeStorageError> {
        let mut transaction = self.pool.begin().await.map_err(lock_storage_error)?;
        sqlx::query(sqlx::AssertSqlSafe(format!(
            "SET LOCAL lock_timeout = '{WECHAT_CONFIG_LOCK_TIMEOUT}'"
        )))
        .execute(&mut *transaction)
        .await
        .map_err(lock_storage_error)?;
        sqlx::query("SELECT pg_advisory_xact_lock($1, hashtext($2))")
            .bind(WECHAT_CONFIG_LOCK_NAMESPACE)
            .bind(key)
            .execute(&mut *transaction)
            .await
            .map_err(|error| {
                KnowledgeStorageError::Conflict(format!(
                    "wechat configuration is being updated by another request: {error}"
                ))
            })?;
        Ok(Box::new(PostgresWechatConfigLockGuard {
            transaction: Mutex::new(Some(transaction)),
        }))
    }
}

struct PostgresWechatConfigLockGuard {
    transaction: Mutex<Option<Transaction<'static, Postgres>>>,
}

#[async_trait]
impl KnowledgeWechatConfigLockGuard for PostgresWechatConfigLockGuard {
    async fn release(&self) -> Result<(), KnowledgeStorageError> {
        let Some(transaction) = self.transaction.lock().await.take() else {
            return Ok(());
        };
        transaction.commit().await.map_err(lock_storage_error)
    }
}

fn lock_storage_error(error: sqlx::Error) -> KnowledgeStorageError {
    KnowledgeStorageError::Internal(format!("wechat config write lock: {error}"))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::{Duration, Instant};

    #[tokio::test]
    #[ignore = "requires SDKWORK_DATABASE_URL pointing at PostgreSQL"]
    async fn advisory_lock_serializes_across_connections_and_releases() {
        let url = std::env::var("SDKWORK_DATABASE_URL").expect("SDKWORK_DATABASE_URL");
        let pool = PgPool::connect(&url).await.expect("connect postgres");
        let lock = PostgresWechatConfigLock::new(pool.clone());
        let key = "wechat:official_accounts:advisory-lock-probe";

        let first = lock.acquire(key).await.expect("first acquire");
        let started = Instant::now();
        let second = lock.acquire(key).await;
        assert!(
            matches!(second, Err(KnowledgeStorageError::Conflict(_))),
            "a second writer must time out instead of entering the critical section"
        );
        assert!(
            started.elapsed() >= Duration::from_secs(2),
            "the second writer must wait for the configured lock_timeout"
        );

        first.release().await.expect("release first");
        let third = lock.acquire(key).await.expect("acquire after release");
        third.release().await.expect("release third");
    }
}
