//! Flowise document-store vector query HTTP client (adapter-local).

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

use crate::config::FlowiseConnectorConfig;
use crate::FLOWISE_IMPLEMENTATION_ID;

#[derive(Clone)]
pub struct FlowiseApiClient {
    config: FlowiseConnectorConfig,
    http: ProviderRuntime,
}

impl FlowiseApiClient {
    pub fn new(config: FlowiseConnectorConfig) -> Result<Self, KnowledgeEngineError> {
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
        ProviderExecutionContext::for_system_health(FLOWISE_IMPLEMENTATION_ID)
    }

    pub async fn connector_health(&self, store_id: &str) -> Result<(), KnowledgeEngineError> {
        let store_id = Self::validated_store_id(store_id)?;
        let url = format!(
            "{}/api/v1/document-store/store/{}",
            self.config.base_url.trim_end_matches('/'),
            encoded_path_segment(store_id),
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

    /// Binding-controlled store ids are spliced into URL paths, so they are
    /// validated and percent-encoded at every interpolation point.
    fn validated_store_id(store_id: &str) -> Result<&str, KnowledgeEngineError> {
        if is_path_segment_id(store_id) {
            Ok(store_id)
        } else {
            Err(KnowledgeEngineError::Validation(
                "Flowise store id must match [A-Za-z0-9._:-]{1,256}".to_string(),
            ))
        }
    }

    pub async fn query_vector_store(
        &self,
        context: &ProviderExecutionContext,
        space_id: u64,
        store_id: &str,
        query: &str,
        top_k: u32,
    ) -> Result<KnowledgeEngineSearchResult, KnowledgeEngineError> {
        let url = format!(
            "{}/api/v1/document-store/vectorstore/query",
            self.config.base_url.trim_end_matches('/')
        );
        let request = ProviderHttpRequest::new(ProviderOperation::Search, Method::POST, url)
            .map_err(KnowledgeEngineError::from)?
            .optional_bearer_auth(self.bearer_token())
            .map_err(KnowledgeEngineError::from)?
            .json(&serde_json::json!({
                "storeId": store_id,
                "query": query,
            }))
            .map_err(KnowledgeEngineError::from)?
            .idempotent(true);
        let response = self
            .http
            .execute(context, request)
            .await
            .map_err(KnowledgeEngineError::from)?;
        let payload: FlowiseVectorQueryResponse =
            response.json().map_err(KnowledgeEngineError::from)?;

        let hits = payload
            .docs
            .into_iter()
            .take(top_k as usize)
            .enumerate()
            .map(|(index, doc)| map_doc_to_hit(space_id, doc, index))
            .collect();

        Ok(KnowledgeEngineSearchResult {
            implementation_id: FLOWISE_IMPLEMENTATION_ID.to_string(),
            hits,
        })
    }

    pub async fn read_chunk(
        &self,
        context: &ProviderExecutionContext,
        space_id: u64,
        store_id: &str,
        document_hint: &str,
        chunk_id: &str,
    ) -> Result<KnowledgeEngineDocument, KnowledgeEngineError> {
        let search = self
            .query_vector_store(context, space_id, store_id, document_hint, 25)
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
                    "flowise chunk not found in store_id={store_id} chunk_id={chunk_id}"
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

fn map_doc_to_hit(space_id: u64, doc: FlowiseDocument, rank: usize) -> KnowledgeEngineSearchHit {
    let content = doc.page_content.unwrap_or_default();
    let title = doc
        .metadata
        .as_ref()
        .and_then(metadata_title)
        .unwrap_or_else(|| "document".to_string());
    let chunk_id = chunk_id_from_content(&content);
    let local_document_id = format!("{title}#{chunk_id}");
    let source_uri = doc.metadata.as_ref().and_then(|metadata| {
        metadata
            .get("source")
            .or_else(|| metadata.get("url"))
            .and_then(Value::as_str)
            .filter(|value| !value.is_empty())
            .map(str::to_string)
    });

    let score = doc
        .metadata
        .as_ref()
        .and_then(|metadata| metadata.get("score"))
        .and_then(Value::as_f64)
        .or_else(|| Some(1.0 / (rank as f64 + 1.0)));

    KnowledgeEngineSearchHit {
        document: KnowledgeEngineDocumentRef {
            document_id: format!("{space_id}/{local_document_id}"),
            title: title.clone(),
            source_uri,
        },
        snippet: content,
        score,
    }
}

fn metadata_title(metadata: &Value) -> Option<String> {
    metadata
        .get("source")
        .or_else(|| metadata.get("title"))
        .or_else(|| metadata.get("name"))
        .and_then(Value::as_str)
        .filter(|value| !value.is_empty())
        .map(str::to_string)
}

pub fn chunk_id_from_content(content: &str) -> String {
    let digest = Sha256::digest(content.as_bytes());
    format!("{:x}", digest)[..16].to_string()
}

#[derive(Debug, Deserialize)]
struct FlowiseVectorQueryResponse {
    #[serde(default)]
    docs: Vec<FlowiseDocument>,
}

#[derive(Debug, Deserialize)]
struct FlowiseDocument {
    #[serde(rename = "pageContent")]
    page_content: Option<String>,
    #[serde(default)]
    metadata: Option<Value>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn maps_flowise_vector_query_payload_to_scoped_hits() {
        let doc = FlowiseDocument {
            page_content: Some("policy snippet".to_string()),
            metadata: Some(serde_json::json!({
                "source": "Policy Doc",
                "url": "file://policy.txt"
            })),
        };
        let hit = map_doc_to_hit(4, doc, 0);
        let chunk_id = chunk_id_from_content("policy snippet");

        assert_eq!(hit.document.document_id, format!("4/Policy Doc#{chunk_id}"));
        assert_eq!(hit.document.title, "Policy Doc");
        assert_eq!(hit.snippet, "policy snippet");
        assert!(hit.score.is_some());
    }
}
