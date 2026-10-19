use crate::document_extraction::{
    bound_extracted_text, is_textual_content_type, DocumentExtractionError,
    DocumentExtractionRequest, DocumentTextExtractor, DEFAULT_DOCUMENT_EXTRACTION_TIMEOUT,
    MAX_DOCUMENT_EXTRACTION_INPUT_BYTES,
};
use crate::ingest::{
    ingest_success_outbox_record, split_markdown_chunks, validate_markdown_payload,
    KnowledgeIngestionService, MarkdownIndexResult, PayloadLimitError, MAX_MARKDOWN_PAYLOAD_BYTES,
};
use crate::ports::knowledge_drive_storage::{KnowledgeDriveStorage, KnowledgeObjectRef};
use crate::ports::knowledge_ingestion_job_store::{
    CompleteRunningIngestionRecord, IngestionJobStore,
};
use sdkwork_knowledgebase_contract::drive::KnowledgeDriveObjectRef;
use sdkwork_knowledgebase_contract::ingest::{IngestionJob, KnowledgeDriveImportResult};
use std::sync::Arc;
use std::time::Duration;
use thiserror::Error;

pub struct KnowledgeDriveImportPipelineService<'a> {
    drive: &'a dyn KnowledgeDriveStorage,
    jobs: &'a dyn IngestionJobStore,
    document_text_extractor: Option<Arc<dyn DocumentTextExtractor>>,
    extraction_timeout: Duration,
}

impl<'a> KnowledgeDriveImportPipelineService<'a> {
    pub fn new(drive: &'a dyn KnowledgeDriveStorage, jobs: &'a dyn IngestionJobStore) -> Self {
        Self {
            drive,
            jobs,
            document_text_extractor: None,
            extraction_timeout: DEFAULT_DOCUMENT_EXTRACTION_TIMEOUT,
        }
    }

    /// Enables bounded binary-document extraction (PDF/DOCX/PPTX).
    #[must_use]
    pub fn with_document_text_extractor(
        mut self,
        extractor: Option<Arc<dyn DocumentTextExtractor>>,
    ) -> Self {
        self.document_text_extractor = extractor;
        self
    }

    /// Overrides the extraction deadline.
    #[must_use]
    pub fn with_extraction_timeout(mut self, timeout: Duration) -> Self {
        self.extraction_timeout = timeout.max(Duration::from_millis(1));
        self
    }

    pub async fn process_import_result(
        &self,
        import: &KnowledgeDriveImportResult,
    ) -> Result<DriveImportPipelineResult, KnowledgeDriveImportPipelineServiceError> {
        self.process_import_result_with_claim(import, None).await
    }

    pub async fn process_claimed_import_result(
        &self,
        import: &KnowledgeDriveImportResult,
        claim_token: &str,
    ) -> Result<DriveImportPipelineResult, KnowledgeDriveImportPipelineServiceError> {
        self.process_import_result_with_claim(import, Some(claim_token))
            .await
    }

