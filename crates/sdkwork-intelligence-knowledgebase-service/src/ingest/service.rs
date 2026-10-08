use crate::ingest::idempotency::api_payload_idempotency_fingerprint_with_source;
use crate::ingest::web_link_fetch::{fetch_web_link_markdown, WebLinkFetchError};
use crate::ports::{
    knowledge_drive_storage::{
        HeadKnowledgeObjectRequest, KnowledgeDriveStorage, KnowledgeObjectRef,
        KnowledgeStorageError, PutKnowledgeObjectRequest,
    },
    knowledge_ingestion_job_store::{
        CompleteRunningIngestionRecord, CompletedIngestionResult, CreateIngestionJobRecord,
        IngestionJobStore, IngestionJobStoreError,
    },
    knowledge_outbox_store::AppendOutboxEventRecord,
};
use sdkwork_knowledgebase_contract::ingest::{
    CreateIngestionJobRequest, IngestionJob, IngestionJobState, KnowledgeIngestRequest,
};
use sdkwork_knowledgebase_observability::{
    deployment_tenant_id, record_ingest_job_failed, record_ingest_job_succeeded,
};
use sdkwork_utils_rust::is_blank;
use thiserror::Error;

pub struct KnowledgeIngestionService<'a> {
    store: &'a dyn IngestionJobStore,
}

impl<'a> KnowledgeIngestionService<'a> {
    pub fn new(store: &'a dyn IngestionJobStore) -> Self {
        Self { store }
    }

    pub async fn create_job(
        &self,
        request: CreateIngestionJobRequest,
    ) -> Result<IngestionJob, KnowledgeIngestionServiceError> {
        if request.space_id == 0 {
            return Err(KnowledgeIngestionServiceError::InvalidRequest(
                "space_id is required".to_string(),
            ));
        }
        let idempotency_key = normalize_idempotency_key(&request.idempotency_key)
            .map_err(KnowledgeIngestionServiceError::InvalidRequest)?;

        self.store
            .create_or_get_job(CreateIngestionJobRecord {
                space_id: request.space_id,
                source_type: request.source_type,
                idempotency_key,
                idempotency_fingerprint_sha256_hex: None,
            })
            .await
            .map(|result| result.job)
            .map_err(KnowledgeIngestionServiceError::Store)
    }

    pub async fn mark_running(
        &self,
        job_id: u64,
    ) -> Result<IngestionJob, KnowledgeIngestionServiceError> {
        self.transition(job_id, IngestionJobState::Running, None)
            .await
    }

    pub async fn mark_succeeded(
        &self,
        job_id: u64,
    ) -> Result<IngestionJob, KnowledgeIngestionServiceError> {
        self.transition(job_id, IngestionJobState::Succeeded, None)
            .await
    }

    pub async fn mark_failed(
        &self,
        job_id: u64,
        error_message: impl Into<String>,
    ) -> Result<IngestionJob, KnowledgeIngestionServiceError> {
        self.transition(
            job_id,
            IngestionJobState::Failed,
            Some(error_message.into()),
        )
        .await
    }

    pub async fn mark_failed_with_claim(
        &self,
        job_id: u64,
        claim_token: &str,
        error_message: impl Into<String>,
    ) -> Result<IngestionJob, KnowledgeIngestionServiceError> {
        let updated = self
            .store
            .fail_claimed_ingestion_job(job_id, claim_token, error_message.into())
            .await
            .map_err(KnowledgeIngestionServiceError::Store)?;
        record_ingest_job_failed(deployment_tenant_id(), updated.id, updated.space_id);
        Ok(updated)
    }

    pub async fn finalize_success_with_outbox(
        &self,
        job_id: u64,
    ) -> Result<IngestionJob, KnowledgeIngestionServiceError> {
        let current = self
            .store
            .get_job(job_id)
            .await
            .map_err(KnowledgeIngestionServiceError::Store)?;
        let updated = self
            .store
            .mark_running_job_succeeded_with_outbox(job_id, ingest_success_outbox_record(&current))
            .await
            .map_err(KnowledgeIngestionServiceError::Store)?;
        record_ingest_job_succeeded(deployment_tenant_id(), updated.id, updated.space_id);
        Ok(updated)
    }

