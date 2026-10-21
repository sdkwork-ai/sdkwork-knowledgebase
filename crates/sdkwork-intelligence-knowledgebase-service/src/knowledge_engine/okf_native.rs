use async_trait::async_trait;
use sdkwork_knowledgebase_contract::knowledge_engine::{
    descriptor_for_mode, KnowledgeEngineDescriptor, KnowledgeEngineDocument,
    KnowledgeEngineDocumentList, KnowledgeEngineDocumentRef, KnowledgeEngineError,
    KnowledgeEngineHealth, KnowledgeEngineHealthStatus, KnowledgeEngineId,
    KnowledgeEngineListRequest, KnowledgeEngineReadRequest, KnowledgeEngineSearchHit,
    KnowledgeEngineSearchRequest, KnowledgeEngineSearchResult,
};
use sdkwork_knowledgebase_contract::okf::{
    KnowledgeOkfConcept, OkfBundleExportRequest, OkfBundleImportRequest, OkfBundleLintResult,
    OkfConceptSummary, OkfConceptUpsertRequest, PublishKnowledgeOkfConceptRequest,
};
use sdkwork_knowledgebase_contract::okf_bundle_file::KnowledgeOkfBundleFile;
use sdkwork_knowledgebase_contract::provider_binding::KnowledgeEngineExecutionContext;
use sdkwork_knowledgebase_contract::rag::KnowledgeAgentKnowledgeMode;
use sdkwork_knowledgebase_contract::OkfBundleFileKind;
use sdkwork_utils_rust::is_blank;
use std::collections::HashMap;
use std::sync::Arc;

use crate::okf::{
    load_import_bundle_from_drive, read_managed_markdown, rebuild_bundle_index_for_space,
    to_contract_lint_result, ExportOkfBundleRequest, ImportOkfBundleRequest,
    OkfBundleExporterService, OkfBundleImporterError, OkfBundleImporterService,
    OkfBundleLinterService, OkfConceptService, OkfConceptServiceError,
};
use crate::ports::knowledge_drive_object_ref_store::KnowledgeDriveObjectRefStore;
use crate::ports::knowledge_drive_storage::{KnowledgeDriveStorage, KnowledgeStorageError};
use crate::ports::knowledge_drive_workspace::KnowledgeDriveWorkspace;
use crate::ports::knowledge_okf_bundle_file_store::{
    CreateKnowledgeOkfBundleFileRecord, KnowledgeOkfBundleFileStore,
};
use crate::ports::knowledge_okf_candidate_store::KnowledgeOkfCandidateStore;
use crate::ports::knowledge_okf_concept_link_store::KnowledgeOkfConceptLinkStore;
use crate::ports::knowledge_okf_concept_store::{
    KnowledgeOkfConceptStore, KnowledgeOkfConceptStoreError,
};
use crate::ports::knowledge_source_store::KnowledgeSourceStore;
use crate::ports::knowledge_space_store::KnowledgeSpaceStore;
use crate::ports::okf_concept_revision_metadata_store::OkfConceptRevisionMetadataStore;

use super::okf_search::{
    body_match_score, combine_metadata_and_body_score, expand_ranked_with_link_edges,
    normalize_query, rank_okf_concepts_with_tokens, snippet_for_concept,
};
use super::KnowledgeEngine;
use crate::ports::knowledge_engine::OkfBundleEngine;

const SPI_ACTOR: &str = "knowledge-engine-spi";
const OKF_DRIVE_READ_CONCURRENCY: usize = 8;
/// Response cap owned by the engine boundary. Callers may request more, but a
/// single search never returns or materializes more hits than this.
const OKF_SEARCH_MAX_TOP_K: u32 = 100;
/// Candidate fan-out factor: fetch a few more rows than requested so body and
/// link signal can reorder them, then cap the candidate set absolutely.
const OKF_SEARCH_CANDIDATE_MULTIPLIER: u32 = 4;
const OKF_SEARCH_MIN_CANDIDATES: usize = 8;
const OKF_SEARCH_MAX_CANDIDATES: usize = 200;
/// Incident link edges fetched for the candidate set, proportional to the
/// candidate count with a hard cap.
const OKF_SEARCH_LINK_EDGE_MULTIPLIER: usize = 8;
const OKF_SEARCH_MIN_LINK_EDGES: u32 = 64;
const OKF_SEARCH_MAX_LINK_EDGES: u32 = 1_000;

