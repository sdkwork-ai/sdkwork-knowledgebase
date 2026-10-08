use async_trait::async_trait;
use sdkwork_intelligence_knowledgebase_service::ingest::{
    ApiMarkdownIngestPipeline, ExistingMarkdownIngestJobParams,
};
use sdkwork_intelligence_knowledgebase_service::ports::knowledge_drive_storage::{
    HeadKnowledgeObjectRequest, KnowledgeDriveStorage, KnowledgeObjectRef, KnowledgeStorageError,
    PutKnowledgeObjectRequest,
};
use sdkwork_intelligence_knowledgebase_service::ports::knowledge_ingestion_job_store::{
    CreateIngestionJobRecord, CreateOrGetIngestionJobResult, DriveImportJobLinkage,
    IngestionJobStore, IngestionJobStoreError,
};
use sdkwork_intelligence_knowledgebase_service::ports::markdown_index_metadata_store::{
    MarkdownIndexMetadataStore, MarkdownIndexMetadataStoreError,
    PrepareMarkdownIndexMetadataRecord, PreparedMarkdownIndexMetadata,
};
use sdkwork_knowledgebase_contract::ingest::{
    IngestionJob, IngestionJobState, KnowledgeIngestRequest,
};
use std::collections::HashMap;
use std::sync::Mutex;

#[tokio::test]
async fn api_markdown_ingest_pipeline_replays_succeeded_job_with_document_version_id() {
    let drive = RecordingDrive::default();
    let jobs = MemoryIngestionJobStore::default();
    let metadata = MemoryMarkdownIndexMetadataStore::default();
    let pipeline = ApiMarkdownIngestPipeline::new(&drive, &jobs, &metadata);

    let request = KnowledgeIngestRequest {
        space_id: 7,
        title: "Replay note".to_string(),
        payload_markdown: "# Replay\n\nStable payload.".to_string(),
        idempotency_key: "api-replay-1".to_string(),
        source_url: None,
    };

    let first = pipeline
        .run(request.clone(), Some("drive-space-7"), "api-ingest")
        .await
        .unwrap();
    assert_eq!(first.job.state, IngestionJobState::Succeeded);
    let document_version_id = first
        .document_version_id
        .expect("first ingest should expose document_version_id");

    let replay = pipeline
        .run(request, Some("drive-space-7"), "api-ingest")
        .await
        .unwrap();
    assert_eq!(replay.job.id, first.job.id);
    assert_eq!(replay.job.state, IngestionJobState::Succeeded);
    assert_eq!(replay.document_version_id, Some(document_version_id));
    assert_eq!(metadata.prepare_count(), 1);
    assert_eq!(drive.write_count(), 1);
}

#[tokio::test]
async fn api_markdown_ingest_pipeline_retries_failed_job_on_replay() {
    let drive = RecordingDrive::default();
    let jobs = MemoryIngestionJobStore::default();
    let metadata = MemoryMarkdownIndexMetadataStore::default();
    let pipeline = ApiMarkdownIngestPipeline::new(&drive, &jobs, &metadata);

    let request = KnowledgeIngestRequest {
        space_id: 7,
        title: "Retry note".to_string(),
        payload_markdown: "# Retry\n\nRecoverable failure.".to_string(),
        idempotency_key: "api-retry-1".to_string(),
        source_url: None,
    };

    jobs.force_next_complete_failure(true);
    let failed = pipeline
        .run(request.clone(), None, "api-ingest")
        .await
        .unwrap_err();
    assert!(failed.to_string().contains("forced completion failure"));

    let job = jobs.get_job(1).await.unwrap();
    assert_eq!(job.state, IngestionJobState::Failed);

    jobs.force_next_complete_failure(false);
    let recovered = pipeline.run(request, None, "api-ingest").await.unwrap();
    assert_eq!(recovered.job.state, IngestionJobState::Succeeded);
    assert!(recovered.document_version_id.is_some());
    assert_eq!(metadata.prepare_count(), 2);
}

