use async_trait::async_trait;
#[path = "support/okf_pagination.rs"]
mod okf_pagination_support;

use okf_pagination_support::validated_okf_test_page_size;
use sdkwork_intelligence_knowledgebase_service::okf::{
    lint_bundle_summaries, lint_published_concept_markdown, render_okf_concept_markdown,
    to_contract_lint_result, OkfBundleLinterService, OkfConceptDocument,
};
use sdkwork_intelligence_knowledgebase_service::ports::knowledge_drive_storage::{
    HeadKnowledgeObjectRequest, KnowledgeDriveStorage, KnowledgeObjectRef, KnowledgeStorageError,
    PutKnowledgeObjectRequest,
};
use sdkwork_intelligence_knowledgebase_service::ports::knowledge_okf_concept_link_store::{
    InboundLinkTargetsPage, KnowledgeOkfConceptLinkStore, KnowledgeOkfConceptLinkStoreError,
    LinkEdgeCursor, LinkEdgePage, ReplaceKnowledgeOkfConceptLinksRecord,
};
use sdkwork_intelligence_knowledgebase_service::ports::knowledge_okf_concept_store::{
    AppendKnowledgeOkfLogEntryRecord, CreateKnowledgeOkfConceptRevisionRecord,
    KnowledgeOkfConceptProjection, KnowledgeOkfConceptStore, KnowledgeOkfConceptStoreError,
    MarkKnowledgeOkfConceptCurrentRevisionRecord, UpsertKnowledgeOkfConceptRecord,
};
use sdkwork_knowledgebase_contract::okf::{
    KnowledgeOkfConcept, KnowledgeOkfConceptRevision, OkfConceptPublishState, OkfConceptSummary,
    OkfLogEntry,
};
use std::collections::HashMap;
use std::sync::Mutex;

#[tokio::test]
async fn bundle_linter_reports_broken_links_and_orphans() {
    let drive = MemoryDrive::default();
    let concepts = MemoryOkfConceptStore::default();
    let links = MemoryLinkStore;

    let markdown = render_okf_concept_markdown(&OkfConceptDocument {
        concept_type: "Entity".to_string(),
        title: Some("Entity A".to_string()),
        description: Some("Summary.".to_string()),
        resource: None,
        tags: vec![],
        timestamp: None,
        extensions: Default::default(),
        body: "See [missing](entities/missing.md).".to_string(),
    });
    drive.put("okf/entities/a.md", &markdown).await;

    concepts
        .upsert_concept(UpsertKnowledgeOkfConceptRecord {
            space_id: 1,
            concept_id: "entities/a".to_string(),
            title: "Entity A".to_string(),
            concept_type: "Entity".to_string(),
            logical_path: "okf/entities/a.md".to_string(),
            description: "Summary.".to_string(),
            source_count: 0,
            tags: vec![],
            publish_state: OkfConceptPublishState::Published,
        })
        .await
        .unwrap();
    concepts
        .upsert_concept(UpsertKnowledgeOkfConceptRecord {
            space_id: 1,
            concept_id: "entities/orphan".to_string(),
            title: "Orphan".to_string(),
            concept_type: "Entity".to_string(),
            logical_path: "okf/entities/orphan.md".to_string(),
            description: "Orphan summary.".to_string(),
            source_count: 0,
            tags: vec![],
            publish_state: OkfConceptPublishState::Published,
        })
        .await
        .unwrap();
    drive
        .put(
            "okf/entities/orphan.md",
            &render_okf_concept_markdown(&OkfConceptDocument {
                concept_type: "Entity".to_string(),
                title: Some("Orphan".to_string()),
                description: Some("Orphan summary.".to_string()),
                resource: None,
                tags: vec![],
                timestamp: None,
                extensions: Default::default(),
                body: "Standalone.".to_string(),
            }),
        )
        .await;

    let report = OkfBundleLinterService::new(&drive, &concepts)
        .with_link_store(&links)
        .lint_space(1, None)
        .await
        .unwrap();
    let contract = to_contract_lint_result(&report);
    assert!(contract
        .issues
        .iter()
        .any(|issue| issue.code == "broken_links"));
    assert!(contract
        .issues
        .iter()
        .any(|issue| issue.code == "orphan_concepts"));
}

