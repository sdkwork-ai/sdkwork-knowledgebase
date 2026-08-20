use crate::{
    hosted_access::{require_numeric_actor_id, require_space_access_with_role},
    ApiError, ApiResult, KnowledgeAppRequestContext, KnowledgeWikiPublicationAppService,
    KnowledgebaseRuntime,
};
use async_trait::async_trait;
use sdkwork_intelligence_knowledgebase_service::{
    ports::{
        knowledge_access_control::KnowledgeAccessRole,
        knowledge_wiki_persistence::{
            WikiIndexState, WikiPagePublicationState, WikiPersistenceScope, WikiPublicationMode,
            WikiPublicationStatus, WikiPublicationStore, WikiSourceFileKind, WikiSourceProjection,
            WikiSourceState, WikiUpdatePolicy, WikiVisibility,
        },
        knowledge_wiki_publication_lifecycle::{
            ChangeWikiPageVisibilityRequest, ChangeWikiPublicationStatusRequest,
            PublishWikiPageRequest, UnpublishWikiPageRequest, WikiLifecycleAuditContext,
            WikiPublicationLifecycleAction,
        },
    },
    wiki_publication_lifecycle::{
        KnowledgeWikiPublicationLifecycleError, KnowledgeWikiPublicationLifecycleService,
    },
};
use sdkwork_knowledgebase_contract::{
    ChangeKnowledgeWikiSourceFileVisibilityRequest, KnowledgeWikiIndexState,
    KnowledgeWikiPagePublicationState, KnowledgeWikiPublication, KnowledgeWikiPublicationMode,
    KnowledgeWikiPublicationStatus, KnowledgeWikiPublicationVersionCommandRequest,
    KnowledgeWikiSourceFile, KnowledgeWikiSourceFileCommandResult, KnowledgeWikiSourceFileKind,
    KnowledgeWikiSourceFileVersionCommandRequest, KnowledgeWikiSourceState,
    KnowledgeWikiUpdatePolicy, KnowledgeWikiVisibility, PublishKnowledgeWikiSourceFileRequest,
};

#[derive(Clone)]
pub(crate) struct HostedWikiPublicationService {
    runtime: KnowledgebaseRuntime,
}

impl HostedWikiPublicationService {
    pub(crate) fn new(runtime: KnowledgebaseRuntime) -> Self {
        Self { runtime }
    }

    fn scope(&self, context: &KnowledgeAppRequestContext) -> WikiPersistenceScope {
        WikiPersistenceScope {
            tenant_id: context.tenant_id,
            organization_id: context.organization_id.unwrap_or(0),
        }
    }

    fn audit_context(context: &KnowledgeAppRequestContext) -> WikiLifecycleAuditContext {
        WikiLifecycleAuditContext {
            request_id: context.request_id.clone(),
            trace_id: context.trace_id.clone(),
        }
    }
}

#[async_trait]
impl KnowledgeWikiPublicationAppService for HostedWikiPublicationService {
    async fn retrieve_wiki_publication(
        &self,
        context: KnowledgeAppRequestContext,
        space_id: u64,
    ) -> ApiResult<KnowledgeWikiPublication> {
        require_space_access_with_role(
            &self.runtime,
            &context,
            space_id,
            KnowledgeAccessRole::Reader,
        )
        .await?;
        self.runtime
            .wiki_store()
            .get_publication_for_space(self.scope(&context), space_id)
            .await
            .map_err(map_persistence_error)?
            .map(map_publication)
            .ok_or_else(|| {
                ApiError::not_found(
                    "wiki_publication_not_found",
                    "Wiki publication was not found for the knowledge space",
                )
            })
    }

    async fn activate_wiki_publication(
        &self,
        context: KnowledgeAppRequestContext,
        space_id: u64,
        request: KnowledgeWikiPublicationVersionCommandRequest,
    ) -> ApiResult<KnowledgeWikiPublication> {
        require_space_access_with_role(
            &self.runtime,
            &context,
            space_id,
            KnowledgeAccessRole::Owner,
        )
        .await?;
        let actor_id = require_numeric_actor_id(&context)?;
        let result = KnowledgeWikiPublicationLifecycleService::new(self.runtime.wiki_store())
            .change_publication_status(ChangeWikiPublicationStatusRequest {
                scope: self.scope(&context),
                space_id,
                expected_version: request.expected_version,
                actor_id,
                action: WikiPublicationLifecycleAction::Activate,
                audit: Self::audit_context(&context),
            })
            .await
            .map_err(map_lifecycle_error)?;
        Ok(map_publication(result.publication))
    }

