//! Shared cursor pagination helpers for Knowledgebase app API list handlers.

use sdkwork_knowledgebase_contract::{
    KnowledgeBrowserListData, KnowledgeBrowserNode, KnowledgeBrowserView,
};
use sdkwork_utils_rust::{
    base64url_decode, base64url_encode, is_blank, PageInfo, PageMode, SdkWorkPageData,
    SdkWorkResultCode, DEFAULT_LIST_PAGE_SIZE, MAX_LIST_PAGE_SIZE,
};
use serde::{Deserialize, Serialize};

const OKF_CURSOR_VERSION: u8 = 1;
const OKF_CONCEPT_CURSOR_KIND: &str = "concepts";
const OKF_REVISION_CURSOR_KIND: &str = "revisions";
const MAX_OKF_CURSOR_LENGTH: usize = 512;

#[derive(Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct OkfConceptCursor {
    version: u8,
    kind: String,
    tenant_id: u64,
    space_id: u64,
    after_concept_id: String,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct OkfRevisionCursor {
    version: u8,
    kind: String,
    tenant_id: u64,
    space_id: u64,
    concept_row_id: u64,
    after_revision_no: u64,
}

/// Validate optional `page_size` against the canonical 1..=200 range.
pub fn normalize_page_size(page_size: Option<u32>) -> Result<u32, SdkWorkResultCode> {
    let page_size = page_size.unwrap_or(DEFAULT_LIST_PAGE_SIZE as u32);
    if !(1..=MAX_LIST_PAGE_SIZE as u32).contains(&page_size) {
        return Err(SdkWorkResultCode::InvalidParameter);
    }
    Ok(page_size)
}

/// Validate an HTTP list page size and map failures to the standard API error contract.
pub fn normalize_api_page_size(page_size: Option<u32>) -> crate::ApiResult<u32> {
    normalize_page_size(page_size).map_err(|_| {
        crate::ApiError::invalid_request(
            "invalid_parameter",
            format!("page_size must be between 1 and {MAX_LIST_PAGE_SIZE}"),
        )
    })
}

/// Encode the next concept business key into an opaque, scope-bound cursor.
pub fn encode_okf_concept_cursor(
    tenant_id: u64,
    space_id: u64,
    after_concept_id: &str,
) -> Result<String, SdkWorkResultCode> {
    if is_blank(Some(after_concept_id)) {
        return Err(SdkWorkResultCode::InvalidParameter);
    }
    encode_okf_cursor(&OkfConceptCursor {
        version: OKF_CURSOR_VERSION,
        kind: OKF_CONCEPT_CURSOR_KIND.to_string(),
        tenant_id,
        space_id,
        after_concept_id: after_concept_id.to_string(),
    })
}

/// Decode and validate a concept cursor for the authenticated tenant and space.
pub fn parse_okf_concept_cursor(
    cursor: Option<&str>,
    tenant_id: u64,
    space_id: u64,
) -> Result<Option<String>, SdkWorkResultCode> {
    let Some(decoded) = decode_okf_cursor(cursor)? else {
        return Ok(None);
    };
    let payload: OkfConceptCursor =
        serde_json::from_slice(&decoded).map_err(|_| SdkWorkResultCode::InvalidParameter)?;
    if payload.version != OKF_CURSOR_VERSION
        || payload.kind != OKF_CONCEPT_CURSOR_KIND
        || payload.tenant_id != tenant_id
        || payload.space_id != space_id
        || is_blank(Some(&payload.after_concept_id))
    {
        return Err(SdkWorkResultCode::InvalidParameter);
    }
    Ok(Some(payload.after_concept_id))
}

/// Encode the next revision number into an opaque, scope-bound cursor.
pub fn encode_okf_revision_cursor(
    tenant_id: u64,
    space_id: u64,
    concept_row_id: u64,
    after_revision_no: u64,
) -> Result<String, SdkWorkResultCode> {
    if after_revision_no == 0 {
        return Err(SdkWorkResultCode::InvalidParameter);
    }
    encode_okf_cursor(&OkfRevisionCursor {
        version: OKF_CURSOR_VERSION,
        kind: OKF_REVISION_CURSOR_KIND.to_string(),
        tenant_id,
        space_id,
        concept_row_id,
        after_revision_no,
    })
}

