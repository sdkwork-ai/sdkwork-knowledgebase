use crate::okf::{
    catalog_log, governance_drive, index_concept_links, parse_okf_markdown,
    render_okf_concept_markdown, standard_bundle_catalog_sync, standard_bundle_refresh,
    storage::read_managed_markdown, validate_concept_document, validate_concept_id,
    OkfConceptDocument, OkfConformanceError,
};
use crate::ports::knowledge_drive_object_ref_store::{
    managed_drive_object_ref_record, CreateKnowledgeDriveObjectRefRecord,
};
use crate::ports::knowledge_okf_candidate_store::{
    KnowledgeOkfCandidateStore, KnowledgeOkfCandidateStoreError, UpsertKnowledgeOkfCandidateRecord,
};
use crate::ports::{
    knowledge_drive_storage::{
        KnowledgeDriveStorage, KnowledgeObjectRef, KnowledgeStorageError, PutKnowledgeObjectRequest,
    },
    knowledge_drive_workspace::{
        EnsureKnowledgeDriveNodeRequest, KnowledgeDriveWorkspace, KnowledgeDriveWorkspaceError,
    },
    knowledge_okf_bundle_file_store::{
        KnowledgeOkfBundleFileStore, KnowledgeOkfBundleFileStoreError,
    },
    knowledge_okf_concept_link_store::{
        KnowledgeOkfConceptLinkRecord, KnowledgeOkfConceptLinkStore,
        KnowledgeOkfConceptLinkStoreError, ReplaceKnowledgeOkfConceptLinksRecord,
    },
    knowledge_okf_concept_store::{
        KnowledgeOkfConceptStore, KnowledgeOkfConceptStoreError,
        MarkKnowledgeOkfConceptCurrentRevisionRecord, UpsertKnowledgeOkfConceptRecord,
    },
    okf_concept_revision_metadata_store::{
        OkfConceptRevisionMetadataStore, OkfConceptRevisionMetadataStoreError,
        PublishOkfConceptRevisionMetadataRecord, StageOkfConceptRevisionMetadataRecord,
        UpdateOkfConceptCandidateStateRecord,
    },
};
use sdkwork_knowledgebase_contract::{
    okf::{
        KnowledgeOkfConcept, KnowledgeOkfConceptPublication, KnowledgeOkfConceptRevision,
        OkfBundlePaths, OkfConceptPublishState, OkfConceptUpsertRequest, OkfLogEventType,
        OkfRevisionReviewState, PublishKnowledgeOkfConceptRequest,
    },
    OkfCandidateType,
};
use sdkwork_knowledgebase_observability::{record_okf_concept_publish, record_okf_concept_upsert};
use sdkwork_utils_rust::{is_blank, sha256_hash};
use thiserror::Error;
use time::format_description::well_known::Rfc3339;
use time::OffsetDateTime;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PublishExistingOkfConceptRevisionRequest {
    pub space_id: u64,
    pub concept: KnowledgeOkfConcept,
    pub revision: KnowledgeOkfConceptRevision,
    pub actor: String,
}

#[derive(Debug, Clone, Copy)]
pub struct OkfPublishConceptOptions {
    pub rebuild_standard_files: bool,
}

impl Default for OkfPublishConceptOptions {
    fn default() -> Self {
        Self {
            rebuild_standard_files: true,
        }
    }
}

impl OkfPublishConceptOptions {
    pub fn bundle_import_batch() -> Self {
        Self {
            rebuild_standard_files: false,
        }
    }
}

pub struct OkfConceptService<'a> {
    drive: &'a dyn KnowledgeDriveStorage,
    revision_metadata: &'a dyn OkfConceptRevisionMetadataStore,
    concept_store: &'a dyn KnowledgeOkfConceptStore,
    link_store: Option<&'a dyn KnowledgeOkfConceptLinkStore>,
    candidate_store: Option<&'a dyn KnowledgeOkfCandidateStore>,
    file_entries: Option<&'a dyn KnowledgeOkfBundleFileStore>,
    drive_workspace: Option<&'a dyn KnowledgeDriveWorkspace>,
}