    pub async fn complete_with_chunks_and_outbox(
        &self,
        record: CompleteRunningIngestionRecord,
    ) -> Result<CompletedIngestionResult, KnowledgeIngestionServiceError> {
        self.store
            .complete_running_ingestion_with_chunks_and_outbox(record)
            .await
            .map_err(KnowledgeIngestionServiceError::Store)
    }

    async fn transition(
        &self,
        job_id: u64,
        next_state: IngestionJobState,
        error_message: Option<String>,
    ) -> Result<IngestionJob, KnowledgeIngestionServiceError> {
        let current = self.store.get_job(job_id).await?;
        if !is_valid_transition(current.state, next_state) {
            return Err(KnowledgeIngestionServiceError::InvalidTransition {
                from: current.state,
                to: next_state,
            });
        }

        let updated = self
            .store
            .update_job_state(job_id, current.state, next_state, error_message)
            .await
            .map_err(KnowledgeIngestionServiceError::Store)?;

        let tenant_id = deployment_tenant_id();
        match next_state {
            IngestionJobState::Succeeded => {
                record_ingest_job_succeeded(tenant_id, updated.id, updated.space_id);
            }
            IngestionJobState::Failed => {
                record_ingest_job_failed(tenant_id, updated.id, updated.space_id);
            }
            _ => {}
        }

        Ok(updated)
    }
}

fn is_valid_transition(from: IngestionJobState, to: IngestionJobState) -> bool {
    matches!(
        (from, to),
        (IngestionJobState::Queued, IngestionJobState::Running)
            | (IngestionJobState::Queued, IngestionJobState::Failed)
            | (IngestionJobState::Failed, IngestionJobState::Running)
            | (IngestionJobState::Running, IngestionJobState::Succeeded)
            | (IngestionJobState::Running, IngestionJobState::Failed)
    )
}

fn is_safe_idempotency_key(value: &str) -> bool {
    let value = value.trim();
    !value.is_empty()
        && value.len() <= 128
        && value
            .chars()
            .all(|ch| ch.is_ascii_alphanumeric() || ch == '-' || ch == '_' || ch == '.')
}

fn normalize_idempotency_key(value: &str) -> Result<String, String> {
    let value = value.trim();
    if value.is_empty() {
        return Err("idempotency_key is required".to_string());
    }
    if !is_safe_idempotency_key(value) {
        return Err("idempotency_key contains unsafe characters".to_string());
    }
    Ok(value.to_string())
}

#[derive(Debug, Error)]
pub enum KnowledgeIngestionServiceError {
    #[error("invalid ingestion request: {0}")]
    InvalidRequest(String),
    #[error("invalid ingestion job transition: {from:?} -> {to:?}")]
    InvalidTransition {
        from: IngestionJobState,
        to: IngestionJobState,
    },
    #[error(transparent)]
    Store(#[from] IngestionJobStoreError),
}

pub struct KnowledgeApiPayloadIngestService<'a> {
    drive: &'a dyn KnowledgeDriveStorage,
    jobs: &'a dyn IngestionJobStore,
}

impl<'a> KnowledgeApiPayloadIngestService<'a> {
    pub fn new(drive: &'a dyn KnowledgeDriveStorage, jobs: &'a dyn IngestionJobStore) -> Self {
        Self { drive, jobs }
    }

