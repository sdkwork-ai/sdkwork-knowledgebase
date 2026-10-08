//! Weaviate GraphQL / REST HTTP client (adapter-local).

use reqwest::Method;
use sdkwork_knowledgebase_contract::knowledge_engine::{
    KnowledgeEngineDocument, KnowledgeEngineDocumentRef, KnowledgeEngineError,
    KnowledgeEngineSearchHit, KnowledgeEngineSearchResult,
};
use sdkwork_knowledgebase_provider_runtime::{
    encoded_path_segment, engine_provider_error, is_path_segment_id, ProviderErrorCategory,
    ProviderExecutionContext, ProviderHttpRequest, ProviderOperation, ProviderRuntime,
};
use serde::Deserialize;
use serde_json::Value;

use crate::config::WeaviateConnectorConfig;
use crate::WEAVIATE_IMPLEMENTATION_ID;

#[derive(Clone)]
pub struct WeaviateApiClient {
    config: WeaviateConnectorConfig,
    http: ProviderRuntime,
}

impl WeaviateApiClient {
    pub fn new(config: WeaviateConnectorConfig) -> Result<Self, KnowledgeEngineError> {
        // Identifiers are spliced into GraphQL text and URL paths, so they must be
        // validated at construction instead of escaped per request.
        validate_graphql_identifier("Weaviate title property", &config.title_property)?;
        validate_graphql_identifier("Weaviate content property", &config.content_property)?;
        if let Some(class_name) = config.default_class_name.as_deref() {
            validate_graphql_identifier("Weaviate class name", class_name)?;
        }
        let http = ProviderRuntime::for_base_url_with_private_targets(
            &config.base_url,
            config.allow_private_network,
        )
        .map_err(KnowledgeEngineError::from)?;
        Ok(Self { config, http })
    }

    fn health_context(&self) -> ProviderExecutionContext {
        ProviderExecutionContext::for_system_health(WEAVIATE_IMPLEMENTATION_ID)
    }

