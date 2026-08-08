use std::net::{IpAddr, Ipv4Addr, Ipv6Addr, SocketAddr};

use crate::bounded_http_body::{
    read_bounded_http_body, redacted_reqwest_error_detail, BoundedHttpBodyError,
};
use reqwest::header::LOCATION;
use reqwest::redirect::Policy;
use reqwest::Url;
use sdkwork_utils_rust::is_blank;
use thiserror::Error;
use url::Host;

const MAX_WEB_LINK_BYTES: usize = 512 * 1024;
const MAX_WEB_LINK_REDIRECTS: usize = 5;

#[derive(Debug, Error)]
pub enum WebLinkFetchError {
    #[error("invalid web link request: {0}")]
    InvalidRequest(String),
    #[error("web link fetch failed: {0}")]
    Upstream(String),
}

pub async fn fetch_web_link_markdown(
    source_url: &str,
    title_hint: &str,
) -> Result<String, WebLinkFetchError> {
    let url = validate_public_http_url(source_url)?;

    let mut current_url = url;
    let mut response = None;
    for redirect_count in 0..=MAX_WEB_LINK_REDIRECTS {
        let next = send_pinned_get(&current_url).await?;

        if next.status().is_redirection() {
            if redirect_count == MAX_WEB_LINK_REDIRECTS {
                return Err(WebLinkFetchError::InvalidRequest(
                    "source_url exceeded redirect limit".to_string(),
                ));
            }
            let location = next
                .headers()
                .get(LOCATION)
                .and_then(|value| value.to_str().ok())
                .ok_or_else(|| {
                    WebLinkFetchError::Upstream(
                        "redirect response missing Location header".to_string(),
                    )
                })?;
            current_url = current_url.join(location).map_err(|error| {
                WebLinkFetchError::InvalidRequest(format!("invalid redirect location: {error}"))
            })?;
            validate_public_http_url(current_url.as_str())?;
            continue;
        }

        response = Some(next);
        break;
    }

    let response = response.ok_or_else(|| {
        WebLinkFetchError::Upstream("web link fetch did not return a response".to_string())
    })?;

    if !response.status().is_success() {
        return Err(WebLinkFetchError::Upstream(format!(
            "upstream returned HTTP {}",
            response.status()
        )));
    }

    let content_type = response
        .headers()
        .get(reqwest::header::CONTENT_TYPE)
        .and_then(|value| value.to_str().ok())
        .unwrap_or("")
        .to_ascii_lowercase();

    let bytes = read_bounded_http_body(response, MAX_WEB_LINK_BYTES)
        .await
        .map_err(map_web_body_error)?;

    let body = String::from_utf8_lossy(&bytes).trim().to_string();
    if is_blank(Some(body.as_str())) {
        return Err(WebLinkFetchError::InvalidRequest(
            "fetched web page is empty".to_string(),
        ));
    }

    if content_type.contains("text/html") || looks_like_html(&body) {
        return Ok(html_to_markdown(&body, current_url.as_str(), title_hint));
    }

    Ok(body)
}

async fn send_pinned_get(url: &Url) -> Result<reqwest::Response, WebLinkFetchError> {
    let socket = resolve_public_socket_addr(url).await?;
    let request = pinned_get_request(url, socket)?;

    request
        .header(
            reqwest::header::USER_AGENT,
            "SDKWork-Knowledgebase-Ingest/1.0",
        )
        .send()
        .await
        .map_err(|error| WebLinkFetchError::Upstream(redacted_reqwest_error_detail(&error)))
}

fn pinned_get_request(
    url: &Url,
    socket: SocketAddr,
) -> Result<reqwest::RequestBuilder, WebLinkFetchError> {
    let client_builder = reqwest::Client::builder()
        .redirect(Policy::none())
        .timeout(std::time::Duration::from_secs(20));
    let client_builder = match url.host() {
        Some(Host::Domain(host)) => client_builder.resolve(host, socket),
        Some(Host::Ipv4(_) | Host::Ipv6(_)) => client_builder,
        None => {
            return Err(WebLinkFetchError::InvalidRequest(
                "source_url host is required".to_string(),
            ));
        }
    };
    let client = client_builder
        .build()
        .map_err(|error| WebLinkFetchError::Upstream(redacted_reqwest_error_detail(&error)))?;
    Ok(client.get(url.clone()))
}

