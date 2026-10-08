//! Haystack Hayhooks / Deepset Cloud HTTP client (adapter-local).

use reqwest::Method;
use sdkwork_knowledgebase_contract::knowledge_engine::{
    KnowledgeEngineDocument, KnowledgeEngineDocumentRef, KnowledgeEngineError,
    KnowledgeEngineSearchHit, KnowledgeEngineSearchResult,
};
use sdkwork_knowledgebase_provider_runtime::{
    encoded_path_segment, is_path_segment_id, ProviderExecutionContext, ProviderHttpRequest,
    ProviderOperation, ProviderRuntime,
};
use serde_json::{json, Value};
use sha2::{Digest, Sha256};

use crate::config::{HaystackConnectorConfig, HaystackDeploymentMode};
use crate::HAYSTACK_IMPLEMENTATION_ID;

#[derive(Clone)]
pub struct HaystackApiClient {
    config: HaystackConnectorConfig,
    http: ProviderRuntime,
}

#[derive(Debug, Clone)]
struct HaystackDocumentRecord {
    document_id: String,
    title: String,
    content: String,
    score: Option<f64>,
    source_uri: Option<String>,
}

impl HaystackApiClient {
    pub fn new(config: HaystackConnectorConfig) -> Result<Self, KnowledgeEngineError> {
        let http = ProviderRuntime::for_base_url_with_private_targets(
            &config.base_url,
            config.allow_private_network,
        )
        .map_err(KnowledgeEngineError::from)?;
        Ok(Self { config, http })
    }

    fn health_context(&self) -> ProviderExecutionContext {
        ProviderExecutionContext::for_system_health(HAYSTACK_IMPLEMENTATION_ID)
    }

