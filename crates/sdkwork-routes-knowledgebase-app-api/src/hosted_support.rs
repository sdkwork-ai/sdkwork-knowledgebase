use sdkwork_intelligence_knowledgebase_service::{
    ingest::KnowledgeIngestionService,
    okf::{
        run_okf_compile_workflow, run_okf_eval_workflow, OkfBundleFileRegistryService,
        OkfBundleStandardFileService, OkfBundleWorkflowDeps, OkfBundleWorkflowEngine,
        PersistStandardFilesRequest, PublishExistingOkfConceptRevisionRequest,
    },
    ports::{
        knowledge_drive_storage::{KnowledgeDriveStorage, PutKnowledgeObjectRequest},
        knowledge_ingestion_job_store::{CreateIngestionJobRecord, IngestionJobStore},
        knowledge_okf_concept_store::KnowledgeOkfConceptStore,
    },
};
use sdkwork_knowledgebase_contract::ingest::IngestionJobState;
use sdkwork_knowledgebase_contract::knowledge_engine::KnowledgeEngineSearchHit;
use sdkwork_knowledgebase_contract::okf::{
    KnowledgeOkfConcept, OkfBundleExportRequest, OkfBundleImportRequest, OkfBundleImportResult,
    OkfBundleLintResult, OkfBundlePaths, OkfConceptSummary, OkfIndexDocument, OkfQualityRun,
    OkfQualityRunRequest,
};
use sdkwork_knowledgebase_contract::rag::{
    KnowledgeCitation, KnowledgeContextFragment, KnowledgeContextPack, KnowledgeRetrievalMethod,
};
use sdkwork_knowledgebase_contract::{IngestionJob, KnowledgeOkfBundleFile};
use sdkwork_utils_rust::is_blank;

use crate::ApiError;

pub(crate) fn concept_to_summary(concept: KnowledgeOkfConcept) -> OkfConceptSummary {
    OkfConceptSummary {
        title: concept.title,
        concept_id: concept.concept_id,
        concept_type: concept.concept_type,
        logical_path: concept.logical_path,
        bundle_relative_path: concept.bundle_relative_path,
        description: concept.description,
        source_count: concept.source_count,
        updated_at: concept.updated_at,
        tags: concept.tags,
    }
}

pub(crate) fn format_okf_engine_answer(hits: &[KnowledgeEngineSearchHit]) -> String {
    if hits.is_empty() {
        return "_No matching OKF concepts were found._".to_string();
    }

    hits.iter()
        .map(|hit| format!("### {}\n\n{}", hit.document.title, hit.snippet.trim()))
        .collect::<Vec<_>>()
        .join("\n\n")
}

pub(crate) fn stable_u64_hash(value: &str) -> u64 {
    use std::hash::{Hash, Hasher};
    let mut hasher = std::collections::hash_map::DefaultHasher::new();
    value.hash(&mut hasher);
    hasher.finish()
}

pub(crate) fn okf_citation_from_hit(
    space_id: u64,
    hit: &KnowledgeEngineSearchHit,
) -> KnowledgeCitation {
    let concept_id = hit.document.document_id.clone();
    let document_id = stable_u64_hash(&concept_id);
    KnowledgeCitation {
        document_id,
        document_version_id: None,
        chunk_id: Some(document_id),
        title: hit.document.title.clone(),
        source_uri: hit.document.source_uri.clone(),
        locator: Some(format!("okf:{space_id}:{concept_id}")),
        score: hit.score,
    }
}

