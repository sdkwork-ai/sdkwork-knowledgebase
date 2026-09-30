//! Outbound target security for Provider HTTP execution.
//!
//! Equivalent to the ingest `web_link_fetch` SSRF protection: every provider call resolves
//! the origin hostname, rejects non-public (private/loopback/link-local/metadata/...)
//! addresses, and pins the resolved socket on the reqwest client so DNS rebinding cannot
//! redirect a request into an internal network after validation.

use reqwest::Url;
use std::net::{IpAddr, Ipv4Addr, Ipv6Addr, SocketAddr};

use crate::{ProviderError, ProviderErrorCategory, ProviderOperation};

/// Resolves every public socket for `url` or fails closed when every resolved address
/// is private or non-public.
///
/// All public addresses are returned (not just the first) so the caller can pin the
/// full resolved set and keep failover across a provider's DNS records.
pub(crate) async fn resolve_public_socket_addrs(
    url: &Url,
    connect_timeout: std::time::Duration,
) -> Result<Vec<SocketAddr>, ProviderError> {
    let port = url.port_or_known_default().unwrap_or(443);
    let host = url
        .host_str()
        .ok_or_else(|| target_error("provider URL host is required"))?;
    // Hostname-layer SSRF protection: loopback and metadata hostnames fail
    // closed before any DNS resolution happens.
    if is_blocked_hostname(host) {
        return Err(target_error(
            "provider URL hostname is not allowed (loopback, metadata, or internal)",
        ));
    }
    // Literal IP hosts are validated directly; domain hosts go through DNS with the same
    // public-address filter, so metadata endpoints and private ranges always fail closed.
    if let Ok(ip) = host.parse::<IpAddr>() {
        return validated_socket_addr(ip, port).map(|socket| vec![socket]);
    }

    let authority = format!("{host}:{port}");
    let addresses =
        tokio::time::timeout(connect_timeout, tokio::net::lookup_host(authority.as_str()))
            .await
            .map_err(|_| target_error("provider URL DNS lookup timed out"))?
            .map_err(|_| target_error("provider URL DNS lookup failed"))?;
    let public: Vec<SocketAddr> = addresses
        .filter(|address| !is_blocked_ip(address.ip()))
        .collect();
    if public.is_empty() {
        return Err(target_error(
            "provider URL resolves only to private or non-public addresses",
        ));
    }
    Ok(public)
}

fn validated_socket_addr(ip: IpAddr, port: u16) -> Result<SocketAddr, ProviderError> {
    if is_blocked_ip(ip) {
        Err(target_error(
            "provider URL must not target private or non-public addresses",
        ))
    } else {
        Ok(SocketAddr::new(ip, port))
    }
}

/// Loopback is exempted only when explicitly requested for wiremock integration fixtures.
/// Production and staging fail closed by default; this escape hatch must never be set in a
/// deployed environment.
const LOOPBACK_ALLOWANCE_ENV: &str = "SDKWORK_KNOWLEDGEBASE_PROVIDER_RUNTIME_ALLOW_LOOPBACK";

fn loopback_allowed_for_tests() -> bool {
    match std::env::var(LOOPBACK_ALLOWANCE_ENV) {
        Ok(value) => matches!(
            value.trim().to_ascii_lowercase().as_str(),
            "1" | "true" | "yes" | "on"
        ),
        Err(_) => false,
    }
}

pub(crate) fn is_blocked_hostname(host: &str) -> bool {
    let normalized = host.trim().trim_end_matches('.').to_ascii_lowercase();
    if loopback_allowed_for_tests()
        && matches!(normalized.as_str(), "localhost" | "127.0.0.1" | "::1")
    {
        return false;
    }
    matches!(
        normalized.as_str(),
        "localhost" | "metadata.google.internal" | "metadata" | "127.0.0.1" | "::1" | "0.0.0.0"
    ) || normalized.ends_with(".localhost")
        || normalized.ends_with(".local")
        || normalized.ends_with(".internal")
}

pub(crate) fn is_blocked_ip(ip: IpAddr) -> bool {
    match ip {
        IpAddr::V4(value) => is_blocked_ipv4(value),
        IpAddr::V6(value) => is_blocked_ipv6(value),
    }
}

