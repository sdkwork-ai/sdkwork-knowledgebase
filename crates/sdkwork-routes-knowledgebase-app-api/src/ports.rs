use async_trait::async_trait;
use sdkwork_knowledgebase_contract::{
    context_binding::{
        CreateKnowledgeSpaceContextBindingRequest, KnowledgeContextType,
        KnowledgeSpaceContextBinding, UpdateKnowledgeSpaceContextBindingRequest,
    },
    group_space::{ConsumeGroupKnowledgebaseLaunchTicketRequest, GroupKnowledgebaseLaunchTarget},
    ChangeKnowledgeWikiSourceFileVisibilityRequest, CreateKnowledgeDocumentRequest,
    CreateKnowledgeDocumentVersionRequest, CreateKnowledgeSpaceRequest,
    GrantKnowledgeSpaceMemberRequest, IngestionJob, KnowledgeAgentBinding,
    KnowledgeAgentBindingList, KnowledgeAgentBindingRequest, KnowledgeAgentChatRequest,
    KnowledgeAgentChatResponse, KnowledgeAgentProfile, KnowledgeAgentProfileRequest,
    KnowledgeBrowserListData, KnowledgeContextPack, KnowledgeContextPackRequest, KnowledgeDocument,
    KnowledgeDocumentContent, KnowledgeDocumentVersion, KnowledgeDriveImportRequest,
    KnowledgeDriveImportResult, KnowledgeGitImportRequest, KnowledgeGitImportResult,
    KnowledgeGitSyncRequest, KnowledgeGitSyncResult, KnowledgeIngestRequest,
    KnowledgeMarketCatalogItem, KnowledgeMarketSubscriptionRequest,
    KnowledgeMarketSubscriptionResult, KnowledgeMediaTaskRequest, KnowledgeMediaTaskResult,
    KnowledgeOkfBundleFile, KnowledgeOkfConceptRevision, KnowledgeRetrievalRequest,
    KnowledgeRetrievalResult, KnowledgeSpace, KnowledgeSpaceMember,
    KnowledgeSpaceMemberSubjectType, KnowledgeWechatAppletList,
    KnowledgeWechatArticlesPreviewRequest, KnowledgeWechatArticlesPublishRequest,
    KnowledgeWechatFanTagList, KnowledgeWechatOfficialAccountList, KnowledgeWechatOperationResult,
    KnowledgeWechatReplaceAppletsRequest, KnowledgeWechatReplaceOfficialAccountsRequest,
    KnowledgeWikiPublication, KnowledgeWikiPublicationVersionCommandRequest,
    KnowledgeWikiSourceFileCommandResult, KnowledgeWikiSourceFileVersionCommandRequest,
    ListKnowledgeBrowserRequest, OkfBundleExportRequest, OkfBundleImportRequest,
    OkfBundleImportResult, OkfConceptSummary, OkfConceptUpsertRequest, OkfContextPackRequest,
    OkfFileAnswerRequest, OkfIndexDocument, OkfLogDocument, OkfProfileDocument, OkfQualityRun,
    OkfQualityRunRequest, OkfQueryRequest, OkfQueryResult, PublishKnowledgeWikiSourceFileRequest,
    UpdateKnowledgeSpaceRequest,
};
use sdkwork_utils_rust::SdkWorkPageData;

use crate::{ApiError, ApiResult};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct KnowledgeAppRequestContext {
    pub tenant_id: u64,
    pub actor_id: Option<u64>,
    pub organization_id: Option<u64>,
    pub session_id: Option<String>,
    pub request_id: String,
    pub trace_id: Option<String>,
    pub idempotency_key: Option<String>,
}

#[async_trait]
pub trait KnowledgeSpaceAppService: Send + Sync + 'static {
    async fn create_space(
        &self,
        context: KnowledgeAppRequestContext,
        request: CreateKnowledgeSpaceRequest,
    ) -> ApiResult<KnowledgeSpace>;

    async fn retrieve_space(
        &self,
        context: KnowledgeAppRequestContext,
        space_id: u64,
    ) -> ApiResult<KnowledgeSpace>;

    async fn update_space(
        &self,
        context: KnowledgeAppRequestContext,
        space_id: u64,
        request: UpdateKnowledgeSpaceRequest,
    ) -> ApiResult<KnowledgeSpace>;

    async fn delete_space(
        &self,
        context: KnowledgeAppRequestContext,
        space_id: u64,
    ) -> ApiResult<()>;

    async fn list_space_members(
        &self,
        context: KnowledgeAppRequestContext,
        space_id: u64,
        cursor: Option<String>,
        page_size: Option<u32>,
    ) -> ApiResult<SdkWorkPageData<KnowledgeSpaceMember>>;

    async fn grant_space_member(
        &self,
        context: KnowledgeAppRequestContext,
        space_id: u64,
        request: GrantKnowledgeSpaceMemberRequest,
    ) -> ApiResult<()>;

    async fn revoke_space_member(
        &self,
        context: KnowledgeAppRequestContext,
        space_id: u64,
        subject_type: KnowledgeSpaceMemberSubjectType,
        subject_id: String,
    ) -> ApiResult<()>;
}

