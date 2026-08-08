-- SDKWork knowledgebase consolidated initialization baseline (sqlite)
-- Application is in initialization state: this file is a SQLite full DDL snapshot; database/migrations/sqlite is reserved for post-GA changes.

-- source: crates/sdkwork-intelligence-knowledgebase-repository-sqlx/migrations/sqlite/V202606010001__knowledgebase_core.sql (folded with pre-GA access-mode and agent-runtime columns)
CREATE TABLE IF NOT EXISTS kb_space (
    id BIGINT NOT NULL,
    uuid TEXT NOT NULL,
    tenant_id INTEGER NOT NULL,
    organization_id INTEGER NOT NULL DEFAULT 0,
    name TEXT NOT NULL,
    description TEXT,
    drive_space_id TEXT,
    status INTEGER NOT NULL,
    okf_bundle_initialized INTEGER NOT NULL DEFAULT 0,
    okf_log_sequence_counter INTEGER NOT NULL DEFAULT 0,
    knowledge_mode TEXT NOT NULL DEFAULT 'okf_bundle',
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL,
    version INTEGER NOT NULL DEFAULT 0,
    PRIMARY KEY (id)
);

CREATE UNIQUE INDEX IF NOT EXISTS uk_kb_space_uuid
    ON kb_space (tenant_id, uuid);

CREATE UNIQUE INDEX IF NOT EXISTS uk_kb_space_drive_space
    ON kb_space (tenant_id, drive_space_id)
    WHERE drive_space_id IS NOT NULL AND status = 1;

CREATE TABLE IF NOT EXISTS kb_collection (
    id BIGINT NOT NULL,
    uuid TEXT NOT NULL,
    tenant_id INTEGER NOT NULL,
    space_id INTEGER NOT NULL,
    parent_id INTEGER NOT NULL DEFAULT 0,
    name TEXT NOT NULL,
    path TEXT NOT NULL,
    level_no INTEGER NOT NULL,
    sort_order INTEGER NOT NULL DEFAULT 0,
    status INTEGER NOT NULL,
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL,
    version INTEGER NOT NULL DEFAULT 0,
    PRIMARY KEY (id)
);

CREATE UNIQUE INDEX IF NOT EXISTS uk_kb_collection_uuid
    ON kb_collection (tenant_id, uuid);

CREATE TABLE IF NOT EXISTS kb_source (
    id BIGINT NOT NULL,
    uuid TEXT NOT NULL,
    tenant_id INTEGER NOT NULL,
    space_id INTEGER NOT NULL,
    source_type TEXT NOT NULL,
    provider TEXT,
    drive_bucket TEXT,
    drive_prefix TEXT,
    sync_policy TEXT,
    last_sync_at TEXT,
    last_sync_job_id INTEGER,
    metadata TEXT,
    status INTEGER NOT NULL,
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL,
    version INTEGER NOT NULL DEFAULT 0,
    PRIMARY KEY (id)
);

CREATE UNIQUE INDEX IF NOT EXISTS uk_kb_source_uuid
    ON kb_source (tenant_id, uuid);

CREATE UNIQUE INDEX IF NOT EXISTS uk_kb_source_identity
    ON kb_source (
        tenant_id,
        space_id,
        source_type,
        COALESCE(provider, ''),
        COALESCE(drive_bucket, ''),
        COALESCE(drive_prefix, '')
    )
    WHERE status = 1;

CREATE TABLE IF NOT EXISTS kb_drive_object_ref (
    id BIGINT NOT NULL,
    uuid TEXT NOT NULL,
    tenant_id INTEGER NOT NULL,
    space_id INTEGER NOT NULL,
    drive_provider_kind TEXT NOT NULL,
    drive_space_id TEXT,
    drive_node_id TEXT,
    logical_path TEXT,
    drive_storage_provider_id TEXT NOT NULL,
    drive_bucket TEXT NOT NULL,
    drive_object_key TEXT NOT NULL,
    drive_object_version TEXT,
    drive_etag TEXT,
    content_type TEXT,
    size_bytes INTEGER NOT NULL,
    checksum_sha256_hex TEXT,
    drive_metadata TEXT,
    object_role TEXT NOT NULL,
    access_mode TEXT NOT NULL,
    status INTEGER NOT NULL,
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL,
    version INTEGER NOT NULL DEFAULT 0,
    PRIMARY KEY (id)
);

CREATE UNIQUE INDEX IF NOT EXISTS uk_kb_drive_object_ref_uuid
    ON kb_drive_object_ref (tenant_id, uuid);

CREATE INDEX IF NOT EXISTS idx_kb_drive_object_locator
    ON kb_drive_object_ref (
        tenant_id,
        drive_storage_provider_id,
        drive_bucket,
        drive_object_key,
        drive_object_version
    );

CREATE UNIQUE INDEX IF NOT EXISTS uk_kb_drive_object_ref_locator
    ON kb_drive_object_ref (
        tenant_id,
        space_id,
        drive_storage_provider_id,
        drive_bucket,
        drive_object_key,
        COALESCE(drive_object_version, ''),
        object_role
    );

CREATE INDEX IF NOT EXISTS idx_kb_drive_object_role
    ON kb_drive_object_ref (tenant_id, object_role, created_at);

CREATE INDEX IF NOT EXISTS idx_kb_drive_object_drive_node
    ON kb_drive_object_ref (tenant_id, space_id, drive_space_id, drive_node_id, status);

CREATE TABLE IF NOT EXISTS kb_document (
    id BIGINT NOT NULL,
    uuid TEXT NOT NULL,
    tenant_id INTEGER NOT NULL,
    space_id INTEGER NOT NULL,
    collection_id INTEGER NOT NULL DEFAULT 0,
    source_id INTEGER,
    identity_scope TEXT NOT NULL DEFAULT 'source_and_original_drive_node',
    original_file_drive_node_id TEXT,
    title TEXT NOT NULL,
    mime_type TEXT,
    language TEXT,
    current_version_id INTEGER,
    visibility INTEGER NOT NULL,
    content_state INTEGER NOT NULL,
    index_state INTEGER NOT NULL,
    metadata TEXT,
    status INTEGER NOT NULL,
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL,
    version INTEGER NOT NULL DEFAULT 0,
    PRIMARY KEY (id)
);

CREATE UNIQUE INDEX IF NOT EXISTS uk_kb_document_uuid
    ON kb_document (tenant_id, uuid);

CREATE INDEX IF NOT EXISTS idx_kb_document_drive_node
    ON kb_document (tenant_id, space_id, original_file_drive_node_id, status);

CREATE UNIQUE INDEX IF NOT EXISTS uk_kb_document_identity
    ON kb_document (
        tenant_id,
        space_id,
        collection_id,
        identity_scope,
        COALESCE(source_id, 0),
        CASE
            WHEN identity_scope = 'source_only' THEN ''
            ELSE COALESCE(original_file_drive_node_id, '')
        END
    )
    WHERE status = 1;

CREATE TABLE IF NOT EXISTS kb_document_version (
    id BIGINT NOT NULL,
    uuid TEXT NOT NULL,
    tenant_id INTEGER NOT NULL,
    document_id INTEGER NOT NULL,
    version_no INTEGER NOT NULL,
    original_object_ref_id INTEGER NOT NULL,
    checksum_sha256_hex TEXT,
    size_bytes INTEGER NOT NULL,
    mime_type TEXT,
    parser_profile_id INTEGER,
    parse_state INTEGER NOT NULL,
    index_state INTEGER NOT NULL,
    submitted_by INTEGER,
    submitted_at TEXT NOT NULL,
    metadata TEXT,
    status INTEGER NOT NULL,
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL,
    version INTEGER NOT NULL DEFAULT 0,
    PRIMARY KEY (id)
);

CREATE UNIQUE INDEX IF NOT EXISTS uk_kb_document_version_uuid
    ON kb_document_version (tenant_id, uuid);

CREATE UNIQUE INDEX IF NOT EXISTS uk_kb_document_version_no
    ON kb_document_version (tenant_id, document_id, version_no);

CREATE TABLE IF NOT EXISTS kb_chunk (
    id BIGINT NOT NULL,
    uuid TEXT NOT NULL,
    tenant_id INTEGER NOT NULL,
    space_id INTEGER NOT NULL,
    collection_id INTEGER NOT NULL DEFAULT 0,
    document_id INTEGER NOT NULL,
    document_version_id INTEGER NOT NULL,
    chunk_index INTEGER NOT NULL,
    content_text TEXT NOT NULL,
    content_hash TEXT NOT NULL,
    search_vector TEXT,
    token_count INTEGER,
    locator TEXT,
    metadata TEXT,
    status INTEGER NOT NULL,
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL,
    version INTEGER NOT NULL DEFAULT 0,
    PRIMARY KEY (id)
);

CREATE UNIQUE INDEX IF NOT EXISTS uk_kb_chunk_uuid
    ON kb_chunk (tenant_id, uuid);

CREATE UNIQUE INDEX IF NOT EXISTS uk_kb_chunk_document_version_chunk
    ON kb_chunk (tenant_id, document_version_id, chunk_index);

CREATE INDEX IF NOT EXISTS idx_kb_chunk_document_version
    ON kb_chunk (tenant_id, document_version_id, status, chunk_index);

CREATE INDEX IF NOT EXISTS idx_kb_chunk_space_status
    ON kb_chunk (tenant_id, space_id, collection_id, status);

