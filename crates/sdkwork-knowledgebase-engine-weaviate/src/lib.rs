//! Weaviate external knowledge engine adapter (`integrationTier: adapter`).
//!
//! Vendor catalog: `external/knowledge-engines/vendors/weaviate/engine.manifest.json`
//! Handlers MUST NOT call Weaviate HTTP directly; only this adapter crate may integrate upstream APIs.

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

pub use client::WeaviateApiClient;
pub use config::{
    WeaviateConnectorConfig, DEFAULT_WEAVIATE_CONTENT_PROPERTY, DEFAULT_WEAVIATE_TITLE_PROPERTY,
    WEAVIATE_ALLOW_PRIVATE_NETWORK_ENV, WEAVIATE_BASE_URL_ENV, WEAVIATE_CLASS_NAME_ENV,
    WEAVIATE_CONTENT_PROPERTY_ENV, WEAVIATE_TITLE_PROPERTY_ENV,
};

pub const WEAVIATE_VENDOR_ID: &str = "weaviate";
pub const WEAVIATE_IMPLEMENTATION_ID: &str = "engine.knowledge.external.weaviate";
pub const WEAVIATE_AGENT_PROVIDER_ID: &str = "provider.knowledge.external.weaviate";

pub struct WeaviateKnowledgeEngine {
    config: Option<WeaviateConnectorConfig>,
    client: Option<WeaviateApiClient>,
}

impl WeaviateKnowledgeEngine {
    pub fn from_env() -> Option<Self> {
        WeaviateConnectorConfig::from_env().map(Self::with_config)
    }

    pub fn with_config(config: WeaviateConnectorConfig) -> Self {
        match WeaviateApiClient::new(config.clone()) {
            Ok(client) => Self {
                config: Some(config),
                client: Some(client),
            },
            Err(error) => {
                sdkwork_knowledgebase_provider_runtime::log_engine_degraded(
                    WEAVIATE_IMPLEMENTATION_ID,
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
            "Weaviate (external adapter)"
        } else {
            "Weaviate (external adapter — unconfigured)"
        };
        if self.config.is_some() {
            descriptor_for_external_search_read(WEAVIATE_VENDOR_ID, display_name)
        } else {
            descriptor_for_external(WEAVIATE_VENDOR_ID, display_name)
        }
    }

    fn unconfigured_message(&self) -> String {
        format!(
            "Weaviate adapter requires {WEAVIATE_BASE_URL_ENV}; an active Provider binding supplies the class name and may supply an optional credential reference"
        )
    }

    fn required_class_name(&self, space_id: u64) -> Result<String, KnowledgeEngineError> {
        self.config
            .as_ref()
            .and_then(|config| config.default_class_name.clone())
            .ok_or_else(|| {
                KnowledgeEngineError::Validation(format!(
                    "Weaviate execution requires an active Provider binding with a remote resource id for space_id={space_id}"
                ))
            })
    }
}

#[async_trait]
impl KnowledgeEngine for WeaviateKnowledgeEngine {
    fn descriptor(&self) -> KnowledgeEngineDescriptor {
        self.descriptor_value()
    }

    fn bind_provider(
        &self,
        binding: &sdkwork_knowledgebase_contract::provider_binding::KnowledgeEngineProviderBinding,
        credential: Option<KnowledgeEngineProviderCredential>,
    ) -> Result<Arc<dyn KnowledgeEngine>, KnowledgeEngineError> {
        if binding.implementation_id != WEAVIATE_IMPLEMENTATION_ID {
            return Err(KnowledgeEngineError::Validation(
                "Weaviate cannot bind a different Provider implementation".to_string(),
            ));
        }
        let mut config = self
            .config
            .clone()
            .ok_or_else(|| KnowledgeEngineError::Unsupported(self.unconfigured_message()))?;
        config.api_key = credential.map(KnowledgeEngineProviderCredential::into_secret);
        config.default_class_name = Some(binding.remote_resource_id.clone());
        let engine = Self::with_config(config);
        if engine.client.is_none() {
            return Err(KnowledgeEngineError::Validation(
                "Weaviate binding was rejected: base URL, class name, or property names do not satisfy the adapter contract"
                    .to_string(),
            ));
        }
        Ok(Arc::new(engine))
    }

    async fn health(&self) -> Result<KnowledgeEngineHealth, KnowledgeEngineError> {
        let Some(client) = self.client.as_ref() else {
            return Ok(KnowledgeEngineHealth {
                implementation_id: WEAVIATE_IMPLEMENTATION_ID.to_string(),
                status: KnowledgeEngineHealthStatus::Degraded,
                detail: Some(self.unconfigured_message()),
            });
        };

        match client.connector_health().await {
            Ok(()) => Ok(KnowledgeEngineHealth {
                implementation_id: WEAVIATE_IMPLEMENTATION_ID.to_string(),
                status: KnowledgeEngineHealthStatus::Available,
                detail: None,
            }),
            Err(error) => Ok(KnowledgeEngineHealth {
                implementation_id: WEAVIATE_IMPLEMENTATION_ID.to_string(),
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

        let class_name = self.required_class_name(request.space_id)?;
        let provider_context = ProviderExecutionContext::from_knowledge_engine_request(
            context,
            WEAVIATE_IMPLEMENTATION_ID,
            ProviderOperation::Search,
            request.tenant_id,
            request.space_id,
        )
        .map_err(KnowledgeEngineError::from)?;
        client
            .near_text_search(
                &provider_context,
                request.space_id,
                &class_name,
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

        let (_, object_id) =
            parse_compound_document_ref(&request.document_id).ok_or_else(|| {
                KnowledgeEngineError::Validation(
                    "Weaviate read_document requires title#objectId ids from search hits"
                        .to_string(),
                )
            })?;

        let class_name = self.required_class_name(request.space_id)?;
        let provider_context = ProviderExecutionContext::from_knowledge_engine_request(
            context,
            WEAVIATE_IMPLEMENTATION_ID,
            ProviderOperation::Read,
            request.tenant_id,
            request.space_id,
        )
        .map_err(KnowledgeEngineError::from)?;
        client
            .get_object(&provider_context, &class_name, &object_id)
            .await
    }

    async fn list_documents(
        &self,
        _context: &KnowledgeEngineExecutionContext,
        _request: KnowledgeEngineListRequest,
    ) -> Result<KnowledgeEngineDocumentList, KnowledgeEngineError> {
        Err(KnowledgeEngineError::Unsupported(
            "Weaviate adapter does not expose a document enumeration API".to_string(),
        ))
    }
}

#[async_trait]
impl ExternalKnowledgeEngine for WeaviateKnowledgeEngine {
    async fn connector_health(&self) -> Result<KnowledgeEngineHealth, KnowledgeEngineError> {
        self.health().await
    }

    async fn sync_sources(
        &self,
        _context: &KnowledgeEngineExecutionContext,
        _space_id: u64,
    ) -> Result<u32, KnowledgeEngineError> {
        Err(KnowledgeEngineError::Unsupported(
            "Weaviate sync_sources is managed via object ingest APIs; adapter exposes search/read only"
                .to_string(),
        ))
    }
}
