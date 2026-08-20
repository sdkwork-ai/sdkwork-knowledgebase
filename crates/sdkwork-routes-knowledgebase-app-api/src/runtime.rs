use async_trait::async_trait;
use sdkwork_database_id::{NodeAllocatorConfig, NodeLease, SnowflakeNodeAllocator};
use sdkwork_drive_storage_local::LocalDriveObjectStore;
use sdkwork_intelligence_knowledgebase_repository_sqlx::{
    connect_knowledgebase_and_install_schema, database_config_from_url,
    default_knowledge_id_generator, install_default_knowledge_id_generator,
    keyword_search_backend_for_database_url, knowledgebase_health_check,
    knowledgebase_process_pool_budget_from_url, require_postgres_rls_organization_id,
    require_postgres_rls_tenant_id, KnowledgeAuditEventRecord, KnowledgeAuditEventStore,
    PgVectorKnowledgeRetrievalBackend, PgVectorLayeredRetrievalBackend, PostgresCommerceStore,
    PostgresContextBindingStore, PostgresDriveImportMetadataStore,
    PostgresGroupKnowledgeSpaceBindingStore, PostgresIngestionJobStore,
    PostgresKnowledgeAgentProfileStore, PostgresKnowledgeAuditEventStore,
    PostgresKnowledgeBrowserProjectionStore, PostgresKnowledgeChunkRetrievalStore,
    PostgresKnowledgeChunkStore, PostgresKnowledgeDocumentStore,
    PostgresKnowledgeDocumentVersionStore, PostgresKnowledgeDriveObjectRefStore,
    PostgresKnowledgeEmbeddingStore, PostgresKnowledgeIndexStore,
    PostgresKnowledgeOkfBundleFileStore, PostgresKnowledgeOkfCandidateStore,
    PostgresKnowledgeOkfConceptLinkStore, PostgresKnowledgeOkfConceptStore,
    PostgresKnowledgeOutboxStore, PostgresKnowledgeRetrievalProfileStore,
    PostgresKnowledgeSourceStore, PostgresKnowledgeSpaceStore, PostgresMarkdownIndexMetadataStore,
    PostgresOkfConceptRevisionMetadataStore, SnowflakeKnowledgeIdGenerator,
    SqlxKnowledgeEngineProviderBindingStore, SqlxKnowledgeEngineProviderMigrationStore,
    SqlxWikiPersistenceStore,
};
use sdkwork_intelligence_knowledgebase_service::{
    agent::KnowledgeAgentService,
    agent_chat::KnowledgeAgentChatService,
    embedding_retrieval_backend::SharedKnowledgeRetrievalBackend,
    group_space::KnowledgeGroupKnowledgeSpaceService,
    knowledge_engine::{
        build_default_registry, DefaultKnowledgeEngineRegistry, KnowledgeEngineRuntimeDeps,
        KnowledgeEngineSpaceResolver,
    },
    okf::{OkfBundleFileRegistryService, OkfBundleInitializerService},
    ports::{
        group_launch_ticket_consumer::GroupLaunchTicketConsumer,
        knowledge_access_control::KnowledgeAccessRole,
        knowledge_chunk_store::KnowledgeChunkStore,
        knowledge_drive_object_ref_store::KnowledgeDriveObjectRefStore,
        knowledge_drive_storage::KnowledgeDriveStorage,
        knowledge_group_space_binding_store::KnowledgeGroupSpaceBindingStore,
        knowledge_outbox_store::KnowledgeOutboxStore,
        knowledge_provider_binding_store::KnowledgeEngineProviderScope,
        knowledge_provider_credential_resolver::KnowledgeEngineProviderCredentialResolver,
        knowledge_retrieval_trace_store::KnowledgeRetrievalTraceStore,
        knowledge_space_store::KnowledgeSpaceStore,
        knowledge_wiki_drive_source::{KnowledgeWikiDriveScope, KnowledgeWikiDriveSource},
    },
    retrieval::{KnowledgeRetrievalService, KnowledgeRetrievalServiceError},
    wiki_public_provider::KnowledgeWikiPublicProviderService,
};
use sdkwork_knowledgebase_contract::agent_chat::{
    KnowledgeAgentChatRequest, KnowledgeAgentChatResponse,
};
use sdkwork_knowledgebase_contract::rag::{
    KnowledgeAgentBinding, KnowledgeAgentBindingList, KnowledgeAgentBindingRequest,
    KnowledgeAgentProfile, KnowledgeAgentProfileRequest, KnowledgeContextPack,
    KnowledgeContextPackRequest, KnowledgeRetrievalRequest, KnowledgeRetrievalResult,
};
use sdkwork_knowledgebase_drive::{
    connect_knowledgebase_drive_pool_with_max_connections, knowledgebase_drive_health_check,
    resolve_cloud_knowledgebase_drive_storage, DomainOutboxDispatchResult,
    KnowledgebaseDriveEmbeddedEventRelay, KnowledgebaseDriveEmbeddedWikiSourceAdapter,
    KnowledgebaseDriveEventDeliveryConfig, KnowledgebaseDriveInternalSdkAdapter,
    KnowledgebaseDriveNodeTreeAdapter, KnowledgebaseDriveSpaceProvisionerAdapter,
    KnowledgebaseDriveStorageAdapter, KnowledgebaseDriveWorkspaceAdapter,
    KnowledgebaseKnowledgeAccessControlAdapter,
};
use sdkwork_knowledgebase_provider_secret_adapter::{
    KnowledgebaseProviderCredentialEnvironment, KnowledgebaseProviderCredentialResolver,
    KnowledgebaseProviderCredentialResolverConfig,
};
use sdkwork_utils_rust::is_blank;
use sdkwork_web_bootstrap::{ReadinessCheck as FrameworkReadinessCheck, ReadinessFuture};
use sqlx::{AnyPool, PgPool};
use std::collections::HashSet;
use std::fs::File;
use std::io::Read;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use tokio::sync::OnceCell;

use sdkwork_knowledgebase_agent_provider::{
    resolve_cloud_router_client_from_env, CloudRouterEmbeddingClient,
};

use crate::{
    agent_chat_runtime::{
        RuntimeKnowledgebaseRetrievalClient, RuntimeOkfKnowledgeClient,
        RuntimeRetrievalPlanResolver, RuntimeSpaceKnowledgeEngineClient, RuntimeSpaceModeResolver,
    },
    build_router_with_shared_app_api_and_readiness,
    hosted::{
        HostedBrowserService, HostedDocumentService, HostedDriveImportService,
        HostedGitImportService, HostedIngestService, HostedOkfService, HostedSpaceService,
    },
    hosted_access::{
        ensure_runtime_tenant, require_actor_id, require_agent_binding_space_access_with_role,
        require_agent_profile_space_access, require_agent_profile_space_access_with_role,
        require_bindings_space_access, require_space_access,
    },
    hosted_backend::HostedBackendApi,
    hosted_commerce::HostedCommerceService,
    hosted_context_binding::HostedContextBindingService,
    hosted_group_launch::HostedGroupLaunchService,
    hosted_open::HostedOpenApi,
    hosted_wechat::HostedWechatService,
    hosted_wiki::HostedWikiPublicationService,
    ApiError, ApiResult, KnowledgeAgentAppService, KnowledgeAppRequestContext,
    KnowledgeRetrievalAppService,
};

const DEFAULT_DRIVE_PROVIDER_ID: &str = "sdkwork-knowledgebase-local";
const DEFAULT_DRIVE_BUCKET: &str = "knowledgebase";
const KNOWLEDGEBASE_ID_SERVICE_NAME: &str = "sdkwork-knowledgebase";
const PROVIDER_SECRETS_DIR_ENV: &str = "SDKWORK_KNOWLEDGEBASE_PROVIDER_SECRETS_DIR";
const DEPLOYMENT_PROFILE_ENV: &str = "SDKWORK_KNOWLEDGEBASE_DEPLOYMENT_PROFILE";
const DRIVE_INTERNAL_API_BASE_URL_ENV: &str = "SDKWORK_KNOWLEDGEBASE_DRIVE_INTERNAL_API_BASE_URL";
const DRIVE_STORAGE_PROVIDER_ID_ENV: &str = "SDKWORK_KNOWLEDGEBASE_DRIVE_STORAGE_PROVIDER_ID";
const DRIVE_INTERNAL_API_INGRESS_TOKEN_FILE_ENV: &str =
    "SDKWORK_KNOWLEDGEBASE_DRIVE_INTERNAL_API_INGRESS_TOKEN_FILE";
const DRIVE_EVENT_CALLBACK_URL_ENV: &str = "SDKWORK_KNOWLEDGEBASE_DRIVE_EVENT_CALLBACK_URL";
const DRIVE_EVENT_SIGNING_SECRET_FILE_ENV: &str =
    "SDKWORK_KNOWLEDGEBASE_DRIVE_EVENT_SIGNING_SECRET_FILE";
const DRIVE_EVENT_PREVIOUS_SIGNING_SECRET_FILE_ENV: &str =
    "SDKWORK_KNOWLEDGEBASE_DRIVE_EVENT_PREVIOUS_SIGNING_SECRET_FILE";
const DRIVE_EVENT_CHANNEL_TTL_SECONDS_ENV: &str =
    "SDKWORK_KNOWLEDGEBASE_DRIVE_EVENT_CHANNEL_TTL_SECONDS";
const DRIVE_EVENT_CALLER_APP_ID_ENV: &str = "SDKWORK_KNOWLEDGEBASE_DRIVE_EVENT_CALLER_APP_ID";
const WIKI_PROVIDER_CALLER_APP_ID_ENV: &str = "SDKWORK_KNOWLEDGEBASE_WIKI_PROVIDER_CALLER_APP_ID";
const MAX_INGRESS_TOKEN_FILE_BYTES: u64 = 16 * 1024;
const MAX_WEBHOOK_SECRET_FILE_BYTES: u64 = 4 * 1024;
const DEFAULT_DRIVE_EVENT_CHANNEL_TTL_SECONDS: u64 = 86_400;

static RUNTIME_NODE_LEASE: OnceCell<Option<NodeLease>> = OnceCell::const_new();

#[derive(Clone)]
pub struct KnowledgebaseRuntime {
    pool: AnyPool,
    drive_pool: PgPool,
    tenant_id: u64,
    organization_id: u64,
    tenant_id_str: String,
    operator_id: String,
    retrieval_store: Arc<PostgresKnowledgeChunkRetrievalStore>,
    retrieval_backend: SharedKnowledgeRetrievalBackend,
    retrieval_profile_store: Arc<PostgresKnowledgeRetrievalProfileStore>,
    index_store: Arc<PostgresKnowledgeIndexStore>,
    embedding_store: Arc<PostgresKnowledgeEmbeddingStore>,
    agent_store: Arc<PostgresKnowledgeAgentProfileStore>,
    space_store: Arc<PostgresKnowledgeSpaceStore>,
    wiki_store: Arc<SqlxWikiPersistenceStore>,
    okf_bundle_file_store: Arc<PostgresKnowledgeOkfBundleFileStore>,
    okf_concept_store: Arc<PostgresKnowledgeOkfConceptStore>,
    okf_concept_link_store: Arc<PostgresKnowledgeOkfConceptLinkStore>,
    okf_candidate_store: Arc<PostgresKnowledgeOkfCandidateStore>,
    document_store: Arc<PostgresKnowledgeDocumentStore>,
    source_store: Arc<PostgresKnowledgeSourceStore>,
    provider_binding_store: Arc<SqlxKnowledgeEngineProviderBindingStore>,
    provider_migration_store: Arc<SqlxKnowledgeEngineProviderMigrationStore>,
    provider_credential_resolver: Arc<dyn KnowledgeEngineProviderCredentialResolver>,
    version_store: Arc<PostgresKnowledgeDocumentVersionStore>,
    object_ref_store: Arc<PostgresKnowledgeDriveObjectRefStore>,
    ingestion_job_store: Arc<PostgresIngestionJobStore>,
    drive_import_metadata_store: Arc<PostgresDriveImportMetadataStore>,
    markdown_index_metadata_store: Arc<PostgresMarkdownIndexMetadataStore>,
    outbox_store: Arc<PostgresKnowledgeOutboxStore>,
    outbox_dispatcher: Arc<dyn sdkwork_intelligence_knowledgebase_service::ports::knowledge_outbox_dispatcher::KnowledgeOutboxDispatcher>,
    chunk_store: Arc<PostgresKnowledgeChunkStore>,
    context_binding_store: Arc<PostgresContextBindingStore>,
    group_space_binding_store: Arc<PostgresGroupKnowledgeSpaceBindingStore>,
    group_launch_ticket_consumer: Option<Arc<dyn GroupLaunchTicketConsumer>>,
    browser_projection_store: Arc<PostgresKnowledgeBrowserProjectionStore>,
    audit_event_store: Arc<PostgresKnowledgeAuditEventStore>,
    drive_storage: Arc<KnowledgebaseDriveStorageAdapter>,
    drive_space_provisioner: Arc<KnowledgebaseDriveSpaceProvisionerAdapter>,
    drive_tree: Arc<KnowledgebaseDriveNodeTreeAdapter>,
    drive_workspace: Arc<KnowledgebaseDriveWorkspaceAdapter>,
    wiki_drive_scope: Arc<dyn KnowledgeWikiDriveScope>,
    wiki_drive_source: Arc<dyn KnowledgeWikiDriveSource>,
    embedded_wiki_drive_event_relay: Option<Arc<KnowledgebaseDriveEmbeddedEventRelay>>,
    wiki_webhook_signing_secrets: Arc<Vec<String>>,
    wiki_event_caller_app_id: Arc<str>,
    wiki_provider_caller_app_id: Arc<str>,
    access_control: Arc<KnowledgebaseKnowledgeAccessControlAdapter>,
    knowledge_engines: Arc<DefaultKnowledgeEngineRegistry>,
    commerce_store: Arc<PostgresCommerceStore>,
    snowflake_node_lease: Option<NodeLease>,
}