impl<'a> OkfConceptService<'a> {
    pub fn new(
        drive: &'a dyn KnowledgeDriveStorage,
        revision_metadata: &'a dyn OkfConceptRevisionMetadataStore,
        concept_store: &'a dyn KnowledgeOkfConceptStore,
    ) -> Self {
        Self {
            drive,
            revision_metadata,
            concept_store,
            link_store: None,
            candidate_store: None,
            file_entries: None,
            drive_workspace: None,
        }
    }

    pub fn with_link_store(mut self, link_store: &'a dyn KnowledgeOkfConceptLinkStore) -> Self {
        self.link_store = Some(link_store);
        self
    }

    pub fn with_candidate_store(
        mut self,
        candidate_store: &'a dyn KnowledgeOkfCandidateStore,
    ) -> Self {
        self.candidate_store = Some(candidate_store);
        self
    }

    pub fn with_file_entry_store(
        mut self,
        file_entries: &'a dyn KnowledgeOkfBundleFileStore,
    ) -> Self {
        self.file_entries = Some(file_entries);
        self
    }

    pub fn with_drive_workspace(
        mut self,
        drive_workspace: &'a dyn KnowledgeDriveWorkspace,
    ) -> Self {
        self.drive_workspace = Some(drive_workspace);
        self
    }

    pub async fn publish_concept(
        &self,
        request: PublishKnowledgeOkfConceptRequest,
        drive_space_id: Option<&str>,
    ) -> Result<KnowledgeOkfConceptPublication, OkfConceptServiceError> {
        self.publish_concept_with_options(
            request,
            drive_space_id,
            OkfPublishConceptOptions::default(),
        )
        .await
    }

    pub async fn publish_concept_with_options(
        &self,
        request: PublishKnowledgeOkfConceptRequest,
        drive_space_id: Option<&str>,
        options: OkfPublishConceptOptions,
    ) -> Result<KnowledgeOkfConceptPublication, OkfConceptServiceError> {
        let staged = self
            .stage_concept_revision(&request, drive_space_id, true)
            .await?;
        self.finalize_publication(
            request.space_id,
            &request.concept_id,
            &request.title,
            &request.actor,
            &staged,
            drive_space_id,
            options.rebuild_standard_files,
        )
        .await?;
        Ok(staged.publication)
    }

    pub async fn rebuild_bundle_standard_files(
        &self,
        space_id: u64,
        drive_space_id: Option<&str>,
    ) -> Result<(), OkfConceptServiceError> {
        self.rebuild_standard_files(space_id, drive_space_id).await
    }

    pub async fn stage_concept_candidate(
        &self,
        request: PublishKnowledgeOkfConceptRequest,
        drive_space_id: Option<&str>,
    ) -> Result<KnowledgeOkfConceptPublication, OkfConceptServiceError> {
        let staged = self
            .stage_concept_revision(&request, drive_space_id, false)
            .await?;
        Ok(staged.publication)
    }

