//! RAGFlow external knowledge engine adapter (`integrationTier: adapter`).
//!
//! Vendor catalog: `external/knowledge-engines/vendors/ragflow/engine.manifest.json`
//! Handlers MUST NOT call RAGFlow HTTP directly; only this adapter crate may integrate upstream APIs.

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

pub use client::RagflowApiClient;
pub use config::{
    RagflowConnectorConfig, RAGFLOW_ALLOW_PRIVATE_NETWORK_ENV, RAGFLOW_BASE_URL_ENV,
    RAGFLOW_DATASET_ID_ENV,
};

pub const RAGFLOW_VENDOR_ID: &str = "ragflow";
pub const RAGFLOW_IMPLEMENTATION_ID: &str = "engine.knowledge.external.ragflow";
pub const RAGFLOW_AGENT_PROVIDER_ID: &str = "provider.knowledge.external.ragflow";

pub struct RagflowKnowledgeEngine {
    config: Option<RagflowConnectorConfig>,
    client: Option<RagflowApiClient>,
}

impl RagflowKnowledgeEngine {
    pub fn from_env() -> Option<Self> {
        RagflowConnectorConfig::from_env().map(Self::with_config)
    }

    pub fn with_config(config: RagflowConnectorConfig) -> Self {
        match RagflowApiClient::new(config.clone()) {
            Ok(client) => Self {
                config: Some(config),
                client: Some(client),
            },
            Err(error) => {
                sdkwork_knowledgebase_provider_runtime::log_engine_degraded(
                    RAGFLOW_IMPLEMENTATION_ID,
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
            "RAGFlow (external adapter)"
        } else {
            "RAGFlow (external adapter — unconfigured)"
        };
        if self.config.is_some() {
            descriptor_for_external_search_read(RAGFLOW_VENDOR_ID, display_name)
        } else {
            descriptor_for_external(RAGFLOW_VENDOR_ID, display_name)
        }
    }

    fn unconfigured_message(&self) -> String {
        format!(
            "RAGFlow adapter requires {RAGFLOW_BASE_URL_ENV}; an active Provider binding must supply a credential reference and the dataset id"
        )
    }

    fn required_dataset_id(&self, space_id: u64) -> Result<String, KnowledgeEngineError> {
        self.config
            .as_ref()
            .and_then(|config| config.default_dataset_id.clone())
            .ok_or_else(|| {
                KnowledgeEngineError::Validation(format!(
                    "RAGFlow execution requires an active Provider binding with a remote resource id for space_id={space_id}"
                ))
            })
    }
}

#[async_trait]
impl KnowledgeEngine for RagflowKnowledgeEngine {
    fn descriptor(&self) -> KnowledgeEngineDescriptor {
        self.descriptor_value()
    }

    fn bind_provider(
        &self,
        binding: &sdkwork_knowledgebase_contract::provider_binding::KnowledgeEngineProviderBinding,
        credential: Option<KnowledgeEngineProviderCredential>,
    ) -> Result<Arc<dyn KnowledgeEngine>, KnowledgeEngineError> {
        if binding.implementation_id != RAGFLOW_IMPLEMENTATION_ID {
            return Err(KnowledgeEngineError::Validation(
                "RAGFlow cannot bind a different Provider implementation".to_string(),
            ));
        }
        let mut config = self
            .config
            .clone()
            .ok_or_else(|| KnowledgeEngineError::Unsupported(self.unconfigured_message()))?;
        config.api_key = credential
            .ok_or_else(|| {
                KnowledgeEngineError::Validation(
                    "RAGFlow Provider binding requires a credential reference".to_string(),
                )
            })?
            .into_secret();
        config.default_dataset_id = Some(binding.remote_resource_id.clone());
        let engine = Self::with_config(config);
        if engine.client.is_none() {
            return Err(KnowledgeEngineError::Validation(
                "RAGFlow base URL does not satisfy the Provider runtime target policy".to_string(),
            ));
        }
        Ok(Arc::new(engine))
    }

    async fn health(&self) -> Result<KnowledgeEngineHealth, KnowledgeEngineError> {
        let Some(client) = self.client.as_ref() else {
            return Ok(KnowledgeEngineHealth {
                implementation_id: RAGFLOW_IMPLEMENTATION_ID.to_string(),
                status: KnowledgeEngineHealthStatus::Degraded,
                detail: Some(self.unconfigured_message()),
            });
        };

        let dataset_id = match self
            .config
            .as_ref()
            .and_then(|config| config.default_dataset_id.clone())
        {
            Some(dataset_id) => dataset_id,
            None => {
                return Ok(KnowledgeEngineHealth {
                    implementation_id: RAGFLOW_IMPLEMENTATION_ID.to_string(),
                    status: KnowledgeEngineHealthStatus::Degraded,
                    detail: Some(
                        "RAGFlow connector health requires an active Provider binding with a remote resource id"
                            .to_string(),
                    ),
                });
            }
        };

        match client.connector_health(&dataset_id).await {
            Ok(()) => Ok(KnowledgeEngineHealth {
                implementation_id: RAGFLOW_IMPLEMENTATION_ID.to_string(),
                status: KnowledgeEngineHealthStatus::Available,
                detail: None,
            }),
            Err(error) => Ok(KnowledgeEngineHealth {
                implementation_id: RAGFLOW_IMPLEMENTATION_ID.to_string(),
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

        let dataset_id = self.required_dataset_id(request.space_id)?;
        let provider_context = ProviderExecutionContext::from_knowledge_engine_request(
            context,
            RAGFLOW_IMPLEMENTATION_ID,
            ProviderOperation::Search,
            request.tenant_id,
            request.space_id,
        )
        .map_err(KnowledgeEngineError::from)?;
        client
            .retrieve(
                &provider_context,
                request.space_id,
                &dataset_id,
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

        let (document_id, chunk_id) = parse_compound_document_ref(&request.document_id)
            .ok_or_else(|| {
                KnowledgeEngineError::Validation(
                    "RAGFlow read_document requires parentDocument#chunkId ids from search hits"
                        .to_string(),
                )
            })?;

        let dataset_id = self.required_dataset_id(request.space_id)?;
        let provider_context = ProviderExecutionContext::from_knowledge_engine_request(
            context,
            RAGFLOW_IMPLEMENTATION_ID,
            ProviderOperation::Read,
            request.tenant_id,
            request.space_id,
        )
        .map_err(KnowledgeEngineError::from)?;
        client
            .read_chunk(&provider_context, &dataset_id, &document_id, &chunk_id)
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
impl ExternalKnowledgeEngine for RagflowKnowledgeEngine {
    async fn connector_health(&self) -> Result<KnowledgeEngineHealth, KnowledgeEngineError> {
        self.health().await
    }

    async fn sync_sources(
        &self,
        _context: &KnowledgeEngineExecutionContext,
        _space_id: u64,
    ) -> Result<u32, KnowledgeEngineError> {
        Err(KnowledgeEngineError::Unsupported(
            "RAGFlow sync_sources is unsupported at adapter tier; use RAGFlow console ingestion"
                .to_string(),
        ))
    }
}