    async fn process_import_result_with_claim(
        &self,
        import: &KnowledgeDriveImportResult,
        claim_token: Option<&str>,
    ) -> Result<DriveImportPipelineResult, KnowledgeDriveImportPipelineServiceError> {
        let ingestion = KnowledgeIngestionService::new(self.jobs);
        let job = match import.job.state {
            sdkwork_knowledgebase_contract::ingest::IngestionJobState::Queued => ingestion
                .mark_running(import.job.id)
                .await
                .map_err(KnowledgeDriveImportPipelineServiceError::Ingestion)?,
            sdkwork_knowledgebase_contract::ingest::IngestionJobState::Running => {
                import.job.clone()
            }
            _ => {
                return Ok(DriveImportPipelineResult {
                    job: import.job.clone(),
                    index_result: None,
                });
            }
        };

        let object_ref = drive_object_ref_to_storage_ref(&import.original_object_ref);
        let mime_type = import.version.mime_type.as_deref();
        let file_name = import
            .original_object_ref
            .logical_path
            .as_deref()
            .or(Some(import.original_object_ref.drive_object_key.as_str()));
        let payload = match self
            .resolve_payload(&object_ref, mime_type, file_name)
            .await
        {
            Ok(payload) => payload,
            Err(error) => {
                if let Err(mark_error) = mark_ingestion_failed(
                    &ingestion,
                    job.id,
                    claim_token,
                    format!("drive import payload failed: {error}"),
                )
                .await
                {
                    tracing::error!(
                        job_id = job.id,
                        ?mark_error,
                        "failed to mark ingestion job as failed after drive import payload error"
                    );
                }
                return Err(error);
            }
        };
        let payload = truncate_to_payload_limit(payload);

        if let Err(limit_error) = validate_markdown_payload(&payload) {
            let failure_message = format!("drive import payload rejected: {limit_error}");
            if let Err(mark_error) =
                mark_ingestion_failed(&ingestion, job.id, claim_token, failure_message.clone())
                    .await
            {
                tracing::error!(
                    job_id = job.id,
                    ?mark_error,
                    "failed to mark ingestion job as failed after drive import payload limit violation"
                );
            }
            return Err(KnowledgeDriveImportPipelineServiceError::PayloadLimit(
                limit_error,
            ));
        }

        let chunk_records = split_markdown_chunks(
            import.document.space_id,
            import.document.id,
            import.version.id,
            &payload,
        );
        let completed = match ingestion
            .complete_with_chunks_and_outbox(CompleteRunningIngestionRecord {
                job_id: job.id,
                claim_token: claim_token.map(str::to_string),
                document_version_id: import.version.id,
                chunks: chunk_records,
                outbox: ingest_success_outbox_record(&job),
            })
            .await
        {
            Ok(completed) => completed,
            Err(error) => {
                if let Err(mark_error) =
                    mark_ingestion_failed(&ingestion, job.id, claim_token, format!("{error:?}"))
                        .await
                {
                    tracing::error!(
                        job_id = job.id,
                        ?mark_error,
                        "failed to mark ingestion job as failed after drive import completion error"
                    );
                }
                return Err(KnowledgeDriveImportPipelineServiceError::Ingestion(error));
            }
        };

        Ok(DriveImportPipelineResult {
            job: completed.job,
            index_result: Some(MarkdownIndexResult {
                document_version_id: import.version.id,
                chunk_count: completed.chunk_count,
            }),
        })
    }

    /// Resolves the textual payload for one Drive import.
    ///
    /// Text payloads are read as bounded UTF-8 exactly as before. Binary
    /// documents (PDF/DOCX/PPTX) are downloaded as bounded bytes and decoded by
    /// the configured extractor on a blocking worker with a deadline. An
    /// unsupported binary format fails closed instead of indexing mojibake.
    async fn resolve_payload(
        &self,
        object_ref: &KnowledgeObjectRef,
        mime_type: Option<&str>,
        file_name: Option<&str>,
    ) -> Result<String, KnowledgeDriveImportPipelineServiceError> {
        let mime_type = mime_type
            .map(str::trim)
            .filter(|value| !value.is_empty())
            .unwrap_or("application/octet-stream")
            .to_string();

        if is_textual_content_type(&mime_type, file_name) {
            return self
                .drive
                .get_object_text_bounded(
                    object_ref,
                    crate::ingest::MAX_MARKDOWN_PAYLOAD_BYTES as u64,
                )
                .await
                .map_err(KnowledgeDriveImportPipelineServiceError::Storage);
        }

        let Some(extractor) = self.document_text_extractor.clone() else {
            return Err(KnowledgeDriveImportPipelineServiceError::UnsupportedDocument(mime_type));
        };
        if !extractor.supports(&mime_type, file_name) {
            return Err(KnowledgeDriveImportPipelineServiceError::UnsupportedDocument(mime_type));
        }

        let bytes = self
            .drive
            .get_object_bytes_bounded(object_ref, MAX_DOCUMENT_EXTRACTION_INPUT_BYTES)
            .await
            .map_err(KnowledgeDriveImportPipelineServiceError::Storage)?;
        let request = DocumentExtractionRequest {
            content_type: mime_type,
            file_name: file_name.map(str::to_string),
            bytes,
        };
        let timeout = self.extraction_timeout;
        let extraction = tokio::time::timeout(
            timeout,
            tokio::task::spawn_blocking(move || extractor.extract(request)),
        )
        .await
        .map_err(|_| KnowledgeDriveImportPipelineServiceError::ExtractionTimedOut { timeout })?
        .map_err(|error| {
            KnowledgeDriveImportPipelineServiceError::ExtractionWorkerFailed(format!(
                "document extraction worker failed: {error}"
            ))
        })?
        .map_err(KnowledgeDriveImportPipelineServiceError::Extraction)?;
        let extraction = bound_extracted_text(extraction);

        if extraction.truncated {
            tracing::warn!(
                logical_path = %object_ref.logical_path,
                format = %extraction.format,
                "extracted document text exceeded the global character limit and was truncated"
            );
        }
        Ok(extraction.text)
    }
}

