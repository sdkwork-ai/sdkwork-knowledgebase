pub const PACKAGE_NAME: &str = "sdkwork-routes-knowledgebase-app-api";
pub const SURFACE: &str = "app-api";
pub const OWNER: &str = "sdkwork-knowledgebase";
pub const DOMAIN: &str = "intelligence";
pub const CAPABILITY: &str = "knowledgebase";
pub const API_AUTHORITY: &str = "sdkwork-knowledgebase-app-api";
pub const SDK_FAMILY: &str = "sdkwork-knowledgebase-app-sdk";
pub const PREFIX: &str = "/app/v3/api";

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RouteManifestEntry {
    pub method: &'static str,
    pub path: &'static str,
    pub operation_id: &'static str,
}

pub const ROUTES: &[RouteManifestEntry] = &[
    RouteManifestEntry {
        method: "POST",
        path: "/app/v3/api/knowledge/group_launches/consume",
        operation_id: "groupLaunches.consume",
    },
    RouteManifestEntry {
        method: "POST",
        path: "/app/v3/api/knowledge/spaces",
        operation_id: "spaces.create",
    },
    RouteManifestEntry {
        method: "GET",
        path: "/app/v3/api/knowledge/spaces/{spaceId}",
        operation_id: "spaces.retrieve",
    },
    RouteManifestEntry {
        method: "PATCH",
        path: "/app/v3/api/knowledge/spaces/{spaceId}",
        operation_id: "spaces.update",
    },
    RouteManifestEntry {
        method: "DELETE",
        path: "/app/v3/api/knowledge/spaces/{spaceId}",
        operation_id: "spaces.delete",
    },
    RouteManifestEntry {
        method: "GET",
        path: "/app/v3/api/knowledge/spaces/{spaceId}/wiki_publication",
        operation_id: "wikiPublications.retrieve",
    },
    RouteManifestEntry {
        method: "POST",
        path: "/app/v3/api/knowledge/spaces/{spaceId}/wiki_publication/activate",
        operation_id: "wikiPublications.activate",
    },
    RouteManifestEntry {
        method: "POST",
        path: "/app/v3/api/knowledge/spaces/{spaceId}/wiki_publication/pause",
        operation_id: "wikiPublications.pause",
    },
    RouteManifestEntry {
        method: "POST",
        path: "/app/v3/api/knowledge/spaces/{spaceId}/wiki_source_files/{sourceFileUuid}/publish",
        operation_id: "wikiSourceFiles.publish",
    },
    RouteManifestEntry {
        method: "POST",
        path: "/app/v3/api/knowledge/spaces/{spaceId}/wiki_source_files/{sourceFileUuid}/unpublish",
        operation_id: "wikiSourceFiles.unpublish",
    },
    RouteManifestEntry {
        method: "PATCH",
        path:
            "/app/v3/api/knowledge/spaces/{spaceId}/wiki_source_files/{sourceFileUuid}/visibility",
        operation_id: "wikiSourceFiles.visibility.update",
    },
    RouteManifestEntry {
        method: "POST",
        path: "/app/v3/api/knowledge/drive_imports",
        operation_id: "driveImports.create",
    },
    RouteManifestEntry {
        method: "POST",
        path: "/app/v3/api/knowledge/git_imports",
        operation_id: "gitImports.create",
    },
    RouteManifestEntry {
        method: "POST",
        path: "/app/v3/api/knowledge/git_syncs",
        operation_id: "gitSyncs.create",
    },
    RouteManifestEntry {
        method: "GET",
        path: "/app/v3/api/knowledge/wechat/official_accounts",
        operation_id: "wechat.officialAccounts.list",
    },
    RouteManifestEntry {
        method: "PUT",
        path: "/app/v3/api/knowledge/wechat/official_accounts",
        operation_id: "wechat.officialAccounts.update",
    },
    RouteManifestEntry {
        method: "GET",
        path: "/app/v3/api/knowledge/wechat/official_accounts/{accountId}/fan_tags",
        operation_id: "wechat.officialAccounts.fanTags.list",
    },
    RouteManifestEntry {
        method: "GET",
        path: "/app/v3/api/knowledge/wechat/applets",
        operation_id: "wechat.applets.list",
    },
    RouteManifestEntry {
        method: "PUT",
        path: "/app/v3/api/knowledge/wechat/applets",
        operation_id: "wechat.applets.update",
    },
    RouteManifestEntry {
        method: "POST",
        path: "/app/v3/api/knowledge/wechat/articles/publish",
        operation_id: "wechat.articles.publish",
    },
    RouteManifestEntry {
        method: "POST",
        path: "/app/v3/api/knowledge/wechat/articles/preview",
        operation_id: "wechat.articles.preview",
    },
    RouteManifestEntry {
        method: "POST",
        path: "/app/v3/api/knowledge/ingests",
        operation_id: "ingests.create",
    },
    RouteManifestEntry {
        method: "GET",
        path: "/app/v3/api/knowledge/ingests/{ingestId}",
        operation_id: "ingests.retrieve",
    },
    RouteManifestEntry {
        method: "GET",
        path: "/app/v3/api/knowledge/documents",
        operation_id: "documents.list",
    },
    RouteManifestEntry {
        method: "POST",
        path: "/app/v3/api/knowledge/documents",
        operation_id: "documents.create",
    },
    RouteManifestEntry {
        method: "GET",
        path: "/app/v3/api/knowledge/documents/{documentId}",
        operation_id: "documents.retrieve",
    },
    RouteManifestEntry {
        method: "PATCH",
        path: "/app/v3/api/knowledge/documents/{documentId}",
        operation_id: "documents.update",
    },
    RouteManifestEntry {
        method: "DELETE",
        path: "/app/v3/api/knowledge/documents/{documentId}",
        operation_id: "documents.delete",
    },
    RouteManifestEntry {
        method: "GET",
        path: "/app/v3/api/knowledge/documents/{documentId}/content",
        operation_id: "documents.content.list",
    },
    RouteManifestEntry {
        method: "GET",
        path: "/app/v3/api/knowledge/documents/{documentId}/versions",
        operation_id: "documents.versions.list",
    },
    RouteManifestEntry {
        method: "POST",
        path: "/app/v3/api/knowledge/documents/{documentId}/versions",
        operation_id: "documents.versions.create",
    },
    RouteManifestEntry {
        method: "GET",
        path: "/app/v3/api/knowledge/okf/concepts",
        operation_id: "okf.concepts.list",
    },
    RouteManifestEntry {
        method: "PUT",
        path: "/app/v3/api/knowledge/okf/concepts/upsert",
        operation_id: "okf.concepts.update",
    },
    RouteManifestEntry {
        method: "GET",
        path: "/app/v3/api/knowledge/okf/concepts/{conceptId}",
        operation_id: "okf.concepts.retrieve",
    },
    RouteManifestEntry {
        method: "DELETE",
        path: "/app/v3/api/knowledge/okf/concepts/{conceptId}",
        operation_id: "okf.concepts.delete",
    },
    RouteManifestEntry {
        method: "GET",
        path: "/app/v3/api/knowledge/okf/concepts/{conceptId}/revisions",
        operation_id: "okf.concepts.revisions.list",
    },
    RouteManifestEntry {
        method: "GET",
        path: "/app/v3/api/knowledge/okf/index",
        operation_id: "okf.bundle.index.list",
    },
    RouteManifestEntry {
        method: "GET",
        path: "/app/v3/api/knowledge/okf/log",
        operation_id: "okf.bundle.log.list",
    },
    RouteManifestEntry {
        method: "GET",
        path: "/app/v3/api/knowledge/okf/profile",
        operation_id: "okf.bundle.profile.list",
    },
    RouteManifestEntry {
        method: "POST",
        path: "/app/v3/api/knowledge/okf/queries",
        operation_id: "okf.queries.create",
    },
    RouteManifestEntry {
        method: "POST",
        path: "/app/v3/api/knowledge/okf/queries/{queryId}/file_answer",
        operation_id: "okf.queries.fileAnswer",
    },
    RouteManifestEntry {
        method: "POST",
        path: "/app/v3/api/knowledge/okf/context_packs",
        operation_id: "okf.contextPacks.create",
    },
    RouteManifestEntry {
        method: "POST",
        path: "/app/v3/api/knowledge/okf/exports",
        operation_id: "okf.bundle.export.create",
    },
    RouteManifestEntry {
        method: "GET",
        path: "/app/v3/api/knowledge/okf/exports/{exportId}",
        operation_id: "okf.bundle.export.retrieve",
    },
    RouteManifestEntry {
        method: "POST",
        path: "/app/v3/api/knowledge/okf/imports",
        operation_id: "okf.bundle.import.create",
    },
    RouteManifestEntry {
        method: "POST",
        path: "/app/v3/api/knowledge/okf/lint_runs",
        operation_id: "okf.lintRuns.create",
    },
    RouteManifestEntry {
        method: "GET",
        path: "/app/v3/api/knowledge/spaces/{spaceId}/browser",
        operation_id: "spaces.browser.list",
    },
    RouteManifestEntry {
        method: "POST",
        path: "/app/v3/api/knowledge/retrievals",
        operation_id: "retrievals.create",
    },
    RouteManifestEntry {
        method: "GET",
        path: "/app/v3/api/knowledge/retrievals/{retrievalId}",
        operation_id: "retrievals.retrieve",
    },
    RouteManifestEntry {
        method: "POST",
        path: "/app/v3/api/knowledge/context_packs",
        operation_id: "contextPacks.create",
    },
    RouteManifestEntry {
        method: "POST",
        path: "/app/v3/api/knowledge/agent_profiles",
        operation_id: "agentProfiles.create",
    },
    RouteManifestEntry {
        method: "GET",
        path: "/app/v3/api/knowledge/agent_profiles/{profileId}",
        operation_id: "agentProfiles.retrieve",
    },
    RouteManifestEntry {
        method: "PATCH",
        path: "/app/v3/api/knowledge/agent_profiles/{profileId}",
        operation_id: "agentProfiles.update",
    },
    RouteManifestEntry {
        method: "DELETE",
        path: "/app/v3/api/knowledge/agent_profiles/{profileId}",
        operation_id: "agentProfiles.delete",
    },
    RouteManifestEntry {
        method: "GET",
        path: "/app/v3/api/knowledge/agent_profiles/{profileId}/bindings",
        operation_id: "agentProfiles.bindings.list",
    },
    RouteManifestEntry {
        method: "POST",
        path: "/app/v3/api/knowledge/agent_profiles/{profileId}/bindings",
        operation_id: "agentProfiles.bindings.create",
    },
    RouteManifestEntry {
        method: "PATCH",
        path: "/app/v3/api/knowledge/agent_profiles/{profileId}/bindings/{bindingId}",
        operation_id: "agentProfiles.bindings.update",
    },
    RouteManifestEntry {
        method: "DELETE",
        path: "/app/v3/api/knowledge/agent_profiles/{profileId}/bindings/{bindingId}",
        operation_id: "agentProfiles.bindings.delete",
    },
    RouteManifestEntry {
        method: "POST",
        path: "/app/v3/api/knowledge/agent_profiles/{profileId}/retrieval_preview",
        operation_id: "agentProfiles.retrievalPreview.create",
    },
    RouteManifestEntry {
        method: "POST",
        path: "/app/v3/api/knowledge/agent_profiles/{profileId}/chat",
        operation_id: "agentProfiles.chat.create",
    },
    RouteManifestEntry {
        method: "GET",
        path: "/app/v3/api/knowledge/spaces/{spaceId}/context_bindings",
        operation_id: "spaces.contextBindings.list",
    },
    RouteManifestEntry {
        method: "POST",
        path: "/app/v3/api/knowledge/spaces/{spaceId}/context_bindings",
        operation_id: "spaces.contextBindings.create",
    },
    RouteManifestEntry {
        method: "GET",
        path: "/app/v3/api/knowledge/spaces/{spaceId}/members",
        operation_id: "spaces.members.list",
    },
    RouteManifestEntry {
        method: "POST",
        path: "/app/v3/api/knowledge/spaces/{spaceId}/members",
        operation_id: "spaces.members.create",
    },
    RouteManifestEntry {
        method: "DELETE",
        path: "/app/v3/api/knowledge/spaces/{spaceId}/members",
        operation_id: "spaces.members.delete",
    },
    RouteManifestEntry {
        method: "GET",
        path: "/app/v3/api/knowledge/context_bindings/{bindingId}",
        operation_id: "contextBindings.retrieve",
    },
    RouteManifestEntry {
        method: "PATCH",
        path: "/app/v3/api/knowledge/context_bindings/{bindingId}",
        operation_id: "contextBindings.update",
    },
    RouteManifestEntry {
        method: "DELETE",
        path: "/app/v3/api/knowledge/context_bindings/{bindingId}",
        operation_id: "contextBindings.delete",
    },
    RouteManifestEntry {
        method: "GET",
        path: "/app/v3/api/knowledge/market/listings",
        operation_id: "market.listings.list",
    },
    RouteManifestEntry {
        method: "POST",
        path: "/app/v3/api/knowledge/market/subscriptions",
        operation_id: "market.subscriptions.create",
    },
    RouteManifestEntry {
        method: "DELETE",
        path: "/app/v3/api/knowledge/market/subscriptions/{listingId}",
        operation_id: "market.subscriptions.delete",
    },
    RouteManifestEntry {
        method: "POST",
        path: "/app/v3/api/knowledge/media_tasks",
        operation_id: "mediaTasks.create",
    },
];