fn map_web_body_error(error: BoundedHttpBodyError) -> WebLinkFetchError {
    match error {
        BoundedHttpBodyError::TooLarge { .. } => WebLinkFetchError::InvalidRequest(format!(
            "web page exceeds {} KB import limit",
            MAX_WEB_LINK_BYTES / 1024
        )),
        BoundedHttpBodyError::Read { detail } => WebLinkFetchError::Upstream(detail),
    }
}

async fn resolve_public_socket_addr(url: &Url) -> Result<SocketAddr, WebLinkFetchError> {
    let port = url.port_or_known_default().unwrap_or(443);
    let host = match url.host() {
        Some(Host::Ipv4(ip)) => return validated_socket_addr(IpAddr::V4(ip), port),
        Some(Host::Ipv6(ip)) => return validated_socket_addr(IpAddr::V6(ip), port),
        Some(Host::Domain(host)) => host,
        None => {
            return Err(WebLinkFetchError::InvalidRequest(
                "source_url host is required".to_string(),
            ));
        }
    };

    let authority = format!("{host}:{port}");
    let mut addresses = tokio::net::lookup_host(authority.as_str())
        .await
        .map_err(|error| {
            WebLinkFetchError::InvalidRequest(format!("source_url DNS lookup failed: {error}"))
        })?;
    addresses
        .find(|address| !is_blocked_ip(address.ip()))
        .ok_or_else(|| {
            WebLinkFetchError::InvalidRequest(
                "source_url resolves only to private or non-public addresses".to_string(),
            )
        })
}

fn validated_socket_addr(ip: IpAddr, port: u16) -> Result<SocketAddr, WebLinkFetchError> {
    if is_blocked_ip(ip) {
        Err(WebLinkFetchError::InvalidRequest(
            "source_url must not target private or non-public addresses".to_string(),
        ))
    } else {
        Ok(SocketAddr::new(ip, port))
    }
}

pub fn validate_public_http_url(raw: &str) -> Result<Url, WebLinkFetchError> {
    let trimmed = raw.trim();
    if is_blank(Some(trimmed)) {
        return Err(WebLinkFetchError::InvalidRequest(
            "source_url is required".to_string(),
        ));
    }

    let url = Url::parse(trimmed).map_err(|error| {
        WebLinkFetchError::InvalidRequest(format!("invalid source_url: {error}"))
    })?;
    match url.scheme() {
        "http" | "https" => {}
        _ => {
            return Err(WebLinkFetchError::InvalidRequest(
                "source_url must use http or https".to_string(),
            ));
        }
    }

    match url.host() {
        Some(Host::Domain(host)) if is_blocked_hostname(host) => {
            return Err(WebLinkFetchError::InvalidRequest(
                "source_url host is not allowed".to_string(),
            ));
        }
        Some(Host::Ipv4(ip)) if is_blocked_ip(IpAddr::V4(ip)) => {
            return Err(WebLinkFetchError::InvalidRequest(
                "source_url must not target private or non-public addresses".to_string(),
            ));
        }
        Some(Host::Ipv6(ip)) if is_blocked_ip(IpAddr::V6(ip)) => {
            return Err(WebLinkFetchError::InvalidRequest(
                "source_url must not target private or non-public addresses".to_string(),
            ));
        }
        Some(_) => {}
        None => {
            return Err(WebLinkFetchError::InvalidRequest(
                "source_url host is required".to_string(),
            ));
        }
    }

    Ok(url)
}

fn looks_like_html(body: &str) -> bool {
    let lower = body.to_ascii_lowercase();
    lower.starts_with("<!doctype") || lower.starts_with("<html")
}

fn is_blocked_hostname(host: &str) -> bool {
    let normalized = host.trim().trim_end_matches('.').to_ascii_lowercase();
    #[cfg(test)]
    if matches!(normalized.as_str(), "localhost" | "127.0.0.1" | "::1") {
        return false;
    }
    matches!(
        normalized.as_str(),
        "localhost" | "metadata.google.internal" | "metadata" | "127.0.0.1" | "::1" | "0.0.0.0"
    ) || normalized.ends_with(".localhost")
        || normalized.ends_with(".local")
        || normalized.ends_with(".internal")
}

