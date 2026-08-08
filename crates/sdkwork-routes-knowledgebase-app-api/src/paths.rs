pub const PREFIX: &str = "/app/v3/api";
pub const LIVEZ: &str = "/livez";
pub const READYZ: &str = "/readyz";
pub const HEALTHZ: &str = "/healthz";
pub const SPACES: &str = "/app/v3/api/knowledge/spaces";
pub const GROUP_LAUNCHES_CONSUME: &str = "/app/v3/api/knowledge/group_launches/consume";
pub const SPACE: &str = "/app/v3/api/knowledge/spaces/{space_id}";
pub const WIKI_PUBLICATION: &str = "/app/v3/api/knowledge/spaces/{space_id}/wiki_publication";
pub const WIKI_PUBLICATION_ACTIVATE: &str =
    "/app/v3/api/knowledge/spaces/{space_id}/wiki_publication/activate";
pub const WIKI_PUBLICATION_PAUSE: &str =
    "/app/v3/api/knowledge/spaces/{space_id}/wiki_publication/pause";
pub const WIKI_SOURCE_FILE_PUBLISH: &str =
    "/app/v3/api/knowledge/spaces/{space_id}/wiki_source_files/{source_file_uuid}/publish";
pub const WIKI_SOURCE_FILE_UNPUBLISH: &str =
    "/app/v3/api/knowledge/spaces/{space_id}/wiki_source_files/{source_file_uuid}/unpublish";
pub const WIKI_SOURCE_FILE_VISIBILITY: &str =
    "/app/v3/api/knowledge/spaces/{space_id}/wiki_source_files/{source_file_uuid}/visibility";
pub const DRIVE_IMPORTS: &str = "/app/v3/api/knowledge/drive_imports";
pub const GIT_IMPORTS: &str = "/app/v3/api/knowledge/git_imports";
pub const GIT_SYNCS: &str = "/app/v3/api/knowledge/git_syncs";
pub const INGESTS: &str = "/app/v3/api/knowledge/ingests";
pub const INGEST: &str = "/app/v3/api/knowledge/ingests/{ingest_id}";
pub const DOCUMENTS: &str = "/app/v3/api/knowledge/documents";
pub const DOCUMENT: &str = "/app/v3/api/knowledge/documents/{document_id}";
pub const DOCUMENT_CONTENT: &str = "/app/v3/api/knowledge/documents/{document_id}/content";
pub const DOCUMENT_VERSIONS: &str = "/app/v3/api/knowledge/documents/{document_id}/versions";
pub const OKF_CONCEPT_UPSERT: &str = "/app/v3/api/knowledge/okf/concepts/upsert";
pub const OKF_CONCEPTS: &str = "/app/v3/api/knowledge/okf/concepts";
pub const OKF_CONCEPT: &str = "/app/v3/api/knowledge/okf/concepts/{concept_id}";
pub const OKF_CONCEPT_REVISIONS: &str = "/app/v3/api/knowledge/okf/concepts/{concept_id}/revisions";
pub const OKF_INDEX: &str = "/app/v3/api/knowledge/okf/index";
pub const OKF_LOG: &str = "/app/v3/api/knowledge/okf/log";
pub const OKF_PROFILE: &str = "/app/v3/api/knowledge/okf/profile";
pub const OKF_QUERIES: &str = "/app/v3/api/knowledge/okf/queries";
pub const OKF_QUERY_FILE_ANSWER: &str = "/app/v3/api/knowledge/okf/queries/{query_id}/file_answer";
pub const OKF_CONTEXT_PACKS: &str = "/app/v3/api/knowledge/okf/context_packs";
pub const OKF_EXPORTS: &str = "/app/v3/api/knowledge/okf/exports";
pub const OKF_EXPORT: &str = "/app/v3/api/knowledge/okf/exports/{export_id}";
pub const OKF_IMPORTS: &str = "/app/v3/api/knowledge/okf/imports";
pub const OKF_LINT_RUNS: &str = "/app/v3/api/knowledge/okf/lint_runs";
pub const SPACE_BROWSER: &str = "/app/v3/api/knowledge/spaces/{space_id}/browser";
pub const RETRIEVALS: &str = "/app/v3/api/knowledge/retrievals";
pub const RETRIEVAL: &str = "/app/v3/api/knowledge/retrievals/{retrieval_id}";
pub const CONTEXT_PACKS: &str = "/app/v3/api/knowledge/context_packs";
pub const AGENT_PROFILES: &str = "/app/v3/api/knowledge/agent_profiles";
pub const AGENT_PROFILE: &str = "/app/v3/api/knowledge/agent_profiles/{profile_id}";
pub const AGENT_PROFILE_BINDINGS: &str =
    "/app/v3/api/knowledge/agent_profiles/{profile_id}/bindings";
pub const AGENT_PROFILE_BINDING: &str =
    "/app/v3/api/knowledge/agent_profiles/{profile_id}/bindings/{binding_id}";
pub const AGENT_PROFILE_RETRIEVAL_PREVIEW: &str =
    "/app/v3/api/knowledge/agent_profiles/{profile_id}/retrieval_preview";
pub const AGENT_PROFILE_CHAT: &str = "/app/v3/api/knowledge/agent_profiles/{profile_id}/chat";
pub const SPACE_CONTEXT_BINDINGS: &str = "/app/v3/api/knowledge/spaces/{space_id}/context_bindings";
pub const SPACE_MEMBERS: &str = "/app/v3/api/knowledge/spaces/{space_id}/members";
pub const CONTEXT_BINDING: &str = "/app/v3/api/knowledge/context_bindings/{binding_id}";
pub const WECHAT_OFFICIAL_ACCOUNTS: &str = "/app/v3/api/knowledge/wechat/official_accounts";
pub const WECHAT_OFFICIAL_ACCOUNT_FAN_TAGS: &str =
    "/app/v3/api/knowledge/wechat/official_accounts/{accountId}/fan_tags";
pub const WECHAT_APPLETS: &str = "/app/v3/api/knowledge/wechat/applets";
pub const WECHAT_ARTICLES_PUBLISH: &str = "/app/v3/api/knowledge/wechat/articles/publish";
pub const WECHAT_ARTICLES_PREVIEW: &str = "/app/v3/api/knowledge/wechat/articles/preview";
pub const MARKET_LISTINGS: &str = "/app/v3/api/knowledge/market/listings";
pub const MARKET_SUBSCRIPTIONS: &str = "/app/v3/api/knowledge/market/subscriptions";
pub const MARKET_SUBSCRIPTION: &str = "/app/v3/api/knowledge/market/subscriptions/{listing_id}";
pub const MEDIA_TASKS: &str = "/app/v3/api/knowledge/media_tasks";