#[tokio::test]
async fn existing_upload_job_drive_put_failure_transitions_running_job_to_failed() {
    let drive = RecordingDrive::default();
    let jobs = MemoryIngestionJobStore::default();
    let metadata = MemoryMarkdownIndexMetadataStore::default();
    let pipeline = ApiMarkdownIngestPipeline::new(&drive, &jobs, &metadata);
    let created = jobs
        .create_or_get_job(CreateIngestionJobRecord {
            space_id: 7,
            source_type: "upload_session".to_string(),
            idempotency_key: "upload-session-7".to_string(),
            idempotency_fingerprint_sha256_hex: None,
        })
        .await
        .unwrap();
    drive.force_put_failure(true);

    let error = pipeline
        .run_existing_queued_job(
            ExistingMarkdownIngestJobParams {
                space_id: 7,
                job_id: created.job.id,
                title: "Upload payload".to_string(),
                payload_markdown: "# Upload".to_string(),
                ingest_provider: "upload-session".to_string(),
                source_drive_prefix: format!("upload_sessions/{}", created.job.id),
                payload_logical_path: format!("inbox/api/{}/payload.md", created.job.id),
            },
            None,
        )
        .await
        .unwrap_err();

    assert!(error.to_string().contains("forced drive put failure"));
    assert_eq!(
        jobs.get_job(created.job.id).await.unwrap().state,
        IngestionJobState::Failed
    );
}

#[tokio::test]
async fn drive_linkage_failure_transitions_running_job_to_failed() {
    let drive = RecordingDrive::default();
    let jobs = MemoryIngestionJobStore::default();
    let metadata = MemoryMarkdownIndexMetadataStore::default();
    let pipeline = ApiMarkdownIngestPipeline::new(&drive, &jobs, &metadata);
    jobs.force_linkage_failure(true);

    let error = pipeline
        .run(
            KnowledgeIngestRequest {
                space_id: 7,
                title: "Linkage failure".to_string(),
                payload_markdown: "# Linkage".to_string(),
                idempotency_key: "linkage-failure-1".to_string(),
                source_url: None,
            },
            None,
            "api-ingest",
        )
        .await
        .unwrap_err();

    assert!(error.to_string().contains("forced linkage failure"));
    assert_eq!(
        jobs.get_job(1).await.unwrap().state,
        IngestionJobState::Failed
    );
}

#[derive(Default)]
struct RecordingDrive {
    objects: Mutex<HashMap<String, Vec<u8>>>,
    write_count: Mutex<usize>,
    force_put_failure: Mutex<bool>,
}

impl RecordingDrive {
    fn write_count(&self) -> usize {
        *self.write_count.lock().unwrap()
    }

    fn force_put_failure(&self, value: bool) {
        *self.force_put_failure.lock().unwrap() = value;
    }
}

