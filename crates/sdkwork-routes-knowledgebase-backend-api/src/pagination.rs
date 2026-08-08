//! Shared cursor pagination helpers for Knowledgebase backend API list handlers.

use sdkwork_utils_rust::{
    base64url_decode, base64url_encode, PageInfo, PageMode, SdkWorkPageData, SdkWorkResultCode,
    DEFAULT_LIST_PAGE_SIZE, MAX_LIST_PAGE_SIZE,
};

/// Validates an optional `page_size` against the canonical 1..=200 range. Out-of-range
/// values are rejected (not silently clamped) so backend pagination semantics match the
/// app API surface exactly.
pub fn normalize_page_size(page_size: Option<u32>) -> Result<u32, SdkWorkResultCode> {
    let page_size = page_size.unwrap_or(DEFAULT_LIST_PAGE_SIZE as u32);
    if !(1..=MAX_LIST_PAGE_SIZE as u32).contains(&page_size) {
        return Err(SdkWorkResultCode::InvalidParameter);
    }
    Ok(page_size)
}

/// Encode a numeric keyset position into an opaque cursor token (PAGINATION_SPEC §3).
pub fn encode_opaque_u64_cursor(value: u64) -> String {
    base64url_encode(value.to_string().as_bytes())
}

/// Parse an opaque keyset cursor token back into the numeric position.
pub fn parse_u64_cursor(cursor: Option<&str>) -> Result<Option<u64>, SdkWorkResultCode> {
    let Some(cursor) = cursor.map(str::trim).filter(|value| !value.is_empty()) else {
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
