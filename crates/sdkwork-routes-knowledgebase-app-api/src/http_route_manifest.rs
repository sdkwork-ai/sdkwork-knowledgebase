use sdkwork_web_core::{HttpMethod, HttpRoute, HttpRouteManifest, RateLimitTier};

const fn knowledge_route(
    method: HttpMethod,
    path: &'static str,
    operation_id: &'static str,
) -> HttpRoute {
    // PERMISSION_STANDARD_SPEC §Surface Authorization Tiers: this is a first-party
    // app-api consumer surface (tier 0-2). OAuth-style per-route scopes are
    // contract violations on this surface; tier 2 operations MUST be enforced by
    // the service layer's space ownership/ACL checks, never by domain permission
    // codes, and tier 1 routes require authentication only. Routes therefore
    // carry dual-token authentication without any `required_permission`.
    HttpRoute::dual_token(method, path, "knowledge", operation_id)
}

const fn knowledge_read_route(
    method: HttpMethod,
    path: &'static str,
    operation_id: &'static str,
) -> HttpRoute {
    knowledge_route(method, path, operation_id)
}

const fn knowledge_abuse_route(
    method: HttpMethod,
    path: &'static str,
    operation_id: &'static str,
) -> HttpRoute {
    knowledge_route(method, path, operation_id)
        .with_rate_limit_tier(RateLimitTier::AuthCritical)
}

/// WeChat public-platform servers call the callback routes directly and cannot
/// present SDKWork dual tokens or an ingress token, so the least-privilege honest
/// declaration is explicit `Public` with the AuthCritical (abuse) rate-limit tier:
/// authentication is the WeChat msg_signature scheme verified inside the handler
/// against the tenant-stored per-account token, plus the `tenant`/`account_id`
/// query scope. `external_wire_protocol` marks these operations as mirroring the
/// WeChat server-callback wire (API_SPEC §4.5.2), so the plain-text echo/"success"
/// bodies are exempt from the SdkWorkApiResponse envelope.
const fn knowledge_wechat_callback_route(
    method: HttpMethod,
    path: &'static str,
    operation_id: &'static str,
) -> HttpRoute {
    HttpRoute::public(method, path, "knowledge", operation_id)
        .with_rate_limit_tier(RateLimitTier::AuthCritical)
        .with_external_wire_protocol("wechat-mp-server-callback")
}