#[async_trait]
impl KnowledgeDriveStorage for RecordingDrive {
    async fn put_object(
        &self,
        request: PutKnowledgeObjectRequest,
    ) -> Result<KnowledgeObjectRef, KnowledgeStorageError> {
        if *self.force_put_failure.lock().unwrap() {
            return Err(KnowledgeStorageError::Internal(
                "forced drive put failure".to_string(),
            ));
        }
        *self.write_count.lock().unwrap() += 1;
        self.objects
            .lock()
            .unwrap()
            .insert(request.logical_path.clone(), request.body.clone());
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
        let objects = self.objects.lock().unwrap();
        let body = objects
            .get(&request.object_key)
            .ok_or_else(|| KnowledgeStorageError::NotFound(request.object_key.clone()))?;
        Ok(KnowledgeObjectRef {
            storage_provider_id: "provider-kb".to_string(),
            bucket: "test".to_string(),
            object_key: request.object_key.clone(),
            logical_path: request
                .logical_path
                .unwrap_or_else(|| request.object_key.clone()),
            object_role: request.object_role,
            content_type: "text/markdown; charset=utf-8".to_string(),
            size_bytes: body.len() as u64,
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
            .and_then(|bytes| String::from_utf8(bytes.clone()).ok())
            .ok_or_else(|| KnowledgeStorageError::NotFound(object_ref.logical_path.clone()))
    }
}

#[derive(Default)]
struct MemoryIngestionJobStore {
    next_id: Mutex<u64>,
    by_id: Mutex<HashMap<u64, IngestionJob>>,
    by_key: Mutex<HashMap<(u64, String), u64>>,
    linkages: Mutex<HashMap<u64, DriveImportJobLinkage>>,
    force_complete_failure: Mutex<bool>,
    force_linkage_failure: Mutex<bool>,
}

impl MemoryIngestionJobStore {
    fn force_next_complete_failure(&self, value: bool) {
        *self.force_complete_failure.lock().unwrap() = value;
    }

    fn force_linkage_failure(&self, value: bool) {
        *self.force_linkage_failure.lock().unwrap() = value;
    }

    async fn get_job(&self, job_id: u64) -> Result<IngestionJob, IngestionJobStoreError> {
        self.by_id
            .lock()
            .unwrap()
            .get(&job_id)
            .cloned()
            .ok_or(IngestionJobStoreError::NotFound(job_id))
    }
}

#[async_trait]
impl IngestionJobStore for MemoryIngestionJobStore {
    async fn create_or_get_job(
        &self,
        record: CreateIngestionJobRecord,
    ) -> Result<CreateOrGetIngestionJobResult, IngestionJobStoreError> {
        let key = (record.space_id, record.idempotency_key.clone());
        if let Some(existing_id) = self.by_key.lock().unwrap().get(&key).copied() {
            let job = self
                .by_id
                .lock()
                .unwrap()
                .get(&existing_id)
                .cloned()
                .ok_or_else(|| IngestionJobStoreError::Internal("missing job".to_string()))?;
            return Ok(CreateOrGetIngestionJobResult {
                job,
                created: false,
            });
        }

        let mut next_id = self.next_id.lock().unwrap();
        *next_id += 1;
        let job = IngestionJob {
            id: *next_id,
            space_id: record.space_id,
            source_type: record.source_type,
            idempotency_key: record.idempotency_key,
            state: IngestionJobState::Queued,
            error_message: None,
        };
        self.by_key.lock().unwrap().insert(key, job.id);
        self.by_id.lock().unwrap().insert(job.id, job.clone());
        Ok(CreateOrGetIngestionJobResult { job, created: true })
    }

    async fn get_job(&self, job_id: u64) -> Result<IngestionJob, IngestionJobStoreError> {
        MemoryIngestionJobStore::get_job(self, job_id).await
    }

    async fn update_job_state(
        &self,
        job_id: u64,
        expected_state: IngestionJobState,
        state: IngestionJobState,
        error_message: Option<String>,
    ) -> Result<IngestionJob, IngestionJobStoreError> {
        let mut jobs = self.by_id.lock().unwrap();
        let job = jobs
            .get_mut(&job_id)
            .ok_or(IngestionJobStoreError::NotFound(job_id))?;
        if job.state != expected_state {
            return Err(IngestionJobStoreError::NotFound(job_id));
        }
        job.state = state;
        job.error_message = error_message;
        Ok(job.clone())
    }

    async fn attach_drive_import_linkage(
        &self,
        job_id: u64,
        linkage: DriveImportJobLinkage,
    ) -> Result<(), IngestionJobStoreError> {
        if *self.force_linkage_failure.lock().unwrap() {
            return Err(IngestionJobStoreError::Internal(
                "forced linkage failure".to_string(),
            ));
        }
        if !self.by_id.lock().unwrap().contains_key(&job_id) {
            return Err(IngestionJobStoreError::NotFound(job_id));
        }
        self.linkages.lock().unwrap().insert(job_id, linkage);
        Ok(())
    }

    async fn get_drive_import_linkage(
        &self,
        job_id: u64,
    ) -> Result<Option<DriveImportJobLinkage>, IngestionJobStoreError> {
        if !self.by_id.lock().unwrap().contains_key(&job_id) {
            return Err(IngestionJobStoreError::NotFound(job_id));
        }
        Ok(self.linkages.lock().unwrap().get(&job_id).cloned())
    }

    async fn list_jobs_by_state(
        &self,
        _state: IngestionJobState,
        _limit: u32,
    ) -> Result<Vec<IngestionJob>, IngestionJobStoreError> {
        Ok(Vec::new())
    }

    async fn mark_running_job_succeeded_with_outbox(
        &self,
        job_id: u64,
        _outbox: sdkwork_intelligence_knowledgebase_service::ports::knowledge_outbox_store::AppendOutboxEventRecord,
    ) -> Result<IngestionJob, IngestionJobStoreError> {
        let mut jobs = self.by_id.lock().unwrap();
        let job = jobs
            .get_mut(&job_id)
            .ok_or(IngestionJobStoreError::NotFound(job_id))?;
        if job.state != IngestionJobState::Running {
            return Err(IngestionJobStoreError::Conflict(format!(
                "invalid ingestion job transition: {:?} -> {:?}",
                job.state,
                IngestionJobState::Succeeded
            )));
        }
        job.state = IngestionJobState::Succeeded;
        job.error_message = None;
        Ok(job.clone())
    }

    async fn complete_running_ingestion_with_chunks_and_outbox(
        &self,
        record: sdkwork_intelligence_knowledgebase_service::ports::knowledge_ingestion_job_store::CompleteRunningIngestionRecord,
    ) -> Result<
        sdkwork_intelligence_knowledgebase_service::ports::knowledge_ingestion_job_store::CompletedIngestionResult,
        IngestionJobStoreError,
    >{
        if *self.force_complete_failure.lock().unwrap() {
            return Err(IngestionJobStoreError::Internal(
                "forced completion failure".to_string(),
            ));
        }
        let job = self
            .mark_running_job_succeeded_with_outbox(record.job_id, record.outbox)
            .await?;
        Ok(
            sdkwork_intelligence_knowledgebase_service::ports::knowledge_ingestion_job_store::CompletedIngestionResult {
                job,
                chunk_count: record.chunks.len(),
            },
        )
    }
}

#[derive(Default)]
struct MemoryMarkdownIndexMetadataStore {
    prepare_count: Mutex<usize>,
    next_source_id: Mutex<u64>,
    next_document_id: Mutex<u64>,
    next_version_id: Mutex<u64>,
    next_object_ref_id: Mutex<u64>,
}

impl MemoryMarkdownIndexMetadataStore {
    fn prepare_count(&self) -> usize {
        *self.prepare_count.lock().unwrap()
    }
}

#[async_trait]
impl MarkdownIndexMetadataStore for MemoryMarkdownIndexMetadataStore {
    async fn create_or_prepare_markdown_index_metadata(
        &self,
        record: PrepareMarkdownIndexMetadataRecord,
    ) -> Result<PreparedMarkdownIndexMetadata, MarkdownIndexMetadataStoreError> {
        *self.prepare_count.lock().unwrap() += 1;
        let source_id = {
            let mut next = self.next_source_id.lock().unwrap();
            *next += 1;
            *next
        };
        let document_id = {
            let mut next = self.next_document_id.lock().unwrap();
            *next += 1;
            *next
        };
        let version_id = {
            let mut next = self.next_version_id.lock().unwrap();
            *next += 1;
            *next
        };
        let object_ref_id = {
            let mut next = self.next_object_ref_id.lock().unwrap();
            *next += 1;
            *next
        };

        Ok(PreparedMarkdownIndexMetadata {
            source_id,
            object_ref: sdkwork_knowledgebase_contract::KnowledgeDriveObjectRef {
                id: object_ref_id,
                space_id: record.object_ref.space_id,
                drive_space_id: record.object_ref.drive_space_id.clone(),
                drive_node_id: record.object_ref.drive_node_id.clone(),
                logical_path: record.object_ref.logical_path.clone(),
                drive_provider_kind: record.object_ref.drive_provider_kind.clone(),
                drive_storage_provider_id: record.object_ref.drive_storage_provider_id.clone(),
                drive_bucket: record.object_ref.drive_bucket.clone(),
                drive_object_key: record.object_ref.drive_object_key.clone(),
                drive_object_version: record.object_ref.drive_object_version.clone(),
                drive_etag: record.object_ref.drive_etag.clone(),
                content_type: record.object_ref.content_type.clone(),
                size_bytes: record.object_ref.size_bytes,
                checksum_sha256_hex: record.object_ref.checksum_sha256_hex.clone(),
                object_role: record.object_ref.object_role.clone(),
                access_mode: record.object_ref.access_mode.clone(),
            },
            document: sdkwork_knowledgebase_contract::document::KnowledgeDocument {
                id: document_id,
                space_id: record.document.space_id,
                source_id: Some(source_id),
                original_file_drive_node_id: record.document.original_file_drive_node_id.clone(),
                title: record.document.title.clone(),
                mime_type: record.document.mime_type.clone(),
                language: record.document.language.clone(),
                content_state:
                    sdkwork_knowledgebase_contract::document::KnowledgeDocumentState::Ready,
                visibility:
                    sdkwork_knowledgebase_contract::document::KnowledgeDocumentVisibility::Private,
                current_version_id: Some(version_id),
                index_state:
                    sdkwork_knowledgebase_contract::document::KnowledgeDocumentVersionState::Pending,
            },
            version: sdkwork_knowledgebase_contract::document::KnowledgeDocumentVersion {
                id: version_id,
                document_id,
                version_no: record.version.version_no,
                original_object_ref_id: object_ref_id,
                checksum_sha256_hex: record.version.checksum_sha256_hex.clone(),
                size_bytes: record.version.size_bytes,
                mime_type: record.version.mime_type.clone(),
                parse_state:
                    sdkwork_knowledgebase_contract::document::KnowledgeDocumentVersionState::Pending,
                index_state:
                    sdkwork_knowledgebase_contract::document::KnowledgeDocumentVersionState::Pending,
            },
        })
    }
}