pub(crate) async fn build_okf_context_pack_from_engine(
    runtime: &crate::runtime::KnowledgebaseRuntime,
    execution_context: &sdkwork_knowledgebase_contract::provider_binding::KnowledgeEngineExecutionContext,
    space_id: u64,
    query: String,
    context_budget_tokens: u32,
) -> Result<KnowledgeContextPack, ApiError> {
    if context_budget_tokens == 0 {
        return Err(ApiError::invalid_request(
            "invalid_okf_context_pack_request",
            "context_budget_tokens must be greater than zero",
        ));
    }

    let search = runtime
        .search_knowledge_engine_for_space(execution_context, space_id, &query, 32)
        .await
        .map_err(|error| ApiError::internal("okf_engine_search_failed", error))?;

    let mut fragments = Vec::new();
    let mut estimated_tokens = 0_u32;
    let mut truncated = false;

    for (rank, hit) in search.hits.into_iter().enumerate() {
        let rank = rank as u32 + 1;
        let document = runtime
            .read_knowledge_engine_document_for_space(
                execution_context,
                space_id,
                &hit.document.document_id,
            )
            .await
            .map_err(|error| ApiError::internal("okf_engine_read_failed", error))?;
        let token_count = document.content.split_whitespace().count().max(1) as u32;
        if estimated_tokens.saturating_add(token_count) > context_budget_tokens {
            truncated = true;
            break;
        }
        estimated_tokens = estimated_tokens.saturating_add(token_count);
        let document_id = stable_u64_hash(&hit.document.document_id);
        fragments.push(KnowledgeContextFragment {
            chunk_id: stable_u64_hash(&format!("{}:{rank}", hit.document.document_id)),
            document_id,
            document_version_id: None,
            space_id,
            collection_id: None,
            title: hit.document.title.clone(),
            content: document.content,
            score: hit.score,
            rank,
            token_count: Some(token_count),
            retrieval_method: KnowledgeRetrievalMethod::Keyword,
            citation: Some(okf_citation_from_hit(space_id, &hit)),
        });
    }

    let citations = fragments
        .iter()
        .filter_map(|fragment| fragment.citation.clone())
        .collect();
    let context_pack_id = stable_u64_hash(&format!("{space_id}:{query}"));

    Ok(KnowledgeContextPack {
        context_pack_id,
        retrieval_id: None,
        query,
        fragments,
        memory_fragments: vec![],
        estimated_tokens,
        citations,
        truncated,
    })
}

pub(crate) fn okf_answer_concept_id(query_id: u64) -> String {
    format!("answers/answer-{query_id}")
}

pub(crate) async fn read_managed_okf_text(
    drive: &dyn KnowledgeDriveStorage,
    logical_path: &str,
    object_role: &str,
    drive_space_id: Option<&str>,
) -> Result<String, ApiError> {
    let _ = object_role;
    sdkwork_intelligence_knowledgebase_service::okf::read_managed_markdown(
        drive,
        logical_path,
        drive_space_id,
    )
    .await
    .map_err(Into::into)
}

pub(crate) async fn publish_okf_concept_revision(
    runtime: &crate::runtime::KnowledgebaseRuntime,
    concept_row_id: u64,
    actor: &str,
) -> Result<KnowledgeOkfConcept, ApiError> {
    let concept = runtime
        .okf_concept_store()
        .get_concept_by_row_id(concept_row_id)
        .await
        .map_err(map_okf_concept_store)?;
    let revision_id = concept.current_revision_id.ok_or_else(|| {
        ApiError::conflict(
            "okf_concept_not_ready",
            format!("okf concept {concept_row_id} has no current revision to publish"),
        )
    })?;
    let revision = runtime
        .okf_concept_store()
        .get_revision_by_id(revision_id)
        .await
        .map_err(map_okf_concept_store)?;
    let space = runtime
        .get_space_for_authorized_operation(concept.space_id)
        .await?;
    runtime
        .resolve_okf_bundle_engine_for_space(concept.space_id)
        .await?;
    let publication = runtime
        .knowledge_engines()
        .publish_okf_existing_revision(
            PublishExistingOkfConceptRevisionRequest {
                space_id: concept.space_id,
                concept: concept.clone(),
                revision,
                actor: actor.to_string(),
            },
            space.drive_space_id.as_deref(),
        )
        .await
        .map_err(ApiError::from)?;
    Ok(publication.concept)
}

pub(crate) fn okf_paths() -> OkfBundlePaths {
    OkfBundlePaths::default()
}

