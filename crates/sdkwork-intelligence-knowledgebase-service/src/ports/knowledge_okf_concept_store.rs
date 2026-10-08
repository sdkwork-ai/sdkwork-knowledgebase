use async_trait::async_trait;
use sdkwork_knowledgebase_contract::okf::{
    KnowledgeOkfConcept, KnowledgeOkfConceptRevision, OkfConceptPublishState, OkfConceptSummary,
    OkfLogEntry, OkfRevisionReviewState,
};
use thiserror::Error;

const OKF_INTERNAL_SCAN_PAGE_SIZE: u32 = 200;
const MAX_OKF_INTERNAL_SCAN_ROWS: usize = 50_000;

pub async fn list_all_published_concept_summaries(
    store: &dyn KnowledgeOkfConceptStore,
    space_id: u64,
) -> Result<Vec<OkfConceptSummary>, KnowledgeOkfConceptStoreError> {
    let mut items = Vec::new();
    let mut cursor = None;
    let max_pages = MAX_OKF_INTERNAL_SCAN_ROWS / OKF_INTERNAL_SCAN_PAGE_SIZE as usize;

    for _ in 0..=max_pages {
        let (mut page, next_cursor, has_more) = store
            .list_concept_summaries_page(space_id, cursor.clone(), OKF_INTERNAL_SCAN_PAGE_SIZE)
            .await?;
        if items.len().saturating_add(page.len()) > MAX_OKF_INTERNAL_SCAN_ROWS {
            return Err(KnowledgeOkfConceptStoreError::Internal(format!(
                "okf internal scan exceeds {MAX_OKF_INTERNAL_SCAN_ROWS} concepts for space {space_id}"
            )));
        }
        items.append(&mut page);
        if !has_more {
            return Ok(items);
        }
        let next_cursor = next_cursor.ok_or_else(|| {
            KnowledgeOkfConceptStoreError::Internal(
                "okf concept page reports has_more without next_cursor".to_string(),
            )
        })?;
        if cursor.as_deref() == Some(next_cursor.as_str()) {
            return Err(KnowledgeOkfConceptStoreError::Internal(
                "okf concept cursor did not advance".to_string(),
            ));
        }
        cursor = Some(next_cursor);
    }

    Err(KnowledgeOkfConceptStoreError::Internal(format!(
        "okf internal scan exceeded the maximum page count for space {space_id}"
    )))
}

#[async_trait]
pub trait KnowledgeOkfConceptStore: Send + Sync {
    async fn upsert_concept(
        &self,
        record: UpsertKnowledgeOkfConceptRecord,
    ) -> Result<KnowledgeOkfConcept, KnowledgeOkfConceptStoreError>;

    async fn create_revision(
        &self,
        record: CreateKnowledgeOkfConceptRevisionRecord,
    ) -> Result<KnowledgeOkfConceptRevision, KnowledgeOkfConceptStoreError>;

    async fn next_revision_no(
        &self,
        concept_row_id: u64,
    ) -> Result<u64, KnowledgeOkfConceptStoreError>;

    async fn mark_current_revision(
        &self,
        record: MarkKnowledgeOkfConceptCurrentRevisionRecord,
    ) -> Result<KnowledgeOkfConcept, KnowledgeOkfConceptStoreError>;

    async fn list_concept_summaries(
        &self,
        space_id: u64,
        page_size: Option<u32>,
    ) -> Result<Vec<OkfConceptSummary>, KnowledgeOkfConceptStoreError>;

    async fn list_concept_summaries_page(
        &self,
        space_id: u64,
        cursor: Option<String>,
        page_size: u32,
    ) -> Result<(Vec<OkfConceptSummary>, Option<String>, bool), KnowledgeOkfConceptStoreError>;

    async fn list_concept_revisions_page(
        &self,
        concept_row_id: u64,
        cursor: Option<u64>,
        page_size: u32,
    ) -> Result<(Vec<KnowledgeOkfConceptRevision>, Option<u64>, bool), KnowledgeOkfConceptStoreError>;

    async fn append_log_entry(
        &self,
        record: AppendKnowledgeOkfLogEntryRecord,
    ) -> Result<OkfLogEntry, KnowledgeOkfConceptStoreError>;

    async fn list_log_entries(
        &self,
        space_id: u64,
    ) -> Result<Vec<OkfLogEntry>, KnowledgeOkfConceptStoreError>;

    async fn batch_concept_projections_by_paths(
        &self,
        space_id: u64,
        logical_paths: Vec<String>,
    ) -> Result<Vec<KnowledgeOkfConceptProjection>, KnowledgeOkfConceptStoreError>;

    async fn mark_concept_deleted(
        &self,
        space_id: u64,
        concept_row_id: u64,
    ) -> Result<KnowledgeOkfConcept, KnowledgeOkfConceptStoreError>;
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UpsertKnowledgeOkfConceptRecord {
    pub space_id: u64,
    pub concept_id: String,
    pub title: String,
    pub concept_type: String,
    pub logical_path: String,
    pub description: String,
    pub source_count: u32,
    pub tags: Vec<String>,
    pub publish_state: OkfConceptPublishState,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CreateKnowledgeOkfConceptRevisionRecord {
    pub concept_row_id: u64,
    pub revision_no: u64,
    pub markdown_object_ref_id: u64,
    pub content_hash: String,
    pub review_state: OkfRevisionReviewState,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MarkKnowledgeOkfConceptCurrentRevisionRecord {
    pub concept_row_id: u64,
    pub revision_id: u64,
    pub publish_state: OkfConceptPublishState,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AppendKnowledgeOkfLogEntryRecord {
    pub space_id: u64,
    pub event_type: String,
    pub event_time: String,
    pub title: String,
    pub actor: String,
    pub affected_concepts: Vec<String>,
    pub audit_event_id: Option<String>,
    pub warnings: Vec<String>,
    pub privacy_level: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct KnowledgeOkfConceptProjection {
    pub logical_path: String,
    pub concept_row_id: u64,
    pub current_revision_id: Option<u64>,
    pub publish_state: OkfConceptPublishState,
}

#[derive(Debug, Error, Clone, PartialEq, Eq)]
pub enum KnowledgeOkfConceptStoreError {
    #[error("missing okf concept: {0}")]
    NotFound(String),
    #[error("okf concept store internal error: {0}")]
    Internal(String),
}
