//! Outbound target security for Provider HTTP execution.
//!
//! Equivalent to the ingest `web_link_fetch` SSRF protection: every provider call resolves
//! the origin hostname, rejects non-public (private/loopback/link-local/metadata/...)
//! addresses unless the runtime configuration explicitly opted into private-network
//! targets, and pins the resolved socket on the reqwest client so DNS rebinding cannot
//! redirect a request into a different network after validation.

use reqwest::Url;
use std::net::{IpAddr, Ipv4Addr, Ipv6Addr, SocketAddr};

use crate::{ProviderError, ProviderErrorCategory, ProviderOperation};

/// Fail-closed private-target policy, decided once at configuration time.
///
/// The default (`PrivateTargetPolicy::new(false)`) rejects every non-public target —
/// loopback, RFC1918 private ranges, link-local/metadata, and the other reserved
/// blocks — with the typed `InvalidTarget` error. Passing `true` is an explicit
/// operator decision for self-hosted engine deployments whose production placement is
/// a trusted private network segment (`<ENGINE>_ALLOW_PRIVATE_NETWORK=1`): the
/// operator owns that trust boundary, and DNS-resolution socket pinning still applies
/// so a rebinding record cannot move traffic off the validated target set.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct PrivateTargetPolicy {
    pub allow_private_network_targets: bool,
}

impl PrivateTargetPolicy {
    pub(crate) fn new(allow_private_network_targets: bool) -> Self {
        Self {
            allow_private_network_targets,
        }
    }
}

/// Resolves every socket address for `url` that satisfies `policy`, or fails closed
/// when every resolved address is blocked.
///
/// All matching addresses are returned (not just the first) so the caller can pin the
/// full resolved set and keep failover across a provider's DNS records.
pub(crate) async fn resolve_pinned_socket_addrs(
    url: &Url,
    connect_timeout: std::time::Duration,
    policy: PrivateTargetPolicy,
) -> Result<Vec<SocketAddr>, ProviderError> {
    let port = url.port_or_known_default().unwrap_or(443);
    let host = url
        .host_str()
        .ok_or_else(|| target_error("provider URL host is required"))?;
    // Hostname-layer SSRF protection: loopback and metadata hostnames fail
    // closed before any DNS resolution happens unless the deployment explicitly
    // opted into private-network targets.
    if is_blocked_hostname(host, policy) {
        return Err(target_error(
            "provider URL hostname is not allowed (loopback, metadata, or internal)",
        ));
    }
    // Literal IP hosts are validated directly; domain hosts go through DNS with the same
    // policy filter, so out-of-policy targets always fail closed.
    if let Ok(ip) = host.parse::<IpAddr>() {
        return validated_socket_addr(ip, port, policy).map(|socket| vec![socket]);
    }

    let authority = format!("{host}:{port}");
    let addresses =
        tokio::time::timeout(connect_timeout, tokio::net::lookup_host(authority.as_str()))
            .await
            .map_err(|_| target_error("provider URL DNS lookup timed out"))?
            .map_err(|_| target_error("provider URL DNS lookup failed"))?;
    let allowed: Vec<SocketAddr> = addresses
        .filter(|address| !is_blocked_ip(address.ip(), policy))
        .collect();
    if allowed.is_empty() {
        return Err(target_error(
            "provider URL resolves only to private or non-public addresses",
        ));
    }
    Ok(allowed)
}

fn validated_socket_addr(
    ip: IpAddr,
    port: u16,
    policy: PrivateTargetPolicy,
) -> Result<SocketAddr, ProviderError> {
    if is_blocked_ip(ip, policy) {
        Err(target_error(
            "provider URL must not target private or non-public addresses",
        ))
    } else {
        Ok(SocketAddr::new(ip, port))
    }
}

pub(crate) fn is_blocked_hostname(host: &str, policy: PrivateTargetPolicy) -> bool {
    if policy.allow_private_network_targets {
        // Explicit opt-in: the deployment trusts its private network segment for
        // engine targets (operator responsibility), so loopback and internal DNS
        // names are valid self-hosted engine hosts. DNS socket pinning still applies.
        return false;
    }
    let normalized = host.trim().trim_end_matches('.').to_ascii_lowercase();
    matches!(
        normalized.as_str(),
        "localhost" | "metadata.google.internal" | "metadata" | "127.0.0.1" | "::1" | "0.0.0.0"
    ) || normalized.ends_with(".localhost")
        || normalized.ends_with(".local")
        || normalized.ends_with(".internal")
}

pub(crate) fn is_blocked_ip(ip: IpAddr, policy: PrivateTargetPolicy) -> bool {
    match ip {
        IpAddr::V4(value) => is_blocked_ipv4(value, policy),
        IpAddr::V6(value) => is_blocked_ipv6(value, policy),
    }
}