fn okf_bundle_workflow_deps(
    runtime: &crate::runtime::KnowledgebaseRuntime,
) -> OkfBundleWorkflowDeps<'_> {
    OkfBundleWorkflowDeps {
        concepts: runtime.okf_concept_store(),
        drive: runtime.drive_storage(),
        space_store: runtime.space_store(),
        source_store: runtime.source_store(),
        link_store: Some(runtime.okf_concept_link_store()),
        bundle_file_store: Some(runtime.okf_bundle_file_store()),
        drive_workspace: Some(runtime.drive_workspace()),
        engine: Some(runtime.knowledge_engines() as &dyn OkfBundleWorkflowEngine),
    }
}

pub(crate) async fn run_okf_compile_workflow_for_space(
    runtime: &crate::runtime::KnowledgebaseRuntime,
    space_id: u64,
    source_id: Option<u64>,
    actor: &str,
) -> Result<(), ApiError> {
    runtime
        .resolve_okf_bundle_engine_for_space(space_id)
        .await?;
    run_okf_compile_workflow(
        okf_bundle_workflow_deps(runtime),
        space_id,
        source_id,
        actor,
    )
    .await
    .map_err(ApiError::from)
}

pub(crate) async fn run_okf_eval_workflow_for_space(
    runtime: &crate::runtime::KnowledgebaseRuntime,
    space_id: u64,
    actor: &str,
) -> Result<OkfBundleLintResult, ApiError> {
    runtime
        .resolve_okf_bundle_engine_for_space(space_id)
        .await?;
    run_okf_eval_workflow(okf_bundle_workflow_deps(runtime), space_id, actor)
        .await
        .map_err(ApiError::from)
}

pub(crate) async fn run_okf_bundle_lint(
    runtime: &crate::runtime::KnowledgebaseRuntime,
    space_id: u64,
) -> Result<OkfBundleLintResult, ApiError> {
    runtime
        .resolve_okf_bundle_engine_for_space(space_id)
        .await?;
    runtime
        .knowledge_engines()
        .lint_okf_bundle_report(space_id)
        .await
        .map_err(|error| ApiError::internal("okf_bundle_lint_failed", error.to_string()))
}

pub(crate) async fn rebuild_okf_index_document(
    runtime: &crate::runtime::KnowledgebaseRuntime,
    space_id: u64,
) -> Result<OkfIndexDocument, ApiError> {
    runtime
        .knowledge_engines()
        .rebuild_okf_index(space_id)
        .await
        .map_err(|error| ApiError::internal("okf_index_rebuild_failed", error.to_string()))?;

    let concepts = sdkwork_intelligence_knowledgebase_service::ports::knowledge_okf_concept_store::list_all_published_concept_summaries(
        runtime.okf_concept_store(),
        space_id,
    )
        .await
        .map_err(map_okf_concept_store)?;
    let space = runtime.get_space_for_authorized_operation(space_id).await?;
    let markdown =
        sdkwork_intelligence_knowledgebase_service::okf::render_index_md(&space.name, &concepts);
    Ok(OkfIndexDocument { markdown })
}

pub(crate) async fn persist_okf_profile(
    runtime: &crate::runtime::KnowledgebaseRuntime,
    space_id: u64,
) -> Result<sdkwork_knowledgebase_contract::KnowledgeOkfBundleFile, ApiError> {
    let space = runtime.get_space_for_authorized_operation(space_id).await?;
    let concepts = sdkwork_intelligence_knowledgebase_service::ports::knowledge_okf_concept_store::list_all_published_concept_summaries(
        runtime.okf_concept_store(),
        space_id,
    )
        .await
        .map_err(map_okf_concept_store)?;
    let logs = runtime
        .okf_concept_store()
        .list_log_entries(space_id)
        .await
        .map_err(map_okf_concept_store)?;
    let files = OkfBundleStandardFileService::new(runtime.drive_storage())
        .persist_standard_files(PersistStandardFilesRequest {
            space_name: space.name,
            concepts,
            log_entries: logs,
            drive_space_id: space.drive_space_id.clone(),
        })
        .await?;
    let registry = OkfBundleFileRegistryService::new(runtime.okf_bundle_file_store());
    let entries = registry
        .register_standard_files(space_id, &files)
        .await
        .map_err(|error| {
            ApiError::internal("okf_bundle_file_registry_failed", error.to_string())
        })?;
    entries
        .into_iter()
        .find(|entry| entry.logical_path.ends_with("okf_profile.yaml"))
        .ok_or_else(|| {
            ApiError::internal(
                "okf_profile_missing",
                "profile registration did not produce okf_profile.yaml entry",
            )
        })
}

