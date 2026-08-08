use async_trait::async_trait;
use sdkwork_intelligence_knowledgebase_service::browser::KnowledgeBrowserService;
use sdkwork_intelligence_knowledgebase_service::ports::knowledge_access_control::{
    GrantKnowledgeSpaceAccessRequest, KnowledgeAccessCheckRequest, KnowledgeAccessControl,
    KnowledgeAccessControlError, KnowledgeAccessGrant, KnowledgeNodeAccessCheckRequest,
    KnowledgeSpaceMemberList, ListKnowledgeSpaceMembersRequest, RevokeKnowledgeSpaceAccessRequest,
};
use sdkwork_intelligence_knowledgebase_service::ports::knowledge_browser_projection_store::{
    KnowledgeBrowserDocumentProjection, KnowledgeBrowserOkfConceptProjection,
    KnowledgeBrowserProjectionStore, KnowledgeBrowserProjectionStoreError,
};
use sdkwork_intelligence_knowledgebase_service::ports::knowledge_drive_node_tree::{
    DriveNodeKind, GetKnowledgeDriveNodeRequest, KnowledgeDriveNodePage, KnowledgeDriveNodeSummary,
    KnowledgeDriveNodeTree, KnowledgeDriveNodeTreeError, ListKnowledgeDriveNodeChildrenRequest,
    ResolveKnowledgeDriveNodePathRequest,
};
use sdkwork_intelligence_knowledgebase_service::ports::knowledge_space_store::{
    CreateKnowledgeSpaceRecord, KnowledgeSpaceStore, KnowledgeSpaceStoreError,
};
use sdkwork_knowledgebase_contract::browser::{
    KnowledgeBrowserNodeType, KnowledgeBrowserView, ListKnowledgeBrowserRequest,
};
use sdkwork_knowledgebase_contract::rag::KnowledgeAgentKnowledgeMode;
use sdkwork_knowledgebase_contract::space::{KnowledgeSpace, KnowledgeSpaceStatus};
use sdkwork_knowledgebase_contract::OkfConceptPublishState;
use std::collections::HashMap;
use std::sync::Mutex;

#[tokio::test]
async fn browser_lists_drive_children_and_batches_document_projection() {
    let spaces = MemorySpaceStore::bound("drv-kb-001");
    let drive_tree = RecordingDriveTree::with_nodes(vec![
        KnowledgeDriveNodeSummary {
            drive_node_id: "node-folder".to_string(),
            parent_drive_node_id: Some("node-raw-root".to_string()),
            kind: DriveNodeKind::Folder,
            name: "Papers".to_string(),
            path: "sources/raw/Papers".to_string(),
            content_type: None,
            size_bytes: None,
            children_count: Some(3),
            updated_at: "2026-06-04T12:00:00Z".to_string(),
            object_locator: None,
        },
        KnowledgeDriveNodeSummary {
            drive_node_id: "node-pdf".to_string(),
            parent_drive_node_id: Some("node-raw-root".to_string()),
            kind: DriveNodeKind::File,
            name: "LLM-Wiki Paper.pdf".to_string(),
            path: "sources/raw/Papers/LLM-Wiki Paper.pdf".to_string(),
            content_type: Some("application/pdf".to_string()),
            size_bytes: Some(42),
            children_count: None,
            updated_at: "2026-06-04T12:01:00Z".to_string(),
            object_locator: None,
        },
    ])
    .with_node(KnowledgeDriveNodeSummary {
        drive_node_id: "node-raw-root".to_string(),
        parent_drive_node_id: None,
        kind: DriveNodeKind::Folder,
        name: "raw".to_string(),
        path: "sources/raw".to_string(),
        content_type: None,
        size_bytes: None,
        children_count: Some(2),
        updated_at: "2026-06-04T12:00:00Z".to_string(),
        object_locator: None,
    });
    let projections =
        RecordingProjectionStore::with_documents(vec![KnowledgeBrowserDocumentProjection {
            drive_node_id: "node-pdf".to_string(),
            document_id: 11,
            current_version_id: Some(7),
            ingest_state: "completed".to_string(),
            parse_state: "succeeded".to_string(),
            index_state: "indexed".to_string(),
            okf_state: "candidate_ready".to_string(),
        }]);
    let service = KnowledgeBrowserService::new(&spaces, &drive_tree, &projections)
        .with_access_control(&PERMISSIVE_ACCESS_CONTROL);

    let page = service
        .list(
            Some(browser_access()),
            ListKnowledgeBrowserRequest {
                space_id: 1,
                parent_id: Some("node-raw-root".to_string()),
                view: KnowledgeBrowserView::Files,
                cursor: Some("cursor-a".to_string()),
                page_size: Some(50),
            },
        )
        .await
        .unwrap();

    assert_eq!(page.space_id, 1);
    assert_eq!(page.drive_space_id, "drv-kb-001");
    assert_eq!(page.parent_id.as_deref(), Some("node-raw-root"));
    assert_eq!(page.items.len(), 2);
    assert_eq!(page.items[0].node_type, KnowledgeBrowserNodeType::Folder);
    assert_eq!(page.items[0].drive_node_id.as_deref(), Some("node-folder"));
    assert_eq!(page.items[0].children_count, Some(3));
    assert_eq!(page.items[1].node_type, KnowledgeBrowserNodeType::Document);
    assert_eq!(page.items[1].document_id, Some(11));
    assert_eq!(page.items[1].document_version_id, Some(7));
    assert_eq!(page.items[1].ingest_state.as_deref(), Some("completed"));
    assert_eq!(page.next_cursor.as_deref(), Some("next-cursor"));
    assert_eq!(drive_tree.calls(), 1);
    assert_eq!(projections.calls(), 1);
    assert_eq!(
        projections.requested_drive_node_ids(),
        vec!["node-folder".to_string(), "node-pdf".to_string()]
    );
}

