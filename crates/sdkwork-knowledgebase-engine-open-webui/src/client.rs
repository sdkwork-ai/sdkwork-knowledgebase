//! Open WebUI retrieval HTTP client (adapter-local).

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
use serde_json::Value;
use sha2::{Digest, Sha256};

use crate::config::OpenWebuiConnectorConfig;
use crate::OPEN_WEBUI_IMPLEMENTATION_ID;

#[derive(Clone)]
pub struct OpenWebuiApiClient {
    config: OpenWebuiConnectorConfig,
    http: ProviderRuntime,
}

impl OpenWebuiApiClient {
    pub fn new(config: OpenWebuiConnectorConfig) -> Result<Self, KnowledgeEngineError> {
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
        ProviderExecutionContext::for_system_health(OPEN_WEBUI_IMPLEMENTATION_ID)
    }

    /// Binding-controlled knowledge ids are spliced into URL paths, so they are
    /// validated and percent-encoded at every interpolation point.
    fn validated_knowledge_id(knowledge_id: &str) -> Result<&str, KnowledgeEngineError> {
        if is_path_segment_id(knowledge_id) {
            Ok(knowledge_id)
        } else {
            Err(KnowledgeEngineError::Validation(
                "Open WebUI knowledge id must match [A-Za-z0-9._:-]{1,256}".to_string(),
            ))
        }
    }

    pub async fn connector_health(&self, knowledge_id: &str) -> Result<(), KnowledgeEngineError> {
        let knowledge_id = Self::validated_knowledge_id(knowledge_id)?;
        let url = format!(
            "{}/api/v1/knowledge/{}",
            self.config.base_url.trim_end_matches('/'),
            encoded_path_segment(knowledge_id),
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

    pub async fn query_collection(
        &self,
        context: &ProviderExecutionContext,
        space_id: u64,
        knowledge_id: &str,
        query: &str,
        top_k: u32,
    ) -> Result<KnowledgeEngineSearchResult, KnowledgeEngineError> {
        let url = format!(
            "{}/api/v1/retrieval/query/collection",
            self.config.base_url.trim_end_matches('/')
        );
        let request = ProviderHttpRequest::new(ProviderOperation::Search, Method::POST, url)
            .map_err(KnowledgeEngineError::from)?
            .optional_bearer_auth(self.bearer_token())
            .map_err(KnowledgeEngineError::from)?
            .json(&serde_json::json!({
                "collection_names": [knowledge_id],
                "query": query,
                "k": top_k,
                "hybrid": false,
            }))
            .map_err(KnowledgeEngineError::from)?
            .idempotent(true);
        let response = self
            .http
            .execute(context, request)
            .await
            .map_err(KnowledgeEngineError::from)?;
        let payload: OpenWebuiQueryResponse =
            response.json().map_err(KnowledgeEngineError::from)?;

        let hits = map_query_response_to_hits(space_id, payload);
        Ok(KnowledgeEngineSearchResult {
            implementation_id: OPEN_WEBUI_IMPLEMENTATION_ID.to_string(),
            hits,
        })
    }

    pub async fn read_chunk(
        &self,
        context: &ProviderExecutionContext,
        space_id: u64,
        knowledge_id: &str,
        document_hint: &str,
        chunk_id: &str,
    ) -> Result<KnowledgeEngineDocument, KnowledgeEngineError> {
        let search = self
            .query_collection(context, space_id, knowledge_id, document_hint, 25)
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
                    "open-webui chunk not found in knowledge_id={knowledge_id} chunk_id={chunk_id}"
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

fn map_query_response_to_hits(
    space_id: u64,
    payload: OpenWebuiQueryResponse,
) -> Vec<KnowledgeEngineSearchHit> {
    let documents = payload.documents.into_iter().next().unwrap_or_default();
    let metadatas = payload.metadatas.into_iter().next().unwrap_or_default();
    let distances = payload.distances.into_iter().next().unwrap_or_default();

    documents
        .into_iter()
        .zip(metadatas)
        .zip(distances)
        .map(|((content, metadata), distance)| {
            map_row_to_hit(space_id, content, metadata, distance)
        })
        .collect()
}

fn map_row_to_hit(
    space_id: u64,
    content: String,
    metadata: Value,
    distance: f64,
) -> KnowledgeEngineSearchHit {
    let title = metadata
        .get("source")
        .or_else(|| metadata.get("name"))
        .and_then(Value::as_str)
        .filter(|value| !value.is_empty())
        .unwrap_or("document")
        .to_string();
    let chunk_id = chunk_id_from_content(&content);
    let local_document_id = format!("{title}#{chunk_id}");
    let source_uri = metadata
        .get("url")
        .or_else(|| metadata.get("source"))
        .and_then(Value::as_str)
        .filter(|value| !value.is_empty())
        .map(str::to_string);

    KnowledgeEngineSearchHit {
        document: KnowledgeEngineDocumentRef {
            document_id: format!("{space_id}/{local_document_id}"),
            title: title.clone(),
            source_uri,
        },
        snippet: content,
        score: Some(distance),
    }
}

pub fn chunk_id_from_content(content: &str) -> String {
    let digest = Sha256::digest(content.as_bytes());
    format!("{:x}", digest)[..16].to_string()
}

#[derive(Debug, Deserialize)]
struct OpenWebuiQueryResponse {
    #[serde(default)]
    distances: Vec<Vec<f64>>,
    #[serde(default)]
    documents: Vec<Vec<String>>,
    #[serde(default)]
    metadatas: Vec<Vec<Value>>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn maps_open_webui_query_payload_to_scoped_hits() {
        let payload = OpenWebuiQueryResponse {
            distances: vec![vec![0.91]],
            documents: vec![vec!["policy snippet".to_string()]],
            metadatas: vec![vec![serde_json::json!({
                "source": "Policy Doc",
                "url": "file://policy.txt"
            })]],
        };
        let hits = map_query_response_to_hits(4, payload);
        let chunk_id = chunk_id_from_content("policy snippet");

        assert_eq!(hits.len(), 1);
        assert_eq!(
            hits[0].document.document_id,
            format!("4/Policy Doc#{chunk_id}")
        );
        assert_eq!(hits[0].document.title, "Policy Doc");
        assert_eq!(hits[0].snippet, "policy snippet");
        assert_eq!(hits[0].score, Some(0.91));
    }
}
