use std::sync::Arc;

use crate::api::base::{RequestHeaders};
use crate::api::paths::app_path;
use crate::api::paths::append_query_string;
use crate::http::{SdkworkError, SdkworkHttpClient};
use crate::models::{ChangeKnowledgeWikiSourceFileVisibilityRequest, ConsumeGroupKnowledgebaseLaunchTicketRequest, CreateKnowledgeDocumentRequest, CreateKnowledgeDocumentVersionRequest, CreateKnowledgeSpaceContextBindingRequest, CreateKnowledgeSpaceRequest, GrantKnowledgeSpaceMemberRequest, GroupKnowledgebaseLaunchTarget, IngestionJob, KnowledgeAgentBinding, KnowledgeAgentBindingList, KnowledgeAgentBindingRequest, KnowledgeAgentChatRequest, KnowledgeAgentChatResponse, KnowledgeAgentProfile, KnowledgeAgentProfileRequest, KnowledgeBrowserListData, KnowledgeContextPack, KnowledgeContextPackRequest, KnowledgeDocument, KnowledgeDocumentContent, KnowledgeDocumentVersion, KnowledgeDriveImportRequest, KnowledgeDriveImportResult, KnowledgeGitImportRequest, KnowledgeGitImportResult, KnowledgeGitSyncRequest, KnowledgeGitSyncResult, KnowledgeIngestRequest, KnowledgeMarketSubscriptionRequest, KnowledgeMarketSubscriptionResult, KnowledgeMediaTaskRequest, KnowledgeMediaTaskResult, KnowledgeOkfBundleFile, KnowledgeOkfConceptRevisionList, KnowledgeRetrievalRequest, KnowledgeRetrievalResult, KnowledgeSpace, KnowledgeSpaceContextBinding, KnowledgeWechatAppletList, KnowledgeWechatArticlesPreviewRequest, KnowledgeWechatArticlesPublishRequest, KnowledgeWechatOfficialAccountList, KnowledgeWechatOperationResult, KnowledgeWechatReplaceAppletsRequest, KnowledgeWechatReplaceOfficialAccountsRequest, KnowledgeWikiPublication, KnowledgeWikiPublicationVersionCommandRequest, KnowledgeWikiSourceFileCommandResult, KnowledgeWikiSourceFileVersionCommandRequest, OkfBundleExportRequest, OkfBundleImportRequest, OkfBundleImportResult, OkfConceptSummary, OkfConceptSummaryList, OkfConceptUpsertRequest, OkfContextPackRequest, OkfFileAnswerRequest, OkfIndexDocument, OkfLogDocument, OkfProfileDocument, OkfQualityRun, OkfQualityRunRequest, OkfQueryRequest, OkfQueryResult, PublishKnowledgeWikiSourceFileRequest, SdkWorkCommandData, UpdateKnowledgeSpaceContextBindingRequest, UpdateKnowledgeSpaceRequest};

#[derive(Clone)]
pub struct KnowledgeApi {
    client: Arc<SdkworkHttpClient>,
}

impl KnowledgeApi {
    pub fn new(client: Arc<SdkworkHttpClient>) -> Self {
        Self { client }
    }

    /// Consume a group knowledgebase launch ticket
    pub async fn group_launches_consume(&self, body: &ConsumeGroupKnowledgebaseLaunchTicketRequest, idempotency_key: &str) -> Result<GroupKnowledgebaseLaunchTarget, SdkworkError> {
        let path = app_path(&"/knowledge/group_launches/consume".to_string());
        let headers = build_request_headers(
            &[
                ("Idempotency-Key", HeaderParameterSpec::new(idempotency_key, "simple", false, None)),
            ],
            &[],
        );
        self.client.post(&path, Some(body), None, headers.as_ref(), Some("application/json")).await
    }

    /// Create a knowledge space
    pub async fn spaces_create(&self, body: &CreateKnowledgeSpaceRequest) -> Result<KnowledgeSpace, SdkworkError> {
        let path = app_path(&"/knowledge/spaces".to_string());
        self.client.post(&path, Some(body), None, None, Some("application/json")).await
    }

    /// Retrieve a knowledge space
    pub async fn spaces_retrieve(&self, space_id: &str) -> Result<KnowledgeSpace, SdkworkError> {
        let path = app_path(&format!("/knowledge/spaces/{}", serialize_path_parameter(space_id, PathParameterSpec::new("spaceId", "simple", false))));
        self.client.get(&path, None, None).await
    }

    /// Update a knowledge space
    pub async fn spaces_update(&self, space_id: &str, body: &UpdateKnowledgeSpaceRequest) -> Result<KnowledgeSpace, SdkworkError> {
        let path = app_path(&format!("/knowledge/spaces/{}", serialize_path_parameter(space_id, PathParameterSpec::new("spaceId", "simple", false))));
        self.client.patch(&path, Some(body), None, None, Some("application/json")).await
    }

    /// Delete a knowledge space
    pub async fn spaces_delete(&self, space_id: &str) -> Result<(), SdkworkError> {
        let path = app_path(&format!("/knowledge/spaces/{}", serialize_path_parameter(space_id, PathParameterSpec::new("spaceId", "simple", false))));
        self.client.delete(&path, None, None).await
    }

    /// Import a drive object into knowledgebase
    pub async fn drive_imports_create(&self, body: &KnowledgeDriveImportRequest) -> Result<KnowledgeDriveImportResult, SdkworkError> {
        let path = app_path(&"/knowledge/drive_imports".to_string());
        self.client.post(&path, Some(body), None, None, Some("application/json")).await
    }

    /// Import a Git repository into knowledgebase
    pub async fn git_imports_create(&self, body: &KnowledgeGitImportRequest) -> Result<KnowledgeGitImportResult, SdkworkError> {
        let path = app_path(&"/knowledge/git_imports".to_string());
        self.client.post(&path, Some(body), None, None, Some("application/json")).await
    }

    /// Create an ingestion job
    pub async fn ingests_create(&self, body: &KnowledgeIngestRequest) -> Result<IngestionJob, SdkworkError> {
        let path = app_path(&"/knowledge/ingests".to_string());
        self.client.post(&path, Some(body), None, None, Some("application/json")).await
    }

