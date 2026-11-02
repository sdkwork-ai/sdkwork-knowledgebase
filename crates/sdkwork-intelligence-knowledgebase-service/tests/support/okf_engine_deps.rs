use async_trait::async_trait;
use sdkwork_intelligence_knowledgebase_service::knowledge_engine::{
    KnowledgeEngineRuntimeDeps, OkfNativeKnowledgeEngineDeps,
};
use sdkwork_intelligence_knowledgebase_service::ports::knowledge_drive_object_ref_store::{
    CreateKnowledgeDriveObjectRefRecord, KnowledgeDriveObjectRefStore,
    KnowledgeDriveObjectRefStoreError,
};
use sdkwork_intelligence_knowledgebase_service::ports::knowledge_drive_storage::KnowledgeDriveStorage;
use sdkwork_intelligence_knowledgebase_service::ports::knowledge_drive_workspace::{
    EnsureKnowledgeDriveNodesRequest, KnowledgeDriveWorkspace, KnowledgeDriveWorkspaceError,
};
use sdkwork_intelligence_knowledgebase_service::ports::knowledge_okf_bundle_file_store::{
    CreateKnowledgeOkfBundleFileRecord, KnowledgeOkfBundleFileStore,
    KnowledgeOkfBundleFileStoreError,
};
use sdkwork_intelligence_knowledgebase_service::ports::knowledge_okf_candidate_store::{
    KnowledgeOkfCandidateListItem, KnowledgeOkfCandidateStore, KnowledgeOkfCandidateStoreError,
    UpsertKnowledgeOkfCandidateRecord,
};
use sdkwork_intelligence_knowledgebase_service::ports::knowledge_okf_concept_link_store::{
    InboundLinkTargetsPage, KnowledgeOkfConceptLinkStore, KnowledgeOkfConceptLinkStoreError,
    LinkEdgeCursor, LinkEdgePage, ReplaceKnowledgeOkfConceptLinksRecord,
};
use sdkwork_intelligence_knowledgebase_service::ports::knowledge_okf_concept_store::KnowledgeOkfConceptStore;
use sdkwork_intelligence_knowledgebase_service::ports::knowledge_source_store::{
    CreateKnowledgeSourceRecord, KnowledgeSourceStore, KnowledgeSourceStoreError,
};
use sdkwork_intelligence_knowledgebase_service::ports::knowledge_space_store::{
    BindKnowledgeDriveSpaceRecord, CreateKnowledgeSpaceRecord, KnowledgeSpaceStore,
    KnowledgeSpaceStoreError, UpdateKnowledgeSpaceRecord,
};
use sdkwork_intelligence_knowledgebase_service::ports::okf_concept_revision_metadata_store::{
    OkfConceptRevisionMetadataStore, OkfConceptRevisionMetadataStoreError,
    PreparedOkfConceptRevisionSlot, PublishOkfConceptRevisionMetadataRecord,
    PublishedOkfConceptRevisionMetadata, StageOkfConceptRevisionMetadataRecord,
    StagedOkfConceptRevisionMetadata,
};
use sdkwork_knowledgebase_contract::okf_bundle_file::KnowledgeOkfBundleFile;
use sdkwork_knowledgebase_contract::space::{KnowledgeSpace, KnowledgeSpaceStatus};
use sdkwork_knowledgebase_contract::{
    KnowledgeDriveObjectRef, KnowledgeSource, OkfConceptPublishState,
};
use std::sync::Arc;

const UNUSED_PORT: &str = "port is intentionally unavailable in this test fixture";

pub fn okf_test_deps(
    concepts: Arc<dyn KnowledgeOkfConceptStore>,
    drive: Arc<dyn KnowledgeDriveStorage>,
) -> OkfNativeKnowledgeEngineDeps {
    KnowledgeEngineRuntimeDeps::okf_from_stores(
        concepts,
        drive,
        Arc::new(UnavailableRevisionMetadataStore),
        Arc::new(UnavailableObjectRefStore),
        Arc::new(EmptyLinkStore),
        Arc::new(UnavailableCandidateStore),
        Arc::new(UnavailableBundleFileStore),
        Arc::new(UnavailableDriveWorkspace),
        Arc::new(UnavailableSourceStore),
        Arc::new(UnboundSpaceStore),
    )
}

