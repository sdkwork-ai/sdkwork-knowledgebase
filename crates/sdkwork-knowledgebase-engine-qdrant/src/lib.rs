//! Qdrant external knowledge engine adapter (`integrationTier: adapter`).
//!
//! Vendor catalog: `external/knowledge-engines/vendors/qdrant/engine.manifest.json`
//! Handlers MUST NOT call Qdrant HTTP directly; only this adapter crate may integrate upstream APIs.

mod client;
mod config;

use async_trait::async_trait;
use sdkwork_intelligence_knowledgebase_service::knowledge_engine::KnowledgeEngine;
use sdkwork_intelligence_knowledgebase_service::ports::knowledge_engine::ExternalKnowledgeEngine;
use sdkwork_intelligence_knowledgebase_service::ports::knowledge_provider_credential_resolver::KnowledgeEngineProviderCredential;
use sdkwork_knowledgebase_contract::knowledge_engine::{
    descriptor_for_external, descriptor_for_external_search_read, parse_compound_document_ref,
    KnowledgeEngineDescriptor, KnowledgeEngineDocument, KnowledgeEngineDocumentList,
    KnowledgeEngineError, KnowledgeEngineHealth, KnowledgeEngineHealthStatus,
    KnowledgeEngineListRequest, KnowledgeEngineReadRequest, KnowledgeEngineSearchRequest,
    KnowledgeEngineSearchResult,
};
use sdkwork_knowledgebase_contract::provider_binding::KnowledgeEngineExecutionContext;
use sdkwork_knowledgebase_provider_runtime::{ProviderExecutionContext, ProviderOperation};
use std::sync::Arc;

pub use client::QdrantApiClient;
pub use config::{
    QdrantConnectorConfig, QDRANT_ALLOW_PRIVATE_NETWORK_ENV, QDRANT_BASE_URL_ENV,
    QDRANT_COLLECTION_NAME_ENV, QDRANT_QUERY_MODEL_ENV, QDRANT_USING_VECTOR_ENV,
};

pub const QDRANT_VENDOR_ID: &str = "qdrant";
pub const QDRANT_IMPLEMENTATION_ID: &str = "engine.knowledge.external.qdrant";
pub const QDRANT_AGENT_PROVIDER_ID: &str = "provider.knowledge.external.qdrant";

pub struct QdrantKnowledgeEngine {
    config: Option<QdrantConnectorConfig>,
    client: Option<QdrantApiClient>,
}

impl QdrantKnowledgeEngine {
    pub fn from_env() -> Option<Self> {
        QdrantConnectorConfig::from_env().map(Self::with_config)
    }

    pub fn with_config(config: QdrantConnectorConfig) -> Self {
        match QdrantApiClient::new(config.clone()) {
            Ok(client) => Self {
                config: Some(config),
                client: Some(client),
            },
            Err(error) => {
                sdkwork_knowledgebase_provider_runtime::log_engine_degraded(
                    QDRANT_IMPLEMENTATION_ID,
                    &error,
                );
                Self {
                    config: None,
                    client: None,
                }
            }
        }
    }

    fn descriptor_value(&self) -> KnowledgeEngineDescriptor {
        let display_name = if self.config.is_some() {
            "Qdrant (external adapter)"
        } else {
            "Qdrant (external adapter — unconfigured)"
        };
        if self.config.is_some() {
            descriptor_for_external_search_read(QDRANT_VENDOR_ID, display_name)
        } else {
            descriptor_for_external(QDRANT_VENDOR_ID, display_name)
        }
    }

    fn unconfigured_message(&self) -> String {
        format!(
            "Qdrant adapter requires {QDRANT_BASE_URL_ENV}; an active Provider binding supplies the collection name and may supply an optional credential reference; text search uses {QDRANT_QUERY_MODEL_ENV}"
        )
    }

    fn required_collection_name(&self, space_id: u64) -> Result<String, KnowledgeEngineError> {
        self.config
            .as_ref()
            .and_then(|config| config.default_collection_name.clone())
            .ok_or_else(|| {
                KnowledgeEngineError::Validation(format!(
                    "Qdrant execution requires an active Provider binding with a remote resource id for space_id={space_id}"
                ))
            })
    }
}

#[async_trait]
impl KnowledgeEngine for QdrantKnowledgeEngine {
    fn descriptor(&self) -> KnowledgeEngineDescriptor {
        self.descriptor_value()
    }