#[tokio::test]
async fn browser_files_root_lists_original_files_under_okf_raw_sources() {
    let spaces = MemorySpaceStore::bound("drv-kb-001");
    let drive_tree = RecordingDriveTree::with_nodes(vec![KnowledgeDriveNodeSummary {
        drive_node_id: "node-raw-note".to_string(),
        parent_drive_node_id: Some("node-raw-root".to_string()),
        kind: DriveNodeKind::File,
        name: "raw-note.md".to_string(),
        path: "sources/raw/raw-note.md".to_string(),
        content_type: Some("text/markdown; charset=utf-8".to_string()),
        size_bytes: Some(128),
        children_count: None,
        updated_at: "2026-06-04T12:04:00Z".to_string(),
        object_locator: None,
    }])
    .with_resolved_path("sources/raw", Some("node-raw-root"))
    .expect_parent_id(Some("node-raw-root"));
    let projections =
        RecordingProjectionStore::with_documents(vec![KnowledgeBrowserDocumentProjection {
            drive_node_id: "node-raw-note".to_string(),
            document_id: 91,
            current_version_id: Some(92),
            ingest_state: "completed".to_string(),
            parse_state: "succeeded".to_string(),
            index_state: "indexed".to_string(),
            okf_state: "candidate_ready".to_string(),
        }]);
    let service = KnowledgeBrowserService::new(&spaces, &drive_tree, &projections)
        .with_access_control(&PERMISSIVE_ACCESS_CONTROL);

    let page = service
        .list(
            Some(browser_access()),
            ListKnowledgeBrowserRequest {
                space_id: 1,
                parent_id: None,
                view: KnowledgeBrowserView::Files,
                cursor: None,
                page_size: Some(50),
            },
        )
        .await
        .unwrap();

    assert_eq!(page.view, KnowledgeBrowserView::Files);
    assert_eq!(page.parent_id.as_deref(), Some("node-raw-root"));
    assert_eq!(page.items.len(), 1);
    assert_eq!(page.items[0].node_type, KnowledgeBrowserNodeType::Document);
    assert_eq!(page.items[0].document_id, Some(91));
    assert_eq!(drive_tree.resolved_paths(), vec!["sources/raw".to_string()]);
    assert_eq!(drive_tree.calls(), 1);
    assert_eq!(
        projections.requested_drive_node_ids(),
        vec!["node-raw-note".to_string()]
    );
}

#[tokio::test]
async fn browser_files_root_returns_empty_page_when_okf_raw_source_root_is_missing() {
    let spaces = MemorySpaceStore::bound("drv-kb-001");
    let drive_tree = RecordingDriveTree::with_nodes(vec![KnowledgeDriveNodeSummary {
        drive_node_id: "node-okf-root".to_string(),
        parent_drive_node_id: None,
        kind: DriveNodeKind::Folder,
        name: "okf".to_string(),
        path: "okf".to_string(),
        content_type: None,
        size_bytes: None,
        children_count: Some(1),
        updated_at: "2026-06-04T12:05:00Z".to_string(),
        object_locator: None,
    }])
    .with_resolved_path("sources/raw", None);
    let projections = RecordingProjectionStore::default();
    let service = KnowledgeBrowserService::new(&spaces, &drive_tree, &projections)
        .with_access_control(&PERMISSIVE_ACCESS_CONTROL);

    let page = service
        .list(
            Some(browser_access()),
            ListKnowledgeBrowserRequest {
                space_id: 1,
                parent_id: None,
                view: KnowledgeBrowserView::Files,
                cursor: None,
                page_size: Some(50),
            },
        )
        .await
        .unwrap();

    assert_eq!(page.view, KnowledgeBrowserView::Files);
    assert_eq!(page.parent_id, None);
    assert!(page.items.is_empty());
    assert_eq!(page.next_cursor, None);
    assert_eq!(drive_tree.resolved_paths(), vec!["sources/raw".to_string()]);
    assert_eq!(drive_tree.calls(), 0);
    assert_eq!(projections.calls(), 0);
}