pub(crate) async fn create_okf_bundle_export(
    runtime: &crate::runtime::KnowledgebaseRuntime,
    request: OkfBundleExportRequest,
) -> Result<KnowledgeOkfBundleFile, ApiError> {
    if request.space_id == 0 || is_blank(Some(request.export_type.as_str())) {
        return Err(ApiError::invalid_request(
            "invalid_okf_export_request",
            "space_id and export_type are required",
        ));
    }
    runtime
        .resolve_okf_bundle_engine_for_space(request.space_id)
        .await?;
    let file_entry = runtime
        .knowledge_engines()
        .export_okf_bundle(OkfBundleExportRequest {
            space_id: request.space_id,
            export_type: request.export_type.clone(),
            stage_for_import: false,
            import_id: None,
        })
        .await
        .map_err(|error| ApiError::internal("okf_export_failed", error.to_string()))?;

    let mut staged_import_root = None;
    let mut response_import_id = None;
    if request.stage_for_import {
        let export_root = file_entry
            .logical_path
            .strip_suffix("/export_manifest.yaml")
            .ok_or_else(|| {
                ApiError::internal(
                    "okf_export_stage_failed",
                    format!(
                        "export manifest path does not end with /export_manifest.yaml: {}",
                        file_entry.logical_path
                    ),
                )
            })?
            .to_string();
        let import_id = request
            .import_id
            .as_deref()
            .map(str::trim)
            .filter(|value| !value.is_empty())
            .map(str::to_string)
            .unwrap_or_else(|| format!("export-{}", file_entry.id));
        let space = runtime
            .get_space_for_authorized_operation(request.space_id)
            .await?;
        let staged =
            sdkwork_intelligence_knowledgebase_service::okf::stage_export_bundle_for_drive_import(
                runtime.drive_storage(),
                &export_root,
                request.space_id,
                &import_id,
                space.drive_space_id.as_deref(),
            )
            .await
            .map_err(|error| ApiError::internal("okf_export_stage_failed", error.to_string()))?;
        staged_import_root = Some(staged);
        response_import_id = Some(import_id);
    }

    Ok(KnowledgeOkfBundleFile {
        staged_import_root,
        import_id: response_import_id,
        ..file_entry
    })
}

pub(crate) async fn retrieve_okf_bundle_export(
    runtime: &crate::runtime::KnowledgebaseRuntime,
    export_id: u64,
) -> Result<KnowledgeOkfBundleFile, ApiError> {
    runtime
        .okf_bundle_file_store()
        .get_file_entry_by_id(export_id)
        .await
        .map_err(|error| {
            let detail = error.to_string();
            if detail.contains("missing okf bundle file") {
                ApiError::not_found("okf_export_not_found", detail)
            } else {
                ApiError::internal("okf_export_retrieve_failed", detail)
            }
        })
}

pub(crate) async fn create_okf_bundle_import(
    runtime: &crate::runtime::KnowledgebaseRuntime,
    request: OkfBundleImportRequest,
    actor: &str,
) -> Result<OkfBundleImportResult, ApiError> {
    if request.space_id == 0 || is_blank(Some(request.import_type.as_str())) {
        return Err(ApiError::invalid_request(
            "invalid_okf_import_request",
            "space_id and import_type are required",
        ));
    }
    let import_type = request.import_type.trim();
    if import_type != "okf_strict" && import_type != "okf_bundle" {
        return Err(ApiError::invalid_request(
            "invalid_okf_import_request",
            format!("unsupported import_type: {import_type}"),
        ));
    }
    runtime
        .resolve_okf_bundle_engine_for_space(request.space_id)
        .await?;
    runtime
        .knowledge_engines()
        .import_okf_bundle_for_actor(request, actor)
        .await
        .map_err(ApiError::from)
}