    async fn pause_wiki_publication(
        &self,
        context: KnowledgeAppRequestContext,
        space_id: u64,
        request: KnowledgeWikiPublicationVersionCommandRequest,
    ) -> ApiResult<KnowledgeWikiPublication> {
        require_space_access_with_role(
            &self.runtime,
            &context,
            space_id,
            KnowledgeAccessRole::Owner,
        )
        .await?;
        let actor_id = require_numeric_actor_id(&context)?;
        let result = KnowledgeWikiPublicationLifecycleService::new(self.runtime.wiki_store())
            .change_publication_status(ChangeWikiPublicationStatusRequest {
                scope: self.scope(&context),
                space_id,
                expected_version: request.expected_version,
                actor_id,
                action: WikiPublicationLifecycleAction::Pause,
                audit: Self::audit_context(&context),
            })
            .await
            .map_err(map_lifecycle_error)?;
        Ok(map_publication(result.publication))
    }

    async fn publish_wiki_source_file(
        &self,
        context: KnowledgeAppRequestContext,
        space_id: u64,
        source_file_uuid: String,
        request: PublishKnowledgeWikiSourceFileRequest,
    ) -> ApiResult<KnowledgeWikiSourceFileCommandResult> {
        require_space_access_with_role(
            &self.runtime,
            &context,
            space_id,
            KnowledgeAccessRole::Writer,
        )
        .await?;
        let actor_id = require_numeric_actor_id(&context)?;
        let result = KnowledgeWikiPublicationLifecycleService::new(self.runtime.wiki_store())
            .publish_page(PublishWikiPageRequest {
                scope: self.scope(&context),
                space_id,
                source_file_uuid: source_file_uuid.clone(),
                visibility: map_visibility_request(request.visibility),
                expected_publication_version: request.expected_publication_version,
                expected_page_version: request.expected_page_version,
                actor_id,
                audit: Self::audit_context(&context),
            })
            .await
            .map_err(map_lifecycle_error)?;
        Ok(map_page_result(result.publication, result.page))
    }

    async fn unpublish_wiki_source_file(
        &self,
        context: KnowledgeAppRequestContext,
        space_id: u64,
        source_file_uuid: String,
        request: KnowledgeWikiSourceFileVersionCommandRequest,
    ) -> ApiResult<KnowledgeWikiSourceFileCommandResult> {
        require_space_access_with_role(
            &self.runtime,
            &context,
            space_id,
            KnowledgeAccessRole::Writer,
        )
        .await?;
        let actor_id = require_numeric_actor_id(&context)?;
        let result = KnowledgeWikiPublicationLifecycleService::new(self.runtime.wiki_store())
            .unpublish_page(UnpublishWikiPageRequest {
                scope: self.scope(&context),
                space_id,
                source_file_uuid: source_file_uuid.clone(),
                expected_publication_version: request.expected_publication_version,
                expected_page_version: request.expected_page_version,
                actor_id,
                audit: Self::audit_context(&context),
            })
            .await
            .map_err(map_lifecycle_error)?;
        Ok(map_page_result(result.publication, result.page))
    }

    async fn change_wiki_source_file_visibility(
        &self,
        context: KnowledgeAppRequestContext,
        space_id: u64,
        source_file_uuid: String,
        request: ChangeKnowledgeWikiSourceFileVisibilityRequest,
    ) -> ApiResult<KnowledgeWikiSourceFileCommandResult> {
        require_space_access_with_role(
            &self.runtime,
            &context,
            space_id,
            KnowledgeAccessRole::Writer,
        )
        .await?;
        let actor_id = require_numeric_actor_id(&context)?;
        let result = KnowledgeWikiPublicationLifecycleService::new(self.runtime.wiki_store())
            .change_page_visibility(ChangeWikiPageVisibilityRequest {
                scope: self.scope(&context),
                space_id,
                source_file_uuid: source_file_uuid.clone(),
                visibility: map_visibility_request(request.visibility),
                expected_publication_version: request.expected_publication_version,
                expected_page_version: request.expected_page_version,
                actor_id,
                audit: Self::audit_context(&context),
            })
            .await
            .map_err(map_lifecycle_error)?;
        Ok(map_page_result(result.publication, result.page))
    }
}