/// Bridges the host's complete runtime dependency check into the shared HTTP readiness route.
/// The response deliberately exposes no dependency configuration or failure detail.
#[derive(Clone)]
pub struct KnowledgebaseRuntimeReadinessCheck {
    runtime: KnowledgebaseRuntime,
}

#[derive(Clone)]
struct KnowledgebaseRuntimeWikiDriveEventReceiver {
    runtime: KnowledgebaseRuntime,
}

#[async_trait]
impl sdkwork_routes_knowledgebase_internal_api::KnowledgebaseDriveEventReceiver
    for KnowledgebaseRuntimeWikiDriveEventReceiver
{
    async fn receive_drive_webhook(
        &self,
        request: sdkwork_intelligence_knowledgebase_service::wiki_event_consumer::ReceiveKnowledgeWikiDriveWebhookRequest,
    ) -> Result<
        sdkwork_intelligence_knowledgebase_service::ports::knowledge_wiki_persistence::WikiDriveEventReceipt,
        sdkwork_intelligence_knowledgebase_service::wiki_event_consumer::KnowledgeWikiDriveEventConsumerError,
    >{
        sdkwork_intelligence_knowledgebase_service::wiki_event_consumer::KnowledgeWikiDriveEventConsumerService::new(
            self.runtime.wiki_store(),
            self.runtime.wiki_store(),
            self.runtime.wiki_store(),
            self.runtime.wiki_drive_source(),
        )
        .with_webhook_signing_secrets(
            self.runtime.wiki_webhook_signing_secrets.as_ref().clone(),
        )
        .receive_webhook(request)
        .await
    }
}

impl KnowledgebaseRuntimeReadinessCheck {
    pub fn new(runtime: KnowledgebaseRuntime) -> Self {
        Self { runtime }
    }
}

async fn initialize_runtime_id_generator(
    database_url: &str,
) -> Result<Option<NodeLease>, sqlx::Error> {
    let lease = RUNTIME_NODE_LEASE
        .get_or_try_init(|| async {
            if let Ok(node_id) = std::env::var("SDKWORK_KNOWLEDGEBASE_SNOWFLAKE_NODE_ID") {
                if sdkwork_knowledgebase_observability::is_production_like_environment()
                    && !environment_flag_enabled(
                        "SDKWORK_KNOWLEDGEBASE_ALLOW_STATIC_SNOWFLAKE_NODE_ID",
                    )
                {
                    return Err(configuration_error(
                        "static Snowflake node IDs require SDKWORK_KNOWLEDGEBASE_ALLOW_STATIC_SNOWFLAKE_NODE_ID=true in production-like environments",
                    ));
                }
                let generator = SnowflakeKnowledgeIdGenerator::from_node_id_config(Some(&node_id))
                    .map_err(|error| configuration_error(error.to_string()))?;
                install_default_knowledge_id_generator(generator)
                    .map_err(|error| configuration_error(error.to_string()))?;
                return Ok(None);
            }

            let database_lease_enabled =
                sdkwork_knowledgebase_observability::is_production_like_environment()
                    || environment_flag_enabled(
                        "SDKWORK_DATABASE_NODE_LEASE_ENABLED",
                    );
            if !database_lease_enabled {
                return Ok(None);
            }

            let database_pool =
                sdkwork_intelligence_knowledgebase_repository_sqlx::db::connect_knowledgebase_pool_from_url(
                    database_url,
                )
                .await
                .map_err(|error| configuration_error(error.to_string()))?;
            let config = NodeAllocatorConfig::from_service_name(KNOWLEDGEBASE_ID_SERVICE_NAME);
            let (generator, lease) =
                SnowflakeNodeAllocator::allocate_process_generator(&database_pool, &config)
                    .await
                    .map_err(|error| configuration_error(error.to_string()))?;
            install_default_knowledge_id_generator(SnowflakeKnowledgeIdGenerator::from_generator(
                generator,
            ))
            .map_err(|error| configuration_error(error.to_string()))?;
            Ok(Some(lease))
        })
        .await?;
    Ok(lease.clone())
}

fn environment_flag_enabled(name: &str) -> bool {
    std::env::var(name).ok().is_some_and(|value| {
        matches!(
            value.trim().to_ascii_lowercase().as_str(),
            "1" | "true" | "yes" | "on"
        )
    })
}

fn configuration_error(message: impl Into<String>) -> sqlx::Error {
    sqlx::Error::Configuration(message.into().into())
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum KnowledgebaseDeploymentProfile {
    Standalone,
    Cloud,
}

type KnowledgeWikiDrivePorts = (
    Arc<dyn KnowledgeWikiDriveScope>,
    Arc<dyn KnowledgeWikiDriveSource>,
);

impl KnowledgebaseDeploymentProfile {
    fn resolve() -> Result<Self, sqlx::Error> {
        match std::env::var(DEPLOYMENT_PROFILE_ENV) {
            Ok(value) if value.trim().eq_ignore_ascii_case("standalone") => Ok(Self::Standalone),
            Ok(value) if value.trim().eq_ignore_ascii_case("cloud") => Ok(Self::Cloud),
            Ok(_) => Err(configuration_error(format!(
                "{DEPLOYMENT_PROFILE_ENV} must be standalone or cloud"
            ))),
            Err(std::env::VarError::NotPresent)
                if !sdkwork_knowledgebase_observability::is_production_like_environment() =>
            {
                Ok(Self::Standalone)
            }
            Err(std::env::VarError::NotPresent) => Err(configuration_error(format!(
                "{DEPLOYMENT_PROFILE_ENV} is required in production-like environments"
            ))),
            Err(error) => Err(configuration_error(format!(
                "{DEPLOYMENT_PROFILE_ENV} could not be read: {error}"
            ))),
        }
    }
}

fn required_cloud_drive_storage_provider_id() -> Result<String, sqlx::Error> {
    let value = std::env::var(DRIVE_STORAGE_PROVIDER_ID_ENV).map_err(|_| {
        configuration_error(format!(
            "{DRIVE_STORAGE_PROVIDER_ID_ENV} is required for cloud deployments"
        ))
    })?;
    let value = value.trim();
    if value.is_empty() || value.len() > 255 || value.chars().any(char::is_control) {
        return Err(configuration_error(format!(
            "{DRIVE_STORAGE_PROVIDER_ID_ENV} must be a non-empty provider id of at most 255 bytes"
        )));
    }
    Ok(value.to_string())
}

fn build_wiki_drive_ports(
    drive_pool: PgPool,
    tenant_id: &str,
    operator_id: &str,
    deployment_profile: KnowledgebaseDeploymentProfile,
    event_delivery: Option<KnowledgebaseDriveEventDeliveryConfig>,
) -> Result<KnowledgeWikiDrivePorts, sqlx::Error> {
    match deployment_profile {
        KnowledgebaseDeploymentProfile::Standalone => {
            let adapter = Arc::new(KnowledgebaseDriveEmbeddedWikiSourceAdapter::new(
                drive_pool,
                tenant_id,
                operator_id,
            ));
            let scope: Arc<dyn KnowledgeWikiDriveScope> = adapter.clone();
            let source: Arc<dyn KnowledgeWikiDriveSource> = adapter;
            Ok((scope, source))
        }
        KnowledgebaseDeploymentProfile::Cloud => {
            let base_url = required_drive_internal_api_base_url()?;
            let ingress_token = read_drive_internal_api_ingress_token()?;
            let adapter = Arc::new(
                KnowledgebaseDriveInternalSdkAdapter::from_ingress_token(base_url, ingress_token)
                    .and_then(|adapter| {
                        adapter.with_event_delivery(event_delivery.ok_or_else(|| {
                            sdkwork_intelligence_knowledgebase_service::ports::knowledge_wiki_drive_source::KnowledgeWikiDriveSourceError::InvalidRequest(
                                "Drive event delivery config is required for cloud deployment".to_string(),
                            )
                        })?)
                    })
                    .map_err(|error| configuration_error(error.to_string()))?,
            );
            let scope: Arc<dyn KnowledgeWikiDriveScope> = adapter.clone();
            let source: Arc<dyn KnowledgeWikiDriveSource> = adapter;
            Ok((scope, source))
        }
    }
}

#[derive(Debug, Clone)]
struct ResolvedWikiDriveEventConfig {
    delivery: Option<KnowledgebaseDriveEventDeliveryConfig>,
    signing_secrets: Vec<String>,
    expected_caller_app_id: String,
}

fn resolve_wiki_drive_event_config(
    deployment_profile: KnowledgebaseDeploymentProfile,
) -> Result<ResolvedWikiDriveEventConfig, sqlx::Error> {
    let current_secret = read_optional_bounded_secret_file(
        DRIVE_EVENT_SIGNING_SECRET_FILE_ENV,
        deployment_profile == KnowledgebaseDeploymentProfile::Cloud,
    )?;
    let previous_secret =
        read_optional_bounded_secret_file(DRIVE_EVENT_PREVIOUS_SIGNING_SECRET_FILE_ENV, false)?;
    if previous_secret.is_some() && current_secret.is_none() {
        return Err(configuration_error(format!(
            "{DRIVE_EVENT_PREVIOUS_SIGNING_SECRET_FILE_ENV} requires {DRIVE_EVENT_SIGNING_SECRET_FILE_ENV}"
        )));
    }
    if current_secret.as_ref().is_some_and(|current| {
        previous_secret
            .as_ref()
            .is_some_and(|previous| previous == current)
    }) {
        return Err(configuration_error(
            "current and previous Drive event signing secrets must differ",
        ));
    }
    let mut signing_secrets = Vec::new();
    if let Some(current) = current_secret.clone() {
        signing_secrets.push(current);
    }
    if let Some(previous) = previous_secret {
        signing_secrets.push(previous);
    }
    let expected_caller_app_id =
        resolve_internal_caller_app_id(DRIVE_EVENT_CALLER_APP_ID_ENV, "sdkwork-drive")?;
    let delivery = match deployment_profile {
        KnowledgebaseDeploymentProfile::Standalone => None,
        KnowledgebaseDeploymentProfile::Cloud => {
            let callback_url = std::env::var(DRIVE_EVENT_CALLBACK_URL_ENV).map_err(|_| {
                configuration_error(format!(
                    "{DRIVE_EVENT_CALLBACK_URL_ENV} is required for cloud deployment"
                ))
            })?;
            let ttl = match std::env::var(DRIVE_EVENT_CHANNEL_TTL_SECONDS_ENV) {
                Ok(value) => value.parse::<u64>().map_err(|_| {
                    configuration_error(format!(
                        "{DRIVE_EVENT_CHANNEL_TTL_SECONDS_ENV} must be an unsigned integer"
                    ))
                })?,
                Err(std::env::VarError::NotPresent) => DEFAULT_DRIVE_EVENT_CHANNEL_TTL_SECONDS,
                Err(error) => {
                    return Err(configuration_error(format!(
                        "{DRIVE_EVENT_CHANNEL_TTL_SECONDS_ENV} could not be read: {error}"
                    )))
                }
            };
            let Some(current_secret) = current_secret else {
                return Err(configuration_error(format!(
                    "{DRIVE_EVENT_SIGNING_SECRET_FILE_ENV} is required for cloud deployment"
                )));
            };
            Some(KnowledgebaseDriveEventDeliveryConfig {
                callback_url,
                signing_master_secret: current_secret,
                channel_ttl_seconds: ttl,
            })
        }
    };
    Ok(ResolvedWikiDriveEventConfig {
        delivery,
        signing_secrets,
        expected_caller_app_id,
    })
}

fn resolve_internal_caller_app_id(
    environment_key: &str,
    default_app_id: &str,
) -> Result<String, sqlx::Error> {
    let app_id = std::env::var(environment_key).unwrap_or_else(|_| default_app_id.to_string());
    if app_id.is_empty()
        || app_id.len() > 128
        || !app_id
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'_' | b'.'))
    {
        return Err(configuration_error(format!("{environment_key} is invalid")));
    }
    Ok(app_id)
}