    pub async fn upsert_concept_from_markdown(
        &self,
        request: OkfConceptUpsertRequest,
        drive_space_id: Option<&str>,
    ) -> Result<KnowledgeOkfConceptPublication, OkfConceptServiceError> {
        if request.space_id == 0 {
            return Err(OkfConceptServiceError::InvalidRequest(
                "space_id is required".to_string(),
            ));
        }
        if is_blank(Some(request.actor.as_str())) {
            return Err(OkfConceptServiceError::InvalidRequest(
                "actor is required".to_string(),
            ));
        }
        if is_blank(Some(request.markdown.as_str())) {
            return Err(OkfConceptServiceError::InvalidRequest(
                "markdown is required".to_string(),
            ));
        }

        let concept_id = normalize_concept_id(&request.concept_id)?;
        let document = parse_okf_markdown(&request.markdown)
            .map_err(|error| OkfConceptServiceError::InvalidRequest(error.to_string()))?
            .ok_or_else(|| {
                OkfConceptServiceError::InvalidRequest(
                    "markdown must include OKF concept frontmatter with type".to_string(),
                )
            })?;
        validate_concept_document(&document, &concept_id)
            .map_err(OkfConceptServiceError::Conformance)?;

        let title = document
            .title
            .clone()
            .filter(|value| !is_blank(Some(value.as_str())))
            .unwrap_or_else(|| title_from_concept_id(&concept_id));
        let publish_request = PublishKnowledgeOkfConceptRequest {
            space_id: request.space_id,
            concept_id,
            title,
            concept_type: document.concept_type,
            description: document.description.unwrap_or_default(),
            markdown: document.body,
            source_count: 0,
            tags: document.tags,
            actor: request.actor,
            resource: document.resource,
            timestamp: document.timestamp,
            frontmatter_extensions: document.extensions,
        };

        let staged = self
            .stage_concept_revision(&publish_request, drive_space_id, request.publish)
            .await?;
        if request.publish {
            self.finalize_publication(
                publish_request.space_id,
                &publish_request.concept_id,
                &publish_request.title,
                &publish_request.actor,
                &staged,
                drive_space_id,
                true,
            )
            .await?;
        }
        Ok(staged.publication)
    }

    pub async fn publish_existing_revision(
        &self,
        request: PublishExistingOkfConceptRevisionRequest,
        drive_space_id: Option<&str>,
    ) -> Result<KnowledgeOkfConceptPublication, OkfConceptServiceError> {
        if request.space_id == 0 {
            return Err(OkfConceptServiceError::InvalidRequest(
                "space_id is required".to_string(),
            ));
        }
        if is_blank(Some(request.actor.as_str())) {
            return Err(OkfConceptServiceError::InvalidRequest(
                "actor is required".to_string(),
            ));
        }
        if request.concept.space_id != request.space_id {
            return Err(OkfConceptServiceError::InvalidRequest(
                "concept does not belong to the requested space".to_string(),
            ));
        }
        if request.concept.publish_state == OkfConceptPublishState::Published {
            return Err(OkfConceptServiceError::InvalidRequest(
                "concept is already published".to_string(),
            ));
        }

        let governance_revision_path = governance_drive::governance_revision_path(
            &request.concept.concept_id,
            request.revision.revision_no,
        );
        let revision_markdown =
            read_managed_markdown(self.drive, &governance_revision_path, drive_space_id)
                .await
                .map_err(|error| {
                    OkfConceptServiceError::Internal(format!(
                        "failed to read governance revision at {governance_revision_path}: {error}"
                    ))
                })?;
        let document = parse_okf_markdown(&revision_markdown)
            .map_err(|error| OkfConceptServiceError::InvalidRequest(error.to_string()))?
            .ok_or_else(|| {
                OkfConceptServiceError::InvalidRequest(
                    "governance revision must include OKF concept frontmatter".to_string(),
                )
            })?;
        validate_concept_document(&document, &request.concept.concept_id)
            .map_err(OkfConceptServiceError::Conformance)?;

        let published_logical_path =
            OkfBundlePaths::concept_logical_path(&request.concept.concept_id);
        let published_ref = self
            .put_markdown(
                &published_logical_path,
                "concept_revision",
                &revision_markdown,
                drive_space_id,
            )
            .await?;
        let published = self
            .revision_metadata
            .publish_existing_revision_metadata(PublishOkfConceptRevisionMetadataRecord {
                published_object_ref: object_ref_record(
                    request.space_id,
                    drive_space_id,
                    None,
                    &published_ref,
                ),
                mark_current: MarkKnowledgeOkfConceptCurrentRevisionRecord {
                    concept_row_id: request.concept.id,
                    revision_id: request.revision.id,
                    publish_state: OkfConceptPublishState::Published,
                },
                candidate_state_update: self.candidate_store.map(|_| {
                    UpdateOkfConceptCandidateStateRecord {
                        concept_row_id: request.concept.id,
                        state: OkfConceptPublishState::Published,
                        reviewer_id: None,
                        review_note: None,
                    }
                }),
            })
            .await?;
        let concept = published.concept;

        let publication = KnowledgeOkfConceptPublication {
            concept,
            revision: request.revision,
            published_logical_path: published_logical_path.clone(),
            governance_revision_path,
        };

        self.ensure_drive_nodes(
            drive_space_id,
            governance_drive::governance_revision_drive_nodes(
                &request.concept.concept_id,
                None,
                Some(&published_ref),
            ),
        )
        .await?;

        let staged = StagedOkfConceptRevision {
            publication,
            concept_document: document,
        };
        self.finalize_publication(
            request.space_id,
            &request.concept.concept_id,
            &request.concept.title,
            &request.actor,
            &staged,
            drive_space_id,
            true,
        )
        .await?;

        Ok(staged.publication)
    }

