//! Process-level cache for bound external knowledge engines.
//!
//! Every `bind_provider` call previously constructed a fresh adapter instance,
//! which rebuilt the adapter's `ProviderRuntime` (circuit breaker state, bulkhead
//! semaphore, pinned HTTP client). Per-request instances made circuit breaking
//! and bulkheads structurally ineffective. This cache reuses the bound engine
//! per `(binding identity, credential reference version)` so the provider
//! runtime persists across requests; credential rotation changes the reference
//! version and naturally produces a new entry.

use std::collections::{HashMap, VecDeque};
use std::sync::{Arc, Mutex};

use sdkwork_knowledgebase_contract::knowledge_engine::KnowledgeEngineError;

use crate::ports::knowledge_engine::KnowledgeEngine;

/// Default bound-engine cache capacity. Bound engines are small (config +
/// HTTP client + counters); the bound prevents unbounded growth under
/// binding/credential churn (e.g. rotation tests or operator flapping).
pub const DEFAULT_PROVIDER_BINDING_ENGINE_CACHE_CAPACITY: usize = 64;

/// Cache key: binding identity plus the credential freshness token.
///
/// `credential_reference_version` increments on rotation, so a rotated
/// credential produces a new key without ever touching secret material.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct ProviderBindingEngineKey {
    pub implementation_id: String,
    pub binding_id: u64,
    pub credential_reference_id: Option<u64>,
    pub credential_reference_version: Option<u64>,
    /// Whether a resolved credential material was part of the bind request.
    pub credential_bound: bool,
}

struct CacheState {
    map: HashMap<ProviderBindingEngineKey, Arc<dyn KnowledgeEngine>>,
    order: VecDeque<ProviderBindingEngineKey>,
}

/// Bounded FIFO cache of bound engines. The bind closure never runs while the
/// lock is held, so a slow adapter constructor cannot block other lookups.
pub struct ProviderBindingEngineCache {
    capacity: usize,
    state: Mutex<CacheState>,
}

impl ProviderBindingEngineCache {
    pub fn new(capacity: usize) -> Self {
        Self {
            capacity: capacity.max(1),
            state: Mutex::new(CacheState {
                map: HashMap::new(),
                order: VecDeque::new(),
            }),
        }
    }

    pub fn with_default_capacity() -> Self {
        Self::new(DEFAULT_PROVIDER_BINDING_ENGINE_CACHE_CAPACITY)
    }

    /// Returns the cached engine for `key`, or binds a fresh one via `bind`,
    /// caches it, and returns it. Bind errors are never cached.
    pub fn get_or_bind(
        &self,
        key: ProviderBindingEngineKey,
        bind: impl FnOnce() -> Result<Arc<dyn KnowledgeEngine>, KnowledgeEngineError>,
    ) -> Result<Arc<dyn KnowledgeEngine>, KnowledgeEngineError> {
        if let Some(hit) = self.get(&key) {
            return Ok(hit);
        }
        let engine = bind()?;
        self.insert(key, engine.clone());
        Ok(engine)
    }

    fn get(&self, key: &ProviderBindingEngineKey) -> Option<Arc<dyn KnowledgeEngine>> {
        let state = self.state.lock().expect("provider binding cache lock");
        state.map.get(key).cloned()
    }