struct UnavailableRevisionMetadataStore;

#[async_trait]
impl OkfConceptRevisionMetadataStore for UnavailableRevisionMetadataStore {
    async fn prepare_concept_revision_slot(
        &self,
        _concept: sdkwork_intelligence_knowledgebase_service::ports::knowledge_okf_concept_store::UpsertKnowledgeOkfConceptRecord,
    ) -> Result<PreparedOkfConceptRevisionSlot, OkfConceptRevisionMetadataStoreError> {
        Err(OkfConceptRevisionMetadataStoreError::internal(UNUSED_PORT))
    }

    async fn stage_concept_revision_metadata(
        &self,
        _record: StageOkfConceptRevisionMetadataRecord,
    ) -> Result<StagedOkfConceptRevisionMetadata, OkfConceptRevisionMetadataStoreError> {
        Err(OkfConceptRevisionMetadataStoreError::internal(UNUSED_PORT))
    }

    async fn publish_existing_revision_metadata(
        &self,
        _record: PublishOkfConceptRevisionMetadataRecord,
    ) -> Result<PublishedOkfConceptRevisionMetadata, OkfConceptRevisionMetadataStoreError> {
        Err(OkfConceptRevisionMetadataStoreError::internal(UNUSED_PORT))
    }
}

struct UnavailableObjectRefStore;

#[async_trait]
impl KnowledgeDriveObjectRefStore for UnavailableObjectRefStore {
    async fn create_object_ref(
        &self,
        _record: CreateKnowledgeDriveObjectRefRecord,
    ) -> Result<KnowledgeDriveObjectRef, KnowledgeDriveObjectRefStoreError> {
        Err(KnowledgeDriveObjectRefStoreError::Internal(
            UNUSED_PORT.to_string(),
        ))
    }

    async fn list_object_refs_by_logical_path_prefix(
        &self,
        _space_id: u64,
        _prefix: &str,
    ) -> Result<Vec<KnowledgeDriveObjectRef>, KnowledgeDriveObjectRefStoreError> {
        Err(KnowledgeDriveObjectRefStoreError::Internal(
            UNUSED_PORT.to_string(),
        ))
    }

    async fn get_object_ref_by_id(
        &self,
        _object_ref_id: u64,
    ) -> Result<KnowledgeDriveObjectRef, KnowledgeDriveObjectRefStoreError> {
        Err(KnowledgeDriveObjectRefStoreError::Internal(
            UNUSED_PORT.to_string(),
        ))
    }
}

struct EmptyLinkStore;

#[async_trait]
impl KnowledgeOkfConceptLinkStore for EmptyLinkStore {
    async fn replace_outbound_links(
        &self,
        _record: ReplaceKnowledgeOkfConceptLinksRecord,
    ) -> Result<(), KnowledgeOkfConceptLinkStoreError> {
        Err(KnowledgeOkfConceptLinkStoreError::Internal(
            UNUSED_PORT.to_string(),
        ))
    }

    async fn list_inbound_concept_ids(
        &self,
        _space_id: u64,
        _to_concept_id: &str,
    ) -> Result<Vec<String>, KnowledgeOkfConceptLinkStoreError> {
        Ok(Vec::new())
    }

    async fn list_inbound_link_targets_page(
        &self,
        _space_id: u64,
        _after_concept_id: Option<&str>,
        _limit: u32,
    ) -> Result<InboundLinkTargetsPage, KnowledgeOkfConceptLinkStoreError> {
        Ok(InboundLinkTargetsPage {
            targets: Vec::new(),
            next_cursor: None,
            has_more: false,
        })
    }

