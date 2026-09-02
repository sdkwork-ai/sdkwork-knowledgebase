use crate::bounded_http_body::read_bounded_http_body;
use sdkwork_knowledgebase_contract::rag::KnowledgeFilter;
use sdkwork_utils_rust::{is_blank, sha256_hash};
use serde::Deserialize;
use thiserror::Error;

pub const INCLUDE_PUBLIC_WEB_METADATA_KEY: &str = "includePublicWeb";
pub const PUBLIC_WEB_TOP_K_METADATA_KEY: &str = "publicWebTopK";

const DEFAULT_PUBLIC_WEB_TOP_K: usize = 5;
const MAX_PUBLIC_WEB_TOP_K: usize = 8;
const MAX_QUERY_LEN: usize = 256;
/// Cap on provider response bodies so a misbehaving DuckDuckGo/searxng
/// instance cannot exhaust process memory.
const MAX_PUBLIC_SEARCH_RESPONSE_BYTES: usize = 512 * 1024;
/// One bounded retry for transient provider failures (connect/HTTP errors); a flapping
/// public search backend must not stall retrieval for longer than the request budget.
const PUBLIC_SEARCH_MAX_RETRIES: u32 = 1;
/// Total request budget (DNS, connect, TLS, response) for provider calls so a
/// hung upstream can never pin a retrieval/agent-chat task or DB connection.
const PUBLIC_SEARCH_TIMEOUT_SECS: u64 = 10;
/// Connect-phase budget, bounded below the total budget to keep DNS/TCP stalls
/// from consuming the whole request allowance.
const PUBLIC_SEARCH_CONNECT_TIMEOUT_SECS: u64 = 5;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PublicWebSearchHit {
    pub title: String,
    pub url: String,
    pub snippet: String,
}

#[derive(Debug, Error)]
pub enum PublicWebSearchError {
    #[error("public web search is disabled")]
    Disabled,
    #[error("public web search query is invalid: {0}")]
    InvalidQuery(String),
    #[error("public web search provider error: {0}")]
    Provider(String),
}

pub fn public_web_search_enabled() -> bool {
    match std::env::var("SDKWORK_KNOWLEDGEBASE_PUBLIC_WEB_SEARCH_ENABLED") {
        Ok(value) => {
            let normalized = value.trim().to_ascii_lowercase();
            normalized == "1" || normalized == "true" || normalized == "yes" || normalized == "on"
        }
        Err(_) => false,
    }
}

pub fn metadata_requests_public_web(metadata: &[KnowledgeFilter]) -> bool {
    metadata_flag(metadata, INCLUDE_PUBLIC_WEB_METADATA_KEY)
}

pub fn metadata_public_web_top_k(metadata: &[KnowledgeFilter]) -> usize {
    metadata
        .iter()
        .find(|entry| entry.key == PUBLIC_WEB_TOP_K_METADATA_KEY)
        .and_then(|entry| entry.value.parse::<usize>().ok())
        .map(|value| value.clamp(1, MAX_PUBLIC_WEB_TOP_K))
        .unwrap_or(DEFAULT_PUBLIC_WEB_TOP_K)
}

pub fn stable_web_hit_ids(url: &str) -> (u64, u64) {
    const WEB_DOCUMENT_ID_BASE: u64 = 9_000_000_000_000_000;
    let digest = sha256_hash(url.as_bytes());
    let suffix = u64::from_str_radix(&digest[..12.min(digest.len())], 16).unwrap_or(1);
    let document_id = WEB_DOCUMENT_ID_BASE | (suffix & 0x0FFF_FFFF_FFFF);
    (document_id, document_id.saturating_add(1))
}

pub async fn search_public_web(
    query: &str,
    top_k: usize,
) -> Result<Vec<PublicWebSearchHit>, PublicWebSearchError> {
    if !public_web_search_enabled() {
        return Err(PublicWebSearchError::Disabled);
    }
    let normalized = query.trim();
    if is_blank(Some(normalized)) {
        return Err(PublicWebSearchError::InvalidQuery(
            "query must not be blank".to_string(),
        ));
    }
    if normalized.chars().count() > MAX_QUERY_LEN {
        return Err(PublicWebSearchError::InvalidQuery(format!(
            "query must be at most {MAX_QUERY_LEN} characters"
        )));
    }

    let mut last_error =
        PublicWebSearchError::Provider("public web search provider is unavailable".to_string());
    for attempt in 0..=PUBLIC_SEARCH_MAX_RETRIES {
        let result = if let Some(base_url) = configured_searxng_base_url() {
            search_via_searxng(base_url, normalized, top_k).await
        } else {
            search_via_duckduckgo(normalized, top_k).await
        };
        match result {
            Ok(hits) => return Ok(hits),
            Err(error) => {
                last_error = error;
                if attempt < PUBLIC_SEARCH_MAX_RETRIES {
                    tokio::time::sleep(std::time::Duration::from_millis(250)).await;
                }
            }
        }
    }
    Err(last_error)
}

