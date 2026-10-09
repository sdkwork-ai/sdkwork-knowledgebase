use crate::okf_ranking::{normalize_query, rank_okf_concept};
use sdkwork_agent_kernel::{
    KernelError, KernelResult, KnowledgeDocument, KnowledgeDocumentFilter, KnowledgeDocumentKind,
    KnowledgeProvider, KnowledgeRetrievalMethod, KnowledgeSearchRequest, KnowledgeSearchResult,
    ProviderHealth, ProviderManifest, RedactionClassification, TrustLevel,
};
use sdkwork_knowledgebase_contract::{
    knowledge_engine::KnowledgeEngineSearchHit,
    okf::{okf_document_id, OkfBundlePaths},
    OkfConceptSummary, OKF_KNOWLEDGE_PROVIDER_ID,
};
use sdkwork_utils_rust::is_blank;

/// Bounded result ceiling for kernel-driven OKF concept listings: upstream
/// engines receive a u32 top_k, so an unbounded usize would request an
/// impossible result set and overload the upstream.
const OKF_LIST_TOP_K: usize = 32;

pub trait OkfKnowledgeClient {
    fn search_okf_concepts(
        &self,
        space_id: u64,
        query: &str,
        top_k: usize,
    ) -> Result<Vec<OkfConceptSummary>, String>;

    fn read_okf_concept_content(&self, space_id: u64, logical_path: &str)
        -> Result<String, String>;
}

pub struct OkfKnowledgeProvider<C> {
    client: C,
}

impl<C> OkfKnowledgeProvider<C> {
    pub fn new(client: C) -> Self {
        Self { client }
    }
}

impl<C> KnowledgeProvider for OkfKnowledgeProvider<C>
where
    C: OkfKnowledgeClient,
{
    fn provider_manifest(&self) -> ProviderManifest {
        ProviderManifest::new(
            OKF_KNOWLEDGE_PROVIDER_ID,
            "knowledge",
            "okf-bundle",
            env!("CARGO_PKG_VERSION"),
            vec![
                "knowledge.search".to_string(),
                "knowledge.read".to_string(),
                "knowledge.list".to_string(),
            ],
        )
    }

    fn search(&self, request: KnowledgeSearchRequest) -> KernelResult<Vec<KnowledgeSearchResult>> {
        if is_blank(Some(request.query.as_str())) {
            return Err(KernelError::validation(
                "okf-bundle knowledge search query must not be blank",
            ));
        }

        let space_id = parse_namespace_space_id(request.namespace.as_deref())?;
        let top_k = request.top_k.max(1);
        let pages = self
            .client
            .search_okf_concepts(space_id, &request.query, top_k)
            .map_err(|message| {
                KernelError::provider_error("okf_bundle.search_failed", message)
                    .with_provider(OKF_KNOWLEDGE_PROVIDER_ID)
            })?;

        Ok(pages
            .into_iter()
            .enumerate()
            .map(|(index, page)| okf_concept_to_search_result(space_id, index, page))
            .collect())
    }

    fn read(&self, document_id: &str) -> KernelResult<KnowledgeDocument> {
        let (space_id, concept_id) = parse_okf_document_id(document_id)?;
        let logical_path = OkfBundlePaths::concept_logical_path(&concept_id);
        let content = self
            .client
            .read_okf_concept_content(space_id, &logical_path)
            .map_err(|message| {
                KernelError::provider_error("okf_bundle.read_failed", message)
                    .with_provider(OKF_KNOWLEDGE_PROVIDER_ID)
            })?;

        Ok(KnowledgeDocument::new(
            document_id,
            KnowledgeDocumentKind::Spec,
            concept_id.clone(),
            content,
        )
        .with_namespace(format!("space:{space_id}"))
        .with_metadata("sdkwork.knowledge.logical_path", logical_path))
    }

    fn list(&self, filter: KnowledgeDocumentFilter) -> KernelResult<Vec<KnowledgeDocument>> {
        let space_id = parse_namespace_space_id(filter.namespace.as_deref())?;
        // A kernel-driven listing must never ask the upstream engine for an
        // unbounded result set (`usize::MAX` truncated into the engine's
        // u32 top_k would request ~4.29 billion hits); use the same bounded
        // ceiling as the runtime's default_top_k.
        let pages = self
            .client
            .search_okf_concepts(space_id, "", OKF_LIST_TOP_K)
            .map_err(|message| {
                KernelError::provider_error("okf_bundle.list_failed", message)
                    .with_provider(OKF_KNOWLEDGE_PROVIDER_ID)
            })?;

        Ok(pages
            .into_iter()
            .map(|page| {
                KnowledgeDocument::new(
                    okf_document_id(space_id, &page.concept_id),
                    KnowledgeDocumentKind::Spec,
                    page.title.clone(),
                    page.description.clone(),
                )
                .with_namespace(format!("space:{space_id}"))
                .with_metadata("sdkwork.knowledge.logical_path", page.logical_path.clone())
                .with_metadata("sdkwork.knowledge.concept_id", page.concept_id.clone())
            })
            .collect())
    }

    fn health(&self) -> ProviderHealth {
        ProviderHealth::available()
    }
}