CREATE TABLE IF NOT EXISTS kb_index (
    id BIGINT NOT NULL,
    uuid TEXT NOT NULL,
    tenant_id INTEGER NOT NULL,
    space_id INTEGER NOT NULL,
    collection_id INTEGER NOT NULL DEFAULT 0,
    index_kind TEXT NOT NULL,
    embedding_provider_id TEXT,
    embedding_model TEXT,
    dimension INTEGER,
    metric TEXT,
    schema_version TEXT NOT NULL,
    metadata TEXT,
    status INTEGER NOT NULL,
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL,
    version INTEGER NOT NULL DEFAULT 0,
    PRIMARY KEY (id)
);

CREATE UNIQUE INDEX IF NOT EXISTS uk_kb_index_uuid
    ON kb_index (tenant_id, uuid);

CREATE INDEX IF NOT EXISTS idx_kb_index_scope
    ON kb_index (tenant_id, space_id, collection_id, index_kind, status);

CREATE TABLE IF NOT EXISTS kb_embedding (
    id BIGINT NOT NULL,
    uuid TEXT NOT NULL,
    tenant_id INTEGER NOT NULL,
    index_id INTEGER NOT NULL,
    chunk_id INTEGER NOT NULL,
    embedding_hash TEXT NOT NULL,
    vector_ref TEXT NOT NULL,
    dimension INTEGER NOT NULL,
    provider_id TEXT,
    model TEXT,
    metadata TEXT,
    vector_json TEXT,
    embedding_vector TEXT,
    status INTEGER NOT NULL,
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL,
    version INTEGER NOT NULL DEFAULT 0,
    PRIMARY KEY (id)
);

CREATE UNIQUE INDEX IF NOT EXISTS uk_kb_embedding_uuid
    ON kb_embedding (tenant_id, uuid);

CREATE UNIQUE INDEX IF NOT EXISTS uk_kb_embedding_index_chunk
    ON kb_embedding (tenant_id, index_id, chunk_id);

CREATE INDEX IF NOT EXISTS idx_kb_embedding_chunk
    ON kb_embedding (tenant_id, chunk_id, status);

CREATE TABLE IF NOT EXISTS kb_retrieval_profile (
    id BIGINT NOT NULL,
    uuid TEXT NOT NULL,
    tenant_id INTEGER NOT NULL,
    name TEXT NOT NULL,
    strategy TEXT NOT NULL,
    top_k INTEGER NOT NULL,
    min_score REAL,
    rerank_enabled INTEGER NOT NULL DEFAULT 0,
    rerank_provider_id TEXT,
    query_rewrite_enabled INTEGER NOT NULL DEFAULT 0,
    citation_policy TEXT,
    filter_policy TEXT,
    context_budget_tokens INTEGER NOT NULL,
    metadata TEXT,
    status INTEGER NOT NULL,
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL,
    version INTEGER NOT NULL DEFAULT 0,
    PRIMARY KEY (id)
);

CREATE UNIQUE INDEX IF NOT EXISTS uk_kb_retrieval_profile_uuid
    ON kb_retrieval_profile (tenant_id, uuid);

CREATE INDEX IF NOT EXISTS idx_kb_retrieval_profile_tenant_status
    ON kb_retrieval_profile (tenant_id, status, updated_at);

CREATE TABLE IF NOT EXISTS kb_retrieval_trace (
    id BIGINT NOT NULL,
    uuid TEXT NOT NULL,
    tenant_id INTEGER NOT NULL,
    actor_id INTEGER,
    retrieval_profile_id INTEGER,
    query_hash TEXT NOT NULL,
    query_text_redacted TEXT,
    request_payload TEXT,
    latency_ms INTEGER,
    result_count INTEGER NOT NULL DEFAULT 0,
    error_code TEXT,
    error_detail TEXT,
    status INTEGER NOT NULL,
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL,
    version INTEGER NOT NULL DEFAULT 0,
    PRIMARY KEY (id)
);

CREATE UNIQUE INDEX IF NOT EXISTS uk_kb_retrieval_trace_uuid
    ON kb_retrieval_trace (tenant_id, uuid);

CREATE INDEX IF NOT EXISTS idx_kb_retrieval_trace_profile_created
    ON kb_retrieval_trace (tenant_id, retrieval_profile_id, created_at);

CREATE INDEX IF NOT EXISTS idx_kb_retrieval_trace_actor_created
    ON kb_retrieval_trace (tenant_id, actor_id, created_at);

CREATE TABLE IF NOT EXISTS kb_retrieval_hit (
    id BIGINT NOT NULL,
    uuid TEXT NOT NULL,
    tenant_id INTEGER NOT NULL,
    retrieval_trace_id INTEGER NOT NULL,
    chunk_id INTEGER NOT NULL,
    document_id INTEGER NOT NULL,
    document_version_id INTEGER,
    score REAL,
    result_rank INTEGER NOT NULL,
    match_reason TEXT,
    citation TEXT,
    metadata TEXT,
    status INTEGER NOT NULL,
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL,
    version INTEGER NOT NULL DEFAULT 0,
    PRIMARY KEY (id)
);

CREATE UNIQUE INDEX IF NOT EXISTS uk_kb_retrieval_hit_uuid
    ON kb_retrieval_hit (tenant_id, uuid);

CREATE INDEX IF NOT EXISTS idx_kb_retrieval_hit_trace_rank
    ON kb_retrieval_hit (tenant_id, retrieval_trace_id, result_rank);

CREATE INDEX IF NOT EXISTS idx_kb_retrieval_hit_chunk
    ON kb_retrieval_hit (tenant_id, chunk_id, status);

CREATE TABLE IF NOT EXISTS kb_agent_profile (
    id BIGINT NOT NULL,
    uuid TEXT NOT NULL,
    tenant_id INTEGER NOT NULL,
    name TEXT NOT NULL,
    description TEXT,
    system_instruction TEXT NOT NULL,
    model_provider_id TEXT NOT NULL,
    model_id TEXT NOT NULL,
    model_parameters TEXT,
    retrieval_profile_id INTEGER,
    citation_policy TEXT,
    memory_policy_ref TEXT,
    tool_policy_ref TEXT,
    answer_policy TEXT,
    metadata TEXT,
    knowledge_mode TEXT NOT NULL DEFAULT 'okf_bundle',
    agent_implementation_id TEXT NOT NULL DEFAULT 'plugin.intelligence.rig',
    status INTEGER NOT NULL,
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL,
    version INTEGER NOT NULL DEFAULT 0,
    PRIMARY KEY (id)
);

CREATE UNIQUE INDEX IF NOT EXISTS uk_kb_agent_profile_uuid
    ON kb_agent_profile (tenant_id, uuid);

CREATE INDEX IF NOT EXISTS idx_kb_agent_profile_model
    ON kb_agent_profile (tenant_id, model_provider_id, model_id, status);

CREATE TABLE IF NOT EXISTS kb_agent_knowledge_binding (
    id BIGINT NOT NULL,
    uuid TEXT NOT NULL,
    tenant_id INTEGER NOT NULL,
    profile_id INTEGER NOT NULL,
    space_id INTEGER NOT NULL,
    collection_id INTEGER,
    source_filter TEXT,
    document_filter TEXT,
    priority INTEGER NOT NULL DEFAULT 0,
    top_k INTEGER,
    min_score REAL,
    enabled INTEGER NOT NULL DEFAULT 1,
    status INTEGER NOT NULL,
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL,
    version INTEGER NOT NULL DEFAULT 0,
    PRIMARY KEY (id)
);

CREATE UNIQUE INDEX IF NOT EXISTS uk_kb_agent_knowledge_binding_uuid
    ON kb_agent_knowledge_binding (tenant_id, uuid);

CREATE INDEX IF NOT EXISTS idx_kb_agent_knowledge_binding_profile
    ON kb_agent_knowledge_binding (tenant_id, profile_id, enabled, priority);

CREATE TABLE IF NOT EXISTS kb_ingestion_job (
    id BIGINT NOT NULL,
    uuid TEXT NOT NULL,
    tenant_id INTEGER NOT NULL,
    space_id INTEGER NOT NULL,
    source_id INTEGER,
    job_type TEXT NOT NULL,
    state INTEGER NOT NULL,
    priority INTEGER NOT NULL DEFAULT 0,
    progress INTEGER NOT NULL DEFAULT 0,
    requested_by INTEGER,
    idempotency_key TEXT NOT NULL,
    request_id TEXT,
    trace_id TEXT,
    error_code TEXT,
    error_detail TEXT,
    started_at TEXT,
    finished_at TEXT,
    metadata TEXT,
    status INTEGER NOT NULL,
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL,
    version INTEGER NOT NULL DEFAULT 0,
    PRIMARY KEY (id)
);

CREATE UNIQUE INDEX IF NOT EXISTS uk_kb_ingestion_job_uuid
    ON kb_ingestion_job (tenant_id, uuid);

CREATE UNIQUE INDEX IF NOT EXISTS uk_kb_ingestion_job_idempotency
    ON kb_ingestion_job (tenant_id, space_id, idempotency_key);

CREATE TABLE IF NOT EXISTS kb_ingestion_job_item (
    id BIGINT NOT NULL,
    uuid TEXT NOT NULL,
    tenant_id INTEGER NOT NULL,
    job_id INTEGER NOT NULL,
    document_id INTEGER,
    document_version_id INTEGER,
    input_object_ref_id INTEGER,
    stage TEXT NOT NULL,
    state INTEGER NOT NULL,
    attempt_count INTEGER NOT NULL DEFAULT 0,
    error_code TEXT,
    error_detail TEXT,
    started_at TEXT,
    finished_at TEXT,
    status INTEGER NOT NULL,
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL,
    version INTEGER NOT NULL DEFAULT 0,
    PRIMARY KEY (id)
);