    async fn list_active_link_edges_page(
        &self,
        _space_id: u64,
        _after: Option<LinkEdgeCursor>,
        _limit: u32,
    ) -> Result<LinkEdgePage, KnowledgeOkfConceptLinkStoreError> {
        Ok(LinkEdgePage {
            edges: Vec::new(),
            next_cursor: None,
            has_more: false,
        })
    }
}

#[allow(dead_code)]
pub struct UnavailableLinkStore;

#[async_trait]
impl KnowledgeOkfConceptLinkStore for UnavailableLinkStore {
    async fn replace_outbound_links(
        &self,
        _record: ReplaceKnowledgeOkfConceptLinksRecord,
    ) -> Result<(), KnowledgeOkfConceptLinkStoreError> {
        Err(link_store_unavailable())
    }

    async fn list_inbound_concept_ids(
        &self,
        _space_id: u64,
        _to_concept_id: &str,
    ) -> Result<Vec<String>, KnowledgeOkfConceptLinkStoreError> {
        Err(link_store_unavailable())
    }

    async fn list_inbound_link_targets_page(
        &self,
        _space_id: u64,
        _after_concept_id: Option<&str>,
        _limit: u32,
    ) -> Result<InboundLinkTargetsPage, KnowledgeOkfConceptLinkStoreError> {
        Err(link_store_unavailable())
    }

    async fn list_active_link_edges_page(
        &self,
        _space_id: u64,
        _after: Option<LinkEdgeCursor>,
        _limit: u32,
    ) -> Result<LinkEdgePage, KnowledgeOkfConceptLinkStoreError> {
        Err(link_store_unavailable())
    }
}

#[allow(dead_code)]
fn link_store_unavailable() -> KnowledgeOkfConceptLinkStoreError {
    KnowledgeOkfConceptLinkStoreError::Internal("test link store unavailable".to_string())
}

struct UnavailableCandidateStore;

#[async_trait]
impl KnowledgeOkfCandidateStore for UnavailableCandidateStore {
    async fn upsert_candidate(
        &self,
        _record: UpsertKnowledgeOkfCandidateRecord,
    ) -> Result<(), KnowledgeOkfCandidateStoreError> {
        Err(KnowledgeOkfCandidateStoreError::Internal(
            UNUSED_PORT.to_string(),
        ))
    }

    async fn update_candidate_state_by_concept_row_id(
        &self,
        _concept_row_id: u64,
        _state: OkfConceptPublishState,
        _reviewer_id: Option<u64>,
        _review_note: Option<String>,
    ) -> Result<(), KnowledgeOkfCandidateStoreError> {
        Err(KnowledgeOkfCandidateStoreError::Internal(
            UNUSED_PORT.to_string(),
        ))
    }

    async fn list_open_candidates(
        &self,
        _space_id: Option<u64>,
    ) -> Result<Vec<KnowledgeOkfCandidateListItem>, KnowledgeOkfCandidateStoreError> {
        Err(KnowledgeOkfCandidateStoreError::Internal(
            UNUSED_PORT.to_string(),
        ))
    }
}

struct UnavailableBundleFileStore;

#[async_trait]
impl KnowledgeOkfBundleFileStore for UnavailableBundleFileStore {
    async fn create_file_entry(
        &self,
        _record: CreateKnowledgeOkfBundleFileRecord,
    ) -> Result<KnowledgeOkfBundleFile, KnowledgeOkfBundleFileStoreError> {
        Err(KnowledgeOkfBundleFileStoreError::Internal(
            UNUSED_PORT.to_string(),
        ))
    }
}

struct UnavailableDriveWorkspace;

#[async_trait]
impl KnowledgeDriveWorkspace for UnavailableDriveWorkspace {
    async fn ensure_nodes(
        &self,
        _request: EnsureKnowledgeDriveNodesRequest,
    ) -> Result<(), KnowledgeDriveWorkspaceError> {
        Err(KnowledgeDriveWorkspaceError::Internal(
            UNUSED_PORT.to_string(),
        ))
    }
}

struct UnavailableSourceStore;