    fn bind_provider(
        &self,
        binding: &sdkwork_knowledgebase_contract::provider_binding::KnowledgeEngineProviderBinding,
        credential: Option<KnowledgeEngineProviderCredential>,
    ) -> Result<Arc<dyn KnowledgeEngine>, KnowledgeEngineError> {
        if binding.implementation_id != QDRANT_IMPLEMENTATION_ID {
            return Err(KnowledgeEngineError::Validation(
                "Qdrant cannot bind a different Provider implementation".to_string(),
            ));
        }
        let mut config = self
            .config
            .clone()
            .ok_or_else(|| KnowledgeEngineError::Unsupported(self.unconfigured_message()))?;
        config.api_key = credential.map(KnowledgeEngineProviderCredential::into_secret);
        config.default_collection_name = Some(binding.remote_resource_id.clone());
        let engine = Self::with_config(config);
        if engine.client.is_none() {
            return Err(KnowledgeEngineError::Validation(
                "Qdrant base URL does not satisfy the Provider runtime target policy".to_string(),
            ));
        }
        Ok(Arc::new(engine))
    }

    async fn health(&self) -> Result<KnowledgeEngineHealth, KnowledgeEngineError> {
        let Some(client) = self.client.as_ref() else {
            return Ok(KnowledgeEngineHealth {
                implementation_id: QDRANT_IMPLEMENTATION_ID.to_string(),
                status: KnowledgeEngineHealthStatus::Degraded,
                detail: Some(self.unconfigured_message()),
            });
        };

        let collection_name = match self
            .config
            .as_ref()
            .and_then(|config| config.default_collection_name.clone())
        {
            Some(collection_name) => collection_name,
            None => {
                return Ok(KnowledgeEngineHealth {
                    implementation_id: QDRANT_IMPLEMENTATION_ID.to_string(),
                    status: KnowledgeEngineHealthStatus::Degraded,
                    detail: Some(
                        "Qdrant connector health requires an active Provider binding with a remote resource id"
                            .to_string(),
                    ),
                });
            }
        };

        match client.connector_health(&collection_name).await {
            Ok(()) => Ok(KnowledgeEngineHealth {
                implementation_id: QDRANT_IMPLEMENTATION_ID.to_string(),
                status: KnowledgeEngineHealthStatus::Available,
                detail: None,
            }),
            Err(error) => Ok(KnowledgeEngineHealth {
                implementation_id: QDRANT_IMPLEMENTATION_ID.to_string(),
                status: KnowledgeEngineHealthStatus::Degraded,
                detail: Some(error.to_string()),
            }),
        }
    }

    async fn search(
        &self,
        context: &KnowledgeEngineExecutionContext,
        request: KnowledgeEngineSearchRequest,
    ) -> Result<KnowledgeEngineSearchResult, KnowledgeEngineError> {
        let Some(client) = self.client.as_ref() else {
            return Err(KnowledgeEngineError::Unsupported(
                self.unconfigured_message(),
            ));
        };

        let collection_name = self.required_collection_name(request.space_id)?;
        let provider_context = ProviderExecutionContext::from_knowledge_engine_request(
            context,
            QDRANT_IMPLEMENTATION_ID,
            ProviderOperation::Search,
            request.tenant_id,
            request.space_id,
        )
        .map_err(KnowledgeEngineError::from)?;
        client
            .query_points(
                &provider_context,
                request.space_id,
                &collection_name,
                &request.query,
                request.top_k,
            )
            .await
    }

    async fn read_document(
        &self,
        context: &KnowledgeEngineExecutionContext,
        request: KnowledgeEngineReadRequest,
    ) -> Result<KnowledgeEngineDocument, KnowledgeEngineError> {
        let Some(client) = self.client.as_ref() else {
            return Err(KnowledgeEngineError::Unsupported(
                self.unconfigured_message(),
            ));
        };

        let (_, point_id) = parse_compound_document_ref(&request.document_id).ok_or_else(|| {
            KnowledgeEngineError::Validation(
                "Qdrant read_document requires title#pointId ids from search hits".to_string(),
            )
        })?;

        let collection_name = self.required_collection_name(request.space_id)?;
        let provider_context = ProviderExecutionContext::from_knowledge_engine_request(
            context,
            QDRANT_IMPLEMENTATION_ID,
            ProviderOperation::Read,
            request.tenant_id,
            request.space_id,
        )
        .map_err(KnowledgeEngineError::from)?;
        client
            .get_point(&provider_context, &collection_name, &point_id)
            .await
    }

    async fn list_documents(
        &self,
        _context: &KnowledgeEngineExecutionContext,
        _request: KnowledgeEngineListRequest,
    ) -> Result<KnowledgeEngineDocumentList, KnowledgeEngineError> {
        Err(KnowledgeEngineError::Unsupported(
            "Qdrant adapter does not expose a document enumeration API".to_string(),
        ))
    }
}

#[async_trait]
impl ExternalKnowledgeEngine for QdrantKnowledgeEngine {
    async fn connector_health(&self) -> Result<KnowledgeEngineHealth, KnowledgeEngineError> {
        self.health().await
    }

    async fn sync_sources(
        &self,
        _context: &KnowledgeEngineExecutionContext,
        _space_id: u64,
    ) -> Result<u32, KnowledgeEngineError> {
        Err(KnowledgeEngineError::Unsupported(
            "Qdrant sync_sources is managed via point upsert APIs; adapter exposes search/read only"
                .to_string(),
        ))
    }
}