#[tokio::test]
async fn paged_concept_fake_lists_only_published_concepts() {
    let concepts = MemoryOkfConceptStore::default();
    for (concept_id, publish_state) in [
        ("entities/candidate", OkfConceptPublishState::CandidateReady),
        ("entities/published", OkfConceptPublishState::Published),
    ] {
        concepts
            .upsert_concept(UpsertKnowledgeOkfConceptRecord {
                space_id: 1,
                concept_id: concept_id.to_string(),
                title: concept_id.to_string(),
                concept_type: "Entity".to_string(),
                logical_path: format!("okf/{concept_id}.md"),
                description: "Summary.".to_string(),
                source_count: 0,
                tags: vec![],
                publish_state,
            })
            .await
            .unwrap();
    }

    let (items, next_cursor, has_more) = concepts
        .list_concept_summaries_page(1, None, 20)
        .await
        .unwrap();

    assert_eq!(
        items
            .iter()
            .map(|item| item.concept_id.as_str())
            .collect::<Vec<_>>(),
        vec!["entities/published"]
    );
    assert!(next_cursor.is_none());
    assert!(!has_more);
}

#[test]
fn lint_published_concept_detects_missing_frontmatter() {
    let issues = lint_published_concept_markdown("entities/a", "# No frontmatter\n", &[]);
    assert!(issues.iter().any(|issue| issue.check == "okf_conformance"));
}

#[test]
fn lint_bundle_summaries_flags_missing_description() {
    let report = lint_bundle_summaries(
        &[OkfConceptSummary {
            title: "Entity".to_string(),
            concept_id: "entities/a".to_string(),
            concept_type: "Entity".to_string(),
            logical_path: "okf/entities/a.md".to_string(),
            bundle_relative_path: "entities/a.md".to_string(),
            description: String::new(),
            source_count: 0,
            updated_at: "2026-06-01T00:00:00Z".to_string(),
            tags: vec![],
        }],
        &[],
    );
    assert!(report
        .issues
        .iter()
        .any(|issue| issue.check == "missing_citations"));
}

#[derive(Default)]
struct MemoryDrive {
    objects: Mutex<HashMap<String, String>>,
}

impl MemoryDrive {
    async fn put(&self, logical_path: &str, body: &str) {
        self.objects
            .lock()
            .unwrap()
            .insert(logical_path.to_string(), body.to_string());
    }
}

#[async_trait]
impl KnowledgeDriveStorage for MemoryDrive {
    async fn put_object(
        &self,
        request: PutKnowledgeObjectRequest,
    ) -> Result<KnowledgeObjectRef, KnowledgeStorageError> {
        let body = String::from_utf8_lossy(&request.body).into_owned();
        self.put(&request.logical_path, &body).await;
        Ok(KnowledgeObjectRef {
            storage_provider_id: "provider-kb".to_string(),
            bucket: "test".to_string(),
            object_key: request.logical_path.clone(),
            logical_path: request.logical_path,
            object_role: request.object_role,
            content_type: request.content_type,
            size_bytes: request.body.len() as u64,
            checksum_sha256_hex: request.checksum_sha256_hex,
            etag: None,
            version_id: None,
        })
    }

    async fn head_object(
        &self,
        request: HeadKnowledgeObjectRequest,
    ) -> Result<KnowledgeObjectRef, KnowledgeStorageError> {
        let logical_path = request
            .logical_path
            .as_deref()
            .ok_or_else(|| KnowledgeStorageError::internal("missing logical_path"))?;
        if !self.objects.lock().unwrap().contains_key(logical_path) {
            return Err(KnowledgeStorageError::internal("missing object"));
        }
        Ok(KnowledgeObjectRef {
            storage_provider_id: "provider-kb".to_string(),
            bucket: "test".to_string(),
            object_key: logical_path.to_string(),
            logical_path: logical_path.to_string(),
            object_role: request.object_role,
            content_type: "text/markdown; charset=utf-8".to_string(),
            size_bytes: 0,
            checksum_sha256_hex: None,
            etag: None,
            version_id: None,
        })
    }