CREATE UNIQUE INDEX IF NOT EXISTS uk_kb_ingestion_job_item_uuid
    ON kb_ingestion_job_item (tenant_id, uuid);

CREATE TABLE IF NOT EXISTS kb_okf_concept (
    id BIGINT NOT NULL,
    uuid TEXT NOT NULL,
    tenant_id INTEGER NOT NULL,
    space_id INTEGER NOT NULL,
    concept_id TEXT NOT NULL,
    title TEXT NOT NULL,
    concept_type TEXT NOT NULL,
    logical_path TEXT NOT NULL,
    description TEXT NOT NULL DEFAULT '',
    source_count INTEGER NOT NULL DEFAULT 0,
    tags TEXT,
    current_revision_id INTEGER,
    publish_state TEXT NOT NULL,
    revision_counter INTEGER NOT NULL DEFAULT 0,
    status INTEGER NOT NULL,
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL,
    version INTEGER NOT NULL DEFAULT 0,
    PRIMARY KEY (id)
);

CREATE UNIQUE INDEX IF NOT EXISTS uk_kb_okf_concept_uuid
    ON kb_okf_concept (tenant_id, uuid);

CREATE UNIQUE INDEX IF NOT EXISTS uk_kb_okf_concept_id
    ON kb_okf_concept (tenant_id, space_id, concept_id);

CREATE UNIQUE INDEX IF NOT EXISTS uk_kb_okf_concept_path
    ON kb_okf_concept (tenant_id, space_id, logical_path);

CREATE INDEX IF NOT EXISTS idx_kb_okf_concept_state
    ON kb_okf_concept (tenant_id, space_id, publish_state, updated_at);

CREATE TABLE IF NOT EXISTS kb_okf_concept_revision (
    id BIGINT NOT NULL,
    uuid TEXT NOT NULL,
    tenant_id INTEGER NOT NULL,
    concept_row_id INTEGER NOT NULL,
    revision_no INTEGER NOT NULL,
    markdown_object_ref_id INTEGER NOT NULL,
    content_hash TEXT NOT NULL,
    review_state TEXT NOT NULL,
    status INTEGER NOT NULL,
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL,
    version INTEGER NOT NULL DEFAULT 0,
    PRIMARY KEY (id)
);

CREATE UNIQUE INDEX IF NOT EXISTS uk_kb_okf_concept_revision_uuid
    ON kb_okf_concept_revision (tenant_id, uuid);

CREATE UNIQUE INDEX IF NOT EXISTS uk_kb_okf_concept_revision_no
    ON kb_okf_concept_revision (tenant_id, concept_row_id, revision_no);

CREATE TABLE IF NOT EXISTS kb_okf_bundle_file (
    id BIGINT NOT NULL,
    uuid TEXT NOT NULL,
    tenant_id INTEGER NOT NULL,
    space_id INTEGER NOT NULL,
    logical_path TEXT NOT NULL,
    file_kind TEXT NOT NULL,
    artifact_role TEXT NOT NULL,
    drive_bucket TEXT NOT NULL,
    drive_object_key TEXT NOT NULL,
    checksum_sha256_hex TEXT,
    status INTEGER NOT NULL,
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL,
    version INTEGER NOT NULL DEFAULT 0,
    PRIMARY KEY (id)
);

CREATE UNIQUE INDEX IF NOT EXISTS uk_kb_okf_bundle_file_uuid
    ON kb_okf_bundle_file (tenant_id, uuid);

CREATE UNIQUE INDEX IF NOT EXISTS uk_kb_okf_bundle_file_path
    ON kb_okf_bundle_file (tenant_id, space_id, logical_path);

CREATE TABLE IF NOT EXISTS kb_okf_schema_profile (
    id BIGINT NOT NULL,
    uuid TEXT NOT NULL,
    tenant_id INTEGER NOT NULL,
    space_id INTEGER NOT NULL,
    profile_version TEXT NOT NULL,
    schema_object_ref_id INTEGER NOT NULL,
    agents_md_object_ref_id INTEGER NOT NULL,
    state TEXT NOT NULL,
    status INTEGER NOT NULL,
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL,
    version INTEGER NOT NULL DEFAULT 0,
    PRIMARY KEY (id)
);

CREATE UNIQUE INDEX IF NOT EXISTS uk_kb_okf_schema_profile_uuid
    ON kb_okf_schema_profile (tenant_id, uuid);

CREATE TABLE IF NOT EXISTS kb_okf_log_entry (
    id BIGINT NOT NULL,
    uuid TEXT NOT NULL,
    tenant_id INTEGER NOT NULL,
    space_id INTEGER NOT NULL,
    sequence_no INTEGER NOT NULL,
    event_type TEXT NOT NULL,
    event_time TEXT NOT NULL,
    title TEXT NOT NULL,
    privacy_level TEXT NOT NULL,
    metadata TEXT,
    status INTEGER NOT NULL,
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL,
    version INTEGER NOT NULL DEFAULT 0,
    PRIMARY KEY (id)
);

CREATE UNIQUE INDEX IF NOT EXISTS uk_kb_okf_log_entry_uuid
    ON kb_okf_log_entry (tenant_id, uuid);

CREATE UNIQUE INDEX IF NOT EXISTS uk_kb_okf_log_entry_sequence
    ON kb_okf_log_entry (tenant_id, space_id, sequence_no);

CREATE TABLE IF NOT EXISTS kb_local_mirror_package (
    id BIGINT NOT NULL,
    uuid TEXT NOT NULL,
    tenant_id INTEGER NOT NULL,
    space_id INTEGER NOT NULL,
    package_type TEXT NOT NULL,
    snapshot_version TEXT NOT NULL,
    object_ref_id INTEGER NOT NULL,
    manifest_object_ref_id INTEGER NOT NULL,
    checksum_sha256_hex TEXT NOT NULL,
    state TEXT NOT NULL,
    status INTEGER NOT NULL,
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL,
    version INTEGER NOT NULL DEFAULT 0,
    PRIMARY KEY (id)
);

CREATE UNIQUE INDEX IF NOT EXISTS uk_kb_local_mirror_package_uuid
    ON kb_local_mirror_package (tenant_id, uuid);

-- source: crates/sdkwork-intelligence-knowledgebase-repository-sqlx/migrations/sqlite/V202606140001__knowledgebase_context_binding.sql
-- Knowledge space context binding: maps spaces to external contexts
-- (chat groups, organizations, circles, channels, etc.)
-- Members are NOT stored here. All permission management is delegated to
-- sdkwork-drive's dr_drive_node_permission table.

CREATE TABLE IF NOT EXISTS kb_space_context_binding (
    id BIGINT NOT NULL,
    tenant_id INTEGER NOT NULL,
    space_id BIGINT NOT NULL,
    context_type TEXT NOT NULL,
    context_id TEXT NOT NULL,
    context_name TEXT,
    access_level TEXT NOT NULL DEFAULT 'reader',
    status INTEGER NOT NULL DEFAULT 1,
    created_by TEXT NOT NULL,
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL,
    version INTEGER NOT NULL DEFAULT 0,
    PRIMARY KEY (id),
    FOREIGN KEY (space_id) REFERENCES kb_space(id)
);

-- Prevent duplicate bindings for the same space-context pair
CREATE UNIQUE INDEX IF NOT EXISTS uk_kb_space_context
    ON kb_space_context_binding (tenant_id, space_id, context_type, context_id)
    WHERE status = 1;

-- Fast lookup: what spaces are bound to a given context?
CREATE INDEX IF NOT EXISTS idx_kb_space_context_lookup
    ON kb_space_context_binding (tenant_id, context_type, context_id, status);

-- Fast lookup: what contexts are bound to a given space?
CREATE INDEX IF NOT EXISTS idx_kb_space_context_space
    ON kb_space_context_binding (tenant_id, space_id, status);

-- Group knowledge spaces are an IM-integrated aggregate, not a generic context binding.
-- The binding row is reserved before space provisioning so concurrent administrator clicks
-- converge on one recoverable lifecycle rather than creating orphaned spaces.
CREATE TABLE IF NOT EXISTS kb_group_knowledge_space_binding (
    id INTEGER NOT NULL,
    uuid TEXT NOT NULL,
    tenant_id INTEGER NOT NULL,
    organization_id INTEGER NOT NULL,
    conversation_id TEXT NOT NULL,
    space_id INTEGER,
    space_uuid TEXT,
    group_name TEXT NOT NULL,
    lifecycle_state TEXT NOT NULL,
    acl_projection_state TEXT NOT NULL DEFAULT 'pending',
    provisioning_idempotency_key_sha256_hex TEXT NOT NULL,
    provisioning_lease_token TEXT,
    provisioning_lease_until TEXT,
    membership_epoch INTEGER NOT NULL DEFAULT 0,
    upstream_link_generation INTEGER NOT NULL DEFAULT 0,
    archive_source_event_id TEXT,
    archive_payload_sha256_hex TEXT,
    archive_lease_token TEXT,
    archive_lease_until TEXT,
    archive_acl_cursor TEXT,
    archive_acl_pages_processed INTEGER NOT NULL DEFAULT 0,
    archive_acl_cleanup_completed_at TEXT,
    last_source_event_id TEXT,
    last_error_code TEXT,
    last_error_at TEXT,
    archived_at TEXT,
    archived_by TEXT,
    deleted_at TEXT,
    deleted_by TEXT,
    created_by TEXT NOT NULL,
    updated_by TEXT NOT NULL,
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL,
    version INTEGER NOT NULL DEFAULT 0,
    PRIMARY KEY (id),
    UNIQUE (tenant_id, organization_id, id),
    CHECK (lifecycle_state IN ('provisioning', 'active', 'failed', 'archiving', 'archived', 'deleted')),
    CHECK (acl_projection_state IN ('pending', 'active', 'failed')),
    CHECK (lifecycle_state <> 'active' OR acl_projection_state = 'active'),
    CHECK (membership_epoch >= 0),
    CHECK (tenant_id > 0),
    CHECK (organization_id >= 0),
    FOREIGN KEY (space_id) REFERENCES kb_space(id)
);