    fn insert(&self, key: ProviderBindingEngineKey, engine: Arc<dyn KnowledgeEngine>) {
        let mut state = self.state.lock().expect("provider binding cache lock");
        if state.map.contains_key(&key) {
            return;
        }
        while state.map.len() >= self.capacity {
            if let Some(oldest) = state.order.pop_front() {
                state.map.remove(&oldest);
            } else {
                break;
            }
        }
        state.order.push_back(key.clone());
        state.map.insert(key, engine);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use async_trait::async_trait;
    use sdkwork_knowledgebase_contract::knowledge_engine::{
        KnowledgeEngineDescriptor, KnowledgeEngineError, KnowledgeEngineHealth,
        KnowledgeEngineListRequest, KnowledgeEngineReadRequest, KnowledgeEngineSearchRequest,
        KnowledgeEngineSearchResult,
    };
    use sdkwork_knowledgebase_contract::provider_binding::KnowledgeEngineExecutionContext;
    use std::sync::atomic::{AtomicUsize, Ordering};

    /// Bare engine used purely as an opaque cache value.
    #[derive(Default)]
    struct OpaqueEngine;

    #[async_trait]
    impl KnowledgeEngine for OpaqueEngine {
        fn descriptor(&self) -> KnowledgeEngineDescriptor {
            unimplemented!("not used in cache test")
        }

        async fn health(&self) -> Result<KnowledgeEngineHealth, KnowledgeEngineError> {
            unimplemented!("not used in cache test")
        }

        async fn search(
            &self,
            _context: &KnowledgeEngineExecutionContext,
            _request: KnowledgeEngineSearchRequest,
        ) -> Result<KnowledgeEngineSearchResult, KnowledgeEngineError> {
            unimplemented!("not used in cache test")
        }

        async fn read_document(
            &self,
            _context: &KnowledgeEngineExecutionContext,
            _request: KnowledgeEngineReadRequest,
        ) -> Result<
            sdkwork_knowledgebase_contract::knowledge_engine::KnowledgeEngineDocument,
            KnowledgeEngineError,
        > {
            unimplemented!("not used in cache test")
        }

        async fn list_documents(
            &self,
            _context: &KnowledgeEngineExecutionContext,
            _request: KnowledgeEngineListRequest,
        ) -> Result<
            sdkwork_knowledgebase_contract::knowledge_engine::KnowledgeEngineDocumentList,
            KnowledgeEngineError,
        > {
            unimplemented!("not used in cache test")
        }
    }

    fn key(binding_id: u64, version: Option<u64>) -> ProviderBindingEngineKey {
        ProviderBindingEngineKey {
            implementation_id: "engine.knowledge.external.qdrant".to_string(),
            binding_id,
            credential_reference_id: version.map(|_| 9),
            credential_reference_version: version,
            credential_bound: version.is_some(),
        }
    }

    #[test]
    fn reuses_bound_engine_until_credential_version_changes() {
        let cache = ProviderBindingEngineCache::with_default_capacity();
        let binds = Arc::new(AtomicUsize::new(0));

        let bind = {
            let binds = binds.clone();
            move || {
                binds.fetch_add(1, Ordering::SeqCst);
                Ok(Arc::new(OpaqueEngine) as Arc<dyn KnowledgeEngine>)
            }
        };

        let first = cache.get_or_bind(key(7, Some(1)), bind.clone()).expect("first bind");
        let second = cache.get_or_bind(key(7, Some(1)), bind.clone()).expect("second bind");
        assert_eq!(binds.load(Ordering::SeqCst), 1, "cache hit must not re-bind");
        assert!(Arc::ptr_eq(&first, &second));

        let rotated = cache.get_or_bind(key(7, Some(2)), bind.clone()).expect("rotated bind");
        assert_eq!(binds.load(Ordering::SeqCst), 2, "credential rotation must re-bind");
        assert!(!Arc::ptr_eq(&first, &rotated));
    }

    #[test]
    fn evicts_oldest_entry_when_capacity_is_reached() {
        let cache = ProviderBindingEngineCache::new(2);
        let binds = Arc::new(AtomicUsize::new(0));

        let bind = {
            let binds = binds.clone();
            move || {
                binds.fetch_add(1, Ordering::SeqCst);
                Ok(Arc::new(OpaqueEngine) as Arc<dyn KnowledgeEngine>)
            }
        };

        cache.get_or_bind(key(1, None), bind.clone()).expect("bind 1");
        cache.get_or_bind(key(2, None), bind.clone()).expect("bind 2");
        cache.get_or_bind(key(3, None), bind.clone()).expect("bind 3");
        assert_eq!(binds.load(Ordering::SeqCst), 3);

        // Binding 1 was evicted (FIFO); asking again must re-bind.
        cache.get_or_bind(key(1, None), bind.clone()).expect("rebind 1");
        assert_eq!(binds.load(Ordering::SeqCst), 4);

        // Binding 3 is still cached.
        cache.get_or_bind(key(3, None), bind.clone()).expect("cached 3");
        assert_eq!(binds.load(Ordering::SeqCst), 4);
    }

    #[test]
    fn bind_errors_are_not_cached() {
        let cache = ProviderBindingEngineCache::with_default_capacity();
        let attempts = Arc::new(AtomicUsize::new(0));

        let failing = {
            let attempts = attempts.clone();
            move || {
                attempts.fetch_add(1, Ordering::SeqCst);
                Err(KnowledgeEngineError::Unsupported(
                    "no credential".to_string(),
                ))
            }
        };

        assert!(cache.get_or_bind(key(5, None), failing.clone()).is_err());
        assert!(cache.get_or_bind(key(5, None), failing.clone()).is_err());
        assert_eq!(attempts.load(Ordering::SeqCst), 2, "errors must retry, not cache");
    }
}