fn read_optional_bounded_secret_file(
    environment_key: &str,
    required: bool,
) -> Result<Option<String>, sqlx::Error> {
    let value = match std::env::var(environment_key) {
        Ok(value) => value,
        Err(std::env::VarError::NotPresent) if !required => return Ok(None),
        Err(std::env::VarError::NotPresent) => {
            return Err(configuration_error(format!(
                "{environment_key} is required for cloud deployment"
            )))
        }
        Err(error) => {
            return Err(configuration_error(format!(
                "{environment_key} could not be read: {error}"
            )))
        }
    };
    if is_blank(Some(&value)) {
        return if required {
            Err(configuration_error(format!(
                "{environment_key} must not be blank"
            )))
        } else {
            Ok(None)
        };
    }
    let path = PathBuf::from(value.trim());
    let secret = read_bounded_utf8_file(&path, MAX_WEBHOOK_SECRET_FILE_BYTES).map_err(|_| {
        configuration_error(format!(
            "{environment_key} must reference a readable bounded UTF-8 secret file"
        ))
    })?;
    if secret.len() < 32
        || secret.len() > 1_024
        || !secret
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'_'))
    {
        return Err(configuration_error(format!(
            "{environment_key} contains an invalid secret"
        )));
    }
    Ok(Some(secret))
}

fn required_drive_internal_api_base_url() -> Result<String, sqlx::Error> {
    let value = std::env::var(DRIVE_INTERNAL_API_BASE_URL_ENV).map_err(|_| {
        configuration_error(format!(
            "{DRIVE_INTERNAL_API_BASE_URL_ENV} is required for cloud deployment"
        ))
    })?;
    validate_drive_internal_api_base_url(
        &value,
        sdkwork_knowledgebase_observability::is_production_like_environment(),
    )
}

fn validate_drive_internal_api_base_url(
    value: &str,
    production_like: bool,
) -> Result<String, sqlx::Error> {
    let value = value.trim();
    let parsed = url::Url::parse(value).map_err(|_| {
        configuration_error(format!(
            "{DRIVE_INTERNAL_API_BASE_URL_ENV} must be an absolute HTTP(S) URL"
        ))
    })?;
    if !matches!(parsed.scheme(), "http" | "https")
        || parsed.host_str().is_none()
        || !parsed.username().is_empty()
        || parsed.password().is_some()
        || parsed.query().is_some()
        || parsed.fragment().is_some()
        || parsed.path() != "/"
    {
        return Err(configuration_error(format!(
            "{DRIVE_INTERNAL_API_BASE_URL_ENV} must be an absolute credential-free HTTP(S) URL"
        )));
    }
    if production_like && parsed.scheme() != "https" {
        return Err(configuration_error(format!(
            "{DRIVE_INTERNAL_API_BASE_URL_ENV} must use HTTPS in production-like environments"
        )));
    }
    Ok(value.trim_end_matches('/').to_string())
}

fn read_drive_internal_api_ingress_token() -> Result<String, sqlx::Error> {
    let path = std::env::var(DRIVE_INTERNAL_API_INGRESS_TOKEN_FILE_ENV).map_err(|_| {
        configuration_error(format!(
            "{DRIVE_INTERNAL_API_INGRESS_TOKEN_FILE_ENV} is required for cloud deployment"
        ))
    })?;
    read_drive_internal_api_ingress_token_file(PathBuf::from(path.trim()))
}

fn read_drive_internal_api_ingress_token_file(path: PathBuf) -> Result<String, sqlx::Error> {
    if path.as_os_str().is_empty() {
        return Err(configuration_error(format!(
            "{DRIVE_INTERNAL_API_INGRESS_TOKEN_FILE_ENV} must not be blank"
        )));
    }
    let token = read_bounded_utf8_file(&path, MAX_INGRESS_TOKEN_FILE_BYTES).map_err(|_| {
        configuration_error(format!(
            "{DRIVE_INTERNAL_API_INGRESS_TOKEN_FILE_ENV} must reference a readable bounded UTF-8 secret file"
        ))
    })?;
    if token.len() < 16 || token.len() > 4_096 || token.chars().any(char::is_whitespace) {
        return Err(configuration_error(format!(
            "{DRIVE_INTERNAL_API_INGRESS_TOKEN_FILE_ENV} contains an invalid ingress token"
        )));
    }
    Ok(token.to_string())
}

fn read_bounded_utf8_file(path: &Path, max_bytes: u64) -> std::io::Result<String> {
    let file = File::open(path)?;
    let metadata = file.metadata()?;
    if !metadata.is_file() || metadata.len() == 0 || metadata.len() > max_bytes {
        return Err(std::io::Error::new(
            std::io::ErrorKind::InvalidData,
            "file size is outside the configured bound",
        ));
    }

    let mut value = String::with_capacity(metadata.len() as usize);
    file.take(max_bytes.saturating_add(1))
        .read_to_string(&mut value)?;
    if value.len() as u64 > max_bytes {
        return Err(std::io::Error::new(
            std::io::ErrorKind::InvalidData,
            "file grew beyond the configured bound while reading",
        ));
    }
    Ok(value)
}

fn default_provider_credential_resolver(
) -> Result<Arc<dyn KnowledgeEngineProviderCredentialResolver>, sqlx::Error> {
    let environment = match sdkwork_knowledgebase_observability::knowledgebase_environment() {
        Some(value) => KnowledgebaseProviderCredentialEnvironment::parse(&value)
            .map_err(|error| configuration_error(error.to_string()))?,
        None if cfg!(debug_assertions) => KnowledgebaseProviderCredentialEnvironment::Development,
        None => {
            return Err(configuration_error(
                "SDKWORK_KNOWLEDGEBASE_ENVIRONMENT is required for Provider credential policy",
            ));
        }
    };
    let local_secret_root = std::env::var(PROVIDER_SECRETS_DIR_ENV)
        .ok()
        .map(|value| PathBuf::from(value.trim()))
        .filter(|path| !path.as_os_str().is_empty());
    local_provider_credential_resolver(environment, local_secret_root)
}

fn local_provider_credential_resolver(
    environment: KnowledgebaseProviderCredentialEnvironment,
    local_secret_root: Option<PathBuf>,
) -> Result<Arc<dyn KnowledgeEngineProviderCredentialResolver>, sqlx::Error> {
    if environment.requires_managed_source() {
        return Err(configuration_error(
            "staging and production require an injected managed Knowledgebase Provider credential resolver",
        ));
    }
    let config =
        KnowledgebaseProviderCredentialResolverConfig::local(environment, local_secret_root)
            .map_err(|error| configuration_error(error.to_string()))?;
    let resolver = KnowledgebaseProviderCredentialResolver::local(config)
        .map_err(|error| configuration_error(error.to_string()))?;
    Ok(Arc::new(resolver))
}

#[cfg(test)]
mod provider_credential_resolver_tests {
    use super::*;

    #[test]
    fn production_default_requires_injected_managed_resolver() {
        let error = match local_provider_credential_resolver(
            KnowledgebaseProviderCredentialEnvironment::Production,
            None,
        ) {
            Err(error) => error,
            Ok(_) => panic!("production default must fail closed"),
        };

        assert!(error
            .to_string()
            .contains("require an injected managed Knowledgebase Provider credential resolver"));
    }
}

#[cfg(test)]
mod wiki_drive_runtime_config_tests {
    use super::*;
    use std::sync::atomic::{AtomicU64, Ordering};

    #[test]
    fn drive_internal_api_url_is_an_origin_and_requires_tls_in_production() {
        assert_eq!(
            validate_drive_internal_api_base_url("http://127.0.0.1:18080/", false).unwrap(),
            "http://127.0.0.1:18080"
        );
        assert!(validate_drive_internal_api_base_url("http://drive.internal", true).is_err());
        assert!(
            validate_drive_internal_api_base_url("https://user:secret@drive.internal", true)
                .is_err()
        );
        assert!(validate_drive_internal_api_base_url(
            "https://drive.internal/internal/v3/api",
            true
        )
        .is_err());
        assert!(
            validate_drive_internal_api_base_url("https://drive.internal?token=secret", true)
                .is_err()
        );
        assert!(
            validate_drive_internal_api_base_url("https://drive.internal#fragment", true).is_err()
        );
    }

    #[test]
    fn drive_internal_api_token_file_is_utf8_bounded_and_whitespace_free() {
        static COUNTER: AtomicU64 = AtomicU64::new(0);
        let root = std::env::temp_dir().join(format!(
            "sdkwork-kb-drive-token-{}-{}",
            std::process::id(),
            COUNTER.fetch_add(1, Ordering::Relaxed)
        ));
        std::fs::create_dir_all(&root).unwrap();
        let token_path = root.join("token");
        std::fs::write(&token_path, "test-drive-internal-key").unwrap();
        assert_eq!(
            read_drive_internal_api_ingress_token_file(token_path.clone()).unwrap(),
            "test-drive-internal-key"
        );

        for invalid in [
            "",
            "short",
            "test-drive-internal-key\n",
            "token with spaces",
        ] {
            std::fs::write(&token_path, invalid).unwrap();
            assert!(read_drive_internal_api_ingress_token_file(token_path.clone()).is_err());
        }
        std::fs::write(
            &token_path,
            vec![b'x'; MAX_INGRESS_TOKEN_FILE_BYTES as usize + 1],
        )
        .unwrap();
        assert!(read_drive_internal_api_ingress_token_file(token_path).is_err());
        std::fs::remove_dir_all(root).unwrap();
    }
}

impl FrameworkReadinessCheck for KnowledgebaseRuntimeReadinessCheck {
    fn check(&self) -> ReadinessFuture<'_> {
        let runtime = self.runtime.clone();
        Box::pin(async move {
            match runtime.readiness_check().await {
                Ok(()) => {
                    sdkwork_knowledgebase_observability::set_readiness_status(true);
                    Ok(())
                }
                Err(_) => {
                    sdkwork_knowledgebase_observability::set_readiness_status(false);
                    Err("knowledgebase runtime readiness check failed".to_string())
                }
            }
        })
    }
}

fn outbox_max_retries() -> u32 {
    std::env::var("SDKWORK_KNOWLEDGEBASE_OUTBOX_MAX_RETRIES")
        .ok()
        .and_then(|value| value.parse::<u32>().ok())
        .unwrap_or(5)
        .clamp(1, 32)
}

impl KnowledgebaseRuntime {
    pub async fn connect(database_url: &str, tenant_id: u64) -> Result<Self, sqlx::Error> {
        let provider_credential_resolver = default_provider_credential_resolver()?;
        Self::connect_with_provider_credential_resolver(
            database_url,
            tenant_id,
            provider_credential_resolver,
        )
        .await
    }

    pub async fn connect_with_provider_credential_resolver(
        database_url: &str,
        tenant_id: u64,
        provider_credential_resolver: Arc<dyn KnowledgeEngineProviderCredentialResolver>,
    ) -> Result<Self, sqlx::Error> {
        let deployment_tenant_id = require_postgres_rls_tenant_id()
            .map_err(|error| sqlx::Error::Configuration(error.to_string().into()))?;
        if tenant_id != deployment_tenant_id {
            return Err(configuration_error(format!(
                "runtime tenant_id {tenant_id} does not match fixed PostgreSQL RLS tenant_id {deployment_tenant_id}"
            )));
        }
        let organization_id = require_postgres_rls_organization_id()
            .map_err(|error| sqlx::Error::Configuration(error.to_string().into()))?;
        let pool = connect_knowledgebase_and_install_schema(database_url).await?;
        let snowflake_node_lease = initialize_runtime_id_generator(database_url).await?;
        let database_config = database_config_from_url(database_url)
            .map_err(|error| sqlx::Error::Configuration(error.to_string().into()))?;
        let pool_budget = knowledgebase_process_pool_budget_from_url(database_url)
            .map_err(|error| sqlx::Error::Configuration(error.to_string().into()))?;
        let postgres_max_connections = pool_budget.postgres_max_connections.ok_or_else(|| {
            configuration_error("server Knowledgebase runtime requires PostgreSQL")
        })?;
        let drive_pool = connect_knowledgebase_drive_pool_with_max_connections(
            &database_config.url,
            postgres_max_connections,
        )
        .await?;
        let pg_pool = Some(drive_pool.clone());
        let keyword_backend = keyword_search_backend_for_database_url(database_url);
        let database_engine = sdkwork_database_config::DatabaseEngine::Postgres;
        Self::from_pools(
            pool,
            drive_pool,
            tenant_id,
            organization_id,
            default_operator_id(),
            default_drive_storage_root(),
            keyword_backend,
            pg_pool,
            database_engine,
            snowflake_node_lease,
            provider_credential_resolver,
        )
        .await
    }

