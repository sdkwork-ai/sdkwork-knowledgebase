//! Chroma v2 HTTP client (adapter-local).

use reqwest::Method;
use sdkwork_knowledgebase_contract::knowledge_engine::{
    KnowledgeEngineDocument, KnowledgeEngineDocumentRef, KnowledgeEngineError,
    KnowledgeEngineSearchHit, KnowledgeEngineSearchResult,
};
use sdkwork_knowledgebase_provider_runtime::{
    encoded_path_segment, is_path_segment_id, ProviderExecutionContext, ProviderHttpRequest,
    ProviderOperation, ProviderRuntime,
};
use serde::Deserialize;
use serde_json::Value;

use crate::config::ChromaConnectorConfig;
use crate::CHROMA_IMPLEMENTATION_ID;

#[derive(Clone)]
pub struct ChromaApiClient {
    config: ChromaConnectorConfig,
    http: ProviderRuntime,
}

impl ChromaApiClient {
    pub fn new(config: ChromaConnectorConfig) -> Result<Self, KnowledgeEngineError> {
        let http = ProviderRuntime::for_base_url_with_private_targets(
            &config.base_url,
            config.allow_private_network,
        )
        .map_err(KnowledgeEngineError::from)?;
        Ok(Self { config, http })
    }

    fn health_context(&self) -> ProviderExecutionContext {
        ProviderExecutionContext::for_system_health(CHROMA_IMPLEMENTATION_ID)
    }

    fn collection_path(
        &self,
        collection_id: &str,
        suffix: &str,
    ) -> Result<String, KnowledgeEngineError> {
        // Tenant, database, and collection ids are spliced into the URL path, so
        // they are validated and percent-encoded at every interpolation point.
        for (kind, value) in [
            ("tenant", self.config.tenant.as_str()),
            ("database", self.config.database.as_str()),
            ("collection id", collection_id),
        ] {
            if !is_path_segment_id(value) {
                return Err(KnowledgeEngineError::Validation(format!(
                    "Chroma {kind} must match [A-Za-z0-9._:-]{{1,256}}"
                )));
            }
        }
        Ok(format!(
            "{}/api/v2/tenants/{}/databases/{}/collections/{}{suffix}",
            self.config.base_url.trim_end_matches('/'),
            encoded_path_segment(&self.config.tenant),
            encoded_path_segment(&self.config.database),
            encoded_path_segment(collection_id),
        ))
    }

    pub async fn connector_health(&self) -> Result<(), KnowledgeEngineError> {
        let url = format!(
            "{}/api/v2/heartbeat",
            self.config.base_url.trim_end_matches('/')
        );
        let request = ProviderHttpRequest::new(ProviderOperation::Health, Method::GET, url)
            .map_err(KnowledgeEngineError::from)?
            .optional_bearer_auth(self.config.api_key.as_ref().map(|value| value.as_str()))
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
        collection_id: &str,
        query: &str,
        top_k: u32,
    ) -> Result<KnowledgeEngineSearchResult, KnowledgeEngineError> {
        let url = format!("{}/query", self.collection_path(collection_id, "")?);
        let request = ProviderHttpRequest::new(ProviderOperation::Search, Method::POST, url)
            .map_err(KnowledgeEngineError::from)?
            .optional_bearer_auth(self.config.api_key.as_ref().map(|value| value.as_str()))
            .map_err(KnowledgeEngineError::from)?
            .json(&serde_json::json!({
                "query_texts": [query],
                "n_results": top_k,
                "include": ["metadatas", "documents", "distances"],
            }))
            .map_err(KnowledgeEngineError::from)?
            .idempotent(true);
        let response = self
            .http
            .execute(context, request)
            .await
            .map_err(KnowledgeEngineError::from)?;
        let payload: ChromaQueryResponse = response.json().map_err(KnowledgeEngineError::from)?;

        let hits = map_query_response_to_hits(space_id, payload);
        Ok(KnowledgeEngineSearchResult {
            implementation_id: CHROMA_IMPLEMENTATION_ID.to_string(),
            hits,
        })
    }

