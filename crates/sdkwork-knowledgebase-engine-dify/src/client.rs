//! Dify dataset retrieve HTTP client (adapter-local; handlers must not call Dify directly).

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

use crate::config::DifyConnectorConfig;
use crate::DIFY_IMPLEMENTATION_ID;

#[derive(Clone)]
pub struct DifyApiClient {
    config: DifyConnectorConfig,
    http: ProviderRuntime,
}

impl DifyApiClient {
    pub fn new(config: DifyConnectorConfig) -> Result<Self, KnowledgeEngineError> {
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
        ProviderExecutionContext::for_system_health(DIFY_IMPLEMENTATION_ID)
    }

    /// Binding-controlled ids are spliced into URL paths, so they are validated
    /// and percent-encoded at every interpolation point.
    fn validated_id<'a>(kind: &str, value: &'a str) -> Result<&'a str, KnowledgeEngineError> {
        if is_path_segment_id(value) {
            Ok(value)
        } else {
            Err(KnowledgeEngineError::Validation(format!(
                "Dify {kind} must match [A-Za-z0-9._:-]{{1,256}}"
            )))
        }
    }

    pub async fn connector_health(&self, dataset_id: &str) -> Result<(), KnowledgeEngineError> {
        let dataset_id = Self::validated_id("dataset id", dataset_id)?;
        let url = format!(
            "{}/datasets/{}",
            self.config.base_url.trim_end_matches('/'),
            encoded_path_segment(dataset_id),
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

    pub async fn retrieve(
        &self,
        context: &ProviderExecutionContext,
        space_id: u64,
        dataset_id: &str,
        query: &str,
        top_k: u32,
    ) -> Result<KnowledgeEngineSearchResult, KnowledgeEngineError> {
        let dataset_id = Self::validated_id("dataset id", dataset_id)?;
        let url = format!(
            "{}/datasets/{}/retrieve",
            self.config.base_url.trim_end_matches('/'),
            encoded_path_segment(dataset_id),
        );
        let request = ProviderHttpRequest::new(ProviderOperation::Search, Method::POST, url)
            .map_err(KnowledgeEngineError::from)?
            .optional_bearer_auth(self.bearer_token())
            .map_err(KnowledgeEngineError::from)?
            .json(&serde_json::json!({
                "query": query,
                "top_k": top_k,
            }))
            .map_err(KnowledgeEngineError::from)?
            .idempotent(true);
        let response = self
            .http
            .execute(context, request)
            .await
            .map_err(KnowledgeEngineError::from)?;
        let payload: DifyRetrieveResponse = response.json().map_err(KnowledgeEngineError::from)?;

        let hits = payload
            .records
            .into_iter()
            .map(|record| map_record_to_hit(space_id, record))
            .collect();

        Ok(KnowledgeEngineSearchResult {
            implementation_id: DIFY_IMPLEMENTATION_ID.to_string(),
            hits,
        })
    }

    pub async fn read_segment(
        &self,
        context: &ProviderExecutionContext,
        dataset_id: &str,
        document_id: &str,
        segment_id: &str,
    ) -> Result<KnowledgeEngineDocument, KnowledgeEngineError> {
        let dataset_id = Self::validated_id("dataset id", dataset_id)?;
        let document_id = Self::validated_id("document id", document_id)?;
        let segment_id = Self::validated_id("segment id", segment_id)?;
        let url = format!(
            "{}/datasets/{}/documents/{}/segments/{}",
            self.config.base_url.trim_end_matches('/'),
            encoded_path_segment(dataset_id),
            encoded_path_segment(document_id),
            encoded_path_segment(segment_id),
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
        let payload: DifySegmentDetailResponse =
            response.json().map_err(KnowledgeEngineError::from)?;

        let segment = payload.data.ok_or_else(|| {
            KnowledgeEngineError::NotFound(format!(
                "dify segment payload missing for segment_id={segment_id}"
            ))
        })?;

        let title = segment
            .document
            .and_then(|document| document.name)
            .unwrap_or_else(|| document_id.to_string());

        Ok(KnowledgeEngineDocument {
            document_id: format!("{document_id}#{segment_id}"),
            title,
            content: segment.content.unwrap_or_default(),
            source_uri: Some(format!(
                "dify://documents/{document_id}/segments/{segment_id}"
            )),
        })
    }
}

fn map_record_to_hit(space_id: u64, record: DifyRetrieveRecord) -> KnowledgeEngineSearchHit {
    let segment = record.segment;
    let segment_id = segment
        .id
        .clone()
        .or_else(|| segment.document_id.clone())
        .unwrap_or_else(|| "unknown".to_string());
    let parent_document_id = segment.document_id.clone();
    let local_document_id = match parent_document_id.as_deref() {
        Some(document_id)
            if !document_id.is_empty() && segment_id != "unknown" && document_id != segment_id =>
        {
            format!("{document_id}#{segment_id}")
        }
        _ => segment_id.clone(),
    };
    let title = segment
        .document
        .and_then(|document| document.name)
        .unwrap_or_else(|| segment_id.clone());

    KnowledgeEngineSearchHit {
        document: KnowledgeEngineDocumentRef {
            document_id: format!("{space_id}/{local_document_id}"),
            title,
            source_uri: parent_document_id
                .map(|document_id| format!("dify://documents/{document_id}/segments/{segment_id}")),
        },
        snippet: segment.content.unwrap_or_default(),
        score: record.score,
    }
}

#[derive(Debug, Deserialize)]
struct DifyRetrieveResponse {
    #[serde(default)]
    records: Vec<DifyRetrieveRecord>,
}

#[derive(Debug, Deserialize)]
struct DifyRetrieveRecord {
    segment: DifySegment,
    score: Option<f64>,
}

#[derive(Debug, Deserialize)]
struct DifySegment {
    id: Option<String>,
    #[serde(rename = "document_id")]
    document_id: Option<String>,
    content: Option<String>,
    document: Option<DifySegmentDocument>,
}

#[derive(Debug, Deserialize)]
struct DifySegmentDocument {
    name: Option<String>,
}

#[derive(Debug, Deserialize)]
struct DifySegmentDetailResponse {
    data: Option<DifySegmentDetail>,
}

#[derive(Debug, Deserialize)]
struct DifySegmentDetail {
    content: Option<String>,
    document: Option<DifySegmentDocument>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn maps_dify_retrieve_payload_to_scoped_hits() {
        let payload = r#"{
            "records": [
                {
                    "segment": {
                        "id": "seg-1",
                        "document_id": "doc-1",
                        "content": "hello world",
                        "document": { "name": "Doc A" }
                    },
                    "score": 0.91
                }
            ]
        }"#;
        let parsed: DifyRetrieveResponse = serde_json::from_str(payload).expect("parse");
        let hit = map_record_to_hit(7, parsed.records.into_iter().next().expect("record"));

        assert_eq!(hit.document.document_id, "7/doc-1#seg-1");
        assert_eq!(hit.document.title, "Doc A");
        assert_eq!(hit.snippet, "hello world");
        assert_eq!(hit.score, Some(0.91));
    }
}
