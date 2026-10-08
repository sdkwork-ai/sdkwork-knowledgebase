-- sdkwork:migration
-- id: 202607210001_live_wiki_publication
-- engine: sqlite
-- module: knowledgebase
-- purpose: Add canonical live Wiki publication, projection, rendition, checkpoint, and inbox state
-- reversible: true
-- transactional: true
-- lock: lightweight
-- contract_version: 1.1.0

-- Composite Wiki foreign keys require this shared parent key before child tables exist.
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
CREATE INDEX IF NOT EXISTS idx_kb_source_projection_previous_route
    ON kb_source_file_projection (
        tenant_id, organization_id, site_publication_id, previous_canonical_route
    ) WHERE status = 1 AND previous_canonical_route IS NOT NULL;

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