CREATE UNIQUE INDEX IF NOT EXISTS uk_kb_group_knowledge_space_binding_uuid
    ON kb_group_knowledge_space_binding (tenant_id, uuid);

CREATE UNIQUE INDEX IF NOT EXISTS uk_kb_group_knowledge_space_binding_conversation
    ON kb_group_knowledge_space_binding (tenant_id, organization_id, conversation_id);

CREATE UNIQUE INDEX IF NOT EXISTS uk_kb_group_knowledge_space_binding_idempotency
    ON kb_group_knowledge_space_binding
       (tenant_id, organization_id, provisioning_idempotency_key_sha256_hex);

CREATE UNIQUE INDEX IF NOT EXISTS uk_kb_group_knowledge_space_binding_space
    ON kb_group_knowledge_space_binding (space_id)
    WHERE space_id IS NOT NULL;

CREATE INDEX IF NOT EXISTS idx_kb_group_knowledge_space_binding_state
    ON kb_group_knowledge_space_binding (tenant_id, organization_id, lifecycle_state, updated_at, id);

CREATE TABLE IF NOT EXISTS kb_group_knowledge_space_member (
    id INTEGER NOT NULL,
    uuid TEXT NOT NULL,
    tenant_id INTEGER NOT NULL,
    organization_id INTEGER NOT NULL,
    binding_id INTEGER NOT NULL,
    principal_kind TEXT NOT NULL,
    actor_id TEXT NOT NULL,
    member_role TEXT NOT NULL,
    access_level TEXT,
    membership_epoch INTEGER NOT NULL,
    status INTEGER NOT NULL DEFAULT 1,
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL,
    version INTEGER NOT NULL DEFAULT 0,
    PRIMARY KEY (id),
    CHECK (member_role IN ('owner', 'admin', 'member', 'guest')),
    CHECK (principal_kind = 'user'),
    CHECK (access_level IS NULL OR access_level IN ('reader', 'writer', 'owner')),
    CHECK (
        COALESCE(access_level, '') = CASE member_role
            WHEN 'owner' THEN 'owner'
            WHEN 'admin' THEN 'writer'
            WHEN 'member' THEN 'reader'
            WHEN 'guest' THEN ''
        END
    ),
    CHECK (membership_epoch >= 0),
    CHECK (tenant_id > 0),
    CHECK (organization_id >= 0),
    FOREIGN KEY (tenant_id, organization_id, binding_id)
        REFERENCES kb_group_knowledge_space_binding(tenant_id, organization_id, id)
);

CREATE UNIQUE INDEX IF NOT EXISTS uk_kb_group_knowledge_space_member_uuid
    ON kb_group_knowledge_space_member (tenant_id, uuid);

CREATE UNIQUE INDEX IF NOT EXISTS uk_kb_group_knowledge_space_member_active
    ON kb_group_knowledge_space_member (tenant_id, organization_id, binding_id, actor_id)
    WHERE status = 1;

CREATE INDEX IF NOT EXISTS idx_kb_group_knowledge_space_member_access
    ON kb_group_knowledge_space_member (tenant_id, organization_id, binding_id, actor_id, status);

CREATE TABLE IF NOT EXISTS kb_group_knowledge_space_event_inbox (
    id INTEGER NOT NULL,
    uuid TEXT NOT NULL,
    tenant_id INTEGER NOT NULL,
    organization_id INTEGER NOT NULL,
    source_event_id TEXT NOT NULL,
    event_type TEXT NOT NULL,
    binding_id INTEGER,
    payload_sha256_hex TEXT NOT NULL,
    applied_at TEXT NOT NULL,
    PRIMARY KEY (id),
    CHECK (tenant_id > 0),
    CHECK (organization_id >= 0),
    FOREIGN KEY (tenant_id, organization_id, binding_id)
        REFERENCES kb_group_knowledge_space_binding(tenant_id, organization_id, id)
);

CREATE UNIQUE INDEX IF NOT EXISTS uk_kb_group_knowledge_space_event_inbox_uuid
    ON kb_group_knowledge_space_event_inbox (tenant_id, uuid);

CREATE UNIQUE INDEX IF NOT EXISTS uk_kb_group_knowledge_space_event_inbox_source
    ON kb_group_knowledge_space_event_inbox (tenant_id, organization_id, source_event_id);

-- A membership projection is deliberately separate from the committed binding snapshot. It
-- reserves exactly one external Drive ACL mutation at a time and makes group access fail closed
-- until that mutation and the snapshot commit complete together.
CREATE TABLE IF NOT EXISTS kb_group_knowledge_space_membership_projection (
    id INTEGER NOT NULL,
    uuid TEXT NOT NULL,
    tenant_id INTEGER NOT NULL,
    organization_id INTEGER NOT NULL,
    binding_id INTEGER NOT NULL,
    source_event_id TEXT NOT NULL,
    payload_sha256_hex TEXT NOT NULL,
    target_membership_epoch INTEGER NOT NULL,
    projection_state TEXT NOT NULL,
    projection_lease_token TEXT,
    projection_lease_until TEXT,
    last_error_code TEXT,
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL,
    version INTEGER NOT NULL DEFAULT 0,
    PRIMARY KEY (id),
    CHECK (projection_state IN ('pending', 'failed', 'completed')),
    CHECK (target_membership_epoch >= 0),
    CHECK (tenant_id > 0),
    CHECK (organization_id >= 0),
    FOREIGN KEY (tenant_id, organization_id, binding_id)
        REFERENCES kb_group_knowledge_space_binding(tenant_id, organization_id, id)
);

CREATE UNIQUE INDEX IF NOT EXISTS uk_kb_group_knowledge_space_membership_projection_uuid
    ON kb_group_knowledge_space_membership_projection (tenant_id, uuid);

CREATE UNIQUE INDEX IF NOT EXISTS uk_kb_group_knowledge_space_membership_projection_event
    ON kb_group_knowledge_space_membership_projection (tenant_id, organization_id, source_event_id);

CREATE UNIQUE INDEX IF NOT EXISTS uk_kb_group_knowledge_space_membership_projection_unsettled
    ON kb_group_knowledge_space_membership_projection (tenant_id, organization_id, binding_id)
    WHERE projection_state IN ('pending', 'failed');

CREATE INDEX IF NOT EXISTS idx_kb_group_knowledge_space_membership_projection_lease
    ON kb_group_knowledge_space_membership_projection
       (tenant_id, organization_id, binding_id, projection_state, projection_lease_until);

-- source: crates/sdkwork-intelligence-knowledgebase-repository-sqlx/migrations/sqlite/V202606170001__knowledge_access_mode.sql (indexes only; columns folded into base tables)
CREATE INDEX IF NOT EXISTS idx_kb_agent_profile_knowledge_mode
    ON kb_agent_profile (tenant_id, knowledge_mode, status);

CREATE INDEX IF NOT EXISTS idx_kb_space_knowledge_mode
    ON kb_space (tenant_id, knowledge_mode, status);

-- source: crates/sdkwork-intelligence-knowledgebase-repository-sqlx/migrations/sqlite/V202606180001__agent_implementation.sql (index only; column folded into kb_agent_profile)
CREATE INDEX IF NOT EXISTS idx_kb_agent_profile_agent_implementation
    ON kb_agent_profile (tenant_id, agent_implementation_id, status);

-- source: crates/sdkwork-intelligence-knowledgebase-repository-sqlx/migrations/sqlite/V202606200001__knowledgebase_outbox.sql (folded with pre-GA delivery and claim columns)
CREATE TABLE IF NOT EXISTS kb_outbox_event (
    id INTEGER NOT NULL,
    uuid TEXT NOT NULL,
    tenant_id INTEGER NOT NULL,
    aggregate_type TEXT NOT NULL,
    aggregate_id INTEGER NOT NULL,
    event_type TEXT NOT NULL,
    payload TEXT NOT NULL,
    status INTEGER NOT NULL,
    created_at TEXT NOT NULL,
    published_at TEXT,
    last_error TEXT,
    retry_count INTEGER NOT NULL DEFAULT 0,
    claimed_at TEXT,
    version INTEGER NOT NULL DEFAULT 0,
    PRIMARY KEY (id)
);

CREATE UNIQUE INDEX IF NOT EXISTS uk_kb_outbox_event_uuid
    ON kb_outbox_event (tenant_id, uuid);

CREATE INDEX IF NOT EXISTS idx_kb_outbox_event_status_created
    ON kb_outbox_event (tenant_id, status, created_at);

-- source: crates/sdkwork-intelligence-knowledgebase-repository-sqlx/migrations/sqlite/V202606210001__okf_link_and_candidate.sql
CREATE TABLE IF NOT EXISTS kb_okf_concept_link (
    id BIGINT NOT NULL,
    uuid TEXT NOT NULL,
    tenant_id INTEGER NOT NULL,
    space_id INTEGER NOT NULL,
    from_concept_id TEXT NOT NULL,
    to_concept_id TEXT NOT NULL,
    anchor_text TEXT NOT NULL DEFAULT '',
    status INTEGER NOT NULL,
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL,
    version INTEGER NOT NULL DEFAULT 0,
    PRIMARY KEY (id)
);