#[tokio::test]
async fn browser_rejects_files_parent_id_outside_okf_raw_sources() {
    let spaces = MemorySpaceStore::bound("drv-kb-001");
    let drive_tree = RecordingDriveTree::with_nodes(vec![]).with_node(KnowledgeDriveNodeSummary {
        drive_node_id: "node-okf-root".to_string(),
        parent_drive_node_id: None,
        kind: DriveNodeKind::Folder,
        name: "okf".to_string(),
        path: "okf".to_string(),
        content_type: None,
        size_bytes: None,
        children_count: Some(1),
        updated_at: "2026-06-04T12:05:00Z".to_string(),
        object_locator: None,
    });
    let projections = RecordingProjectionStore::default();
    let service = KnowledgeBrowserService::new(&spaces, &drive_tree, &projections)
        .with_access_control(&PERMISSIVE_ACCESS_CONTROL);

    let error = service
        .list(
            Some(browser_access()),
            ListKnowledgeBrowserRequest {
                space_id: 1,
                parent_id: Some("node-okf-root".to_string()),
                view: KnowledgeBrowserView::Files,
                cursor: None,
                page_size: Some(50),
            },
        )
        .await
        .unwrap_err();

    assert!(error.to_string().contains("outside files view"));
    assert_eq!(drive_tree.calls(), 0);
    assert_eq!(projections.calls(), 0);
}

#[tokio::test]
async fn browser_files_root_keeps_drive_root_for_external_knowledge_spaces() {
    let spaces =
        MemorySpaceStore::bound_with_mode("drv-kb-001", KnowledgeAgentKnowledgeMode::External);
    let drive_tree = RecordingDriveTree::with_nodes(vec![KnowledgeDriveNodeSummary {
        drive_node_id: "node-external-folder".to_string(),
        parent_drive_node_id: None,
        kind: DriveNodeKind::Folder,
        name: "ExternalProvider".to_string(),
        path: "ExternalProvider".to_string(),
        content_type: None,
        size_bytes: None,
        children_count: Some(0),
        updated_at: "2026-06-04T12:05:00Z".to_string(),
        object_locator: None,
    }])
    .expect_parent_id(None);
    let projections = RecordingProjectionStore::default();
    let service = KnowledgeBrowserService::new(&spaces, &drive_tree, &projections)
        .with_access_control(&PERMISSIVE_ACCESS_CONTROL);

    let page = service
        .list(
            Some(browser_access()),
            ListKnowledgeBrowserRequest {
                space_id: 1,
                parent_id: None,
                view: KnowledgeBrowserView::Files,
                cursor: None,
                page_size: Some(50),
            },
        )
        .await
        .unwrap();

    assert_eq!(page.view, KnowledgeBrowserView::Files);
    assert_eq!(page.parent_id, None);
    assert_eq!(page.items.len(), 1);
    assert_eq!(page.items[0].name, "ExternalProvider");
    assert!(drive_tree.resolved_paths().is_empty());
    assert_eq!(drive_tree.calls(), 1);
}

#[tokio::test]
async fn browser_rejects_missing_files_parent_id() {
    let spaces = MemorySpaceStore::bound("drv-kb-001");
    let drive_tree = RecordingDriveTree::with_nodes(vec![]);
    let projections = RecordingProjectionStore::default();
    let service = KnowledgeBrowserService::new(&spaces, &drive_tree, &projections)
        .with_access_control(&PERMISSIVE_ACCESS_CONTROL);

    let error = service
        .list(
            Some(browser_access()),
            ListKnowledgeBrowserRequest {
                space_id: 1,
                parent_id: Some("missing-parent".to_string()),
                view: KnowledgeBrowserView::Files,
                cursor: None,
                page_size: Some(50),
            },
        )
        .await
        .unwrap_err();

    assert!(error.to_string().contains("browser parent node is missing"));
    assert_eq!(drive_tree.calls(), 0);
    assert_eq!(projections.calls(), 0);
}

