use sdkwork_agent_kernel::{
    KernelError, KernelResult, KnowledgeDocumentKind, KnowledgeRetrievalMethod,
    KnowledgeSearchResult, RedactionClassification, TrustLevel,
};
use sdkwork_knowledgebase_contract::KnowledgeContextFragment;

pub(crate) fn parse_tenant_id(value: Option<&str>) -> Option<u64> {
    value.and_then(|value| value.parse::<u64>().ok())
}

pub(crate) fn parse_namespace_space_id(value: Option<&str>) -> KernelResult<u64> {
    let Some(value) = value else {
        return Err(KernelError::validation(
            "knowledgebase provider requires namespace to contain a knowledge space id",
        ));
    };

    let value = value.strip_prefix("space:").unwrap_or(value);
    value
        .parse::<u64>()
        .map_err(|_| KernelError::validation("knowledge namespace must be a numeric space id"))
}

pub(crate) fn format_scoped_document_id(space_id: u64, document_id: &str) -> String {
    format!("{space_id}/{document_id}")
}

pub(crate) fn parse_scoped_document_id(document_id: &str) -> Option<(u64, String)> {
    let (space_id, scoped_id) = document_id.split_once('/')?;
    Some((space_id.parse().ok()?, scoped_id.to_string()))
}

pub(crate) fn map_method(
    method: KnowledgeRetrievalMethod,
) -> sdkwork_knowledgebase_contract::KnowledgeRetrievalMethod {
    match method {
        KnowledgeRetrievalMethod::Exact => {
            sdkwork_knowledgebase_contract::KnowledgeRetrievalMethod::Exact
        }
        KnowledgeRetrievalMethod::Keyword => {
            sdkwork_knowledgebase_contract::KnowledgeRetrievalMethod::Keyword
        }
        KnowledgeRetrievalMethod::FullText => {
            sdkwork_knowledgebase_contract::KnowledgeRetrievalMethod::FullText
        }
        KnowledgeRetrievalMethod::Structured => {
            sdkwork_knowledgebase_contract::KnowledgeRetrievalMethod::Structured
        }
        KnowledgeRetrievalMethod::Graph => {
            sdkwork_knowledgebase_contract::KnowledgeRetrievalMethod::Graph
        }
        KnowledgeRetrievalMethod::Vector => {
            sdkwork_knowledgebase_contract::KnowledgeRetrievalMethod::Vector
        }
        KnowledgeRetrievalMethod::Hybrid => {
            sdkwork_knowledgebase_contract::KnowledgeRetrievalMethod::Hybrid
        }
        KnowledgeRetrievalMethod::LlmRerank => {
            sdkwork_knowledgebase_contract::KnowledgeRetrievalMethod::LlmRerank
        }
        KnowledgeRetrievalMethod::External => {
            sdkwork_knowledgebase_contract::KnowledgeRetrievalMethod::External
        }
    }
}

pub(crate) fn hit_to_search_result(hit: KnowledgeContextFragment) -> KnowledgeSearchResult {
    let document_ref_id = scoped_knowledge_document_ref(hit.space_id, hit.document_id);
    let mut result = KnowledgeSearchResult::new(
        document_ref_id,
        KnowledgeDocumentKind::Other,
        hit.title,
        map_contract_method(hit.retrieval_method),
    )
    .with_snippet(hit.content)
    .with_trust_level(TrustLevel::TrustedHost)
    .with_redaction_classification(RedactionClassification::TenantSensitive)
    .with_metadata("sdkwork.knowledge.chunk_id", hit.chunk_id.to_string())
    .with_metadata("sdkwork.knowledge.document_id", hit.document_id.to_string())
    .with_metadata("sdkwork.knowledge.space_id", hit.space_id.to_string())
    .with_metadata("sdkwork.knowledge.rank", hit.rank.to_string());

    if let Some(score) = hit.score {
        result = result.with_score(score);
    }

    if let Some(document_version_id) = hit.document_version_id {
        result = result.with_metadata(
            "sdkwork.knowledge.document_version_id",
            document_version_id.to_string(),
        );
    }

    if let Some(citation) = hit.citation {
        if let Some(source_uri) = citation.source_uri {
            result = result.with_source_uri(source_uri);
        }
        if let Some(locator) = citation.locator {
            result = result.with_metadata("sdkwork.knowledge.citation.locator", locator);
        }
    }

    result
}

pub(crate) fn scoped_knowledge_document_ref(space_id: u64, document_id: u64) -> String {
    format!("{space_id}/{document_id}")
}

fn map_contract_method(
    method: sdkwork_knowledgebase_contract::KnowledgeRetrievalMethod,
) -> KnowledgeRetrievalMethod {
    match method {
        sdkwork_knowledgebase_contract::KnowledgeRetrievalMethod::Exact => {
            KnowledgeRetrievalMethod::Exact
        }
        sdkwork_knowledgebase_contract::KnowledgeRetrievalMethod::Keyword => {
            KnowledgeRetrievalMethod::Keyword
        }
        sdkwork_knowledgebase_contract::KnowledgeRetrievalMethod::FullText => {
            KnowledgeRetrievalMethod::FullText
        }
        sdkwork_knowledgebase_contract::KnowledgeRetrievalMethod::Structured => {
            KnowledgeRetrievalMethod::Structured
        }
        sdkwork_knowledgebase_contract::KnowledgeRetrievalMethod::Graph => {
            KnowledgeRetrievalMethod::Graph
        }
        sdkwork_knowledgebase_contract::KnowledgeRetrievalMethod::Vector => {
            KnowledgeRetrievalMethod::Vector
        }
        sdkwork_knowledgebase_contract::KnowledgeRetrievalMethod::Hybrid => {
            KnowledgeRetrievalMethod::Hybrid
        }
        sdkwork_knowledgebase_contract::KnowledgeRetrievalMethod::LlmRerank => {
            KnowledgeRetrievalMethod::LlmRerank
        }
        sdkwork_knowledgebase_contract::KnowledgeRetrievalMethod::External => {
            KnowledgeRetrievalMethod::External
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use sdkwork_knowledgebase_contract::KnowledgeContextFragment;

    #[test]
    fn search_result_uses_scoped_document_ref_for_read_alignment() {
        let hit = KnowledgeContextFragment {
            chunk_id: 201,
            document_id: 301,
            document_version_id: None,
            space_id: 7,
            title: "Hybrid Retrieval".to_string(),
            content: "chunk body".to_string(),
            score: Some(0.88),
            rank: 1,
            token_count: Some(4),
            retrieval_method: sdkwork_knowledgebase_contract::KnowledgeRetrievalMethod::Hybrid,
            citation: None,
        };

        let result = hit_to_search_result(hit);
        assert_eq!(result.document_id, "7/301");
        assert_eq!(
            result
                .metadata
                .iter()
                .find(|(key, _)| key == "sdkwork.knowledge.chunk_id")
                .map(|(_, value)| value.as_str()),
            Some("201")
        );
    }

    #[test]
    fn scoped_document_ref_formats_space_and_document_ids() {
        assert_eq!(scoped_knowledge_document_ref(7, 301), "7/301");
    }
}