#[derive(Clone)]
pub struct OkfNativeKnowledgeEngineDeps {
    pub concepts: Arc<dyn KnowledgeOkfConceptStore>,
    pub drive: Arc<dyn KnowledgeDriveStorage>,
    pub revision_metadata: Arc<dyn OkfConceptRevisionMetadataStore>,
    pub object_refs: Arc<dyn KnowledgeDriveObjectRefStore>,
    pub link_store: Arc<dyn KnowledgeOkfConceptLinkStore>,
    pub candidate_store: Arc<dyn KnowledgeOkfCandidateStore>,
    pub bundle_file_store: Arc<dyn KnowledgeOkfBundleFileStore>,
    pub drive_workspace: Arc<dyn KnowledgeDriveWorkspace>,
    pub source_store: Arc<dyn KnowledgeSourceStore>,
    pub space_store: Arc<dyn KnowledgeSpaceStore>,
}

pub struct OkfNativeKnowledgeEngine {
    deps: OkfNativeKnowledgeEngineDeps,
}

impl OkfNativeKnowledgeEngine {
    pub fn from_deps(deps: OkfNativeKnowledgeEngineDeps) -> Self {
        Self { deps }
    }

    fn concept_service(&self) -> OkfConceptService<'_> {
        OkfConceptService::new(
            self.deps.drive.as_ref(),
            self.deps.revision_metadata.as_ref(),
            self.deps.concepts.as_ref(),
        )
        .with_link_store(self.deps.link_store.as_ref())
        .with_candidate_store(self.deps.candidate_store.as_ref())
        .with_file_entry_store(self.deps.bundle_file_store.as_ref())
        .with_drive_workspace(self.deps.drive_workspace.as_ref())
    }

    async fn drive_space_id(&self, space_id: u64) -> Result<Option<String>, KnowledgeEngineError> {
        let space = self
            .deps
            .space_store
            .get_space(space_id)
            .await
            .map_err(|error| KnowledgeEngineError::Internal(error.to_string()))?;
        Ok(space.drive_space_id)
    }

    pub async fn import_bundle_for_actor(
        &self,
        request: OkfBundleImportRequest,
        actor: &str,
    ) -> Result<sdkwork_knowledgebase_contract::okf::OkfBundleImportResult, KnowledgeEngineError>
    {
        if request.space_id == 0 || is_blank(Some(request.import_type.as_str())) {
            return Err(KnowledgeEngineError::Validation(
                "space_id and import_type are required".to_string(),
            ));
        }
        let import_type = request.import_type.trim();
        if import_type != "okf_strict" && import_type != "okf_bundle" {
            return Err(KnowledgeEngineError::Validation(format!(
                "unsupported import_type: {import_type}"
            )));
        }
        let publish = import_type == "okf_strict";
        let drive_space_id = self.drive_space_id(request.space_id).await?;
        let files = load_import_bundle_from_drive(
            self.deps.drive.as_ref(),
            request.space_id,
            request.import_id.as_deref(),
            drive_space_id.as_deref(),
        )
        .await
        .map_err(|error| KnowledgeEngineError::Internal(error.to_string()))?;

        self.import_bundle_files(
            ImportOkfBundleRequest {
                space_id: request.space_id,
                actor: actor.to_string(),
                publish,
                files,
            },
            drive_space_id.as_deref(),
        )
        .await
    }

    pub async fn import_bundle_files(
        &self,
        request: ImportOkfBundleRequest,
        drive_space_id: Option<&str>,
    ) -> Result<sdkwork_knowledgebase_contract::okf::OkfBundleImportResult, KnowledgeEngineError>
    {
        let result = OkfBundleImporterService::new(self.concept_service())
            .import_bundle(request, drive_space_id)
            .await
            .map_err(map_importer_error)?;

        Ok(sdkwork_knowledgebase_contract::okf::OkfBundleImportResult {
            imported_concept_count: result.imported_concept_count,
            skipped_files: result.skipped_files,
        })
    }

    pub async fn publish_existing_revision(
        &self,
        request: crate::okf::PublishExistingOkfConceptRevisionRequest,
        drive_space_id: Option<&str>,
    ) -> Result<
        sdkwork_knowledgebase_contract::okf::KnowledgeOkfConceptPublication,
        KnowledgeEngineError,
    > {
        self.concept_service()
            .publish_existing_revision(request, drive_space_id)
            .await
            .map_err(map_concept_service_error)
    }
}

#[async_trait]
impl KnowledgeEngine for OkfNativeKnowledgeEngine {
    fn descriptor(&self) -> KnowledgeEngineDescriptor {
        descriptor_for_mode(KnowledgeAgentKnowledgeMode::OkfBundle)
    }