fn metadata_flag(metadata: &[KnowledgeFilter], key: &str) -> bool {
    metadata.iter().any(|entry| {
        entry.key == key
            && matches!(
                entry.value.trim().to_ascii_lowercase().as_str(),
                "1" | "true" | "yes" | "on"
            )
    })
}

fn configured_searxng_base_url() -> Option<String> {
    std::env::var("SDKWORK_KNOWLEDGEBASE_SEARXNG_BASE_URL")
        .ok()
        .map(|value| value.trim().trim_end_matches('/').to_string())
        .filter(|value| !value.is_empty())
}

fn public_search_http_client() -> Result<reqwest::Client, reqwest::Error> {
    reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(PUBLIC_SEARCH_TIMEOUT_SECS))
        .connect_timeout(std::time::Duration::from_secs(
            PUBLIC_SEARCH_CONNECT_TIMEOUT_SECS,
        ))
        .build()
}

async fn search_via_duckduckgo(
    query: &str,
    top_k: usize,
) -> Result<Vec<PublicWebSearchHit>, PublicWebSearchError> {
    let endpoint = format!(
        "https://api.duckduckgo.com/?q={}&format=json&no_redirect=1&no_html=1",
        urlencoding::encode(query)
    );
    let response = public_search_http_client()
        .map_err(|error| PublicWebSearchError::Provider(error.to_string()))?
        .get(endpoint)
        .header(reqwest::header::USER_AGENT, "sdkwork-knowledgebase/0.1")
        .send()
        .await
        .map_err(|error| PublicWebSearchError::Provider(error.to_string()))?;
    if !response.status().is_success() {
        return Err(PublicWebSearchError::Provider(format!(
            "duckduckgo returned HTTP {}",
            response.status()
        )));
    }

    let bytes = read_bounded_http_body(response, MAX_PUBLIC_SEARCH_RESPONSE_BYTES)
        .await
        .map_err(|error| PublicWebSearchError::Provider(error.to_string()))?;
    let payload = serde_json::from_slice::<DuckDuckGoResponse>(&bytes)
        .map_err(|error| PublicWebSearchError::Provider(error.to_string()))?;
    Ok(collect_duckduckgo_hits(payload, top_k))
}

async fn search_via_searxng(
    base_url: String,
    query: &str,
    top_k: usize,
) -> Result<Vec<PublicWebSearchHit>, PublicWebSearchError> {
    if !base_url.starts_with("http://") && !base_url.starts_with("https://") {
        return Err(PublicWebSearchError::Provider(
            "searxng base url must use http or https".to_string(),
        ));
    }

    let endpoint = format!(
        "{base_url}/search?format=json&q={}",
        urlencoding::encode(query)
    );
    let response = public_search_http_client()
        .map_err(|error| PublicWebSearchError::Provider(error.to_string()))?
        .get(endpoint)
        .header(reqwest::header::USER_AGENT, "sdkwork-knowledgebase/0.1")
        .send()
        .await
        .map_err(|error| PublicWebSearchError::Provider(error.to_string()))?;
    if !response.status().is_success() {
        return Err(PublicWebSearchError::Provider(format!(
            "searxng returned HTTP {}",
            response.status()
        )));
    }

    let bytes = read_bounded_http_body(response, MAX_PUBLIC_SEARCH_RESPONSE_BYTES)
        .await
        .map_err(|error| PublicWebSearchError::Provider(error.to_string()))?;
    let payload = serde_json::from_slice::<SearxngResponse>(&bytes)
        .map_err(|error| PublicWebSearchError::Provider(error.to_string()))?;

    let mut hits = Vec::new();
    for result in payload.results.unwrap_or_default() {
        let Some(url) = result.url.filter(|value| is_safe_public_url(value)) else {
            continue;
        };
        let title = result
            .title
            .filter(|value| !is_blank(Some(value.as_str())))
            .unwrap_or_else(|| url.clone());
        let snippet = result
            .content
            .unwrap_or_default()
            .split_whitespace()
            .collect::<Vec<_>>()
            .join(" ");
        if snippet.is_empty() {
            continue;
        }
        hits.push(PublicWebSearchHit {
            title,
            url,
            snippet: truncate_snippet(&snippet),
        });
        if hits.len() >= top_k {
            break;
        }
    }
    Ok(hits)
}