    #[allow(clippy::too_many_arguments)]
    async fn from_pools(
        pool: AnyPool,
        drive_pool: PgPool,
        tenant_id: u64,
        organization_id: u64,
        operator_id: String,
        drive_storage_root: PathBuf,
        keyword_backend: sdkwork_intelligence_knowledgebase_repository_sqlx::KeywordSearchBackend,
        pg_pool: Option<sqlx::PgPool>,
        database_engine: sdkwork_database_config::DatabaseEngine,
        snowflake_node_lease: Option<NodeLease>,
        provider_credential_resolver: Arc<dyn KnowledgeEngineProviderCredentialResolver>,
    ) -> Result<Self, sqlx::Error> {
        let tenant_id_str = tenant_id.to_string();
        let deployment_profile = KnowledgebaseDeploymentProfile::resolve()?;
        let wiki_event_config = resolve_wiki_drive_event_config(deployment_profile)?;
        let wiki_provider_caller_app_id =
            resolve_internal_caller_app_id(WIKI_PROVIDER_CALLER_APP_ID_ENV, "sdkwork-web")?;
        let quota_limits =
            sdkwork_knowledgebase_observability::KnowledgebaseTenantQuotaLimits::from_env();
        let drive_storage = Arc::new(match deployment_profile {
            KnowledgebaseDeploymentProfile::Standalone => {
                let object_store = Arc::new(LocalDriveObjectStore::new(drive_storage_root));
                let adapter = KnowledgebaseDriveStorageAdapter::new(
                    object_store,
                    DEFAULT_DRIVE_PROVIDER_ID,
                    DEFAULT_DRIVE_BUCKET,
                    tenant_id_str.clone(),
                );
                adapter.ensure_bucket().await.map_err(|error| {
                    configuration_error(format!(
                        "standalone Knowledgebase storage bucket initialization failed: {error}"
                    ))
                })?;
                adapter
            }
            KnowledgebaseDeploymentProfile::Cloud => {
                let provider_id = required_cloud_drive_storage_provider_id()?;
                let provider_pool = pg_pool.clone().ok_or_else(|| {
                    configuration_error(
                        "cloud Knowledgebase storage requires the authoritative PostgreSQL pool",
                    )
                })?;
                resolve_cloud_knowledgebase_drive_storage(
                    provider_pool,
                    &provider_id,
                    &tenant_id_str,
                )
                .await
                .map_err(|error| configuration_error(error.to_string()))?
            }
        });
        let drive_space_provisioner = Arc::new(KnowledgebaseDriveSpaceProvisionerAdapter::new(
            drive_pool.clone(),
        ));
        let drive_tree = Arc::new(KnowledgebaseDriveNodeTreeAdapter::new(
            drive_pool.clone(),
            tenant_id_str.clone(),
        ));
        let drive_workspace = Arc::new(KnowledgebaseDriveWorkspaceAdapter::new(
            drive_pool.clone(),
            tenant_id_str.clone(),
            operator_id.clone(),
        ));
        let (wiki_drive_scope, wiki_drive_source) = build_wiki_drive_ports(
            drive_pool.clone(),
            &tenant_id_str,
            &operator_id,
            deployment_profile,
            wiki_event_config.delivery.clone(),
        )?;
        let access_control = Arc::new(KnowledgebaseKnowledgeAccessControlAdapter::new(
            drive_pool.clone(),
        ));

        let retrieval_store = Arc::new(
            PostgresKnowledgeChunkRetrievalStore::with_keyword_backend(
                pool.clone(),
                tenant_id,
                organization_id,
                keyword_backend,
                default_knowledge_id_generator(),
            )
            .with_database_engine(database_engine),
        );

        let base_retrieval: SharedKnowledgeRetrievalBackend = if let Some(pg_pool) = pg_pool {
            Arc::new(PgVectorLayeredRetrievalBackend::new(
                retrieval_store.clone(),
                Arc::new(PgVectorKnowledgeRetrievalBackend::new(
                    pg_pool,
                    tenant_id,
                    organization_id,
                )),
            ))
        } else {
            retrieval_store.clone()
        };

        let retrieval_backend: SharedKnowledgeRetrievalBackend =
            resolve_cloud_router_client_from_env()
                .ok()
                .map(|client| {
                    Arc::new(
                        sdkwork_intelligence_knowledgebase_service::embedding_retrieval_backend::CloudRouterEmbeddingRetrievalBackend::new(
                            base_retrieval.clone(),
                            CloudRouterEmbeddingClient::new(Arc::new(client)),
                        ),
                    ) as SharedKnowledgeRetrievalBackend
                })
                .unwrap_or(base_retrieval);

        let okf_concept_store = Arc::new(
            PostgresKnowledgeOkfConceptStore::new(pool.clone(), tenant_id, organization_id)
                .with_database_engine(database_engine),
        );
        let space_store = Arc::new(
            PostgresKnowledgeSpaceStore::new(pool.clone(), tenant_id, organization_id)
                .with_database_engine(database_engine),
        );
        let wiki_store = Arc::new(
            SqlxWikiPersistenceStore::new(pool.clone()).with_database_engine(database_engine),
        );
        let embedded_wiki_drive_event_relay =
            (deployment_profile == KnowledgebaseDeploymentProfile::Standalone).then(|| {
                Arc::new(KnowledgebaseDriveEmbeddedEventRelay::new(
                    wiki_store.clone(),
                    wiki_store.clone(),
                    wiki_store.clone(),
                    wiki_drive_source.clone(),
                ))
            });
        let provider_binding_store = Arc::new(
            SqlxKnowledgeEngineProviderBindingStore::new(pool.clone())
                .with_database_engine(database_engine),
        );
        let provider_migration_store = Arc::new(
            SqlxKnowledgeEngineProviderMigrationStore::new(pool.clone())
                .with_database_engine(database_engine),
        );
        let okf_bundle_file_store = Arc::new(
            PostgresKnowledgeOkfBundleFileStore::new(pool.clone(), tenant_id, organization_id)
                .with_database_engine(database_engine),
        );
        let okf_concept_link_store = Arc::new(
            PostgresKnowledgeOkfConceptLinkStore::new(pool.clone(), tenant_id, organization_id)
                .with_database_engine(database_engine),
        );
        let okf_candidate_store = Arc::new(
            PostgresKnowledgeOkfCandidateStore::new(pool.clone(), tenant_id, organization_id)
                .with_database_engine(database_engine),
        );
        let okf_revision_metadata_store = Arc::new(
            PostgresOkfConceptRevisionMetadataStore::new(pool.clone(), tenant_id, organization_id)
                .with_database_engine(database_engine)
                .with_quota_limits(quota_limits),
        );
        let source_store = Arc::new(
            PostgresKnowledgeSourceStore::new(pool.clone(), tenant_id, organization_id)
                .with_database_engine(database_engine),
        );
        let object_ref_store = Arc::new(
            PostgresKnowledgeDriveObjectRefStore::new(pool.clone(), tenant_id, organization_id)
                .with_database_engine(database_engine)
                .with_quota_limits(quota_limits),
        );
        let document_store = Arc::new(
            PostgresKnowledgeDocumentStore::new(pool.clone(), tenant_id, organization_id)
                .with_database_engine(database_engine)
                .with_quota_limits(quota_limits),
        );
        let index_store = Arc::new(
            PostgresKnowledgeIndexStore::new(pool.clone(), tenant_id, organization_id)
                .with_database_engine(database_engine),
        );
        let embedding_store = Arc::new(PostgresKnowledgeEmbeddingStore::new(
            pool.clone(),
            tenant_id,
            organization_id,
            database_engine,
        ));
        let rag_embedder = resolve_cloud_router_client_from_env()
            .ok()
            .map(|client| CloudRouterEmbeddingClient::new(Arc::new(client)));

        let knowledge_engines = Arc::new(
            build_default_registry(KnowledgeEngineRuntimeDeps {
                tenant_id,
                okf: KnowledgeEngineRuntimeDeps::okf_from_stores(
                    okf_concept_store.clone(),
                    drive_storage.clone(),
                    okf_revision_metadata_store.clone(),
                    object_ref_store.clone(),
                    okf_concept_link_store.clone(),
                    okf_candidate_store.clone(),
                    okf_bundle_file_store.clone(),
                    drive_workspace.clone(),
                    source_store.clone(),
                    space_store.clone(),
                ),
                rag_documents: document_store.clone(),
                retrieval_backend: retrieval_backend.clone(),
                retrieval_traces: retrieval_store.clone(),
                rag_index_store: Some(index_store.clone()),
                rag_embedding_store: Some(embedding_store.clone()),
                rag_embedder,
                external_engines:
                    crate::knowledge_engine_adapters::load_runtime_external_adapter_engines(),
            })
            .map_err(|error| {
                sqlx::Error::Configuration(format!(
                    "knowledge engine registry build failed: {error}"
                )
                .into())
            })?,
        );

        let audit_event_store = Arc::new(
            PostgresKnowledgeAuditEventStore::new(pool.clone(), tenant_id, organization_id)
                .with_database_engine(database_engine),
        );
        let audit_hook_store = audit_event_store.clone();
        sdkwork_knowledgebase_observability::install_audit_persistence(move |event| {
            let audit_hook_store = audit_hook_store.clone();
            async move {
                let trace_id =
                    sdkwork_knowledgebase_observability::request_correlation::current_request_id();
                audit_hook_store
                    .record(KnowledgeAuditEventRecord {
                        id: None,
                        uuid: None,
                        event_type: event.event_type,
                        actor_type: event.actor_type,
                        actor_id: event.actor_id,
                        resource_type: event.resource_type,
                        resource_id: event.resource_id,
                        result: event.result,
                        request_id: None,
                        trace_id,
                        payload: event.payload,
                        created_at: None,
                    })
                    .await
                    .map_err(|error| {
                        sdkwork_knowledgebase_observability::AuditPersistenceError::write_failed(
                            error.to_string(),
                        )
                    })
            }
        });

        Ok(Self {
            retrieval_store,
            retrieval_backend,
            retrieval_profile_store: Arc::new(PostgresKnowledgeRetrievalProfileStore::new(
                pool.clone(),
                tenant_id,
                organization_id,
            )
            .with_database_engine(database_engine)),
            index_store,
            embedding_store,
            agent_store: Arc::new(PostgresKnowledgeAgentProfileStore::new(
                pool.clone(),
                tenant_id,
                organization_id,
            )
            .with_database_engine(database_engine)),
            space_store,
            wiki_store,
            okf_bundle_file_store,
            okf_concept_store,
            okf_concept_link_store,
            okf_candidate_store,
            document_store,
            source_store,
            provider_binding_store,
            provider_migration_store,
            provider_credential_resolver,
            version_store: Arc::new(PostgresKnowledgeDocumentVersionStore::new(
                pool.clone(),
                tenant_id,
                organization_id,
            )
            .with_database_engine(database_engine)),
            object_ref_store,
            ingestion_job_store: Arc::new(PostgresIngestionJobStore::with_keyword_backend(
                pool.clone(),
                tenant_id,
                organization_id,
                keyword_backend,
                default_knowledge_id_generator(),
            )
            .with_database_engine(database_engine)
            .with_quota_limits(quota_limits)),
            drive_import_metadata_store: Arc::new(PostgresDriveImportMetadataStore::new(
                pool.clone(),
                tenant_id,
                organization_id,
            )
            .with_database_engine(database_engine)
            .with_quota_limits(quota_limits)),
            markdown_index_metadata_store: Arc::new(PostgresMarkdownIndexMetadataStore::new(
                pool.clone(),
                tenant_id,
                organization_id,
            )
            .with_database_engine(database_engine)
            .with_quota_limits(quota_limits)),
            outbox_store: Arc::new(
                PostgresKnowledgeOutboxStore::new(
                    pool.clone(),
                    tenant_id,
                    organization_id,
                    default_outbox_claim_owner(),
                )
                    .with_database_engine(database_engine)
                    .with_postgres_skip_locked_claim(
                        database_engine == sdkwork_database_config::DatabaseEngine::Postgres,
                    ),
            ),
            outbox_dispatcher:
                sdkwork_intelligence_knowledgebase_service::outbox::knowledge_outbox_dispatcher_from_env(),
            chunk_store: Arc::new(
                PostgresKnowledgeChunkStore::with_keyword_backend(
                    pool.clone(),
                    tenant_id,
                    organization_id,
                    keyword_backend,
                    default_knowledge_id_generator(),
                )
                .with_database_engine(database_engine),
            ),
            context_binding_store: Arc::new(
                PostgresContextBindingStore::new(pool.clone(), organization_id)
                    .with_database_engine(database_engine),
            ),
            group_space_binding_store: Arc::new(
                PostgresGroupKnowledgeSpaceBindingStore::new(pool.clone())
                    .with_database_engine(database_engine),
            ),
            group_launch_ticket_consumer: None,
            browser_projection_store: Arc::new(PostgresKnowledgeBrowserProjectionStore::new(
                pool.clone(),
                tenant_id,
                organization_id,
            )),
            audit_event_store,
            pool: pool.clone(),
            drive_pool,
            tenant_id,
            organization_id,
            tenant_id_str,
            operator_id,
            drive_storage,
            drive_space_provisioner,
            drive_tree,
            drive_workspace,
            wiki_drive_scope,
            wiki_drive_source,
            embedded_wiki_drive_event_relay,
            wiki_webhook_signing_secrets: Arc::new(wiki_event_config.signing_secrets),
            wiki_event_caller_app_id: Arc::from(wiki_event_config.expected_caller_app_id),
            wiki_provider_caller_app_id: Arc::from(wiki_provider_caller_app_id),
            access_control,
            knowledge_engines,
            commerce_store: Arc::new(
                PostgresCommerceStore::new(pool.clone(), organization_id)
                    .with_database_engine(database_engine),
            ),
            snowflake_node_lease,
        })
    }

    pub fn pool(&self) -> &AnyPool {
        &self.pool
    }

    pub fn tenant_id(&self) -> u64 {
        self.tenant_id
    }

    /// Production composition injects the generated IM internal RPC adapter here. A runtime
    /// without it remains fail-closed for group-launch ticket consumption.
    pub fn with_group_launch_ticket_consumer(
        mut self,
        consumer: Arc<dyn GroupLaunchTicketConsumer>,
    ) -> Self {
        self.group_launch_ticket_consumer = Some(consumer);
        self
    }