    async fn get_object_text(
        &self,
        object_ref: &KnowledgeObjectRef,
    ) -> Result<String, KnowledgeStorageError> {
        self.objects
            .lock()
            .unwrap()
            .get(&object_ref.logical_path)
            .cloned()
            .ok_or_else(|| KnowledgeStorageError::internal("missing object"))
    }
}

#[derive(Default)]
struct MemoryLinkStore;

#[async_trait]
impl KnowledgeOkfConceptLinkStore for MemoryLinkStore {
    async fn replace_outbound_links(
        &self,
        _record: ReplaceKnowledgeOkfConceptLinksRecord,
    ) -> Result<(), KnowledgeOkfConceptLinkStoreError> {
        Ok(())
    }

    async fn list_inbound_concept_ids(
        &self,
        _space_id: u64,
        _to_concept_id: &str,
    ) -> Result<Vec<String>, KnowledgeOkfConceptLinkStoreError> {
        Ok(vec![])
    }

    async fn list_inbound_link_targets_page(
        &self,
        _space_id: u64,
        _after_concept_id: Option<&str>,
        _limit: u32,
    ) -> Result<InboundLinkTargetsPage, KnowledgeOkfConceptLinkStoreError> {
        // The bundle linter fixture marks `entities/orphan` as orphaned by
        // providing no inbound target at all.
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

#[derive(Default)]
struct MemoryOkfConceptStore {
    concepts: Mutex<Vec<KnowledgeOkfConcept>>,
}

#[async_trait]
impl KnowledgeOkfConceptStore for MemoryOkfConceptStore {
    async fn upsert_concept(
        &self,
        record: UpsertKnowledgeOkfConceptRecord,
    ) -> Result<KnowledgeOkfConcept, KnowledgeOkfConceptStoreError> {
        let bundle_relative_path = record
            .logical_path
            .strip_prefix("okf/")
            .unwrap_or(&record.logical_path)
            .to_string();
        let concept = KnowledgeOkfConcept {
            id: self.concepts.lock().unwrap().len() as u64 + 1,
            space_id: record.space_id,
            concept_id: record.concept_id,
            title: record.title,
            concept_type: record.concept_type,
            logical_path: record.logical_path,
            bundle_relative_path,
            description: record.description,
            source_count: record.source_count,
            tags: record.tags,
            current_revision_id: None,
            publish_state: record.publish_state,
            updated_at: "2026-06-01T00:00:00Z".to_string(),
        };
        self.concepts.lock().unwrap().push(concept.clone());
        Ok(concept)
    }

    async fn create_revision(
        &self,
        _record: CreateKnowledgeOkfConceptRevisionRecord,
    ) -> Result<KnowledgeOkfConceptRevision, KnowledgeOkfConceptStoreError> {
        Err(KnowledgeOkfConceptStoreError::Internal(
            "unsupported in test fake".to_string(),
        ))
    }

    async fn next_revision_no(
        &self,
        _concept_row_id: u64,
    ) -> Result<u64, KnowledgeOkfConceptStoreError> {
        Ok(1)
    }

    async fn mark_current_revision(
        &self,
        _record: MarkKnowledgeOkfConceptCurrentRevisionRecord,
    ) -> Result<KnowledgeOkfConcept, KnowledgeOkfConceptStoreError> {
        Err(KnowledgeOkfConceptStoreError::Internal(
            "unsupported in test fake".to_string(),
        ))
    }

    async fn list_concept_summaries(
        &self,
        space_id: u64,
        limit: Option<u32>,
    ) -> Result<Vec<OkfConceptSummary>, KnowledgeOkfConceptStoreError> {
        let mut summaries = self
            .concepts
            .lock()
            .unwrap()
            .iter()
            .filter(|concept| concept.space_id == space_id)
            .map(|concept| OkfConceptSummary {
                title: concept.title.clone(),
                concept_id: concept.concept_id.clone(),
                concept_type: concept.concept_type.clone(),
                logical_path: concept.logical_path.clone(),
                bundle_relative_path: concept.bundle_relative_path.clone(),
                description: concept.description.clone(),
                source_count: concept.source_count,
                updated_at: concept.updated_at.clone(),
                tags: concept.tags.clone(),
            })
            .collect::<Vec<_>>();
        if let Some(limit) = limit {
            summaries.truncate(limit.max(1) as usize);
        }
        Ok(summaries)
    }

    async fn list_concept_summaries_page(
        &self,
        space_id: u64,
        cursor: Option<String>,
        page_size: u32,
    ) -> Result<(Vec<OkfConceptSummary>, Option<String>, bool), KnowledgeOkfConceptStoreError> {
        let page_size = validated_okf_test_page_size(page_size)?;
        let fetch_size = page_size + 1;
        let concepts = self.concepts.lock().unwrap();
        let mut summaries = Vec::with_capacity(fetch_size);

        for concept in concepts.iter().filter(|concept| {
            concept.space_id == space_id
                && concept.publish_state == OkfConceptPublishState::Published
                && match cursor.as_ref() {
                    Some(cursor) => concept.concept_id.as_str() > cursor.as_str(),
                    None => true,
                }
        }) {
            let summary = OkfConceptSummary {
                title: concept.title.clone(),
                concept_id: concept.concept_id.clone(),
                concept_type: concept.concept_type.clone(),
                logical_path: concept.logical_path.clone(),
                bundle_relative_path: concept.bundle_relative_path.clone(),
                description: concept.description.clone(),
                source_count: concept.source_count,
                updated_at: concept.updated_at.clone(),
                tags: concept.tags.clone(),
            };
            let index = summaries
                .partition_point(|item: &OkfConceptSummary| item.concept_id <= summary.concept_id);
            summaries.insert(index, summary);
            if summaries.len() > fetch_size {
                summaries.pop();
            }
        }

        let has_more = summaries.len() > page_size;
        summaries.truncate(page_size);
        let next_cursor = if has_more {
            summaries.last().map(|item| item.concept_id.clone())
        } else {
            None
        };
        Ok((summaries, next_cursor, has_more))
    }

    async fn list_concept_revisions_page(
        &self,
        _concept_row_id: u64,
        _cursor: Option<u64>,
        page_size: u32,
    ) -> Result<(Vec<KnowledgeOkfConceptRevision>, Option<u64>, bool), KnowledgeOkfConceptStoreError>
    {
        validated_okf_test_page_size(page_size)?;
        Ok((Vec::new(), None, false))
    }

    async fn append_log_entry(
        &self,
        _record: AppendKnowledgeOkfLogEntryRecord,
    ) -> Result<OkfLogEntry, KnowledgeOkfConceptStoreError> {
        Err(KnowledgeOkfConceptStoreError::Internal(
            "unsupported in test fake".to_string(),
        ))
    }

    async fn list_log_entries(
        &self,
        _space_id: u64,
    ) -> Result<Vec<OkfLogEntry>, KnowledgeOkfConceptStoreError> {
        Ok(vec![])
    }

    async fn batch_concept_projections_by_paths(
        &self,
        _space_id: u64,
        _logical_paths: Vec<String>,
    ) -> Result<Vec<KnowledgeOkfConceptProjection>, KnowledgeOkfConceptStoreError> {
        Ok(vec![])
    }

    async fn mark_concept_deleted(
        &self,
        space_id: u64,
        concept_row_id: u64,
    ) -> Result<KnowledgeOkfConcept, KnowledgeOkfConceptStoreError> {
        let mut concepts = self.concepts.lock().unwrap();
        let index = concepts
            .iter()
            .position(|concept| concept.id == concept_row_id && concept.space_id == space_id)
            .ok_or_else(|| {
                KnowledgeOkfConceptStoreError::NotFound(format!(
                    "missing okf concept: {concept_row_id}"
                ))
            })?;
        Ok(concepts.remove(index))
    }
}