#[tokio::test]
async fn browser_rejects_file_as_files_parent_id() {
    let spaces = MemorySpaceStore::bound("drv-kb-001");
    let drive_tree = RecordingDriveTree::with_nodes(vec![]).with_node(KnowledgeDriveNodeSummary {
        drive_node_id: "node-pdf".to_string(),
        parent_drive_node_id: Some("root".to_string()),
        kind: DriveNodeKind::File,
        name: "LLM-Wiki Paper.pdf".to_string(),
        path: "Files/Papers/LLM-Wiki Paper.pdf".to_string(),
        content_type: Some("application/pdf".to_string()),
        size_bytes: Some(42),
        children_count: None,
        updated_at: "2026-06-04T12:01:00Z".to_string(),
        object_locator: None,
    });
    let projections = RecordingProjectionStore::default();
    let service = KnowledgeBrowserService::new(&spaces, &drive_tree, &projections)
        .with_access_control(&PERMISSIVE_ACCESS_CONTROL);

    let error = service
        .list(
            Some(browser_access()),
            ListKnowledgeBrowserRequest {
                space_id: 1,
                parent_id: Some("node-pdf".to_string()),
                view: KnowledgeBrowserView::Files,
                cursor: None,
                page_size: Some(50),
            },
        )
        .await
        .unwrap_err();

    assert!(error
        .to_string()
        .contains("browser parent node must be a folder"));
    assert_eq!(drive_tree.calls(), 0);
    assert_eq!(projections.calls(), 0);
}

#[tokio::test]
async fn browser_caps_page_size_to_prevent_unbounded_directory_scans() {
    let spaces = MemorySpaceStore::bound("drv-kb-001");
    let drive_tree = RecordingDriveTree::with_nodes(vec![])
        .with_resolved_path("sources/raw", Some("node-raw-root"))
        .expect_parent_id(Some("node-raw-root"));
    let projections = RecordingProjectionStore::default();
    let service = KnowledgeBrowserService::new(&spaces, &drive_tree, &projections)
        .with_access_control(&PERMISSIVE_ACCESS_CONTROL);

    // Oversized page sizes are rejected (PAGINATION_SPEC §10.1), never silently clamped,
    // so an unbounded directory scan cannot hide behind a compliant-looking page size.
    let error = service
        .list(
            Some(browser_access()),
            ListKnowledgeBrowserRequest {
                space_id: 1,
                parent_id: None,
                view: KnowledgeBrowserView::Files,
                cursor: None,
                page_size: Some(10_000),
            },
        )
        .await
        .expect_err("page_size above the maximum must be rejected");

    assert!(error.to_string().contains("page_size"));
    assert_eq!(drive_tree.last_page_size(), None);
    assert!(drive_tree.resolved_paths().is_empty());
}

#[tokio::test]
async fn browser_okf_root_lists_children_under_okf_drive_folder() {
    let spaces = MemorySpaceStore::bound("drv-kb-001");
    let drive_tree = RecordingDriveTree::with_nodes(vec![KnowledgeDriveNodeSummary {
        drive_node_id: "node-index".to_string(),
        parent_drive_node_id: Some("node-okf-root".to_string()),
        kind: DriveNodeKind::File,
        name: "index.md".to_string(),
        path: "okf/index.md".to_string(),
        content_type: Some("text/markdown; charset=utf-8".to_string()),
        size_bytes: Some(128),
        children_count: None,
        updated_at: "2026-06-04T12:02:00Z".to_string(),
        object_locator: None,
    }])
    .with_resolved_path("okf", Some("node-okf-root"))
    .expect_parent_id(Some("node-okf-root"));
    let projections =
        RecordingProjectionStore::with_okf_concepts(vec![KnowledgeBrowserOkfConceptProjection {
            logical_path: "okf/index.md".to_string(),
            concept_row_id: 21,
            current_revision_id: Some(34),
            publish_state: OkfConceptPublishState::Published,
        }]);
    let service = KnowledgeBrowserService::new(&spaces, &drive_tree, &projections)
        .with_access_control(&PERMISSIVE_ACCESS_CONTROL);

    let page = service
        .list(
            Some(browser_access()),
            ListKnowledgeBrowserRequest {
                space_id: 1,
                parent_id: None,
                view: KnowledgeBrowserView::OkfBundle,
                cursor: None,
                page_size: Some(50),
            },
        )
        .await
        .unwrap();

    assert_eq!(page.view, KnowledgeBrowserView::OkfBundle);
    assert_eq!(page.parent_id.as_deref(), Some("node-okf-root"));
    assert_eq!(page.items.len(), 1);
    assert_eq!(
        page.items[0].node_type,
        KnowledgeBrowserNodeType::OkfConcept
    );
    assert_eq!(page.items[0].name, "index.md");
    assert_eq!(page.items[0].concept_id, Some(21));
    assert_eq!(page.items[0].concept_revision_id, Some(34));
    assert_eq!(page.items[0].okf_state.as_deref(), Some("published"));
    assert_eq!(drive_tree.resolved_paths(), vec!["okf".to_string()]);
    assert_eq!(drive_tree.calls(), 1);
    assert_eq!(projections.calls(), 1);
    assert_eq!(projections.okf_calls(), 1);
}

