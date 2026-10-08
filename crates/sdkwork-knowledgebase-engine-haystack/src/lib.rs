//! Haystack external knowledge engine adapter (`integrationTier: adapter`).
//!
//! Vendor catalog: `external/knowledge-engines/vendors/haystack/engine.manifest.json`
//! Handlers MUST NOT call Haystack HTTP directly; only this adapter crate may integrate upstream APIs.

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

pub use client::{chunk_id_from_content, HaystackApiClient};
pub use config::{
    HaystackConnectorConfig, HaystackDeploymentMode, HAYSTACK_ALLOW_PRIVATE_NETWORK_ENV,
    HAYSTACK_BASE_URL_ENV, HAYSTACK_DEPLOYMENT_MODE_ENV, HAYSTACK_PIPELINE_ENV,
    HAYSTACK_QUERY_FIELD_ENV, HAYSTACK_WORKSPACE_ENV,
};

pub const HAYSTACK_VENDOR_ID: &str = "haystack";
pub const HAYSTACK_IMPLEMENTATION_ID: &str = "engine.knowledge.external.haystack";
pub const HAYSTACK_AGENT_PROVIDER_ID: &str = "provider.knowledge.external.haystack";

pub struct HaystackKnowledgeEngine {
    config: Option<HaystackConnectorConfig>,
    client: Option<HaystackApiClient>,
}

impl HaystackKnowledgeEngine {
    pub fn from_env() -> Option<Self> {
        HaystackConnectorConfig::from_env().map(Self::with_config)
    }

    pub fn with_config(config: HaystackConnectorConfig) -> Self {
        match HaystackApiClient::new(config.clone()) {
            Ok(client) => Self {
                config: Some(config),
                client: Some(client),
            },
            Err(error) => {
                sdkwork_knowledgebase_provider_runtime::log_engine_degraded(
                    HAYSTACK_IMPLEMENTATION_ID,
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
            "Haystack (external adapter)"
        } else {
            "Haystack (external adapter — unconfigured)"
        };
        if self.config.is_some() {
            descriptor_for_external_search_read(HAYSTACK_VENDOR_ID, display_name)
        } else {
            descriptor_for_external(HAYSTACK_VENDOR_ID, display_name)
        }
    }

    fn unconfigured_message(&self) -> String {
        format!(
            "Haystack adapter requires {HAYSTACK_BASE_URL_ENV}; an active Provider binding supplies the pipeline and may supply an optional credential reference; {HAYSTACK_WORKSPACE_ENV} selects the Deepset Cloud workspace"
        )
    }

    fn required_pipeline(&self, space_id: u64) -> Result<String, KnowledgeEngineError> {
        self.config
            .as_ref()
            .and_then(|config| config.default_pipeline.clone())
            .ok_or_else(|| {
                KnowledgeEngineError::Validation(format!(
                    "Haystack execution requires an active Provider binding with a remote resource id for space_id={space_id}"
                ))
            })
    }

    fn workspace(&self) -> Option<String> {
        self.config
            .as_ref()
            .and_then(|config| config.default_workspace.clone())
    }
}

#[async_trait]
impl KnowledgeEngine for HaystackKnowledgeEngine {
    fn descriptor(&self) -> KnowledgeEngineDescriptor {
        self.descriptor_value()
    }

    fn bind_provider(
        &self,
        binding: &sdkwork_knowledgebase_contract::provider_binding::KnowledgeEngineProviderBinding,
        credential: Option<KnowledgeEngineProviderCredential>,
    ) -> Result<Arc<dyn KnowledgeEngine>, KnowledgeEngineError> {
        if binding.implementation_id != HAYSTACK_IMPLEMENTATION_ID {
            return Err(KnowledgeEngineError::Validation(
                "Haystack cannot bind a different Provider implementation".to_string(),
            ));
        }
        let mut config = self
            .config
            .clone()
            .ok_or_else(|| KnowledgeEngineError::Unsupported(self.unconfigured_message()))?;
        config.api_key = credential.map(KnowledgeEngineProviderCredential::into_secret);
        config.default_pipeline = Some(binding.remote_resource_id.clone());
        let engine = Self::with_config(config);
        if engine.client.is_none() {
            return Err(KnowledgeEngineError::Validation(
                "Haystack base URL does not satisfy the Provider runtime target policy".to_string(),
            ));
        }
        Ok(Arc::new(engine))
    }

    async fn health(&self) -> Result<KnowledgeEngineHealth, KnowledgeEngineError> {
        let Some(client) = self.client.as_ref() else {
            return Ok(KnowledgeEngineHealth {
                implementation_id: HAYSTACK_IMPLEMENTATION_ID.to_string(),
                status: KnowledgeEngineHealthStatus::Degraded,
                detail: Some(self.unconfigured_message()),
            });
        };

        let pipeline = match self
            .config
            .as_ref()
            .and_then(|config| config.default_pipeline.clone())
        {
            Some(pipeline) => pipeline,
            None => {
                return Ok(KnowledgeEngineHealth {
                    implementation_id: HAYSTACK_IMPLEMENTATION_ID.to_string(),
                    status: KnowledgeEngineHealthStatus::Degraded,
                    detail: Some(
                        "Haystack connector health requires an active Provider binding with a remote resource id"
                            .to_string(),
                    ),
                });
            }
        };

        let workspace = self
            .config
            .as_ref()
            .and_then(|config| config.default_workspace.clone());

        match client
            .connector_health(workspace.as_deref(), &pipeline)
            .await
        {
            Ok(()) => Ok(KnowledgeEngineHealth {
                implementation_id: HAYSTACK_IMPLEMENTATION_ID.to_string(),
                status: KnowledgeEngineHealthStatus::Available,
                detail: None,
            }),
            Err(error) => Ok(KnowledgeEngineHealth {
                implementation_id: HAYSTACK_IMPLEMENTATION_ID.to_string(),
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

        let pipeline = self.required_pipeline(request.space_id)?;
        let workspace = self.workspace();
        let provider_context = ProviderExecutionContext::from_knowledge_engine_request(
            context,
            HAYSTACK_IMPLEMENTATION_ID,
            ProviderOperation::Search,
            request.tenant_id,
            request.space_id,
        )
        .map_err(KnowledgeEngineError::from)?;
        client
            .search(
                &provider_context,
                request.space_id,
                workspace.as_deref(),
                &pipeline,
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
                    "Haystack read_document requires title#documentId ids from search hits"
                        .to_string(),
                )
            })?;

        let pipeline = self.required_pipeline(request.space_id)?;
        let workspace = self.workspace();
        let provider_context = ProviderExecutionContext::from_knowledge_engine_request(
            context,
            HAYSTACK_IMPLEMENTATION_ID,
            ProviderOperation::Read,
            request.tenant_id,
            request.space_id,
        )
        .map_err(KnowledgeEngineError::from)?;
        client
            .read_document(
                &provider_context,
                request.space_id,
                workspace.as_deref(),
                &pipeline,
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
            "Haystack adapter does not expose a document enumeration API".to_string(),
        ))
    }
}

#[async_trait]
impl ExternalKnowledgeEngine for HaystackKnowledgeEngine {
    async fn connector_health(&self) -> Result<KnowledgeEngineHealth, KnowledgeEngineError> {
        self.health().await
    }

    async fn sync_sources(
        &self,
        _context: &KnowledgeEngineExecutionContext,
        _space_id: u64,
    ) -> Result<u32, KnowledgeEngineError> {
        Err(KnowledgeEngineError::Unsupported(
            "Haystack sync_sources is managed via pipeline deployment; adapter exposes search/read only"
                .to_string(),
        ))
    }
}