    /// Retrieve an ingestion job
    pub async fn ingests_retrieve(&self, ingest_id: &str) -> Result<IngestionJob, SdkworkError> {
        let path = app_path(&format!("/knowledge/ingests/{}", serialize_path_parameter(ingest_id, PathParameterSpec::new("ingestId", "simple", false))));
        self.client.get(&path, None, None).await
    }

    /// List knowledge documents
    pub async fn documents_list(&self, space_id: &str, cursor: Option<&str>, page_size: Option<i64>) -> Result<serde_json::Value, SdkworkError> {
        let query = build_query_string(&[
            QueryParameterSpec::new("space_id", space_id, "form", true, false, None),
            QueryParameterSpec::new("cursor", cursor, "form", true, false, None),
            QueryParameterSpec::new("page_size", page_size, "form", true, false, None),
        ]);
        let path = append_query_string(app_path(&"/knowledge/documents".to_string()), &query);
        self.client.get(&path, None, None).await
    }

    /// Create a knowledge document
    pub async fn documents_create(&self, body: &CreateKnowledgeDocumentRequest) -> Result<KnowledgeDocument, SdkworkError> {
        let path = app_path(&"/knowledge/documents".to_string());
        self.client.post(&path, Some(body), None, None, Some("application/json")).await
    }

    /// Retrieve a knowledge document
    pub async fn documents_retrieve(&self, document_id: &str) -> Result<KnowledgeDocument, SdkworkError> {
        let path = app_path(&format!("/knowledge/documents/{}", serialize_path_parameter(document_id, PathParameterSpec::new("documentId", "simple", false))));
        self.client.get(&path, None, None).await
    }

    /// Update a knowledge document
    pub async fn documents_update(&self, document_id: &str, body: &CreateKnowledgeDocumentRequest) -> Result<KnowledgeDocument, SdkworkError> {
        let path = app_path(&format!("/knowledge/documents/{}", serialize_path_parameter(document_id, PathParameterSpec::new("documentId", "simple", false))));
        self.client.patch(&path, Some(body), None, None, Some("application/json")).await
    }

    /// Delete a knowledge document
    pub async fn documents_delete(&self, document_id: &str) -> Result<(), SdkworkError> {
        let path = app_path(&format!("/knowledge/documents/{}", serialize_path_parameter(document_id, PathParameterSpec::new("documentId", "simple", false))));
        self.client.delete(&path, None, None).await
    }

    /// Retrieve authoritative knowledge document content
    pub async fn documents_content_list(&self, document_id: &str) -> Result<KnowledgeDocumentContent, SdkworkError> {
        let path = app_path(&format!("/knowledge/documents/{}/content", serialize_path_parameter(document_id, PathParameterSpec::new("documentId", "simple", false))));
        self.client.get(&path, None, None).await
    }

    /// List document versions
    pub async fn documents_versions_list(&self, document_id: &str, cursor: Option<&str>, page_size: Option<i64>) -> Result<serde_json::Value, SdkworkError> {
        let query = build_query_string(&[
            QueryParameterSpec::new("cursor", cursor, "form", true, false, None),
            QueryParameterSpec::new("page_size", page_size, "form", true, false, None),
        ]);
        let path = append_query_string(app_path(&format!("/knowledge/documents/{}/versions", serialize_path_parameter(document_id, PathParameterSpec::new("documentId", "simple", false)))), &query);
        self.client.get(&path, None, None).await
    }

    /// Create a document version
    pub async fn documents_versions_create(&self, document_id: &str, body: &CreateKnowledgeDocumentVersionRequest) -> Result<KnowledgeDocumentVersion, SdkworkError> {
        let path = app_path(&format!("/knowledge/documents/{}/versions", serialize_path_parameter(document_id, PathParameterSpec::new("documentId", "simple", false))));
        self.client.post(&path, Some(body), None, None, Some("application/json")).await
    }

    /// List OKF concepts
    pub async fn okf_concepts_list(&self, space_id: &str, cursor: Option<&str>, page_size: Option<i64>) -> Result<OkfConceptSummaryList, SdkworkError> {
        let query = build_query_string(&[
            QueryParameterSpec::new("space_id", space_id, "form", true, false, None),
            QueryParameterSpec::new("cursor", cursor, "form", true, false, None),
            QueryParameterSpec::new("page_size", page_size, "form", true, false, None),
        ]);
        let path = append_query_string(app_path(&"/knowledge/okf/concepts".to_string()), &query);
        self.client.get(&path, None, None).await
    }

    /// Retrieve an OKF concept
    pub async fn okf_concepts_retrieve(&self, concept_id: &str) -> Result<OkfConceptSummary, SdkworkError> {
        let path = app_path(&format!("/knowledge/okf/concepts/{}", serialize_path_parameter(concept_id, PathParameterSpec::new("conceptId", "simple", false))));
        self.client.get(&path, None, None).await
    }

    /// Delete an OKF concept
    pub async fn okf_concepts_delete(&self, concept_id: &str) -> Result<(), SdkworkError> {
        let path = app_path(&format!("/knowledge/okf/concepts/{}", serialize_path_parameter(concept_id, PathParameterSpec::new("conceptId", "simple", false))));
        self.client.delete(&path, None, None).await
    }

    /// List OKF concept revisions
    pub async fn okf_concepts_revisions_list(&self, concept_id: &str, cursor: Option<&str>, page_size: Option<i64>) -> Result<KnowledgeOkfConceptRevisionList, SdkworkError> {
        let query = build_query_string(&[
            QueryParameterSpec::new("cursor", cursor, "form", true, false, None),
            QueryParameterSpec::new("page_size", page_size, "form", true, false, None),
        ]);
        let path = append_query_string(app_path(&format!("/knowledge/okf/concepts/{}/revisions", serialize_path_parameter(concept_id, PathParameterSpec::new("conceptId", "simple", false)))), &query);
        self.client.get(&path, None, None).await
    }

