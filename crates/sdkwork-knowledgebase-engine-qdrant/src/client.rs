//! Qdrant REST HTTP client (adapter-local).

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

use crate::config::{QdrantConnectorConfig, QDRANT_QUERY_MODEL_ENV};
use crate::QDRANT_IMPLEMENTATION_ID;

#[derive(Clone)]
pub struct QdrantApiClient {
    config: QdrantConnectorConfig,
    http: ProviderRuntime,
}

impl QdrantApiClient {
    pub fn new(config: QdrantConnectorConfig) -> Result<Self, KnowledgeEngineError> {
        let http = ProviderRuntime::for_base_url_with_private_targets(
            &config.base_url,
            config.allow_private_network,
        )
        .map_err(KnowledgeEngineError::from)?;
        Ok(Self { config, http })
    }

    fn health_context(&self) -> ProviderExecutionContext {
        ProviderExecutionContext::for_system_health(QDRANT_IMPLEMENTATION_ID)
    }

    fn authed(
        &self,
        request: ProviderHttpRequest,
    ) -> Result<ProviderHttpRequest, KnowledgeEngineError> {
        match self.config.api_key.as_ref().map(|value| value.as_str()) {
            Some(api_key) => request
                .sensitive_header("api-key", api_key)
                .map_err(KnowledgeEngineError::from),
            None => Ok(request),
        }
    }

    /// Binding-controlled collection names are spliced into the URL path, so
    /// they are validated and percent-encoded at every interpolation point.
    fn collection_url(
        &self,
        collection_name: &str,
        suffix: &str,
    ) -> Result<String, KnowledgeEngineError> {
        if !is_path_segment_id(collection_name) {
            return Err(KnowledgeEngineError::Validation(
                "Qdrant collection name must match [A-Za-z0-9._:-]{1,256}".to_string(),
            ));
        }
        Ok(format!(
            "{}/collections/{}{suffix}",
            self.config.base_url.trim_end_matches('/'),
            encoded_path_segment(collection_name),
        ))
    }

    pub async fn connector_health(
        &self,
        collection_name: &str,
    ) -> Result<(), KnowledgeEngineError> {
        let url = self.collection_url(collection_name, "")?;
        let request = self
            .authed(
                ProviderHttpRequest::new(ProviderOperation::Health, Method::GET, url)
                    .map_err(KnowledgeEngineError::from)?,
            )?
            .idempotent(true);
        self.http
            .execute(&self.health_context(), request)
            .await
            .map_err(KnowledgeEngineError::from)?;
        Ok(())
    }

    pub async fn query_points(
        &self,
        context: &ProviderExecutionContext,
        space_id: u64,
        collection_name: &str,
        query: &str,
        top_k: u32,
    ) -> Result<KnowledgeEngineSearchResult, KnowledgeEngineError> {
        let query_model = self.config.query_model.as_deref().ok_or_else(|| {
            KnowledgeEngineError::Validation(format!(
                "Qdrant text query requires {QDRANT_QUERY_MODEL_ENV} for server-side embedding"
            ))
        })?;

        let mut body = serde_json::json!({
            "query": {
                "text": query,
                "model": query_model,
            },
            "limit": top_k,
            "with_payload": true,
        });
        if let Some(using_vector) = self.config.using_vector.as_deref() {
            body["using"] = Value::String(using_vector.to_string());
        }

        let url = self.collection_url(collection_name, "/points/query")?;
        let request = self
            .authed(
                ProviderHttpRequest::new(ProviderOperation::Search, Method::POST, url)
                    .map_err(KnowledgeEngineError::from)?,
            )?
            .json(&body)
            .map_err(KnowledgeEngineError::from)?
            .idempotent(true);
        let response = self
            .http
            .execute(context, request)
            .await
            .map_err(KnowledgeEngineError::from)?;
        let payload: QdrantQueryResponse = response.json().map_err(KnowledgeEngineError::from)?;

        let hits = payload
            .result
            .map(|result| map_points_to_hits(space_id, result.points))
            .unwrap_or_default();

        Ok(KnowledgeEngineSearchResult {
            implementation_id: QDRANT_IMPLEMENTATION_ID.to_string(),
            hits,
        })
    }

    pub async fn get_point(
        &self,
        context: &ProviderExecutionContext,
        collection_name: &str,
        point_id: &str,
    ) -> Result<KnowledgeEngineDocument, KnowledgeEngineError> {
        let parsed_id = parse_point_id(point_id);
        let url = self.collection_url(collection_name, "/points")?;
        let request = self
            .authed(
                ProviderHttpRequest::new(ProviderOperation::Read, Method::POST, url)
                    .map_err(KnowledgeEngineError::from)?,
            )?
            .json(&serde_json::json!({
                "ids": [parsed_id],
                "with_payload": true,
            }))
            .map_err(KnowledgeEngineError::from)?
            .idempotent(true);
        let response = self
            .http
            .execute(context, request)
            .await
            .map_err(KnowledgeEngineError::from)?;
        let payload: QdrantGetPointsResponse =
            response.json().map_err(KnowledgeEngineError::from)?;

        let point = payload.result.into_iter().next().ok_or_else(|| {
            KnowledgeEngineError::NotFound(format!(
                "qdrant point not found: collection={collection_name} id={point_id}"
            ))
        })?;

        let payload_value = point.payload.unwrap_or(Value::Null);
        let title = payload_title(&payload_value).unwrap_or_else(|| point_id.to_string());
        let content = payload_content(&payload_value);
        let point_id_string = point_id_string(&point.id);

        Ok(KnowledgeEngineDocument {
            document_id: format!("{title}#{point_id_string}"),
            title,
            content,
            source_uri: payload_value
                .get("source")
                .or_else(|| payload_value.get("url"))
                .and_then(Value::as_str)
                .filter(|value| !value.is_empty())
                .map(str::to_string),
        })
    }

