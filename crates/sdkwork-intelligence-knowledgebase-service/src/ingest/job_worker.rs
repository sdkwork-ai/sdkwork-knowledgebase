use crate::imports::{
    DriveImportPipelineResult, KnowledgeDriveImportPipelineService,
    KnowledgeDriveImportPipelineServiceError,
};

use crate::ingest::KnowledgeIngestionService;

use crate::ports::knowledge_drive_storage::KnowledgeDriveStorage;

use crate::ports::knowledge_ingestion_job_store::{
    ClaimIngestionJobsRequest, ClaimedIngestionJob, DriveImportJobLinkage, IngestionJobStore,
    IngestionJobStoreError,
};

use sdkwork_knowledgebase_contract::document::{
    KnowledgeDocumentState, KnowledgeDocumentVersionState, KnowledgeDocumentVisibility,
};

use sdkwork_knowledgebase_contract::ingest::{IngestionJob, KnowledgeDriveImportResult};

use sdkwork_knowledgebase_contract::source::KnowledgeSourceType;

use sdkwork_knowledgebase_contract::{
    KnowledgeDocument, KnowledgeDocumentVersion, KnowledgeSource,
};

use thiserror::Error;
use time::Duration;

pub struct KnowledgeIngestionJobWorkerService<'a> {
    jobs: &'a dyn IngestionJobStore,

    drive: &'a dyn KnowledgeDriveStorage,
}

impl<'a> KnowledgeIngestionJobWorkerService<'a> {
    pub fn new(jobs: &'a dyn IngestionJobStore, drive: &'a dyn KnowledgeDriveStorage) -> Self {
        Self { jobs, drive }
    }

    pub async fn process_queued_jobs(
        &self,
        worker_id: &str,
        lease_duration: Duration,
        limit: u32,
        max_attempts: u32,
    ) -> Result<IngestionJobWorkerBatchResult, KnowledgeIngestionJobWorkerServiceError> {
        let jobs = self
            .jobs
            .claim_ingestion_jobs(ClaimIngestionJobsRequest {
                claim_owner: worker_id.to_string(),
                lease_duration,
                limit,
                max_attempts,
            })
            .await
            .map_err(KnowledgeIngestionJobWorkerServiceError::Store)?;

        let mut processed = 0usize;

        let skipped = 0usize;

        let mut failed = 0usize;

        for claimed in jobs {
            let job = &claimed.job;
            if job.source_type != KnowledgeSourceType::DriveObject.as_str() {
                self.fail_claimed_job(
                    job.id,
                    &claimed.claim_token,
                    format!("unsupported queued ingestion job type: {}", job.source_type),
                )
                .await;
                failed += 1;
                continue;
            }

            let import = match self.resolve_drive_import(job).await {
                Ok(Some(import)) => import,
                Ok(None) => {
                    self.fail_claimed_job(
                        job.id,
                        &claimed.claim_token,
                        "drive import linkage is missing".to_string(),
                    )
                    .await;
                    failed += 1;
                    continue;
                }
                Err(error) => {
                    self.fail_claimed_job(job.id, &claimed.claim_token, error.to_string())
                        .await;
                    failed += 1;
                    continue;
                }
            };

            match self
                .process_claimed_drive_import_result(&import, &claimed, lease_duration)
                .await
            {
                Ok(_) => processed += 1,

                Err(error) => {
                    tracing::warn!(

                        job_id = job.id,

                        error = %error,

                        "failed to process queued drive ingestion job"

                    );

                    failed += 1;
                }
            }
        }

        Ok(IngestionJobWorkerBatchResult {
            processed,

            skipped,

            failed,
        })
    }

    async fn fail_claimed_job(&self, job_id: u64, claim_token: &str, detail: String) {
        let ingestion = KnowledgeIngestionService::new(self.jobs);
        if let Err(error) = ingestion
            .mark_failed_with_claim(job_id, claim_token, detail)
            .await
        {
            tracing::error!(
                job_id,
                ?error,
                "failed to mark claimed ingestion job as failed"
            );
        }
    }

    async fn resolve_drive_import(
        &self,

        job: &IngestionJob,
    ) -> Result<Option<KnowledgeDriveImportResult>, KnowledgeIngestionJobWorkerServiceError> {
        let linkage = self
            .jobs
            .get_drive_import_linkage(job.id)
            .await
            .map_err(KnowledgeIngestionJobWorkerServiceError::Store)?;

        let Some(linkage) = linkage else {
            return Ok(None);
        };

        Ok(Some(build_drive_import_result(job, linkage)))
    }