    pub async fn delete_concept(
        &self,
        space_id: u64,
        concept_row_id: u64,
        actor: &str,
        drive_space_id: Option<&str>,
    ) -> Result<(), OkfConceptServiceError> {
        if space_id == 0 {
            return Err(OkfConceptServiceError::InvalidRequest(
                "space_id is required".to_string(),
            ));
        }
        if is_blank(Some(actor)) {
            return Err(OkfConceptServiceError::InvalidRequest(
                "actor is required".to_string(),
            ));
        }

        let concept = self
            .concept_store
            .mark_concept_deleted(space_id, concept_row_id)
            .await?;

        // The concept is now deleted (irreversible). The remaining steps only
        // maintain derived state — link edges and the bundle catalog — so a
        // transient failure must not make the API report failure for a delete
        // that already happened (a caller retry would just get NotFound).
        // Bounded-retry, then degrade to a logged error for reconciliation.
        let mut follow_up_error = None;
        for attempt in 0..2u8 {
            let result = async {
                if let Some(link_store) = self.link_store {
                    link_store
                        .replace_outbound_links(ReplaceKnowledgeOkfConceptLinksRecord {
                            space_id,
                            from_concept_id: concept.concept_id.clone(),
                            links: vec![],
                        })
                        .await?;
                }
                self.finalize_bundle_catalog_deletion(
                    space_id,
                    &concept.title,
                    actor,
                    drive_space_id,
                )
                .await
            }
            .await;
            match result {
                Ok(()) => {
                    follow_up_error = None;
                    break;
                }
                Err(error) => {
                    if attempt == 0 {
                        tracing::warn!(
                            space_id,
                            concept_row_id,
                            concept_id = %concept.concept_id,
                            error = %error,
                            "okf concept delete follow-up failed; retrying once"
                        );
                    } else {
                        follow_up_error = Some(error);
                    }
                }
            }
        }
        if let Some(error) = follow_up_error {
            tracing::error!(
                space_id,
                concept_row_id,
                concept_id = %concept.concept_id,
                error = %error,
                "okf concept deleted; derived link/bundle state still needs reconciliation"
            );
        }

        Ok(())
    }

    async fn finalize_bundle_catalog_deletion(
        &self,
        space_id: u64,
        title: &str,
        actor: &str,
        drive_space_id: Option<&str>,
    ) -> Result<(), OkfConceptServiceError> {
        catalog_log::append_okf_bundle_log_entry(
            self.concept_store,
            space_id,
            "delete",
            format!("Deleted {title}"),
            actor,
            vec![title.to_string()],
            vec![],
        )
        .await?;
        self.rebuild_bundle_standard_files(space_id, drive_space_id)
            .await
    }

