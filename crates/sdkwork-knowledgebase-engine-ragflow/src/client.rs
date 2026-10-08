//! RAGFlow retrieval HTTP client (adapter-local; handlers must not call RAGFlow directly).

use reqwest::Method;
use sdkwork_knowledgebase_contract::knowledge_engine::{
    KnowledgeEngineDocument, KnowledgeEngineDocumentRef, KnowledgeEngineError,
    KnowledgeEngineSearchHit, KnowledgeEngineSearchResult,
};
use sdkwork_knowledgebase_provider_runtime::{
    encoded_path_segment, engine_provider_error, is_path_segment_id, optional_bearer_token,
    ProviderErrorCategory, ProviderExecutionContext, ProviderHttpRequest, ProviderOperation,
    ProviderRuntime,
};
use serde::Deserialize;

use crate::config::RagflowConnectorConfig;
use crate::RAGFLOW_IMPLEMENTATION_ID;

#[derive(Clone)]
pub struct RagflowApiClient {
    config: RagflowConnectorConfig,
    http: ProviderRuntime,
}

impl RagflowApiClient {
    pub fn new(config: RagflowConnectorConfig) -> Result<Self, KnowledgeEngineError> {
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
        ProviderExecutionContext::for_system_health(RAGFLOW_IMPLEMENTATION_ID)
    }

    /// Caller- and binding-controlled ids are spliced into URL paths and queries,
    /// so they are validated and percent-encoded at every interpolation point.
    fn validated_id<'a>(kind: &str, value: &'a str) -> Result<&'a str, KnowledgeEngineError> {
        if is_path_segment_id(value) {
            Ok(value)
        } else {
            Err(KnowledgeEngineError::Validation(format!(
                "RAGFlow {kind} must match [A-Za-z0-9._:-]{{1,256}}"
            )))
        }
    }

    pub async fn connector_health(&self, dataset_id: &str) -> Result<(), KnowledgeEngineError> {
        let dataset_id = Self::validated_id("dataset id", dataset_id)?;
        let url = format!(
            "{}/api/v1/datasets?id={}",
            self.config.base_url.trim_end_matches('/'),
            encoded_path_segment(dataset_id),
        );
        let request = ProviderHttpRequest::new(ProviderOperation::Health, Method::GET, url)
            .map_err(KnowledgeEngineError::from)?
            .optional_bearer_auth(self.bearer_token())
            .map_err(KnowledgeEngineError::from)?
            .idempotent(true);
        let response = self
            .http
            .execute(&self.health_context(), request)
            .await
            .map_err(KnowledgeEngineError::from)?;
        let payload: RagflowApiResponse<RagflowDatasetList> =
            response.json().map_err(KnowledgeEngineError::from)?;

        if payload.code != 0 {
            // Raw engine error text is diagnostic-only and must not reach API errors.
            let detail = payload.message.unwrap_or_default();
            tracing::warn!(implementation_id = RAGFLOW_IMPLEMENTATION_ID, error = %detail, "engine request failed");
            return Err(engine_provider_error(
                ProviderOperation::Health,
                RAGFLOW_IMPLEMENTATION_ID,
                ProviderErrorCategory::InvalidResponse,
                "ragflow rejected the request",
            ));
        }

        Ok(())
    }

    pub async fn retrieve(
        &self,
        context: &ProviderExecutionContext,
        space_id: u64,
        dataset_id: &str,
        query: &str,
        top_k: u32,
    ) -> Result<KnowledgeEngineSearchResult, KnowledgeEngineError> {
        let url = format!(
            "{}/api/v1/retrieval",
            self.config.base_url.trim_end_matches('/')
        );
        let request = ProviderHttpRequest::new(ProviderOperation::Search, Method::POST, url)
            .map_err(KnowledgeEngineError::from)?
            .optional_bearer_auth(self.bearer_token())
            .map_err(KnowledgeEngineError::from)?
            .json(&serde_json::json!({
                "question": query,
                "dataset_ids": [dataset_id],
                "page_size": top_k,
            }))
            .map_err(KnowledgeEngineError::from)?
            .idempotent(true);
        let response = self
            .http
            .execute(context, request)
            .await
            .map_err(KnowledgeEngineError::from)?;
        let payload: RagflowApiResponse<RagflowRetrievalData> =
            response.json().map_err(KnowledgeEngineError::from)?;

        if payload.code != 0 {
            // Raw engine error text is diagnostic-only and must not reach API errors.
            let detail = payload.message.unwrap_or_default();
            tracing::warn!(implementation_id = RAGFLOW_IMPLEMENTATION_ID, error = %detail, "engine request failed");
            return Err(engine_provider_error(
                ProviderOperation::Search,
                RAGFLOW_IMPLEMENTATION_ID,
                ProviderErrorCategory::InvalidResponse,
                "ragflow rejected the request",
            ));
        }

        let chunks = payload.data.map(|data| data.chunks).unwrap_or_default();

        let hits = chunks
            .into_iter()
            .map(|chunk| map_chunk_to_hit(space_id, chunk))
            .collect();

        Ok(KnowledgeEngineSearchResult {
            implementation_id: RAGFLOW_IMPLEMENTATION_ID.to_string(),
            hits,
        })
    }

    pub async fn read_chunk(
        &self,
        context: &ProviderExecutionContext,
        dataset_id: &str,
        document_id: &str,
        chunk_id: &str,
    ) -> Result<KnowledgeEngineDocument, KnowledgeEngineError> {
        let dataset_id = Self::validated_id("dataset id", dataset_id)?;
        let document_id = Self::validated_id("document id", document_id)?;
        let chunk_id = Self::validated_id("chunk id", chunk_id)?;
        let url = format!(
            "{}/api/v1/datasets/{}/documents/{}/chunks/{}",
            self.config.base_url.trim_end_matches('/'),
            encoded_path_segment(dataset_id),
            encoded_path_segment(document_id),
            encoded_path_segment(chunk_id),
        );
        let request = ProviderHttpRequest::new(ProviderOperation::Read, Method::GET, url)
            .map_err(KnowledgeEngineError::from)?
            .optional_bearer_auth(self.bearer_token())
            .map_err(KnowledgeEngineError::from)?
            .idempotent(true);
        let response = self
            .http
            .execute(context, request)
            .await
            .map_err(KnowledgeEngineError::from)?;
        let payload: RagflowApiResponse<RagflowChunkDetail> =
            response.json().map_err(KnowledgeEngineError::from)?;

        if payload.code != 0 {
            return Err(KnowledgeEngineError::NotFound(
                "ragflow chunk not found for the requested document".to_string(),
            ));
        }

        let chunk = payload.data.ok_or_else(|| {
            KnowledgeEngineError::NotFound(format!(
                "ragflow chunk payload missing for chunk_id={chunk_id}"
            ))
        })?;

        let title = chunk
            .document_keyword
            .or(chunk.document_name)
            .unwrap_or_else(|| document_id.to_string());

        Ok(KnowledgeEngineDocument {
            document_id: format!("{document_id}#{chunk_id}"),
            title,
            content: chunk.content.unwrap_or_default(),
            source_uri: Some(format!(
                "ragflow://document/{document_id}/chunks/{chunk_id}"
            )),
        })
    }
}

