use async_trait::async_trait;
use sdkwork_knowledgebase_contract::okf::{
    KnowledgeOkfConcept, KnowledgeOkfConceptRevision, OkfConceptPublishState, OkfConceptSummary,
    OkfLogEntry, OkfRevisionReviewState,
};
use thiserror::Error;

const OKF_INTERNAL_SCAN_PAGE_SIZE: u32 = 200;
const MAX_OKF_INTERNAL_SCAN_ROWS: usize = 50_000;

/// Hard upper bound for a single bounded search or single-concept lookup
/// result. Repositories with native filtering should clamp to this bound so a
/// caller-supplied limit can never materialize an unbounded row set.
pub const MAX_OKF_SEARCH_CONCEPT_ROWS: u32 = 500;

/// Page size used by the bounded fallback scans in the default trait methods.
/// It must stay within `MAX_OKF_LIST_ROWS` accepted by `list_concept_summaries_page`.
const OKF_BOUNDED_SCAN_PAGE_SIZE: u32 = 200;

/// Maximum number of published summaries a fallback search may read from the
/// store. Search is a ranking query, not an export: a store without native
/// filtering still never walks a whole space on the request path.
const MAX_OKF_SEARCH_SCAN_ROWS: usize = 2_000;

/// Maximum number of pages the fallback single-concept lookup may read before
/// reporting "not found". The PostgreSQL override resolves the same lookup with
/// a single `LIMIT 1` statement.
const MAX_OKF_LOOKUP_SCAN_PAGES: usize = 10;

fn summary_matches_tokens(summary: &OkfConceptSummary, tokens: &[String]) -> bool {
    if tokens.is_empty() {
        return true;
    }
    // One lowercased haystack per scanned row; tokens are already normalized by
    // the caller and are compared as substrings.
    let haystack = format!(
        "{} {} {} {}",
        summary.title,
        summary.concept_id,
        summary.description,
        summary.tags.join(" ")
    )
    .to_lowercase();
    tokens
        .iter()
        .any(|token| !token.is_empty() && haystack.contains(token.as_str()))
}

