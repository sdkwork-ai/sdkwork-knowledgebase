use async_trait::async_trait;
use std::collections::HashSet;
use thiserror::Error;

/// Hard safety bound for in-service aggregation sweeps over the OKF link
/// graph. Large spaces page through the store; the aggregate is bounded so a
/// pathological space cannot exhaust process memory.
pub const MAX_OKF_LINK_SCAN_ROWS: usize = 50_000;

/// Per-page OKF link-edge fetch limit used by aggregation sweeps.
pub const OKF_LINK_SCAN_PAGE_SIZE: u32 = 1_000;

/// Hard upper bound for a bounded incident-edge expansion. Link expansion is a
/// ranking hint on the request path, not a graph export.
pub const MAX_OKF_INCIDENT_LINK_EDGES: u32 = 2_000;

/// Rows the in-memory fallback expansion may scan before it stops. The
/// PostgreSQL override pushes the incident-id predicate and `LIMIT` into SQL.
const MAX_OKF_INCIDENT_EDGE_SCAN_ROWS: usize = 10_000;

#[async_trait]
pub trait KnowledgeOkfConceptLinkStore: Send + Sync {
    async fn replace_outbound_links(
        &self,
        record: ReplaceKnowledgeOkfConceptLinksRecord,
    ) -> Result<(), KnowledgeOkfConceptLinkStoreError>;

    async fn list_inbound_concept_ids(
        &self,
        space_id: u64,
        to_concept_id: &str,
    ) -> Result<Vec<String>, KnowledgeOkfConceptLinkStoreError>;

    /// Paged distinct inbound link targets for a space (keyset on
    /// `to_concept_id`). Aggregation sweeps page through this instead of
    /// loading an unbounded target set.
    async fn list_inbound_link_targets_page(
        &self,
        space_id: u64,
        after_concept_id: Option<&str>,
        limit: u32,
    ) -> Result<InboundLinkTargetsPage, KnowledgeOkfConceptLinkStoreError>;

    /// Paged active link edges for a space (keyset on
    /// `(from_concept_id, to_concept_id, anchor_text)`). Aggregation sweeps
    /// page through this instead of silently truncating at a fixed row cap.
    async fn list_active_link_edges_page(
        &self,
        space_id: u64,
        after: Option<LinkEdgeCursor>,
        limit: u32,
    ) -> Result<LinkEdgePage, KnowledgeOkfConceptLinkStoreError>;