fn is_blocked_ip(ip: IpAddr) -> bool {
    match ip {
        IpAddr::V4(value) => is_blocked_ipv4(value),
        IpAddr::V6(value) => is_blocked_ipv6(value),
    }
}

fn is_blocked_ipv4(ip: Ipv4Addr) -> bool {
    #[cfg(test)]
    if ip.is_loopback() {
        return false;
    }
    let [first, second, third, _] = ip.octets();
    ip.is_unspecified()
        || ip.is_private()
        || ip.is_loopback()
        || ip.is_link_local()
        || ip.is_broadcast()
        || ip.is_documentation()
        || ip.is_multicast()
        || first == 0
        || (first == 100 && (64..=127).contains(&second))
        || (first == 192 && second == 0 && third == 0)
        || (first == 192 && second == 88 && third == 99)
        || (first == 198 && matches!(second, 18 | 19))
        || first >= 240
}

fn is_blocked_ipv6(ip: Ipv6Addr) -> bool {
    let segments = ip.segments();
    let is_global_unicast_prefix = segments[0] & 0xe000 == 0x2000;
    let is_ietf_special = segments[0] == 0x2001 && segments[1] <= 0x01ff;
    let is_documentation = segments[0] == 0x2001 && segments[1] == 0x0db8;
    let is_6to4 = segments[0] == 0x2002;
    let is_extended_documentation = segments[0] == 0x3fff && segments[1] & 0xfff0 == 0;

    !is_global_unicast_prefix
        || is_ietf_special
        || is_documentation
        || is_6to4
        || is_extended_documentation
}

fn html_to_markdown(html: &str, page_url: &str, title_hint: &str) -> String {
    let without_scripts = strip_tag_blocks(html, "script");
    let without_styles = strip_tag_blocks(&without_scripts, "style");
    let without_noscript = strip_tag_blocks(&without_styles, "noscript");
    let title = extract_html_title(&without_noscript)
        .filter(|value| !value.is_empty())
        .unwrap_or_else(|| title_hint.trim().to_string());
    let article = extract_tag_inner(&without_noscript, "article")
        .or_else(|| extract_tag_inner(&without_noscript, "main"))
        .or_else(|| extract_tag_inner(&without_noscript, "body"))
        .unwrap_or_else(|| without_noscript.clone());
    let text = strip_html_tags(&article)
        .lines()
        .map(str::trim)
        .filter(|line| !line.is_empty())
        .collect::<Vec<_>>()
        .join("\n\n");

    if text.is_empty() {
        return format!("# {title}\n\nSource: {page_url}\n");
    }

    format!("# {title}\n\nSource: {page_url}\n\n{text}")
}

fn strip_tag_blocks(input: &str, tag: &str) -> String {
    let mut output = input.to_string();
    let open = format!("<{tag}");
    let close = format!("</{tag}>");
    while let Some(start) = output.to_ascii_lowercase().find(&open) {
        let Some(relative_end) = output[start..].to_ascii_lowercase().find(&close) else {
            output.replace_range(start.., "");
            break;
        };
        let end = start + relative_end + close.len();
        output.replace_range(start..end, "");
    }
    output
}

fn extract_html_title(html: &str) -> Option<String> {
    extract_tag_inner(html, "title").map(|value| strip_html_tags(&value).trim().to_string())
}

fn extract_tag_inner(html: &str, tag: &str) -> Option<String> {
    let lower = html.to_ascii_lowercase();
    let open = format!("<{tag}");
    let start = lower.find(&open)?;
    let open_end = lower[start..].find('>')? + start + 1;
    let close = format!("</{tag}>");
    let relative_end = lower[open_end..].find(&close)?;
    Some(html[open_end..open_end + relative_end].to_string())
}

