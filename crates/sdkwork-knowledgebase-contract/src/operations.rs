pub const SPACES_CREATE: &str = "spaces.create";
pub const SPACES_RETRIEVE: &str = "spaces.retrieve";
pub const SOURCES_LIST: &str = "sources.list";
pub const SOURCES_CREATE: &str = "sources.create";
pub const DOCUMENTS_LIST: &str = "documents.list";
pub const DOCUMENTS_CREATE: &str = "documents.create";
pub const DOCUMENTS_RETRIEVE: &str = "documents.retrieve";
pub const DOCUMENTS_UPDATE: &str = "documents.update";
pub const DOCUMENTS_DELETE: &str = "documents.delete";
pub const DOCUMENTS_VERSIONS_CREATE: &str = "documents.versions.create";
pub const DOCUMENTS_VERSIONS_LIST: &str = "documents.versions.list";
pub const DOCUMENTS_CONTENT_RETRIEVE: &str = "documents.content.list";
pub const DRIVE_IMPORTS_CREATE: &str = "driveImports.create";
pub const INGESTS_CREATE: &str = "ingests.create";
pub const INGESTS_RETRIEVE: &str = "ingests.retrieve";
pub const SPACES_BROWSER_LIST: &str = "spaces.browser.list";
pub const RETRIEVALS_CREATE: &str = "retrievals.create";
pub const RETRIEVALS_RETRIEVE: &str = "retrievals.retrieve";
pub const CONTEXT_PACKS_CREATE: &str = "contextPacks.create";
pub const AGENT_PROFILES_CREATE: &str = "agentProfiles.create";
pub const AGENT_PROFILES_RETRIEVE: &str = "agentProfiles.retrieve";
pub const AGENT_PROFILES_UPDATE: &str = "agentProfiles.update";
pub const AGENT_PROFILES_DELETE: &str = "agentProfiles.delete";
pub const AGENT_PROFILES_BINDINGS_LIST: &str = "agentProfiles.bindings.list";
pub const AGENT_PROFILES_BINDINGS_CREATE: &str = "agentProfiles.bindings.create";
pub const AGENT_PROFILES_BINDINGS_UPDATE: &str = "agentProfiles.bindings.update";
pub const AGENT_PROFILES_BINDINGS_DELETE: &str = "agentProfiles.bindings.delete";
pub const AGENT_PROFILES_RETRIEVAL_PREVIEW_CREATE: &str =
    "agentProfiles.retrievalPreview.create";
pub const AGENT_PROFILES_CHAT_CREATE: &str = "agentProfiles.chat.create";
pub const OKF_CONCEPTS_LIST: &str = "okf.concepts.list";
pub const OKF_CONCEPTS_RETRIEVE: &str = "okf.concepts.retrieve";
pub const OKF_CONCEPTS_UPSERT: &str = "okf.concepts.update";
pub const OKF_CONCEPTS_REVISIONS_LIST: &str = "okf.concepts.revisions.list";
pub const OKF_CONCEPTS_PUBLISH: &str = "okf.concepts.publish";
pub const OKF_BUNDLE_INDEX_RETRIEVE: &str = "okf.bundle.index.list";
pub const OKF_BUNDLE_INDEX_REBUILD: &str = "okf.bundle.index.rebuild";
pub const OKF_BUNDLE_LOG_RETRIEVE: &str = "okf.bundle.log.list";
pub const OKF_LOG_ENTRIES_CREATE: &str = "okf.log.entries.create";
pub const OKF_BUNDLE_PROFILE_RETRIEVE: &str = "okf.bundle.profile.list";
pub const OKF_PROFILE_CREATE: &str = "okf.profile.create";
pub const OKF_PROFILE_UPDATE: &str = "okf.profile.update";
pub const OKF_QUERIES_CREATE: &str = "okf.queries.create";
pub const OKF_QUERIES_FILE_ANSWER: &str = "okf.queries.fileAnswer";
pub const OKF_CONTEXT_PACKS_CREATE: &str = "okf.contextPacks.create";
pub const OKF_COMPILE_JOBS_CREATE: &str = "okf.compileJobs.create";
pub const OKF_CANDIDATES_LIST: &str = "okf.candidates.list";
pub const OKF_CANDIDATES_APPROVE: &str = "okf.candidates.approve";
pub const OKF_CANDIDATES_REJECT: &str = "okf.candidates.reject";
pub const OKF_BUNDLE_EXPORT_CREATE: &str = "okf.bundle.export.create";
pub const OKF_BUNDLE_EXPORT_RETRIEVE: &str = "okf.bundle.export.retrieve";
pub const OKF_BUNDLE_IMPORT_CREATE: &str = "okf.bundle.import.create";
pub const OKF_BUNDLE_FILES_LIST: &str = "okf.bundle.files.list";
pub const OKF_LINT_RUNS_CREATE: &str = "okf.lintRuns.create";
pub const OKF_EVAL_RUNS_CREATE: &str = "okf.evalRuns.create";
pub const INDEXES_CREATE: &str = "indexes.create";
pub const INDEXES_RETRIEVE: &str = "indexes.retrieve";
pub const INDEXES_REBUILD: &str = "indexes.rebuild";
pub const RETRIEVAL_PROFILES_CREATE: &str = "retrievalProfiles.create";
pub const RETRIEVAL_PROFILES_RETRIEVE: &str = "retrievalProfiles.retrieve";
pub const RETRIEVAL_PROFILES_UPDATE: &str = "retrievalProfiles.update";
pub const RETRIEVAL_TRACES_LIST: &str = "retrievalTraces.list";
pub const RETRIEVAL_TRACES_RETRIEVE: &str = "retrievalTraces.retrieve";
pub const CONTEXT_BINDINGS_RETRIEVE: &str = "contextBindings.retrieve";
pub const CONTEXT_BINDINGS_UPDATE: &str = "contextBindings.update";
pub const CONTEXT_BINDINGS_DELETE: &str = "contextBindings.delete";
pub const SPACES_CONTEXT_BINDINGS_LIST: &str = "spaces.contextBindings.list";
pub const SPACES_CONTEXT_BINDINGS_CREATE: &str = "spaces.contextBindings.create";
pub const PROVIDER_HEALTH_RETRIEVE: &str = "providerHealth.list";