#[tokio::test]
async fn browser_outputs_root_lists_children_under_standard_output_drive_folder() {
    let spaces = MemorySpaceStore::bound("drv-kb-001");
    let drive_tree = RecordingDriveTree::with_nodes(vec![KnowledgeDriveNodeSummary {
        drive_node_id: "node-answer".to_string(),
        parent_drive_node_id: Some("node-output-root".to_string()),
        kind: DriveNodeKind::Folder,
        name: "answers".to_string(),
        path: "output/answers".to_string(),
        content_type: None,
        size_bytes: None,
        children_count: Some(12),
        updated_at: "2026-06-04T12:03:00Z".to_string(),
        object_locator: None,
    }])
    .with_resolved_path("output", Some("node-output-root"))
    .expect_parent_id(Some("node-output-root"));
    let projections = RecordingProjectionStore::default();
    let service = KnowledgeBrowserService::new(&spaces, &drive_tree, &projections)
        .with_access_control(&PERMISSIVE_ACCESS_CONTROL);

    let page = service
        .list(
            Some(browser_access()),
            ListKnowledgeBrowserRequest {
                space_id: 1,
                parent_id: None,
                view: KnowledgeBrowserView::Outputs,
                cursor: None,
                page_size: Some(50),
            },
        )
        .await
        .unwrap();

    assert_eq!(page.view, KnowledgeBrowserView::Outputs);
    assert_eq!(page.parent_id.as_deref(), Some("node-output-root"));
    assert_eq!(page.items.len(), 1);
    assert_eq!(
        page.items[0].node_type,
        KnowledgeBrowserNodeType::VirtualFolder
    );
    assert_eq!(page.items[0].name, "answers");
    assert_eq!(drive_tree.resolved_paths(), vec!["output".to_string()]);
    assert_eq!(drive_tree.calls(), 1);
}

#[tokio::test]
async fn browser_rejects_okf_parent_id_outside_okf_drive_tree() {
    let spaces = MemorySpaceStore::bound("drv-kb-001");
    let drive_tree = RecordingDriveTree::with_nodes(vec![KnowledgeDriveNodeSummary {
        drive_node_id: "node-source-file".to_string(),
        parent_drive_node_id: Some("node-sources-root".to_string()),
        kind: DriveNodeKind::File,
        name: "raw-note.md".to_string(),
        path: "sources/raw/raw-note.md".to_string(),
        content_type: Some("text/markdown; charset=utf-8".to_string()),
        size_bytes: Some(128),
        children_count: None,
        updated_at: "2026-06-04T12:04:00Z".to_string(),
        object_locator: None,
    }])
    .with_node(KnowledgeDriveNodeSummary {
        drive_node_id: "node-sources-root".to_string(),
        parent_drive_node_id: None,
        kind: DriveNodeKind::Folder,
        name: "raw".to_string(),
        path: "sources/raw".to_string(),
        content_type: None,
        size_bytes: None,
        children_count: Some(1),
        updated_at: "2026-06-04T12:04:00Z".to_string(),
        object_locator: None,
    });
    let projections = RecordingProjectionStore::default();
    let service = KnowledgeBrowserService::new(&spaces, &drive_tree, &projections)
        .with_access_control(&PERMISSIVE_ACCESS_CONTROL);

    let error = service
        .list(
            Some(browser_access()),
            ListKnowledgeBrowserRequest {
                space_id: 1,
                parent_id: Some("node-sources-root".to_string()),
                view: KnowledgeBrowserView::OkfBundle,
                cursor: None,
                page_size: Some(50),
            },
        )
        .await
        .unwrap_err();

    assert!(error.to_string().contains("outside okf view"));
    assert_eq!(drive_tree.calls(), 0);
    assert_eq!(projections.calls(), 0);
    assert_eq!(projections.okf_calls(), 0);
}