fn map_page_result(
    publication: sdkwork_intelligence_knowledgebase_service::ports::knowledge_wiki_persistence::WikiPublication,
    page: WikiSourceProjection,
) -> KnowledgeWikiSourceFileCommandResult {
    KnowledgeWikiSourceFileCommandResult {
        publication: map_publication(publication),
        source_file: map_source_file(page),
    }
}

fn map_publication(
    value: sdkwork_intelligence_knowledgebase_service::ports::knowledge_wiki_persistence::WikiPublication,
) -> KnowledgeWikiPublication {
    KnowledgeWikiPublication {
        uuid: value.uuid,
        space_id: value.space_id,
        drive_space_uuid: value.drive_space_uuid,
        source_root_node_uuid: value.source_root_node_uuid,
        status: match value.wiki_status {
            WikiPublicationStatus::Draft => KnowledgeWikiPublicationStatus::Draft,
            WikiPublicationStatus::Validating => KnowledgeWikiPublicationStatus::Validating,
            WikiPublicationStatus::Ready => KnowledgeWikiPublicationStatus::Ready,
            WikiPublicationStatus::Active => KnowledgeWikiPublicationStatus::Active,
            WikiPublicationStatus::Degraded => KnowledgeWikiPublicationStatus::Degraded,
            WikiPublicationStatus::Paused => KnowledgeWikiPublicationStatus::Paused,
            WikiPublicationStatus::Archived => KnowledgeWikiPublicationStatus::Archived,
            WikiPublicationStatus::Failed => KnowledgeWikiPublicationStatus::Failed,
        },
        title: value.title,
        homepage_source_path: value.homepage_source_path,
        publication_mode: match value.publication_mode {
            WikiPublicationMode::ReviewRequired => KnowledgeWikiPublicationMode::ReviewRequired,
            WikiPublicationMode::AutoPublicAfterChecks => {
                KnowledgeWikiPublicationMode::AutoPublicAfterChecks
            }
        },
        default_visibility: map_visibility(value.default_visibility),
        update_policy: match value.update_policy {
            WikiUpdatePolicy::KeepLastPublicUntilReady => {
                KnowledgeWikiUpdatePolicy::KeepLastPublicUntilReady
            }
            WikiUpdatePolicy::UnpublishDuringProcessing => {
                KnowledgeWikiUpdatePolicy::UnpublishDuringProcessing
            }
        },
        provider_generation: value.provider_generation,
        navigation_generation: value.navigation_generation,
        search_generation: value.search_generation,
        last_projected_drive_checkpoint: value.last_projected_drive_checkpoint,
        version: value.version,
    }
}

