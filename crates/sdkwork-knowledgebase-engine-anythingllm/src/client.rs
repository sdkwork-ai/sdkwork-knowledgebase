//! AnythingLLM workspace vector-search HTTP client (adapter-local).

use reqwest::Method;
use sdkwork_knowledgebase_contract::knowledge_engine::{
    KnowledgeEngineDocument, KnowledgeEngineDocumentRef, KnowledgeEngineError,
    KnowledgeEngineSearchHit, KnowledgeEngineSearchResult,
};
use sdkwork_knowledgebase_provider_runtime::{
    encoded_path_segment, is_path_segment_id, optional_bearer_token, ProviderExecutionContext,
    ProviderHttpRequest, ProviderOperation, ProviderRuntime,
};
use serde::Deserialize;

use crate::config::AnythingLlmConnectorConfig;
use crate::ANYTHINGLLM_IMPLEMENTATION_ID;

#[derive(Clone)]
pub struct AnythingLlmApiClient {
    config: AnythingLlmConnectorConfig,
    http: ProviderRuntime,
}

impl AnythingLlmApiClient {
    pub fn new(config: AnythingLlmConnectorConfig) -> Result<Self, KnowledgeEngineError> {
        let http = ProviderRuntime::for_base_url_with_private_targets(
            &config.base_url,
            config.allow_private_network,
        )
        .map_err(KnowledgeEngineError::from)?;
        Ok(Self { config, http })
    }

    fn bearer_token(&self) -> Option<&str> {
        optional_bearer_token(Some(self.config.api_key.as_str()))
    }

    fn health_context(&self) -> ProviderExecutionContext {
        ProviderExecutionContext::for_system_health(ANYTHINGLLM_IMPLEMENTATION_ID)
    }

    /// Binding-controlled workspace slugs are spliced into URL paths, so they are
    /// validated and percent-encoded at every interpolation point.
    fn validated_workspace_slug<'a>(
        &self,
        workspace_slug: &'a str,
    ) -> Result<&'a str, KnowledgeEngineError> {
        if is_path_segment_id(workspace_slug) {
            Ok(workspace_slug)
        } else {
            Err(KnowledgeEngineError::Validation(
                "AnythingLLM workspace slug must match [A-Za-z0-9._:-]{1,256}".to_string(),
            ))
        }
    }

    pub async fn connector_health(&self, workspace_slug: &str) -> Result<(), KnowledgeEngineError> {
        let workspace_slug = self.validated_workspace_slug(workspace_slug)?;
        let url = format!(
            "{}/api/v1/workspace/{}",
            self.config.base_url.trim_end_matches('/'),
            encoded_path_segment(workspace_slug),
        );
        let request = ProviderHttpRequest::new(ProviderOperation::Health, Method::GET, url)
            .map_err(KnowledgeEngineError::from)?
            .optional_bearer_auth(self.bearer_token())
            .map_err(KnowledgeEngineError::from)?
            .idempotent(true);
        self.http
            .execute(&self.health_context(), request)
            .await
            .map_err(KnowledgeEngineError::from)?;
        Ok(())
    }

    pub async fn vector_search(
        &self,
        context: &ProviderExecutionContext,
        space_id: u64,
        workspace_slug: &str,
        query: &str,
        top_k: u32,
    ) -> Result<KnowledgeEngineSearchResult, KnowledgeEngineError> {
        let workspace_slug = self.validated_workspace_slug(workspace_slug)?;
        let url = format!(
            "{}/api/v1/workspace/{}/vector-search",
            self.config.base_url.trim_end_matches('/'),
            encoded_path_segment(workspace_slug),
        );
        let request = ProviderHttpRequest::new(ProviderOperation::Search, Method::POST, url)
            .map_err(KnowledgeEngineError::from)?
            .optional_bearer_auth(self.bearer_token())
            .map_err(KnowledgeEngineError::from)?
            .json(&serde_json::json!({
                "query": query,
                "topN": top_k,
                "scoreThreshold": 0.0,
            }))
            .map_err(KnowledgeEngineError::from)?
            .idempotent(true);
        let response = self
            .http
            .execute(context, request)
            .await
            .map_err(KnowledgeEngineError::from)?;
        let payload: AnythingLlmVectorSearchResponse =
            response.json().map_err(KnowledgeEngineError::from)?;

        let hits = payload
            .results
            .into_iter()
            .map(|result| map_result_to_hit(space_id, result))
            .collect();

        Ok(KnowledgeEngineSearchResult {
            implementation_id: ANYTHINGLLM_IMPLEMENTATION_ID.to_string(),
            hits,
        })
    }

    pub async fn read_chunk(
        &self,
        context: &ProviderExecutionContext,
        space_id: u64,
        workspace_slug: &str,
        document_hint: &str,
        chunk_id: &str,
    ) -> Result<KnowledgeEngineDocument, KnowledgeEngineError> {
        let search = self
            .vector_search(context, space_id, workspace_slug, document_hint, 25)
            .await?;

        let hit = search
            .hits
            .into_iter()
            .find(|candidate| {
                let local = candidate
                    .document
                    .document_id
                    .split_once('/')
                    .map(|(_, rest)| rest)
                    .unwrap_or(candidate.document.document_id.as_str());
                local
                    .split_once('#')
                    .is_some_and(|(title, id)| title == document_hint && id == chunk_id)
            })
            .ok_or_else(|| {
                KnowledgeEngineError::NotFound(format!(
                    "anythingllm chunk not found in workspace={workspace_slug} chunk_id={chunk_id}"
                ))
            })?;

        let local_document_id = hit
            .document
            .document_id
            .split_once('/')
            .map(|(_, rest)| rest.to_string())
            .unwrap_or_else(|| format!("{document_hint}#{chunk_id}"));

        Ok(KnowledgeEngineDocument {
            document_id: local_document_id,
            title: hit.document.title,
            content: hit.snippet,
            source_uri: hit.document.source_uri,
        })
    }
}