pub const ALL_OPERATION_IDS: &[&str] = &[
    SPACES_CREATE,
    SPACES_RETRIEVE,
    SOURCES_LIST,
    SOURCES_CREATE,
    DOCUMENTS_LIST,
    DOCUMENTS_CREATE,
    DOCUMENTS_RETRIEVE,
    DOCUMENTS_UPDATE,
    DOCUMENTS_DELETE,
    DOCUMENTS_VERSIONS_CREATE,
    DOCUMENTS_VERSIONS_LIST,
    DOCUMENTS_CONTENT_RETRIEVE,
    DRIVE_IMPORTS_CREATE,
    INGESTS_CREATE,
    INGESTS_RETRIEVE,
    SPACES_BROWSER_LIST,
    RETRIEVALS_CREATE,
    RETRIEVALS_RETRIEVE,
    CONTEXT_PACKS_CREATE,
    AGENT_PROFILES_CREATE,
    AGENT_PROFILES_RETRIEVE,
    AGENT_PROFILES_UPDATE,
    AGENT_PROFILES_DELETE,
    AGENT_PROFILES_BINDINGS_LIST,
    AGENT_PROFILES_BINDINGS_CREATE,
    AGENT_PROFILES_BINDINGS_UPDATE,
    AGENT_PROFILES_BINDINGS_DELETE,
    AGENT_PROFILES_RETRIEVAL_PREVIEW_CREATE,
    AGENT_PROFILES_CHAT_CREATE,
    OKF_CONCEPTS_LIST,
    OKF_CONCEPTS_RETRIEVE,
    OKF_CONCEPTS_UPSERT,
    OKF_CONCEPTS_REVISIONS_LIST,
    OKF_CONCEPTS_PUBLISH,
    OKF_BUNDLE_INDEX_RETRIEVE,
    OKF_BUNDLE_INDEX_REBUILD,
    OKF_BUNDLE_LOG_RETRIEVE,
    OKF_LOG_ENTRIES_CREATE,
    OKF_BUNDLE_PROFILE_RETRIEVE,
    OKF_PROFILE_CREATE,
    OKF_PROFILE_UPDATE,
    OKF_QUERIES_CREATE,
    OKF_QUERIES_FILE_ANSWER,
    OKF_CONTEXT_PACKS_CREATE,
    OKF_COMPILE_JOBS_CREATE,
    OKF_CANDIDATES_LIST,
    OKF_CANDIDATES_APPROVE,
    OKF_CANDIDATES_REJECT,
    OKF_BUNDLE_EXPORT_CREATE,
    OKF_BUNDLE_EXPORT_RETRIEVE,
    OKF_BUNDLE_IMPORT_CREATE,
    OKF_BUNDLE_FILES_LIST,
    OKF_LINT_RUNS_CREATE,
    OKF_EVAL_RUNS_CREATE,
    INDEXES_CREATE,
    INDEXES_RETRIEVE,
    INDEXES_REBUILD,
    RETRIEVAL_PROFILES_CREATE,
    RETRIEVAL_PROFILES_RETRIEVE,
    RETRIEVAL_PROFILES_UPDATE,
    RETRIEVAL_TRACES_LIST,
    RETRIEVAL_TRACES_RETRIEVE,
    SPACES_CONTEXT_BINDINGS_LIST,
    SPACES_CONTEXT_BINDINGS_CREATE,
    CONTEXT_BINDINGS_RETRIEVE,
    CONTEXT_BINDINGS_UPDATE,
    CONTEXT_BINDINGS_DELETE,
    PROVIDER_HEALTH_RETRIEVE,
];