fn map_source_file(value: WikiSourceProjection) -> KnowledgeWikiSourceFile {
    KnowledgeWikiSourceFile {
        uuid: value.uuid,
        drive_node_uuid: value.drive_node_uuid,
        drive_version_uuid: value.drive_version_uuid,
        source_path: value.source_path,
        canonical_route: value.canonical_route,
        file_kind: match value.file_kind {
            WikiSourceFileKind::Page => KnowledgeWikiSourceFileKind::Page,
            WikiSourceFileKind::Document => KnowledgeWikiSourceFileKind::Document,
            WikiSourceFileKind::Presentation => KnowledgeWikiSourceFileKind::Presentation,
            WikiSourceFileKind::Spreadsheet => KnowledgeWikiSourceFileKind::Spreadsheet,
            WikiSourceFileKind::Code => KnowledgeWikiSourceFileKind::Code,
            WikiSourceFileKind::Media => KnowledgeWikiSourceFileKind::Media,
            WikiSourceFileKind::Asset => KnowledgeWikiSourceFileKind::Asset,
            WikiSourceFileKind::Archive => KnowledgeWikiSourceFileKind::Archive,
        },
        media_type: value.media_type,
        size_bytes: value.size_bytes,
        content_sha256: value.content_sha256,
        source_state: match value.source_state {
            WikiSourceState::Discovered => KnowledgeWikiSourceState::Discovered,
            WikiSourceState::Queued => KnowledgeWikiSourceState::Queued,
            WikiSourceState::Processing => KnowledgeWikiSourceState::Processing,
            WikiSourceState::Ready => KnowledgeWikiSourceState::Ready,
            WikiSourceState::Error => KnowledgeWikiSourceState::Error,
            WikiSourceState::Quarantined => KnowledgeWikiSourceState::Quarantined,
            WikiSourceState::Deleted => KnowledgeWikiSourceState::Deleted,
        },
        publication_state: match value.publication_state {
            WikiPagePublicationState::Draft => KnowledgeWikiPagePublicationState::Draft,
            WikiPagePublicationState::InReview => KnowledgeWikiPagePublicationState::InReview,
            WikiPagePublicationState::Scheduled => KnowledgeWikiPagePublicationState::Scheduled,
            WikiPagePublicationState::Published => KnowledgeWikiPagePublicationState::Published,
            WikiPagePublicationState::Unpublished => KnowledgeWikiPagePublicationState::Unpublished,
            WikiPagePublicationState::Archived => KnowledgeWikiPagePublicationState::Archived,
        },
        visibility: map_visibility(value.visibility),
        index_state: match value.index_state {
            WikiIndexState::NotRequired => KnowledgeWikiIndexState::NotRequired,
            WikiIndexState::Pending => KnowledgeWikiIndexState::Pending,
            WikiIndexState::Indexing => KnowledgeWikiIndexState::Indexing,
            WikiIndexState::Ready => KnowledgeWikiIndexState::Ready,
            WikiIndexState::Error => KnowledgeWikiIndexState::Error,
        },
        public_drive_version_uuid: value.public_drive_version_uuid,
        page_public_version: value.page_public_version,
        version: value.version,
    }
}

fn map_visibility(value: WikiVisibility) -> KnowledgeWikiVisibility {
    match value {
        WikiVisibility::Private => KnowledgeWikiVisibility::Private,
        WikiVisibility::Unlisted => KnowledgeWikiVisibility::Unlisted,
        WikiVisibility::Public => KnowledgeWikiVisibility::Public,
    }
}

fn map_visibility_request(value: KnowledgeWikiVisibility) -> WikiVisibility {
    match value {
        KnowledgeWikiVisibility::Private => WikiVisibility::Private,
        KnowledgeWikiVisibility::Unlisted => WikiVisibility::Unlisted,
        KnowledgeWikiVisibility::Public => WikiVisibility::Public,
    }
}

fn map_lifecycle_error(error: KnowledgeWikiPublicationLifecycleError) -> ApiError {
    match error {
        KnowledgeWikiPublicationLifecycleError::InvalidRequest(detail) => {
            ApiError::invalid_request("invalid_wiki_publication_command", detail)
        }
        KnowledgeWikiPublicationLifecycleError::Persistence(error) => map_persistence_error(error),
    }
}

fn map_persistence_error(
    error: sdkwork_intelligence_knowledgebase_service::ports::knowledge_wiki_persistence::WikiPersistenceError,
) -> ApiError {
    use sdkwork_intelligence_knowledgebase_service::ports::knowledge_wiki_persistence::WikiPersistenceError;

    match error {
        WikiPersistenceError::InvalidRequest(detail) => {
            ApiError::invalid_request("invalid_wiki_publication_command", detail)
        }
        WikiPersistenceError::NotFound { .. } => ApiError::not_found(
            "wiki_publication_resource_not_found",
            "Wiki publication resource was not found",
        ),
        WikiPersistenceError::Conflict(_) | WikiPersistenceError::StaleVersion { .. } => {
            ApiError::conflict(
                "wiki_publication_version_conflict",
                "Wiki publication state changed; refresh the resource and retry",
            )
        }
        WikiPersistenceError::Internal(detail) => {
            ApiError::sanitized_internal("wiki_publication_store_failed", detail)
        }
    }
}