fn advance_scan_cursor(
    current: Option<&str>,
    next_cursor: Option<String>,
) -> Result<String, KnowledgeOkfConceptStoreError> {
    let next_cursor = next_cursor.ok_or_else(|| {
        KnowledgeOkfConceptStoreError::Internal(
            "okf concept page reports has_more without next_cursor".to_string(),
        )
    })?;
    if current == Some(next_cursor.as_str()) {
        return Err(KnowledgeOkfConceptStoreError::Internal(
            "okf concept cursor did not advance".to_string(),
        ));
    }
    Ok(next_cursor)
}

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

    /// Returns up to `limit` published summaries that match `tokens`, bounding
    /// both the result size and the number of rows read from the store.
    ///
    /// The default implementation pages through `list_concept_summaries_page`
    /// and filters in memory. It reads at most
    /// [`MAX_OKF_SEARCH_SCAN_ROWS`] rows and returns at most `limit` (clamped to
    /// [`MAX_OKF_SEARCH_CONCEPT_ROWS`]) summaries in store page order.
    /// Repositories that can rank natively (for example PostgreSQL full text)
    /// override this method so filtering, ranking, and `LIMIT` stay in SQL.
    async fn search_published_concept_summaries(
        &self,
        space_id: u64,
        tokens: &[String],
        limit: u32,
    ) -> Result<Vec<OkfConceptSummary>, KnowledgeOkfConceptStoreError> {
        let limit = limit.clamp(1, MAX_OKF_SEARCH_CONCEPT_ROWS) as usize;
        let mut matched = Vec::with_capacity(limit.min(OKF_BOUNDED_SCAN_PAGE_SIZE as usize));
        let mut cursor: Option<String> = None;
        let mut scanned = 0_usize;

        while matched.len() < limit && scanned < MAX_OKF_SEARCH_SCAN_ROWS {
            let (page, next_cursor, has_more) = self
                .list_concept_summaries_page(space_id, cursor.clone(), OKF_BOUNDED_SCAN_PAGE_SIZE)
                .await?;
            if page.is_empty() {
                break;
            }
            scanned = scanned.saturating_add(page.len());
            for summary in page {
                if summary_matches_tokens(&summary, tokens) {
                    matched.push(summary);
                    if matched.len() >= limit {
                        break;
                    }
                }
            }
            if !has_more || matched.len() >= limit {
                break;
            }
            cursor = Some(advance_scan_cursor(cursor.as_deref(), next_cursor)?);
        }

        Ok(matched)
    }

    /// Resolves one published summary by concept id or logical path.
    ///
    /// The default implementation scans bounded pages and stops at the first
    /// match, reading at most [`MAX_OKF_LOOKUP_SCAN_PAGES`] pages before
    /// returning `None`. The PostgreSQL override resolves the same lookup with a
    /// single `LIMIT 1` statement.
    async fn get_published_concept_summary(
        &self,
        space_id: u64,
        concept_id_or_logical_path: &str,
    ) -> Result<Option<OkfConceptSummary>, KnowledgeOkfConceptStoreError> {
        let needle = concept_id_or_logical_path.trim();
        if needle.is_empty() {
            return Ok(None);
        }

        let mut cursor: Option<String> = None;
        for _ in 0..MAX_OKF_LOOKUP_SCAN_PAGES {
            let (page, next_cursor, has_more) = self
                .list_concept_summaries_page(space_id, cursor.clone(), OKF_BOUNDED_SCAN_PAGE_SIZE)
                .await?;
            if let Some(found) = page
                .into_iter()
                .find(|summary| summary.concept_id == needle || summary.logical_path == needle)
            {
                return Ok(Some(found));
            }
            if !has_more {
                return Ok(None);
            }
            cursor = Some(advance_scan_cursor(cursor.as_deref(), next_cursor)?);
        }

        Ok(None)
    }

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

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicUsize, Ordering};

    #[derive(Default)]
    struct VecConceptStore {
        summaries: Vec<OkfConceptSummary>,
        page_calls: AtomicUsize,
        page_rows_served: AtomicUsize,
    }

    impl VecConceptStore {
        fn new(summaries: Vec<OkfConceptSummary>) -> Self {
            Self {
                summaries,
                ..Default::default()
            }
        }

        fn page_calls(&self) -> usize {
            self.page_calls.load(Ordering::Relaxed)
        }

        fn page_rows_served(&self) -> usize {
            self.page_rows_served.load(Ordering::Relaxed)
        }
    }

    fn summary(index: usize, title: &str) -> OkfConceptSummary {
        OkfConceptSummary {
            title: title.to_string(),
            concept_id: format!("concepts/doc-{index:05}"),
            concept_type: "concept".to_string(),
            logical_path: format!("okf/concepts/doc-{index:05}.md"),
            bundle_relative_path: format!("concepts/doc-{index:05}.md"),
            description: "fixture".to_string(),
            source_count: 1,
            updated_at: "2026-01-01T00:00:00Z".to_string(),
            tags: Vec::new(),
        }
    }

    fn unsupported() -> KnowledgeOkfConceptStoreError {
        KnowledgeOkfConceptStoreError::Internal("unsupported in test store".to_string())
    }

    #[async_trait]
    impl KnowledgeOkfConceptStore for VecConceptStore {
        async fn upsert_concept(
            &self,
            _record: UpsertKnowledgeOkfConceptRecord,
        ) -> Result<KnowledgeOkfConcept, KnowledgeOkfConceptStoreError> {
            Err(unsupported())
        }

        async fn create_revision(
            &self,
            _record: CreateKnowledgeOkfConceptRevisionRecord,
        ) -> Result<KnowledgeOkfConceptRevision, KnowledgeOkfConceptStoreError> {
            Err(unsupported())
        }

        async fn next_revision_no(
            &self,
            _concept_row_id: u64,
        ) -> Result<u64, KnowledgeOkfConceptStoreError> {
            Err(unsupported())
        }

        async fn mark_current_revision(
            &self,
            _record: MarkKnowledgeOkfConceptCurrentRevisionRecord,
        ) -> Result<KnowledgeOkfConcept, KnowledgeOkfConceptStoreError> {
            Err(unsupported())
        }

        async fn list_concept_summaries(
            &self,
            _space_id: u64,
            _page_size: Option<u32>,
        ) -> Result<Vec<OkfConceptSummary>, KnowledgeOkfConceptStoreError> {
            Err(unsupported())
        }

        async fn list_concept_revisions_page(
            &self,
            _concept_row_id: u64,
            _cursor: Option<u64>,
            _page_size: u32,
        ) -> Result<
            (Vec<KnowledgeOkfConceptRevision>, Option<u64>, bool),
            KnowledgeOkfConceptStoreError,
        > {
            Err(unsupported())
        }

        async fn append_log_entry(
            &self,
            _record: AppendKnowledgeOkfLogEntryRecord,
        ) -> Result<OkfLogEntry, KnowledgeOkfConceptStoreError> {
            Err(unsupported())
        }

        async fn list_log_entries(
            &self,
            _space_id: u64,
        ) -> Result<Vec<OkfLogEntry>, KnowledgeOkfConceptStoreError> {
            Err(unsupported())
        }

        async fn batch_concept_projections_by_paths(
            &self,
            _space_id: u64,
            _logical_paths: Vec<String>,
        ) -> Result<Vec<KnowledgeOkfConceptProjection>, KnowledgeOkfConceptStoreError> {
            Err(unsupported())
        }

        async fn mark_concept_deleted(
            &self,
            _space_id: u64,
            _concept_row_id: u64,
        ) -> Result<KnowledgeOkfConcept, KnowledgeOkfConceptStoreError> {
            Err(unsupported())
        }
        async fn list_concept_summaries_page(
            &self,
            _space_id: u64,
            cursor: Option<String>,
            page_size: u32,
        ) -> Result<(Vec<OkfConceptSummary>, Option<String>, bool), KnowledgeOkfConceptStoreError>
        {
            self.page_calls.fetch_add(1, Ordering::Relaxed);
            let page_size = (page_size.clamp(1, OKF_BOUNDED_SCAN_PAGE_SIZE)) as usize;
            let start = match cursor {
                Some(cursor) => self
                    .summaries
                    .partition_point(|summary| summary.concept_id <= cursor),
                None => 0,
            };
            let end = start.saturating_add(page_size).min(self.summaries.len());
            let page = self.summaries[start..end].to_vec();
            self.page_rows_served
                .fetch_add(page.len(), Ordering::Relaxed);
            let has_more = end < self.summaries.len();
            let next_cursor = if has_more {
                page.last().map(|summary| summary.concept_id.clone())
            } else {
                None
            };
            Ok((page, next_cursor, has_more))
        }
    }

    #[tokio::test]
    async fn default_search_returns_at_most_limit_and_filters() {
        let summaries = (0..500)
            .map(|index| {
                if index < 300 {
                    summary(index, "needle concept")
                } else {
                    summary(index, "filler concept")
                }
            })
            .collect();
        let store = VecConceptStore::new(summaries);

        let matches = store
            .search_published_concept_summaries(7, &["needle".to_string()], 25)
            .await
            .expect("search");

        assert_eq!(matches.len(), 25);
        assert!(matches
            .iter()
            .all(|summary| summary.title.contains("needle")));
        assert!(store.page_rows_served() <= OKF_BOUNDED_SCAN_PAGE_SIZE as usize);
    }

    #[tokio::test]
    async fn default_search_stops_at_scan_row_cap() {
        let summaries = (0..10_000)
            .map(|index| summary(index, "filler concept"))
            .collect();
        let store = VecConceptStore::new(summaries);

        let matches = store
            .search_published_concept_summaries(7, &["needle".to_string()], 100)
            .await
            .expect("search");

        assert!(matches.is_empty());
        assert_eq!(store.page_rows_served(), MAX_OKF_SEARCH_SCAN_ROWS);
        assert!(
            store.page_rows_served() < 10_000,
            "fallback search must not materialize the whole space"
        );
        assert_eq!(
            store.page_calls(),
            MAX_OKF_SEARCH_SCAN_ROWS / OKF_BOUNDED_SCAN_PAGE_SIZE as usize
        );
    }

    #[tokio::test]
    async fn default_search_clamps_oversized_limit() {
        let summaries = (0..10_000)
            .map(|index| summary(index, "needle concept"))
            .collect();
        let store = VecConceptStore::new(summaries);

        let matches = store
            .search_published_concept_summaries(7, &["needle".to_string()], u32::MAX)
            .await
            .expect("search");

        assert_eq!(matches.len(), MAX_OKF_SEARCH_CONCEPT_ROWS as usize);
        assert!(store.page_rows_served() <= MAX_OKF_SEARCH_SCAN_ROWS);
    }

    #[tokio::test]
    async fn default_lookup_stops_at_first_match() {
        let summaries = (0..500)
            .map(|index| summary(index, "filler concept"))
            .collect();
        let store = VecConceptStore::new(summaries);

        let concept_id = store
            .get_published_concept_summary(7, "concepts/doc-00003")
            .await
            .expect("lookup")
            .expect("matched concept");
        assert_eq!(concept_id.concept_id, "concepts/doc-00003");
        assert_eq!(store.page_calls(), 1);
        assert_eq!(
            store.page_rows_served(),
            OKF_BOUNDED_SCAN_PAGE_SIZE as usize
        );

        let logical_path = store
            .get_published_concept_summary(7, "okf/concepts/doc-00003.md")
            .await
            .expect("lookup")
            .expect("matched concept");
        assert_eq!(logical_path.concept_id, "concepts/doc-00003");
    }

    #[tokio::test]
    async fn default_lookup_is_page_capped() {
        let summaries = (0..10_000)
            .map(|index| summary(index, "filler concept"))
            .collect();
        let store = VecConceptStore::new(summaries);

        let missing = store
            .get_published_concept_summary(7, "concepts/missing")
            .await
            .expect("lookup");

        assert!(missing.is_none());
        assert_eq!(store.page_calls(), MAX_OKF_LOOKUP_SCAN_PAGES);
        assert_eq!(
            store.page_rows_served(),
            MAX_OKF_LOOKUP_SCAN_PAGES * OKF_BOUNDED_SCAN_PAGE_SIZE as usize
        );
    }
}