fn is_blocked_ipv4(ip: Ipv4Addr, policy: PrivateTargetPolicy) -> bool {
    if policy.allow_private_network_targets {
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

fn is_blocked_ipv6(ip: Ipv6Addr, policy: PrivateTargetPolicy) -> bool {
    if policy.allow_private_network_targets {
        return false;
    }
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
    use super::{
        is_blocked_hostname, is_blocked_ip, is_blocked_ipv4, is_blocked_ipv6, PrivateTargetPolicy,
    };
    use std::net::{IpAddr, Ipv4Addr, Ipv6Addr};

    #[test]
    fn private_loopback_and_metadata_addresses_are_blocked_by_default() {
        // Without an explicit opt-in, loopback, RFC1918 private ranges, and the other
        // reserved ranges fail closed.
        let policy = PrivateTargetPolicy::new(false);
        assert!(is_blocked_ip(
            IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1)),
            policy
        ));
        assert!(is_blocked_ip(
            IpAddr::V4(Ipv4Addr::new(10, 0, 0, 1)),
            policy
        ));
        assert!(is_blocked_ip(
            IpAddr::V4(Ipv4Addr::new(172, 16, 0, 1)),
            policy
        ));
        assert!(is_blocked_ip(
            IpAddr::V4(Ipv4Addr::new(192, 168, 0, 1)),
            policy
        ));
        assert!(is_blocked_ip(
            IpAddr::V4(Ipv4Addr::new(169, 254, 169, 254)),
            policy
        ));
        assert!(is_blocked_ip(IpAddr::V4(Ipv4Addr::new(0, 0, 0, 0)), policy));
        assert!(is_blocked_ip(
            IpAddr::V4(Ipv4Addr::new(100, 64, 0, 1)),
            policy
        ));
        assert!(is_blocked_ip(
            IpAddr::V4(Ipv4Addr::new(255, 255, 255, 255)),
            policy
        ));
        assert!(is_blocked_ip(IpAddr::V6(Ipv6Addr::LOCALHOST), policy));
        assert!(is_blocked_ip(IpAddr::V6(Ipv6Addr::UNSPECIFIED), policy));
        assert!(is_blocked_hostname("localhost", policy));
        assert!(is_blocked_hostname("metadata.google.internal", policy));
    }

    #[test]
    fn private_target_opt_in_allows_loopback_and_rfc1918_targets() {
        // The explicit per-runtime opt-in covers loopback and RFC1918: self-hosted
        // engines on a trusted private segment become valid targets (operator
        // responsibility; DNS socket pinning still applies).
        let policy = PrivateTargetPolicy::new(true);
        assert!(!is_blocked_ip(
            IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1)),
            policy
        ));
        assert!(!is_blocked_ip(
            IpAddr::V4(Ipv4Addr::new(10, 0, 0, 1)),
            policy
        ));
        assert!(!is_blocked_ip(
            IpAddr::V4(Ipv4Addr::new(172, 16, 0, 1)),
            policy
        ));
        assert!(!is_blocked_ip(
            IpAddr::V4(Ipv4Addr::new(192, 168, 0, 1)),
            policy
        ));
        assert!(!is_blocked_ip(
            IpAddr::V4(Ipv4Addr::new(169, 254, 169, 254)),
            policy
        ));
        assert!(!is_blocked_ip(IpAddr::V6(Ipv6Addr::LOCALHOST), policy));
        assert!(!is_blocked_hostname("localhost", policy));
        assert!(!is_blocked_hostname("dify.internal", policy));
    }

    #[test]
    fn public_addresses_are_allowed_under_both_policies() {
        for allow in [false, true] {
            let policy = PrivateTargetPolicy::new(allow);
            assert!(!is_blocked_ip(
                IpAddr::V4(Ipv4Addr::new(8, 8, 8, 8)),
                policy
            ));
            assert!(!is_blocked_ip(
                IpAddr::V4(Ipv4Addr::new(1, 1, 1, 1)),
                policy
            ));
            assert!(!is_blocked_ip(
                IpAddr::V6(Ipv6Addr::new(0x2606, 0x4700, 0, 0, 0, 0, 0, 0)),
                policy
            ));
            assert!(!is_blocked_hostname("api.example.com", policy));
        }
    }

    #[test]
    fn blocked_hostnames_are_rejected_before_dns_by_default() {
        let policy = PrivateTargetPolicy::new(false);
        assert!(is_blocked_hostname("metadata.google.internal", policy));
        assert!(is_blocked_hostname("router.local", policy));
        assert!(is_blocked_hostname("k8s.internal", policy));
        assert!(is_blocked_hostname("127.0.0.1", policy));
        assert!(!is_blocked_hostname("api.example.com", policy));
    }

    #[test]
    fn ipv4_block_ranges_match_ingest_web_link_protection() {
        // CGNAT 100.64/10, 192.0.0.0/24, 192.88.99.0/24, 198.18/15 and >= 240/4.
        let policy = PrivateTargetPolicy::new(false);
        assert!(is_blocked_ipv4(Ipv4Addr::new(100, 64, 0, 1), policy));
        assert!(is_blocked_ipv4(Ipv4Addr::new(192, 0, 0, 1), policy));
        assert!(is_blocked_ipv4(Ipv4Addr::new(192, 88, 99, 1), policy));
        assert!(is_blocked_ipv4(Ipv4Addr::new(198, 18, 0, 1), policy));
        assert!(is_blocked_ipv4(Ipv4Addr::new(240, 0, 0, 1), policy));
        assert!(!is_blocked_ipv4(Ipv4Addr::new(8, 8, 8, 8), policy));
    }

    #[test]
    fn ipv6_non_global_prefixes_are_blocked_by_default() {
        let policy = PrivateTargetPolicy::new(false);
        assert!(is_blocked_ipv6(
            Ipv6Addr::new(0xfe80, 0, 0, 0, 0, 0, 0, 1),
            policy
        ));
        assert!(is_blocked_ipv6(
            Ipv6Addr::new(0xfc00, 0, 0, 0, 0, 0, 0, 1),
            policy
        ));
        assert!(is_blocked_ipv6(
            Ipv6Addr::new(0x2001, 0x0db8, 0, 0, 0, 0, 0, 1),
            policy
        ));
        assert!(!is_blocked_ipv6(
            Ipv6Addr::new(0x2606, 0x4700, 0, 0, 0, 0, 0, 0),
            policy
        ));
    }
}