#[async_trait]
pub trait KnowledgeWikiPublicationAppService: Send + Sync + 'static {
    async fn retrieve_wiki_publication(
        &self,
        context: KnowledgeAppRequestContext,
        space_id: u64,
    ) -> ApiResult<KnowledgeWikiPublication>;

    async fn activate_wiki_publication(
        &self,
        context: KnowledgeAppRequestContext,
        space_id: u64,
        request: KnowledgeWikiPublicationVersionCommandRequest,
    ) -> ApiResult<KnowledgeWikiPublication>;

    async fn pause_wiki_publication(
        &self,
        context: KnowledgeAppRequestContext,
        space_id: u64,
        request: KnowledgeWikiPublicationVersionCommandRequest,
    ) -> ApiResult<KnowledgeWikiPublication>;

    async fn publish_wiki_source_file(
        &self,
        context: KnowledgeAppRequestContext,
        space_id: u64,
        source_file_uuid: String,
        request: PublishKnowledgeWikiSourceFileRequest,
    ) -> ApiResult<KnowledgeWikiSourceFileCommandResult>;

    async fn unpublish_wiki_source_file(
        &self,
        context: KnowledgeAppRequestContext,
        space_id: u64,
        source_file_uuid: String,
        request: KnowledgeWikiSourceFileVersionCommandRequest,
    ) -> ApiResult<KnowledgeWikiSourceFileCommandResult>;

    async fn change_wiki_source_file_visibility(
        &self,
        context: KnowledgeAppRequestContext,
        space_id: u64,
        source_file_uuid: String,
        request: ChangeKnowledgeWikiSourceFileVisibilityRequest,
    ) -> ApiResult<KnowledgeWikiSourceFileCommandResult>;
}

/// User-facing ticket-consumption surface. Trusted IM provisioning and membership synchronization
/// deliberately do not share this App API boundary.
#[async_trait]
pub trait KnowledgeGroupLaunchAppService: Send + Sync + 'static {
    async fn consume_group_launch_ticket(
        &self,
        context: KnowledgeAppRequestContext,
        request: ConsumeGroupKnowledgebaseLaunchTicketRequest,
    ) -> ApiResult<GroupKnowledgebaseLaunchTarget>;
}

#[async_trait]
pub trait KnowledgeDriveImportAppService: Send + Sync + 'static {
    async fn import_drive_object(
        &self,
        context: KnowledgeAppRequestContext,
        request: KnowledgeDriveImportRequest,
    ) -> ApiResult<KnowledgeDriveImportResult>;
}

#[async_trait]
pub trait KnowledgeGitImportAppService: Send + Sync + 'static {
    async fn create_git_import(
        &self,
        context: KnowledgeAppRequestContext,
        request: KnowledgeGitImportRequest,
    ) -> ApiResult<KnowledgeGitImportResult>;

    async fn create_git_sync(
        &self,
        context: KnowledgeAppRequestContext,
        request: KnowledgeGitSyncRequest,
    ) -> ApiResult<KnowledgeGitSyncResult>;
}

#[async_trait]
pub trait KnowledgeWechatAppService: Send + Sync + 'static {
    async fn list_official_accounts(
        &self,
        context: KnowledgeAppRequestContext,
    ) -> ApiResult<KnowledgeWechatOfficialAccountList>;

    async fn replace_official_accounts(
        &self,
        context: KnowledgeAppRequestContext,
        request: KnowledgeWechatReplaceOfficialAccountsRequest,
    ) -> ApiResult<KnowledgeWechatOfficialAccountList>;

    async fn list_official_account_fan_tags(
        &self,
        context: KnowledgeAppRequestContext,
        account_id: String,
    ) -> ApiResult<KnowledgeWechatFanTagList>;

    async fn list_applets(
        &self,
        context: KnowledgeAppRequestContext,
    ) -> ApiResult<KnowledgeWechatAppletList>;

    async fn replace_applets(
        &self,
        context: KnowledgeAppRequestContext,
        request: KnowledgeWechatReplaceAppletsRequest,
    ) -> ApiResult<KnowledgeWechatAppletList>;

    async fn publish_articles(
        &self,
        context: KnowledgeAppRequestContext,
        request: KnowledgeWechatArticlesPublishRequest,
    ) -> ApiResult<KnowledgeWechatOperationResult>;

    async fn preview_articles(
        &self,
        context: KnowledgeAppRequestContext,
        request: KnowledgeWechatArticlesPreviewRequest,
    ) -> ApiResult<KnowledgeWechatOperationResult>;
}

#[async_trait]
pub trait KnowledgeCommerceAppService: Send + Sync + 'static {
    async fn list_market_listings(
        &self,
        context: KnowledgeAppRequestContext,
        cursor: Option<String>,
        page_size: Option<u32>,
    ) -> ApiResult<sdkwork_utils_rust::SdkWorkPageData<KnowledgeMarketCatalogItem>>;

    async fn create_market_subscription(
        &self,
        context: KnowledgeAppRequestContext,
        request: KnowledgeMarketSubscriptionRequest,
    ) -> ApiResult<KnowledgeMarketSubscriptionResult>;

    async fn delete_market_subscription(
        &self,
        context: KnowledgeAppRequestContext,
        listing_id: u64,
    ) -> ApiResult<KnowledgeMarketSubscriptionResult>;

    async fn create_media_task(
        &self,
        context: KnowledgeAppRequestContext,
        request: KnowledgeMediaTaskRequest,
    ) -> ApiResult<KnowledgeMediaTaskResult>;
}