    /// Returns up to `limit` active link edges that touch any of `concept_ids`,
    /// bounding both the result size and the number of rows read.
    ///
    /// The default implementation pages through
    /// [`KnowledgeOkfConceptLinkStore::list_active_link_edges_page`] and filters
    /// in memory: at most [`MAX_OKF_INCIDENT_EDGE_SCAN_ROWS`] edges are read and
    /// at most `limit` (clamped to [`MAX_OKF_INCIDENT_LINK_EDGES`]) edges are
    /// returned. The PostgreSQL override pushes the incident-id predicate and
    /// `LIMIT` into SQL.
    async fn list_active_link_edges_incident_to(
        &self,
        space_id: u64,
        concept_ids: &[String],
        limit: u32,
    ) -> Result<Vec<KnowledgeOkfConceptLinkEdge>, KnowledgeOkfConceptLinkStoreError> {
        let limit = limit.clamp(1, MAX_OKF_INCIDENT_LINK_EDGES) as usize;
        let wanted = concept_ids
            .iter()
            .filter(|concept_id| !concept_id.is_empty())
            .map(String::as_str)
            .collect::<HashSet<_>>();
        if wanted.is_empty() {
            return Ok(Vec::new());
        }

        let mut edges = Vec::new();
        let mut cursor: Option<LinkEdgeCursor> = None;
        let mut scanned = 0_usize;

        while edges.len() < limit && scanned < MAX_OKF_INCIDENT_EDGE_SCAN_ROWS {
            let page = self
                .list_active_link_edges_page(space_id, cursor.clone(), OKF_LINK_SCAN_PAGE_SIZE)
                .await?;
            if page.edges.is_empty() {
                break;
            }
            scanned = scanned.saturating_add(page.edges.len());
            for edge in page.edges {
                if wanted.contains(edge.from_concept_id.as_str())
                    || wanted.contains(edge.to_concept_id.as_str())
                {
                    edges.push(edge);
                    if edges.len() >= limit {
                        break;
                    }
                }
            }
            if !page.has_more || edges.len() >= limit {
                break;
            }
            cursor = Some(advance_edge_cursor(cursor.as_ref(), page.next_cursor)?);
        }

        Ok(edges)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct KnowledgeOkfConceptLinkEdge {
    pub from_concept_id: String,
    pub to_concept_id: String,
    pub anchor_text: String,
}

/// Keyset continuation for [`KnowledgeOkfConceptLinkStore::list_active_link_edges_page`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LinkEdgeCursor {
    pub from_concept_id: String,
    pub to_concept_id: String,
    pub anchor_text: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LinkEdgePage {
    pub edges: Vec<KnowledgeOkfConceptLinkEdge>,
    pub next_cursor: Option<LinkEdgeCursor>,
    pub has_more: bool,
}

/// One page of distinct inbound link targets.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct InboundLinkTargetsPage {
    pub targets: Vec<String>,
    pub next_cursor: Option<String>,
    pub has_more: bool,
}

/// Aggregates the distinct inbound link-target set for a space by paging
/// through the store. Bounded by [`MAX_OKF_LINK_SCAN_ROWS`].
pub async fn list_all_inbound_link_targets(
    store: &dyn KnowledgeOkfConceptLinkStore,
    space_id: u64,
) -> Result<Vec<String>, KnowledgeOkfConceptLinkStoreError> {
    let mut targets = Vec::new();
    let mut cursor = None;
    loop {
        let page = store
            .list_inbound_link_targets_page(space_id, cursor.as_deref(), OKF_LINK_SCAN_PAGE_SIZE)
            .await?;
        if targets.len().saturating_add(page.targets.len()) > MAX_OKF_LINK_SCAN_ROWS {
            return Err(KnowledgeOkfConceptLinkStoreError::Internal(format!(
                "OKF inbound link targets exceed the {MAX_OKF_LINK_SCAN_ROWS} row safety bound for space {space_id}"
            )));
        }
        targets.extend(page.targets);
        if !page.has_more {
            break;
        }
        cursor = page.next_cursor;
    }
    Ok(targets)
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ReplaceKnowledgeOkfConceptLinksRecord {
    pub space_id: u64,
    pub from_concept_id: String,
    pub links: Vec<KnowledgeOkfConceptLinkRecord>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct KnowledgeOkfConceptLinkRecord {
    pub to_concept_id: String,
    pub anchor_text: String,
}

#[derive(Debug, Error)]
pub enum KnowledgeOkfConceptLinkStoreError {
    #[error("internal knowledge okf concept link store error: {0}")]
    Internal(String),
}

fn advance_edge_cursor(
    current: Option<&LinkEdgeCursor>,
    next_cursor: Option<LinkEdgeCursor>,
) -> Result<LinkEdgeCursor, KnowledgeOkfConceptLinkStoreError> {
    let next_cursor = next_cursor.ok_or_else(|| {
        KnowledgeOkfConceptLinkStoreError::Internal(
            "okf link edge page reports has_more without next_cursor".to_string(),
        )
    })?;
    if current == Some(&next_cursor) {
        return Err(KnowledgeOkfConceptLinkStoreError::Internal(
            "okf link edge cursor did not advance".to_string(),
        ));
    }
    Ok(next_cursor)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicUsize, Ordering};

    #[derive(Default)]
    struct VecLinkStore {
        edges: Vec<KnowledgeOkfConceptLinkEdge>,
        page_calls: AtomicUsize,
        page_rows_served: AtomicUsize,
    }

    impl VecLinkStore {
        fn new(edges: Vec<KnowledgeOkfConceptLinkEdge>) -> Self {
            Self {
                edges,
                ..Default::default()
            }
        }

        fn page_rows_served(&self) -> usize {
            self.page_rows_served.load(Ordering::Relaxed)
        }

        fn page_calls(&self) -> usize {
            self.page_calls.load(Ordering::Relaxed)
        }
    }

    fn edge(from_concept_id: &str, to_concept_id: &str) -> KnowledgeOkfConceptLinkEdge {
        KnowledgeOkfConceptLinkEdge {
            from_concept_id: from_concept_id.to_string(),
            to_concept_id: to_concept_id.to_string(),
            anchor_text: String::new(),
        }
    }

    fn unsupported() -> KnowledgeOkfConceptLinkStoreError {
        KnowledgeOkfConceptLinkStoreError::Internal("unsupported in test store".to_string())
    }

    #[async_trait]
    impl KnowledgeOkfConceptLinkStore for VecLinkStore {
        async fn replace_outbound_links(
            &self,
            _record: ReplaceKnowledgeOkfConceptLinksRecord,
        ) -> Result<(), KnowledgeOkfConceptLinkStoreError> {
            Err(unsupported())
        }

        async fn list_inbound_concept_ids(
            &self,
            _space_id: u64,
            _to_concept_id: &str,
        ) -> Result<Vec<String>, KnowledgeOkfConceptLinkStoreError> {
            Err(unsupported())
        }

        async fn list_inbound_link_targets_page(
            &self,
            _space_id: u64,
            _after_concept_id: Option<&str>,
            _limit: u32,
        ) -> Result<InboundLinkTargetsPage, KnowledgeOkfConceptLinkStoreError> {
            Err(unsupported())
        }
        async fn list_active_link_edges_page(
            &self,
            _space_id: u64,
            after: Option<LinkEdgeCursor>,
            limit: u32,
        ) -> Result<LinkEdgePage, KnowledgeOkfConceptLinkStoreError> {
            self.page_calls.fetch_add(1, Ordering::Relaxed);
            let limit = (limit.clamp(1, OKF_LINK_SCAN_PAGE_SIZE)) as usize;
            let start = match &after {
                Some(cursor) => self.edges.partition_point(|edge| {
                    (
                        edge.from_concept_id.as_str(),
                        edge.to_concept_id.as_str(),
                        edge.anchor_text.as_str(),
                    ) <= (
                        cursor.from_concept_id.as_str(),
                        cursor.to_concept_id.as_str(),
                        cursor.anchor_text.as_str(),
                    )
                }),
                None => 0,
            };
            let end = start.saturating_add(limit).min(self.edges.len());
            let edges = self.edges[start..end].to_vec();
            self.page_rows_served
                .fetch_add(edges.len(), Ordering::Relaxed);
            let has_more = end < self.edges.len();
            let next_cursor = if has_more {
                edges.last().map(|edge| LinkEdgeCursor {
                    from_concept_id: edge.from_concept_id.clone(),
                    to_concept_id: edge.to_concept_id.clone(),
                    anchor_text: edge.anchor_text.clone(),
                })
            } else {
                None
            };
            Ok(LinkEdgePage {
                edges,
                next_cursor,
                has_more,
            })
        }
    }

    #[tokio::test]
    async fn default_incident_edges_filters_and_respects_limit() {
        let edges = (0..1_000)
            .map(|index| {
                edge(
                    &format!("concepts/node-{index:05}"),
                    &format!("concepts/node-{:05}", index + 1),
                )
            })
            .collect();
        let store = VecLinkStore::new(edges);

        let incident = store
            .list_active_link_edges_incident_to(7, &["concepts/node-00005".to_string()], 10)
            .await
            .expect("incident edges");

        assert_eq!(incident.len(), 2);
        assert!(incident.iter().all(|edge| {
            edge.from_concept_id == "concepts/node-00005"
                || edge.to_concept_id == "concepts/node-00005"
        }));
        assert!(store.page_rows_served() <= OKF_LINK_SCAN_PAGE_SIZE as usize);
    }

    #[tokio::test]
    async fn default_incident_edges_is_scan_capped() {
        let mut edges = (0..20_000)
            .map(|index| {
                edge(
                    &format!("concepts/node-{index:05}"),
                    &format!("concepts/node-{:05}", index + 1),
                )
            })
            .collect::<Vec<_>>();
        // Sorted after every filler edge, so it is only reachable by scanning
        // past the fallback row cap.
        edges.push(edge("concepts/zzz-needle", "concepts/node-00000"));
        let store = VecLinkStore::new(edges);

        let incident = store
            .list_active_link_edges_incident_to(7, &["concepts/zzz-needle".to_string()], 10)
            .await
            .expect("incident edges");

        assert!(incident.is_empty());
        assert_eq!(store.page_rows_served(), MAX_OKF_INCIDENT_EDGE_SCAN_ROWS);
        assert!(store.page_rows_served() < 20_000);
        assert_eq!(
            store.page_calls(),
            MAX_OKF_INCIDENT_EDGE_SCAN_ROWS / OKF_LINK_SCAN_PAGE_SIZE as usize
        );
    }

    #[tokio::test]
    async fn default_incident_edges_short_circuits_without_ids() {
        let store = VecLinkStore::new(vec![edge("concepts/a", "concepts/b")]);

        let incident = store
            .list_active_link_edges_incident_to(7, &[], 10)
            .await
            .expect("incident edges");

        assert!(incident.is_empty());
        assert_eq!(store.page_calls(), 0);
    }
}