async fn mark_ingestion_failed(
    ingestion: &KnowledgeIngestionService<'_>,
    job_id: u64,
    claim_token: Option<&str>,
    detail: String,
) -> Result<IngestionJob, crate::ingest::KnowledgeIngestionServiceError> {
    match claim_token {
        Some(claim_token) => {
            ingestion
                .mark_failed_with_claim(job_id, claim_token, detail)
                .await
        }
        None => ingestion.mark_failed(job_id, detail).await,
    }
}

#[derive(Debug, Clone)]
pub struct DriveImportPipelineResult {
    pub job: IngestionJob,
    pub index_result: Option<MarkdownIndexResult>,
}

fn drive_object_ref_to_storage_ref(value: &KnowledgeDriveObjectRef) -> KnowledgeObjectRef {
    KnowledgeObjectRef {
        storage_provider_id: value.drive_storage_provider_id.clone(),
        bucket: value.drive_bucket.clone(),
        object_key: value.drive_object_key.clone(),
        logical_path: value
            .logical_path
            .clone()
            .unwrap_or_else(|| value.drive_object_key.clone()),
        object_role: value.object_role.clone(),
        content_type: value
            .content_type
            .clone()
            .unwrap_or_else(|| "text/plain".to_string()),
        size_bytes: value.size_bytes,
        checksum_sha256_hex: value.checksum_sha256_hex.clone(),
        etag: value.drive_etag.clone(),
        version_id: value.drive_object_version.clone(),
    }
}

/// Truncates an extracted payload to the markdown ingestion byte ceiling on a
/// character boundary. Oversized documents stay indexable instead of failing
/// the whole job; the cut is logged by the caller.
fn truncate_to_payload_limit(payload: String) -> String {
    if payload.len() <= MAX_MARKDOWN_PAYLOAD_BYTES {
        return payload;
    }
    let mut truncated = String::with_capacity(MAX_MARKDOWN_PAYLOAD_BYTES);
    let mut used = 0usize;
    for character in payload.chars() {
        let width = character.len_utf8();
        if used + width > MAX_MARKDOWN_PAYLOAD_BYTES {
            break;
        }
        truncated.push(character);
        used += width;
    }
    tracing::warn!(
        original_bytes = payload.len(),
        retained_bytes = used,
        "drive import payload exceeded the markdown byte ceiling and was truncated"
    );
    truncated
}