fn map_chunk_to_hit(space_id: u64, chunk: RagflowChunk) -> KnowledgeEngineSearchHit {
    let chunk_id = chunk.id.clone().unwrap_or_else(|| "unknown".to_string());
    let local_document_id = match chunk.document_id.as_deref() {
        Some(document_id)
            if !document_id.is_empty() && chunk_id != "unknown" && document_id != chunk_id =>
        {
            format!("{document_id}#{chunk_id}")
        }
        _ => chunk_id.clone(),
    };
    let title = chunk
        .document_keyword
        .or(chunk.document_name)
        .unwrap_or_else(|| chunk_id.clone());

    KnowledgeEngineSearchHit {
        document: KnowledgeEngineDocumentRef {
            document_id: format!("{space_id}/{local_document_id}"),
            title,
            source_uri: chunk
                .document_id
                .map(|value| format!("ragflow://document/{value}/chunks/{chunk_id}")),
        },
        snippet: chunk.content.unwrap_or_default(),
        score: chunk.similarity,
    }
}

#[derive(Debug, Deserialize)]
struct RagflowApiResponse<T> {
    code: i32,
    message: Option<String>,
    data: Option<T>,
}

#[derive(Debug, Deserialize)]
struct RagflowRetrievalData {
    #[serde(default)]
    chunks: Vec<RagflowChunk>,
}

#[derive(Debug, Deserialize)]
struct RagflowDatasetList {
    #[serde(default)]
    _data: Vec<serde_json::Value>,
}

#[derive(Debug, Deserialize)]
struct RagflowChunkDetail {
    content: Option<String>,
    document_keyword: Option<String>,
    document_name: Option<String>,
}

#[derive(Debug, Deserialize)]
struct RagflowChunk {
    id: Option<String>,
    content: Option<String>,
    document_id: Option<String>,
    document_keyword: Option<String>,
    document_name: Option<String>,
    similarity: Option<f64>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn maps_ragflow_retrieve_payload_to_scoped_hits() {
        let payload = r#"{
            "code": 0,
            "data": {
                "chunks": [
                    {
                        "id": "chunk-1",
                        "content": "ragflow snippet",
                        "document_id": "doc-9",
                        "document_keyword": "Policy Doc",
                        "similarity": 0.87
                    }
                ]
            }
        }"#;
        let parsed: RagflowApiResponse<RagflowRetrievalData> =
            serde_json::from_str(payload).expect("parse");
        let chunk = parsed
            .data
            .expect("data")
            .chunks
            .into_iter()
            .next()
            .expect("chunk");
        let hit = map_chunk_to_hit(11, chunk);

        assert_eq!(hit.document.document_id, "11/doc-9#chunk-1");
        assert_eq!(hit.document.title, "Policy Doc");
        assert_eq!(hit.snippet, "ragflow snippet");
        assert_eq!(hit.score, Some(0.87));
    }
}