    /// Retrieve the OKF bundle index
    pub async fn okf_bundle_index_list(&self, space_id: &str) -> Result<OkfIndexDocument, SdkworkError> {
        let query = build_query_string(&[
            QueryParameterSpec::new("space_id", space_id, "form", true, false, None),
        ]);
        let path = append_query_string(app_path(&"/knowledge/okf/index".to_string()), &query);
        self.client.get(&path, None, None).await
    }

    /// Retrieve the OKF bundle log
    pub async fn okf_bundle_log_list(&self, space_id: &str) -> Result<OkfLogDocument, SdkworkError> {
        let query = build_query_string(&[
            QueryParameterSpec::new("space_id", space_id, "form", true, false, None),
        ]);
        let path = append_query_string(app_path(&"/knowledge/okf/log".to_string()), &query);
        self.client.get(&path, None, None).await
    }

    /// Retrieve the OKF bundle profile
    pub async fn okf_bundle_profile_list(&self, space_id: &str) -> Result<OkfProfileDocument, SdkworkError> {
        let query = build_query_string(&[
            QueryParameterSpec::new("space_id", space_id, "form", true, false, None),
        ]);
        let path = append_query_string(app_path(&"/knowledge/okf/profile".to_string()), &query);
        self.client.get(&path, None, None).await
    }

    /// Create an OKF query
    pub async fn okf_queries_create(&self, body: &OkfQueryRequest) -> Result<OkfQueryResult, SdkworkError> {
        let path = app_path(&"/knowledge/okf/queries".to_string());
        self.client.post(&path, Some(body), None, None, Some("application/json")).await
    }

    /// File an answer for an OKF query
    pub async fn okf_queries_file_answer(&self, query_id: &str, body: &OkfFileAnswerRequest) -> Result<OkfQueryResult, SdkworkError> {
        let path = app_path(&format!("/knowledge/okf/queries/{}/file_answer", serialize_path_parameter(query_id, PathParameterSpec::new("queryId", "simple", false))));
        self.client.post(&path, Some(body), None, None, Some("application/json")).await
    }

    /// Create an OKF context pack
    pub async fn okf_context_packs_create(&self, body: &OkfContextPackRequest) -> Result<KnowledgeOkfBundleFile, SdkworkError> {
        let path = app_path(&"/knowledge/okf/context_packs".to_string());
        self.client.post(&path, Some(body), None, None, Some("application/json")).await
    }

    /// List knowledge browser view
    pub async fn spaces_browser_list(&self, space_id: &str, view: &str, parent_id: Option<&str>, cursor: Option<&str>, page_size: Option<i64>) -> Result<KnowledgeBrowserListData, SdkworkError> {
        let query = build_query_string(&[
            QueryParameterSpec::new("view", view, "form", true, false, None),
            QueryParameterSpec::new("parent_id", parent_id, "form", true, false, None),
            QueryParameterSpec::new("cursor", cursor, "form", true, false, None),
            QueryParameterSpec::new("page_size", page_size, "form", true, false, None),
        ]);
        let path = append_query_string(app_path(&format!("/knowledge/spaces/{}/browser", serialize_path_parameter(space_id, PathParameterSpec::new("spaceId", "simple", false)))), &query);
        self.client.get(&path, None, None).await
    }

    /// Create a knowledge retrieval
    pub async fn retrievals_create(&self, body: &KnowledgeRetrievalRequest) -> Result<KnowledgeRetrievalResult, SdkworkError> {
        let path = app_path(&"/knowledge/retrievals".to_string());
        self.client.post(&path, Some(body), None, None, Some("application/json")).await
    }

    /// Retrieve a knowledge retrieval result
    pub async fn retrievals_retrieve(&self, retrieval_id: &str) -> Result<KnowledgeRetrievalResult, SdkworkError> {
        let path = app_path(&format!("/knowledge/retrievals/{}", serialize_path_parameter(retrieval_id, PathParameterSpec::new("retrievalId", "simple", false))));
        self.client.get(&path, None, None).await
    }

    /// Create a knowledge context pack
    pub async fn context_packs_create(&self, body: &KnowledgeContextPackRequest) -> Result<KnowledgeContextPack, SdkworkError> {
        let path = app_path(&"/knowledge/context_packs".to_string());
        self.client.post(&path, Some(body), None, None, Some("application/json")).await
    }

    /// Create a knowledge agent profile
    pub async fn agent_profiles_create(&self, body: &KnowledgeAgentProfileRequest) -> Result<KnowledgeAgentProfile, SdkworkError> {
        let path = app_path(&"/knowledge/agent_profiles".to_string());
        self.client.post(&path, Some(body), None, None, Some("application/json")).await
    }

    /// Retrieve a knowledge agent profile
    pub async fn agent_profiles_retrieve(&self, profile_id: &str) -> Result<KnowledgeAgentProfile, SdkworkError> {
        let path = app_path(&format!("/knowledge/agent_profiles/{}", serialize_path_parameter(profile_id, PathParameterSpec::new("profileId", "simple", false))));
        self.client.get(&path, None, None).await
    }

    /// Update a knowledge agent profile
    pub async fn agent_profiles_update(&self, profile_id: &str, body: &KnowledgeAgentProfileRequest) -> Result<KnowledgeAgentProfile, SdkworkError> {
        let path = app_path(&format!("/knowledge/agent_profiles/{}", serialize_path_parameter(profile_id, PathParameterSpec::new("profileId", "simple", false))));
        self.client.patch(&path, Some(body), None, None, Some("application/json")).await
    }

    /// Delete a knowledge agent profile
    pub async fn agent_profiles_delete(&self, profile_id: &str) -> Result<(), SdkworkError> {
        let path = app_path(&format!("/knowledge/agent_profiles/{}", serialize_path_parameter(profile_id, PathParameterSpec::new("profileId", "simple", false))));
        self.client.delete(&path, None, None).await
    }

    /// List agent profile bindings
    pub async fn agent_profiles_bindings_list(&self, profile_id: &str) -> Result<KnowledgeAgentBindingList, SdkworkError> {
        let path = app_path(&format!("/knowledge/agent_profiles/{}/bindings", serialize_path_parameter(profile_id, PathParameterSpec::new("profileId", "simple", false))));
        self.client.get(&path, None, None).await
    }

