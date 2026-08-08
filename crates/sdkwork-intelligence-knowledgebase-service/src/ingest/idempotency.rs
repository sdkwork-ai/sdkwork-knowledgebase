use sha2::{Digest, Sha256};

#[cfg(test)]
pub fn api_payload_idempotency_fingerprint_sha256_hex(
    space_id: u64,
    title: &str,
    payload_markdown: &str,
) -> String {
    api_payload_idempotency_fingerprint_with_source(space_id, title, payload_markdown, None)
}

pub fn api_payload_idempotency_fingerprint_with_source(
    space_id: u64,
    title: &str,
    payload_markdown: &str,
    source_url: Option<&str>,
) -> String {
    let mut hasher = Sha256::new();
    hash_field(&mut hasher, "kind", Some("api_payload"));
    hash_field(&mut hasher, "space_id", Some(&space_id.to_string()));
    hash_field(&mut hasher, "title", Some(title));
    if let Some(source_url) = source_url.filter(|value| !value.is_empty()) {
        hash_field(&mut hasher, "source_url", Some(source_url));
    } else {
        hash_field(&mut hasher, "payload_markdown", Some(payload_markdown));
    }
    digest_to_hex(hasher.finalize())
}

fn hash_field(hasher: &mut Sha256, field_name: &str, value: Option<&str>) {
    hasher.update(field_name.as_bytes());
    hasher.update([0]);
    match value {
        Some(value) => {
            hasher.update(value.len().to_string().as_bytes());
            hasher.update(b":");
            hasher.update(value.as_bytes());
        }
        None => hasher.update(b"null"),
    }
    hasher.update([0xff]);
}

fn digest_to_hex(digest: impl AsRef<[u8]>) -> String {
    digest
        .as_ref()
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn api_payload_fingerprint_is_stable_and_hex() {
        let first = api_payload_idempotency_fingerprint_sha256_hex(1, "Title", "# Hello");
        let second = api_payload_idempotency_fingerprint_sha256_hex(1, "Title", "# Hello");
        assert_eq!(first, second);
        assert_eq!(first.len(), 64);
        assert!(first.chars().all(|ch| ch.is_ascii_hexdigit()));
    }

    #[test]
    fn api_payload_fingerprint_uses_source_url_when_present() {
        let from_url = api_payload_idempotency_fingerprint_with_source(
            1,
            "Title",
            "",
            Some("https://example.com/article"),
        );
        let from_payload =
            api_payload_idempotency_fingerprint_with_source(1, "Title", "# Hello", None);
        assert_ne!(from_url, from_payload);
        assert_eq!(from_url.len(), 64);
    }
}