CREATE UNIQUE INDEX IF NOT EXISTS uk_kb_okf_concept_link_uuid
    ON kb_okf_concept_link (tenant_id, uuid);

CREATE UNIQUE INDEX IF NOT EXISTS uk_kb_okf_concept_link_edge
    ON kb_okf_concept_link (tenant_id, space_id, from_concept_id, to_concept_id, anchor_text);

CREATE INDEX IF NOT EXISTS idx_kb_okf_concept_link_space_from
    ON kb_okf_concept_link (tenant_id, space_id, from_concept_id);

CREATE INDEX IF NOT EXISTS idx_kb_okf_concept_link_space_to
    ON kb_okf_concept_link (tenant_id, space_id, to_concept_id);

CREATE TABLE IF NOT EXISTS kb_okf_candidate (
    id BIGINT NOT NULL,
    uuid TEXT NOT NULL,
    tenant_id INTEGER NOT NULL,
    space_id INTEGER NOT NULL,
    concept_id TEXT NOT NULL,
    candidate_type TEXT NOT NULL,
    state TEXT NOT NULL,
    markdown_object_ref_id INTEGER,
    reviewer_id INTEGER,
    review_note TEXT,
    status INTEGER NOT NULL,
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL,
    version INTEGER NOT NULL DEFAULT 0,
    PRIMARY KEY (id)
);

CREATE UNIQUE INDEX IF NOT EXISTS uk_kb_okf_candidate_uuid
    ON kb_okf_candidate (tenant_id, uuid);

CREATE INDEX IF NOT EXISTS idx_kb_okf_candidate_space_state
    ON kb_okf_candidate (tenant_id, space_id, state, updated_at);

-- source: crates/sdkwork-intelligence-knowledgebase-repository-sqlx/migrations/sqlite/V202606220002__knowledgebase_chunk_fts.sql (virtual table only; baseline starts empty)
CREATE VIRTUAL TABLE IF NOT EXISTS kb_chunk_fts USING fts5(
    content_text,
    chunk_id UNINDEXED,
    tenant_id UNINDEXED,
    space_id UNINDEXED,
    document_id UNINDEXED,
    tokenize = 'unicode61'
);

-- source: crates/sdkwork-intelligence-knowledgebase-repository-sqlx/migrations/sqlite/V202606230001__knowledgebase_performance_indexes.sql
-- Performance indexes for ingestion job polling and outbox stale-claim release.

CREATE INDEX IF NOT EXISTS idx_kb_ingestion_job_tenant_state_status
    ON kb_ingestion_job (tenant_id, state, status);

CREATE INDEX IF NOT EXISTS idx_kb_outbox_stale_claim
    ON kb_outbox_event (tenant_id, status, claimed_at);

-- source: crates/sdkwork-intelligence-knowledgebase-repository-sqlx/migrations/sqlite/V202606240001__knowledge_market.sql
CREATE TABLE IF NOT EXISTS kb_market_listing (
    id BIGINT NOT NULL,
    tenant_id INTEGER NOT NULL,
    space_id BIGINT NOT NULL,
    title TEXT NOT NULL,
    icon TEXT,
    description TEXT,
    author TEXT,
    tags_json TEXT NOT NULL DEFAULT '[]',
    provider TEXT,
    model_name TEXT,
    subscribers_count INTEGER NOT NULL DEFAULT 0,
    documents_count INTEGER NOT NULL DEFAULT 0,
    status INTEGER NOT NULL DEFAULT 1,
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL,
    version INTEGER NOT NULL DEFAULT 0,
    PRIMARY KEY (id),
    FOREIGN KEY (space_id) REFERENCES kb_space(id)
);

CREATE UNIQUE INDEX IF NOT EXISTS uk_kb_market_listing_space
    ON kb_market_listing (tenant_id, space_id)
    WHERE status = 1;

CREATE INDEX IF NOT EXISTS idx_kb_market_listing_status
    ON kb_market_listing (tenant_id, status, updated_at);

CREATE TABLE IF NOT EXISTS kb_market_subscription (
    id BIGINT NOT NULL,
    tenant_id INTEGER NOT NULL,
    subscriber_actor_id BIGINT NOT NULL,
    listing_id BIGINT NOT NULL,
    created_at TEXT NOT NULL,
    status INTEGER NOT NULL DEFAULT 1,
    PRIMARY KEY (id),
    FOREIGN KEY (listing_id) REFERENCES kb_market_listing(id)
);

CREATE UNIQUE INDEX IF NOT EXISTS uk_kb_market_subscription_actor_listing
    ON kb_market_subscription (tenant_id, subscriber_actor_id, listing_id)
    WHERE status = 1;

-- source: crates/sdkwork-intelligence-knowledgebase-repository-sqlx/migrations/sqlite/V202606250001__knowledgebase_audit_event.sql
-- Durable append-oriented audit events for security-relevant knowledge mutations.

CREATE TABLE IF NOT EXISTS kb_audit_event (
    id INTEGER PRIMARY KEY,
    uuid TEXT NOT NULL,
    tenant_id INTEGER NOT NULL,
    event_type TEXT NOT NULL,
    actor_type TEXT NOT NULL,
    actor_id TEXT NOT NULL,
    resource_type TEXT NOT NULL,
    resource_id INTEGER,
    result TEXT NOT NULL,
    request_id TEXT,
    trace_id TEXT,
    ip_hash TEXT,
    user_agent_hash TEXT,
    payload TEXT,
    created_at TEXT NOT NULL,
    version INTEGER NOT NULL DEFAULT 0
);

CREATE UNIQUE INDEX IF NOT EXISTS uk_kb_audit_event_uuid
    ON kb_audit_event (tenant_id, uuid);

CREATE INDEX IF NOT EXISTS idx_kb_audit_event_tenant_created
    ON kb_audit_event (tenant_id, created_at DESC);

CREATE INDEX IF NOT EXISTS idx_kb_audit_event_resource
    ON kb_audit_event (tenant_id, resource_type, resource_id);

CREATE INDEX IF NOT EXISTS idx_kb_audit_event_event_type
    ON kb_audit_event (tenant_id, event_type, created_at DESC);

-- source: sdkwork-web-framework/crates/sdkwork-web-store-sqlx/migrations/003_web_audit_event.sql, 009_web_audit_outcome.sql, 013_web_event_expires_at.sql (folded)
CREATE TABLE IF NOT EXISTS web_audit_event (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    request_id TEXT NOT NULL,
    tenant_id TEXT,
    user_id TEXT,
    api_surface TEXT NOT NULL,
    path TEXT NOT NULL,
    method TEXT NOT NULL,
    operation_id TEXT,
    status_code INTEGER,
    duration_ms INTEGER,
    expires_at INTEGER,
    created_at INTEGER NOT NULL
);

CREATE INDEX IF NOT EXISTS idx_web_audit_event_created
    ON web_audit_event (created_at);

CREATE INDEX IF NOT EXISTS idx_web_audit_event_request
    ON web_audit_event (request_id);

CREATE INDEX IF NOT EXISTS idx_web_audit_event_tenant
    ON web_audit_event (tenant_id);

CREATE INDEX IF NOT EXISTS idx_web_audit_expires
    ON web_audit_event (expires_at);

-- Canonical live Wiki publication authority (ADR-20260721).
-- Shared scoped parent key for Wiki publication and Provider Binding foreign keys.
CREATE UNIQUE INDEX IF NOT EXISTS uk_kb_space_provider_scope
    ON kb_space (tenant_id, organization_id, id);