    /// Create an agent profile binding
    pub async fn agent_profiles_bindings_create(&self, profile_id: &str, body: &KnowledgeAgentBindingRequest) -> Result<KnowledgeAgentBinding, SdkworkError> {
        let path = app_path(&format!("/knowledge/agent_profiles/{}/bindings", serialize_path_parameter(profile_id, PathParameterSpec::new("profileId", "simple", false))));
        self.client.post(&path, Some(body), None, None, Some("application/json")).await
    }

    /// Update an agent profile binding
    pub async fn agent_profiles_bindings_update(&self, profile_id: &str, binding_id: &str, body: &KnowledgeAgentBindingRequest) -> Result<KnowledgeAgentBinding, SdkworkError> {
        let path = app_path(&format!("/knowledge/agent_profiles/{}/bindings/{}", serialize_path_parameter(profile_id, PathParameterSpec::new("profileId", "simple", false)), serialize_path_parameter(binding_id, PathParameterSpec::new("bindingId", "simple", false))));
        self.client.patch(&path, Some(body), None, None, Some("application/json")).await
    }

    /// Delete an agent profile binding
    pub async fn agent_profiles_bindings_delete(&self, profile_id: &str, binding_id: &str) -> Result<(), SdkworkError> {
        let path = app_path(&format!("/knowledge/agent_profiles/{}/bindings/{}", serialize_path_parameter(profile_id, PathParameterSpec::new("profileId", "simple", false)), serialize_path_parameter(binding_id, PathParameterSpec::new("bindingId", "simple", false))));
        self.client.delete(&path, None, None).await
    }

    /// Preview retrieval for an agent profile
    pub async fn agent_profiles_retrieval_preview_create(&self, profile_id: &str, body: &KnowledgeRetrievalRequest) -> Result<KnowledgeRetrievalResult, SdkworkError> {
        let path = app_path(&format!("/knowledge/agent_profiles/{}/retrieval_preview", serialize_path_parameter(profile_id, PathParameterSpec::new("profileId", "simple", false))));
        self.client.post(&path, Some(body), None, None, Some("application/json")).await
    }

    /// Chat with a knowledge-backed agent profile
    pub async fn agent_profiles_chat_create(&self, profile_id: &str, body: &KnowledgeAgentChatRequest) -> Result<KnowledgeAgentChatResponse, SdkworkError> {
        let path = app_path(&format!("/knowledge/agent_profiles/{}/chat", serialize_path_parameter(profile_id, PathParameterSpec::new("profileId", "simple", false))));
        self.client.post(&path, Some(body), None, None, Some("application/json")).await
    }

    /// List knowledge space context bindings
    pub async fn spaces_context_bindings_list(&self, space_id: &str, cursor: Option<&str>, page_size: Option<i64>) -> Result<serde_json::Value, SdkworkError> {
        let query = build_query_string(&[
            QueryParameterSpec::new("cursor", cursor, "form", true, false, None),
            QueryParameterSpec::new("page_size", page_size, "form", true, false, None),
        ]);
        let path = append_query_string(app_path(&format!("/knowledge/spaces/{}/context_bindings", serialize_path_parameter(space_id, PathParameterSpec::new("spaceId", "simple", false)))), &query);
        self.client.get(&path, None, None).await
    }

    /// Create a knowledge space context binding
    pub async fn spaces_context_bindings_create(&self, space_id: &str, body: &CreateKnowledgeSpaceContextBindingRequest) -> Result<KnowledgeSpaceContextBinding, SdkworkError> {
        let path = app_path(&format!("/knowledge/spaces/{}/context_bindings", serialize_path_parameter(space_id, PathParameterSpec::new("spaceId", "simple", false))));
        self.client.post(&path, Some(body), None, None, Some("application/json")).await
    }

    /// List knowledge space members
    pub async fn spaces_members_list(&self, space_id: &str, cursor: Option<&str>, page_size: Option<i64>) -> Result<serde_json::Value, SdkworkError> {
        let query = build_query_string(&[
            QueryParameterSpec::new("cursor", cursor, "form", true, false, None),
            QueryParameterSpec::new("page_size", page_size, "form", true, false, None),
        ]);
        let path = append_query_string(app_path(&format!("/knowledge/spaces/{}/members", serialize_path_parameter(space_id, PathParameterSpec::new("spaceId", "simple", false)))), &query);
        self.client.get(&path, None, None).await
    }

    /// Grant knowledge space member access
    pub async fn spaces_members_create(&self, space_id: &str, body: &GrantKnowledgeSpaceMemberRequest) -> Result<SdkWorkCommandData, SdkworkError> {
        let path = app_path(&format!("/knowledge/spaces/{}/members", serialize_path_parameter(space_id, PathParameterSpec::new("spaceId", "simple", false))));
        self.client.post(&path, Some(body), None, None, Some("application/json")).await
    }

    /// Revoke knowledge space member access
    pub async fn spaces_members_delete(&self, space_id: &str, subject_type: &str, subject_id: &str) -> Result<(), SdkworkError> {
        let query = build_query_string(&[
            QueryParameterSpec::new("subject_type", subject_type, "form", true, false, None),
            QueryParameterSpec::new("subject_id", subject_id, "form", true, false, None),
        ]);
        let path = append_query_string(app_path(&format!("/knowledge/spaces/{}/members", serialize_path_parameter(space_id, PathParameterSpec::new("spaceId", "simple", false)))), &query);
        self.client.delete(&path, None, None).await
    }

    /// Retrieve a knowledge space context binding
    pub async fn context_bindings_retrieve(&self, binding_id: &str) -> Result<KnowledgeSpaceContextBinding, SdkworkError> {
        let path = app_path(&format!("/knowledge/context_bindings/{}", serialize_path_parameter(binding_id, PathParameterSpec::new("bindingId", "simple", false))));
        self.client.get(&path, None, None).await
    }