fn map_result_to_hit(space_id: u64, result: AnythingLlmVectorResult) -> KnowledgeEngineSearchHit {
    let chunk_id = result.id.clone();
    let title = result
        .metadata
        .as_ref()
        .and_then(|metadata| metadata.title.clone())
        .filter(|value| !value.is_empty())
        .unwrap_or_else(|| chunk_id.clone());
    let local_document_id = format!("{title}#{chunk_id}");
    let source_uri = result
        .metadata
        .as_ref()
        .and_then(|metadata| metadata.url.clone())
        .filter(|value| !value.is_empty());

    KnowledgeEngineSearchHit {
        document: KnowledgeEngineDocumentRef {
            document_id: format!("{space_id}/{local_document_id}"),
            title: title.clone(),
            source_uri,
        },
        snippet: result.text.unwrap_or_default(),
        score: result.score,
    }
}

#[derive(Debug, Deserialize)]
struct AnythingLlmVectorSearchResponse {
    #[serde(default)]
    results: Vec<AnythingLlmVectorResult>,
}

#[derive(Debug, Deserialize)]
struct AnythingLlmVectorResult {
    id: String,
    text: Option<String>,
    score: Option<f64>,
    metadata: Option<AnythingLlmVectorMetadata>,
}

#[derive(Debug, Deserialize)]
struct AnythingLlmVectorMetadata {
    title: Option<String>,
    url: Option<String>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn maps_anythingllm_vector_search_payload_to_scoped_hits() {
        let payload = r#"{
            "results": [{
                "id": "chunk-1",
                "text": "workspace snippet",
                "score": 0.82,
                "metadata": {
                    "title": "Policy Doc",
                    "url": "file://policy.txt"
                }
            }]
        }"#;
        let parsed: AnythingLlmVectorSearchResponse = serde_json::from_str(payload).expect("parse");
        let hit = map_result_to_hit(4, parsed.results.into_iter().next().expect("result"));

        assert_eq!(hit.document.document_id, "4/Policy Doc#chunk-1");
        assert_eq!(hit.document.title, "Policy Doc");
        assert_eq!(hit.snippet, "workspace snippet");
        assert_eq!(hit.score, Some(0.82));
    }
}