    async fn health(&self) -> Result<KnowledgeEngineHealth, KnowledgeEngineError> {
        Ok(KnowledgeEngineHealth {
            implementation_id: KnowledgeEngineId::OKF_NATIVE.to_string(),
            status: KnowledgeEngineHealthStatus::Available,
            detail: Some("native OKF bundle engine".to_string()),
        })
    }

    async fn search(
        &self,
        _context: &KnowledgeEngineExecutionContext,
        request: KnowledgeEngineSearchRequest,
    ) -> Result<KnowledgeEngineSearchResult, KnowledgeEngineError> {
        // The engine boundary owns the result cap: a caller-supplied `top_k`
        // can never make search materialize or return an unbounded result.
        let top_k = request.top_k.clamp(1, OKF_SEARCH_MAX_TOP_K);
        let candidate_limit = ((top_k as usize) * (OKF_SEARCH_CANDIDATE_MULTIPLIER as usize))
            .clamp(OKF_SEARCH_MIN_CANDIDATES, OKF_SEARCH_MAX_CANDIDATES);
        let tokens = normalize_query(&request.query);

        // Ask the store for a bounded, ranked candidate set. The PostgreSQL
        // store pushes filtering and LIMIT into SQL; the fallback pages a
        // bounded scan. Either way the service never receives a whole space.
        let candidate_summaries = self
            .deps
            .concepts
            .search_published_concept_summaries(request.space_id, &tokens, candidate_limit as u32)
            .await
            .map_err(map_okf_store_error)?;

        let summaries_by_id = candidate_summaries
            .iter()
            .map(|summary| (summary.concept_id.clone(), summary.clone()))
            .collect::<HashMap<_, _>>();
        let mut ranked = rank_okf_concepts_with_tokens(candidate_summaries, &tokens);

        if !ranked.is_empty() {
            // Expand link signal only for the bounded candidate set: no
            // space-wide edge graph and no all-pairs scoring loop.
            let candidate_ids = ranked
                .iter()
                .map(|(_, concept)| concept.concept_id.clone())
                .collect::<Vec<_>>();
            let edge_limit = candidate_ids
                .len()
                .saturating_mul(OKF_SEARCH_LINK_EDGE_MULTIPLIER)
                .clamp(
                    OKF_SEARCH_MIN_LINK_EDGES as usize,
                    OKF_SEARCH_MAX_LINK_EDGES as usize,
                );
            let edges = self
                .deps
                .link_store
                .list_active_link_edges_incident_to(
                    request.space_id,
                    &candidate_ids,
                    edge_limit as u32,
                )
                .await
                .map_err(|error| KnowledgeEngineError::Internal(error.to_string()))?;
            ranked = expand_ranked_with_link_edges(
                ranked,
                &edges,
                &summaries_by_id,
                &tokens,
                candidate_limit,
            );
        }

        let drive_space_id = self.drive_space_id(request.space_id).await?;
        let candidates: Vec<_> = ranked.into_iter().take(candidate_limit).collect();
        let drive = Arc::clone(&self.deps.drive);
        let mut hits = Vec::with_capacity(candidates.len().min(top_k as usize));

        for chunk in candidates.chunks(OKF_DRIVE_READ_CONCURRENCY) {
            let mut handles = Vec::with_capacity(chunk.len());
            for (metadata_score, concept) in chunk {
                let drive = Arc::clone(&drive);
                let drive_space_id = drive_space_id.clone();
                let logical_path = concept.logical_path.clone();
                let metadata_score = *metadata_score;
                let concept = concept.clone();
                handles.push(tokio::spawn(async move {
                    let body = match read_managed_markdown(
                        drive.as_ref(),
                        &logical_path,
                        drive_space_id.as_deref(),
                    )
                    .await
                    {
                        Ok(content) => Some(content),
                        Err(error) => {
                            tracing::warn!(
                                logical_path = %logical_path,
                                ?error,
                                "okf native search skipped drive body read; using metadata-only ranking"
                            );
                            None
                        }
                    };
                    (metadata_score, concept, body)
                }));
            }

            for handle in handles {
                let (metadata_score, concept, body) = handle
                    .await
                    .map_err(|error| KnowledgeEngineError::Internal(error.to_string()))?;
                let body_score = body
                    .as_deref()
                    .map(|content| body_match_score(content, &tokens))
                    .unwrap_or(0.0);
                let score = combine_metadata_and_body_score(metadata_score, body_score);
                if score <= 0.0 && !tokens.is_empty() {
                    continue;
                }
                hits.push(KnowledgeEngineSearchHit {
                    document: KnowledgeEngineDocumentRef {
                        document_id: concept.concept_id.clone(),
                        title: concept.title.clone(),
                        source_uri: Some(concept.logical_path.clone()),
                    },
                    snippet: snippet_for_concept(
                        &concept.description,
                        body.as_deref(),
                        &request.query,
                    ),
                    score: Some(score),
                });
            }
        }

        hits.sort_by(|left, right| {
            right
                .score
                .partial_cmp(&left.score)
                .unwrap_or(std::cmp::Ordering::Equal)
                .then_with(|| left.document.document_id.cmp(&right.document.document_id))
        });
        hits.truncate(top_k as usize);

        Ok(KnowledgeEngineSearchResult {
            implementation_id: KnowledgeEngineId::OKF_NATIVE.to_string(),
            hits,
        })
    }