    /// Update a knowledge space context binding
    pub async fn context_bindings_update(&self, binding_id: &str, body: &UpdateKnowledgeSpaceContextBindingRequest) -> Result<KnowledgeSpaceContextBinding, SdkworkError> {
        let path = app_path(&format!("/knowledge/context_bindings/{}", serialize_path_parameter(binding_id, PathParameterSpec::new("bindingId", "simple", false))));
        self.client.patch(&path, Some(body), None, None, Some("application/json")).await
    }

    /// Delete a knowledge space context binding
    pub async fn context_bindings_delete(&self, binding_id: &str) -> Result<(), SdkworkError> {
        let path = app_path(&format!("/knowledge/context_bindings/{}", serialize_path_parameter(binding_id, PathParameterSpec::new("bindingId", "simple", false))));
        self.client.delete(&path, None, None).await
    }

    /// Upsert an OKF concept revision
    pub async fn okf_concepts_update(&self, body: &OkfConceptUpsertRequest) -> Result<OkfConceptSummary, SdkworkError> {
        let path = app_path(&"/knowledge/okf/concepts/upsert".to_string());
        self.client.put(&path, Some(body), None, None, Some("application/json")).await
    }

    /// Create an OKF bundle export
    pub async fn okf_bundle_export_create(&self, body: &OkfBundleExportRequest) -> Result<KnowledgeOkfBundleFile, SdkworkError> {
        let path = app_path(&"/knowledge/okf/exports".to_string());
        self.client.post(&path, Some(body), None, None, Some("application/json")).await
    }

    /// Retrieve an OKF bundle export
    pub async fn okf_bundle_export_retrieve(&self, export_id: &str) -> Result<KnowledgeOkfBundleFile, SdkworkError> {
        let path = app_path(&format!("/knowledge/okf/exports/{}", serialize_path_parameter(export_id, PathParameterSpec::new("exportId", "simple", false))));
        self.client.get(&path, None, None).await
    }

    /// Import an OKF bundle from drive staging
    pub async fn okf_bundle_import_create(&self, body: &OkfBundleImportRequest) -> Result<OkfBundleImportResult, SdkworkError> {
        let path = app_path(&"/knowledge/okf/imports".to_string());
        self.client.post(&path, Some(body), None, None, Some("application/json")).await
    }

    /// Create an OKF bundle lint run
    pub async fn okf_lint_runs_create(&self, body: &OkfQualityRunRequest) -> Result<OkfQualityRun, SdkworkError> {
        let path = app_path(&"/knowledge/okf/lint_runs".to_string());
        self.client.post(&path, Some(body), None, None, Some("application/json")).await
    }

    /// List WeChat official accounts
    pub async fn wechat_official_accounts_list(&self, cursor: Option<&str>, page_size: Option<i64>) -> Result<serde_json::Value, SdkworkError> {
        let query = build_query_string(&[
            QueryParameterSpec::new("cursor", cursor, "form", true, false, None),
            QueryParameterSpec::new("page_size", page_size, "form", true, false, None),
        ]);
        let path = append_query_string(app_path(&"/knowledge/wechat/official_accounts".to_string()), &query);
        self.client.get(&path, None, None).await
    }

    /// Replace WeChat official accounts
    pub async fn wechat_official_accounts_update(&self, body: &KnowledgeWechatReplaceOfficialAccountsRequest) -> Result<KnowledgeWechatOfficialAccountList, SdkworkError> {
        let path = app_path(&"/knowledge/wechat/official_accounts".to_string());
        self.client.put(&path, Some(body), None, None, Some("application/json")).await
    }

    /// List WeChat official account fan tags
    pub async fn wechat_official_accounts_fan_tags_list(&self, account_id: &str, cursor: Option<&str>, page_size: Option<i64>) -> Result<serde_json::Value, SdkworkError> {
        let query = build_query_string(&[
            QueryParameterSpec::new("cursor", cursor, "form", true, false, None),
            QueryParameterSpec::new("page_size", page_size, "form", true, false, None),
        ]);
        let path = append_query_string(app_path(&format!("/knowledge/wechat/official_accounts/{}/fan_tags", serialize_path_parameter(account_id, PathParameterSpec::new("accountId", "simple", false)))), &query);
        self.client.get(&path, None, None).await
    }

    /// List WeChat applets
    pub async fn wechat_applets_list(&self, cursor: Option<&str>, page_size: Option<i64>) -> Result<serde_json::Value, SdkworkError> {
        let query = build_query_string(&[
            QueryParameterSpec::new("cursor", cursor, "form", true, false, None),
            QueryParameterSpec::new("page_size", page_size, "form", true, false, None),
        ]);
        let path = append_query_string(app_path(&"/knowledge/wechat/applets".to_string()), &query);
        self.client.get(&path, None, None).await
    }

    /// Replace WeChat applets
    pub async fn wechat_applets_update(&self, body: &KnowledgeWechatReplaceAppletsRequest) -> Result<KnowledgeWechatAppletList, SdkworkError> {
        let path = app_path(&"/knowledge/wechat/applets".to_string());
        self.client.put(&path, Some(body), None, None, Some("application/json")).await
    }

    /// Publish WeChat articles
    pub async fn wechat_articles_publish(&self, body: &KnowledgeWechatArticlesPublishRequest) -> Result<KnowledgeWechatOperationResult, SdkworkError> {
        let path = app_path(&"/knowledge/wechat/articles/publish".to_string());
        self.client.post(&path, Some(body), None, None, Some("application/json")).await
    }

    /// Preview WeChat articles
    pub async fn wechat_articles_preview(&self, body: &KnowledgeWechatArticlesPreviewRequest) -> Result<KnowledgeWechatOperationResult, SdkworkError> {
        let path = app_path(&"/knowledge/wechat/articles/preview".to_string());
        self.client.post(&path, Some(body), None, None, Some("application/json")).await
    }

    /// Sync knowledge space documents to a Git repository
    pub async fn git_syncs_create(&self, body: &KnowledgeGitSyncRequest) -> Result<KnowledgeGitSyncResult, SdkworkError> {
        let path = app_path(&"/knowledge/git_syncs".to_string());
        self.client.post(&path, Some(body), None, None, Some("application/json")).await
    }