/// Decode and validate a revision cursor for the authenticated resource scope.
pub fn parse_okf_revision_cursor(
    cursor: Option<&str>,
    tenant_id: u64,
    space_id: u64,
    concept_row_id: u64,
) -> Result<Option<u64>, SdkWorkResultCode> {
    let Some(decoded) = decode_okf_cursor(cursor)? else {
        return Ok(None);
    };
    let payload: OkfRevisionCursor =
        serde_json::from_slice(&decoded).map_err(|_| SdkWorkResultCode::InvalidParameter)?;
    if payload.version != OKF_CURSOR_VERSION
        || payload.kind != OKF_REVISION_CURSOR_KIND
        || payload.tenant_id != tenant_id
        || payload.space_id != space_id
        || payload.concept_row_id != concept_row_id
        || payload.after_revision_no == 0
    {
        return Err(SdkWorkResultCode::InvalidParameter);
    }
    Ok(Some(payload.after_revision_no))
}

fn encode_okf_cursor<T: Serialize>(payload: &T) -> Result<String, SdkWorkResultCode> {
    let json = serde_json::to_vec(payload).map_err(|_| SdkWorkResultCode::InvalidParameter)?;
    let cursor = base64url_encode(&json);
    if cursor.len() > MAX_OKF_CURSOR_LENGTH {
        return Err(SdkWorkResultCode::InvalidParameter);
    }
    Ok(cursor)
}

fn decode_okf_cursor(cursor: Option<&str>) -> Result<Option<Vec<u8>>, SdkWorkResultCode> {
    let Some(cursor) = cursor else {
        return Ok(None);
    };
    if is_blank(Some(cursor))
        || cursor.trim() != cursor
        || cursor.len() > MAX_OKF_CURSOR_LENGTH
        || cursor.bytes().all(|byte| byte.is_ascii_digit())
    {
        return Err(SdkWorkResultCode::InvalidParameter);
    }
    base64url_decode(cursor)
        .map(Some)
        .ok_or(SdkWorkResultCode::InvalidParameter)
}

/// Encode a numeric keyset position into an opaque cursor token (PAGINATION_SPEC §3:
/// cursor tokens must be opaque to clients; clients must not parse or construct them).
pub fn encode_opaque_u64_cursor(value: u64) -> String {
    base64url_encode(value.to_string().as_bytes())
}

/// Parse an opaque keyset cursor token back into the numeric position.
pub fn parse_opaque_u64_cursor(cursor: Option<&str>) -> Result<Option<u64>, SdkWorkResultCode> {
    let Some(cursor) = cursor.map(str::trim).filter(|value| !is_blank(Some(value))) else {
        return Ok(None);
    };
    let decoded = base64url_decode(cursor).ok_or(SdkWorkResultCode::InvalidParameter)?;
    let text = String::from_utf8(decoded).map_err(|_| SdkWorkResultCode::InvalidParameter)?;
    text.parse::<u64>()
        .map(Some)
        .map_err(|_| SdkWorkResultCode::InvalidParameter)
}

/// Encode a store-returned numeric next-cursor into the opaque wire form.
pub fn encode_opaque_next_cursor(next_cursor: Option<String>) -> Option<String> {
    next_cursor
        .and_then(|value| value.parse::<u64>().ok())
        .map(encode_opaque_u64_cursor)
}

/// Build cursor-mode `SdkWorkPageData` from a bounded page window.
pub fn cursor_page_data<T>(
    items: Vec<T>,
    next_cursor: Option<String>,
    has_more: bool,
    page_size: u32,
) -> SdkWorkPageData<T> {
    SdkWorkPageData {
        items,
        page_info: PageInfo {
            mode: PageMode::Cursor,
            page: None,
            page_size: Some(page_size as i32),
            total_items: None,
            total_pages: None,
            next_cursor,
            has_more: Some(has_more),
        },
    }
}