fn okf_concept_to_search_result(
    space_id: u64,
    index: usize,
    page: OkfConceptSummary,
) -> KnowledgeSearchResult {
    let score = score_okf_concept(&page);
    let title = page.title.clone();
    KnowledgeSearchResult::new(
        okf_document_id(space_id, &page.concept_id),
        KnowledgeDocumentKind::Spec,
        title.clone(),
        KnowledgeRetrievalMethod::Keyword,
    )
    .with_snippet(page.description)
    .with_score(score)
    .with_source_uri(page.logical_path.clone())
    .with_trust_level(TrustLevel::TrustedHost)
    .with_redaction_classification(RedactionClassification::TenantSensitive)
    .with_metadata("sdkwork.knowledge.space_id", space_id.to_string())
    .with_metadata("sdkwork.knowledge.logical_path", page.logical_path)
    .with_metadata("sdkwork.knowledge.concept_id", page.concept_id)
    .with_metadata("sdkwork.knowledge.rank", (index + 1).to_string())
    .with_metadata("sdkwork.knowledge.title", title)
}

fn score_okf_concept(page: &OkfConceptSummary) -> f64 {
    let tag_bonus = page.tags.len() as f64 * 0.01;
    let source_bonus = page.source_count as f64 * 0.02;
    0.5 + tag_bonus + source_bonus
}

fn parse_okf_document_id(document_id: &str) -> KernelResult<(u64, String)> {
    let rest = document_id
        .strip_prefix("okf:")
        .ok_or_else(|| KernelError::validation("okf-bundle document id must start with okf:"))?;
    let (space_id, concept_id) = rest
        .split_once(':')
        .ok_or_else(|| KernelError::validation("okf-bundle document id must include space id"))?;
    let space_id = space_id
        .parse::<u64>()
        .map_err(|_| KernelError::validation("okf-bundle document id space id must be numeric"))?;
    if is_blank(Some(concept_id)) {
        return Err(KernelError::validation(
            "okf-bundle document id concept id must not be blank",
        ));
    }
    Ok((space_id, concept_id.to_string()))
}

fn parse_namespace_space_id(namespace: Option<&str>) -> KernelResult<u64> {
    let namespace = namespace
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .ok_or_else(|| {
            KernelError::validation("okf-bundle knowledge namespace must include space id")
        })?;
    let space_id = namespace
        .strip_prefix("space:")
        .unwrap_or(namespace)
        .parse::<u64>()
        .map_err(|_| {
            KernelError::validation("okf-bundle knowledge namespace space id must be numeric")
        })?;
    if space_id == 0 {
        return Err(KernelError::validation(
            "okf-bundle knowledge namespace space id must be positive",
        ));
    }
    Ok(space_id)
}

pub fn citations_from_rag_hits(
    hits: &[sdkwork_knowledgebase_contract::KnowledgeContextFragment],
) -> Vec<sdkwork_knowledgebase_contract::KnowledgeAgentChatCitation> {
    hits.iter()
        .map(
            |hit| sdkwork_knowledgebase_contract::KnowledgeAgentChatCitation {
                document_id: Some(hit.document_id),
                concept_id: None,
                title: hit.title.clone(),
                source_uri: hit
                    .citation
                    .as_ref()
                    .and_then(|citation| citation.source_uri.clone()),
                logical_path: None,
                locator: hit
                    .citation
                    .as_ref()
                    .and_then(|citation| citation.locator.clone()),
                score: hit.score,
                snippet: Some(hit.content.clone()),
            },
        )
        .collect()
}

pub fn citations_from_okf_concepts(
    space_id: u64,
    concepts: &[OkfConceptSummary],
) -> Vec<sdkwork_knowledgebase_contract::KnowledgeAgentChatCitation> {
    citations_from_okf_concepts_with_query(space_id, "", concepts)
}

pub fn citations_from_okf_concepts_with_query(
    space_id: u64,
    query: &str,
    concepts: &[OkfConceptSummary],
) -> Vec<sdkwork_knowledgebase_contract::KnowledgeAgentChatCitation> {
    let tokens = normalize_query(query);
    concepts
        .iter()
        .map(|concept| {
            let score = if is_blank(Some(query)) {
                score_okf_concept(concept)
            } else {
                rank_okf_concept(concept, &tokens)
            };
            sdkwork_knowledgebase_contract::KnowledgeAgentChatCitation {
                document_id: None,
                concept_id: Some(concept.concept_id.clone()),
                title: concept.title.clone(),
                source_uri: Some(concept.logical_path.clone()),
                logical_path: Some(format!("{space_id}/{}", concept.concept_id)),
                locator: Some(okf_document_id(space_id, &concept.concept_id)),
                score: Some(score),
                snippet: Some(concept.description.clone()),
            }
        })
        .collect()
}

pub fn citations_from_engine_hits(
    space_id: u64,
    hits: &[KnowledgeEngineSearchHit],
) -> Vec<sdkwork_knowledgebase_contract::KnowledgeAgentChatCitation> {
    hits.iter()
        .map(|hit| {
            let scoped_ref = if hit.document.document_id.contains('/') {
                hit.document.document_id.clone()
            } else {
                format!("{space_id}/{}", hit.document.document_id)
            };
            let document_key = hit
                .document
                .document_id
                .rsplit('/')
                .next()
                .unwrap_or(hit.document.document_id.as_str());
            let numeric_document_id = document_key.parse::<u64>().ok();
            sdkwork_knowledgebase_contract::KnowledgeAgentChatCitation {
                document_id: numeric_document_id,
                concept_id: if numeric_document_id.is_some() {
                    None
                } else {
                    Some(document_key.to_string())
                },
                title: hit.document.title.clone(),
                source_uri: hit.document.source_uri.clone(),
                logical_path: Some(scoped_ref),
                locator: Some(okf_document_id(space_id, document_key)),
                score: hit.score,
                snippet: Some(hit.snippet.clone()),
            }
        })
        .collect()
}