    pub fn organization_id(&self) -> u64 {
        self.organization_id
    }

    pub fn group_launch_capability(
        &self,
    ) -> sdkwork_knowledgebase_contract::group_space::GroupKnowledgebaseLaunchCapability {
        use sdkwork_knowledgebase_contract::group_space::{
            GroupKnowledgebaseLaunchCapability, GroupKnowledgebaseLaunchCapabilityState,
        };

        GroupKnowledgebaseLaunchCapability {
            state: if self.group_launch_ticket_consumer.is_some() {
                GroupKnowledgebaseLaunchCapabilityState::Configured
            } else {
                GroupKnowledgebaseLaunchCapabilityState::Disabled
            },
        }
    }

    pub(crate) fn commerce_store(&self) -> &PostgresCommerceStore {
        &self.commerce_store
    }

    pub(crate) fn retrieval_service(&self) -> KnowledgeRetrievalService<'_> {
        KnowledgeRetrievalService::new(
            self.retrieval_backend.as_ref(),
            self.retrieval_store.as_ref(),
        )
    }

    pub async fn readiness_check(&self) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        if self
            .snowflake_node_lease
            .as_ref()
            .is_some_and(|lease| !lease.is_healthy())
        {
            return Err(Box::new(std::io::Error::other(
                "snowflake node lease is unhealthy",
            )));
        }
        knowledgebase_health_check(&self.pool)
            .await
            .map_err(|error| Box::new(error) as Box<dyn std::error::Error + Send + Sync>)?;
        knowledgebase_drive_health_check(&self.drive_pool)
            .await
            .map_err(|error| Box::new(error) as Box<dyn std::error::Error + Send + Sync>)?;
        let object_store_readiness = tokio::time::timeout(
            std::time::Duration::from_secs(5),
            self.drive_storage.readiness_check(),
        )
        .await
        .map_err(|_| {
            Box::new(std::io::Error::other(
                "knowledge storage readiness check timed out",
            )) as Box<dyn std::error::Error + Send + Sync>
        })?;
        object_store_readiness
            .map_err(|error| Box::new(error) as Box<dyn std::error::Error + Send + Sync>)?;
        Ok(())
    }

    pub fn readiness_check_adapter(
        &self,
    ) -> sdkwork_routes_knowledgebase_backend_api::KnowledgebaseReadinessCheck {
        Arc::new(KnowledgebaseRuntimeReadinessCheck::new(self.clone()))
    }

    pub fn build_agent_and_retrieval_router(&self) -> axum::Router {
        build_router_with_shared_app_api_and_readiness(
            Arc::new(AgentAndRetrievalHostedApi::new(self.clone())),
            Some(self.readiness_check_adapter()),
        )
    }

    pub fn build_full_app_router(&self) -> axum::Router {
        use crate::adapters::FullAppApi;

        build_router_with_shared_app_api_and_readiness(
            Arc::new(FullAppApi::new(
                Arc::new(HostedSpaceService::new(self.clone())),
                Arc::new(HostedWikiPublicationService::new(self.clone())),
                Arc::new(HostedGroupLaunchService::new(self.clone())),
                Arc::new(HostedDriveImportService::new(self.clone())),
                Arc::new(HostedGitImportService::new(self.clone())),
                Arc::new(HostedIngestService::new(self.clone())),
                Arc::new(HostedDocumentService::new(self.clone())),
                Arc::new(HostedOkfService::new(self.clone())),
                Arc::new(HostedBrowserService::new(self.clone())),
                Arc::new(HostedRetrievalService::new(self.clone())),
                Arc::new(HostedAgentService::new(self.clone())),
                Arc::new(HostedContextBindingService::new(self.clone())),
                Arc::new(HostedWechatService::new(self.clone())),
                Arc::new(HostedCommerceService::new(self.clone())),
            )),
            Some(self.readiness_check_adapter()),
        )
    }

    pub fn build_backend_business_router(&self) -> axum::Router {
        sdkwork_routes_knowledgebase_backend_api::build_business_router_with_shared_backend_api(
            Arc::new(HostedBackendApi::new(self.clone())),
            self.tenant_id(),
        )
    }

    pub fn build_open_business_router(&self) -> axum::Router {
        sdkwork_routes_knowledgebase_open_api::build_business_router_with_shared_open_api(Arc::new(
            HostedOpenApi::new(self.clone()),
        ))
    }

    pub fn build_internal_business_router(&self) -> axum::Router {
        sdkwork_routes_knowledgebase_internal_api::build_business_router_with_services(
            Arc::new(KnowledgebaseRuntimeWikiDriveEventReceiver {
                runtime: self.clone(),
            }),
            Arc::new(KnowledgeWikiPublicProviderService::new(
                self.wiki_store.clone(),
                self.wiki_drive_source.clone(),
            )),
            self.wiki_event_caller_app_id.to_string(),
            self.wiki_provider_caller_app_id.to_string(),
        )
    }

    pub fn build_backend_router(&self) -> axum::Router {
        sdkwork_routes_knowledgebase_backend_api::build_router_with_shared_backend_api_and_readiness(
            Arc::new(HostedBackendApi::new(self.clone())),
            self.tenant_id(),
            Some(self.readiness_check_adapter()),
        )
    }

    pub fn build_open_api_router(&self) -> axum::Router {
        sdkwork_routes_knowledgebase_open_api::build_router_with_shared_open_api_and_readiness(
            Arc::new(HostedOpenApi::new(self.clone())),
            Some(self.readiness_check_adapter()),
        )
    }

    pub(crate) fn retrieval_store(&self) -> &PostgresKnowledgeChunkRetrievalStore {
        &self.retrieval_store
    }

    pub(crate) fn retrieval_profile_store(&self) -> &PostgresKnowledgeRetrievalProfileStore {
        &self.retrieval_profile_store
    }

    pub(crate) fn index_store(&self) -> &PostgresKnowledgeIndexStore {
        &self.index_store
    }

    pub(crate) fn embedding_store(&self) -> &PostgresKnowledgeEmbeddingStore {
        &self.embedding_store
    }

    pub(crate) fn okf_concept_store(&self) -> &PostgresKnowledgeOkfConceptStore {
        &self.okf_concept_store
    }

    pub(crate) fn okf_concept_link_store(&self) -> &PostgresKnowledgeOkfConceptLinkStore {
        &self.okf_concept_link_store
    }

    pub(crate) fn okf_candidate_store(&self) -> &PostgresKnowledgeOkfCandidateStore {
        &self.okf_candidate_store
    }

    pub(crate) fn space_store(&self) -> &PostgresKnowledgeSpaceStore {
        &self.space_store
    }

    pub(crate) fn document_store(&self) -> &PostgresKnowledgeDocumentStore {
        &self.document_store
    }

    pub(crate) fn ingestion_job_store(&self) -> &PostgresIngestionJobStore {
        &self.ingestion_job_store
    }

    pub(crate) fn audit_event_store(&self) -> &PostgresKnowledgeAuditEventStore {
        &self.audit_event_store
    }

    pub(crate) fn drive_import_metadata_store(&self) -> &PostgresDriveImportMetadataStore {
        &self.drive_import_metadata_store
    }

    pub(crate) fn markdown_index_metadata_store(&self) -> &PostgresMarkdownIndexMetadataStore {
        &self.markdown_index_metadata_store
    }

    pub(crate) fn outbox_store(&self) -> &PostgresKnowledgeOutboxStore {
        &self.outbox_store
    }

    pub(crate) fn outbox_dispatcher(
        &self,
    ) -> &dyn sdkwork_intelligence_knowledgebase_service::ports::knowledge_outbox_dispatcher::KnowledgeOutboxDispatcher
    {
        self.outbox_dispatcher.as_ref()
    }

    pub(crate) fn chunk_store(&self) -> &PostgresKnowledgeChunkStore {
        &self.chunk_store
    }

    pub(crate) fn context_binding_store(&self) -> &PostgresContextBindingStore {
        &self.context_binding_store
    }

    pub(crate) fn group_space_binding_store(&self) -> &PostgresGroupKnowledgeSpaceBindingStore {
        &self.group_space_binding_store
    }

    pub(crate) fn group_launch_ticket_consumer(&self) -> Option<&dyn GroupLaunchTicketConsumer> {
        self.group_launch_ticket_consumer.as_deref()
    }

    pub(crate) fn okf_bundle_file_store(&self) -> &PostgresKnowledgeOkfBundleFileStore {
        &self.okf_bundle_file_store
    }

    pub(crate) fn source_store(&self) -> &PostgresKnowledgeSourceStore {
        &self.source_store
    }

    pub(crate) fn knowledge_engine_registry(&self) -> &DefaultKnowledgeEngineRegistry {
        &self.knowledge_engines
    }

    pub(crate) fn knowledge_engines(&self) -> &DefaultKnowledgeEngineRegistry {
        &self.knowledge_engines
    }

    pub(crate) fn knowledge_engine_space_resolver(
        &self,
    ) -> KnowledgeEngineSpaceResolver<DefaultKnowledgeEngineRegistry> {
        KnowledgeEngineSpaceResolver::new(
            self.knowledge_engines.clone(),
            self.space_store.clone(),
            self.provider_binding_store.clone(),
            KnowledgeEngineProviderScope {
                tenant_id: self.tenant_id,
                organization_id: self.organization_id,
            },
            self.provider_credential_resolver.clone(),
        )
    }

    pub(crate) fn knowledge_engine_provider_binding_service(
        &self,
    ) -> sdkwork_intelligence_knowledgebase_service::provider_binding::KnowledgeEngineProviderBindingService<
        DefaultKnowledgeEngineRegistry,
    >{
        sdkwork_intelligence_knowledgebase_service::provider_binding::KnowledgeEngineProviderBindingService::new(
            self.provider_binding_store.clone(),
            self.knowledge_engines.clone(),
            self.provider_credential_resolver.clone(),
        )
    }

    pub(crate) fn knowledge_engine_provider_migration_service(
        &self,
    ) -> sdkwork_intelligence_knowledgebase_service::provider_migration::KnowledgeEngineProviderMigrationService<
        SqlxKnowledgeEngineProviderBindingStore,
        SqlxKnowledgeEngineProviderMigrationStore,
    >{
        sdkwork_intelligence_knowledgebase_service::provider_migration::KnowledgeEngineProviderMigrationService::new(
            self.provider_binding_store.clone(),
            self.provider_migration_store.clone(),
            KnowledgeEngineProviderScope {
                tenant_id: self.tenant_id,
                organization_id: self.organization_id,
            },
        )
    }

    pub(crate) fn knowledge_engine_execution_context(
        &self,
        request_context: &crate::KnowledgeAppRequestContext,
        allowed_space_ids: Vec<u64>,
    ) -> Result<
        sdkwork_knowledgebase_contract::provider_binding::KnowledgeEngineExecutionContext,
        crate::ApiError,
    > {
        use sdkwork_knowledgebase_contract::provider_binding::{
            KnowledgeEngineDataScope, KnowledgeEngineExecutionContext,
        };

        if request_context.tenant_id != self.tenant_id {
            return Err(crate::ApiError::forbidden(
                "knowledge_engine_tenant_scope_mismatch",
                "request tenant does not match the Knowledgebase runtime",
            ));
        }
        let organization_id = request_context
            .organization_id
            .unwrap_or(self.organization_id);
        if organization_id != self.organization_id {
            return Err(crate::ApiError::forbidden(
                "knowledge_engine_organization_scope_mismatch",
                "request organization does not match the Knowledgebase runtime",
            ));
        }
        let actor_id = request_context.actor_id.ok_or_else(|| {
            crate::ApiError::unauthorized(
                "knowledge_engine_actor_required",
                "authenticated actor is required for knowledge execution",
            )
        })?;
        if allowed_space_ids.contains(&0) {
            return Err(crate::ApiError::invalid_request(
                "knowledge_engine_space_scope_required",
                "allowed knowledge spaces must use valid identifiers",
            ));
        }
        let trace_id = request_context
            .trace_id
            .as_deref()
            .filter(|value| !is_blank(Some(value)))
            .unwrap_or(request_context.request_id.as_str())
            .trim()
            .to_string();
        if trace_id.is_empty() {
            return Err(crate::ApiError::invalid_request(
                "knowledge_engine_trace_required",
                "trace_id is required for knowledge execution",
            ));
        }
        let now_ms = sdkwork_utils_rust::to_unix_millis(sdkwork_utils_rust::now());
        let deadline_unix_ms = u64::try_from(now_ms)
            .ok()
            .and_then(|value| value.checked_add(30_000))
            .ok_or_else(|| {
                crate::ApiError::internal(
                    "knowledge_engine_deadline_failed",
                    "failed to create a bounded knowledge execution deadline",
                )
            })?;

        Ok(KnowledgeEngineExecutionContext {
            tenant_id: self.tenant_id,
            organization_id: self.organization_id,
            actor_id: actor_id.to_string(),
            permission_scope: vec!["knowledge.read".to_string()],
            data_scope: KnowledgeEngineDataScope {
                allowed_space_ids,
                allowed_source_ids: Vec::new(),
                allowed_document_ids: Vec::new(),
            },
            space_id: 0,
            binding_id: None,
            trace_id,
            deadline_unix_ms,
        })
    }

    pub async fn resolve_knowledge_engine_implementation_id_for_space(
        &self,
        space_id: u64,
    ) -> Result<String, String> {
        self.knowledge_engine_space_resolver()
            .resolve_for_space(space_id, None)
            .await
            .map(|engine| engine.descriptor().implementation_id)
            .map_err(|error| error.to_string())
    }

    pub async fn read_knowledge_engine_document_for_space(
        &self,
        context: &sdkwork_knowledgebase_contract::provider_binding::KnowledgeEngineExecutionContext,
        space_id: u64,
        document_id: &str,
    ) -> Result<sdkwork_knowledgebase_contract::knowledge_engine::KnowledgeEngineDocument, String>
    {
        use sdkwork_knowledgebase_contract::knowledge_engine::KnowledgeEngineReadRequest;

        let mut context = context.clone();
        context.space_id = space_id;
        context.binding_id = None;

        self.knowledge_engine_space_resolver()
            .resolve_for_space(space_id, None)
            .await
            .map_err(|error| error.to_string())?
            .read_document(
                &context,
                KnowledgeEngineReadRequest {
                    tenant_id: self.tenant_id,
                    space_id,
                    document_id: document_id.to_string(),
                },
            )
            .await
            .map_err(|error| error.to_string())
    }

    pub(crate) fn okf_bundle_engine_for_space(
        &self,
        space_id: u64,
        implementation_id: &str,
    ) -> Result<
        &sdkwork_intelligence_knowledgebase_service::knowledge_engine::OkfNativeKnowledgeEngine,
        crate::ApiError,
    > {
        use sdkwork_knowledgebase_contract::knowledge_engine::KnowledgeEngineId;

        if implementation_id != KnowledgeEngineId::OKF_NATIVE {
            return Err(crate::ApiError::invalid_request(
                "okf_bundle_engine_required",
                format!(
                    "space {space_id} resolves to {implementation_id}, expected {}",
                    KnowledgeEngineId::OKF_NATIVE
                ),
            ));
        }
        Ok(self.knowledge_engines().okf_native())
    }

    pub async fn resolve_okf_bundle_engine_for_space(
        &self,
        space_id: u64,
    ) -> Result<
        &sdkwork_intelligence_knowledgebase_service::knowledge_engine::OkfNativeKnowledgeEngine,
        crate::ApiError,
    > {
        let implementation_id = self
            .resolve_knowledge_engine_implementation_id_for_space(space_id)
            .await
            .map_err(|detail| crate::ApiError::internal("okf_engine_resolve_failed", detail))?;
        self.okf_bundle_engine_for_space(space_id, &implementation_id)
    }

    pub async fn ensure_bindings_support_rag_retrieval(
        &self,
        bindings: &[sdkwork_knowledgebase_contract::rag::KnowledgeRetrievalBinding],
    ) -> Result<(), crate::ApiError> {
        use sdkwork_knowledgebase_contract::rag::KnowledgeAgentKnowledgeMode;

        for binding in bindings {
            let space = self
                .get_space_for_authorized_operation(binding.space_id)
                .await?;
            if space.knowledge_mode != KnowledgeAgentKnowledgeMode::Rag {
                return Err(crate::ApiError::invalid_request(
                    "rag_retrieval_mode_required",
                    format!(
                        "space {} uses {:?} knowledge mode; use okf query/context-pack or external engine APIs instead of RAG retrievals",
                        binding.space_id, space.knowledge_mode
                    ),
                ));
            }
        }
        Ok(())
    }

    /// Internal helpers may use this only after an App API authorization check or from a trusted
    /// backend control operation. Generic `KnowledgeSpaceStore::get_space` intentionally hides
    /// group-managed spaces; this method is the narrow escape hatch for already-authorized group
    /// content work such as OKF export staging.
    pub(crate) async fn get_space_for_authorized_operation(
        &self,
        space_id: u64,
    ) -> Result<sdkwork_knowledgebase_contract::space::KnowledgeSpace, crate::ApiError> {
        match self.space_store().get_space(space_id).await {
            Ok(space) => Ok(space),
            Err(generic_error) => self
                .space_store()
                .get_group_managed_space(space_id)
                .await
                .map_err(|_| crate::ApiError::from(generic_error)),
        }
    }

    pub async fn search_knowledge_engine_for_space(
        &self,
        context: &sdkwork_knowledgebase_contract::provider_binding::KnowledgeEngineExecutionContext,
        space_id: u64,
        query: &str,
        top_k: u32,
    ) -> Result<sdkwork_knowledgebase_contract::knowledge_engine::KnowledgeEngineSearchResult, String>
    {
        use sdkwork_knowledgebase_contract::knowledge_engine::KnowledgeEngineSearchRequest;

        let mut context = context.clone();
        context.space_id = space_id;
        context.binding_id = None;

        self.knowledge_engine_space_resolver()
            .resolve_for_space(space_id, None)
            .await
            .map_err(|error| error.to_string())?
            .search(
                &context,
                KnowledgeEngineSearchRequest {
                    tenant_id: self.tenant_id,
                    space_id,
                    query: query.to_string(),
                    top_k,
                },
            )
            .await
            .map_err(|error| error.to_string())
    }

    pub(crate) fn object_ref_store(&self) -> &PostgresKnowledgeDriveObjectRefStore {
        &self.object_ref_store
    }

    pub(crate) fn version_store(&self) -> &PostgresKnowledgeDocumentVersionStore {
        &self.version_store
    }

    pub(crate) fn browser_projection_store(&self) -> &PostgresKnowledgeBrowserProjectionStore {
        &self.browser_projection_store
    }

    pub(crate) fn drive_storage(&self) -> &KnowledgebaseDriveStorageAdapter {
        &self.drive_storage
    }

    pub(crate) fn drive_space_provisioner(&self) -> &KnowledgebaseDriveSpaceProvisionerAdapter {
        &self.drive_space_provisioner
    }

    pub(crate) fn drive_tree(&self) -> &KnowledgebaseDriveNodeTreeAdapter {
        &self.drive_tree
    }

    pub(crate) fn drive_workspace(&self) -> &KnowledgebaseDriveWorkspaceAdapter {
        &self.drive_workspace
    }

    pub(crate) fn wiki_store(&self) -> &SqlxWikiPersistenceStore {
        &self.wiki_store
    }

    pub(crate) fn wiki_drive_scope(&self) -> &dyn KnowledgeWikiDriveScope {
        self.wiki_drive_scope.as_ref()
    }

    pub(crate) fn wiki_drive_source(&self) -> &dyn KnowledgeWikiDriveSource {
        self.wiki_drive_source.as_ref()
    }

    pub(crate) fn access_control(&self) -> &KnowledgebaseKnowledgeAccessControlAdapter {
        &self.access_control
    }

    pub(crate) fn tenant_id_str(&self) -> &str {
        &self.tenant_id_str
    }

    pub(crate) fn operator_id(&self) -> &str {
        &self.operator_id
    }

    pub(crate) async fn try_embed_document_version(
        &self,
        space_id: u64,
        document_version_id: u64,
    ) -> Option<usize> {
        use sdkwork_intelligence_knowledgebase_service::ingest::KnowledgePostIngestEmbeddingService;
        use sdkwork_knowledgebase_agent_provider::{
            resolve_cloud_router_client_from_env, CloudRouterEmbeddingClient,
        };

        // Failures must never be silent: an unindexed document version is a data-quality
        // incident for retrieval. Every failure below records a metric and an error log so
        // operators can reconcile `index_state: Pending` documents with a rebuild.
        let client = match resolve_cloud_router_client_from_env() {
            Ok(client) => client,
            Err(reason) => {
                tracing::warn!(
                    tenant_id = self.tenant_id,
                    space_id,
                    document_version_id,
                    error = %reason,
                    "post-ingest embedding skipped: cloud-router client unavailable"
                );
                return None;
            }
        };
        let index = match self
            .index_store()
            .get_or_create_active_vector_index(space_id, 0)
            .await
        {
            Ok(index) => index,
            Err(error) => {
                tracing::warn!(
                    tenant_id = self.tenant_id,
                    space_id,
                    document_version_id,
                    error = %error,
                    "post-ingest embedding failed: cannot resolve active vector index"
                );
                sdkwork_knowledgebase_observability::record_embedding_failed(
                    self.tenant_id,
                    space_id,
                    document_version_id,
                );
                return None;
            }
        };
        let embedder = CloudRouterEmbeddingClient::new(Arc::new(client));
        let service = KnowledgePostIngestEmbeddingService::new(
            self.chunk_store(),
            self.embedding_store(),
            embedder,
        );
        match service
            .embed_document_version(self.tenant_id, &index, document_version_id)
            .await
        {
            Ok(count) => Some(count),
            Err(error) => {
                tracing::warn!(
                    tenant_id = self.tenant_id,
                    space_id,
                    document_version_id,
                    error = %error,
                    "post-ingest embedding failed; document version remains unindexed until a rebuild"
                );
                sdkwork_knowledgebase_observability::record_embedding_failed(
                    self.tenant_id,
                    space_id,
                    document_version_id,
                );
                None
            }
        }
    }

    pub async fn publish_pending_outbox_events(
        &self,
        limit: u32,
    ) -> Result<sdkwork_intelligence_knowledgebase_service::outbox::OutboxPublishBatchResult, String>
    {
        use sdkwork_intelligence_knowledgebase_service::outbox::KnowledgeOutboxPublisherService;

        let requeue = self.requeue_failed_outbox_events(limit).await?;
        let mut result = KnowledgeOutboxPublisherService::new(
            self.tenant_id(),
            self.outbox_store(),
            self.outbox_dispatcher(),
        )
        .publish_pending(limit)
        .await
        .map_err(|error| error.to_string())?;
        result.requeued = requeue.requeued;
        result.dead_lettered = requeue.dead_lettered;
        if result.dead_lettered > 0 {
            tracing::error!(
                target: "sdkwork.knowledgebase.outbox",
                dead_lettered = result.dead_lettered,
                max_retries = outbox_max_retries(),
                "knowledgebase outbox events exhausted their retry budget and were dead-lettered; inspect the webhook endpoint"
            );
        }
        Ok(result)
    }

    pub async fn requeue_failed_outbox_events(
        &self,
        limit: u32,
    ) -> Result<
        sdkwork_intelligence_knowledgebase_service::ports::knowledge_outbox_store::OutboxRequeueResult,
        String,
    >{
        self.outbox_store()
            .requeue_failed_events(limit, outbox_max_retries())
            .await
            .map_err(|error| error.to_string())
    }

    pub async fn run_wiki_publication_backfill_page(
        &self,
        request: sdkwork_intelligence_knowledgebase_service::wiki_backfill::RunWikiPublicationBackfillRequest,
    ) -> Result<
        sdkwork_intelligence_knowledgebase_service::wiki_backfill::WikiPublicationBackfillPageResult,
        String,
    >{
        use sdkwork_intelligence_knowledgebase_service::{
            wiki_backfill::KnowledgeWikiBackfillService,
            wiki_initialization::KnowledgeWikiInitializationService,
        };

        let initializer = KnowledgeWikiInitializationService::new(
            self.wiki_store(),
            self.wiki_store(),
            self.wiki_drive_scope(),
        );
        KnowledgeWikiBackfillService::new(self.wiki_store(), self.wiki_store(), &initializer)
            .run_page(request)
            .await
            .map_err(|error| error.to_string())
    }

    pub async fn receive_trusted_wiki_drive_event(
        &self,
        request: sdkwork_intelligence_knowledgebase_service::wiki_event_consumer::ReceiveKnowledgeWikiDriveTrustedEventRequest,
    ) -> Result<
        sdkwork_intelligence_knowledgebase_service::ports::knowledge_wiki_persistence::WikiDriveEventReceipt,
        String,
    >{
        use sdkwork_intelligence_knowledgebase_service::wiki_event_consumer::KnowledgeWikiDriveEventConsumerService;

        self.ensure_wiki_runtime_scope(request.scope)?;
        KnowledgeWikiDriveEventConsumerService::new(
            self.wiki_store(),
            self.wiki_store(),
            self.wiki_store(),
            self.wiki_drive_source(),
        )
        .receive_trusted(request)
        .await
        .map_err(|error| error.to_string())
    }

    pub async fn relay_embedded_wiki_drive_outbox_events(
        &self,
    ) -> Result<DomainOutboxDispatchResult, String> {
        let Some(relay) = self.embedded_wiki_drive_event_relay.as_deref() else {
            return Ok(Default::default());
        };
        relay.relay_pending_outbox_events(&self.drive_pool).await
    }

    pub async fn process_wiki_drive_event_checkpoint_page(
        &self,
        request: sdkwork_intelligence_knowledgebase_service::wiki_event_consumer::ProcessKnowledgeWikiDriveCheckpointPageRequest,
    ) -> Result<
        sdkwork_intelligence_knowledgebase_service::wiki_event_consumer::KnowledgeWikiDriveCheckpointPageResult,
        String,
    >{
        use sdkwork_intelligence_knowledgebase_service::wiki_event_consumer::KnowledgeWikiDriveEventConsumerService;

        self.ensure_wiki_runtime_scope(request.scope)?;
        KnowledgeWikiDriveEventConsumerService::new(
            self.wiki_store(),
            self.wiki_store(),
            self.wiki_store(),
            self.wiki_drive_source(),
        )
        .process_checkpoint_page(request)
        .await
        .map_err(|error| error.to_string())
    }

    pub async fn process_wiki_source_checkpoint_page(
        &self,
        request: sdkwork_intelligence_knowledgebase_service::wiki_source_processor::ProcessKnowledgeWikiSourceCheckpointPageRequest,
    ) -> Result<
        sdkwork_intelligence_knowledgebase_service::wiki_source_processor::KnowledgeWikiSourceCheckpointPageResult,
        String,
    >{
        use sdkwork_intelligence_knowledgebase_service::wiki_source_processor::KnowledgeWikiSourceProcessorService;

        self.ensure_wiki_runtime_scope(request.scope)?;
        KnowledgeWikiSourceProcessorService::new(
            self.wiki_store(),
            self.wiki_store(),
            self.wiki_store(),
            self.wiki_store(),
            self.wiki_drive_source(),
        )
        .process_checkpoint_page(request)
        .await
        .map_err(|error| error.to_string())
    }

    pub async fn renew_wiki_drive_event_delivery_page(
        &self,
        request: sdkwork_intelligence_knowledgebase_service::wiki_event_delivery::RenewWikiDriveEventDeliveryPageRequest,
    ) -> Result<
        sdkwork_intelligence_knowledgebase_service::wiki_event_delivery::WikiDriveEventDeliveryRenewalPageResult,
        String,
    >{
        use sdkwork_intelligence_knowledgebase_service::wiki_event_delivery::KnowledgeWikiDriveEventDeliveryRenewalService;

        self.ensure_wiki_runtime_scope(request.scope)?;
        KnowledgeWikiDriveEventDeliveryRenewalService::new(
            self.wiki_store(),
            self.wiki_drive_scope(),
        )
        .renew_page(request)
        .await
        .map_err(|error| error.to_string())
    }

    fn ensure_wiki_runtime_scope(
        &self,
        scope: sdkwork_intelligence_knowledgebase_service::ports::knowledge_wiki_persistence::WikiPersistenceScope,
    ) -> Result<(), String> {
        if scope.tenant_id != self.tenant_id || scope.organization_id != self.organization_id {
            return Err("Wiki event scope does not match the bound runtime tenant".to_string());
        }
        Ok(())
    }

    pub async fn read_document_content_markdown(
        &self,
        document_id: u64,
    ) -> Result<sdkwork_knowledgebase_contract::document::KnowledgeDocumentContent, String> {
        use sdkwork_intelligence_knowledgebase_service::ports::knowledge_drive_storage::KnowledgeObjectRef;
        use sdkwork_knowledgebase_contract::document::KnowledgeDocumentContent;

        let latest = self
            .version_store()
            .get_latest_version_for_document(document_id)
            .await
            .map_err(|error| error.to_string())?
            .ok_or_else(|| "document has no versions".to_string())?;
        let object_ref = self
            .object_ref_store()
            .get_object_ref_by_id(latest.original_object_ref_id)
            .await
            .map_err(|error| error.to_string())?;
        let storage_ref = KnowledgeObjectRef {
            storage_provider_id: object_ref.drive_storage_provider_id.clone(),
            bucket: object_ref.drive_bucket.clone(),
            object_key: object_ref.drive_object_key.clone(),
            logical_path: object_ref
                .logical_path
                .clone()
                .unwrap_or_else(|| object_ref.drive_object_key.clone()),
            object_role: object_ref.object_role.clone(),
            content_type: object_ref
                .content_type
                .clone()
                .unwrap_or_else(|| "text/markdown; charset=utf-8".to_string()),
            size_bytes: object_ref.size_bytes,
            checksum_sha256_hex: object_ref.checksum_sha256_hex.clone(),
            etag: object_ref.drive_etag.clone(),
            version_id: object_ref.drive_object_version.clone(),
        };

        if let Ok(content) = self.drive_storage().get_object_text(&storage_ref).await {
            if !is_blank(Some(content.as_str())) {
                let content_source = "drive_object".to_string();
                return Ok(KnowledgeDocumentContent {
                    document_id,
                    content_markdown: content,
                    content_source: content_source.clone(),
                    content_version: build_document_content_version(
                        &latest,
                        object_ref.drive_etag.as_deref(),
                        &content_source,
                    ),
                });
            }
        }

        let chunks = self
            .chunk_store()
            .list_chunk_texts_for_document_version(latest.id)
            .await
            .map_err(|error| error.to_string())?;
        if chunks.is_empty() {
            return Err("document content is not available".to_string());
        }

        let content_source = "chunk_concat".to_string();
        Ok(KnowledgeDocumentContent {
            document_id,
            content_markdown: chunks.join("\n\n"),
            content_source: content_source.clone(),
            content_version: build_document_content_version(&latest, None, &content_source),
        })
    }

    pub async fn process_queued_ingestion_jobs(
        &self,
        worker_id: &str,
        lease_duration: time::Duration,
        limit: u32,
        max_attempts: u32,
    ) -> Result<usize, String> {
        use sdkwork_intelligence_knowledgebase_service::ingest::KnowledgeIngestionJobWorkerService;

        KnowledgeIngestionJobWorkerService::new(self.ingestion_job_store(), self.drive_storage())
            .process_queued_jobs(worker_id, lease_duration, limit, max_attempts)
            .await
            .map(|result| result.processed)
            .map_err(|error| error.to_string())
    }

    pub async fn process_provider_migrations(
        &self,
        worker_id: &str,
        lease_duration: std::time::Duration,
        limit: u32,
    ) -> Result<
        sdkwork_intelligence_knowledgebase_service::provider_migration::ProviderMigrationBatchResult,
        String,
    >{
        self.knowledge_engine_provider_migration_service()
            .process_batch(worker_id, lease_duration, limit)
            .await
            .map_err(|error| error.to_string())
    }

    /// Advances bounded, durable group archive sagas for every organization in this runtime's
    /// worker-authorized tenant. Each work item owns only one Drive ACL page or one physical
    /// archive transition, keeping a worker tick independent from the IM relay deadline and
    /// restart-safe from binding state.
    pub async fn process_resumable_group_space_archives(&self, limit: u32) -> usize {
        if self.tenant_id == 0 || limit == 0 {
            return 0;
        }
        let commands = match self
            .group_space_binding_store
            .list_resumable_group_space_archives_for_tenant(self.tenant_id, limit.min(200))
            .await
        {
            Ok(commands) => commands,
            Err(error) => {
                tracing::warn!(
                    target: "sdkwork.knowledgebase",
                    error = %error,
                    "failed to load resumable group archive work"
                );
                return 0;
            }
        };
        let file_registry = OkfBundleFileRegistryService::new(self.okf_bundle_file_store());
        let okf_initializer = OkfBundleInitializerService::new(self.drive_storage())
            .with_registry(&file_registry)
            .with_drive_workspace(self.drive_workspace());
        let service = KnowledgeGroupKnowledgeSpaceService::new(
            self.group_space_binding_store(),
            self.space_store(),
            &okf_initializer,
            self.drive_space_provisioner(),
            self.access_control(),
            self.operator_id.clone(),
            None,
        );
        let mut processed = 0usize;
        for command in commands {
            match service.resume_archiving_from_worker(command).await {
                Ok(_) => processed += 1,
                Err(error) => tracing::warn!(
                    target: "sdkwork.knowledgebase",
                    error = %error,
                    "resumable group archive step did not converge"
                ),
            }
        }
        processed
    }
}

