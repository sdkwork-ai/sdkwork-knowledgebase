use sdkwork_utils_rust::is_blank;
use std::sync::Arc;

use crate::async_bridge::block_on_async;

use cloudrouter_open_sdk::{OpenAiEmbeddingsRequest, SdkworkAiClient, SdkworkError};

pub const DEFAULT_CLOUDROUTER_EMBEDDING_MODEL_ID: &str = "openai/text-embedding-3-small";
pub const CLOUDROUTER_EMBEDDINGS_METHOD: &str = "embeddings.create";

#[derive(Clone)]
pub struct CloudRouterEmbeddingClient {
    client: Arc<SdkworkAiClient>,
    default_model_id: String,
}

impl CloudRouterEmbeddingClient {
    pub fn new(client: Arc<SdkworkAiClient>) -> Self {
        Self {
            client,
            default_model_id: DEFAULT_CLOUDROUTER_EMBEDDING_MODEL_ID.to_string(),
        }
    }

    pub fn with_default_model_id(mut self, model_id: impl Into<String>) -> Self {
        self.default_model_id = model_id.into();
        self
    }

    pub fn embed_text(&self, input: &str, model_id: Option<&str>) -> Result<Vec<f32>, String> {
        let input = input.trim();
        if input.is_empty() {
            return Err("embedding input must not be blank".to_string());
        }

        let model = model_id
            .filter(|value| !is_blank(Some(value)))
            .unwrap_or(self.default_model_id.as_str())
            .to_string();
        let request = OpenAiEmbeddingsRequest {
            input: input.to_string(),
            model,
            ..Default::default()
        };

        let client = Arc::clone(&self.client);
        let response = block_on_async(async move { client.embeddings().create(&request).await })
            .map_err(|error| error.to_string())?
            .map_err(map_sdk_error)?;

        response
            .data
            .first()
            .map(|item| item.embedding.iter().map(|value| *value as f32).collect())
            .ok_or_else(|| {
                "cloud-router embeddings response did not include vector data".to_string()
            })
    }

    pub fn embed_texts(
        &self,
        inputs: &[String],
        model_id: Option<&str>,
    ) -> Result<Vec<Vec<f32>>, String> {
        inputs
            .iter()
            .map(|input| self.embed_text(input, model_id))
            .collect()
    }
}

pub fn serialize_embedding_vector(vector: &[f32]) -> Result<String, String> {
    serde_json::to_string(vector).map_err(|error| error.to_string())
}

pub fn deserialize_embedding_vector(payload: &str) -> Result<Vec<f32>, String> {
    serde_json::from_str(payload).map_err(|error| error.to_string())
}

pub fn cosine_similarity(left: &[f32], right: &[f32]) -> f64 {
    if left.is_empty() || right.is_empty() || left.len() != right.len() {
        return 0.0;
    }

    let mut dot = 0.0f64;
    let mut left_norm = 0.0f64;
    let mut right_norm = 0.0f64;
    for (a, b) in left.iter().zip(right.iter()) {
        let a = f64::from(*a);
        let b = f64::from(*b);
        dot += a * b;
        left_norm += a * a;
        right_norm += b * b;
    }

    if left_norm == 0.0 || right_norm == 0.0 {
        return 0.0;
    }

    dot / (left_norm.sqrt() * right_norm.sqrt())
}

fn map_sdk_error(error: SdkworkError) -> String {
    // Raw SDK error text is diagnostic-only (see cloud_router::map_sdk_error).
    tracing::warn!(error = %error, "cloud router embeddings sdk call failed");
    "cloud router request failed".to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cosine_similarity_is_one_for_identical_vectors() {
        let vector = vec![1.0, 0.0, 0.0];
        assert!((cosine_similarity(&vector, &vector) - 1.0).abs() < f64::EPSILON);
    }
}