    pub async fn ingest_markdown_payload(
        &self,
        request: KnowledgeIngestRequest,
        drive_space_id: Option<&str>,
    ) -> Result<ApiPayloadIngestResult, KnowledgeApiPayloadIngestServiceError> {
        if request.space_id == 0 {
            return Err(KnowledgeApiPayloadIngestServiceError::InvalidRequest(
                "space_id is required".to_string(),
            ));
        }
        if is_blank(Some(request.title.as_str())) {
            return Err(KnowledgeApiPayloadIngestServiceError::InvalidRequest(
                "title is required".to_string(),
            ));
        }
        let payload_markdown = resolve_ingest_payload_markdown(&request).await?;
        let idempotency_key = normalize_idempotency_key(&request.idempotency_key)
            .map_err(KnowledgeApiPayloadIngestServiceError::InvalidRequest)?;

        let idempotency_fingerprint = api_payload_idempotency_fingerprint_with_source(
            request.space_id,
            request.title.as_str(),
            payload_markdown.as_str(),
            request.source_url.as_deref(),
        );
        let job_result = self
            .jobs
            .create_or_get_job(CreateIngestionJobRecord {
                space_id: request.space_id,
                source_type: "api".to_string(),
                idempotency_key,
                idempotency_fingerprint_sha256_hex: Some(idempotency_fingerprint),
            })
            .await
            .map_err(KnowledgeApiPayloadIngestServiceError::Store)?;
        let job = job_result.job;
        let payload_path = format!("inbox/api/{}/payload.md", job.id);

        if !job_result.created {
            match self
                .drive
                .head_object(
                    HeadKnowledgeObjectRequest::managed_artifact(
                        payload_path.clone(),
                        "api_payload",
                    )
                    .with_drive_space_id(drive_space_id),
                )
                .await
            {
                Ok(existing_payload) => {
                    // The stored payload is the authoritative replay content: silently
                    // falling back to the new request's markdown would answer an
                    // idempotent replay with different content than what the original
                    // job actually ingested.
                    let resolved_payload_markdown = self
                        .drive
                        .get_object_text_bounded(
                            &existing_payload,
                            crate::ingest::MAX_MARKDOWN_PAYLOAD_BYTES as u64,
                        )
                        .await?;
                    return Ok(ApiPayloadIngestResult {
                        payload_object_ref: existing_payload,
                        job,
                        resolved_payload_markdown,
                    });
                }
                Err(KnowledgeStorageError::NotFound(_)) => {}
                Err(error) => return Err(KnowledgeApiPayloadIngestServiceError::Storage(error)),
            }
        }

        let payload_object_ref = self
            .drive
            .put_object(
                PutKnowledgeObjectRequest::text(
                    payload_path,
                    "api_payload",
                    payload_markdown.clone(),
                    None,
                )
                .with_drive_space_id(drive_space_id),
            )
            .await?;

        Ok(ApiPayloadIngestResult {
            payload_object_ref,
            job,
            resolved_payload_markdown: payload_markdown,
        })
    }
}

#[derive(Debug, Clone)]
pub struct ApiPayloadIngestResult {
    pub payload_object_ref: KnowledgeObjectRef,
    pub job: IngestionJob,
    pub resolved_payload_markdown: String,
}

#[derive(Debug, Error)]
pub enum KnowledgeApiPayloadIngestServiceError {
    #[error("invalid api payload ingest request: {0}")]
    InvalidRequest(String),
    #[error(transparent)]
    WebLink(#[from] WebLinkFetchError),
    #[error(transparent)]
    Storage(#[from] KnowledgeStorageError),
    #[error(transparent)]
    Store(#[from] IngestionJobStoreError),
}

async fn resolve_ingest_payload_markdown(
    request: &KnowledgeIngestRequest,
) -> Result<String, KnowledgeApiPayloadIngestServiceError> {
    if let Some(source_url) = request
        .source_url
        .as_deref()
        .filter(|value| !is_blank(Some(*value)))
    {
        return fetch_web_link_markdown(source_url, request.title.as_str())
            .await
            .map_err(Into::into);
    }

    if is_blank(Some(request.payload_markdown.as_str())) {
        return Err(KnowledgeApiPayloadIngestServiceError::InvalidRequest(
            "payload_markdown or source_url is required".to_string(),
        ));
    }

    Ok(request.payload_markdown.clone())
}

pub fn ingest_success_outbox_record(job: &IngestionJob) -> AppendOutboxEventRecord {
    AppendOutboxEventRecord {
        aggregate_type: "ingestion_job".to_string(),
        aggregate_id: job.id,
        event_type: "knowledge.ingest.succeeded".to_string(),
        payload_json: serde_json::json!({
            "spaceId": job.space_id,
            "sourceType": job.source_type,
            "idempotencyKey": job.idempotency_key,
            "state": "succeeded",
        })
        .to_string(),
    }
}
