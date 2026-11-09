use sdkwork_web_core::{HttpMethod, HttpRoute, HttpRouteManifest, RateLimitTier};

const fn abuse_sensitive_open_route(
    method: HttpMethod,
    path: &'static str,
    tag: &'static str,
    operation_id: &'static str,
    permission: &'static str,
) -> HttpRoute {
    // Open-api authorization is scope-based: the contract declares an OAuth-style
    // scope per mutating operation and the manifest gate MUST enforce it
    // (PERMISSION_STANDARD_SPEC §Surface Authorization Tiers; API_SPEC §4.2.1).
    HttpRoute::api_key(method, path, tag, operation_id)
        .with_required_permission(permission)
        .with_rate_limit_tier(RateLimitTier::AuthCritical)
}

const HTTP_ROUTES: &[HttpRoute] = &[
    abuse_sensitive_open_route(
        HttpMethod::Post,
        "/knowledge/v3/api/retrievals",
        "knowledge",
        "retrievals.create",
        "knowledge.retrievals.write",
    ),
    HttpRoute::api_key(
        HttpMethod::Get,
        "/knowledge/v3/api/retrievals/{retrievalId}",
        "knowledge",
        "retrievals.retrieve",
    ),
    abuse_sensitive_open_route(
        HttpMethod::Post,
        "/knowledge/v3/api/context_packs",
        "knowledge",
        "contextPacks.create",
        "knowledge.context_packs.write",
    ),
    abuse_sensitive_open_route(
        HttpMethod::Post,
        "/knowledge/v3/api/ingests",
        "knowledge",
        "ingests.create",
        "knowledge.ingests.write",
    ),
    HttpRoute::api_key(
        HttpMethod::Get,
        "/knowledge/v3/api/ingests/{ingestId}",
        "knowledge",
        "ingests.retrieve",
    ),
    HttpRoute::api_key(
        HttpMethod::Get,
        "/knowledge/v3/api/documents",
        "knowledge",
        "documents.list",
    ),
    HttpRoute::api_key(
        HttpMethod::Get,
        "/knowledge/v3/api/documents/{documentId}",
        "knowledge",
        "documents.retrieve",
    ),
    HttpRoute::api_key(
        HttpMethod::Get,
        "/knowledge/v3/api/spaces/{spaceId}/browser",
        "knowledge",
        "spaces.browser.list",
    ),
];

pub fn open_route_manifest() -> HttpRouteManifest {
    HttpRouteManifest::new(HTTP_ROUTES)
}
