//! Flowise external knowledge engine adapter (`integrationTier: adapter`).
//!
//! Vendor catalog: `external/knowledge-engines/vendors/flowise/engine.manifest.json`
//! Handlers MUST NOT call Flowise HTTP directly; only this adapter crate may integrate upstream APIs.

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

pub use client::{chunk_id_from_content, FlowiseApiClient};
pub use config::{
    FlowiseConnectorConfig, FLOWISE_ALLOW_PRIVATE_NETWORK_ENV, FLOWISE_BASE_URL_ENV,
    FLOWISE_STORE_ID_ENV,
};

pub const FLOWISE_VENDOR_ID: &str = "flowise";
pub const FLOWISE_IMPLEMENTATION_ID: &str = "engine.knowledge.external.flowise";
pub const FLOWISE_AGENT_PROVIDER_ID: &str = "provider.knowledge.external.flowise";

pub struct FlowiseKnowledgeEngine {
    config: Option<FlowiseConnectorConfig>,
    client: Option<FlowiseApiClient>,
}

impl FlowiseKnowledgeEngine {
    pub fn from_env() -> Option<Self> {
        FlowiseConnectorConfig::from_env().map(Self::with_config)
    }

    pub fn with_config(config: FlowiseConnectorConfig) -> Self {
        match FlowiseApiClient::new(config.clone()) {
            Ok(client) => Self {
                config: Some(config),
                client: Some(client),
            },
            Err(error) => {
                sdkwork_knowledgebase_provider_runtime::log_engine_degraded(
                    FLOWISE_IMPLEMENTATION_ID,
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
            "Flowise (external adapter)"
        } else {
            "Flowise (external adapter — unconfigured)"
        };
        if self.config.is_some() {
            descriptor_for_external_search_read(FLOWISE_VENDOR_ID, display_name)
        } else {
            descriptor_for_external(FLOWISE_VENDOR_ID, display_name)
        }
    }

    fn unconfigured_message(&self) -> String {
        format!(
            "Flowise adapter requires {FLOWISE_BASE_URL_ENV}; an active Provider binding must supply a credential reference and the document store id"
        )
    }

    fn required_store_id(&self, space_id: u64) -> Result<String, KnowledgeEngineError> {
        self.config
            .as_ref()
            .and_then(|config| config.default_store_id.clone())
            .ok_or_else(|| {
                KnowledgeEngineError::Validation(format!(
                    "Flowise execution requires an active Provider binding with a remote resource id for space_id={space_id}"
                ))
            })
    }
}

#[async_trait]
impl KnowledgeEngine for FlowiseKnowledgeEngine {
    fn descriptor(&self) -> KnowledgeEngineDescriptor {
        self.descriptor_value()
    }

    fn bind_provider(
        &self,
        binding: &sdkwork_knowledgebase_contract::provider_binding::KnowledgeEngineProviderBinding,
        credential: Option<KnowledgeEngineProviderCredential>,
    ) -> Result<Arc<dyn KnowledgeEngine>, KnowledgeEngineError> {
        if binding.implementation_id != FLOWISE_IMPLEMENTATION_ID {
            return Err(KnowledgeEngineError::Validation(
                "Flowise cannot bind a different Provider implementation".to_string(),
            ));
        }
        let mut config = self
            .config
            .clone()
            .ok_or_else(|| KnowledgeEngineError::Unsupported(self.unconfigured_message()))?;
        config.api_key = credential
            .ok_or_else(|| {
                KnowledgeEngineError::Validation(
                    "Flowise Provider binding requires a credential reference".to_string(),
                )
            })?
            .into_secret();
        config.default_store_id = Some(binding.remote_resource_id.clone());
        let engine = Self::with_config(config);
        if engine.client.is_none() {
            return Err(KnowledgeEngineError::Validation(
                "Flowise base URL does not satisfy the Provider runtime target policy".to_string(),
            ));
        }
        Ok(Arc::new(engine))
    }

    async fn health(&self) -> Result<KnowledgeEngineHealth, KnowledgeEngineError> {
        let Some(client) = self.client.as_ref() else {
            return Ok(KnowledgeEngineHealth {
                implementation_id: FLOWISE_IMPLEMENTATION_ID.to_string(),
                status: KnowledgeEngineHealthStatus::Degraded,
                detail: Some(self.unconfigured_message()),
            });
        };

        let store_id = match self
            .config
            .as_ref()
            .and_then(|config| config.default_store_id.clone())
        {
            Some(store_id) => store_id,
            None => {
                return Ok(KnowledgeEngineHealth {
                    implementation_id: FLOWISE_IMPLEMENTATION_ID.to_string(),
                    status: KnowledgeEngineHealthStatus::Degraded,
                    detail: Some(
                        "Flowise connector health requires an active Provider binding with a remote resource id"
                            .to_string(),
                    ),
                });
            }
        };

        match client.connector_health(&store_id).await {
            Ok(()) => Ok(KnowledgeEngineHealth {
                implementation_id: FLOWISE_IMPLEMENTATION_ID.to_string(),
                status: KnowledgeEngineHealthStatus::Available,
                detail: None,
            }),
            Err(error) => Ok(KnowledgeEngineHealth {
                implementation_id: FLOWISE_IMPLEMENTATION_ID.to_string(),
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

        let store_id = self.required_store_id(request.space_id)?;
        let provider_context = ProviderExecutionContext::from_knowledge_engine_request(
            context,
            FLOWISE_IMPLEMENTATION_ID,
            ProviderOperation::Search,
            request.tenant_id,
            request.space_id,
        )
        .map_err(KnowledgeEngineError::from)?;
        client
            .query_vector_store(
                &provider_context,
                request.space_id,
                &store_id,
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

        let (document_hint, chunk_id) = parse_compound_document_ref(&request.document_id)
            .ok_or_else(|| {
                KnowledgeEngineError::Validation(
                    "Flowise read_document requires source#chunkId ids from search hits"
                        .to_string(),
                )
            })?;

        let store_id = self.required_store_id(request.space_id)?;
        let provider_context = ProviderExecutionContext::from_knowledge_engine_request(
            context,
            FLOWISE_IMPLEMENTATION_ID,
            ProviderOperation::Read,
            request.tenant_id,
            request.space_id,
        )
        .map_err(KnowledgeEngineError::from)?;
        client
            .read_chunk(
                &provider_context,
                request.space_id,
                &store_id,
                &document_hint,
                &chunk_id,
            )
            .await
    }

    async fn list_documents(
        &self,
        _context: &KnowledgeEngineExecutionContext,
        _request: KnowledgeEngineListRequest,
    ) -> Result<KnowledgeEngineDocumentList, KnowledgeEngineError> {
        Err(KnowledgeEngineError::Unsupported(
            "adapter-tier list_documents is unsupported; use search hits or native ingestion"
                .to_string(),
        ))
    }
}

#[async_trait]
impl ExternalKnowledgeEngine for FlowiseKnowledgeEngine {
    async fn connector_health(&self) -> Result<KnowledgeEngineHealth, KnowledgeEngineError> {
        self.health().await
    }

    async fn sync_sources(
        &self,
        _context: &KnowledgeEngineExecutionContext,
        _space_id: u64,
    ) -> Result<u32, KnowledgeEngineError> {
        Err(KnowledgeEngineError::Unsupported(
            "Flowise sync_sources is managed via document store UI; adapter exposes search/read only"
                .to_string(),
        ))
    }
}