    /// List knowledge market catalog listings
    pub async fn market_listings_list(&self, cursor: Option<&str>, page_size: Option<i64>) -> Result<serde_json::Value, SdkworkError> {
        let query = build_query_string(&[
            QueryParameterSpec::new("cursor", cursor, "form", true, false, None),
            QueryParameterSpec::new("page_size", page_size, "form", true, false, None),
        ]);
        let path = append_query_string(app_path(&"/knowledge/market/listings".to_string()), &query);
        self.client.get(&path, None, None).await
    }

    /// Subscribe to a knowledge market listing
    pub async fn market_subscriptions_create(&self, body: &KnowledgeMarketSubscriptionRequest) -> Result<KnowledgeMarketSubscriptionResult, SdkworkError> {
        let path = app_path(&"/knowledge/market/subscriptions".to_string());
        self.client.post(&path, Some(body), None, None, Some("application/json")).await
    }

    /// Unsubscribe from a knowledge market listing
    pub async fn market_subscriptions_delete(&self, listing_id: &str) -> Result<(), SdkworkError> {
        let path = app_path(&format!("/knowledge/market/subscriptions/{}", serialize_path_parameter(listing_id, PathParameterSpec::new("listingId", "simple", false))));
        self.client.delete(&path, None, None).await
    }

    /// Create a knowledge media task (image generation or speech-to-text)
    pub async fn media_tasks_create(&self, body: &KnowledgeMediaTaskRequest) -> Result<KnowledgeMediaTaskResult, SdkworkError> {
        let path = app_path(&"/knowledge/media_tasks".to_string());
        self.client.post(&path, Some(body), None, None, Some("application/json")).await
    }

    /// Retrieve Wiki publication
    pub async fn wiki_publications_retrieve(&self, space_id: &str) -> Result<KnowledgeWikiPublication, SdkworkError> {
        let path = app_path(&format!("/knowledge/spaces/{}/wiki_publication", serialize_path_parameter(space_id, PathParameterSpec::new("spaceId", "simple", false))));
        self.client.get(&path, None, None).await
    }

    /// Activate Wiki publication
    pub async fn wiki_publications_activate(&self, space_id: &str, body: &KnowledgeWikiPublicationVersionCommandRequest, idempotency_key: &str) -> Result<KnowledgeWikiPublication, SdkworkError> {
        let path = app_path(&format!("/knowledge/spaces/{}/wiki_publication/activate", serialize_path_parameter(space_id, PathParameterSpec::new("spaceId", "simple", false))));
        let headers = build_request_headers(
            &[
                ("Idempotency-Key", HeaderParameterSpec::new(idempotency_key, "simple", false, None)),
            ],
            &[],
        );
        self.client.post(&path, Some(body), None, headers.as_ref(), Some("application/json")).await
    }

    /// Pause Wiki publication
    pub async fn wiki_publications_pause(&self, space_id: &str, body: &KnowledgeWikiPublicationVersionCommandRequest, idempotency_key: &str) -> Result<KnowledgeWikiPublication, SdkworkError> {
        let path = app_path(&format!("/knowledge/spaces/{}/wiki_publication/pause", serialize_path_parameter(space_id, PathParameterSpec::new("spaceId", "simple", false))));
        let headers = build_request_headers(
            &[
                ("Idempotency-Key", HeaderParameterSpec::new(idempotency_key, "simple", false, None)),
            ],
            &[],
        );
        self.client.post(&path, Some(body), None, headers.as_ref(), Some("application/json")).await
    }

    /// Publish Wiki source file
    pub async fn wiki_source_files_publish(&self, space_id: &str, source_file_uuid: &str, body: &PublishKnowledgeWikiSourceFileRequest, idempotency_key: &str) -> Result<KnowledgeWikiSourceFileCommandResult, SdkworkError> {
        let path = app_path(&format!("/knowledge/spaces/{}/wiki_source_files/{}/publish", serialize_path_parameter(space_id, PathParameterSpec::new("spaceId", "simple", false)), serialize_path_parameter(source_file_uuid, PathParameterSpec::new("sourceFileUuid", "simple", false))));
        let headers = build_request_headers(
            &[
                ("Idempotency-Key", HeaderParameterSpec::new(idempotency_key, "simple", false, None)),
            ],
            &[],
        );
        self.client.post(&path, Some(body), None, headers.as_ref(), Some("application/json")).await
    }

    /// Unpublish Wiki source file
    pub async fn wiki_source_files_unpublish(&self, space_id: &str, source_file_uuid: &str, body: &KnowledgeWikiSourceFileVersionCommandRequest, idempotency_key: &str) -> Result<KnowledgeWikiSourceFileCommandResult, SdkworkError> {
        let path = app_path(&format!("/knowledge/spaces/{}/wiki_source_files/{}/unpublish", serialize_path_parameter(space_id, PathParameterSpec::new("spaceId", "simple", false)), serialize_path_parameter(source_file_uuid, PathParameterSpec::new("sourceFileUuid", "simple", false))));
        let headers = build_request_headers(
            &[
                ("Idempotency-Key", HeaderParameterSpec::new(idempotency_key, "simple", false, None)),
            ],
            &[],
        );
        self.client.post(&path, Some(body), None, headers.as_ref(), Some("application/json")).await
    }

    /// Change Wiki source file visibility
    pub async fn wiki_source_files_visibility_update(&self, space_id: &str, source_file_uuid: &str, body: &ChangeKnowledgeWikiSourceFileVisibilityRequest, idempotency_key: &str) -> Result<KnowledgeWikiSourceFileCommandResult, SdkworkError> {
        let path = app_path(&format!("/knowledge/spaces/{}/wiki_source_files/{}/visibility", serialize_path_parameter(space_id, PathParameterSpec::new("spaceId", "simple", false)), serialize_path_parameter(source_file_uuid, PathParameterSpec::new("sourceFileUuid", "simple", false))));
        let headers = build_request_headers(
            &[
                ("Idempotency-Key", HeaderParameterSpec::new(idempotency_key, "simple", false, None)),
            ],
            &[],
        );
        self.client.patch(&path, Some(body), None, headers.as_ref(), Some("application/json")).await
    }