#[async_trait]
pub trait KnowledgeIngestAppService: Send + Sync + 'static {
    async fn create_ingest(
        &self,
        context: KnowledgeAppRequestContext,
        request: KnowledgeIngestRequest,
    ) -> ApiResult<IngestionJob>;

    async fn retrieve_ingest(
        &self,
        context: KnowledgeAppRequestContext,
        ingest_id: u64,
    ) -> ApiResult<IngestionJob>;
}

#[async_trait]
pub trait KnowledgeDocumentAppService: Send + Sync + 'static {
    async fn list_documents(
        &self,
        context: KnowledgeAppRequestContext,
        space_id: u64,
        cursor: Option<String>,
        page_size: Option<u32>,
    ) -> ApiResult<SdkWorkPageData<KnowledgeDocument>>;

    async fn create_document(
        &self,
        context: KnowledgeAppRequestContext,
        request: CreateKnowledgeDocumentRequest,
    ) -> ApiResult<KnowledgeDocument>;

    async fn retrieve_document(
        &self,
        context: KnowledgeAppRequestContext,
        document_id: u64,
    ) -> ApiResult<KnowledgeDocument>;

    async fn update_document(
        &self,
        context: KnowledgeAppRequestContext,
        document_id: u64,
        request: CreateKnowledgeDocumentRequest,
    ) -> ApiResult<KnowledgeDocument>;

    async fn delete_document(
        &self,
        context: KnowledgeAppRequestContext,
        document_id: u64,
    ) -> ApiResult<()>;

    async fn list_document_versions(
        &self,
        context: KnowledgeAppRequestContext,
        document_id: u64,
        cursor: Option<String>,
        page_size: Option<u32>,
    ) -> ApiResult<SdkWorkPageData<KnowledgeDocumentVersion>>;

    async fn create_document_version(
        &self,
        context: KnowledgeAppRequestContext,
        document_id: u64,
        request: CreateKnowledgeDocumentVersionRequest,
    ) -> ApiResult<KnowledgeDocumentVersion>;

    async fn retrieve_document_content(
        &self,
        context: KnowledgeAppRequestContext,
        document_id: u64,
    ) -> ApiResult<KnowledgeDocumentContent>;
}

#[async_trait]
pub trait KnowledgeOkfAppService: Send + Sync + 'static {
    async fn list_okf_concepts(
        &self,
        context: KnowledgeAppRequestContext,
        space_id: u64,
        cursor: Option<String>,
        page_size: Option<u32>,
    ) -> ApiResult<SdkWorkPageData<OkfConceptSummary>>;

    async fn retrieve_okf_concept(
        &self,
        context: KnowledgeAppRequestContext,
        concept_row_id: u64,
    ) -> ApiResult<OkfConceptSummary>;

    async fn list_okf_concept_revisions(
        &self,
        context: KnowledgeAppRequestContext,
        concept_row_id: u64,
        cursor: Option<String>,
        page_size: Option<u32>,
    ) -> ApiResult<SdkWorkPageData<KnowledgeOkfConceptRevision>>;

    async fn upsert_okf_concept(
        &self,
        context: KnowledgeAppRequestContext,
        request: OkfConceptUpsertRequest,
    ) -> ApiResult<OkfConceptSummary>;

    async fn delete_okf_concept(
        &self,
        context: KnowledgeAppRequestContext,
        concept_row_id: u64,
    ) -> ApiResult<()>;

    async fn retrieve_okf_index(
        &self,
        context: KnowledgeAppRequestContext,
        space_id: u64,
    ) -> ApiResult<OkfIndexDocument>;

    async fn retrieve_okf_log(
        &self,
        context: KnowledgeAppRequestContext,
        space_id: u64,
    ) -> ApiResult<OkfLogDocument>;

    async fn retrieve_okf_schema(
        &self,
        context: KnowledgeAppRequestContext,
        space_id: u64,
    ) -> ApiResult<OkfProfileDocument>;

    async fn create_okf_query(
        &self,
        context: KnowledgeAppRequestContext,
        request: OkfQueryRequest,
    ) -> ApiResult<OkfQueryResult>;

    async fn file_okf_query_answer(
        &self,
        context: KnowledgeAppRequestContext,
        query_id: u64,
        request: OkfFileAnswerRequest,
    ) -> ApiResult<OkfQueryResult>;

    async fn create_okf_context_pack(
        &self,
        context: KnowledgeAppRequestContext,
        request: OkfContextPackRequest,
    ) -> ApiResult<KnowledgeOkfBundleFile>;

    async fn create_okf_export(
        &self,
        context: KnowledgeAppRequestContext,
        request: OkfBundleExportRequest,
    ) -> ApiResult<KnowledgeOkfBundleFile>;

    async fn retrieve_okf_export(
        &self,
        context: KnowledgeAppRequestContext,
        export_id: u64,
    ) -> ApiResult<KnowledgeOkfBundleFile>;

    async fn create_okf_import(
        &self,
        context: KnowledgeAppRequestContext,
        request: OkfBundleImportRequest,
    ) -> ApiResult<OkfBundleImportResult>;

    async fn create_okf_lint_run(
        &self,
        context: KnowledgeAppRequestContext,
        request: OkfQualityRunRequest,
    ) -> ApiResult<OkfQualityRun>;
}