    pub async fn connector_health(&self) -> Result<(), KnowledgeEngineError> {
        let url = format!(
            "{}/v1/.well-known/ready",
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

    pub async fn near_text_search(
        &self,
        context: &ProviderExecutionContext,
        space_id: u64,
        class_name: &str,
        query: &str,
        top_k: u32,
    ) -> Result<KnowledgeEngineSearchResult, KnowledgeEngineError> {
        validate_graphql_identifier("Weaviate class name", class_name)?;
        // User query text is parameterized through GraphQL variables; only the
        // validated identifiers are spliced into the query document.
        let graphql = format!(
            "query NearTextSearch($concepts: [String!]!, $limit: Int!) {{ Get {{ {class_name}(nearText: {{ concepts: $concepts }}, limit: $limit) {{ {title} {content} _additional {{ id certainty }} }} }} }}",
            class_name = class_name,
            title = self.config.title_property,
            content = self.config.content_property,
        );
        let url = format!("{}/v1/graphql", self.config.base_url.trim_end_matches('/'));
        let request = ProviderHttpRequest::new(ProviderOperation::Search, Method::POST, url)
            .map_err(KnowledgeEngineError::from)?
            .optional_bearer_auth(self.config.api_key.as_ref().map(|value| value.as_str()))
            .map_err(KnowledgeEngineError::from)?
            .json(&serde_json::json!({
                "query": graphql,
                "variables": {
                    "concepts": [query],
                    "limit": top_k,
                },
            }))
            .map_err(KnowledgeEngineError::from)?
            .idempotent(true);
        let response = self
            .http
            .execute(context, request)
            .await
            .map_err(KnowledgeEngineError::from)?;
        let payload: WeaviateGraphqlResponse =
            response.json().map_err(KnowledgeEngineError::from)?;

        if let Some(errors) = payload.errors.filter(|errors| !errors.is_empty()) {
            // Raw GraphQL error text is diagnostic-only and must not reach API errors.
            let detail = errors
                .into_iter()
                .filter_map(|error| error.message)
                .collect::<Vec<_>>()
                .join("; ");
            tracing::warn!(implementation_id = WEAVIATE_IMPLEMENTATION_ID, error = %detail, "engine request failed");
            return Err(engine_provider_error(
                ProviderOperation::Search,
                WEAVIATE_IMPLEMENTATION_ID,
                ProviderErrorCategory::InvalidResponse,
                "weaviate query rejected",
            ));
        }

        let objects = payload
            .data
            .and_then(|data| data.get)
            .and_then(|get| get.get(class_name).cloned())
            .and_then(|value| value.as_array().cloned())
            .unwrap_or_default();

        let hits = objects
            .into_iter()
            .filter_map(|object| {
                map_object_to_hit(
                    space_id,
                    object,
                    &self.config.title_property,
                    &self.config.content_property,
                )
            })
            .collect();

        Ok(KnowledgeEngineSearchResult {
            implementation_id: WEAVIATE_IMPLEMENTATION_ID.to_string(),
            hits,
        })
    }

    pub async fn get_object(
        &self,
        context: &ProviderExecutionContext,
        class_name: &str,
        object_id: &str,
    ) -> Result<KnowledgeEngineDocument, KnowledgeEngineError> {
        validate_graphql_identifier("Weaviate class name", class_name)?;
        if !is_path_segment_id(object_id) {
            return Err(KnowledgeEngineError::Validation(
                "Weaviate object id must match [A-Za-z0-9._:-]{1,256}".to_string(),
            ));
        }
        let url = format!(
            "{}/v1/objects/{}/{}",
            self.config.base_url.trim_end_matches('/'),
            encoded_path_segment(class_name),
            encoded_path_segment(object_id),
        );
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
        let payload: WeaviateObjectResponse =
            response.json().map_err(KnowledgeEngineError::from)?;

        let properties = payload.properties.unwrap_or(Value::Null);
        let title = property_string(&properties, &self.config.title_property)
            .unwrap_or_else(|| object_id.to_string());
        let content =
            property_string(&properties, &self.config.content_property).unwrap_or_default();

        Ok(KnowledgeEngineDocument {
            document_id: format!("{title}#{object_id}"),
            title,
            content,
            source_uri: properties
                .get("source")
                .or_else(|| properties.get("url"))
                .and_then(Value::as_str)
                .filter(|value| !value.is_empty())
                .map(str::to_string),
        })
    }
}

/// GraphQL and URL-path identifiers (class names, property names) are restricted
/// to `^[A-Za-z0-9_]{1,64}$` so splicing them into a query document cannot change
/// its structure.
fn validate_graphql_identifier(kind: &str, value: &str) -> Result<(), KnowledgeEngineError> {
    let valid = !value.is_empty()
        && value.len() <= 64
        && value
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || byte == b'_');
    if valid {
        Ok(())
    } else {
        Err(KnowledgeEngineError::Validation(format!(
            "{kind} must match [A-Za-z0-9_]{{1,64}}"
        )))
    }
}

fn map_object_to_hit(
    space_id: u64,
    object: Value,
    title_property: &str,
    content_property: &str,
) -> Option<KnowledgeEngineSearchHit> {
    let object_id = object
        .get("_additional")
        .and_then(|additional| additional.get("id"))
        .and_then(Value::as_str)?
        .to_string();
    let title = property_string(&object, title_property).unwrap_or_else(|| object_id.clone());
    let snippet = property_string(&object, content_property).unwrap_or_default();
    let score = object
        .get("_additional")
        .and_then(|additional| additional.get("certainty"))
        .and_then(Value::as_f64);
    let local_document_id = format!("{title}#{object_id}");

    Some(KnowledgeEngineSearchHit {
        document: KnowledgeEngineDocumentRef {
            document_id: format!("{space_id}/{local_document_id}"),
            title: title.clone(),
            source_uri: object
                .get("source")
                .or_else(|| object.get("url"))
                .and_then(Value::as_str)
                .filter(|value| !value.is_empty())
                .map(str::to_string),
        },
        snippet,
        score,
    })
}

fn property_string(object: &Value, property: &str) -> Option<String> {
    object
        .get(property)
        .and_then(Value::as_str)
        .filter(|value| !value.is_empty())
        .map(str::to_string)
}

#[derive(Debug, Deserialize)]
struct WeaviateGraphqlResponse {
    data: Option<WeaviateGraphqlData>,
    #[serde(default)]
    errors: Option<Vec<WeaviateGraphqlError>>,
}

#[derive(Debug, Deserialize)]
struct WeaviateGraphqlData {
    #[serde(rename = "Get")]
    get: Option<serde_json::Map<String, Value>>,
}

#[derive(Debug, Deserialize)]
struct WeaviateGraphqlError {
    message: Option<String>,
}

#[derive(Debug, Deserialize)]
struct WeaviateObjectResponse {
    properties: Option<Value>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn maps_weaviate_graphql_object_to_scoped_hit() {
        let object = serde_json::json!({
            "title": "Policy Doc",
            "content": "policy snippet",
            "_additional": {
                "id": "rec-1",
                "certainty": 0.91
            }
        });
        let hit = map_object_to_hit(4, object, "title", "content").expect("hit");

        assert_eq!(hit.document.document_id, "4/Policy Doc#rec-1");
        assert_eq!(hit.snippet, "policy snippet");
        assert_eq!(hit.score, Some(0.91));
    }

    #[test]
    fn graphql_identifier_validation_rejects_structural_characters() {
        assert!(validate_graphql_identifier("class name", "KnowledgeChunk").is_ok());
        assert!(validate_graphql_identifier("class name", "Chunk_2").is_ok());
        assert!(validate_graphql_identifier("class name", "").is_err());
        assert!(validate_graphql_identifier("class name", "Chunk{").is_err());
        assert!(validate_graphql_identifier("class name", "Chunk(Id").is_err());
        assert!(validate_graphql_identifier("class name", "a b").is_err());
        assert!(validate_graphql_identifier("class name", &"a".repeat(65)).is_err());
    }

    #[test]
    fn graphql_query_splices_only_validated_identifiers_and_parameterizes_query() {
        let class_name = "KnowledgeChunk";
        let graphql = format!(
            "query NearTextSearch($concepts: [String!]!, $limit: Int!) {{ Get {{ {class_name}(nearText: {{ concepts: $concepts }}, limit: $limit) {{ {title} {content} _additional {{ id certainty }} }} }} }}",
            class_name = class_name,
            title = "title",
            content = "content",
        );
        assert!(graphql.contains("$concepts"));
        assert!(graphql.contains("KnowledgeChunk(nearText: { concepts: $concepts }"));
        assert!(!graphql.contains("\\\""));
    }
}
