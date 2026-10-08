//! URL path-segment safety helpers for engine adapters that interpolate
//! caller-controlled resource ids into upstream request paths.

use percent_encoding::{utf8_percent_encode, AsciiSet, CONTROLS};

/// Maximum accepted length for one engine resource id.
pub const MAX_PATH_SEGMENT_ID_LEN: usize = 256;

/// Equivalent to the WHATWG `PATH_SEGMENT` percent-encode set (controls, space,
/// `"`, `#`, `<`, `>`, `?`, backtick, `{`, `}`, `%`, `/`), so an encoded value
/// can never introduce a new path or query boundary.
const PATH_SEGMENT_ENCODE_SET: &AsciiSet = &CONTROLS
    .add(b' ')
    .add(b'"')
    .add(b'#')
    .add(b'<')
    .add(b'>')
    .add(b'?')
    .add(b'`')
    .add(b'{')
    .add(b'}')
    .add(b'%')
    .add(b'/');

/// Percent-encodes one caller-controlled value for use as a single URL path
/// segment, so ids containing separators cannot escape the intended engine
/// request path.
#[must_use]
pub fn encoded_path_segment(value: &str) -> String {
    utf8_percent_encode(value, PATH_SEGMENT_ENCODE_SET).to_string()
}

/// Validates an engine resource id against `^[A-Za-z0-9._:-]{1,256}$` before it
/// may be interpolated into an engine URL path. `.` and `..` are rejected even
/// though they match the charset, so a crafted id cannot become a URL traversal
/// segment; callers must still combine this with [`encoded_path_segment`].
#[must_use]
pub fn is_path_segment_id(value: &str) -> bool {
    if value.is_empty() || value.len() > MAX_PATH_SEGMENT_ID_LEN {
        return false;
    }
    if value == "." || value == ".." {
        return false;
    }
    value.bytes().all(|byte| {
        matches!(
            byte,
            b'a'..=b'z' | b'A'..=b'Z' | b'0'..=b'9' | b'.' | b'_' | b':' | b'-'
        )
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn encodes_path_separators_and_boundaries() {
        assert_eq!(encoded_path_segment("doc-1"), "doc-1");
        assert_eq!(
            encoded_path_segment("../admin?x=1#f"),
            "..%2Fadmin%3Fx=1%23f"
        );
    }

    #[test]
    fn accepts_only_bounded_flat_ids() {
        assert!(is_path_segment_id("doc-1_seg.2:x"));
        assert!(is_path_segment_id(&"a".repeat(MAX_PATH_SEGMENT_ID_LEN)));
        assert!(!is_path_segment_id(""));
        assert!(!is_path_segment_id(
            &"a".repeat(MAX_PATH_SEGMENT_ID_LEN + 1)
        ));
        assert!(!is_path_segment_id("."));
        assert!(!is_path_segment_id(".."));
        assert!(!is_path_segment_id("a/b"));
        assert!(!is_path_segment_id("a b"));
        assert!(!is_path_segment_id("é"));
    }
}