fn strip_html_tags(input: &str) -> String {
    let mut output = String::with_capacity(input.len());
    let mut in_tag = false;
    for ch in input.chars() {
        match ch {
            '<' => in_tag = true,
            '>' => in_tag = false,
            _ if !in_tag => {
                output.push(ch);
            }
            _ => {}
        }
    }
    output
        .replace("&nbsp;", " ")
        .replace("&amp;", "&")
        .replace("&lt;", "<")
        .replace("&gt;", ">")
        .replace("&quot;", "\"")
        .replace("&#39;", "'")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn validate_public_http_url_rejects_private_hosts() {
        assert!(validate_public_http_url("http://10.0.0.1/test").is_err());
        assert!(validate_public_http_url("http://192.168.1.1/test").is_err());
        assert!(validate_public_http_url("http://100.64.0.1/test").is_err());
        assert!(validate_public_http_url("http://198.18.0.1/test").is_err());
        assert!(validate_public_http_url("http://224.0.0.1/test").is_err());
        assert!(validate_public_http_url("http://[2001:db8::1]/test").is_err());
        assert!(validate_public_http_url("http://metadata.google.internal/test").is_err());
        assert!(validate_public_http_url("ftp://example.com/test").is_err());
        assert!(validate_public_http_url("https://example.com/article").is_ok());
    }

    #[test]
    fn pinned_request_url_preserves_https_hostname_for_tls_identity() {
        let url = Url::parse("https://example.com/article").expect("valid URL");
        let socket = SocketAddr::new(IpAddr::V4(Ipv4Addr::new(93, 184, 216, 34)), 443);

        let request = pinned_get_request(&url, socket)
            .expect("request builder")
            .build()
            .expect("request");

        assert_eq!(request.url().host_str(), Some("example.com"));
    }

    #[test]
    fn html_to_markdown_extracts_title_and_body() {
        let html = "<html><head><title>Example</title></head><body><article><p>Hello world</p></article></body></html>";
        let markdown = html_to_markdown(html, "https://example.com/article", "Fallback");
        assert!(markdown.contains("# Example"));
        assert!(markdown.contains("Hello world"));
        assert!(markdown.contains("https://example.com/article"));
    }

    #[tokio::test]
    async fn fetch_web_link_markdown_downloads_html_from_public_url() {
        use wiremock::matchers::{method, path};
        use wiremock::{Mock, MockServer, ResponseTemplate};

        let mock_server = MockServer::start().await;
        Mock::given(method("GET"))
            .and(path("/article"))
            .respond_with(ResponseTemplate::new(200).set_body_string(
                "<html><head><title>Example Article</title></head><body><article><p>Hello from web</p></article></body></html>",
            ))
            .mount(&mock_server)
            .await;

        let markdown =
            fetch_web_link_markdown(&format!("{}/article", mock_server.uri()), "Fallback title")
                .await
                .expect("fetch should succeed");

        assert!(markdown.contains("Hello from web"));
        assert!(markdown.contains("Example Article"));
    }

    #[tokio::test]
    async fn fetch_web_link_markdown_rejects_declared_oversize_before_reading_body() {
        let listener = tokio::net::TcpListener::bind((Ipv4Addr::LOCALHOST, 0))
            .await
            .expect("bind test server");
        let address = listener.local_addr().expect("test server address");
        let server = tokio::spawn(async move {
            let (socket, _) = listener.accept().await.expect("accept request");
            let mut request = Vec::new();
            let mut buffer = [0_u8; 1024];
            while !request.windows(4).any(|window| window == b"\r\n\r\n") {
                socket.readable().await.expect("socket readable");
                match socket.try_read(&mut buffer) {
                    Ok(0) => break,
                    Ok(count) => request.extend_from_slice(&buffer[..count]),
                    Err(error) if error.kind() == std::io::ErrorKind::WouldBlock => continue,
                    Err(error) => panic!("read request: {error}"),
                }
            }
            let response = format!(
                "HTTP/1.1 200 OK\r\nContent-Type: text/plain\r\nContent-Length: {}\r\nConnection: close\r\n\r\n",
                MAX_WEB_LINK_BYTES + 1
            );
            let mut written = 0;
            while written < response.len() {
                socket.writable().await.expect("socket writable");
                match socket.try_write(&response.as_bytes()[written..]) {
                    Ok(0) => panic!("socket closed while writing response headers"),
                    Ok(count) => written += count,
                    Err(error) if error.kind() == std::io::ErrorKind::WouldBlock => continue,
                    Err(error) => panic!("write response headers: {error}"),
                }
            }
        });

        let error = fetch_web_link_markdown(&format!("http://{address}/oversize"), "Oversize")
            .await
            .expect_err("declared oversized body must be rejected");
        server.await.expect("test server task");

        assert!(
            matches!(error, WebLinkFetchError::InvalidRequest(_)),
            "unexpected error: {error:?}"
        );
        assert!(error.to_string().contains("import limit"));
    }
}