    async fn read_document(
        &self,
        _context: &KnowledgeEngineExecutionContext,
        request: KnowledgeEngineReadRequest,
    ) -> Result<KnowledgeEngineDocument, KnowledgeEngineError> {
        // Single-concept resolution: the store answers with one bounded lookup
        // (SQL `LIMIT 1` in PostgreSQL) instead of materializing every published
        // concept in the space to find one.
        let concept = self
            .deps
            .concepts
            .get_published_concept_summary(request.space_id, &request.document_id)
            .await
            .map_err(map_okf_store_error)?
            .ok_or_else(|| {
                KnowledgeEngineError::NotFound(format!(
                    "okf concept not found: {}",
                    request.document_id
                ))
            })?;

        let drive_space_id = self.drive_space_id(request.space_id).await?;
        let content = read_managed_markdown(
            self.deps.drive.as_ref(),
            &concept.logical_path,
            drive_space_id.as_deref(),
        )
        .await
        .map_err(map_drive_error)?;

        Ok(KnowledgeEngineDocument {
            document_id: concept.concept_id,
            title: concept.title,
            content,
            source_uri: Some(concept.logical_path),
        })
    }

    async fn list_documents(
        &self,
        _context: &KnowledgeEngineExecutionContext,
        request: KnowledgeEngineListRequest,
    ) -> Result<KnowledgeEngineDocumentList, KnowledgeEngineError> {
        let pages = self
            .deps
            .concepts
            .list_concept_summaries(request.space_id, Some(request.limit.max(1)))
            .await
            .map_err(map_okf_store_error)?;

        let items = pages
            .into_iter()
            .map(|concept| KnowledgeEngineDocumentRef {
                document_id: concept.concept_id,
                title: concept.title,
                source_uri: Some(concept.logical_path),
            })
            .collect();

        Ok(KnowledgeEngineDocumentList { items })
    }
}

#[async_trait]
impl OkfBundleEngine for OkfNativeKnowledgeEngine {
    async fn list_concepts(
        &self,
        space_id: u64,
    ) -> Result<Vec<OkfConceptSummary>, KnowledgeEngineError> {
        crate::ports::knowledge_okf_concept_store::list_all_published_concept_summaries(
            self.deps.concepts.as_ref(),
            space_id,
        )
        .await
        .map_err(map_okf_store_error)
    }

    async fn upsert_concept(
        &self,
        request: OkfConceptUpsertRequest,
    ) -> Result<KnowledgeOkfConcept, KnowledgeEngineError> {
        let drive_space_id = self.drive_space_id(request.space_id).await?;
        let publication = self
            .concept_service()
            .upsert_concept_from_markdown(request, drive_space_id.as_deref())
            .await
            .map_err(map_concept_service_error)?;
        Ok(publication.concept)
    }

    async fn delete_concept(
        &self,
        space_id: u64,
        concept_row_id: u64,
        actor: &str,
    ) -> Result<(), KnowledgeEngineError> {
        let drive_space_id = self.drive_space_id(space_id).await?;
        self.concept_service()
            .delete_concept(space_id, concept_row_id, actor, drive_space_id.as_deref())
            .await
            .map_err(map_concept_service_error)
    }

    async fn publish_concept(
        &self,
        request: PublishKnowledgeOkfConceptRequest,
    ) -> Result<
        sdkwork_knowledgebase_contract::okf::KnowledgeOkfConceptPublication,
        KnowledgeEngineError,
    > {
        let drive_space_id = self.drive_space_id(request.space_id).await?;
        self.concept_service()
            .publish_concept(request, drive_space_id.as_deref())
            .await
            .map_err(map_concept_service_error)
    }

    async fn lint_bundle_report(
        &self,
        space_id: u64,
    ) -> Result<OkfBundleLintResult, KnowledgeEngineError> {
        let drive_space_id = self.drive_space_id(space_id).await?;
        let report =
            OkfBundleLinterService::new(self.deps.drive.as_ref(), self.deps.concepts.as_ref())
                .with_link_store(self.deps.link_store.as_ref())
                .with_source_store(self.deps.source_store.as_ref())
                .lint_space(space_id, drive_space_id.as_deref())
                .await
                .map_err(|error| KnowledgeEngineError::Internal(error.to_string()))?;
        Ok(to_contract_lint_result(&report))
    }