    pub async fn get_collection(
        &self,
        context: &ProviderExecutionContext,
        collection_name: &str,
    ) -> Result<QdrantCollectionInfo, KnowledgeEngineError> {
        let url = self.collection_url(collection_name, "")?;
        let request = self
            .authed(
                ProviderHttpRequest::new(ProviderOperation::Read, Method::GET, url)
                    .map_err(KnowledgeEngineError::from)?,
            )?
            .idempotent(true);
        let response = self
            .http
            .execute(context, request)
            .await
            .map_err(KnowledgeEngineError::from)?;
        let payload: QdrantCollectionResponse =
            response.json().map_err(KnowledgeEngineError::from)?;

        payload.result.ok_or_else(|| {
            KnowledgeEngineError::Internal("qdrant collection payload missing result".to_string())
        })
    }
}

fn map_points_to_hits(
    space_id: u64,
    points: Vec<QdrantScoredPoint>,
) -> Vec<KnowledgeEngineSearchHit> {
    points
        .into_iter()
        .map(|point| {
            let payload = point.payload.unwrap_or(Value::Null);
            let point_id = point_id_string(&point.id);
            let title = payload_title(&payload).unwrap_or_else(|| point_id.clone());
            let local_document_id = format!("{title}#{point_id}");
            let snippet = payload_content(&payload);

            KnowledgeEngineSearchHit {
                document: KnowledgeEngineDocumentRef {
                    document_id: format!("{space_id}/{local_document_id}"),
                    title: title.clone(),
                    source_uri: payload
                        .get("source")
                        .or_else(|| payload.get("url"))
                        .and_then(Value::as_str)
                        .filter(|value| !value.is_empty())
                        .map(str::to_string),
                },
                snippet,
                score: point.score,
            }
        })
        .collect()
}

fn payload_title(payload: &Value) -> Option<String> {
    payload
        .get("title")
        .or_else(|| payload.get("source"))
        .or_else(|| payload.get("name"))
        .or_else(|| payload.get("document"))
        .and_then(Value::as_str)
        .filter(|value| !value.is_empty())
        .map(str::to_string)
}

fn payload_content(payload: &Value) -> String {
    payload
        .get("text")
        .or_else(|| payload.get("content"))
        .or_else(|| payload.get("page_content"))
        .and_then(Value::as_str)
        .unwrap_or_default()
        .to_string()
}

fn point_id_string(id: &Value) -> String {
    match id {
        Value::String(value) => value.clone(),
        Value::Number(value) => value.to_string(),
        other => other.to_string(),
    }
}

fn parse_point_id(point_id: &str) -> Value {
    if let Ok(value) = point_id.parse::<u64>() {
        Value::Number(value.into())
    } else {
        Value::String(point_id.to_string())
    }
}

#[derive(Debug, Deserialize)]
struct QdrantQueryResponse {
    result: Option<QdrantQueryResult>,
}

#[derive(Debug, Deserialize)]
struct QdrantQueryResult {
    #[serde(default)]
    points: Vec<QdrantScoredPoint>,
}

#[derive(Debug, Deserialize)]
struct QdrantScoredPoint {
    id: Value,
    score: Option<f64>,
    payload: Option<Value>,
}

#[derive(Debug, Deserialize)]
struct QdrantGetPointsResponse {
    #[serde(default)]
    result: Vec<QdrantPointRecord>,
}

#[derive(Debug, Deserialize)]
struct QdrantPointRecord {
    id: Value,
    payload: Option<Value>,
}

#[derive(Debug, Deserialize)]
struct QdrantCollectionResponse {
    result: Option<QdrantCollectionInfo>,
}

#[derive(Debug, Deserialize)]
pub struct QdrantCollectionInfo {
    #[serde(default)]
    pub points_count: Option<u64>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn maps_qdrant_query_payload_to_scoped_hits() {
        let points = vec![QdrantScoredPoint {
            id: Value::String("pt-1".to_string()),
            score: Some(0.91),
            payload: Some(serde_json::json!({
                "title": "Policy Doc",
                "text": "policy snippet",
                "source": "file://policy.txt"
            })),
        }];
        let hits = map_points_to_hits(4, points);

        assert_eq!(hits.len(), 1);
        assert_eq!(hits[0].document.document_id, "4/Policy Doc#pt-1");
        assert_eq!(hits[0].snippet, "policy snippet");
        assert_eq!(hits[0].score, Some(0.91));
    }
}