fn build_document_content_version(
    version: &sdkwork_knowledgebase_contract::document::KnowledgeDocumentVersion,
    drive_etag: Option<&str>,
    content_source: &str,
) -> String {
    if let Some(etag) = drive_etag.filter(|value| !value.is_empty()) {
        return format!("{content_source}:etag:{etag}");
    }
    if let Some(checksum) = version
        .checksum_sha256_hex
        .as_deref()
        .filter(|value| !value.is_empty())
    {
        return format!("{content_source}:v{}:{checksum}", version.id);
    }
    format!("{content_source}:v{}:{}", version.id, version.version_no)
}

fn default_operator_id() -> String {
    std::env::var("SDKWORK_KNOWLEDGEBASE_OPERATOR_ID").unwrap_or_else(|_| "system".to_string())
}

fn default_outbox_claim_owner() -> String {
    std::env::var("SDKWORK_KNOWLEDGEBASE_WORKER_ID")
        .or_else(|_| std::env::var("SDKWORK_NODE_INSTANCE_ID"))
        .unwrap_or_else(|_| format!("knowledgebase-runtime-{}", std::process::id()))
}

fn default_drive_storage_root() -> PathBuf {
    std::env::var("SDKWORK_KNOWLEDGEBASE_DRIVE_STORAGE_ROOT")
        .map(PathBuf::from)
        .unwrap_or_else(|_| PathBuf::from("data/drive-objects"))
}