#[tokio::test]
async fn browser_rejects_outputs_parent_id_outside_output_drive_tree_even_when_empty() {
    let spaces = MemorySpaceStore::bound("drv-kb-001");
    let drive_tree = RecordingDriveTree::with_nodes(vec![]).with_node(KnowledgeDriveNodeSummary {
        drive_node_id: "node-misc-root".to_string(),
        parent_drive_node_id: None,
        kind: DriveNodeKind::Folder,
        name: "misc".to_string(),
        path: "misc".to_string(),
        content_type: None,
        size_bytes: None,
        children_count: Some(0),
        updated_at: "2026-06-04T12:04:00Z".to_string(),
        object_locator: None,
    });
    let projections = RecordingProjectionStore::default();
    let service = KnowledgeBrowserService::new(&spaces, &drive_tree, &projections)
        .with_access_control(&PERMISSIVE_ACCESS_CONTROL);

    let error = service
        .list(
            Some(browser_access()),
            ListKnowledgeBrowserRequest {
                space_id: 1,
                parent_id: Some("node-misc-root".to_string()),
                view: KnowledgeBrowserView::Outputs,
                cursor: None,
                page_size: Some(50),
            },
        )
        .await
        .unwrap_err();

    assert!(error.to_string().contains("outside outputs view"));
    assert_eq!(drive_tree.calls(), 0);
    assert_eq!(projections.calls(), 0);
}

#[tokio::test]
async fn browser_rejects_spaces_without_drive_space_binding() {
    let spaces = MemorySpaceStore::unbound();
    let drive_tree = RecordingDriveTree::with_nodes(vec![]);
    let projections = RecordingProjectionStore::default();
    let service = KnowledgeBrowserService::new(&spaces, &drive_tree, &projections)
        .with_access_control(&PERMISSIVE_ACCESS_CONTROL);

    let error = service
        .list(
            Some(browser_access()),
            ListKnowledgeBrowserRequest {
                space_id: 1,
                parent_id: None,
                view: KnowledgeBrowserView::Files,
                cursor: None,
                page_size: None,
            },
        )
        .await
        .unwrap_err();

    assert!(error.to_string().contains("drive space is not bound"));
    assert_eq!(drive_tree.calls(), 0);
    assert_eq!(projections.calls(), 0);
}

struct MemorySpaceStore {
    space: Mutex<KnowledgeSpace>,
}

impl MemorySpaceStore {
    fn bound(drive_space_id: &str) -> Self {
        Self::bound_with_mode(drive_space_id, KnowledgeAgentKnowledgeMode::OkfBundle)
    }

    fn bound_with_mode(drive_space_id: &str, knowledge_mode: KnowledgeAgentKnowledgeMode) -> Self {
        Self {
            space: Mutex::new(space(Some(drive_space_id.to_string()), knowledge_mode)),
        }
    }

    fn unbound() -> Self {
        Self {
            space: Mutex::new(space(None, KnowledgeAgentKnowledgeMode::OkfBundle)),
        }
    }
}

#[async_trait]
impl KnowledgeSpaceStore for MemorySpaceStore {
    async fn create_space(
        &self,
        _record: CreateKnowledgeSpaceRecord,
    ) -> Result<KnowledgeSpace, KnowledgeSpaceStoreError> {
        Ok(self.space.lock().unwrap().clone())
    }

    async fn get_space(&self, _space_id: u64) -> Result<KnowledgeSpace, KnowledgeSpaceStoreError> {
        Ok(self.space.lock().unwrap().clone())
    }

    async fn mark_drive_space_bound(
        &self,
        _space_id: u64,
        record: sdkwork_intelligence_knowledgebase_service::ports::knowledge_space_store::BindKnowledgeDriveSpaceRecord,
    ) -> Result<KnowledgeSpace, KnowledgeSpaceStoreError> {
        let mut space = self.space.lock().unwrap();
        space.drive_space_id = Some(record.drive_space_id);
        Ok(space.clone())
    }

    async fn mark_okf_bundle_initialized(
        &self,
        _space_id: u64,
    ) -> Result<KnowledgeSpace, KnowledgeSpaceStoreError> {
        let mut space = self.space.lock().unwrap();
        space.okf_bundle_initialized = true;
        Ok(space.clone())
    }

    async fn mark_space_deleted(&self, _space_id: u64) -> Result<(), KnowledgeSpaceStoreError> {
        let mut space = self.space.lock().unwrap();
        space.status = KnowledgeSpaceStatus::Deleted;
        Ok(())
    }

    async fn update_space(
        &self,
        _space_id: u64,
        _record: sdkwork_intelligence_knowledgebase_service::ports::knowledge_space_store::UpdateKnowledgeSpaceRecord,
    ) -> Result<KnowledgeSpace, KnowledgeSpaceStoreError> {
        Err(KnowledgeSpaceStoreError::Internal(
            "unsupported in test fake".to_string(),
        ))
    }
}

