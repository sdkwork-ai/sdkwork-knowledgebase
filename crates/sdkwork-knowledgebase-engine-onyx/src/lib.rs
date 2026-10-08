//! Onyx external knowledge engine adapter (`integrationTier: adapter`).
//!
//! Vendor catalog: `external/knowledge-engines/vendors/onyx/engine.manifest.json`
//! Handlers MUST NOT call Onyx HTTP directly; only this adapter crate may integrate upstream APIs.

mod client;
mod config;

use async_trait::async_trait;
use sdkwork_intelligence_knowledgebase_service::knowledge_engine::KnowledgeEngine;
use sdkwork_intelligence_knowledgebase_service::ports::knowledge_engine::ExternalKnowledgeEngine;
use sdkwork_intelligence_knowledgebase_service::ports::knowledge_provider_credential_resolver::KnowledgeEngineProviderCredential;
use sdkwork_knowledgebase_contract::knowledge_engine::{
    descriptor_for_external, descriptor_for_external_search_read, KnowledgeEngineDescriptor,
    KnowledgeEngineDocument, KnowledgeEngineDocumentList, KnowledgeEngineError,
    KnowledgeEngineHealth, KnowledgeEngineHealthStatus, KnowledgeEngineListRequest,
    KnowledgeEngineReadRequest, KnowledgeEngineSearchRequest, KnowledgeEngineSearchResult,
};
use sdkwork_knowledgebase_contract::provider_binding::KnowledgeEngineExecutionContext;
use sdkwork_knowledgebase_provider_runtime::{ProviderExecutionContext, ProviderOperation};
use std::sync::Arc;

pub use client::{decode_url_document_id, encode_url_document_id, OnyxApiClient};
pub use config::{OnyxConnectorConfig, ONYX_ALLOW_PRIVATE_NETWORK_ENV, ONYX_BASE_URL_ENV};

pub const ONYX_VENDOR_ID: &str = "onyx";
pub const ONYX_IMPLEMENTATION_ID: &str = "engine.knowledge.external.onyx";
pub const ONYX_AGENT_PROVIDER_ID: &str = "provider.knowledge.external.onyx";

pub struct OnyxKnowledgeEngine {
    config: Option<OnyxConnectorConfig>,
    client: Option<OnyxApiClient>,
}

impl OnyxKnowledgeEngine {
    pub fn from_env() -> Option<Self> {
        OnyxConnectorConfig::from_env().map(Self::with_config)
    }

    pub fn with_config(config: OnyxConnectorConfig) -> Self {
        match OnyxApiClient::new(config.clone()) {
            Ok(client) => Self {
                config: Some(config),
                client: Some(client),
            },
            Err(error) => {
                sdkwork_knowledgebase_provider_runtime::log_engine_degraded(
                    ONYX_IMPLEMENTATION_ID,
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
            "Onyx (external adapter)"
        } else {
            "Onyx (external adapter — unconfigured)"
        };
        if self.config.is_some() {
            descriptor_for_external_search_read(ONYX_VENDOR_ID, display_name)
        } else {
            descriptor_for_external(ONYX_VENDOR_ID, display_name)
        }
    }

    fn unconfigured_message(&self) -> String {
        format!(
            "Onyx adapter requires {ONYX_BASE_URL_ENV}; an active Provider binding must supply a credential reference"
        )
    }
}

#[async_trait]
impl KnowledgeEngine for OnyxKnowledgeEngine {
    fn descriptor(&self) -> KnowledgeEngineDescriptor {
        self.descriptor_value()
    }

    fn bind_provider(
        &self,
        binding: &sdkwork_knowledgebase_contract::provider_binding::KnowledgeEngineProviderBinding,
        credential: Option<KnowledgeEngineProviderCredential>,
    ) -> Result<Arc<dyn KnowledgeEngine>, KnowledgeEngineError> {
        if binding.implementation_id != ONYX_IMPLEMENTATION_ID {
            return Err(KnowledgeEngineError::Validation(
                "Onyx cannot bind a different Provider implementation".to_string(),
            ));
        }
        let mut config = self
            .config
            .clone()
            .ok_or_else(|| KnowledgeEngineError::Unsupported(self.unconfigured_message()))?;
        config.api_key = credential
            .ok_or_else(|| {
                KnowledgeEngineError::Validation(
                    "Onyx Provider binding requires a credential reference".to_string(),
                )
            })?
            .into_secret();
        let engine = Self::with_config(config);
        if engine.client.is_none() {
            return Err(KnowledgeEngineError::Validation(
                "Onyx base URL does not satisfy the Provider runtime target policy".to_string(),
            ));
        }
        Ok(Arc::new(engine))
    }

    async fn health(&self) -> Result<KnowledgeEngineHealth, KnowledgeEngineError> {
        let Some(client) = self.client.as_ref() else {
            return Ok(KnowledgeEngineHealth {
                implementation_id: ONYX_IMPLEMENTATION_ID.to_string(),
                status: KnowledgeEngineHealthStatus::Degraded,
                detail: Some(self.unconfigured_message()),
            });
        };

        match client.connector_health().await {
            Ok(()) => Ok(KnowledgeEngineHealth {
                implementation_id: ONYX_IMPLEMENTATION_ID.to_string(),
                status: KnowledgeEngineHealthStatus::Available,
                detail: None,
            }),
            Err(error) => Ok(KnowledgeEngineHealth {
                implementation_id: ONYX_IMPLEMENTATION_ID.to_string(),
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

        let provider_context = ProviderExecutionContext::from_knowledge_engine_request(
            context,
            ONYX_IMPLEMENTATION_ID,
            ProviderOperation::Search,
            request.tenant_id,
            request.space_id,
        )
        .map_err(KnowledgeEngineError::from)?;
        client
            .search(&provider_context, request.space_id, &request.query)
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

        let Some(url) = decode_url_document_id(&request.document_id) else {
            return Err(KnowledgeEngineError::Validation(
                "Onyx read_document requires url:{url} document ids from search hits".to_string(),
            ));
        };

        let provider_context = ProviderExecutionContext::from_knowledge_engine_request(
            context,
            ONYX_IMPLEMENTATION_ID,
            ProviderOperation::Read,
            request.tenant_id,
            request.space_id,
        )
        .map_err(KnowledgeEngineError::from)?;
        client.read_url_document(&provider_context, &url).await
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
impl ExternalKnowledgeEngine for OnyxKnowledgeEngine {
    async fn connector_health(&self) -> Result<KnowledgeEngineHealth, KnowledgeEngineError> {
        self.health().await
    }

    async fn sync_sources(
        &self,
        _context: &KnowledgeEngineExecutionContext,
        _space_id: u64,
    ) -> Result<u32, KnowledgeEngineError> {
        Err(KnowledgeEngineError::Unsupported(
            "Onyx sync_sources is managed via Onyx connector admin; adapter exposes search/read only"
                .to_string(),
        ))
    }
}