#[async_trait]
impl KnowledgeSourceStore for UnavailableSourceStore {
    async fn create_source(
        &self,
        _record: CreateKnowledgeSourceRecord,
    ) -> Result<KnowledgeSource, KnowledgeSourceStoreError> {
        Err(KnowledgeSourceStoreError::Unsupported(
            UNUSED_PORT.to_string(),
        ))
    }
}

struct UnboundSpaceStore;

#[async_trait]
impl KnowledgeSpaceStore for UnboundSpaceStore {
    async fn create_space(
        &self,
        _record: CreateKnowledgeSpaceRecord,
    ) -> Result<KnowledgeSpace, KnowledgeSpaceStoreError> {
        Err(unused_space_port())
    }

    async fn get_space(&self, space_id: u64) -> Result<KnowledgeSpace, KnowledgeSpaceStoreError> {
        Ok(KnowledgeSpace {
            id: space_id,
            uuid: format!("test-space-{space_id}"),
            created_by: 0,
            name: format!("Test Space {space_id}"),
            description: None,
            drive_space_id: None,
            status: KnowledgeSpaceStatus::Active,
            okf_bundle_initialized: false,
            knowledge_mode:
                sdkwork_knowledgebase_contract::rag::KnowledgeAgentKnowledgeMode::OkfBundle,
        })
    }

    async fn mark_drive_space_bound(
        &self,
        _space_id: u64,
        _record: BindKnowledgeDriveSpaceRecord,
    ) -> Result<KnowledgeSpace, KnowledgeSpaceStoreError> {
        Err(unused_space_port())
    }

    async fn mark_okf_bundle_initialized(
        &self,
        _space_id: u64,
    ) -> Result<KnowledgeSpace, KnowledgeSpaceStoreError> {
        Err(unused_space_port())
    }

    async fn update_space(
        &self,
        _space_id: u64,
        _record: UpdateKnowledgeSpaceRecord,
    ) -> Result<KnowledgeSpace, KnowledgeSpaceStoreError> {
        Err(unused_space_port())
    }

    async fn mark_space_deleted(&self, _space_id: u64) -> Result<(), KnowledgeSpaceStoreError> {
        Err(unused_space_port())
    }
}

#[allow(dead_code)]
pub struct UnavailableSpaceStore;

#[async_trait]
impl KnowledgeSpaceStore for UnavailableSpaceStore {
    async fn create_space(
        &self,
        _record: CreateKnowledgeSpaceRecord,
    ) -> Result<KnowledgeSpace, KnowledgeSpaceStoreError> {
        Err(space_store_unavailable())
    }

    async fn get_space(&self, _space_id: u64) -> Result<KnowledgeSpace, KnowledgeSpaceStoreError> {
        Err(space_store_unavailable())
    }

    async fn mark_drive_space_bound(
        &self,
        _space_id: u64,
        _record: BindKnowledgeDriveSpaceRecord,
    ) -> Result<KnowledgeSpace, KnowledgeSpaceStoreError> {
        Err(space_store_unavailable())
    }

    async fn mark_okf_bundle_initialized(
        &self,
        _space_id: u64,
    ) -> Result<KnowledgeSpace, KnowledgeSpaceStoreError> {
        Err(space_store_unavailable())
    }

    async fn update_space(
        &self,
        _space_id: u64,
        _record: UpdateKnowledgeSpaceRecord,
    ) -> Result<KnowledgeSpace, KnowledgeSpaceStoreError> {
        Err(space_store_unavailable())
    }

    async fn mark_space_deleted(&self, _space_id: u64) -> Result<(), KnowledgeSpaceStoreError> {
        Err(space_store_unavailable())
    }
}

fn unused_space_port() -> KnowledgeSpaceStoreError {
    KnowledgeSpaceStoreError::Internal(UNUSED_PORT.to_string())
}

#[allow(dead_code)]
fn space_store_unavailable() -> KnowledgeSpaceStoreError {
    KnowledgeSpaceStoreError::Internal("test space store unavailable".to_string())
}