    /// List Wiki source files
    pub async fn wiki_source_files_list(&self, space_id: &str, cursor: Option<&str>, page_size: Option<i64>) -> Result<serde_json::Value, SdkworkError> {
        let query = build_query_string(&[
            QueryParameterSpec::new("cursor", cursor, "form", true, false, None),
            QueryParameterSpec::new("page_size", page_size, "form", true, false, None),
        ]);
        let path = append_query_string(app_path(&format!("/knowledge/spaces/{}/wiki_source_files", serialize_path_parameter(space_id, PathParameterSpec::new("spaceId", "simple", false)))), &query);
        self.client.get(&path, None, None).await
    }

}

struct PathParameterSpec<'a> {
    name: &'a str,
    style: &'a str,
    explode: bool,
}

impl<'a> PathParameterSpec<'a> {
    fn new(name: &'a str, style: &'a str, explode: bool) -> Self {
        Self { name, style, explode }
    }
}

fn serialize_path_parameter<T: serde::Serialize>(value: T, spec: PathParameterSpec<'_>) -> String {
    let value = serde_json::to_value(value).unwrap_or(serde_json::Value::Null);
    if value.is_null() {
        return String::new();
    }
    let style = if spec.style.is_empty() { "simple" } else { spec.style };
    match value {
        serde_json::Value::Array(values) => serialize_path_array(spec.name, &values, style, spec.explode),
        serde_json::Value::Object(values) => serialize_path_object(spec.name, &values, style, spec.explode),
        value => format!("{}{}", path_primitive_prefix(spec.name, style), percent_encode(&primitive_to_string(&value))),
    }
}

fn serialize_path_array(name: &str, values: &[serde_json::Value], style: &str, explode: bool) -> String {
    let serialized = values
        .iter()
        .filter(|value| !value.is_null())
        .map(|value| percent_encode(&primitive_to_string(value)))
        .collect::<Vec<_>>();
    if serialized.is_empty() {
        return path_prefix(name, style);
    }
    if style == "matrix" {
        if explode {
            return serialized.iter().map(|item| format!(";{}={}", name, item)).collect::<Vec<_>>().join("");
        }
        return format!(";{}={}", name, serialized.join(","));
    }
    let separator = if explode { "." } else { "," };
    format!("{}{}", path_prefix(name, style), serialized.join(separator))
}

fn serialize_path_object(
    name: &str,
    values: &serde_json::Map<String, serde_json::Value>,
    style: &str,
    explode: bool,
) -> String {
    let mut entries = Vec::new();
    let mut exploded = Vec::new();
    for (key, value) in values {
        if value.is_null() {
            continue;
        }
        let escaped_key = percent_encode(key);
        let escaped_value = percent_encode(&primitive_to_string(value));
        if explode {
            if style == "matrix" {
                exploded.push(format!(";{}={}", escaped_key, escaped_value));
            } else {
                exploded.push(format!("{}={}", escaped_key, escaped_value));
            }
        } else {
            entries.push(escaped_key);
            entries.push(escaped_value);
        }
    }
    if style == "matrix" {
        if explode {
            return exploded.join("");
        }
        return format!(";{}={}", name, entries.join(","));
    }
    if explode {
        let separator = if style == "label" { "." } else { "," };
        return format!("{}{}", path_prefix(name, style), exploded.join(separator));
    }
    format!("{}{}", path_prefix(name, style), entries.join(","))
}

fn path_prefix(name: &str, style: &str) -> String {
    match style {
        "label" => ".".to_string(),
        "matrix" => format!(";{}", name),
        _ => String::new(),
    }
}

fn path_primitive_prefix(name: &str, style: &str) -> String {
    if style == "matrix" {
        format!(";{}=", name)
    } else {
        path_prefix(name, style)
    }
}

struct HeaderParameterSpec {
    value: serde_json::Value,
    explode: bool,
    content_type: Option<&'static str>,
}

impl HeaderParameterSpec {
    fn new<T: serde::Serialize>(
        value: T,
        _style: &'static str,
        explode: bool,
        content_type: Option<&'static str>,
    ) -> Self {
        Self {
            value: serde_json::to_value(value).unwrap_or(serde_json::Value::Null),
            explode,
            content_type,
        }
    }
}

fn build_request_headers(headers: &[(&str, HeaderParameterSpec)], cookies: &[(&str, HeaderParameterSpec)]) -> Option<RequestHeaders> {
    let mut request_headers = RequestHeaders::new();
    for (name, parameter) in headers {
        if let Some(value) = serialize_header_parameter(parameter) {
            request_headers.insert((*name).to_string(), value);
        }
    }

    let cookie_header = build_cookie_header(cookies);
    if !cookie_header.is_empty() {
        request_headers
            .entry("Cookie".to_string())
            .and_modify(|existing| {
                existing.push_str("; ");
                existing.push_str(&cookie_header);
            })
            .or_insert(cookie_header);
    }

    if request_headers.is_empty() {
        None
    } else {
        Some(request_headers)
    }
}

fn build_cookie_header(cookies: &[(&str, HeaderParameterSpec)]) -> String {
    cookies
        .iter()
        .filter_map(|(name, value)| {
            serialize_header_parameter(value)
                .map(|value| format!("{}={}", percent_encode(name), percent_encode(&value)))
        })
        .collect::<Vec<_>>()
        .join("; ")
}

fn serialize_header_parameter(parameter: &HeaderParameterSpec) -> Option<String> {
    if parameter.value.is_null() {
        return None;
    }
    if parameter.content_type.is_some() {
        return Some(parameter.value.to_string());
    }
    match &parameter.value {
        serde_json::Value::Null => None,
        serde_json::Value::String(value) => Some(value.clone()),
        serde_json::Value::Number(value) => Some(value.to_string()),
        serde_json::Value::Bool(value) => Some(value.to_string()),
        serde_json::Value::Array(values) => {
            let serialized = values
                .iter()
                .filter_map(serialize_json_value)
                .collect::<Vec<_>>();
            if serialized.is_empty() {
                None
            } else {
                Some(serialized.join(","))
            }
        }
        serde_json::Value::Object(values) => {
            let serialized = values
                .iter()
                .filter_map(|(key, value)| {
                    serialize_json_value(value).map(|serialized| {
                        if parameter.explode {
                            format!("{}={}", key, serialized)
                        } else {
                            format!("{},{}", key, serialized)
                        }
                    })
                })
                .collect::<Vec<_>>();
            if serialized.is_empty() {
                None
            } else {
                Some(serialized.join(","))
            }
        }
    }
}