/// Map a browser list window to standard cursor-mode list data with browser view context.
pub fn browser_list_page_data(
    space_id: u64,
    drive_space_id: String,
    parent_id: Option<String>,
    view: KnowledgeBrowserView,
    items: Vec<KnowledgeBrowserNode>,
    next_cursor: Option<String>,
    page_size: u32,
) -> KnowledgeBrowserListData {
    let page = cursor_page_data(items, next_cursor.clone(), next_cursor.is_some(), page_size);
    KnowledgeBrowserListData {
        space_id,
        drive_space_id,
        parent_id,
        view,
        page_size,
        items: page.items,
        page_info: page.page_info,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn normalize_page_size_defaults_and_accepts_canonical_bounds() {
        assert_eq!(20, normalize_page_size(None).expect("default"));
        assert_eq!(1, normalize_page_size(Some(1)).expect("minimum"));
        assert_eq!(200, normalize_page_size(Some(200)).expect("maximum"));
    }

    #[test]
    fn normalize_page_size_rejects_values_outside_canonical_bounds() {
        assert_eq!(
            SdkWorkResultCode::InvalidParameter,
            normalize_page_size(Some(0)).expect_err("zero")
        );
        assert_eq!(
            SdkWorkResultCode::InvalidParameter,
            normalize_page_size(Some(201)).expect_err("above maximum")
        );
    }

    #[test]
    fn okf_concept_cursor_round_trips_opaque_position() {
        let cursor = encode_okf_concept_cursor(1001, 2002, "topics/concept-0200")
            .expect("encode concept cursor");

        assert_ne!(cursor, "topics/concept-0200");
        assert_eq!(
            Some("topics/concept-0200".to_string()),
            parse_okf_concept_cursor(Some(&cursor), 1001, 2002).expect("parse concept cursor")
        );
    }

    #[test]
    fn okf_revision_cursor_round_trips_opaque_position() {
        let cursor =
            encode_okf_revision_cursor(1001, 2002, 3003, 200).expect("encode revision cursor");

        assert_ne!(cursor, "200");
        assert_eq!(
            Some(200),
            parse_okf_revision_cursor(Some(&cursor), 1001, 2002, 3003)
                .expect("parse revision cursor")
        );
    }

    #[test]
    fn okf_cursor_rejects_empty_numeric_malformed_and_oversized_tokens() {
        for cursor in ["", "   ", "42", "not-a-cursor"] {
            assert_eq!(
                SdkWorkResultCode::InvalidParameter,
                parse_okf_concept_cursor(Some(cursor), 1001, 2002).expect_err("invalid cursor")
            );
        }

        assert_eq!(
            SdkWorkResultCode::InvalidParameter,
            parse_okf_concept_cursor(Some(&"x".repeat(513)), 1001, 2002)
                .expect_err("oversized cursor")
        );
    }

    #[test]
    fn okf_cursor_rejects_unknown_version_and_unknown_fields() {
        let unknown_version = raw_cursor(serde_json::json!({
            "version": 2,
            "kind": "concepts",
            "tenant_id": 1001,
            "space_id": 2002,
            "after_concept_id": "topics/concept-0200"
        }));
        let unknown_field = raw_cursor(serde_json::json!({
            "version": 1,
            "kind": "concepts",
            "tenant_id": 1001,
            "space_id": 2002,
            "after_concept_id": "topics/concept-0200",
            "unexpected": true
        }));

        for cursor in [unknown_version, unknown_field] {
            assert_eq!(
                SdkWorkResultCode::InvalidParameter,
                parse_okf_concept_cursor(Some(&cursor), 1001, 2002)
                    .expect_err("unsupported cursor")
            );
        }
    }

    #[test]
    fn okf_cursor_rejects_wrong_kind_tenant_space_and_concept() {
        let concept_cursor =
            encode_okf_concept_cursor(1001, 2002, "topics/concept-0200").expect("concept cursor");
        let revision_cursor =
            encode_okf_revision_cursor(1001, 2002, 3003, 200).expect("revision cursor");

        assert_eq!(
            SdkWorkResultCode::InvalidParameter,
            parse_okf_revision_cursor(Some(&concept_cursor), 1001, 2002, 3003)
                .expect_err("wrong kind")
        );
        assert_eq!(
            SdkWorkResultCode::InvalidParameter,
            parse_okf_concept_cursor(Some(&concept_cursor), 9999, 2002).expect_err("wrong tenant")
        );
        assert_eq!(
            SdkWorkResultCode::InvalidParameter,
            parse_okf_concept_cursor(Some(&concept_cursor), 1001, 9999).expect_err("wrong space")
        );
        assert_eq!(
            SdkWorkResultCode::InvalidParameter,
            parse_okf_revision_cursor(Some(&revision_cursor), 1001, 2002, 9999)
                .expect_err("wrong concept")
        );
    }

    #[test]
    fn cursor_page_data_uses_cursor_mode_page_info() {
        let page = cursor_page_data(vec!["a".to_string()], Some("99".to_string()), true, 20);
        assert_eq!(PageMode::Cursor, page.page_info.mode);
        assert_eq!(Some(20), page.page_info.page_size);
        assert_eq!(Some("99".to_string()), page.page_info.next_cursor);
        assert_eq!(Some(true), page.page_info.has_more);
    }

    fn raw_cursor(value: serde_json::Value) -> String {
        sdkwork_utils_rust::base64url_encode(value.to_string().as_bytes())
    }
}