#[derive(Clone)]
pub(crate) struct HostedRetrievalService {
    runtime: KnowledgebaseRuntime,
}

impl HostedRetrievalService {
    pub(crate) fn new(runtime: KnowledgebaseRuntime) -> Self {
        Self { runtime }
    }

    fn service(&self) -> KnowledgeRetrievalService<'_> {
        self.runtime.retrieval_service()
    }

    async fn authorize_retrieval_request(
        &self,
        context: &KnowledgeAppRequestContext,
        request: &KnowledgeRetrievalRequest,
    ) -> ApiResult<()> {
        ensure_runtime_tenant(&self.runtime, context)?;
        require_bindings_space_access(&self.runtime, context, &request.bindings).await?;
        self.runtime
            .ensure_bindings_support_rag_retrieval(&request.bindings)
            .await
    }

    async fn authorize_retrieval_trace(
        &self,
        context: &KnowledgeAppRequestContext,
        retrieval_id: u64,
    ) -> ApiResult<()> {
        ensure_runtime_tenant(&self.runtime, context)?;
        let _actor_id = require_actor_id(context)?;

        let trace = self
            .runtime
            .retrieval_store()
            .retrieve_trace(context.tenant_id, retrieval_id)
            .await
            .map_err(|error| ApiError::from(KnowledgeRetrievalServiceError::TraceStore(error)))?;

        if trace.actor_id.is_none() {
            return Err(ApiError::new(
                axum::http::StatusCode::FORBIDDEN,
                "retrieval_trace_access_denied",
                "retrieval trace is not accessible without an owning actor",
            ));
        }
        if let Some(trace_actor_id) = trace.actor_id {
            if context.actor_id != Some(trace_actor_id) {
                return Err(ApiError::new(
                    axum::http::StatusCode::FORBIDDEN,
                    "retrieval_trace_access_denied",
                    "authenticated actor does not own this retrieval trace",
                ));
            }
        }

        let hits = self
            .runtime
            .retrieval_store()
            .list_trace_hits(context.tenant_id, retrieval_id)
            .await
            .map_err(|error| ApiError::from(KnowledgeRetrievalServiceError::TraceStore(error)))?;

        let mut seen = HashSet::new();
        for hit in hits {
            if seen.insert(hit.space_id) {
                require_space_access(&self.runtime, context, hit.space_id).await?;
            }
        }

        Ok(())
    }
}

#[async_trait]
impl KnowledgeRetrievalAppService for HostedRetrievalService {
    async fn retrieve(
        &self,
        context: KnowledgeAppRequestContext,
        mut request: KnowledgeRetrievalRequest,
    ) -> ApiResult<KnowledgeRetrievalResult> {
        self.authorize_retrieval_request(&context, &request).await?;
        crate::tenant_quota_enforcement::ensure_tenant_retrieval_rate(context.tenant_id).await?;
        request = request.with_actor_id(context.actor_id);
        self.service().retrieve(request).await.map_err(Into::into)
    }

    async fn retrieve_retrieval(
        &self,
        context: KnowledgeAppRequestContext,
        retrieval_id: u64,
    ) -> ApiResult<KnowledgeRetrievalResult> {
        self.authorize_retrieval_trace(&context, retrieval_id)
            .await?;
        self.service()
            .retrieve_persisted(context.tenant_id, retrieval_id)
            .await
            .map_err(Into::into)
    }

    async fn create_context_pack(
        &self,
        context: KnowledgeAppRequestContext,
        mut request: KnowledgeContextPackRequest,
    ) -> ApiResult<KnowledgeContextPack> {
        let retrieval_request = KnowledgeRetrievalRequest {
            tenant_id: request.tenant_id,
            actor_id: request.actor_id,
            query: request.query.clone(),
            retrieval_profile_id: request.retrieval_profile_id,
            bindings: request.bindings.clone(),
            methods: vec![],
            top_k: None,
            include_citations: request.include_citations,
            include_trace: true,
            context_budget_tokens: Some(request.context_budget_tokens),
            metadata: vec![],
        };
        self.authorize_retrieval_request(&context, &retrieval_request)
            .await?;
        crate::tenant_quota_enforcement::ensure_tenant_retrieval_rate(context.tenant_id).await?;
        request = request
            .with_tenant_id(context.tenant_id)
            .with_actor_id(context.actor_id);
        self.service()
            .create_context_pack(request)
            .await
            .map_err(Into::into)
    }
}