#[derive(Default)]
struct RecordingDriveTree {
    nodes: Vec<KnowledgeDriveNodeSummary>,
    node_bindings: HashMap<String, KnowledgeDriveNodeSummary>,
    resolved_paths: Mutex<Vec<String>>,
    path_bindings: HashMap<String, Option<String>>,
    expected_parent_id: Option<Option<String>>,
    calls: Mutex<u32>,
    last_page_size: Mutex<Option<u32>>,
}

impl RecordingDriveTree {
    fn with_nodes(nodes: Vec<KnowledgeDriveNodeSummary>) -> Self {
        Self {
            nodes,
            node_bindings: HashMap::new(),
            resolved_paths: Mutex::new(vec![]),
            path_bindings: HashMap::new(),
            expected_parent_id: None,
            calls: Mutex::new(0),
            last_page_size: Mutex::new(None),
        }
    }

    fn with_resolved_path(mut self, logical_path: &str, drive_node_id: Option<&str>) -> Self {
        self.path_bindings.insert(
            logical_path.to_string(),
            drive_node_id.map(std::string::ToString::to_string),
        );
        self
    }

    fn with_node(mut self, node: KnowledgeDriveNodeSummary) -> Self {
        self.node_bindings.insert(node.drive_node_id.clone(), node);
        self
    }

    fn expect_parent_id(mut self, parent_id: Option<&str>) -> Self {
        self.expected_parent_id = Some(parent_id.map(std::string::ToString::to_string));
        self
    }

    fn calls(&self) -> u32 {
        *self.calls.lock().unwrap()
    }

    fn last_page_size(&self) -> Option<u32> {
        *self.last_page_size.lock().unwrap()
    }

    fn resolved_paths(&self) -> Vec<String> {
        self.resolved_paths.lock().unwrap().clone()
    }
}

#[async_trait]
impl KnowledgeDriveNodeTree for RecordingDriveTree {
    async fn resolve_path(
        &self,
        request: ResolveKnowledgeDriveNodePathRequest,
    ) -> Result<Option<KnowledgeDriveNodeSummary>, KnowledgeDriveNodeTreeError> {
        assert_eq!(request.drive_space_id, "drv-kb-001");
        self.resolved_paths
            .lock()
            .unwrap()
            .push(request.logical_path.clone());

        Ok(self
            .path_bindings
            .get(&request.logical_path)
            .and_then(|drive_node_id| {
                drive_node_id
                    .as_ref()
                    .map(|drive_node_id| KnowledgeDriveNodeSummary {
                        drive_node_id: drive_node_id.clone(),
                        parent_drive_node_id: None,
                        kind: DriveNodeKind::Folder,
                        name: request.logical_path.clone(),
                        path: request.logical_path.clone(),
                        content_type: None,
                        size_bytes: None,
                        children_count: Some(self.nodes.len() as u64),
                        updated_at: "2026-06-04T12:00:00Z".to_string(),
                        object_locator: None,
                    })
            }))
    }

    async fn get_node(
        &self,
        request: GetKnowledgeDriveNodeRequest,
    ) -> Result<Option<KnowledgeDriveNodeSummary>, KnowledgeDriveNodeTreeError> {
        assert_eq!(request.drive_space_id, "drv-kb-001");
        Ok(self.node_bindings.get(&request.drive_node_id).cloned())
    }

    async fn list_children(
        &self,
        request: ListKnowledgeDriveNodeChildrenRequest,
    ) -> Result<KnowledgeDriveNodePage, KnowledgeDriveNodeTreeError> {
        *self.calls.lock().unwrap() += 1;
        *self.last_page_size.lock().unwrap() = Some(request.page_size);
        assert_eq!(request.drive_space_id, "drv-kb-001");
        if let Some(expected_parent_id) = &self.expected_parent_id {
            assert_eq!(&request.parent_drive_node_id, expected_parent_id);
        }
        Ok(KnowledgeDriveNodePage {
            nodes: self.nodes.clone(),
            next_cursor: Some("next-cursor".to_string()),
        })
    }
}

#[derive(Default)]
struct RecordingProjectionStore {
    documents: Vec<KnowledgeBrowserDocumentProjection>,
    okf_concepts: Vec<KnowledgeBrowserOkfConceptProjection>,
    calls: Mutex<u32>,
    okf_calls: Mutex<u32>,
    requested_drive_node_ids: Mutex<Vec<String>>,
    requested_logical_paths: Mutex<Vec<String>>,
}