    pub async fn process_drive_import_result(
        &self,

        import: &KnowledgeDriveImportResult,
    ) -> Result<DriveImportPipelineResult, KnowledgeDriveImportPipelineServiceError> {
        KnowledgeDriveImportPipelineService::new(self.drive, self.jobs)
            .process_import_result(import)
            .await
    }

    async fn process_claimed_drive_import_result(
        &self,
        import: &KnowledgeDriveImportResult,
        claimed: &ClaimedIngestionJob,
        lease_duration: Duration,
    ) -> Result<DriveImportPipelineResult, ClaimedDriveImportProcessingError> {
        let heartbeat_millis = (lease_duration.whole_milliseconds() / 3).max(1_000);
        let heartbeat_millis = u64::try_from(heartbeat_millis).unwrap_or(1_000);
        let heartbeat_period = std::time::Duration::from_millis(heartbeat_millis);
        let start = tokio::time::Instant::now() + heartbeat_period;
        let mut heartbeat = tokio::time::interval_at(start, heartbeat_period);
        heartbeat.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Delay);

        let pipeline = KnowledgeDriveImportPipelineService::new(self.drive, self.jobs);
        let processing = pipeline.process_claimed_import_result(import, &claimed.claim_token);
        tokio::pin!(processing);

        loop {
            tokio::select! {
                result = &mut processing => {
                    return result.map_err(ClaimedDriveImportProcessingError::Pipeline);
                }
                _ = heartbeat.tick() => {
                    self.jobs
                        .renew_ingestion_job_lease(
                            claimed.job.id,
                            &claimed.claim_token,
                            lease_duration,
                        )
                        .await
                        .map_err(ClaimedDriveImportProcessingError::LeaseLost)?;
                }
            }
        }
    }
}

fn build_drive_import_result(
    job: &IngestionJob,

    linkage: DriveImportJobLinkage,
) -> KnowledgeDriveImportResult {
    KnowledgeDriveImportResult {
        source: KnowledgeSource {
            id: linkage.source_id,

            space_id: job.space_id,

            source_type: KnowledgeSourceType::DriveObject,

            provider: Some("sdkwork-drive".to_string()),

            drive_bucket: Some(linkage.original_object_ref.drive_bucket.clone()),

            drive_prefix: Some(linkage.original_object_ref.drive_object_key.clone()),

            connector_metadata_json: None,
        },

        document: KnowledgeDocument {
            id: linkage.document_id,

            space_id: job.space_id,

            source_id: Some(linkage.source_id),

            original_file_drive_node_id: linkage.original_object_ref.drive_node_id.clone(),

            title: String::new(),

            mime_type: linkage.original_object_ref.content_type.clone(),

            language: None,

            current_version_id: Some(linkage.document_version_id),

            visibility: KnowledgeDocumentVisibility::Private,

            content_state: KnowledgeDocumentState::Ready,

            index_state: KnowledgeDocumentVersionState::Pending,
        },

        version: KnowledgeDocumentVersion {
            id: linkage.document_version_id,

            document_id: linkage.document_id,

            version_no: 1,

            original_object_ref_id: linkage.original_object_ref.id,

            checksum_sha256_hex: linkage.original_object_ref.checksum_sha256_hex.clone(),

            size_bytes: linkage.original_object_ref.size_bytes,

            mime_type: linkage.original_object_ref.content_type.clone(),

            parse_state: KnowledgeDocumentVersionState::Succeeded,

            index_state: KnowledgeDocumentVersionState::Pending,
        },

        original_object_ref: linkage.original_object_ref,

        job: job.clone(),
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]

pub struct IngestionJobWorkerBatchResult {
    pub processed: usize,

    pub skipped: usize,

    pub failed: usize,
}

#[derive(Debug, Error)]

pub enum KnowledgeIngestionJobWorkerServiceError {
    #[error(transparent)]
    Store(#[from] IngestionJobStoreError),
}

#[derive(Debug, Error)]
enum ClaimedDriveImportProcessingError {
    #[error(transparent)]
    Pipeline(#[from] KnowledgeDriveImportPipelineServiceError),
    #[error("ingestion job lease lost: {0}")]
    LeaseLost(IngestionJobStoreError),
}
