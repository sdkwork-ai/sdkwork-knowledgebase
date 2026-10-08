use async_trait::async_trait;
use sdkwork_knowledgebase_contract::{
    context_binding::{
        CreateKnowledgeSpaceContextBindingRequest, KnowledgeSpaceContextBinding,
        UpdateKnowledgeSpaceContextBindingRequest,
    },
    CreateKnowledgeDocumentRequest, CreateKnowledgeDocumentVersionRequest,
    CreateKnowledgeSpaceRequest, GrantKnowledgeSpaceMemberRequest, IngestionJob,
    KnowledgeAgentBinding, KnowledgeAgentBindingRequest, KnowledgeAgentChatRequest,
    KnowledgeAgentChatResponse, KnowledgeAgentProfile, KnowledgeAgentProfileRequest,
    KnowledgeBrowserListData, KnowledgeContextPack, KnowledgeContextPackRequest,
    KnowledgeDocument, KnowledgeDocumentContent, KnowledgeDocumentVersion,
    KnowledgeDriveImportRequest, KnowledgeDriveImportResult, KnowledgeGitImportRequest,
    KnowledgeGitImportResult, KnowledgeGitSyncRequest, KnowledgeGitSyncResult,
    KnowledgeIngestRequest, KnowledgeMarketCatalogItem, KnowledgeMarketSubscriptionRequest,
    KnowledgeMarketSubscriptionResult, KnowledgeMediaTaskRequest, KnowledgeMediaTaskResult,
    KnowledgeOkfBundleFile, KnowledgeOkfConceptRevision, KnowledgeRetrievalRequest,
    KnowledgeRetrievalResult, KnowledgeSpace, KnowledgeSpaceMember,
    KnowledgeSpaceMemberSubjectType, KnowledgeWechatApplet, KnowledgeWechatAppletList,
    KnowledgeWechatArticlesPreviewRequest, KnowledgeWechatArticlesPublishRequest,
    KnowledgeWechatFanTag, KnowledgeWechatOfficialAccount, KnowledgeWechatOfficialAccountList,
    KnowledgeWechatOperationResult, KnowledgeWechatReplaceAppletsRequest,
    KnowledgeWechatReplaceOfficialAccountsRequest,
    ListKnowledgeBrowserRequest, OkfBundleExportRequest, OkfBundleImportRequest,
    OkfBundleImportResult, OkfConceptSummary, OkfConceptUpsertRequest, OkfContextPackRequest,
    OkfFileAnswerRequest, OkfIndexDocument, OkfLogDocument, OkfProfileDocument, OkfQualityRun,
    OkfQualityRunRequest, OkfQueryRequest, OkfQueryResult, UpdateKnowledgeSpaceRequest,
};
use sdkwork_intelligence_knowledgebase_service::wechat::{
    WechatCallbackReceipt, WechatCallbackReceiptRequest, WechatCallbackVerificationRequest,
};
use sdkwork_utils_rust::{SdkWorkCommandData, SdkWorkPageData};
use std::sync::Arc;

use crate::{
    ApiResult, KnowledgeAgentAppService, KnowledgeAppApi, KnowledgeAppRequestContext,
    KnowledgeBrowserApi, KnowledgeCommerceAppService, KnowledgeContextBindingAppService,
    KnowledgeDocumentAppService, KnowledgeDriveImportAppService, KnowledgeGitImportAppService,
    KnowledgeIngestAppService, KnowledgeOkfAppService, KnowledgeRetrievalAppService,
    KnowledgeSpaceAppService, KnowledgeWechatAppService,
};

pub struct BrowserOnlyAppApi {
    browser: Arc<dyn KnowledgeBrowserApi>,
}

impl BrowserOnlyAppApi {
    pub fn new(browser: Arc<dyn KnowledgeBrowserApi>) -> Self {
        Self { browser }
    }
}