    async fn stage_concept_revision(
        &self,
        request: &PublishKnowledgeOkfConceptRequest,
        drive_space_id: Option<&str>,
        project_to_bundle: bool,
    ) -> Result<StagedOkfConceptRevision, OkfConceptServiceError> {
        validate_publish_request(request)?;
        let drive_space_id = self.required_drive_space_id(drive_space_id)?;
        let concept_id = normalize_concept_id(&request.concept_id)?;
        let published_logical_path = OkfBundlePaths::concept_logical_path(&concept_id);
        let concept_document = OkfConceptDocument {
            concept_type: request.concept_type.clone(),
            title: Some(request.title.clone()),
            description: Some(request.description.clone()),
            resource: request.resource.clone(),
            tags: request.tags.clone(),
            timestamp: request.timestamp.clone().or_else(|| now_rfc3339().ok()),
            extensions: request.frontmatter_extensions.clone(),
            body: request.markdown.clone(),
        };
        validate_concept_document(&concept_document, &concept_id)
            .map_err(OkfConceptServiceError::Conformance)?;
        let published_markdown = render_okf_concept_markdown(&concept_document);
        let publish_state = if project_to_bundle {
            OkfConceptPublishState::Published
        } else {
            OkfConceptPublishState::CandidateReady
        };

        let prepared = self
            .revision_metadata
            .prepare_concept_revision_slot(UpsertKnowledgeOkfConceptRecord {
                space_id: request.space_id,
                concept_id: concept_id.clone(),
                title: request.title.clone(),
                concept_type: request.concept_type.clone(),
                logical_path: published_logical_path.clone(),
                description: request.description.clone(),
                source_count: request.source_count,
                tags: request.tags.clone(),
                publish_state,
            })
            .await?;
        let concept = prepared.concept;
        let revision_no = prepared.revision_no;
        let governance_revision_path =
            governance_drive::governance_revision_path(&concept_id, revision_no);

        let published_ref = if project_to_bundle {
            Some(
                self.put_markdown(
                    &published_logical_path,
                    "concept_revision",
                    &published_markdown,
                    drive_space_id.as_deref(),
                )
                .await?,
            )
        } else {
            None
        };
        let candidate = if project_to_bundle || self.candidate_store.is_none() {
            None
        } else {
            let candidate_type = if revision_no > 1 {
                OkfCandidateType::ConceptUpdate
            } else {
                OkfCandidateType::ConceptCreate
            };
            Some(UpsertKnowledgeOkfCandidateRecord {
                space_id: request.space_id,
                concept_row_id: concept.id,
                concept_id: concept_id.clone(),
                candidate_type,
                state: OkfConceptPublishState::CandidateReady,
                markdown_object_ref_id: 0,
            })
        };
        let revision_ref = self
            .put_markdown(
                &governance_revision_path,
                "concept_revision",
                &published_markdown,
                drive_space_id.as_deref(),
            )
            .await?;
        let staged_metadata = self
            .revision_metadata
            .stage_concept_revision_metadata(StageOkfConceptRevisionMetadataRecord {
                revision_object_ref: object_ref_record(
                    request.space_id,
                    drive_space_id.as_deref(),
                    None,
                    &revision_ref,
                ),
                published_object_ref: published_ref.as_ref().map(|published_ref| {
                    object_ref_record(
                        request.space_id,
                        drive_space_id.as_deref(),
                        None,
                        published_ref,
                    )
                }),
                concept_row_id: concept.id,
                revision_no,
                content_hash: sha256_hash(published_markdown.as_bytes()),
                review_state: OkfRevisionReviewState::Approved,
                publish_state,
                candidate,
            })
            .await?;
        let revision = staged_metadata.revision;
        let concept = staged_metadata.concept;

        self.ensure_drive_nodes(
            drive_space_id.as_deref(),
            governance_drive::governance_revision_drive_nodes(
                &concept_id,
                Some(&revision_ref),
                published_ref.as_ref(),
            ),
        )
        .await?;

        record_okf_concept_upsert(request.space_id, &concept_id, &request.actor);

        Ok(StagedOkfConceptRevision {
            publication: KnowledgeOkfConceptPublication {
                concept,
                revision,
                published_logical_path,
                governance_revision_path,
            },
            concept_document,
        })
    }