#[derive(Debug, Error)]
pub enum KnowledgeDriveImportPipelineServiceError {
    #[error(transparent)]
    Ingestion(#[from] crate::ingest::KnowledgeIngestionServiceError),
    #[error(transparent)]
    Storage(#[from] crate::ports::knowledge_drive_storage::KnowledgeStorageError),
    #[error(transparent)]
    PayloadLimit(#[from] PayloadLimitError),
    #[error("unsupported document content type: {0}")]
    UnsupportedDocument(String),
    #[error(transparent)]
    Extraction(#[from] DocumentExtractionError),
    #[error("document extraction did not complete within {timeout:?}")]
    ExtractionTimedOut { timeout: Duration },
    #[error("document extraction worker failed: {0}")]
    ExtractionWorkerFailed(String),
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ports::knowledge_drive_storage::{
        HeadKnowledgeObjectRequest, PutKnowledgeObjectRequest,
    };
    use crate::ports::knowledge_ingestion_job_store::{
        ClaimIngestionJobsRequest, ClaimedIngestionJob, CompleteRunningIngestionRecord,
        CompletedIngestionResult, CreateIngestionJobRecord, CreateOrGetIngestionJobResult,
        DriveImportJobLinkage, IngestionJobStoreError,
    };
    use crate::ports::knowledge_outbox_store::AppendOutboxEventRecord;
    use async_trait::async_trait;
    use sdkwork_knowledgebase_contract::ingest::IngestionJobState;
    use std::sync::atomic::{AtomicUsize, Ordering};
    use time::{Duration as TimeDuration, OffsetDateTime};

    struct FakeDrive {
        text: String,
        bytes: Vec<u8>,
        byte_reads: AtomicUsize,
    }

    impl FakeDrive {
        fn new(text: &str, bytes: Vec<u8>) -> Self {
            Self {
                text: text.to_string(),
                bytes,
                byte_reads: AtomicUsize::new(0),
            }
        }
    }

    #[async_trait]
    impl KnowledgeDriveStorage for FakeDrive {
        async fn put_object(
            &self,
            _request: PutKnowledgeObjectRequest,
        ) -> Result<KnowledgeObjectRef, crate::ports::knowledge_drive_storage::KnowledgeStorageError>
        {
            unimplemented!("pipeline payload tests never write objects")
        }

        async fn head_object(
            &self,
            _request: HeadKnowledgeObjectRequest,
        ) -> Result<KnowledgeObjectRef, crate::ports::knowledge_drive_storage::KnowledgeStorageError>
        {
            unimplemented!("pipeline payload tests never head objects")
        }

        async fn get_object_text(
            &self,
            _object_ref: &KnowledgeObjectRef,
        ) -> Result<String, crate::ports::knowledge_drive_storage::KnowledgeStorageError> {
            Ok(self.text.clone())
        }

        async fn get_object_text_bounded(
            &self,
            _object_ref: &KnowledgeObjectRef,
            _max_bytes: u64,
        ) -> Result<String, crate::ports::knowledge_drive_storage::KnowledgeStorageError> {
            Ok(self.text.clone())
        }

        async fn get_object_bytes(
            &self,
            _object_ref: &KnowledgeObjectRef,
        ) -> Result<Vec<u8>, crate::ports::knowledge_drive_storage::KnowledgeStorageError> {
            self.byte_reads.fetch_add(1, Ordering::SeqCst);
            Ok(self.bytes.clone())
        }
    }

    struct NoopJobs;

    #[async_trait]
    impl IngestionJobStore for NoopJobs {
        async fn create_or_get_job(
            &self,
            _record: CreateIngestionJobRecord,
        ) -> Result<CreateOrGetIngestionJobResult, IngestionJobStoreError> {
            unimplemented!("payload resolution never creates jobs")
        }

        async fn get_job(&self, _job_id: u64) -> Result<IngestionJob, IngestionJobStoreError> {
            unimplemented!("payload resolution never reads jobs")
        }

        async fn update_job_state(
            &self,
            _job_id: u64,
            _expected_state: IngestionJobState,
            _state: IngestionJobState,
            _error_message: Option<String>,
        ) -> Result<IngestionJob, IngestionJobStoreError> {
            unimplemented!("payload resolution never updates jobs")
        }

        async fn list_jobs_by_state(
            &self,
            _state: IngestionJobState,
            _limit: u32,
        ) -> Result<Vec<IngestionJob>, IngestionJobStoreError> {
            unimplemented!("payload resolution never lists jobs")
        }

        async fn claim_ingestion_jobs(
            &self,
            _request: ClaimIngestionJobsRequest,
        ) -> Result<Vec<ClaimedIngestionJob>, IngestionJobStoreError> {
            unimplemented!("payload resolution never claims jobs")
        }

        async fn renew_ingestion_job_lease(
            &self,
            _job_id: u64,
            _claim_token: &str,
            _lease_duration: TimeDuration,
        ) -> Result<OffsetDateTime, IngestionJobStoreError> {
            unimplemented!("payload resolution never renews leases")
        }

        async fn fail_claimed_ingestion_job(
            &self,
            _job_id: u64,
            _claim_token: &str,
            _error_message: String,
        ) -> Result<IngestionJob, IngestionJobStoreError> {
            unimplemented!("payload resolution never fails jobs")
        }

        async fn attach_drive_import_linkage(
            &self,
            _job_id: u64,
            _linkage: DriveImportJobLinkage,
        ) -> Result<(), IngestionJobStoreError> {
            unimplemented!("payload resolution never attaches linkage")
        }

        async fn get_drive_import_linkage(
            &self,
            _job_id: u64,
        ) -> Result<Option<DriveImportJobLinkage>, IngestionJobStoreError> {
            unimplemented!("payload resolution never reads linkage")
        }

        async fn mark_running_job_succeeded_with_outbox(
            &self,
            _job_id: u64,
            _outbox: AppendOutboxEventRecord,
        ) -> Result<IngestionJob, IngestionJobStoreError> {
            unimplemented!("payload resolution never completes jobs")
        }

        async fn complete_running_ingestion_with_chunks_and_outbox(
            &self,
            _record: CompleteRunningIngestionRecord,
        ) -> Result<CompletedIngestionResult, IngestionJobStoreError> {
            unimplemented!("payload resolution never completes jobs")
        }
    }

    struct FakeExtractor {
        text: String,
        delay: std::time::Duration,
    }

    impl DocumentTextExtractor for FakeExtractor {
        fn supports(&self, content_type: &str, _file_name: Option<&str>) -> bool {
            content_type == "application/pdf"
        }

        fn extract(
            &self,
            request: DocumentExtractionRequest,
        ) -> Result<crate::document_extraction::ExtractedDocument, DocumentExtractionError>
        {
            assert!(!request.bytes.is_empty());
            std::thread::sleep(self.delay);
            Ok(crate::document_extraction::ExtractedDocument {
                text: self.text.clone(),
                format: "pdf".to_string(),
                pages: None,
                truncated: false,
            })
        }
    }

    fn object_ref() -> KnowledgeObjectRef {
        KnowledgeObjectRef {
            storage_provider_id: "local".to_string(),
            bucket: "bucket".to_string(),
            object_key: "key".to_string(),
            logical_path: "policy.pdf".to_string(),
            object_role: "original_document".to_string(),
            content_type: "application/pdf".to_string(),
            size_bytes: 32,
            checksum_sha256_hex: None,
            etag: None,
            version_id: None,
        }
    }

    #[tokio::test]
    async fn binary_document_is_extracted_before_chunking() {
        let drive = FakeDrive::new("not used", b"%PDF-1.7 fake".to_vec());
        let service = KnowledgeDriveImportPipelineService::new(&drive, &NoopJobs)
            .with_document_text_extractor(Some(Arc::new(FakeExtractor {
                text: "Extracted refund policy text.".to_string(),
                delay: std::time::Duration::ZERO,
            })));

        let payload = service
            .resolve_payload(&object_ref(), Some("application/pdf"), Some("policy.pdf"))
            .await
            .expect("pdf payload");

        assert_eq!(payload, "Extracted refund policy text.");
        assert_eq!(drive.byte_reads.load(Ordering::SeqCst), 1);
    }

    #[tokio::test]
    async fn binary_document_without_extractor_fails_closed() {
        let drive = FakeDrive::new("not used", b"%PDF-1.7 fake".to_vec());
        let service = KnowledgeDriveImportPipelineService::new(&drive, &NoopJobs);

        let error = service
            .resolve_payload(&object_ref(), Some("application/pdf"), Some("policy.pdf"))
            .await
            .expect_err("binary payload without extractor must fail");

        assert!(matches!(
            error,
            KnowledgeDriveImportPipelineServiceError::UnsupportedDocument(_)
        ));
        assert_eq!(drive.byte_reads.load(Ordering::SeqCst), 0);
    }

    #[tokio::test]
    async fn textual_payload_keeps_the_utf8_text_read_path() {
        let drive = FakeDrive::new("# Refund policy\n\n30 days.", Vec::new());
        let service = KnowledgeDriveImportPipelineService::new(&drive, &NoopJobs);

        let payload = service
            .resolve_payload(&object_ref(), Some("text/markdown"), Some("policy.md"))
            .await
            .expect("markdown payload");

        assert!(payload.contains("30 days"));
        assert_eq!(drive.byte_reads.load(Ordering::SeqCst), 0);
    }

    #[tokio::test]
    async fn extraction_deadline_is_enforced() {
        let drive = FakeDrive::new("not used", b"%PDF-1.7 fake".to_vec());
        let service = KnowledgeDriveImportPipelineService::new(&drive, &NoopJobs)
            .with_document_text_extractor(Some(Arc::new(FakeExtractor {
                text: "late".to_string(),
                delay: std::time::Duration::from_millis(200),
            })))
            .with_extraction_timeout(std::time::Duration::from_millis(10));

        let error = service
            .resolve_payload(&object_ref(), Some("application/pdf"), Some("policy.pdf"))
            .await
            .expect_err("slow extraction must time out");

        assert!(matches!(
            error,
            KnowledgeDriveImportPipelineServiceError::ExtractionTimedOut { .. }
        ));
    }
}