    /// Binding-controlled workspace and pipeline names are spliced into URL
    /// paths, so they are validated and percent-encoded at every interpolation
    /// point.
    fn validated_id<'a>(kind: &str, value: &'a str) -> Result<&'a str, KnowledgeEngineError> {
        if is_path_segment_id(value) {
            Ok(value)
        } else {
            Err(KnowledgeEngineError::Validation(format!(
                "Haystack {kind} must match [A-Za-z0-9._:-]{{1,256}}"
            )))
        }
    }

    pub async fn connector_health(
        &self,
        workspace: Option<&str>,
        pipeline: &str,
    ) -> Result<(), KnowledgeEngineError> {
        let url = match self.config.deployment_mode {
            HaystackDeploymentMode::Hayhooks => {
                format!("{}/status", self.config.base_url.trim_end_matches('/'))
            }
            HaystackDeploymentMode::DeepsetCloud => {
                let workspace = workspace.ok_or_else(|| {
                    KnowledgeEngineError::Validation(
                        "Haystack cloud health requires workspace name".to_string(),
                    )
                })?;
                let workspace = Self::validated_id("workspace name", workspace)?;
                let pipeline = Self::validated_id("pipeline name", pipeline)?;
                format!(
                    "{}/api/v1/workspaces/{}/pipelines/{}",
                    self.config.base_url.trim_end_matches('/'),
                    encoded_path_segment(workspace),
                    encoded_path_segment(pipeline),
                )
            }
        };

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

    pub async fn search(
        &self,
        context: &ProviderExecutionContext,
        space_id: u64,
        workspace: Option<&str>,
        pipeline: &str,
        query: &str,
        top_k: u32,
    ) -> Result<KnowledgeEngineSearchResult, KnowledgeEngineError> {
        let payload = match self.config.deployment_mode {
            HaystackDeploymentMode::Hayhooks => {
                self.run_hayhooks_pipeline(context, pipeline, query).await?
            }
            HaystackDeploymentMode::DeepsetCloud => {
                self.run_deepset_search(context, workspace, pipeline, query, top_k)
                    .await?
            }
        };

        let mut documents = extract_haystack_documents(&payload);
        documents.truncate(top_k as usize);
        let hits = documents
            .into_iter()
            .map(|document| map_record_to_hit(space_id, document))
            .collect();

        Ok(KnowledgeEngineSearchResult {
            implementation_id: HAYSTACK_IMPLEMENTATION_ID.to_string(),
            hits,
        })
    }

    pub async fn read_document(
        &self,
        context: &ProviderExecutionContext,
        space_id: u64,
        workspace: Option<&str>,
        pipeline: &str,
        document_hint: &str,
        chunk_id: &str,
    ) -> Result<KnowledgeEngineDocument, KnowledgeEngineError> {
        let search = self
            .search(context, space_id, workspace, pipeline, document_hint, 25)
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
                    "haystack document not found in pipeline={pipeline} chunk_id={chunk_id}"
                ))
            })?;

        Ok(KnowledgeEngineDocument {
            document_id: hit
                .document
                .document_id
                .split_once('/')
                .map(|(_, rest)| rest.to_string())
                .unwrap_or_else(|| format!("{document_hint}#{chunk_id}")),
            title: hit.document.title,
            content: hit.snippet,
            source_uri: hit.document.source_uri,
        })
    }

    async fn run_hayhooks_pipeline(
        &self,
        context: &ProviderExecutionContext,
        pipeline: &str,
        query: &str,
    ) -> Result<Value, KnowledgeEngineError> {
        let pipeline = Self::validated_id("pipeline name", pipeline)?;
        let url = format!(
            "{}/{}/run",
            self.config.base_url.trim_end_matches('/'),
            encoded_path_segment(pipeline),
        );
        let mut body = json!({});
        body[self.config.query_field.clone()] = Value::String(query.to_string());

        let request = ProviderHttpRequest::new(ProviderOperation::Search, Method::POST, url)
            .map_err(KnowledgeEngineError::from)?
            .optional_bearer_auth(self.config.api_key.as_ref().map(|value| value.as_str()))
            .map_err(KnowledgeEngineError::from)?
            .json(&body)
            .map_err(KnowledgeEngineError::from)?
            .idempotent(true);
        let response = self
            .http
            .execute(context, request)
            .await
            .map_err(KnowledgeEngineError::from)?;
        response.json().map_err(KnowledgeEngineError::from)
    }

    async fn run_deepset_search(
        &self,
        context: &ProviderExecutionContext,
        workspace: Option<&str>,
        pipeline: &str,
        query: &str,
        top_k: u32,
    ) -> Result<Value, KnowledgeEngineError> {
        let workspace = workspace.ok_or_else(|| {
            KnowledgeEngineError::Validation(
                "Haystack cloud search requires workspace name".to_string(),
            )
        })?;
        let workspace = Self::validated_id("workspace name", workspace)?;
        let pipeline = Self::validated_id("pipeline name", pipeline)?;
        let url = format!(
            "{}/api/v1/workspaces/{}/pipelines/{}/search",
            self.config.base_url.trim_end_matches('/'),
            encoded_path_segment(workspace),
            encoded_path_segment(pipeline),
        );
        let request = ProviderHttpRequest::new(ProviderOperation::Search, Method::POST, url)
            .map_err(KnowledgeEngineError::from)?
            .optional_bearer_auth(self.config.api_key.as_ref().map(|value| value.as_str()))
            .map_err(KnowledgeEngineError::from)?
            .json(&json!({
                "queries": [query],
                "params": {
                    "retriever": { "top_k": top_k }
                }
            }))
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

fn map_record_to_hit(space_id: u64, record: HaystackDocumentRecord) -> KnowledgeEngineSearchHit {
    let local_document_id = format!("{}#{}", record.title, record.document_id);
    KnowledgeEngineSearchHit {
        document: KnowledgeEngineDocumentRef {
            document_id: format!("{space_id}/{local_document_id}"),
            title: record.title.clone(),
            source_uri: record.source_uri,
        },
        snippet: record.content,
        score: record.score,
    }
}

fn extract_haystack_documents(value: &Value) -> Vec<HaystackDocumentRecord> {
    let mut documents = Vec::new();
    collect_document_nodes(value, &mut documents);
    documents
}

fn collect_document_nodes(value: &Value, out: &mut Vec<HaystackDocumentRecord>) {
    match value {
        Value::Array(items) => {
            for item in items {
                collect_document_nodes(item, out);
            }
        }
        Value::Object(map) => {
            if looks_like_document(map) {
                if let Some(record) = parse_document_object(map) {
                    out.push(record);
                    return;
                }
            }

            for key in ["documents", "results", "answers", "retriever", "data"] {
                if let Some(nested) = map.get(key) {
                    collect_document_nodes(nested, out);
                }
            }
        }
        _ => {}
    }
}

fn looks_like_document(map: &serde_json::Map<String, Value>) -> bool {
    map.contains_key("content")
        || map.contains_key("page_content")
        || map.contains_key("text")
        || map.contains_key("document")
}

fn parse_document_object(map: &serde_json::Map<String, Value>) -> Option<HaystackDocumentRecord> {
    let content = map
        .get("content")
        .or_else(|| map.get("page_content"))
        .or_else(|| map.get("text"))
        .or_else(|| map.get("document"))
        .and_then(Value::as_str)
        .filter(|value| !value.is_empty())?
        .to_string();

    let meta = map
        .get("meta")
        .or_else(|| map.get("metadata"))
        .cloned()
        .unwrap_or(Value::Null);

    let document_id = map
        .get("id")
        .and_then(Value::as_str)
        .filter(|value| !value.is_empty())
        .map(str::to_string)
        .or_else(|| meta_string(&meta, "id"))
        .unwrap_or_else(|| chunk_id_from_content(&content));

    let title = meta_string(&meta, "title")
        .or_else(|| meta_string(&meta, "source"))
        .or_else(|| meta_string(&meta, "name"))
        .or_else(|| map.get("title").and_then(Value::as_str).map(str::to_string))
        .unwrap_or_else(|| document_id.clone());

    let score = map
        .get("score")
        .or_else(|| map.get("similarity"))
        .and_then(Value::as_f64);

    let source_uri = meta_string(&meta, "source")
        .or_else(|| meta_string(&meta, "url"))
        .or_else(|| meta_string(&meta, "file_path"));

    Some(HaystackDocumentRecord {
        document_id,
        title,
        content,
        score,
        source_uri,
    })
}

fn meta_string(meta: &Value, key: &str) -> Option<String> {
    meta.get(key)
        .and_then(Value::as_str)
        .filter(|value| !value.is_empty())
        .map(str::to_string)
}

pub fn chunk_id_from_content(content: &str) -> String {
    let digest = Sha256::digest(content.as_bytes());
    format!("{:x}", digest)[..16].to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn extracts_documents_from_hayhooks_wrapper_payload() {
        let payload = json!({
            "retriever": {
                "documents": [{
                    "id": "doc-1",
                    "content": "policy snippet",
                    "meta": {
                        "title": "Policy Doc",
                        "source": "file://policy.txt"
                    },
                    "score": 0.91
                }]
            }
        });
        let documents = extract_haystack_documents(&payload);

        assert_eq!(documents.len(), 1);
        assert_eq!(documents[0].document_id, "doc-1");
        assert_eq!(documents[0].title, "Policy Doc");
        assert_eq!(documents[0].content, "policy snippet");
    }

    #[test]
    fn extracts_documents_from_deepset_search_payload() {
        let payload = json!({
            "results": [{
                "documents": [{
                    "content": "cloud snippet",
                    "meta": { "title": "Cloud Doc" },
                    "id": "cloud-1"
                }]
            }]
        });
        let documents = extract_haystack_documents(&payload);

        assert_eq!(documents.len(), 1);
        assert_eq!(documents[0].title, "Cloud Doc");
    }
}