#[async_trait]
impl KnowledgeAppApi for BrowserOnlyAppApi {
    async fn list_browser(
        &self,
        context: KnowledgeAppRequestContext,
        request: ListKnowledgeBrowserRequest,
    ) -> ApiResult<KnowledgeBrowserListData> {
        self.browser.list_browser(context, request).await
    }
}

pub struct RetrievalOnlyAppApi {
    retrieval: Arc<dyn KnowledgeRetrievalAppService>,
}

impl RetrievalOnlyAppApi {
    pub fn new(retrieval: Arc<dyn KnowledgeRetrievalAppService>) -> Self {
        Self { retrieval }
    }
}

#[async_trait]
impl KnowledgeAppApi for RetrievalOnlyAppApi {
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
}

pub struct AgentOnlyAppApi {
    agent: Arc<dyn KnowledgeAgentAppService>,
}

impl AgentOnlyAppApi {
    pub fn new(agent: Arc<dyn KnowledgeAgentAppService>) -> Self {
        Self { agent }
    }
}

#[async_trait]
impl KnowledgeAppApi for AgentOnlyAppApi {
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
    ) -> ApiResult<SdkWorkPageData<KnowledgeAgentBinding>> {
        let bindings = self.agent.list_bindings(context, profile_id).await?;
        Ok(crate::pagination::fixed_list_page_data(bindings.items))
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

pub struct AgentAndRetrievalAppApi {
    agent: Arc<dyn KnowledgeAgentAppService>,
    retrieval: Arc<dyn KnowledgeRetrievalAppService>,
}

impl AgentAndRetrievalAppApi {
    pub fn new(
        agent: Arc<dyn KnowledgeAgentAppService>,
        retrieval: Arc<dyn KnowledgeRetrievalAppService>,
    ) -> Self {
        Self { agent, retrieval }
    }
}

#[async_trait]
impl KnowledgeAppApi for AgentAndRetrievalAppApi {
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
    ) -> ApiResult<SdkWorkPageData<KnowledgeAgentBinding>> {
        let bindings = self.agent.list_bindings(context, profile_id).await?;
        Ok(crate::pagination::fixed_list_page_data(bindings.items))
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

pub struct FullAppApi {
    space: Arc<dyn KnowledgeSpaceAppService>,
    wiki: Arc<dyn crate::KnowledgeWikiPublicationAppService>,
    group_launch: Arc<dyn crate::KnowledgeGroupLaunchAppService>,
    drive_import: Arc<dyn KnowledgeDriveImportAppService>,
    git_import: Arc<dyn KnowledgeGitImportAppService>,
    ingest: Arc<dyn KnowledgeIngestAppService>,
    document: Arc<dyn KnowledgeDocumentAppService>,
    okf: Arc<dyn KnowledgeOkfAppService>,
    browser: Arc<dyn KnowledgeBrowserApi>,
    retrieval: Arc<dyn KnowledgeRetrievalAppService>,
    agent: Arc<dyn KnowledgeAgentAppService>,
    context_binding: Arc<dyn KnowledgeContextBindingAppService>,
    wechat: Arc<dyn KnowledgeWechatAppService>,
    commerce: Arc<dyn KnowledgeCommerceAppService>,
}

impl FullAppApi {
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        space: Arc<dyn KnowledgeSpaceAppService>,
        wiki: Arc<dyn crate::KnowledgeWikiPublicationAppService>,
        group_launch: Arc<dyn crate::KnowledgeGroupLaunchAppService>,
        drive_import: Arc<dyn KnowledgeDriveImportAppService>,
        git_import: Arc<dyn KnowledgeGitImportAppService>,
        ingest: Arc<dyn KnowledgeIngestAppService>,
        document: Arc<dyn KnowledgeDocumentAppService>,
        okf: Arc<dyn KnowledgeOkfAppService>,
        browser: Arc<dyn KnowledgeBrowserApi>,
        retrieval: Arc<dyn KnowledgeRetrievalAppService>,
        agent: Arc<dyn KnowledgeAgentAppService>,
        context_binding: Arc<dyn KnowledgeContextBindingAppService>,
        wechat: Arc<dyn KnowledgeWechatAppService>,
        commerce: Arc<dyn KnowledgeCommerceAppService>,
    ) -> Self {
        Self {
            space,
            wiki,
            group_launch,
            drive_import,
            git_import,
            ingest,
            document,
            okf,
            browser,
            retrieval,
            agent,
            context_binding,
            wechat,
            commerce,
        }
    }
}

#[async_trait]
impl KnowledgeAppApi for FullAppApi {
    async fn create_space(
        &self,
        context: KnowledgeAppRequestContext,
        request: CreateKnowledgeSpaceRequest,
    ) -> ApiResult<KnowledgeSpace> {
        self.space.create_space(context, request).await
    }

    async fn retrieve_space(
        &self,
        context: KnowledgeAppRequestContext,
        space_id: u64,
    ) -> ApiResult<KnowledgeSpace> {
        self.space.retrieve_space(context, space_id).await
    }

    async fn update_space(
        &self,
        context: KnowledgeAppRequestContext,
        space_id: u64,
        request: UpdateKnowledgeSpaceRequest,
    ) -> ApiResult<KnowledgeSpace> {
        self.space.update_space(context, space_id, request).await
    }

    async fn delete_space(
        &self,
        context: KnowledgeAppRequestContext,
        space_id: u64,
    ) -> ApiResult<()> {
        self.space.delete_space(context, space_id).await
    }

    async fn retrieve_wiki_publication(
        &self,
        context: KnowledgeAppRequestContext,
        space_id: u64,
    ) -> ApiResult<sdkwork_knowledgebase_contract::KnowledgeWikiPublication> {
        self.wiki.retrieve_wiki_publication(context, space_id).await
    }

    async fn activate_wiki_publication(
        &self,
        context: KnowledgeAppRequestContext,
        space_id: u64,
        request: sdkwork_knowledgebase_contract::KnowledgeWikiPublicationVersionCommandRequest,
    ) -> ApiResult<sdkwork_knowledgebase_contract::KnowledgeWikiPublication> {
        self.wiki
            .activate_wiki_publication(context, space_id, request)
            .await
    }

    async fn pause_wiki_publication(
        &self,
        context: KnowledgeAppRequestContext,
        space_id: u64,
        request: sdkwork_knowledgebase_contract::KnowledgeWikiPublicationVersionCommandRequest,
    ) -> ApiResult<sdkwork_knowledgebase_contract::KnowledgeWikiPublication> {
        self.wiki
            .pause_wiki_publication(context, space_id, request)
            .await
    }

    async fn publish_wiki_source_file(
        &self,
        context: KnowledgeAppRequestContext,
        space_id: u64,
        source_file_uuid: String,
        request: sdkwork_knowledgebase_contract::PublishKnowledgeWikiSourceFileRequest,
    ) -> ApiResult<sdkwork_knowledgebase_contract::KnowledgeWikiSourceFileCommandResult> {
        self.wiki
            .publish_wiki_source_file(context, space_id, source_file_uuid, request)
            .await
    }

    async fn unpublish_wiki_source_file(
        &self,
        context: KnowledgeAppRequestContext,
        space_id: u64,
        source_file_uuid: String,
        request: sdkwork_knowledgebase_contract::KnowledgeWikiSourceFileVersionCommandRequest,
    ) -> ApiResult<sdkwork_knowledgebase_contract::KnowledgeWikiSourceFileCommandResult> {
        self.wiki
            .unpublish_wiki_source_file(context, space_id, source_file_uuid, request)
            .await
    }

    async fn change_wiki_source_file_visibility(
        &self,
        context: KnowledgeAppRequestContext,
        space_id: u64,
        source_file_uuid: String,
        request: sdkwork_knowledgebase_contract::ChangeKnowledgeWikiSourceFileVisibilityRequest,
    ) -> ApiResult<sdkwork_knowledgebase_contract::KnowledgeWikiSourceFileCommandResult> {
        self.wiki
            .change_wiki_source_file_visibility(context, space_id, source_file_uuid, request)
            .await
    }

    async fn list_space_members(
        &self,
        context: KnowledgeAppRequestContext,
        space_id: u64,
        cursor: Option<String>,
        page_size: Option<u32>,
    ) -> ApiResult<SdkWorkPageData<KnowledgeSpaceMember>> {
        self.space
            .list_space_members(context, space_id, cursor, page_size)
            .await
    }

    async fn grant_space_member(
        &self,
        context: KnowledgeAppRequestContext,
        space_id: u64,
        request: GrantKnowledgeSpaceMemberRequest,
    ) -> ApiResult<SdkWorkCommandData> {
        self.space
            .grant_space_member(context, space_id, request)
            .await
    }

    async fn revoke_space_member(
        &self,
        context: KnowledgeAppRequestContext,
        space_id: u64,
        subject_type: KnowledgeSpaceMemberSubjectType,
        subject_id: String,
    ) -> ApiResult<()> {
        self.space
            .revoke_space_member(context, space_id, subject_type, subject_id)
            .await
    }

    async fn consume_group_launch_ticket(
        &self,
        context: KnowledgeAppRequestContext,
        request: sdkwork_knowledgebase_contract::group_space::ConsumeGroupKnowledgebaseLaunchTicketRequest,
    ) -> ApiResult<sdkwork_knowledgebase_contract::group_space::GroupKnowledgebaseLaunchTarget>
    {
        self.group_launch
            .consume_group_launch_ticket(context, request)
            .await
    }

    async fn create_drive_import(
        &self,
        context: KnowledgeAppRequestContext,
        request: KnowledgeDriveImportRequest,
    ) -> ApiResult<KnowledgeDriveImportResult> {
        self.drive_import
            .import_drive_object(context, request)
            .await
    }

    async fn create_git_import(
        &self,
        context: KnowledgeAppRequestContext,
        request: KnowledgeGitImportRequest,
    ) -> ApiResult<KnowledgeGitImportResult> {
        self.git_import.create_git_import(context, request).await
    }

    async fn create_git_sync(
        &self,
        context: KnowledgeAppRequestContext,
        request: KnowledgeGitSyncRequest,
    ) -> ApiResult<KnowledgeGitSyncResult> {
        self.git_import.create_git_sync(context, request).await
    }

    async fn create_ingest(
        &self,
        context: KnowledgeAppRequestContext,
        request: KnowledgeIngestRequest,
    ) -> ApiResult<IngestionJob> {
        self.ingest.create_ingest(context, request).await
    }

    async fn retrieve_ingest(
        &self,
        context: KnowledgeAppRequestContext,
        ingest_id: u64,
    ) -> ApiResult<IngestionJob> {
        self.ingest.retrieve_ingest(context, ingest_id).await
    }

    async fn list_documents(
        &self,
        context: KnowledgeAppRequestContext,
        space_id: u64,
        cursor: Option<String>,
        page_size: Option<u32>,
    ) -> ApiResult<SdkWorkPageData<KnowledgeDocument>> {
        self.document
            .list_documents(context, space_id, cursor, page_size)
            .await
    }

    async fn create_document(
        &self,
        context: KnowledgeAppRequestContext,
        request: CreateKnowledgeDocumentRequest,
    ) -> ApiResult<KnowledgeDocument> {
        self.document.create_document(context, request).await
    }

    async fn retrieve_document(
        &self,
        context: KnowledgeAppRequestContext,
        document_id: u64,
    ) -> ApiResult<KnowledgeDocument> {
        self.document.retrieve_document(context, document_id).await
    }

    async fn update_document(
        &self,
        context: KnowledgeAppRequestContext,
        document_id: u64,
        request: CreateKnowledgeDocumentRequest,
    ) -> ApiResult<KnowledgeDocument> {
        self.document
            .update_document(context, document_id, request)
            .await
    }

    async fn delete_document(
        &self,
        context: KnowledgeAppRequestContext,
        document_id: u64,
    ) -> ApiResult<()> {
        self.document.delete_document(context, document_id).await
    }

    async fn list_document_versions(
        &self,
        context: KnowledgeAppRequestContext,
        document_id: u64,
        cursor: Option<String>,
        page_size: Option<u32>,
    ) -> ApiResult<SdkWorkPageData<KnowledgeDocumentVersion>> {
        self.document
            .list_document_versions(context, document_id, cursor, page_size)
            .await
    }

    async fn create_document_version(
        &self,
        context: KnowledgeAppRequestContext,
        document_id: u64,
        request: CreateKnowledgeDocumentVersionRequest,
    ) -> ApiResult<KnowledgeDocumentVersion> {
        self.document
            .create_document_version(context, document_id, request)
            .await
    }

    async fn retrieve_document_content(
        &self,
        context: KnowledgeAppRequestContext,
        document_id: u64,
    ) -> ApiResult<KnowledgeDocumentContent> {
        self.document
            .retrieve_document_content(context, document_id)
            .await
    }

    async fn list_okf_concepts(
        &self,
        context: KnowledgeAppRequestContext,
        space_id: u64,
        cursor: Option<String>,
        page_size: Option<u32>,
    ) -> ApiResult<SdkWorkPageData<OkfConceptSummary>> {
        self.okf
            .list_okf_concepts(context, space_id, cursor, page_size)
            .await
    }

    async fn retrieve_okf_concept(
        &self,
        context: KnowledgeAppRequestContext,
        concept_row_id: u64,
    ) -> ApiResult<OkfConceptSummary> {
        self.okf.retrieve_okf_concept(context, concept_row_id).await
    }

    async fn list_okf_concept_revisions(
        &self,
        context: KnowledgeAppRequestContext,
        concept_row_id: u64,
        cursor: Option<String>,
        page_size: Option<u32>,
    ) -> ApiResult<SdkWorkPageData<KnowledgeOkfConceptRevision>> {
        self.okf
            .list_okf_concept_revisions(context, concept_row_id, cursor, page_size)
            .await
    }

    async fn upsert_okf_concept(
        &self,
        context: KnowledgeAppRequestContext,
        request: OkfConceptUpsertRequest,
    ) -> ApiResult<OkfConceptSummary> {
        self.okf.upsert_okf_concept(context, request).await
    }

    async fn delete_okf_concept(
        &self,
        context: KnowledgeAppRequestContext,
        concept_row_id: u64,
    ) -> ApiResult<()> {
        self.okf.delete_okf_concept(context, concept_row_id).await
    }

    async fn retrieve_okf_index(
        &self,
        context: KnowledgeAppRequestContext,
        space_id: u64,
    ) -> ApiResult<OkfIndexDocument> {
        self.okf.retrieve_okf_index(context, space_id).await
    }

    async fn retrieve_okf_log(
        &self,
        context: KnowledgeAppRequestContext,
        space_id: u64,
    ) -> ApiResult<OkfLogDocument> {
        self.okf.retrieve_okf_log(context, space_id).await
    }

    async fn retrieve_okf_schema(
        &self,
        context: KnowledgeAppRequestContext,
        space_id: u64,
    ) -> ApiResult<OkfProfileDocument> {
        self.okf.retrieve_okf_schema(context, space_id).await
    }

    async fn create_okf_query(
        &self,
        context: KnowledgeAppRequestContext,
        request: OkfQueryRequest,
    ) -> ApiResult<OkfQueryResult> {
        self.okf.create_okf_query(context, request).await
    }

    async fn file_okf_query_answer(
        &self,
        context: KnowledgeAppRequestContext,
        query_id: u64,
        request: OkfFileAnswerRequest,
    ) -> ApiResult<OkfQueryResult> {
        self.okf
            .file_okf_query_answer(context, query_id, request)
            .await
    }

    async fn create_okf_context_pack(
        &self,
        context: KnowledgeAppRequestContext,
        request: OkfContextPackRequest,
    ) -> ApiResult<KnowledgeOkfBundleFile> {
        self.okf.create_okf_context_pack(context, request).await
    }

    async fn create_okf_export(
        &self,
        context: KnowledgeAppRequestContext,
        request: OkfBundleExportRequest,
    ) -> ApiResult<KnowledgeOkfBundleFile> {
        self.okf.create_okf_export(context, request).await
    }

    async fn retrieve_okf_export(
        &self,
        context: KnowledgeAppRequestContext,
        export_id: u64,
    ) -> ApiResult<KnowledgeOkfBundleFile> {
        self.okf.retrieve_okf_export(context, export_id).await
    }

    async fn create_okf_import(
        &self,
        context: KnowledgeAppRequestContext,
        request: OkfBundleImportRequest,
    ) -> ApiResult<OkfBundleImportResult> {
        self.okf.create_okf_import(context, request).await
    }

    async fn create_okf_lint_run(
        &self,
        context: KnowledgeAppRequestContext,
        request: OkfQualityRunRequest,
    ) -> ApiResult<OkfQualityRun> {
        self.okf.create_okf_lint_run(context, request).await
    }

    async fn list_browser(
        &self,
        context: KnowledgeAppRequestContext,
        request: ListKnowledgeBrowserRequest,
    ) -> ApiResult<KnowledgeBrowserListData> {
        self.browser.list_browser(context, request).await
    }

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
    ) -> ApiResult<SdkWorkPageData<KnowledgeAgentBinding>> {
        let bindings = self.agent.list_bindings(context, profile_id).await?;
        Ok(crate::pagination::fixed_list_page_data(bindings.items))
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

    async fn list_space_context_bindings(
        &self,
        context: KnowledgeAppRequestContext,
        space_id: u64,
        cursor: Option<String>,
        page_size: Option<u32>,
        context_type: Option<sdkwork_knowledgebase_contract::context_binding::KnowledgeContextType>,
    ) -> ApiResult<
        sdkwork_utils_rust::SdkWorkPageData<
            sdkwork_knowledgebase_contract::context_binding::KnowledgeSpaceContextBinding,
        >,
    > {
        self.context_binding
            .list_space_context_bindings(context, space_id, cursor, page_size, context_type)
            .await
    }

    async fn create_space_context_binding(
        &self,
        context: KnowledgeAppRequestContext,
        space_id: u64,
        request: CreateKnowledgeSpaceContextBindingRequest,
    ) -> ApiResult<KnowledgeSpaceContextBinding> {
        self.context_binding
            .create_space_context_binding(context, space_id, request)
            .await
    }

    async fn retrieve_context_binding(
        &self,
        context: KnowledgeAppRequestContext,
        binding_id: u64,
    ) -> ApiResult<KnowledgeSpaceContextBinding> {
        self.context_binding
            .retrieve_context_binding(context, binding_id)
            .await
    }

    async fn update_context_binding(
        &self,
        context: KnowledgeAppRequestContext,
        binding_id: u64,
        request: UpdateKnowledgeSpaceContextBindingRequest,
    ) -> ApiResult<KnowledgeSpaceContextBinding> {
        self.context_binding
            .update_context_binding(context, binding_id, request)
            .await
    }

    async fn delete_context_binding(
        &self,
        context: KnowledgeAppRequestContext,
        binding_id: u64,
    ) -> ApiResult<()> {
        self.context_binding
            .delete_context_binding(context, binding_id)
            .await
    }

    async fn list_wechat_official_accounts(
        &self,
        context: KnowledgeAppRequestContext,
    ) -> ApiResult<SdkWorkPageData<KnowledgeWechatOfficialAccount>> {
        let accounts = self.wechat.list_official_accounts(context).await?;
        Ok(crate::pagination::fixed_list_page_data(accounts.accounts))
    }

    async fn replace_wechat_official_accounts(
        &self,
        context: KnowledgeAppRequestContext,
        request: KnowledgeWechatReplaceOfficialAccountsRequest,
    ) -> ApiResult<KnowledgeWechatOfficialAccountList> {
        self.wechat
            .replace_official_accounts(context, request)
            .await
    }

    async fn list_official_account_fan_tags(
        &self,
        context: KnowledgeAppRequestContext,
        account_id: String,
    ) -> ApiResult<SdkWorkPageData<KnowledgeWechatFanTag>> {
        let fan_tags = self
            .wechat
            .list_official_account_fan_tags(context, account_id)
            .await?;
        Ok(crate::pagination::fixed_list_page_data(fan_tags.tags))
    }

    async fn list_wechat_applets(
        &self,
        context: KnowledgeAppRequestContext,
    ) -> ApiResult<SdkWorkPageData<KnowledgeWechatApplet>> {
        let applets = self.wechat.list_applets(context).await?;
        Ok(crate::pagination::fixed_list_page_data(applets.applets))
    }

    async fn replace_wechat_applets(
        &self,
        context: KnowledgeAppRequestContext,
        request: KnowledgeWechatReplaceAppletsRequest,
    ) -> ApiResult<KnowledgeWechatAppletList> {
        self.wechat.replace_applets(context, request).await
    }

    async fn publish_wechat_articles(
        &self,
        context: KnowledgeAppRequestContext,
        request: KnowledgeWechatArticlesPublishRequest,
    ) -> ApiResult<KnowledgeWechatOperationResult> {
        self.wechat.publish_articles(context, request).await
    }

    async fn preview_wechat_articles(
        &self,
        context: KnowledgeAppRequestContext,
        request: KnowledgeWechatArticlesPreviewRequest,
    ) -> ApiResult<KnowledgeWechatOperationResult> {
        self.wechat.preview_articles(context, request).await
    }

    async fn verify_wechat_callback(
        &self,
        request: WechatCallbackVerificationRequest,
    ) -> ApiResult<String> {
        self.wechat.verify_wechat_callback(request).await
    }

    async fn receive_wechat_callback(
        &self,
        request: WechatCallbackReceiptRequest,
    ) -> ApiResult<WechatCallbackReceipt> {
        self.wechat.receive_wechat_callback(request).await
    }

    async fn list_market_listings(
        &self,
        context: KnowledgeAppRequestContext,
        cursor: Option<String>,
        page_size: Option<u32>,
    ) -> ApiResult<sdkwork_utils_rust::SdkWorkPageData<KnowledgeMarketCatalogItem>> {
        self.commerce
            .list_market_listings(context, cursor, page_size)
            .await
    }

    async fn create_market_subscription(
        &self,
        context: KnowledgeAppRequestContext,
        request: KnowledgeMarketSubscriptionRequest,
    ) -> ApiResult<KnowledgeMarketSubscriptionResult> {
        self.commerce
            .create_market_subscription(context, request)
            .await
    }

    async fn delete_market_subscription(
        &self,
        context: KnowledgeAppRequestContext,
        listing_id: u64,
    ) -> ApiResult<()> {
        self.commerce
            .delete_market_subscription(context, listing_id)
            .await
    }

    async fn create_media_task(
        &self,
        context: KnowledgeAppRequestContext,
        request: KnowledgeMediaTaskRequest,
    ) -> ApiResult<KnowledgeMediaTaskResult> {
        self.commerce.create_media_task(context, request).await
    }
}
