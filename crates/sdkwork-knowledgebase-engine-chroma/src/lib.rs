//! Chroma external knowledge engine adapter (`integrationTier: adapter`).
//!
//! Vendor catalog: `external/knowledge-engines/vendors/chroma/engine.manifest.json`
//! Handlers MUST NOT call Chroma HTTP directly; only this adapter crate may integrate upstream APIs.

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

pub use client::ChromaApiClient;
pub use config::{
    ChromaConnectorConfig, CHROMA_ALLOW_PRIVATE_NETWORK_ENV, CHROMA_BASE_URL_ENV,
    CHROMA_COLLECTION_ID_ENV, CHROMA_DATABASE_ENV, CHROMA_TENANT_ENV, DEFAULT_CHROMA_DATABASE,
    DEFAULT_CHROMA_TENANT,
};

pub const CHROMA_VENDOR_ID: &str = "chroma";
pub const CHROMA_IMPLEMENTATION_ID: &str = "engine.knowledge.external.chroma";
pub const CHROMA_AGENT_PROVIDER_ID: &str = "provider.knowledge.external.chroma";

pub struct ChromaKnowledgeEngine {
    config: Option<ChromaConnectorConfig>,
    client: Option<ChromaApiClient>,
}

impl ChromaKnowledgeEngine {
    pub fn from_env() -> Option<Self> {
        ChromaConnectorConfig::from_env().map(Self::with_config)
    }

    pub fn with_config(config: ChromaConnectorConfig) -> Self {
        match ChromaApiClient::new(config.clone()) {
            Ok(client) => Self {
                config: Some(config),
                client: Some(client),
            },
            Err(error) => {
                sdkwork_knowledgebase_provider_runtime::log_engine_degraded(
                    CHROMA_IMPLEMENTATION_ID,
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
            "Chroma (external adapter)"
        } else {
            "Chroma (external adapter — unconfigured)"
        };
        if self.config.is_some() {
            descriptor_for_external_search_read(CHROMA_VENDOR_ID, display_name)
        } else {
            descriptor_for_external(CHROMA_VENDOR_ID, display_name)
        }
    }

    fn unconfigured_message(&self) -> String {
        format!(
            "Chroma adapter requires {CHROMA_BASE_URL_ENV}; an active Provider binding supplies the collection id and may supply an optional credential reference"
        )
    }

    fn required_collection_id(&self, space_id: u64) -> Result<String, KnowledgeEngineError> {
        self.config
            .as_ref()
            .and_then(|config| config.default_collection_id.clone())
            .ok_or_else(|| {
                KnowledgeEngineError::Validation(format!(
                    "Chroma execution requires an active Provider binding with a remote resource id for space_id={space_id}"
                ))
            })
    }
}

#[async_trait]
impl KnowledgeEngine for ChromaKnowledgeEngine {
    fn descriptor(&self) -> KnowledgeEngineDescriptor {
        self.descriptor_value()
    }

    fn bind_provider(
        &self,
        binding: &sdkwork_knowledgebase_contract::provider_binding::KnowledgeEngineProviderBinding,
        credential: Option<KnowledgeEngineProviderCredential>,
    ) -> Result<Arc<dyn KnowledgeEngine>, KnowledgeEngineError> {
        if binding.implementation_id != CHROMA_IMPLEMENTATION_ID {
            return Err(KnowledgeEngineError::Validation(
                "Chroma cannot bind a different Provider implementation".to_string(),
            ));
        }
        let mut config = self
            .config
            .clone()
            .ok_or_else(|| KnowledgeEngineError::Unsupported(self.unconfigured_message()))?;
        config.api_key = credential.map(KnowledgeEngineProviderCredential::into_secret);
        config.default_collection_id = Some(binding.remote_resource_id.clone());
        let engine = Self::with_config(config);
        if engine.client.is_none() {
            return Err(KnowledgeEngineError::Validation(
                "Chroma base URL does not satisfy the Provider runtime target policy".to_string(),
            ));
        }
        Ok(Arc::new(engine))
    }

    async fn health(&self) -> Result<KnowledgeEngineHealth, KnowledgeEngineError> {
        let Some(client) = self.client.as_ref() else {
            return Ok(KnowledgeEngineHealth {
                implementation_id: CHROMA_IMPLEMENTATION_ID.to_string(),
                status: KnowledgeEngineHealthStatus::Degraded,
                detail: Some(self.unconfigured_message()),
            });
        };

        match client.connector_health().await {
            Ok(()) => Ok(KnowledgeEngineHealth {
                implementation_id: CHROMA_IMPLEMENTATION_ID.to_string(),
                status: KnowledgeEngineHealthStatus::Available,
                detail: None,
            }),
            Err(error) => Ok(KnowledgeEngineHealth {
                implementation_id: CHROMA_IMPLEMENTATION_ID.to_string(),
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

        let collection_id = self.required_collection_id(request.space_id)?;
        let provider_context = ProviderExecutionContext::from_knowledge_engine_request(
            context,
            CHROMA_IMPLEMENTATION_ID,
            ProviderOperation::Search,
            request.tenant_id,
            request.space_id,
        )
        .map_err(KnowledgeEngineError::from)?;
        client
            .query_collection(
                &provider_context,
                request.space_id,
                &collection_id,
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

        let (_, record_id) =
            parse_compound_document_ref(&request.document_id).ok_or_else(|| {
                KnowledgeEngineError::Validation(
                    "Chroma read_document requires title#recordId ids from search hits".to_string(),
                )
            })?;

        let collection_id = self.required_collection_id(request.space_id)?;
        let provider_context = ProviderExecutionContext::from_knowledge_engine_request(
            context,
            CHROMA_IMPLEMENTATION_ID,
            ProviderOperation::Read,
            request.tenant_id,
            request.space_id,
        )
        .map_err(KnowledgeEngineError::from)?;
        client
            .get_record(&provider_context, &collection_id, &record_id)
            .await
    }

    async fn list_documents(
        &self,
        _context: &KnowledgeEngineExecutionContext,
        _request: KnowledgeEngineListRequest,
    ) -> Result<KnowledgeEngineDocumentList, KnowledgeEngineError> {
        Err(KnowledgeEngineError::Unsupported(
            "Chroma adapter does not expose a document enumeration API".to_string(),
        ))
    }
}

#[async_trait]
impl ExternalKnowledgeEngine for ChromaKnowledgeEngine {
    async fn connector_health(&self) -> Result<KnowledgeEngineHealth, KnowledgeEngineError> {
        self.health().await
    }

    async fn sync_sources(
        &self,
        _context: &KnowledgeEngineExecutionContext,
        _space_id: u64,
    ) -> Result<u32, KnowledgeEngineError> {
        Err(KnowledgeEngineError::Unsupported(
            "Chroma sync_sources is managed via collection ingest APIs; adapter exposes search/read only"
                .to_string(),
        ))
    }
}