    #[allow(clippy::too_many_arguments)]
    async fn finalize_publication(
        &self,
        space_id: u64,
        concept_id: &str,
        title: &str,
        actor: &str,
        staged: &StagedOkfConceptRevision,
        drive_space_id: Option<&str>,
        rebuild_standard_files: bool,
    ) -> Result<(), OkfConceptServiceError> {
        record_okf_concept_publish(space_id, concept_id, actor);
        catalog_log::append_okf_bundle_log_entry(
            self.concept_store,
            space_id,
            OkfLogEventType::Publish.as_str(),
            format!("Published {title}"),
            actor,
            vec![title.to_string()],
            vec![],
        )
        .await?;
        self.reindex_concept_links(space_id, concept_id, &staged.concept_document.body)
            .await?;
        if rebuild_standard_files {
            self.rebuild_standard_files(space_id, drive_space_id)
                .await?;
        }
        Ok(())
    }

    async fn put_markdown(
        &self,
        logical_path: &str,
        object_role: &str,
        markdown: &str,
        drive_space_id: Option<&str>,
    ) -> Result<KnowledgeObjectRef, OkfConceptServiceError> {
        Ok(self
            .drive
            .put_object(PutKnowledgeObjectRequest::managed_text(
                logical_path.to_string(),
                object_role.to_string(),
                markdown.to_string(),
                drive_space_id,
            ))
            .await?)
    }

    async fn rebuild_standard_files(
        &self,
        space_id: u64,
        drive_space_id: Option<&str>,
    ) -> Result<(), OkfConceptServiceError> {
        let summaries =
            crate::ports::knowledge_okf_concept_store::list_all_published_concept_summaries(
                self.concept_store,
                space_id,
            )
            .await?;
        let logs = self.concept_store.list_log_entries(space_id).await?;
        let dynamic = standard_bundle_refresh::persist_dynamic_standard_bundle_files(
            self.drive,
            &summaries,
            &logs,
            drive_space_id,
        )
        .await?;
        if self.file_entries.is_none() && self.drive_workspace.is_none() {
            return Ok(());
        }
        standard_bundle_catalog_sync::sync_dynamic_standard_bundle_catalog(
            standard_bundle_catalog_sync::StandardBundleCatalogSyncDeps {
                bundle_file_store: self.file_entries,
                drive_workspace: self.drive_workspace,
            },
            space_id,
            &dynamic,
            drive_space_id,
        )
        .await
        .map_err(map_catalog_sync_error)?;
        Ok(())
    }

    async fn ensure_drive_nodes(
        &self,
        drive_space_id: Option<&str>,
        nodes: Vec<EnsureKnowledgeDriveNodeRequest>,
    ) -> Result<(), OkfConceptServiceError> {
        let Some(workspace) = self.drive_workspace else {
            return Ok(());
        };
        governance_drive::ensure_drive_workspace_nodes(workspace, drive_space_id, nodes).await?;
        Ok(())
    }

    fn required_drive_space_id(
        &self,
        drive_space_id: Option<&str>,
    ) -> Result<Option<String>, OkfConceptServiceError> {
        if self.drive_workspace.is_none() {
            return Ok(None);
        }
        governance_drive::trim_bound_drive_space_id(drive_space_id)
            .map(Some)
            .ok_or_else(|| {
                OkfConceptServiceError::InvalidRequest(
                    governance_drive::DRIVE_WORKSPACE_SYNC_DRIVE_SPACE_REQUIRED.to_string(),
                )
            })
    }

    async fn reindex_concept_links(
        &self,
        space_id: u64,
        concept_id: &str,
        body: &str,
    ) -> Result<(), OkfConceptServiceError> {
        let Some(link_store) = self.link_store else {
            return Ok(());
        };
        let known =
            crate::ports::knowledge_okf_concept_store::list_all_published_concept_summaries(
                self.concept_store,
                space_id,
            )
            .await?
            .into_iter()
            .map(|summary| summary.concept_id)
            .collect::<Vec<_>>();
        let links = index_concept_links(body, concept_id, &known)
            .into_iter()
            .filter_map(|link| {
                link.target_concept_id
                    .map(|to_concept_id| KnowledgeOkfConceptLinkRecord {
                        to_concept_id,
                        anchor_text: link.anchor_text,
                    })
            })
            .collect();
        link_store
            .replace_outbound_links(ReplaceKnowledgeOkfConceptLinksRecord {
                space_id,
                from_concept_id: concept_id.to_string(),
                links,
            })
            .await?;
        Ok(())
    }
}