#[async_trait]
pub trait KnowledgeBrowserApi: Send + Sync + 'static {
    async fn list_browser(
        &self,
        context: KnowledgeAppRequestContext,
        request: ListKnowledgeBrowserRequest,
    ) -> ApiResult<KnowledgeBrowserListData>;
}

#[async_trait]
pub trait KnowledgeRetrievalAppService: Send + Sync + 'static {
    async fn retrieve(
        &self,
        context: KnowledgeAppRequestContext,
        request: KnowledgeRetrievalRequest,
    ) -> ApiResult<KnowledgeRetrievalResult>;

    async fn retrieve_retrieval(
        &self,
        context: KnowledgeAppRequestContext,
        retrieval_id: u64,
    ) -> ApiResult<KnowledgeRetrievalResult>;

    async fn create_context_pack(
        &self,
        context: KnowledgeAppRequestContext,
        request: KnowledgeContextPackRequest,
    ) -> ApiResult<KnowledgeContextPack>;
}

#[async_trait]
pub trait KnowledgeContextBindingAppService: Send + Sync + 'static {
    async fn list_space_context_bindings(
        &self,
        context: KnowledgeAppRequestContext,
        space_id: u64,
        cursor: Option<String>,
        page_size: Option<u32>,
        context_type: Option<KnowledgeContextType>,
    ) -> ApiResult<SdkWorkPageData<KnowledgeSpaceContextBinding>>;

    async fn create_space_context_binding(
        &self,
        context: KnowledgeAppRequestContext,
        space_id: u64,
        request: CreateKnowledgeSpaceContextBindingRequest,
    ) -> ApiResult<KnowledgeSpaceContextBinding>;

    async fn retrieve_context_binding(
        &self,
        context: KnowledgeAppRequestContext,
        binding_id: u64,
    ) -> ApiResult<KnowledgeSpaceContextBinding>;

    async fn update_context_binding(
        &self,
        context: KnowledgeAppRequestContext,
        binding_id: u64,
        request: UpdateKnowledgeSpaceContextBindingRequest,
    ) -> ApiResult<KnowledgeSpaceContextBinding>;

    async fn delete_context_binding(
        &self,
        context: KnowledgeAppRequestContext,
        binding_id: u64,
    ) -> ApiResult<()>;
}

#[async_trait]
pub trait KnowledgeAgentAppService: Send + Sync + 'static {
    async fn create_profile(
        &self,
        context: KnowledgeAppRequestContext,
        request: KnowledgeAgentProfileRequest,
    ) -> ApiResult<KnowledgeAgentProfile>;

    async fn retrieve_profile(
        &self,
        context: KnowledgeAppRequestContext,
        profile_id: u64,
    ) -> ApiResult<KnowledgeAgentProfile>;

    async fn update_profile(
        &self,
        context: KnowledgeAppRequestContext,
        profile_id: u64,
        request: KnowledgeAgentProfileRequest,
    ) -> ApiResult<KnowledgeAgentProfile>;

    async fn delete_profile(
        &self,
        context: KnowledgeAppRequestContext,
        profile_id: u64,
    ) -> ApiResult<()>;

    async fn list_bindings(
        &self,
        context: KnowledgeAppRequestContext,
        profile_id: u64,
    ) -> ApiResult<KnowledgeAgentBindingList>;

    async fn create_binding(
        &self,
        context: KnowledgeAppRequestContext,
        profile_id: u64,
        request: KnowledgeAgentBindingRequest,
    ) -> ApiResult<KnowledgeAgentBinding>;

    async fn update_binding(
        &self,
        context: KnowledgeAppRequestContext,
        profile_id: u64,
        binding_id: u64,
        request: KnowledgeAgentBindingRequest,
    ) -> ApiResult<KnowledgeAgentBinding>;

    async fn delete_binding(
        &self,
        context: KnowledgeAppRequestContext,
        profile_id: u64,
        binding_id: u64,
    ) -> ApiResult<()>;

    async fn preview_retrieval(
        &self,
        context: KnowledgeAppRequestContext,
        profile_id: u64,
        request: KnowledgeRetrievalRequest,
    ) -> ApiResult<KnowledgeRetrievalResult>;

    async fn create_agent_chat(
        &self,
        context: KnowledgeAppRequestContext,
        profile_id: u64,
        request: KnowledgeAgentChatRequest,
    ) -> ApiResult<KnowledgeAgentChatResponse>;
}