const HTTP_ROUTES: &[HttpRoute] = &[
    knowledge_abuse_route(
        HttpMethod::Post,
        "/app/v3/api/knowledge/group_launches/consume",
        "groupLaunches.consume",
    )
    .with_idempotent(true),
    // Any authenticated principal may create a knowledge space: dual-token
    // authentication only, no RBAC permission point (platform app-api
    // pattern; the service layer grants the creator owner access).
    knowledge_read_route(
        HttpMethod::Post,
        "/app/v3/api/knowledge/spaces",
        "spaces.create",
    ),
    knowledge_read_route(
        HttpMethod::Get,
        "/app/v3/api/knowledge/spaces/{spaceId}",
        "spaces.retrieve",
    ),
    knowledge_route(
        HttpMethod::Patch,
        "/app/v3/api/knowledge/spaces/{spaceId}",
        "spaces.update",
    ),
    knowledge_abuse_route(
        HttpMethod::Delete,
        "/app/v3/api/knowledge/spaces/{spaceId}",
        "spaces.delete",
    ),
    knowledge_read_route(
        HttpMethod::Get,
        "/app/v3/api/knowledge/spaces/{spaceId}/wiki_publication",
        "wikiPublications.retrieve",
    ),
    knowledge_read_route(
        HttpMethod::Get,
        "/app/v3/api/knowledge/spaces/{spaceId}/wiki_source_files",
        "wikiSourceFiles.list",
    ),
    knowledge_abuse_route(
        HttpMethod::Post,
        "/app/v3/api/knowledge/spaces/{spaceId}/wiki_publication/activate",
        "wikiPublications.activate",
    )
    .with_idempotent(true),
    knowledge_abuse_route(
        HttpMethod::Post,
        "/app/v3/api/knowledge/spaces/{spaceId}/wiki_publication/pause",
        "wikiPublications.pause",
    )
    .with_idempotent(true),
    knowledge_abuse_route(
        HttpMethod::Post,
        "/app/v3/api/knowledge/spaces/{spaceId}/wiki_source_files/{sourceFileUuid}/publish",
        "wikiSourceFiles.publish",
    )
    .with_idempotent(true),
    knowledge_abuse_route(
        HttpMethod::Post,
        "/app/v3/api/knowledge/spaces/{spaceId}/wiki_source_files/{sourceFileUuid}/unpublish",
        "wikiSourceFiles.unpublish",
    )
    .with_idempotent(true),
    knowledge_abuse_route(
        HttpMethod::Patch,
        "/app/v3/api/knowledge/spaces/{spaceId}/wiki_source_files/{sourceFileUuid}/visibility",
        "wikiSourceFiles.visibility.update",
    )
    .with_idempotent(true),
    knowledge_abuse_route(
        HttpMethod::Post,
        "/app/v3/api/knowledge/drive_imports",
        "driveImports.create",
    ),
    knowledge_abuse_route(
        HttpMethod::Post,
        "/app/v3/api/knowledge/git_imports",
        "gitImports.create",
    ),
    knowledge_abuse_route(
        HttpMethod::Post,
        "/app/v3/api/knowledge/git_syncs",
        "gitSyncs.create",
    ),
    knowledge_read_route(
        HttpMethod::Get,
        "/app/v3/api/knowledge/wechat/official_accounts",
        "wechat.officialAccounts.list",
    ),
    knowledge_abuse_route(
        HttpMethod::Put,
        "/app/v3/api/knowledge/wechat/official_accounts",
        "wechat.officialAccounts.update",
    ),
    // Fan tags proxy straight into an outbound WeChat call, so this read carries the abuse tier
    // like the other externally-amplifying routes.
    knowledge_abuse_route(
        HttpMethod::Get,
        "/app/v3/api/knowledge/wechat/official_accounts/{accountId}/fan_tags",
        "wechat.officialAccounts.fanTags.list",
    ),
    knowledge_read_route(
        HttpMethod::Get,
        "/app/v3/api/knowledge/wechat/applets",
        "wechat.applets.list",
    ),
    knowledge_abuse_route(
        HttpMethod::Put,
        "/app/v3/api/knowledge/wechat/applets",
        "wechat.applets.update",
    ),
    knowledge_abuse_route(
        HttpMethod::Post,
        "/app/v3/api/knowledge/wechat/articles/publish",
        "wechat.articles.publish",
    ),
    knowledge_abuse_route(
        HttpMethod::Post,
        "/app/v3/api/knowledge/wechat/articles/preview",
        "wechat.articles.preview",
    ),
    knowledge_wechat_callback_route(
        HttpMethod::Get,
        "/app/v3/api/knowledge/wechat/callback",
        "wechat.callback.verify",
    ),
    knowledge_wechat_callback_route(
        HttpMethod::Post,
        "/app/v3/api/knowledge/wechat/callback",
        "wechat.callback.receive",
    ),
    knowledge_abuse_route(
        HttpMethod::Post,
        "/app/v3/api/knowledge/ingests",
        "ingests.create",
    ),
    knowledge_read_route(
        HttpMethod::Get,
        "/app/v3/api/knowledge/ingests/{ingestId}",
        "ingests.retrieve",
    ),
    knowledge_read_route(
        HttpMethod::Get,
        "/app/v3/api/knowledge/documents",
        "documents.list",
    ),
    knowledge_route(
        HttpMethod::Post,
        "/app/v3/api/knowledge/documents",
        "documents.create",
    ),
    knowledge_read_route(
        HttpMethod::Get,
        "/app/v3/api/knowledge/documents/{documentId}",
        "documents.retrieve",
    ),
    knowledge_route(
        HttpMethod::Patch,
        "/app/v3/api/knowledge/documents/{documentId}",
        "documents.update",
    ),
    knowledge_abuse_route(
        HttpMethod::Delete,
        "/app/v3/api/knowledge/documents/{documentId}",
        "documents.delete",
    ),
    knowledge_read_route(
        HttpMethod::Get,
        "/app/v3/api/knowledge/documents/{documentId}/content",
        "documents.content.list",
    ),
    knowledge_read_route(
        HttpMethod::Get,
        "/app/v3/api/knowledge/documents/{documentId}/versions",
        "documents.versions.list",
    ),
    knowledge_route(
        HttpMethod::Post,
        "/app/v3/api/knowledge/documents/{documentId}/versions",
        "documents.versions.create",
    ),
    knowledge_read_route(
        HttpMethod::Get,
        "/app/v3/api/knowledge/okf/concepts",
        "okf.concepts.list",
    ),
    knowledge_route(
        HttpMethod::Put,
        "/app/v3/api/knowledge/okf/concepts/upsert",
        "okf.concepts.update",
    ),
    knowledge_read_route(
        HttpMethod::Get,
        "/app/v3/api/knowledge/okf/concepts/{conceptId}",
        "okf.concepts.retrieve",
    ),
    knowledge_route(
        HttpMethod::Delete,
        "/app/v3/api/knowledge/okf/concepts/{conceptId}",
        "okf.concepts.delete",
    ),
    knowledge_read_route(
        HttpMethod::Get,
        "/app/v3/api/knowledge/okf/concepts/{conceptId}/revisions",
        "okf.concepts.revisions.list",
    ),
    knowledge_read_route(
        HttpMethod::Get,
        "/app/v3/api/knowledge/okf/index",
        "okf.bundle.index.list",
    ),
    knowledge_read_route(
        HttpMethod::Get,
        "/app/v3/api/knowledge/okf/log",
        "okf.bundle.log.list",
    ),
    knowledge_read_route(
        HttpMethod::Get,
        "/app/v3/api/knowledge/okf/profile",
        "okf.bundle.profile.list",
    ),
    knowledge_route(
        HttpMethod::Post,
        "/app/v3/api/knowledge/okf/queries",
        "okf.queries.create",
    ),
    knowledge_route(
        HttpMethod::Post,
        "/app/v3/api/knowledge/okf/queries/{queryId}/file_answer",
        "okf.queries.fileAnswer",
    ),
    knowledge_route(
        HttpMethod::Post,
        "/app/v3/api/knowledge/okf/context_packs",
        "okf.contextPacks.create",
    ),
    knowledge_abuse_route(
        HttpMethod::Post,
        "/app/v3/api/knowledge/okf/exports",
        "okf.bundle.export.create",
    ),
    knowledge_read_route(
        HttpMethod::Get,
        "/app/v3/api/knowledge/okf/exports/{exportId}",
        "okf.bundle.export.retrieve",
    ),
    knowledge_abuse_route(
        HttpMethod::Post,
        "/app/v3/api/knowledge/okf/imports",
        "okf.bundle.import.create",
    ),
    knowledge_route(
        HttpMethod::Post,
        "/app/v3/api/knowledge/okf/lint_runs",
        "okf.lintRuns.create",
    ),
    knowledge_read_route(
        HttpMethod::Get,
        "/app/v3/api/knowledge/spaces/{spaceId}/browser",
        "spaces.browser.list",
    ),
    knowledge_abuse_route(
        HttpMethod::Post,
        "/app/v3/api/knowledge/retrievals",
        "retrievals.create",
    ),
    knowledge_read_route(
        HttpMethod::Get,
        "/app/v3/api/knowledge/retrievals/{retrievalId}",
        "retrievals.retrieve",
    ),
    knowledge_abuse_route(
        HttpMethod::Post,
        "/app/v3/api/knowledge/context_packs",
        "contextPacks.create",
    ),
    knowledge_route(
        HttpMethod::Post,
        "/app/v3/api/knowledge/agent_profiles",
        "agentProfiles.create",
    ),
    knowledge_read_route(
        HttpMethod::Get,
        "/app/v3/api/knowledge/agent_profiles/{profileId}",
        "agentProfiles.retrieve",
    ),
    knowledge_route(
        HttpMethod::Patch,
        "/app/v3/api/knowledge/agent_profiles/{profileId}",
        "agentProfiles.update",
    ),
    knowledge_route(
        HttpMethod::Delete,
        "/app/v3/api/knowledge/agent_profiles/{profileId}",
        "agentProfiles.delete",
    ),
    knowledge_read_route(
        HttpMethod::Get,
        "/app/v3/api/knowledge/agent_profiles/{profileId}/bindings",
        "agentProfiles.bindings.list",
    ),
    knowledge_route(
        HttpMethod::Post,
        "/app/v3/api/knowledge/agent_profiles/{profileId}/bindings",
        "agentProfiles.bindings.create",
    ),
    knowledge_route(
        HttpMethod::Patch,
        "/app/v3/api/knowledge/agent_profiles/{profileId}/bindings/{bindingId}",
        "agentProfiles.bindings.update",
    ),
    knowledge_route(
        HttpMethod::Delete,
        "/app/v3/api/knowledge/agent_profiles/{profileId}/bindings/{bindingId}",
        "agentProfiles.bindings.delete",
    ),
    knowledge_abuse_route(
        HttpMethod::Post,
        "/app/v3/api/knowledge/agent_profiles/{profileId}/retrieval_preview",
        "agentProfiles.retrievalPreview.create",
    ),
    knowledge_abuse_route(
        HttpMethod::Post,
        "/app/v3/api/knowledge/agent_profiles/{profileId}/chat",
        "agentProfiles.chat.create",
    ),
    knowledge_read_route(
        HttpMethod::Get,
        "/app/v3/api/knowledge/spaces/{spaceId}/context_bindings",
        "spaces.contextBindings.list",
    ),
    knowledge_route(
        HttpMethod::Post,
        "/app/v3/api/knowledge/spaces/{spaceId}/context_bindings",
        "spaces.contextBindings.create",
    ),
    knowledge_read_route(
        HttpMethod::Get,
        "/app/v3/api/knowledge/spaces/{spaceId}/members",
        "spaces.members.list",
    ),
    knowledge_route(
        HttpMethod::Post,
        "/app/v3/api/knowledge/spaces/{spaceId}/members",
        "spaces.members.create",
    ),
    knowledge_route(
        HttpMethod::Delete,
        "/app/v3/api/knowledge/spaces/{spaceId}/members",
        "spaces.members.delete",
    ),
    knowledge_read_route(
        HttpMethod::Get,
        "/app/v3/api/knowledge/context_bindings/{bindingId}",
        "contextBindings.retrieve",
    ),
    knowledge_route(
        HttpMethod::Patch,
        "/app/v3/api/knowledge/context_bindings/{bindingId}",
        "contextBindings.update",
    ),
    knowledge_route(
        HttpMethod::Delete,
        "/app/v3/api/knowledge/context_bindings/{bindingId}",
        "contextBindings.delete",
    ),
    knowledge_read_route(
        HttpMethod::Get,
        "/app/v3/api/knowledge/market/listings",
        "market.listings.list",
    ),
    knowledge_abuse_route(
        HttpMethod::Post,
        "/app/v3/api/knowledge/market/subscriptions",
        "market.subscriptions.create",
    ),
    knowledge_abuse_route(
        HttpMethod::Delete,
        "/app/v3/api/knowledge/market/subscriptions/{listingId}",
        "market.subscriptions.delete",
    ),
    knowledge_abuse_route(
        HttpMethod::Post,
        "/app/v3/api/knowledge/media_tasks",
        "mediaTasks.create",
    ),
];

pub fn app_route_manifest() -> HttpRouteManifest {
    HttpRouteManifest::new(HTTP_ROUTES)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn group_launch_consumption_requires_framework_idempotency() {
        let manifest = app_route_manifest();
        let route = manifest
            .match_route("POST", "/app/v3/api/knowledge/group_launches/consume")
            .expect("group launch consume route");
        assert!(route.idempotent);
    }
}