fn serialize_json_value(value: &serde_json::Value) -> Option<String> {
    match value {
        serde_json::Value::Null => None,
        serde_json::Value::String(value) => Some(value.clone()),
        serde_json::Value::Number(value) => Some(value.to_string()),
        serde_json::Value::Bool(value) => Some(value.to_string()),
        other => Some(other.to_string()),
    }
}

struct QueryParameterSpec<'a> {
    name: &'a str,
    value: serde_json::Value,
    style: &'a str,
    explode: bool,
    allow_reserved: bool,
    content_type: Option<&'a str>,
}

impl<'a> QueryParameterSpec<'a> {
    fn new<T: serde::Serialize>(
        name: &'a str,
        value: T,
        style: &'a str,
        explode: bool,
        allow_reserved: bool,
        content_type: Option<&'a str>,
    ) -> Self {
        Self {
            name,
            value: serde_json::to_value(value).unwrap_or(serde_json::Value::Null),
            style,
            explode,
            allow_reserved,
            content_type,
        }
    }
}

fn build_query_string(parameters: &[QueryParameterSpec<'_>]) -> String {
    let mut pairs = Vec::new();
    for parameter in parameters {
        append_serialized_parameter(&mut pairs, parameter);
    }
    pairs.join("&")
}

fn append_serialized_parameter(pairs: &mut Vec<String>, parameter: &QueryParameterSpec<'_>) {
    if parameter.value.is_null() {
        return;
    }
    if parameter.content_type.is_some() {
        pairs.push(format!(
            "{}={}",
            percent_encode(parameter.name),
            encode_query_value(&parameter.value.to_string(), parameter.allow_reserved)
        ));
        return;
    }

    let style = if parameter.style.is_empty() { "form" } else { parameter.style };
    match &parameter.value {
        serde_json::Value::Array(values) => append_array_parameter(pairs, parameter.name, values, style, parameter.explode, parameter.allow_reserved),
        serde_json::Value::Object(values) if style == "deepObject" => append_deep_object_parameter(pairs, parameter.name, values, parameter.allow_reserved),
        serde_json::Value::Object(values) => append_object_parameter(pairs, parameter.name, values, style, parameter.explode, parameter.allow_reserved),
        value => pairs.push(format!("{}={}", percent_encode(parameter.name), encode_query_value(&primitive_to_string(value), parameter.allow_reserved))),
    }
}

fn append_array_parameter(
    pairs: &mut Vec<String>,
    name: &str,
    values: &[serde_json::Value],
    style: &str,
    explode: bool,
    allow_reserved: bool,
) {
    let serialized = values.iter().filter(|value| !value.is_null()).map(primitive_to_string).collect::<Vec<_>>();
    if serialized.is_empty() {
        return;
    }
    if style == "form" && explode {
        for item in serialized {
            pairs.push(format!("{}={}", percent_encode(name), encode_query_value(&item, allow_reserved)));
        }
        return;
    }
    pairs.push(format!("{}={}", percent_encode(name), encode_query_value(&serialized.join(","), allow_reserved)));
}

fn append_object_parameter(
    pairs: &mut Vec<String>,
    name: &str,
    values: &serde_json::Map<String, serde_json::Value>,
    style: &str,
    explode: bool,
    allow_reserved: bool,
) {
    let mut serialized = Vec::new();
    for (key, value) in values {
        if value.is_null() {
            continue;
        }
        if style == "form" && explode {
            pairs.push(format!("{}={}", percent_encode(key), encode_query_value(&primitive_to_string(value), allow_reserved)));
        } else {
            serialized.push(key.clone());
            serialized.push(primitive_to_string(value));
        }
    }
    if !serialized.is_empty() {
        pairs.push(format!("{}={}", percent_encode(name), encode_query_value(&serialized.join(","), allow_reserved)));
    }
}

fn append_deep_object_parameter(
    pairs: &mut Vec<String>,
    name: &str,
    values: &serde_json::Map<String, serde_json::Value>,
    allow_reserved: bool,
) {
    for (key, value) in values {
        if !value.is_null() {
            pairs.push(format!("{}={}", percent_encode(&format!("{}[{}]", name, key)), encode_query_value(&primitive_to_string(value), allow_reserved)));
        }
    }
}

fn encode_query_value(value: &str, allow_reserved: bool) -> String {
    let mut encoded = percent_encode(value);
    if !allow_reserved {
        return encoded;
    }
    for (escaped, reserved) in [
        ("%3A", ":"), ("%2F", "/"), ("%3F", "?"), ("%23", "#"),
        ("%5B", "["), ("%5D", "]"), ("%40", "@"), ("%21", "!"),
        ("%24", "$"), ("%26", "&"), ("%27", "'"), ("%28", "("),
        ("%29", ")"), ("%2A", "*"), ("%2B", "+"), ("%2C", ","),
        ("%3B", ";"), ("%3D", "="),
    ] {
        encoded = encoded.replace(escaped, reserved);
    }
    encoded
}

fn primitive_to_string(value: &serde_json::Value) -> String {
    match value {
        serde_json::Value::String(value) => value.clone(),
        serde_json::Value::Number(value) => value.to_string(),
        serde_json::Value::Bool(value) => value.to_string(),
        other => other.to_string(),
    }
}

fn percent_encode(value: &str) -> String {
    value
        .bytes()
        .flat_map(|byte| match byte {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' => {
                vec![byte as char]
            }
            _ => format!("%{:02X}", byte).chars().collect(),
        })
        .collect()
}