pub(crate) async fn create_okf_lint_run(
    runtime: &crate::runtime::KnowledgebaseRuntime,
    request: OkfQualityRunRequest,
) -> Result<OkfQualityRun, ApiError> {
    if request.space_id == 0 {
        return Err(ApiError::invalid_request(
            "invalid_okf_quality_run_request",
            "space_id is required",
        ));
    }
    let space_id = request.space_id;
    let runtime_for_job = runtime.clone();
    let runtime_in_closure = runtime.clone();
    let job = run_okf_ingestion_job(
        &runtime_for_job,
        space_id,
        "okf_lint_run",
        format!(
            "okf-lint:{}:{}",
            request.space_id,
            request.profile.as_deref().unwrap_or("default")
        ),
        || async move {
            let space = runtime_in_closure
                .get_space_for_authorized_operation(space_id)
                .await
                .map_err(|error| format!("{error:?}"))?;
            let lint_result = run_okf_bundle_lint(&runtime_in_closure, space_id)
                .await
                .map_err(|error| format!("{error:?}"))?;
            let report_path = format!("output/lint-reports/{space_id}.json");
            runtime_in_closure
                .drive_storage()
                .put_object(
                    PutKnowledgeObjectRequest {
                        logical_path: report_path,
                        object_role: "output_export".to_string(),
                        content_type: "application/json; charset=utf-8".to_string(),
                        body: serde_json::to_vec_pretty(&lint_result)
                            .map_err(|error| format!("failed to serialize lint report: {error}"))?,
                        checksum_sha256_hex: None,
                        space_uuid: None,
                    }
                    .with_drive_space_id(space.drive_space_id.as_deref()),
                )
                .await
                .map_err(|error| format!("{error:?}"))?;
            if lint_result.conformance != "pass" {
                return Err(format!(
                    "okf bundle lint failed with {} issue(s)",
                    lint_result.issues.len()
                ));
            }
            rebuild_okf_index_document(&runtime_in_closure, space_id)
                .await
                .map(|_| ())
                .map_err(|error| format!("{error:?}"))
        },
    )
    .await?;
    Ok(OkfQualityRun {
        id: job.id,
        state: format!("{:?}", job.state).to_ascii_lowercase(),
    })
}

async fn run_okf_ingestion_job<F, Fut>(
    runtime: &crate::runtime::KnowledgebaseRuntime,
    space_id: u64,
    source_type: &str,
    idempotency_key: String,
    run: F,
) -> Result<IngestionJob, ApiError>
where
    F: FnOnce() -> Fut,
    Fut: std::future::Future<Output = Result<(), String>>,
{
    let result = runtime
        .ingestion_job_store()
        .create_or_get_job(CreateIngestionJobRecord {
            space_id,
            source_type: source_type.to_string(),
            idempotency_key,
            idempotency_fingerprint_sha256_hex: None,
        })
        .await
        .map_err(ApiError::from)?;

    let mut job = result.job;
    if job.state != IngestionJobState::Queued {
        return Ok(job);
    }

    let ingestion = KnowledgeIngestionService::new(runtime.ingestion_job_store());
    job = ingestion
        .mark_running(job.id)
        .await
        .map_err(|error| ApiError::internal("okf_ingestion_job_failed", error.to_string()))?;
    match run().await {
        Ok(()) => ingestion
            .mark_succeeded(job.id)
            .await
            .map_err(|error| ApiError::internal("okf_ingestion_job_failed", error.to_string())),
        Err(detail) => ingestion
            .mark_failed(job.id, detail)
            .await
            .map_err(|error| ApiError::internal("okf_ingestion_job_failed", error.to_string())),
    }
}

fn map_okf_concept_store(
    error: sdkwork_intelligence_knowledgebase_service::ports::knowledge_okf_concept_store::KnowledgeOkfConceptStoreError,
) -> ApiError {
    ApiError::internal("knowledge_okf_concept_store_failed", error.to_string())
}