CREATE TABLE IF NOT EXISTS kb_site_publication (
    id BIGINT NOT NULL PRIMARY KEY,
    uuid TEXT NOT NULL,
    tenant_id BIGINT NOT NULL,
    organization_id BIGINT NOT NULL,
    space_id BIGINT NOT NULL,
    drive_space_uuid TEXT NOT NULL,
    source_root_node_uuid TEXT,
    source_scope_uuid TEXT,
    publication_type TEXT NOT NULL DEFAULT 'wiki',
    wiki_status TEXT NOT NULL DEFAULT 'DRAFT',
    title TEXT NOT NULL,
    description TEXT,
    homepage_source_path TEXT NOT NULL DEFAULT 'index.md',
    default_locale TEXT NOT NULL DEFAULT 'zh-CN',
    supported_locales_json TEXT NOT NULL DEFAULT '["zh-CN"]',
    publication_mode TEXT NOT NULL DEFAULT 'REVIEW_REQUIRED',
    default_visibility TEXT NOT NULL DEFAULT 'PRIVATE',
    update_policy TEXT NOT NULL DEFAULT 'KEEP_LAST_PUBLIC_UNTIL_READY',
    navigation_mode TEXT NOT NULL DEFAULT 'DIRECTORY',
    navigation_config_json TEXT NOT NULL DEFAULT '{}',
    theme_key TEXT NOT NULL DEFAULT 'sdkwork-wiki-default',
    theme_version TEXT NOT NULL DEFAULT '1',
    theme_config_json TEXT NOT NULL DEFAULT '{}',
    renderer_policy_version TEXT NOT NULL DEFAULT '1',
    search_enabled INTEGER NOT NULL DEFAULT 1,
    robots_policy TEXT NOT NULL DEFAULT 'NOINDEX_NOFOLLOW',
    sitemap_enabled INTEGER NOT NULL DEFAULT 0,
    provider_generation BIGINT NOT NULL DEFAULT 1,
    navigation_generation BIGINT NOT NULL DEFAULT 1,
    search_generation BIGINT NOT NULL DEFAULT 1,
    last_projected_drive_checkpoint BIGINT NOT NULL DEFAULT 0,
    activated_at TEXT,
    paused_at TEXT,
    last_error_code TEXT,
    created_by BIGINT NOT NULL,
    updated_by BIGINT NOT NULL,
    status INTEGER NOT NULL DEFAULT 1,
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL,
    version BIGINT NOT NULL DEFAULT 0,
    CHECK (tenant_id > 0),
    CHECK (organization_id >= 0),
    CHECK (publication_type = 'wiki'),
    CHECK (wiki_status IN (
        'DRAFT', 'VALIDATING', 'READY', 'ACTIVE', 'DEGRADED', 'PAUSED', 'ARCHIVED', 'FAILED'
    )),
    CHECK (publication_mode IN ('REVIEW_REQUIRED', 'AUTO_PUBLIC_AFTER_CHECKS')),
    CHECK (default_visibility IN ('PRIVATE', 'UNLISTED', 'PUBLIC')),
    CHECK (update_policy IN ('KEEP_LAST_PUBLIC_UNTIL_READY', 'UNPUBLISH_DURING_PROCESSING')),
    CHECK (navigation_mode IN ('DIRECTORY', 'FRONT_MATTER', 'CURATED')),
    CHECK (robots_policy IN ('INDEX_FOLLOW', 'NOINDEX_NOFOLLOW')),
    CHECK (search_enabled IN (0, 1)),
    CHECK (sitemap_enabled IN (0, 1)),
    CHECK (
        provider_generation >= 1 AND navigation_generation >= 1
        AND search_generation >= 1 AND last_projected_drive_checkpoint >= 0
    ),
    CHECK (
        wiki_status IN ('DRAFT', 'VALIDATING', 'ARCHIVED', 'FAILED')
        OR (source_root_node_uuid IS NOT NULL AND source_scope_uuid IS NOT NULL)
    ),
    CHECK (
        length(supported_locales_json) <= 8192
        AND length(navigation_config_json) <= 32768
        AND length(theme_config_json) <= 32768
    ),
    FOREIGN KEY (tenant_id, organization_id, space_id)
        REFERENCES kb_space(tenant_id, organization_id, id)
);

CREATE UNIQUE INDEX IF NOT EXISTS uk_kb_site_publication_uuid
    ON kb_site_publication (tenant_id, uuid);
CREATE UNIQUE INDEX IF NOT EXISTS uk_kb_site_publication_scope_id
    ON kb_site_publication (tenant_id, organization_id, id);
CREATE UNIQUE INDEX IF NOT EXISTS uk_kb_site_publication_space
    ON kb_site_publication (tenant_id, space_id);
CREATE UNIQUE INDEX IF NOT EXISTS uk_kb_site_publication_drive_space
    ON kb_site_publication (tenant_id, drive_space_uuid)
    WHERE status = 1;
CREATE INDEX IF NOT EXISTS idx_kb_site_publication_state
    ON kb_site_publication (
        tenant_id, organization_id, wiki_status, updated_at DESC, id DESC
    );

CREATE TABLE IF NOT EXISTS kb_source_file_projection (
    id BIGINT NOT NULL PRIMARY KEY,
    uuid TEXT NOT NULL,
    tenant_id BIGINT NOT NULL,
    organization_id BIGINT NOT NULL,
    site_publication_id BIGINT NOT NULL,
    space_id BIGINT NOT NULL,
    drive_space_uuid TEXT NOT NULL,
    drive_node_uuid TEXT NOT NULL,
    drive_version_uuid TEXT NOT NULL,
    source_path TEXT NOT NULL,
    canonical_route TEXT,
    file_kind TEXT NOT NULL,
    media_type TEXT NOT NULL,
    size_bytes BIGINT NOT NULL,
    content_sha256 TEXT NOT NULL,
    source_state TEXT NOT NULL DEFAULT 'DISCOVERED',
    publication_state TEXT NOT NULL DEFAULT 'DRAFT',
    visibility TEXT NOT NULL DEFAULT 'PRIVATE',
    index_state TEXT NOT NULL DEFAULT 'NOT_REQUIRED',
    title TEXT,
    description TEXT,
    locale TEXT,
    nav_order INTEGER,
    nav_hidden INTEGER NOT NULL DEFAULT 0,
    scheduled_publish_at TEXT,
    published_at TEXT,
    unpublished_at TEXT,
    public_drive_version_uuid TEXT,
    page_public_version BIGINT NOT NULL DEFAULT 0,
    parser_version TEXT,
    renderer_policy_version TEXT,
    index_version TEXT,
    previous_canonical_route TEXT,
    redirect_status INTEGER,
    redirect_expires_at TEXT,
    source_sequence_no BIGINT NOT NULL DEFAULT 0,
    last_source_event_id TEXT,
    processing_attempt_count INTEGER NOT NULL DEFAULT 0,
    next_processing_at TEXT,
    processing_lease_owner TEXT,
    processing_lease_token TEXT,
    processing_lease_expires_at TEXT,
    processing_fence BIGINT NOT NULL DEFAULT 0,
    last_error_code TEXT,
    last_error_summary TEXT,
    last_processed_at TEXT,
    last_indexed_at TEXT,
    created_by BIGINT NOT NULL,
    updated_by BIGINT NOT NULL,
    status INTEGER NOT NULL DEFAULT 1,
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL,
    version BIGINT NOT NULL DEFAULT 0,
    CHECK (tenant_id > 0),
    CHECK (organization_id >= 0),
    CHECK (file_kind IN (
        'PAGE', 'DOCUMENT', 'PRESENTATION', 'SPREADSHEET', 'CODE', 'MEDIA', 'ASSET', 'ARCHIVE'
    )),
    CHECK (source_state IN (
        'DISCOVERED', 'QUEUED', 'PROCESSING', 'READY', 'ERROR', 'QUARANTINED', 'DELETED'
    )),
    CHECK (publication_state IN (
        'DRAFT', 'IN_REVIEW', 'SCHEDULED', 'PUBLISHED', 'UNPUBLISHED', 'ARCHIVED'
    )),
    CHECK (visibility IN ('PRIVATE', 'UNLISTED', 'PUBLIC')),
    CHECK (index_state IN ('NOT_REQUIRED', 'PENDING', 'INDEXING', 'READY', 'ERROR')),
    CHECK (
        length(content_sha256) = 71 AND substr(content_sha256, 1, 7) = 'sha256:'
        AND substr(content_sha256, 8) NOT GLOB '*[^0-9a-f]*'
    ),
    CHECK (
        size_bytes >= 0 AND page_public_version >= 0 AND source_sequence_no >= 0
        AND processing_attempt_count >= 0 AND processing_fence >= 0
    ),
    CHECK (nav_hidden IN (0, 1)),
    CHECK (redirect_status IS NULL OR redirect_status IN (301, 302, 307, 308)),
    CHECK (
        publication_state <> 'PUBLISHED'
        OR (visibility IN ('UNLISTED', 'PUBLIC')
            AND canonical_route IS NOT NULL AND public_drive_version_uuid IS NOT NULL
            AND page_public_version > 0)
    ),
    FOREIGN KEY (tenant_id, organization_id, site_publication_id)
        REFERENCES kb_site_publication(tenant_id, organization_id, id),
    FOREIGN KEY (tenant_id, organization_id, space_id)
        REFERENCES kb_space(tenant_id, organization_id, id)
);

CREATE UNIQUE INDEX IF NOT EXISTS uk_kb_source_projection_uuid
    ON kb_source_file_projection (tenant_id, uuid);
CREATE UNIQUE INDEX IF NOT EXISTS uk_kb_source_projection_scope_id
    ON kb_source_file_projection (tenant_id, organization_id, id);
CREATE UNIQUE INDEX IF NOT EXISTS uk_kb_source_projection_node
    ON kb_source_file_projection (
        tenant_id, organization_id, site_publication_id, drive_node_uuid
    ) WHERE status = 1;
CREATE UNIQUE INDEX IF NOT EXISTS uk_kb_source_projection_path
    ON kb_source_file_projection (
        tenant_id, organization_id, site_publication_id, source_path
    ) WHERE status = 1 AND source_state <> 'DELETED';
CREATE UNIQUE INDEX IF NOT EXISTS uk_kb_source_projection_public_route
    ON kb_source_file_projection (
        tenant_id, organization_id, site_publication_id, canonical_route
    ) WHERE status = 1 AND publication_state = 'PUBLISHED'
      AND visibility IN ('UNLISTED', 'PUBLIC');
CREATE INDEX IF NOT EXISTS idx_kb_source_projection_state
    ON kb_source_file_projection (
        tenant_id, organization_id, site_publication_id,
        source_state, publication_state, updated_at DESC, id DESC
    );
CREATE INDEX IF NOT EXISTS idx_kb_source_projection_claimable
    ON kb_source_file_projection (
        tenant_id, organization_id, source_state, next_processing_at, updated_at, id
    ) WHERE status = 1 AND source_state IN ('DISCOVERED', 'QUEUED', 'ERROR');
CREATE INDEX IF NOT EXISTS idx_kb_source_projection_scheduled
    ON kb_source_file_projection (
        tenant_id, organization_id, scheduled_publish_at, id
    ) WHERE status = 1 AND publication_state = 'SCHEDULED';