#[async_trait]
pub trait KnowledgeAppApi: Send + Sync + 'static {
    async fn create_space(
        &self,
        _context: KnowledgeAppRequestContext,
        _request: CreateKnowledgeSpaceRequest,
    ) -> ApiResult<KnowledgeSpace> {
        Err(ApiError::unsupported_operation("spaces.create"))
    }

    async fn retrieve_space(
        &self,
        _context: KnowledgeAppRequestContext,
        _space_id: u64,
    ) -> ApiResult<KnowledgeSpace> {
        Err(ApiError::unsupported_operation("spaces.retrieve"))
    }

    async fn update_space(
        &self,
        _context: KnowledgeAppRequestContext,
        _space_id: u64,
        _request: UpdateKnowledgeSpaceRequest,
    ) -> ApiResult<KnowledgeSpace> {
        Err(ApiError::unsupported_operation("spaces.update"))
    }

    async fn delete_space(
        &self,
        _context: KnowledgeAppRequestContext,
        _space_id: u64,
    ) -> ApiResult<()> {
        Err(ApiError::unsupported_operation("spaces.delete"))
    }

    async fn retrieve_wiki_publication(
        &self,
        _context: KnowledgeAppRequestContext,
        _space_id: u64,
    ) -> ApiResult<KnowledgeWikiPublication> {
        Err(ApiError::unsupported_operation("wikiPublications.retrieve"))
    }

    async fn activate_wiki_publication(
        &self,
        _context: KnowledgeAppRequestContext,
        _space_id: u64,
        _request: KnowledgeWikiPublicationVersionCommandRequest,
    ) -> ApiResult<KnowledgeWikiPublication> {
        Err(ApiError::unsupported_operation("wikiPublications.activate"))
    }

    async fn pause_wiki_publication(
        &self,
        _context: KnowledgeAppRequestContext,
        _space_id: u64,
        _request: KnowledgeWikiPublicationVersionCommandRequest,
    ) -> ApiResult<KnowledgeWikiPublication> {
        Err(ApiError::unsupported_operation("wikiPublications.pause"))
    }

    async fn publish_wiki_source_file(
        &self,
        _context: KnowledgeAppRequestContext,
        _space_id: u64,
        _source_file_uuid: String,
        _request: PublishKnowledgeWikiSourceFileRequest,
    ) -> ApiResult<KnowledgeWikiSourceFileCommandResult> {
        Err(ApiError::unsupported_operation("wikiSourceFiles.publish"))
    }

    async fn unpublish_wiki_source_file(
        &self,
        _context: KnowledgeAppRequestContext,
        _space_id: u64,
        _source_file_uuid: String,
        _request: KnowledgeWikiSourceFileVersionCommandRequest,
    ) -> ApiResult<KnowledgeWikiSourceFileCommandResult> {
        Err(ApiError::unsupported_operation("wikiSourceFiles.unpublish"))
    }

    async fn change_wiki_source_file_visibility(
        &self,
        _context: KnowledgeAppRequestContext,
        _space_id: u64,
        _source_file_uuid: String,
        _request: ChangeKnowledgeWikiSourceFileVisibilityRequest,
    ) -> ApiResult<KnowledgeWikiSourceFileCommandResult> {
        Err(ApiError::unsupported_operation(
            "wikiSourceFiles.visibility.update",
        ))
    }

    async fn list_space_members(
        &self,
        _context: KnowledgeAppRequestContext,
        _space_id: u64,
        _cursor: Option<String>,
        _page_size: Option<u32>,
    ) -> ApiResult<SdkWorkPageData<KnowledgeSpaceMember>> {
        Err(ApiError::unsupported_operation("spaces.members.list"))
    }

    async fn grant_space_member(
        &self,
        _context: KnowledgeAppRequestContext,
        _space_id: u64,
        _request: GrantKnowledgeSpaceMemberRequest,
    ) -> ApiResult<()> {
        Err(ApiError::unsupported_operation("spaces.members.create"))
    }

    async fn revoke_space_member(
        &self,
        _context: KnowledgeAppRequestContext,
        _space_id: u64,
        _subject_type: KnowledgeSpaceMemberSubjectType,
        _subject_id: String,
    ) -> ApiResult<()> {
        Err(ApiError::unsupported_operation("spaces.members.delete"))
    }

    async fn consume_group_launch_ticket(
        &self,
        _context: KnowledgeAppRequestContext,
        _request: ConsumeGroupKnowledgebaseLaunchTicketRequest,
    ) -> ApiResult<GroupKnowledgebaseLaunchTarget> {
        Err(ApiError::unsupported_operation("groupLaunches.consume"))
    }

    async fn create_drive_import(
        &self,
        _context: KnowledgeAppRequestContext,
        _request: KnowledgeDriveImportRequest,
    ) -> ApiResult<KnowledgeDriveImportResult> {
        Err(ApiError::unsupported_operation("driveImports.create"))
    }

    async fn create_git_import(
        &self,
        _context: KnowledgeAppRequestContext,
        _request: KnowledgeGitImportRequest,
    ) -> ApiResult<KnowledgeGitImportResult> {
        Err(ApiError::unsupported_operation("gitImports.create"))
    }

    async fn create_git_sync(
        &self,
        _context: KnowledgeAppRequestContext,
        _request: KnowledgeGitSyncRequest,
    ) -> ApiResult<KnowledgeGitSyncResult> {
        Err(ApiError::unsupported_operation("gitSyncs.create"))
    }

    async fn create_ingest(
        &self,
        _context: KnowledgeAppRequestContext,
        _request: KnowledgeIngestRequest,
    ) -> ApiResult<IngestionJob> {
        Err(ApiError::unsupported_operation("ingests.create"))
    }

    async fn retrieve_ingest(
        &self,
        _context: KnowledgeAppRequestContext,
        _ingest_id: u64,
    ) -> ApiResult<IngestionJob> {
        Err(ApiError::unsupported_operation("ingests.retrieve"))
    }

    async fn list_documents(
        &self,
        _context: KnowledgeAppRequestContext,
        _space_id: u64,
        _cursor: Option<String>,
        _page_size: Option<u32>,
    ) -> ApiResult<SdkWorkPageData<KnowledgeDocument>> {
        Err(ApiError::unsupported_operation("documents.list"))
    }

    async fn create_document(
        &self,
        _context: KnowledgeAppRequestContext,
        _request: CreateKnowledgeDocumentRequest,
    ) -> ApiResult<KnowledgeDocument> {
        Err(ApiError::unsupported_operation("documents.create"))
    }

    async fn retrieve_document(
        &self,
        _context: KnowledgeAppRequestContext,
        _document_id: u64,
    ) -> ApiResult<KnowledgeDocument> {
        Err(ApiError::unsupported_operation("documents.retrieve"))
    }

    async fn update_document(
        &self,
        _context: KnowledgeAppRequestContext,
        _document_id: u64,
        _request: CreateKnowledgeDocumentRequest,
    ) -> ApiResult<KnowledgeDocument> {
        Err(ApiError::unsupported_operation("documents.update"))
    }

    async fn delete_document(
        &self,
        _context: KnowledgeAppRequestContext,
        _document_id: u64,
    ) -> ApiResult<()> {
        Err(ApiError::unsupported_operation("documents.delete"))
    }

    async fn list_document_versions(
        &self,
        _context: KnowledgeAppRequestContext,
        _document_id: u64,
        _cursor: Option<String>,
        _page_size: Option<u32>,
    ) -> ApiResult<SdkWorkPageData<KnowledgeDocumentVersion>> {
        Err(ApiError::unsupported_operation("documents.versions.list"))
    }

    async fn create_document_version(
        &self,
        _context: KnowledgeAppRequestContext,
        _document_id: u64,
        _request: CreateKnowledgeDocumentVersionRequest,
    ) -> ApiResult<KnowledgeDocumentVersion> {
        Err(ApiError::unsupported_operation(
            "documents.versions.create",
        ))
    }

    async fn retrieve_document_content(
        &self,
        _context: KnowledgeAppRequestContext,
        _document_id: u64,
    ) -> ApiResult<KnowledgeDocumentContent> {
        Err(ApiError::unsupported_operation("documents.content.list"))
    }

    async fn list_okf_concepts(
        &self,
        _context: KnowledgeAppRequestContext,
        _space_id: u64,
        _cursor: Option<String>,
        _page_size: Option<u32>,
    ) -> ApiResult<SdkWorkPageData<OkfConceptSummary>> {
        Err(ApiError::unsupported_operation("okf.concepts.list"))
    }

    async fn retrieve_okf_concept(
        &self,
        _context: KnowledgeAppRequestContext,
        _concept_row_id: u64,
    ) -> ApiResult<OkfConceptSummary> {
        Err(ApiError::unsupported_operation("okf.concepts.retrieve"))
    }

    async fn list_okf_concept_revisions(
        &self,
        _context: KnowledgeAppRequestContext,
        _concept_row_id: u64,
        _cursor: Option<String>,
        _page_size: Option<u32>,
    ) -> ApiResult<SdkWorkPageData<KnowledgeOkfConceptRevision>> {
        Err(ApiError::unsupported_operation(
            "okf.concepts.revisions.list",
        ))
    }

    async fn upsert_okf_concept(
        &self,
        _context: KnowledgeAppRequestContext,
        _request: OkfConceptUpsertRequest,
    ) -> ApiResult<OkfConceptSummary> {
        Err(ApiError::unsupported_operation("okf.concepts.update"))
    }

    async fn delete_okf_concept(
        &self,
        _context: KnowledgeAppRequestContext,
        _concept_row_id: u64,
    ) -> ApiResult<()> {
        Err(ApiError::unsupported_operation("okf.concepts.delete"))
    }

    async fn retrieve_okf_index(
        &self,
        _context: KnowledgeAppRequestContext,
        _space_id: u64,
    ) -> ApiResult<OkfIndexDocument> {
        Err(ApiError::unsupported_operation("okf.bundle.index.list"))
    }

    async fn retrieve_okf_log(
        &self,
        _context: KnowledgeAppRequestContext,
        _space_id: u64,
    ) -> ApiResult<OkfLogDocument> {
        Err(ApiError::unsupported_operation("okf.bundle.log.list"))
    }

    async fn retrieve_okf_schema(
        &self,
        _context: KnowledgeAppRequestContext,
        _space_id: u64,
    ) -> ApiResult<OkfProfileDocument> {
        Err(ApiError::unsupported_operation("okf.bundle.profile.list"))
    }

    async fn create_okf_query(
        &self,
        _context: KnowledgeAppRequestContext,
        _request: OkfQueryRequest,
    ) -> ApiResult<OkfQueryResult> {
        Err(ApiError::unsupported_operation("okf.queries.create"))
    }

    async fn file_okf_query_answer(
        &self,
        _context: KnowledgeAppRequestContext,
        _query_id: u64,
        _request: OkfFileAnswerRequest,
    ) -> ApiResult<OkfQueryResult> {
        Err(ApiError::unsupported_operation("okf.queries.fileAnswer"))
    }

    async fn create_okf_context_pack(
        &self,
        _context: KnowledgeAppRequestContext,
        _request: OkfContextPackRequest,
    ) -> ApiResult<KnowledgeOkfBundleFile> {
        Err(ApiError::unsupported_operation("okf.contextPacks.create"))
    }

    async fn create_okf_export(
        &self,
        _context: KnowledgeAppRequestContext,
        _request: OkfBundleExportRequest,
    ) -> ApiResult<KnowledgeOkfBundleFile> {
        Err(ApiError::unsupported_operation("okf.bundle.export.create"))
    }

    async fn retrieve_okf_export(
        &self,
        _context: KnowledgeAppRequestContext,
        _export_id: u64,
    ) -> ApiResult<KnowledgeOkfBundleFile> {
        Err(ApiError::unsupported_operation(
            "okf.bundle.export.retrieve",
        ))
    }

    async fn create_okf_import(
        &self,
        _context: KnowledgeAppRequestContext,
        _request: OkfBundleImportRequest,
    ) -> ApiResult<OkfBundleImportResult> {
        Err(ApiError::unsupported_operation("okf.bundle.import.create"))
    }

    async fn create_okf_lint_run(
        &self,
        _context: KnowledgeAppRequestContext,
        _request: OkfQualityRunRequest,
    ) -> ApiResult<OkfQualityRun> {
        Err(ApiError::unsupported_operation("okf.lintRuns.create"))
    }

    async fn list_browser(
        &self,
        _context: KnowledgeAppRequestContext,
        _request: ListKnowledgeBrowserRequest,
    ) -> ApiResult<KnowledgeBrowserListData> {
        Err(ApiError::unsupported_operation("spaces.browser.list"))
    }

    async fn create_retrieval(
        &self,
        _context: KnowledgeAppRequestContext,
        _request: KnowledgeRetrievalRequest,
    ) -> ApiResult<KnowledgeRetrievalResult> {
        Err(ApiError::unsupported_operation("retrievals.create"))
    }

    async fn retrieve_retrieval(
        &self,
        _context: KnowledgeAppRequestContext,
        _retrieval_id: u64,
    ) -> ApiResult<KnowledgeRetrievalResult> {
        Err(ApiError::unsupported_operation("retrievals.retrieve"))
    }

    async fn create_context_pack(
        &self,
        _context: KnowledgeAppRequestContext,
        _request: KnowledgeContextPackRequest,
    ) -> ApiResult<KnowledgeContextPack> {
        Err(ApiError::unsupported_operation("contextPacks.create"))
    }

    async fn create_agent_profile(
        &self,
        _context: KnowledgeAppRequestContext,
        _request: KnowledgeAgentProfileRequest,
    ) -> ApiResult<KnowledgeAgentProfile> {
        Err(ApiError::unsupported_operation("agentProfiles.create"))
    }

    async fn retrieve_agent_profile(
        &self,
        _context: KnowledgeAppRequestContext,
        _profile_id: u64,
    ) -> ApiResult<KnowledgeAgentProfile> {
        Err(ApiError::unsupported_operation("agentProfiles.retrieve"))
    }

    async fn update_agent_profile(
        &self,
        _context: KnowledgeAppRequestContext,
        _profile_id: u64,
        _request: KnowledgeAgentProfileRequest,
    ) -> ApiResult<KnowledgeAgentProfile> {
        Err(ApiError::unsupported_operation("agentProfiles.update"))
    }

    async fn delete_agent_profile(
        &self,
        _context: KnowledgeAppRequestContext,
        _profile_id: u64,
    ) -> ApiResult<()> {
        Err(ApiError::unsupported_operation("agentProfiles.delete"))
    }

    async fn list_agent_profile_bindings(
        &self,
        _context: KnowledgeAppRequestContext,
        _profile_id: u64,
    ) -> ApiResult<KnowledgeAgentBindingList> {
        Err(ApiError::unsupported_operation(
            "agentProfiles.bindings.list",
        ))
    }

    async fn create_agent_profile_binding(
        &self,
        _context: KnowledgeAppRequestContext,
        _profile_id: u64,
        _request: KnowledgeAgentBindingRequest,
    ) -> ApiResult<KnowledgeAgentBinding> {
        Err(ApiError::unsupported_operation(
            "agentProfiles.bindings.create",
        ))
    }

    async fn update_agent_profile_binding(
        &self,
        _context: KnowledgeAppRequestContext,
        _profile_id: u64,
        _binding_id: u64,
        _request: KnowledgeAgentBindingRequest,
    ) -> ApiResult<KnowledgeAgentBinding> {
        Err(ApiError::unsupported_operation(
            "agentProfiles.bindings.update",
        ))
    }

    async fn delete_agent_profile_binding(
        &self,
        _context: KnowledgeAppRequestContext,
        _profile_id: u64,
        _binding_id: u64,
    ) -> ApiResult<()> {
        Err(ApiError::unsupported_operation(
            "agentProfiles.bindings.delete",
        ))
    }

    async fn create_agent_profile_retrieval_preview(
        &self,
        _context: KnowledgeAppRequestContext,
        _profile_id: u64,
        _request: KnowledgeRetrievalRequest,
    ) -> ApiResult<KnowledgeRetrievalResult> {
        Err(ApiError::unsupported_operation(
            "agentProfiles.retrievalPreview.create",
        ))
    }

    async fn create_agent_chat(
        &self,
        _context: KnowledgeAppRequestContext,
        _profile_id: u64,
        _request: KnowledgeAgentChatRequest,
    ) -> ApiResult<KnowledgeAgentChatResponse> {
        Err(ApiError::unsupported_operation("agentProfiles.chat.create"))
    }

    async fn list_space_context_bindings(
        &self,
        _context: KnowledgeAppRequestContext,
        _space_id: u64,
        _cursor: Option<String>,
        _page_size: Option<u32>,
        _context_type: Option<KnowledgeContextType>,
    ) -> ApiResult<SdkWorkPageData<KnowledgeSpaceContextBinding>> {
        Err(ApiError::unsupported_operation(
            "spaces.contextBindings.list",
        ))
    }

    async fn create_space_context_binding(
        &self,
        _context: KnowledgeAppRequestContext,
        _space_id: u64,
        _request: CreateKnowledgeSpaceContextBindingRequest,
    ) -> ApiResult<KnowledgeSpaceContextBinding> {
        Err(ApiError::unsupported_operation(
            "spaces.contextBindings.create",
        ))
    }

    async fn retrieve_context_binding(
        &self,
        _context: KnowledgeAppRequestContext,
        _binding_id: u64,
    ) -> ApiResult<KnowledgeSpaceContextBinding> {
        Err(ApiError::unsupported_operation("contextBindings.retrieve"))
    }

    async fn update_context_binding(
        &self,
        _context: KnowledgeAppRequestContext,
        _binding_id: u64,
        _request: UpdateKnowledgeSpaceContextBindingRequest,
    ) -> ApiResult<KnowledgeSpaceContextBinding> {
        Err(ApiError::unsupported_operation("contextBindings.update"))
    }

    async fn delete_context_binding(
        &self,
        _context: KnowledgeAppRequestContext,
        _binding_id: u64,
    ) -> ApiResult<()> {
        Err(ApiError::unsupported_operation("contextBindings.delete"))
    }

    async fn list_wechat_official_accounts(
        &self,
        _context: KnowledgeAppRequestContext,
    ) -> ApiResult<KnowledgeWechatOfficialAccountList> {
        Err(ApiError::unsupported_operation(
            "wechat.officialAccounts.list",
        ))
    }

    async fn replace_wechat_official_accounts(
        &self,
        _context: KnowledgeAppRequestContext,
        _request: KnowledgeWechatReplaceOfficialAccountsRequest,
    ) -> ApiResult<KnowledgeWechatOfficialAccountList> {
        Err(ApiError::unsupported_operation(
            "wechat.officialAccounts.update",
        ))
    }

    async fn list_official_account_fan_tags(
        &self,
        _context: KnowledgeAppRequestContext,
        _account_id: String,
    ) -> ApiResult<KnowledgeWechatFanTagList> {
        Err(ApiError::unsupported_operation(
            "wechat.officialAccounts.fanTags.list",
        ))
    }

    async fn list_wechat_applets(
        &self,
        _context: KnowledgeAppRequestContext,
    ) -> ApiResult<KnowledgeWechatAppletList> {
        Err(ApiError::unsupported_operation("wechat.applets.list"))
    }

    async fn replace_wechat_applets(
        &self,
        _context: KnowledgeAppRequestContext,
        _request: KnowledgeWechatReplaceAppletsRequest,
    ) -> ApiResult<KnowledgeWechatAppletList> {
        Err(ApiError::unsupported_operation("wechat.applets.update"))
    }

    async fn publish_wechat_articles(
        &self,
        _context: KnowledgeAppRequestContext,
        _request: KnowledgeWechatArticlesPublishRequest,
    ) -> ApiResult<KnowledgeWechatOperationResult> {
        Err(ApiError::unsupported_operation("wechat.articles.publish"))
    }

    async fn preview_wechat_articles(
        &self,
        _context: KnowledgeAppRequestContext,
        _request: KnowledgeWechatArticlesPreviewRequest,
    ) -> ApiResult<KnowledgeWechatOperationResult> {
        Err(ApiError::unsupported_operation("wechat.articles.preview"))
    }

    async fn list_market_listings(
        &self,
        _context: KnowledgeAppRequestContext,
        _cursor: Option<String>,
        _page_size: Option<u32>,
    ) -> ApiResult<sdkwork_utils_rust::SdkWorkPageData<KnowledgeMarketCatalogItem>> {
        Err(ApiError::unsupported_operation("market.listings.list"))
    }

    async fn create_market_subscription(
        &self,
        _context: KnowledgeAppRequestContext,
        _request: KnowledgeMarketSubscriptionRequest,
    ) -> ApiResult<KnowledgeMarketSubscriptionResult> {
        Err(ApiError::unsupported_operation(
            "market.subscriptions.create",
        ))
    }

    async fn delete_market_subscription(
        &self,
        _context: KnowledgeAppRequestContext,
        _listing_id: u64,
    ) -> ApiResult<KnowledgeMarketSubscriptionResult> {
        Err(ApiError::unsupported_operation(
            "market.subscriptions.delete",
        ))
    }

    async fn create_media_task(
        &self,
        _context: KnowledgeAppRequestContext,
        _request: KnowledgeMediaTaskRequest,
    ) -> ApiResult<KnowledgeMediaTaskResult> {
        Err(ApiError::unsupported_operation("mediaTasks.create"))
    }
}