#[derive(Clone)]
struct HostedAgentService {
    runtime: KnowledgebaseRuntime,
}

impl HostedAgentService {
    fn new(runtime: KnowledgebaseRuntime) -> Self {
        Self { runtime }
    }
}

#[async_trait]
impl KnowledgeAgentAppService for HostedAgentService {
    async fn create_profile(
        &self,
        context: KnowledgeAppRequestContext,
        request: KnowledgeAgentProfileRequest,
    ) -> ApiResult<KnowledgeAgentProfile> {
        ensure_runtime_tenant(&self.runtime, &context)?;
        let retrieval = self.runtime.retrieval_service();
        let service = KnowledgeAgentService::new(self.runtime.agent_store.as_ref(), &retrieval);
        service
            .create_profile(request.with_tenant_id(context.tenant_id))
            .await
            .map_err(Into::into)
    }

    async fn retrieve_profile(
        &self,
        context: KnowledgeAppRequestContext,
        profile_id: u64,
    ) -> ApiResult<KnowledgeAgentProfile> {
        let retrieval = self.runtime.retrieval_service();
        let service = KnowledgeAgentService::new(self.runtime.agent_store.as_ref(), &retrieval);
        let profile = service
            .retrieve_profile(profile_id)
            .await
            .map_err(ApiError::from)?;
        require_agent_profile_space_access(&self.runtime, &context, &profile).await?;
        Ok(profile)
    }

    async fn update_profile(
        &self,
        context: KnowledgeAppRequestContext,
        profile_id: u64,
        request: KnowledgeAgentProfileRequest,
    ) -> ApiResult<KnowledgeAgentProfile> {
        let retrieval = self.runtime.retrieval_service();
        let service = KnowledgeAgentService::new(self.runtime.agent_store.as_ref(), &retrieval);
        let existing = service
            .retrieve_profile(profile_id)
            .await
            .map_err(ApiError::from)?;
        require_agent_profile_space_access_with_role(
            &self.runtime,
            &context,
            &existing,
            KnowledgeAccessRole::Writer,
        )
        .await?;
        service
            .update_profile(profile_id, request.with_tenant_id(context.tenant_id))
            .await
            .map_err(Into::into)
    }

    async fn delete_profile(
        &self,
        context: KnowledgeAppRequestContext,
        profile_id: u64,
    ) -> ApiResult<()> {
        let retrieval = self.runtime.retrieval_service();
        let service = KnowledgeAgentService::new(self.runtime.agent_store.as_ref(), &retrieval);
        let existing = service
            .retrieve_profile(profile_id)
            .await
            .map_err(ApiError::from)?;
        require_agent_profile_space_access_with_role(
            &self.runtime,
            &context,
            &existing,
            KnowledgeAccessRole::Writer,
        )
        .await?;
        service.delete_profile(profile_id).await.map_err(Into::into)
    }

    async fn list_bindings(
        &self,
        context: KnowledgeAppRequestContext,
        profile_id: u64,
    ) -> ApiResult<KnowledgeAgentBindingList> {
        let retrieval = self.runtime.retrieval_service();
        let service = KnowledgeAgentService::new(self.runtime.agent_store.as_ref(), &retrieval);
        let profile = service
            .retrieve_profile(profile_id)
            .await
            .map_err(ApiError::from)?;
        require_agent_profile_space_access(&self.runtime, &context, &profile).await?;
        service.list_bindings(profile_id).await.map_err(Into::into)
    }

    async fn create_binding(
        &self,
        context: KnowledgeAppRequestContext,
        profile_id: u64,
        request: KnowledgeAgentBindingRequest,
    ) -> ApiResult<KnowledgeAgentBinding> {
        let retrieval = self.runtime.retrieval_service();
        let service = KnowledgeAgentService::new(self.runtime.agent_store.as_ref(), &retrieval);
        let profile = service
            .retrieve_profile(profile_id)
            .await
            .map_err(ApiError::from)?;
        require_agent_profile_space_access_with_role(
            &self.runtime,
            &context,
            &profile,
            KnowledgeAccessRole::Writer,
        )
        .await?;
        require_agent_binding_space_access_with_role(
            &self.runtime,
            &context,
            request.space_id,
            KnowledgeAccessRole::Writer,
        )
        .await?;
        service
            .create_binding(profile_id, request.with_tenant_id(context.tenant_id))
            .await
            .map_err(Into::into)
    }

    async fn update_binding(
        &self,
        context: KnowledgeAppRequestContext,
        profile_id: u64,
        binding_id: u64,
        request: KnowledgeAgentBindingRequest,
    ) -> ApiResult<KnowledgeAgentBinding> {
        let retrieval = self.runtime.retrieval_service();
        let service = KnowledgeAgentService::new(self.runtime.agent_store.as_ref(), &retrieval);
        let profile = service
            .retrieve_profile(profile_id)
            .await
            .map_err(ApiError::from)?;
        require_agent_profile_space_access_with_role(
            &self.runtime,
            &context,
            &profile,
            KnowledgeAccessRole::Writer,
        )
        .await?;
        require_agent_binding_space_access_with_role(
            &self.runtime,
            &context,
            request.space_id,
            KnowledgeAccessRole::Writer,
        )
        .await?;
        service
            .update_binding(
                profile_id,
                binding_id,
                request.with_tenant_id(context.tenant_id),
            )
            .await
            .map_err(Into::into)
    }

    async fn delete_binding(
        &self,
        context: KnowledgeAppRequestContext,
        profile_id: u64,
        binding_id: u64,
    ) -> ApiResult<()> {
        let retrieval = self.runtime.retrieval_service();
        let service = KnowledgeAgentService::new(self.runtime.agent_store.as_ref(), &retrieval);
        let profile = service
            .retrieve_profile(profile_id)
            .await
            .map_err(ApiError::from)?;
        require_agent_profile_space_access_with_role(
            &self.runtime,
            &context,
            &profile,
            KnowledgeAccessRole::Writer,
        )
        .await?;
        service
            .delete_binding(profile_id, binding_id)
            .await
            .map_err(Into::into)
    }

    async fn preview_retrieval(
        &self,
        context: KnowledgeAppRequestContext,
        profile_id: u64,
        request: KnowledgeRetrievalRequest,
    ) -> ApiResult<KnowledgeRetrievalResult> {
        let retrieval = self.runtime.retrieval_service();
        let service = KnowledgeAgentService::new(self.runtime.agent_store.as_ref(), &retrieval);
        let profile = service
            .retrieve_profile(profile_id)
            .await
            .map_err(ApiError::from)?;
        require_agent_profile_space_access(&self.runtime, &context, &profile).await?;
        require_bindings_space_access(&self.runtime, &context, &request.bindings).await?;
        service
            .preview_retrieval(profile_id, request)
            .await
            .map_err(Into::into)
    }

    async fn create_agent_chat(
        &self,
        context: KnowledgeAppRequestContext,
        profile_id: u64,
        mut request: KnowledgeAgentChatRequest,
    ) -> ApiResult<KnowledgeAgentChatResponse> {
        let retrieval = self.runtime.retrieval_service();
        let service = KnowledgeAgentService::new(self.runtime.agent_store.as_ref(), &retrieval);
        let profile = service
            .retrieve_profile(profile_id)
            .await
            .map_err(ApiError::from)?;
        require_agent_profile_space_access(&self.runtime, &context, &profile).await?;

        let mut allowed_space_ids = profile
            .bindings
            .iter()
            .filter(|binding| binding.enabled)
            .map(|binding| binding.space_id)
            .collect::<Vec<_>>();
        allowed_space_ids.sort_unstable();
        allowed_space_ids.dedup();
        let execution_context = self
            .runtime
            .knowledge_engine_execution_context(&context, allowed_space_ids)?;
        request.tenant_id = context.tenant_id;
        request.actor_id = context.actor_id;

        let retrieval_client = RuntimeKnowledgebaseRetrievalClient::new(
            self.runtime.clone(),
            execution_context.clone(),
        );
        let okf_client =
            RuntimeOkfKnowledgeClient::new(self.runtime.clone(), execution_context.clone());
        let cloud_router_client = resolve_cloud_router_client_from_env()
            .ok()
            .map(std::sync::Arc::new);
        let retrieval = self.runtime.retrieval_service();
        let plan_resolver =
            RuntimeRetrievalPlanResolver::new(self.runtime.retrieval_profile_store.clone());
        let space_mode_resolver = RuntimeSpaceModeResolver::new(self.runtime.clone());
        let space_engine_client = Arc::new(RuntimeSpaceKnowledgeEngineClient::new(
            self.runtime.clone(),
            execution_context,
        ));
        let chat_service = KnowledgeAgentChatService::new(
            self.runtime.agent_store.as_ref(),
            &retrieval,
            retrieval_client,
            okf_client,
            cloud_router_client,
            Some(&plan_resolver),
            Some(&space_mode_resolver),
            Some(space_engine_client),
        );
        chat_service
            .chat(profile_id, request)
            .await
            .map_err(Into::into)
    }
}

#[derive(Clone)]
struct AgentAndRetrievalHostedApi {
    retrieval: HostedRetrievalService,
    agent: HostedAgentService,
}

impl AgentAndRetrievalHostedApi {
    fn new(runtime: KnowledgebaseRuntime) -> Self {
        Self {
            retrieval: HostedRetrievalService::new(runtime.clone()),
            agent: HostedAgentService::new(runtime),
        }
    }
}

#[async_trait::async_trait]
impl crate::KnowledgeAppApi for AgentAndRetrievalHostedApi {
    async fn create_retrieval(
        &self,
        context: KnowledgeAppRequestContext,
        request: KnowledgeRetrievalRequest,
    ) -> ApiResult<KnowledgeRetrievalResult> {
        self.retrieval.retrieve(context, request).await
    }

    async fn retrieve_retrieval(
        &self,
        context: KnowledgeAppRequestContext,
        retrieval_id: u64,
    ) -> ApiResult<KnowledgeRetrievalResult> {
        self.retrieval
            .retrieve_retrieval(context, retrieval_id)
            .await
    }

    async fn create_context_pack(
        &self,
        context: KnowledgeAppRequestContext,
        request: KnowledgeContextPackRequest,
    ) -> ApiResult<KnowledgeContextPack> {
        self.retrieval.create_context_pack(context, request).await
    }

    async fn create_agent_profile(
        &self,
        context: KnowledgeAppRequestContext,
        request: KnowledgeAgentProfileRequest,
    ) -> ApiResult<KnowledgeAgentProfile> {
        self.agent.create_profile(context, request).await
    }

    async fn retrieve_agent_profile(
        &self,
        context: KnowledgeAppRequestContext,
        profile_id: u64,
    ) -> ApiResult<KnowledgeAgentProfile> {
        self.agent.retrieve_profile(context, profile_id).await
    }

    async fn update_agent_profile(
        &self,
        context: KnowledgeAppRequestContext,
        profile_id: u64,
        request: KnowledgeAgentProfileRequest,
    ) -> ApiResult<KnowledgeAgentProfile> {
        self.agent
            .update_profile(context, profile_id, request)
            .await
    }

    async fn delete_agent_profile(
        &self,
        context: KnowledgeAppRequestContext,
        profile_id: u64,
    ) -> ApiResult<()> {
        self.agent.delete_profile(context, profile_id).await
    }

    async fn list_agent_profile_bindings(
        &self,
        context: KnowledgeAppRequestContext,
        profile_id: u64,
    ) -> ApiResult<KnowledgeAgentBindingList> {
        self.agent.list_bindings(context, profile_id).await
    }

    async fn create_agent_profile_binding(
        &self,
        context: KnowledgeAppRequestContext,
        profile_id: u64,
        request: KnowledgeAgentBindingRequest,
    ) -> ApiResult<KnowledgeAgentBinding> {
        self.agent
            .create_binding(context, profile_id, request)
            .await
    }

    async fn update_agent_profile_binding(
        &self,
        context: KnowledgeAppRequestContext,
        profile_id: u64,
        binding_id: u64,
        request: KnowledgeAgentBindingRequest,
    ) -> ApiResult<KnowledgeAgentBinding> {
        self.agent
            .update_binding(context, profile_id, binding_id, request)
            .await
    }

    async fn delete_agent_profile_binding(
        &self,
        context: KnowledgeAppRequestContext,
        profile_id: u64,
        binding_id: u64,
    ) -> ApiResult<()> {
        self.agent
            .delete_binding(context, profile_id, binding_id)
            .await
    }

    async fn create_agent_profile_retrieval_preview(
        &self,
        context: KnowledgeAppRequestContext,
        profile_id: u64,
        request: KnowledgeRetrievalRequest,
    ) -> ApiResult<KnowledgeRetrievalResult> {
        self.agent
            .preview_retrieval(context, profile_id, request)
            .await
    }

    async fn create_agent_chat(
        &self,
        context: KnowledgeAppRequestContext,
        profile_id: u64,
        request: KnowledgeAgentChatRequest,
    ) -> ApiResult<KnowledgeAgentChatResponse> {
        self.agent
            .create_agent_chat(context, profile_id, request)
            .await
    }
}