CREATE INDEX IF NOT EXISTS idx_kb_source_projection_public_lookup
    ON kb_source_file_projection (
        tenant_id, organization_id, site_publication_id, canonical_route,
        page_public_version, id
    ) WHERE status = 1 AND publication_state = 'PUBLISHED'
      AND visibility IN ('UNLISTED', 'PUBLIC');

CREATE TABLE IF NOT EXISTS kb_source_file_rendition (
    id BIGINT NOT NULL PRIMARY KEY,
    uuid TEXT NOT NULL,
    tenant_id BIGINT NOT NULL,
    organization_id BIGINT NOT NULL,
    site_publication_id BIGINT NOT NULL,
    source_file_projection_id BIGINT NOT NULL,
    drive_version_uuid TEXT NOT NULL,
    source_content_sha256 TEXT NOT NULL,
    processor_id TEXT NOT NULL,
    processor_version TEXT NOT NULL,
    policy_version TEXT NOT NULL,
    rendition_kind TEXT NOT NULL,
    rendition_key_sha256 TEXT NOT NULL,
    rendition_state TEXT NOT NULL DEFAULT 'PENDING',
    locale TEXT,
    rendition_drive_space_uuid TEXT,
    rendition_drive_node_uuid TEXT,
    rendition_drive_version_uuid TEXT,
    media_resource_snapshot TEXT,
    content_sha256 TEXT,
    media_type TEXT,
    size_bytes BIGINT,
    page_or_slide_count INTEGER,
    width INTEGER,
    height INTEGER,
    duration_millis BIGINT,
    processing_attempt_count INTEGER NOT NULL DEFAULT 0,
    next_processing_at TEXT,
    processing_lease_owner TEXT,
    processing_lease_token TEXT,
    processing_lease_expires_at TEXT,
    processing_fence BIGINT NOT NULL DEFAULT 0,
    error_code TEXT,
    error_summary TEXT,
    processed_at TEXT,
    expires_at TEXT,
    created_by BIGINT NOT NULL,
    updated_by BIGINT NOT NULL,
    status INTEGER NOT NULL DEFAULT 1,
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL,
    version BIGINT NOT NULL DEFAULT 0,
    CHECK (tenant_id > 0),
    CHECK (organization_id >= 0),
    CHECK (rendition_kind IN (
        'SANITIZED_HTML', 'PDF', 'PAGE_IMAGE', 'THUMBNAIL', 'POSTER', 'PLAIN_TEXT',
        'SLIDE_TEXT', 'SHEET_PREVIEW', 'ARCHIVE_MANIFEST', 'MEDIA_METADATA'
    )),
    CHECK (rendition_state IN ('PENDING', 'PROCESSING', 'READY', 'ERROR', 'QUARANTINED', 'EXPIRED')),
    CHECK (
        length(source_content_sha256) = 71
        AND substr(source_content_sha256, 1, 7) = 'sha256:'
        AND substr(source_content_sha256, 8) NOT GLOB '*[^0-9a-f]*'
        AND length(rendition_key_sha256) = 71
        AND substr(rendition_key_sha256, 1, 7) = 'sha256:'
        AND substr(rendition_key_sha256, 8) NOT GLOB '*[^0-9a-f]*'
        AND (content_sha256 IS NULL OR (
            length(content_sha256) = 71 AND substr(content_sha256, 1, 7) = 'sha256:'
            AND substr(content_sha256, 8) NOT GLOB '*[^0-9a-f]*'
        ))
    ),
    CHECK (
        processing_attempt_count >= 0 AND processing_fence >= 0
        AND (size_bytes IS NULL OR size_bytes >= 0)
        AND (page_or_slide_count IS NULL OR page_or_slide_count >= 0)
        AND (width IS NULL OR width >= 0) AND (height IS NULL OR height >= 0)
        AND (duration_millis IS NULL OR duration_millis >= 0)
    ),
    CHECK (
        rendition_state <> 'READY'
        OR (rendition_drive_space_uuid IS NOT NULL AND rendition_drive_node_uuid IS NOT NULL
            AND rendition_drive_version_uuid IS NOT NULL AND content_sha256 IS NOT NULL
            AND media_type IS NOT NULL AND size_bytes IS NOT NULL AND processed_at IS NOT NULL)
    ),
    CHECK (media_resource_snapshot IS NULL OR length(media_resource_snapshot) <= 32768),
    FOREIGN KEY (tenant_id, organization_id, site_publication_id)
        REFERENCES kb_site_publication(tenant_id, organization_id, id),
    FOREIGN KEY (tenant_id, organization_id, source_file_projection_id)
        REFERENCES kb_source_file_projection(tenant_id, organization_id, id)
);

CREATE UNIQUE INDEX IF NOT EXISTS uk_kb_source_rendition_uuid
    ON kb_source_file_rendition (tenant_id, uuid);
CREATE UNIQUE INDEX IF NOT EXISTS uk_kb_source_rendition_identity
    ON kb_source_file_rendition (
        tenant_id, organization_id, source_file_projection_id, rendition_key_sha256
    ) WHERE status = 1;
CREATE INDEX IF NOT EXISTS idx_kb_source_rendition_claimable
    ON kb_source_file_rendition (
        tenant_id, organization_id, rendition_state, next_processing_at, updated_at, id
    ) WHERE status = 1 AND rendition_state IN ('PENDING', 'ERROR');
CREATE INDEX IF NOT EXISTS idx_kb_source_rendition_source_version
    ON kb_source_file_rendition (
        tenant_id, organization_id, source_file_projection_id,
        drive_version_uuid, rendition_kind, updated_at DESC, id DESC
    );

CREATE TABLE IF NOT EXISTS kb_drive_source_checkpoint (
    id BIGINT NOT NULL PRIMARY KEY,
    uuid TEXT NOT NULL,
    tenant_id BIGINT NOT NULL,
    organization_id BIGINT NOT NULL,
    site_publication_id BIGINT NOT NULL,
    drive_space_uuid TEXT NOT NULL,
    source_scope_uuid TEXT NOT NULL,
    last_sequence_no BIGINT NOT NULL DEFAULT 0,
    last_event_id TEXT,
    stream_state TEXT NOT NULL DEFAULT 'HEALTHY',
    gap_from_sequence_no BIGINT,
    gap_to_sequence_no BIGINT,
    reconciliation_cursor TEXT,
    reconciliation_started_at TEXT,
    reconciliation_completed_at TEXT,
    lease_owner TEXT,
    lease_token TEXT,
    lease_expires_at TEXT,
    fence_token BIGINT NOT NULL DEFAULT 0,
    last_event_time TEXT,
    last_observed_at TEXT,
    last_error_code TEXT,
    last_error_summary TEXT,
    created_by BIGINT NOT NULL,
    updated_by BIGINT NOT NULL,
    status INTEGER NOT NULL DEFAULT 1,
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL,
    version BIGINT NOT NULL DEFAULT 0,
    CHECK (tenant_id > 0),
    CHECK (organization_id >= 0),
    CHECK (stream_state IN ('HEALTHY', 'GAP_DETECTED', 'RECONCILING', 'PAUSED', 'FAILED')),
    CHECK (
        last_sequence_no >= 0 AND fence_token >= 0
        AND (gap_from_sequence_no IS NULL OR gap_from_sequence_no > last_sequence_no)
        AND (gap_to_sequence_no IS NULL OR gap_to_sequence_no >= gap_from_sequence_no)
    ),
    FOREIGN KEY (tenant_id, organization_id, site_publication_id)
        REFERENCES kb_site_publication(tenant_id, organization_id, id)
);

CREATE UNIQUE INDEX IF NOT EXISTS uk_kb_drive_checkpoint_uuid
    ON kb_drive_source_checkpoint (tenant_id, uuid);
CREATE UNIQUE INDEX IF NOT EXISTS uk_kb_drive_checkpoint_scope_id
    ON kb_drive_source_checkpoint (tenant_id, organization_id, id);
CREATE UNIQUE INDEX IF NOT EXISTS uk_kb_drive_checkpoint_publication
    ON kb_drive_source_checkpoint (tenant_id, organization_id, site_publication_id);
CREATE UNIQUE INDEX IF NOT EXISTS uk_kb_drive_checkpoint_source_scope
    ON kb_drive_source_checkpoint (
        tenant_id, organization_id, drive_space_uuid, source_scope_uuid
    );
CREATE INDEX IF NOT EXISTS idx_kb_drive_checkpoint_reconcile
    ON kb_drive_source_checkpoint (
        tenant_id, organization_id, stream_state, lease_expires_at, updated_at, id
    ) WHERE status = 1 AND stream_state IN ('GAP_DETECTED', 'RECONCILING', 'FAILED');