    pub async fn get_record(
        &self,
        context: &ProviderExecutionContext,
        collection_id: &str,
        record_id: &str,
    ) -> Result<KnowledgeEngineDocument, KnowledgeEngineError> {
        let url = format!("{}/get", self.collection_path(collection_id, "")?);
        let request = ProviderHttpRequest::new(ProviderOperation::Read, Method::POST, url)
            .map_err(KnowledgeEngineError::from)?
            .optional_bearer_auth(self.config.api_key.as_ref().map(|value| value.as_str()))
            .map_err(KnowledgeEngineError::from)?
            .json(&serde_json::json!({
                "ids": [record_id],
                "include": ["metadatas", "documents"],
            }))
            .map_err(KnowledgeEngineError::from)?
            .idempotent(true);
        let response = self
            .http
            .execute(context, request)
            .await
            .map_err(KnowledgeEngineError::from)?;
        let payload: ChromaGetResponse = response.json().map_err(KnowledgeEngineError::from)?;

        let record_id_value = payload.ids.first().cloned().ok_or_else(|| {
            KnowledgeEngineError::NotFound(format!("chroma record payload missing id={record_id}"))
        })?;
        let content = payload
            .documents
            .and_then(|documents| documents.into_iter().next())
            .unwrap_or_default();
        let metadata = payload
            .metadatas
            .and_then(|metadatas| metadatas.into_iter().next())
            .unwrap_or(Value::Null);
        let title = metadata_title(&metadata).unwrap_or_else(|| record_id_value.clone());

        Ok(KnowledgeEngineDocument {
            document_id: format!("{title}#{record_id_value}"),
            title,
            content,
            source_uri: metadata
                .get("source")
                .or_else(|| metadata.get("url"))
                .and_then(Value::as_str)
                .filter(|value| !value.is_empty())
                .map(str::to_string),
        })
    }

    pub async fn get_collection(
        &self,
        context: &ProviderExecutionContext,
        collection_id: &str,
    ) -> Result<ChromaCollection, KnowledgeEngineError> {
        let url = self.collection_path(collection_id, "")?;
        let request = ProviderHttpRequest::new(ProviderOperation::Read, Method::GET, url)
            .map_err(KnowledgeEngineError::from)?
            .optional_bearer_auth(self.config.api_key.as_ref().map(|value| value.as_str()))
            .map_err(KnowledgeEngineError::from)?
            .idempotent(true);
        let response = self
            .http
            .execute(context, request)
            .await
            .map_err(KnowledgeEngineError::from)?;
        response.json().map_err(KnowledgeEngineError::from)
    }
}

fn map_query_response_to_hits(
    space_id: u64,
    payload: ChromaQueryResponse,
) -> Vec<KnowledgeEngineSearchHit> {
    let ids = payload.ids.into_iter().next().unwrap_or_default();
    let documents = payload
        .documents
        .unwrap_or_default()
        .into_iter()
        .next()
        .unwrap_or_default();
    let metadatas = payload
        .metadatas
        .unwrap_or_default()
        .into_iter()
        .next()
        .unwrap_or_default();
    let distances = payload
        .distances
        .unwrap_or_default()
        .into_iter()
        .next()
        .unwrap_or_default();

    ids.into_iter()
        .zip(documents)
        .zip(metadatas)
        .zip(distances)
        .map(|(((record_id, content), metadata), distance)| {
            let title = metadata_title(&metadata).unwrap_or_else(|| record_id.clone());
            let local_document_id = format!("{title}#{record_id}");
            let source_uri = metadata
                .get("source")
                .or_else(|| metadata.get("url"))
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
                score: Some(1.0 - distance),
            }
        })
        .collect()
}

fn metadata_title(metadata: &Value) -> Option<String> {
    metadata
        .get("title")
        .or_else(|| metadata.get("source"))
        .or_else(|| metadata.get("name"))
        .and_then(Value::as_str)
        .filter(|value| !value.is_empty())
        .map(str::to_string)
}

#[derive(Debug, Deserialize)]
struct ChromaQueryResponse {
    #[serde(default)]
    ids: Vec<Vec<String>>,
    documents: Option<Vec<Vec<String>>>,
    metadatas: Option<Vec<Vec<Value>>>,
    distances: Option<Vec<Vec<f64>>>,
}

#[derive(Debug, Deserialize)]
struct ChromaGetResponse {
    #[serde(default)]
    ids: Vec<String>,
    documents: Option<Vec<String>>,
    metadatas: Option<Vec<Value>>,
}

#[derive(Debug, Deserialize)]
pub struct ChromaCollection {
    pub id: String,
    pub name: Option<String>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn maps_chroma_query_payload_to_scoped_hits() {
        let payload = ChromaQueryResponse {
            ids: vec![vec!["rec-1".to_string()]],
            documents: Some(vec![vec!["policy snippet".to_string()]]),
            metadatas: Some(vec![vec![serde_json::json!({
                "title": "Policy Doc",
                "source": "file://policy.txt"
            })]]),
            distances: Some(vec![vec![0.12]]),
        };
        let hits = map_query_response_to_hits(4, payload);

        assert_eq!(hits.len(), 1);
        assert_eq!(hits[0].document.document_id, "4/Policy Doc#rec-1");
        assert_eq!(hits[0].snippet, "policy snippet");
        assert_eq!(hits[0].score, Some(0.88));
    }
}