fn collect_duckduckgo_hits(payload: DuckDuckGoResponse, top_k: usize) -> Vec<PublicWebSearchHit> {
    let mut hits = Vec::new();

    if let (Some(abstract_text), Some(url)) = (
        payload
            .abstract_text
            .filter(|value| !is_blank(Some(value.as_str()))),
        payload
            .abstract_url
            .filter(|value| is_safe_public_url(value)),
    ) {
        let title = payload
            .heading
            .filter(|value| !is_blank(Some(value.as_str())))
            .or(payload.abstract_source)
            .unwrap_or_else(|| "Web summary".to_string());
        hits.push(PublicWebSearchHit {
            title,
            url,
            snippet: truncate_snippet(&abstract_text),
        });
    }

    for topic in flatten_duckduckgo_topics(payload.related_topics) {
        if hits.len() >= top_k {
            break;
        }
        let Some(url) = topic.first_url.filter(|value| is_safe_public_url(value)) else {
            continue;
        };
        let text = topic.text.unwrap_or_default();
        let snippet = text.split(" - ").nth(1).unwrap_or("").trim().to_string();
        if snippet.is_empty() {
            continue;
        }
        let title = text
            .split(" - ")
            .next()
            .map(str::trim)
            .filter(|value| !value.is_empty())
            .map(str::to_string)
            .unwrap_or_else(|| url.clone());
        if hits.iter().any(|hit| hit.url == url) {
            continue;
        }
        hits.push(PublicWebSearchHit {
            title,
            url,
            snippet: truncate_snippet(&snippet),
        });
    }

    hits
}

fn flatten_duckduckgo_topics(topics: Vec<DuckDuckGoTopic>) -> Vec<DuckDuckGoTopic> {
    let mut flattened = Vec::new();
    for topic in topics {
        if let Some(nested) = topic.topics {
            flattened.extend(nested);
        } else {
            flattened.push(topic);
        }
    }
    flattened
}

fn is_safe_public_url(url: &str) -> bool {
    let trimmed = url.trim();
    trimmed.starts_with("https://") || trimmed.starts_with("http://")
}

fn truncate_snippet(value: &str) -> String {
    let normalized = value.split_whitespace().collect::<Vec<_>>().join(" ");
    if normalized.chars().count() <= 220 {
        return normalized;
    }
    normalized.chars().take(217).collect::<String>() + "..."
}

#[derive(Debug, Deserialize)]
struct DuckDuckGoResponse {
    #[serde(rename = "Abstract")]
    abstract_text: Option<String>,
    #[serde(rename = "AbstractURL")]
    abstract_url: Option<String>,
    #[serde(rename = "AbstractSource")]
    abstract_source: Option<String>,
    #[serde(rename = "Heading")]
    heading: Option<String>,
    #[serde(rename = "RelatedTopics")]
    related_topics: Vec<DuckDuckGoTopic>,
}

#[derive(Debug, Deserialize)]
struct DuckDuckGoTopic {
    #[serde(rename = "Text")]
    text: Option<String>,
    #[serde(rename = "FirstURL")]
    first_url: Option<String>,
    #[serde(rename = "Topics")]
    topics: Option<Vec<DuckDuckGoTopic>>,
}

#[derive(Debug, Deserialize)]
struct SearxngResponse {
    results: Option<Vec<SearxngResult>>,
}

#[derive(Debug, Deserialize)]
struct SearxngResult {
    url: Option<String>,
    title: Option<String>,
    content: Option<String>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn metadata_requests_public_web_reads_flag() {
        let metadata = vec![KnowledgeFilter {
            key: INCLUDE_PUBLIC_WEB_METADATA_KEY.to_string(),
            value: "true".to_string(),
        }];
        assert!(metadata_requests_public_web(&metadata));
        assert_eq!(
            metadata_public_web_top_k(&metadata),
            DEFAULT_PUBLIC_WEB_TOP_K
        );
    }

    #[test]
    fn collect_duckduckgo_hits_deduplicates_urls() {
        let payload = DuckDuckGoResponse {
            abstract_text: Some("Summary text".to_string()),
            abstract_url: Some("https://example.com/a".to_string()),
            abstract_source: Some("Example".to_string()),
            heading: Some("Example heading".to_string()),
            related_topics: vec![DuckDuckGoTopic {
                text: Some("Example heading - More detail".to_string()),
                first_url: Some("https://example.com/a".to_string()),
                topics: None,
            }],
        };
        let hits = collect_duckduckgo_hits(payload, 5);
        assert_eq!(hits.len(), 1);
        assert_eq!(hits[0].url, "https://example.com/a");
    }

    #[test]
    fn stable_web_hit_ids_are_deterministic() {
        let left = stable_web_hit_ids("https://example.com/docs");
        let right = stable_web_hit_ids("https://example.com/docs");
        assert_eq!(left, right);
        assert!(left.0 >= 9_000_000_000_000_000);
    }
}