CREATE TABLE IF NOT EXISTS kb_drive_event_inbox (
    id BIGINT NOT NULL PRIMARY KEY,
    uuid TEXT NOT NULL,
    tenant_id BIGINT NOT NULL,
    organization_id BIGINT NOT NULL,
    site_publication_id BIGINT NOT NULL,
    checkpoint_id BIGINT NOT NULL,
    source_event_id TEXT NOT NULL,
    event_type TEXT NOT NULL,
    sequence_no BIGINT NOT NULL,
    drive_node_uuid TEXT NOT NULL,
    drive_version_uuid TEXT,
    payload_sha256 TEXT NOT NULL,
    payload_json TEXT NOT NULL,
    source_event_time TEXT NOT NULL,
    processing_state TEXT NOT NULL DEFAULT 'RECEIVED',
    attempt_count INTEGER NOT NULL DEFAULT 0,
    next_retry_at TEXT,
    lease_owner TEXT,
    lease_token TEXT,
    lease_expires_at TEXT,
    last_error_code TEXT,
    last_error_summary TEXT,
    received_at TEXT NOT NULL,
    applied_at TEXT,
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL,
    version BIGINT NOT NULL DEFAULT 0,
    CHECK (tenant_id > 0),
    CHECK (organization_id >= 0),
    CHECK (event_type IN (
        'drive.node.version.committed.v1', 'drive.node.path.changed.v1',
        'drive.node.eligibility.changed.v1', 'drive.node.deleted.v1'
    )),
    CHECK (processing_state IN ('RECEIVED', 'DEFERRED', 'APPLIED', 'RETRY', 'DEAD_LETTER', 'IGNORED')),
    CHECK (sequence_no >= 1 AND attempt_count >= 0 AND length(payload_json) <= 65536),
    CHECK (
        length(payload_sha256) = 71 AND substr(payload_sha256, 1, 7) = 'sha256:'
        AND substr(payload_sha256, 8) NOT GLOB '*[^0-9a-f]*'
    ),
    FOREIGN KEY (tenant_id, organization_id, site_publication_id)
        REFERENCES kb_site_publication(tenant_id, organization_id, id),
    FOREIGN KEY (tenant_id, organization_id, checkpoint_id)
        REFERENCES kb_drive_source_checkpoint(tenant_id, organization_id, id)
);

CREATE UNIQUE INDEX IF NOT EXISTS uk_kb_drive_inbox_uuid
    ON kb_drive_event_inbox (tenant_id, uuid);
CREATE UNIQUE INDEX IF NOT EXISTS uk_kb_drive_inbox_event
    ON kb_drive_event_inbox (
        tenant_id, organization_id, site_publication_id, source_event_id
    );
CREATE UNIQUE INDEX IF NOT EXISTS uk_kb_drive_inbox_sequence
    ON kb_drive_event_inbox (
        tenant_id, organization_id, checkpoint_id, sequence_no
    );
CREATE INDEX IF NOT EXISTS idx_kb_drive_inbox_apply
    ON kb_drive_event_inbox (
        tenant_id, organization_id, checkpoint_id, processing_state,
        sequence_no, id
    );
CREATE INDEX IF NOT EXISTS idx_kb_drive_inbox_retry
    ON kb_drive_event_inbox (
        tenant_id, organization_id, processing_state, next_retry_at, sequence_no, id
    ) WHERE processing_state IN ('RECEIVED', 'RETRY', 'DEFERRED');

-- Provider Binding SPI v2 authority (ADR-20260720).
CREATE TABLE IF NOT EXISTS kb_provider_credential_reference (
    id BIGINT NOT NULL,
    uuid TEXT NOT NULL,
    tenant_id BIGINT NOT NULL,
    organization_id BIGINT NOT NULL,
    implementation_id TEXT NOT NULL,
    display_name TEXT NOT NULL,
    reference_locator TEXT NOT NULL,
    reference_fingerprint TEXT NOT NULL,
    rotation_state TEXT NOT NULL DEFAULT 'current',
    last_rotated_at TEXT,
    created_by TEXT NOT NULL,
    updated_by TEXT NOT NULL,
    status INTEGER NOT NULL DEFAULT 1,
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL,
    version BIGINT NOT NULL DEFAULT 0,
    PRIMARY KEY (id),
    CHECK (tenant_id > 0),
    CHECK (organization_id >= 0),
    CHECK (length(trim(implementation_id)) > 0),
    CHECK (length(trim(reference_locator)) > 0),
    CHECK (rotation_state IN ('current', 'rotation_due', 'revoked'))
);

CREATE UNIQUE INDEX IF NOT EXISTS uk_kb_provider_credential_reference_uuid
    ON kb_provider_credential_reference (tenant_id, organization_id, uuid);
CREATE UNIQUE INDEX IF NOT EXISTS uk_kb_provider_credential_reference_scope_id
    ON kb_provider_credential_reference (tenant_id, organization_id, id);
CREATE UNIQUE INDEX IF NOT EXISTS uk_kb_provider_credential_reference_active
    ON kb_provider_credential_reference (
        tenant_id, organization_id, implementation_id, reference_fingerprint
    )
    WHERE status = 1;

CREATE TABLE IF NOT EXISTS kb_provider_binding (
    id BIGINT NOT NULL,
    uuid TEXT NOT NULL,
    tenant_id BIGINT NOT NULL,
    organization_id BIGINT NOT NULL,
    space_id BIGINT NOT NULL,
    implementation_id TEXT NOT NULL,
    remote_resource_type TEXT NOT NULL,
    remote_resource_id TEXT NOT NULL,
    credential_reference_id BIGINT,
    lifecycle_state TEXT NOT NULL DEFAULT 'draft',
    capability_snapshot TEXT NOT NULL DEFAULT '[]',
    capability_snapshot_version BIGINT NOT NULL DEFAULT 0,
    last_tested_at TEXT,
    activated_at TEXT,
    disabled_at TEXT,
    last_error_category TEXT,
    created_by TEXT NOT NULL,
    updated_by TEXT NOT NULL,
    status INTEGER NOT NULL DEFAULT 1,
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL,
    version BIGINT NOT NULL DEFAULT 0,
    PRIMARY KEY (id),
    CHECK (tenant_id > 0),
    CHECK (organization_id >= 0),
    CHECK (length(trim(implementation_id)) > 0),
    CHECK (length(trim(remote_resource_type)) > 0),
    CHECK (length(trim(remote_resource_id)) > 0),
    CHECK (lifecycle_state IN ('draft', 'testing', 'active', 'degraded', 'disabled', 'failed')),
    FOREIGN KEY (tenant_id, organization_id, space_id)
        REFERENCES kb_space(tenant_id, organization_id, id),
    FOREIGN KEY (tenant_id, organization_id, credential_reference_id)
        REFERENCES kb_provider_credential_reference(tenant_id, organization_id, id)
);

CREATE UNIQUE INDEX IF NOT EXISTS uk_kb_provider_binding_uuid
    ON kb_provider_binding (tenant_id, organization_id, uuid);
CREATE UNIQUE INDEX IF NOT EXISTS uk_kb_provider_binding_scope_id
    ON kb_provider_binding (tenant_id, organization_id, id);
CREATE UNIQUE INDEX IF NOT EXISTS uk_kb_provider_binding_remote_resource
    ON kb_provider_binding (
        tenant_id, organization_id, space_id, implementation_id,
        remote_resource_type, remote_resource_id
    )
    WHERE status = 1;
CREATE UNIQUE INDEX IF NOT EXISTS uk_kb_provider_binding_active_space
    ON kb_provider_binding (tenant_id, organization_id, space_id)
    WHERE lifecycle_state = 'active' AND status = 1;
CREATE INDEX IF NOT EXISTS idx_kb_provider_binding_space_lifecycle
    ON kb_provider_binding (
        tenant_id, organization_id, space_id, lifecycle_state, updated_at DESC, id DESC
    );

CREATE TABLE IF NOT EXISTS kb_provider_migration_operation (
    id BIGINT NOT NULL,
    uuid TEXT NOT NULL,
    tenant_id BIGINT NOT NULL,
    organization_id BIGINT NOT NULL,
    space_id BIGINT NOT NULL,
    source_binding_id BIGINT NOT NULL,
    target_binding_id BIGINT NOT NULL,
    operation_state TEXT NOT NULL DEFAULT 'dry_run',
    idempotency_key TEXT NOT NULL,
    requested_by TEXT NOT NULL,
    checkpoint TEXT NOT NULL DEFAULT '{}',
    claim_owner TEXT,
    claim_token TEXT,
    lease_expires_at TEXT,
    attempt_count INTEGER NOT NULL DEFAULT 0,
    cutover_at TEXT,
    observation_until TEXT,
    completed_at TEXT,
    last_error_category TEXT,
    status INTEGER NOT NULL DEFAULT 1,
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL,
    version BIGINT NOT NULL DEFAULT 0,
    PRIMARY KEY (id),
    CHECK (tenant_id > 0),
    CHECK (organization_id >= 0),
    CHECK (source_binding_id <> target_binding_id),
    CHECK (operation_state IN (
        'dry_run', 'preparing', 'validating', 'cutover', 'observing',
        'completed', 'rolling_back', 'rolled_back', 'failed'
    )),
    FOREIGN KEY (tenant_id, organization_id, space_id)
        REFERENCES kb_space(tenant_id, organization_id, id),
    FOREIGN KEY (tenant_id, organization_id, source_binding_id)
        REFERENCES kb_provider_binding(tenant_id, organization_id, id),
    FOREIGN KEY (tenant_id, organization_id, target_binding_id)
        REFERENCES kb_provider_binding(tenant_id, organization_id, id)
);

CREATE UNIQUE INDEX IF NOT EXISTS uk_kb_provider_migration_operation_uuid
    ON kb_provider_migration_operation (tenant_id, organization_id, uuid);
CREATE UNIQUE INDEX IF NOT EXISTS uk_kb_provider_migration_operation_idempotency
    ON kb_provider_migration_operation (tenant_id, organization_id, idempotency_key);
CREATE UNIQUE INDEX IF NOT EXISTS uk_kb_provider_migration_operation_active_space
    ON kb_provider_migration_operation (tenant_id, organization_id, space_id)
    WHERE operation_state NOT IN ('completed', 'rolled_back', 'failed') AND status = 1;
CREATE INDEX IF NOT EXISTS idx_kb_provider_migration_operation_claimable
    ON kb_provider_migration_operation (
        tenant_id, organization_id, operation_state, lease_expires_at, updated_at, id
    );