    async fn import_bundle(
        &self,
        request: OkfBundleImportRequest,
    ) -> Result<sdkwork_knowledgebase_contract::okf::OkfBundleImportResult, KnowledgeEngineError>
    {
        self.import_bundle_for_actor(request, SPI_ACTOR).await
    }

    async fn export_bundle(
        &self,
        request: OkfBundleExportRequest,
    ) -> Result<KnowledgeOkfBundleFile, KnowledgeEngineError> {
        if request.space_id == 0 || is_blank(Some(request.export_type.as_str())) {
            return Err(KnowledgeEngineError::Validation(
                "space_id and export_type are required".to_string(),
            ));
        }
        if request.stage_for_import {
            return Err(KnowledgeEngineError::Unsupported(
                "okf export stage_for_import is owned by hosted export routes".to_string(),
            ));
        }

        let source_object_refs = if request.export_type.trim() == "okf_with_sources" {
            self.deps
                .object_refs
                .list_object_refs_by_logical_path_prefix(request.space_id, "sources/raw/")
                .await
                .map_err(|error| KnowledgeEngineError::Internal(error.to_string()))?
        } else {
            Vec::new()
        };

        let drive_space_id = self.drive_space_id(request.space_id).await?;
        let exported =
            OkfBundleExporterService::new(self.deps.drive.as_ref(), self.deps.concepts.as_ref())
                .with_source_object_refs(source_object_refs)
                .export_bundle(
                    ExportOkfBundleRequest {
                        space_id: request.space_id,
                        export_type: request.export_type,
                    },
                    drive_space_id.as_deref(),
                )
                .await
                .map_err(|error| KnowledgeEngineError::Internal(error.to_string()))?;

        self.deps
            .bundle_file_store
            .create_file_entry(CreateKnowledgeOkfBundleFileRecord {
                space_id: request.space_id,
                logical_path: exported.manifest_path,
                file_kind: OkfBundleFileKind::OutputExport,
                artifact_role: exported.manifest_ref.object_role.clone(),
                drive_bucket: exported.manifest_ref.bucket.clone(),
                drive_object_key: exported.manifest_ref.object_key.clone(),
                checksum_sha256_hex: exported.manifest_ref.checksum_sha256_hex.clone(),
            })
            .await
            .map_err(|error| KnowledgeEngineError::Internal(error.to_string()))
    }

    async fn rebuild_index(&self, space_id: u64) -> Result<(), KnowledgeEngineError> {
        rebuild_bundle_index_for_space(
            self.deps.drive.as_ref(),
            self.deps.concepts.as_ref(),
            self.deps.space_store.as_ref(),
            space_id,
        )
        .await
        .map_err(|error| KnowledgeEngineError::Internal(error.to_string()))
    }
}

fn map_okf_store_error(error: KnowledgeOkfConceptStoreError) -> KnowledgeEngineError {
    KnowledgeEngineError::Internal(error.to_string())
}

fn map_drive_error(error: KnowledgeStorageError) -> KnowledgeEngineError {
    match error {
        KnowledgeStorageError::NotFound(detail) => KnowledgeEngineError::NotFound(detail),
        other => KnowledgeEngineError::Internal(other.to_string()),
    }
}

fn map_importer_error(error: OkfBundleImporterError) -> KnowledgeEngineError {
    match error {
        OkfBundleImporterError::InvalidRequest(message) => {
            KnowledgeEngineError::Validation(message)
        }
        OkfBundleImporterError::Conformance(message) => KnowledgeEngineError::Validation(message),
        OkfBundleImporterError::Storage(error) => map_drive_error(error),
        OkfBundleImporterError::ConceptService(error) => map_concept_service_error(error),
    }
}

fn map_concept_service_error(error: OkfConceptServiceError) -> KnowledgeEngineError {
    match error {
        OkfConceptServiceError::InvalidRequest(message) => {
            KnowledgeEngineError::Validation(message)
        }
        OkfConceptServiceError::Conformance(error) => {
            KnowledgeEngineError::Validation(error.to_string())
        }
        OkfConceptServiceError::Storage(error) => match error {
            KnowledgeStorageError::NotFound(detail) => KnowledgeEngineError::NotFound(detail),
            other => KnowledgeEngineError::Internal(other.to_string()),
        },
        other => KnowledgeEngineError::Internal(other.to_string()),
    }
}