fn object_ref_record(
    space_id: u64,
    drive_space_id: Option<&str>,
    drive_node_id: Option<String>,
    object_ref: &KnowledgeObjectRef,
) -> CreateKnowledgeDriveObjectRefRecord {
    managed_drive_object_ref_record(space_id, object_ref, drive_space_id, drive_node_id)
}

fn validate_publish_request(
    request: &PublishKnowledgeOkfConceptRequest,
) -> Result<(), OkfConceptServiceError> {
    if request.space_id == 0 {
        return Err(OkfConceptServiceError::InvalidRequest(
            "space_id is required".to_string(),
        ));
    }
    if is_blank(Some(request.title.as_str())) {
        return Err(OkfConceptServiceError::InvalidRequest(
            "title is required".to_string(),
        ));
    }
    if is_blank(Some(request.markdown.as_str())) {
        return Err(OkfConceptServiceError::InvalidRequest(
            "markdown is required".to_string(),
        ));
    }
    safe_concept_id(&request.concept_id)?;
    Ok(())
}

fn normalize_concept_id(value: &str) -> Result<String, OkfConceptServiceError> {
    validate_concept_id(value).map_err(OkfConceptServiceError::Conformance)?;
    Ok(value.trim().to_string())
}

fn safe_concept_id(value: &str) -> Result<String, OkfConceptServiceError> {
    normalize_concept_id(value)
}

fn now_rfc3339() -> Result<String, OkfConceptServiceError> {
    OffsetDateTime::now_utc()
        .format(&Rfc3339)
        .map_err(|error| OkfConceptServiceError::Internal(error.to_string()))
}

#[derive(Debug, Clone)]
struct StagedOkfConceptRevision {
    publication: KnowledgeOkfConceptPublication,
    concept_document: OkfConceptDocument,
}

fn title_from_concept_id(concept_id: &str) -> String {
    concept_id
        .rsplit('/')
        .next()
        .unwrap_or(concept_id)
        .replace(['-', '_'], " ")
}

fn map_catalog_sync_error(
    error: standard_bundle_catalog_sync::StandardBundleCatalogSyncError,
) -> OkfConceptServiceError {
    match error {
        standard_bundle_catalog_sync::StandardBundleCatalogSyncError::Registry(error) => {
            OkfConceptServiceError::Internal(error.to_string())
        }
        standard_bundle_catalog_sync::StandardBundleCatalogSyncError::DriveWorkspace(error) => {
            OkfConceptServiceError::DriveWorkspace(error)
        }
    }
}

#[derive(Debug, Error)]
pub enum OkfConceptServiceError {
    #[error("invalid knowledge okf concept request: {0}")]
    InvalidRequest(String),
    #[error(transparent)]
    Conformance(#[from] OkfConformanceError),
    #[error(transparent)]
    Storage(#[from] KnowledgeStorageError),
    #[error(transparent)]
    RevisionMetadata(#[from] OkfConceptRevisionMetadataStoreError),
    #[error(transparent)]
    ConceptStore(#[from] KnowledgeOkfConceptStoreError),
    #[error(transparent)]
    LinkStore(#[from] KnowledgeOkfConceptLinkStoreError),
    #[error(transparent)]
    CandidateStore(#[from] KnowledgeOkfCandidateStoreError),
    #[error(transparent)]
    FileEntryStore(#[from] KnowledgeOkfBundleFileStoreError),
    #[error(transparent)]
    DriveWorkspace(#[from] KnowledgeDriveWorkspaceError),
    #[error("knowledge okf concept internal error: {0}")]
    Internal(String),
}