fn is_blocked_ipv4(ip: Ipv4Addr) -> bool {
    if ip.is_loopback() && loopback_allowed_for_tests() {
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

fn target_error(message: &str) -> ProviderError {
    ProviderError::new(
        ProviderErrorCategory::InvalidTarget,
        ProviderOperation::Health,
        "unresolved",
        None,
        None,
        false,
        None,
        message,
    )
}

#[cfg(test)]
mod tests {
    use super::{is_blocked_hostname, is_blocked_ip, is_blocked_ipv4, is_blocked_ipv6};
    use std::net::{IpAddr, Ipv4Addr, Ipv6Addr};

    #[test]
    fn private_loopback_and_metadata_addresses_are_blocked() {
        // IPv4 loopback is exempted under `cfg(test)` for wiremock fixtures; the other
        // reserved ranges must stay blocked even in tests.
        assert!(is_blocked_ip(IpAddr::V4(Ipv4Addr::new(10, 0, 0, 1))));
        assert!(is_blocked_ip(IpAddr::V4(Ipv4Addr::new(172, 16, 0, 1))));
        assert!(is_blocked_ip(IpAddr::V4(Ipv4Addr::new(192, 168, 0, 1))));
        assert!(is_blocked_ip(IpAddr::V4(Ipv4Addr::new(169, 254, 169, 254))));
        assert!(is_blocked_ip(IpAddr::V4(Ipv4Addr::new(0, 0, 0, 0))));
        assert!(is_blocked_ip(IpAddr::V4(Ipv4Addr::new(100, 64, 0, 1))));
        assert!(is_blocked_ip(IpAddr::V4(Ipv4Addr::new(255, 255, 255, 255))));
        assert!(is_blocked_ip(IpAddr::V6(Ipv6Addr::LOCALHOST)));
        assert!(is_blocked_ip(IpAddr::V6(Ipv6Addr::UNSPECIFIED)));
    }

    #[test]
    fn public_addresses_are_allowed() {
        assert!(!is_blocked_ip(IpAddr::V4(Ipv4Addr::new(8, 8, 8, 8))));
        assert!(!is_blocked_ip(IpAddr::V4(Ipv4Addr::new(1, 1, 1, 1))));
        assert!(!is_blocked_ip(IpAddr::V6(Ipv6Addr::new(
            0x2606, 0x4700, 0, 0, 0, 0, 0, 0
        ))));
    }

    #[test]
    fn blocked_hostnames_are_rejected_before_dns() {
        // `localhost` is exempted under `cfg(test)` for wiremock fixtures.
        assert!(is_blocked_hostname("metadata.google.internal"));
        assert!(is_blocked_hostname("router.local"));
        assert!(is_blocked_hostname("k8s.internal"));
        assert!(!is_blocked_hostname("api.example.com"));
    }

    #[test]
    fn ipv4_block_ranges_match_ingest_web_link_protection() {
        // CGNAT 100.64/10, 192.0.0.0/24, 192.88.99.0/24, 198.18/15 and >= 240/4.
        assert!(is_blocked_ipv4(Ipv4Addr::new(100, 64, 0, 1)));
        assert!(is_blocked_ipv4(Ipv4Addr::new(192, 0, 0, 1)));
        assert!(is_blocked_ipv4(Ipv4Addr::new(192, 88, 99, 1)));
        assert!(is_blocked_ipv4(Ipv4Addr::new(198, 18, 0, 1)));
        assert!(is_blocked_ipv4(Ipv4Addr::new(240, 0, 0, 1)));
        assert!(!is_blocked_ipv4(Ipv4Addr::new(8, 8, 8, 8)));
    }

    #[test]
    fn ipv6_non_global_prefixes_are_blocked() {
        assert!(is_blocked_ipv6(Ipv6Addr::new(0xfe80, 0, 0, 0, 0, 0, 0, 1)));
        assert!(is_blocked_ipv6(Ipv6Addr::new(0xfc00, 0, 0, 0, 0, 0, 0, 1)));
        assert!(is_blocked_ipv6(Ipv6Addr::new(
            0x2001, 0x0db8, 0, 0, 0, 0, 0, 1
        )));
        assert!(!is_blocked_ipv6(Ipv6Addr::new(
            0x2606, 0x4700, 0, 0, 0, 0, 0, 0
        )));
    }
}