impl RecordingProjectionStore {
    fn with_documents(documents: Vec<KnowledgeBrowserDocumentProjection>) -> Self {
        Self {
            documents,
            okf_concepts: vec![],
            calls: Mutex::new(0),
            okf_calls: Mutex::new(0),
            requested_drive_node_ids: Mutex::new(vec![]),
            requested_logical_paths: Mutex::new(vec![]),
        }
    }

    fn with_okf_concepts(okf_concepts: Vec<KnowledgeBrowserOkfConceptProjection>) -> Self {
        Self {
            documents: vec![],
            okf_concepts,
            calls: Mutex::new(0),
            okf_calls: Mutex::new(0),
            requested_drive_node_ids: Mutex::new(vec![]),
            requested_logical_paths: Mutex::new(vec![]),
        }
    }

    fn calls(&self) -> u32 {
        *self.calls.lock().unwrap()
    }

    fn requested_drive_node_ids(&self) -> Vec<String> {
        self.requested_drive_node_ids.lock().unwrap().clone()
    }

    fn okf_calls(&self) -> u32 {
        *self.okf_calls.lock().unwrap()
    }
}

#[async_trait]
impl KnowledgeBrowserProjectionStore for RecordingProjectionStore {
    async fn batch_document_projections(
        &self,
        _space_id: u64,
        drive_node_ids: Vec<String>,
    ) -> Result<Vec<KnowledgeBrowserDocumentProjection>, KnowledgeBrowserProjectionStoreError> {
        *self.calls.lock().unwrap() += 1;
        *self.requested_drive_node_ids.lock().unwrap() = drive_node_ids;
        Ok(self.documents.clone())
    }

    async fn batch_okf_concept_projections(
        &self,
        _space_id: u64,
        logical_paths: Vec<String>,
    ) -> Result<Vec<KnowledgeBrowserOkfConceptProjection>, KnowledgeBrowserProjectionStoreError>
    {
        *self.okf_calls.lock().unwrap() += 1;
        *self.requested_logical_paths.lock().unwrap() = logical_paths;
        Ok(self.okf_concepts.clone())
    }
}

/// Permissive test double: browser tests exercise projection/keyset behavior, not ACL
/// enforcement; fail-closed access-control configuration is covered by service tests.
struct PermissiveKnowledgeAccessControl;

#[async_trait]
impl KnowledgeAccessControl for PermissiveKnowledgeAccessControl {
    async fn check_space_access(
        &self,
        _request: KnowledgeAccessCheckRequest,
    ) -> Result<KnowledgeAccessGrant, KnowledgeAccessControlError> {
        Ok(KnowledgeAccessGrant {
            allowed: true,
            effective_role: None,
        })
    }

    async fn check_node_access(
        &self,
        _request: KnowledgeNodeAccessCheckRequest,
    ) -> Result<KnowledgeAccessGrant, KnowledgeAccessControlError> {
        Ok(KnowledgeAccessGrant {
            allowed: true,
            effective_role: None,
        })
    }

    async fn grant_space_access(
        &self,
        _request: GrantKnowledgeSpaceAccessRequest,
    ) -> Result<(), KnowledgeAccessControlError> {
        Ok(())
    }

    async fn revoke_space_access(
        &self,
        _request: RevokeKnowledgeSpaceAccessRequest,
    ) -> Result<(), KnowledgeAccessControlError> {
        Ok(())
    }

    async fn list_space_members(
        &self,
        _request: ListKnowledgeSpaceMembersRequest,
    ) -> Result<KnowledgeSpaceMemberList, KnowledgeAccessControlError> {
        Ok(KnowledgeSpaceMemberList {
            members: Vec::new(),
            next_cursor: None,
        })
    }
}

static PERMISSIVE_ACCESS_CONTROL: PermissiveKnowledgeAccessControl =
    PermissiveKnowledgeAccessControl;

fn browser_access(
) -> sdkwork_intelligence_knowledgebase_service::browser::KnowledgeBrowserAccessContext {
    sdkwork_intelligence_knowledgebase_service::browser::KnowledgeBrowserAccessContext {
        tenant_id: 1,
        actor_id: "actor-1".to_string(),
    }
}

fn space(
    drive_space_id: Option<String>,
    knowledge_mode: KnowledgeAgentKnowledgeMode,
) -> KnowledgeSpace {
    KnowledgeSpace {
        id: 1,
        uuid: "kb-001".to_string(),
        name: "Research Space".to_string(),
        description: None,
        drive_space_id,
        status: KnowledgeSpaceStatus::Active,
        okf_bundle_initialized: false,
        knowledge_mode,
    }
}
