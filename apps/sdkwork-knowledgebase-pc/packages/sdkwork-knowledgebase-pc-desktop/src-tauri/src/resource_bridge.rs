use base64::{engine::general_purpose::STANDARD, Engine as _};
use reqwest::header::LOCATION;
use reqwest::redirect::Policy;
use reqwest::Url;
use serde::Deserialize;
use std::net::{IpAddr, Ipv4Addr, Ipv6Addr};
use std::path::{Component, Path, PathBuf};
use std::sync::LazyLock;
use tauri::Manager;
use tokio::io::AsyncReadExt;
use tokio::sync::Semaphore;

use crate::export_save::MAX_EXPORT_FILE_BYTES;

const MAX_REMOTE_RESOURCE_BYTES: usize = 32 * 1024 * 1024;
const MAX_REMOTE_REDIRECTS: usize = 5;
const MAX_CONCURRENT_RESOURCE_IO: usize = 2;
static RESOURCE_IO_LIMIT: LazyLock<Semaphore> =
    LazyLock::new(|| Semaphore::new(MAX_CONCURRENT_RESOURCE_IO));

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FetchBinaryResourceRequest {
    url: String,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ReadLocalResourceRequest {
    path: String,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct OpenExternalUrlRequest {
    url: String,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SaveBinaryResourceRequest {
    suggested_name: String,
    data_base64: String,
}

#[derive(Debug, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct BinaryResourcePayload {
    data_base64: String,
    mime_type: Option<String>,
    byte_length: usize,
}

fn map_io_error(error: std::io::Error) -> String {
    format!("resource read failed: {error}")
}

fn normalize_local_path(raw: &str) -> Result<PathBuf, String> {
    let trimmed = raw.trim();
    if trimmed.is_empty() {
        return Err("local path is empty".to_string());
    }

    let without_scheme = trimmed.strip_prefix("file://").unwrap_or(trimmed).trim();

    let path = PathBuf::from(without_scheme);
    if !path.is_absolute() {
        return Err("only absolute local paths are allowed".to_string());
    }

    for component in path.components() {
        if matches!(component, Component::ParentDir) {
            return Err("parent directory traversal is not allowed".to_string());
        }
    }

    Ok(path)
}

fn validate_remote_url(raw: &str) -> Result<Url, String> {
    let parsed = Url::parse(raw.trim()).map_err(|error| format!("invalid URL: {error}"))?;
    match parsed.scheme() {
        "https" | "http" => {}
        _ => return Err("only HTTP(S) resource URLs are allowed".to_string()),
    }

    let host = parsed
        .host_str()
        .ok_or_else(|| "resource URL host is required".to_string())?;
    if is_blocked_hostname(host) {
        return Err("resource URL host is not allowed".to_string());
    }
    if let Some(ip) = parse_url_ip_host(host) {
        if is_blocked_ip(ip) {
            return Err("resource URL must not target private or loopback addresses".to_string());
        }
    }

    Ok(parsed)
}

fn validate_external_url(raw: &str) -> Result<Url, String> {
    let parsed = Url::parse(raw.trim()).map_err(|error| format!("invalid URL: {error}"))?;
    if parsed.scheme() == "https" || parsed.scheme() == "http" {
        Ok(parsed)
    } else {
        Err("only HTTP(S) URLs can be opened externally".to_string())
    }
}

async fn pinned_client_for_url(url: &Url) -> Result<reqwest::Client, String> {
    let host = url
        .host_str()
        .ok_or_else(|| "resource URL host is required".to_string())?;
    let port = url.port_or_known_default().unwrap_or(443);
    let mut builder = reqwest::Client::builder()
        .redirect(Policy::none())
        .timeout(std::time::Duration::from_secs(30));
    if let Some(ip) = parse_url_ip_host(host) {
        if is_blocked_ip(ip) {
            return Err("resource URL must not target private or loopback addresses".to_string());
        }
        return builder
            .build()
            .map_err(|error| format!("HTTP client init failed: {error}"));
    }

    let addresses: Vec<std::net::SocketAddr> = tokio::net::lookup_host((host, port))
        .await
        .map_err(|error| format!("resource URL DNS lookup failed: {error}"))?
        .collect();
    if addresses.is_empty() {
        return Err("resource URL host could not be resolved".to_string());
    }
    for address in &addresses {
        if is_blocked_ip(address.ip()) {
            return Err("resource URL resolves to a private or loopback address".to_string());
        }
    }
    builder = builder.resolve_to_addrs(host, &addresses);
    builder
        .build()
        .map_err(|error| format!("HTTP client init failed: {error}"))
}

fn is_blocked_hostname(host: &str) -> bool {
    let normalized = host.trim().trim_end_matches('.').to_ascii_lowercase();
    matches!(
        normalized.as_str(),
        "localhost" | "metadata.google.internal" | "metadata" | "127.0.0.1" | "::1" | "0.0.0.0"
    ) || normalized.ends_with(".localhost")
        || normalized.ends_with(".local")
        || normalized.ends_with(".internal")
}

fn parse_url_ip_host(host: &str) -> Option<IpAddr> {
    host.trim_start_matches('[')
        .trim_end_matches(']')
        .parse()
        .ok()
}

fn is_blocked_ip(ip: IpAddr) -> bool {
    match ip {
        IpAddr::V4(value) => is_blocked_ipv4(value),
        IpAddr::V6(value) => is_blocked_ipv6(value),
    }
}

fn is_blocked_ipv4(ip: Ipv4Addr) -> bool {
    let [first, second, third, _] = ip.octets();
    first == 0
        || first == 10
        || first == 127
        || (first == 100 && (second & 0xc0) == 64)
        || (first == 169 && second == 254)
        || (first == 172 && (16..=31).contains(&second))
        || (first == 192 && second == 168)
        || (first == 192 && second == 0 && matches!(third, 0 | 2))
        || (first == 198 && matches!(second, 18 | 19))
        || (first == 198 && second == 51 && third == 100)
        || (first == 203 && second == 0 && third == 113)
        || first >= 224
}

fn is_blocked_ipv6(ip: Ipv6Addr) -> bool {
    if let Some(mapped) = ip.to_ipv4_mapped() {
        return is_blocked_ipv4(mapped);
    }
    let segments = ip.segments();
    ip.is_loopback()
        || ip.is_unspecified()
        || segments[0] & 0xfe00 == 0xfc00
        || segments[0] & 0xffc0 == 0xfe80
        || segments[0] & 0xff00 == 0xff00
        || (segments[0] == 0x2001 && segments[1] == 0x0db8)
}

fn allowed_local_roots(app: &tauri::AppHandle) -> Result<Vec<PathBuf>, String> {
    let mut roots = Vec::new();
    if let Ok(app_data) = app.path().app_data_dir() {
        roots.push(app_data);
    }
    if let Ok(app_cache) = app.path().app_cache_dir() {
        roots.push(app_cache);
    }
    if let Ok(downloads) = app.path().download_dir() {
        roots.push(downloads);
    }
    if let Ok(documents) = app.path().document_dir() {
        roots.push(documents);
    }
    if roots.is_empty() {
        return Err("no allowed local resource roots are available".to_string());
    }
    Ok(roots)
}

fn validate_local_read_path(app: &tauri::AppHandle, path: &Path) -> Result<PathBuf, String> {
    let canonical = std::fs::canonicalize(path)
        .map_err(|error| format!("local resource not accessible: {error}"))?;
    let roots = allowed_local_roots(app)?;
    let allowed = roots.iter().any(|root| {
        std::fs::canonicalize(root)
            .ok()
            .is_some_and(|canonical_root| canonical.starts_with(&canonical_root))
    });
    if !allowed {
        return Err("local resource path is outside the desktop sandbox".to_string());
    }
    Ok(canonical)
}

fn payload_from_bytes(bytes: Vec<u8>, mime_type: Option<String>) -> BinaryResourcePayload {
    BinaryResourcePayload {
        byte_length: bytes.len(),
        data_base64: STANDARD.encode(bytes),
        mime_type,
    }
}

pub fn binary_payload_from_bytes(
    bytes: Vec<u8>,
    mime_type: Option<String>,
) -> BinaryResourcePayload {
    payload_from_bytes(bytes, mime_type)
}

#[tauri::command]
pub async fn fetch_binary_resource(
    request: FetchBinaryResourceRequest,
) -> Result<BinaryResourcePayload, String> {
    let _permit = RESOURCE_IO_LIMIT
        .acquire()
        .await
        .map_err(|_| "resource IO limiter is unavailable".to_string())?;
    let mut current_url = validate_remote_url(&request.url)?;

    let mut response = None;
    for redirect_count in 0..=MAX_REMOTE_REDIRECTS {
        // Pin each hop to the public addresses that were validated for that exact URL.
        let client = pinned_client_for_url(&current_url).await?;
        let next = client
            .get(current_url.clone())
            .send()
            .await
            .map_err(|error| format!("resource fetch failed: {error}"))?;

        if next.status().is_redirection() {
            if redirect_count == MAX_REMOTE_REDIRECTS {
                return Err("resource fetch exceeded redirect limit".to_string());
            }
            let location = next
                .headers()
                .get(LOCATION)
                .and_then(|value| value.to_str().ok())
                .ok_or_else(|| "redirect response missing Location header".to_string())?;
            current_url = current_url
                .join(location)
                .map_err(|error| format!("invalid redirect location: {error}"))?;
            current_url = validate_remote_url(current_url.as_str())?;
            continue;
        }

        response = Some(next);
        break;
    }

    let response =
        response.ok_or_else(|| "resource fetch did not return a response".to_string())?;
    if !response.status().is_success() {
        return Err(format!(
            "resource fetch failed with status {}",
            response.status()
        ));
    }

    let mime_type = response
        .headers()
        .get(reqwest::header::CONTENT_TYPE)
        .and_then(|value| value.to_str().ok())
        .map(str::to_string);
    if response
        .content_length()
        .is_some_and(|length| length > MAX_REMOTE_RESOURCE_BYTES as u64)
    {
        return Err(format!(
            "resource exceeds maximum allowed size of {MAX_REMOTE_RESOURCE_BYTES} bytes"
        ));
    }
    let initial_capacity = response
        .content_length()
        .unwrap_or(0)
        .min(MAX_REMOTE_RESOURCE_BYTES as u64) as usize;
    let mut response = response;
    let mut bytes = Vec::with_capacity(initial_capacity);
    while let Some(chunk) = response
        .chunk()
        .await
        .map_err(|error| format!("resource body read failed: {error}"))?
    {
        let next_len = bytes
            .len()
            .checked_add(chunk.len())
            .ok_or_else(|| "resource size overflow".to_string())?;
        if next_len > MAX_REMOTE_RESOURCE_BYTES {
            return Err(format!(
                "resource exceeds maximum allowed size of {MAX_REMOTE_RESOURCE_BYTES} bytes"
            ));
        }
        bytes.extend_from_slice(&chunk);
    }

    Ok(payload_from_bytes(bytes, mime_type))
}

#[tauri::command]
pub async fn read_local_resource(
    app: tauri::AppHandle,
    request: ReadLocalResourceRequest,
) -> Result<BinaryResourcePayload, String> {
    let _permit = RESOURCE_IO_LIMIT
        .acquire()
        .await
        .map_err(|_| "resource IO limiter is unavailable".to_string())?;
    let path = normalize_local_path(&request.path)?;
    if !path.exists() {
        return Err(format!("local resource not found: {}", path.display()));
    }
    if !path.is_file() {
        return Err(format!("local resource is not a file: {}", path.display()));
    }

    let path = validate_local_read_path(&app, &path)?;
    let file = tokio::fs::File::open(&path).await.map_err(map_io_error)?;
    let metadata = file.metadata().await.map_err(map_io_error)?;
    if metadata.len() as usize > MAX_REMOTE_RESOURCE_BYTES {
        return Err(format!(
            "local resource exceeds maximum allowed size of {MAX_REMOTE_RESOURCE_BYTES} bytes"
        ));
    }

    let mut bytes = Vec::with_capacity(metadata.len() as usize);
    let mut reader = file.take((MAX_REMOTE_RESOURCE_BYTES + 1) as u64);
    reader.read_to_end(&mut bytes).await.map_err(map_io_error)?;
    if bytes.len() > MAX_REMOTE_RESOURCE_BYTES {
        return Err(format!(
            "local resource exceeds maximum allowed size of {MAX_REMOTE_RESOURCE_BYTES} bytes"
        ));
    }
    let mime_type = match path.extension().and_then(|value| value.to_str()) {
        Some("pdf") => Some("application/pdf".to_string()),
        Some("png") => Some("image/png".to_string()),
        Some("jpg") | Some("jpeg") => Some("image/jpeg".to_string()),
        _ => None,
    };

    Ok(payload_from_bytes(bytes, mime_type))
}

#[tauri::command]
pub fn open_external_url(request: OpenExternalUrlRequest) -> Result<(), String> {
    let url = validate_external_url(&request.url)?;
    open::that(url.as_str()).map_err(|error| format!("open external URL failed: {error}"))
}

#[tauri::command]
pub async fn save_binary_resource(request: SaveBinaryResourceRequest) -> Result<bool, String> {
    use crate::export_save::{save_base64_to_export_path_async, ExportSaveMode};

    let _permit = RESOURCE_IO_LIMIT
        .acquire()
        .await
        .map_err(|_| "resource IO limiter is unavailable".to_string())?;
    let response = save_base64_to_export_path_async(
        request.data_base64,
        MAX_EXPORT_FILE_BYTES,
        "save payload",
        request.suggested_name,
        ExportSaveMode::Downloads,
    )
    .await?;
    Ok(response.saved)
}

#[cfg(test)] // WORKSPACE-PATH:allow-fixture-block: this module is the file's #[cfg(test)] unit-test fixture data
mod tests {
    use super::*;

    #[test]
    fn rejects_parent_dir_paths() {
        assert!(normalize_local_path("C:\\Users\\..\\secret.pdf").is_err());
    }

    #[test]
    fn accepts_https_urls() {
        assert!(validate_remote_url("https://example.com/file.pdf").is_ok());
    }

    #[test]
    fn rejects_loopback_urls() {
        assert!(validate_remote_url("http://127.0.0.1/file.pdf").is_err());
    }

    #[test]
    fn rejects_metadata_host() {
        assert!(validate_remote_url("http://metadata.google.internal/file").is_err());
    }

    #[test]
    fn rejects_non_public_address_ranges() {
        for address in [
            "100.64.0.1",
            "198.18.0.1",
            "224.0.0.1",
            "[fe80::1]",
            "[::ffff:127.0.0.1]",
        ] {
            assert!(
                validate_remote_url(&format!("http://{address}/file")).is_err(),
                "expected {address} to be rejected"
            );
        }
    }
}
