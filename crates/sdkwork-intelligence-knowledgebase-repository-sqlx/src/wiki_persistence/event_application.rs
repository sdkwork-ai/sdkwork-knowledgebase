use sdkwork_intelligence_knowledgebase_service::ports::knowledge_wiki_persistence::{
    WikiDriveInboxEvent, WikiDriveProjectionMutation, WikiPagePublicationState,
    WikiPersistenceError, WikiPublicRouteChange, WikiSourceProjection, WikiSourceState,
    WikiUpdatePolicy,
};
use sdkwork_utils_rust::uuid;
use serde_json::json;
use sqlx::{Any, Row, Transaction};

use super::projection::{projection_from_row, PROJECTION_COLUMNS};
use super::{
    from_i64, parse_enum, require_id, require_sha256, require_text, row_error, sql_error, to_i64,
    SqlxWikiPersistenceStore,
};

const ROUTE_REVOKED_EVENT: &str = "knowledgebase.wiki.route.revoked.v1";
const NAVIGATION_CHANGED_EVENT: &str = "knowledgebase.wiki.navigation.changed.v1";
const SEARCH_CHANGED_EVENT: &str = "knowledgebase.wiki.search.changed.v1";

struct PublicationIdentity {
    id: u64,
    uuid: String,
    space_id: u64,
    drive_space_uuid: String,
    provider_generation: u64,
    default_visibility: String,
    update_policy: WikiUpdatePolicy,
}

pub(super) async fn apply_projection_mutation(
    store: &SqlxWikiPersistenceStore,
    transaction: &mut Transaction<'_, Any>,
    event: &WikiDriveInboxEvent,
    mutation: &WikiDriveProjectionMutation,
    actor_id: u64,
    now: &str,
) -> Result<(Option<WikiSourceProjection>, Option<WikiPublicRouteChange>), WikiPersistenceError> {
    if matches!(mutation, WikiDriveProjectionMutation::None) {
        return Ok((None, None));
    }
    let actor_id = require_id("actor_id", actor_id)?;
    let publication = load_publication(store, transaction, event).await?;
    let existing = load_projection(store, transaction, event).await?;

    if let Some(existing) = existing.as_ref() {
        if event.sequence_no < existing.source_sequence_no {
            return Ok((Some(existing.clone()), None));
        }
        if event.sequence_no == existing.source_sequence_no {
            if existing.last_source_event_id.as_deref() == Some(event.source_event_id.as_str()) {
                return Ok((Some(existing.clone()), None));
            }
            return Err(WikiPersistenceError::Conflict(format!(
                "Drive sequence {} conflicts with the existing Wiki projection",
                event.sequence_no
            )));
        }
    }

    let (projection, public_change) = match mutation {
        WikiDriveProjectionMutation::None => {
            return Err(WikiPersistenceError::Internal(
                "WikiDriveProjectionMutation::None reached the wiki projection write path"
                    .to_string(),
            ));
        }
        WikiDriveProjectionMutation::Upsert(metadata) => {
            validate_metadata(metadata)?;
            upsert_version(
                store,
                transaction,
                event,
                &publication,
                existing.as_ref(),
                metadata,
                actor_id,
                now,
            )
            .await?
        }
        WikiDriveProjectionMutation::MoveWithin { source_path } => {
            let source_path = validate_relative_path(source_path)?;
            let existing = existing.ok_or_else(|| {
                WikiPersistenceError::Conflict(
                    "a path-enter event requires source reconciliation before checkpoint advance"
                        .to_string(),
                )
            })?;
            move_projection(
                store,
                transaction,
                event,
                &publication,
                &existing,
                source_path,
                actor_id,
                now,
            )
            .await?
        }
        WikiDriveProjectionMutation::MarkEligible => {
            let existing = existing.ok_or_else(|| {
                WikiPersistenceError::Conflict(
                    "an eligibility event requires source reconciliation before checkpoint advance"
                        .to_string(),
                )
            })?;
            mark_eligible(store, transaction, event, &existing, actor_id, now).await?
        }
        WikiDriveProjectionMutation::Revoke {
            source_state,
            publication_state,
            reason_code,
        } => {
            validate_revocation(*source_state, *publication_state, reason_code)?;
            let Some(existing) = existing else {
                return Ok((None, None));
            };
            revoke_projection(
                store,
                transaction,
                event,
                &publication,
                &existing,
                *source_state,
                *publication_state,
                reason_code,
                actor_id,
                now,
            )
            .await?
        }
    };

    if let Some(change) = public_change.as_ref() {
        let generations = advance_public_collection_generations(
            store,
            transaction,
            event,
            publication.id,
            actor_id,
            now,
        )
        .await?;
        append_public_change_outbox(
            store,
            transaction,
            event,
            &publication,
            change,
            generations,
            now,
        )
        .await?;
    }
    Ok((Some(projection), public_change))
}

async fn load_publication(
    _store: &SqlxWikiPersistenceStore,
    transaction: &mut Transaction<'_, Any>,
    event: &WikiDriveInboxEvent,
) -> Result<PublicationIdentity, WikiPersistenceError> {
    let row = sqlx::query(
        r#"
        SELECT id, uuid, space_id, drive_space_uuid, provider_generation,
               default_visibility, update_policy
        FROM kb_site_publication
        WHERE tenant_id = $1 AND organization_id = $2 AND id = $3 AND status = 1
        "#,
    )
    .bind(to_i64("tenant_id", event.scope.tenant_id)?)
    .bind(to_i64("organization_id", event.scope.organization_id)?)
    .bind(to_i64("site_publication_id", event.site_publication_id)?)
    .fetch_optional(&mut **transaction)
    .await
    .map_err(sql_error)?
    .ok_or(WikiPersistenceError::NotFound {
        resource: "wiki_publication",
        id: event.site_publication_id,
    })?;
    Ok(PublicationIdentity {
        id: from_i64("publication_id", row.try_get("id").map_err(row_error)?)?,
        uuid: row.try_get("uuid").map_err(row_error)?,
        space_id: from_i64("space_id", row.try_get("space_id").map_err(row_error)?)?,
        drive_space_uuid: row.try_get("drive_space_uuid").map_err(row_error)?,
        provider_generation: from_i64(
            "provider_generation",
            row.try_get("provider_generation").map_err(row_error)?,
        )?,
        default_visibility: row.try_get("default_visibility").map_err(row_error)?,
        update_policy: parse_enum(
            "update_policy",
            row.try_get("update_policy").map_err(row_error)?,
        )?,
    })
}

async fn load_projection(
    _store: &SqlxWikiPersistenceStore,
    transaction: &mut Transaction<'_, Any>,
    event: &WikiDriveInboxEvent,
) -> Result<Option<WikiSourceProjection>, WikiPersistenceError> {
    let query = format!(
        "SELECT {PROJECTION_COLUMNS} FROM kb_source_file_projection WHERE tenant_id = $1 AND organization_id = $2 AND site_publication_id = $3 AND drive_node_uuid = $4 AND status = 1",
    );
    sqlx::query(sqlx::AssertSqlSafe(query.as_str()))
        .bind(to_i64("tenant_id", event.scope.tenant_id)?)
        .bind(to_i64("organization_id", event.scope.organization_id)?)
        .bind(to_i64("site_publication_id", event.site_publication_id)?)
        .bind(event.drive_node_uuid.as_str())
        .fetch_optional(&mut **transaction)
        .await
        .map_err(sql_error)?
        .map(|row| projection_from_row(&row))
        .transpose()
}

#[allow(clippy::too_many_arguments)]
async fn upsert_version(
    store: &SqlxWikiPersistenceStore,
    transaction: &mut Transaction<'_, Any>,
    event: &WikiDriveInboxEvent,
    publication: &PublicationIdentity,
    existing: Option<&WikiSourceProjection>,
    metadata: &sdkwork_intelligence_knowledgebase_service::ports::knowledge_wiki_persistence::WikiDriveSourceMetadata,
    actor_id: i64,
    now: &str,
) -> Result<(WikiSourceProjection, Option<WikiPublicRouteChange>), WikiPersistenceError> {
    if let Some(existing) = existing {
        let revoke_public = publication.update_policy
            == WikiUpdatePolicy::UnpublishDuringProcessing
            && existing.publication_state == WikiPagePublicationState::Published
            && existing.public_drive_version_uuid.is_some();
        let timestamp = store.dialect.sql_timestamp_expr("$14");
        let query = format!(
            r#"
            UPDATE kb_source_file_projection
            SET drive_version_uuid = $4, source_path = $5, file_kind = $6,
                media_type = $7, size_bytes = $8, content_sha256 = $9,
                source_state = 'DISCOVERED',
                publication_state = CASE WHEN $15 THEN 'UNPUBLISHED' ELSE publication_state END,
                public_drive_version_uuid = CASE WHEN $15 THEN NULL ELSE public_drive_version_uuid END,
                page_public_version = page_public_version + CASE WHEN $15 THEN 1 ELSE 0 END,
                index_state = CASE WHEN $6 = 'PAGE' THEN 'PENDING' ELSE 'NOT_REQUIRED' END,
                source_sequence_no = $10, last_source_event_id = $11,
                processing_attempt_count = 0, next_processing_at = NULL,
                processing_lease_owner = NULL, processing_lease_token = NULL,
                processing_lease_expires_at = NULL, processing_fence = processing_fence + 1,
                last_error_code = NULL, last_error_summary = NULL,
                updated_by = $12, updated_at = {timestamp}, version = version + 1
            WHERE tenant_id = $1 AND organization_id = $2 AND id = $3
              AND version = $13 AND status = 1
            RETURNING {PROJECTION_COLUMNS}
            "#,
        );
        let row = sqlx::query(sqlx::AssertSqlSafe(query.as_str()))
            .bind(to_i64("tenant_id", event.scope.tenant_id)?)
            .bind(to_i64("organization_id", event.scope.organization_id)?)
            .bind(to_i64("projection_id", existing.id)?)
            .bind(metadata.drive_version_uuid.trim())
            .bind(metadata.source_path.trim())
            .bind(metadata.file_kind.as_str())
            .bind(metadata.media_type.trim())
            .bind(to_i64("size_bytes", metadata.size_bytes)?)
            .bind(metadata.content_sha256.as_str())
            .bind(to_i64("source_sequence_no", event.sequence_no)?)
            .bind(event.source_event_id.as_str())
            .bind(actor_id)
            .bind(to_i64("expected_version", existing.version)?)
            .bind(now)
            .bind(revoke_public)
            .fetch_optional(&mut **transaction)
            .await
            .map_err(sql_error)?
            .ok_or(WikiPersistenceError::StaleVersion {
                resource: "wiki_source_projection",
                id: existing.id,
                expected: existing.version,
            })?;
        let projection = projection_from_row(&row)?;
        let change = revoke_public.then(|| {
            public_change(
                existing.canonical_route.clone(),
                &projection,
                publication.provider_generation,
                "source_update_processing",
            )
        });
        return Ok((projection, change));
    }

    let timestamp = store.dialect.sql_timestamp_expr("$20");
    let query = format!(
        r#"
        INSERT INTO kb_source_file_projection (
            id, uuid, tenant_id, organization_id, site_publication_id, space_id,
            drive_space_uuid, drive_node_uuid, drive_version_uuid, source_path,
            file_kind, media_type, size_bytes, content_sha256, visibility, index_state,
            source_sequence_no, last_source_event_id, created_by, updated_by,
            created_at, updated_at
        ) VALUES (
            $1, $2, $3, $4, $5, $6, $7, $8, $9, $10,
            $11, $12, $13, $14, $15, $16, $17, $18, $19, $19,
            {timestamp}, {timestamp}
        ) RETURNING {PROJECTION_COLUMNS}
        "#,
    );
    let row = sqlx::query(sqlx::AssertSqlSafe(query.as_str()))
        .bind(store.next_id()?)
        .bind(uuid())
        .bind(to_i64("tenant_id", event.scope.tenant_id)?)
        .bind(to_i64("organization_id", event.scope.organization_id)?)
        .bind(to_i64("site_publication_id", event.site_publication_id)?)
        .bind(to_i64("space_id", publication.space_id)?)
        .bind(publication.drive_space_uuid.as_str())
        .bind(event.drive_node_uuid.as_str())
        .bind(metadata.drive_version_uuid.trim())
        .bind(metadata.source_path.trim())
        .bind(metadata.file_kind.as_str())
        .bind(metadata.media_type.trim())
        .bind(to_i64("size_bytes", metadata.size_bytes)?)
        .bind(metadata.content_sha256.as_str())
        .bind(publication.default_visibility.as_str())
        .bind(if metadata.file_kind.as_str() == "PAGE" {
            "PENDING"
        } else {
            "NOT_REQUIRED"
        })
        .bind(to_i64("source_sequence_no", event.sequence_no)?)
        .bind(event.source_event_id.as_str())
        .bind(actor_id)
        .bind(now)
        .fetch_one(&mut **transaction)
        .await
        .map_err(sql_error)?;
    Ok((projection_from_row(&row)?, None))
}

#[allow(clippy::too_many_arguments)]
async fn move_projection(
    store: &SqlxWikiPersistenceStore,
    transaction: &mut Transaction<'_, Any>,
    event: &WikiDriveInboxEvent,
    publication: &PublicationIdentity,
    existing: &WikiSourceProjection,
    source_path: &str,
    actor_id: i64,
    now: &str,
) -> Result<(WikiSourceProjection, Option<WikiPublicRouteChange>), WikiPersistenceError> {
    let revoke_public = existing.publication_state == WikiPagePublicationState::Published
        && existing.public_drive_version_uuid.is_some();
    let timestamp = store.dialect.sql_timestamp_expr("$10");
    let query = format!(
        r#"
        UPDATE kb_source_file_projection
        SET source_path = $4, previous_canonical_route = canonical_route,
            canonical_route = NULL, source_state = 'DISCOVERED',
            publication_state = CASE WHEN $9 THEN 'UNPUBLISHED' ELSE publication_state END,
            public_drive_version_uuid = CASE WHEN $9 THEN NULL ELSE public_drive_version_uuid END,
            page_public_version = page_public_version + CASE WHEN $9 THEN 1 ELSE 0 END,
            source_sequence_no = $5, last_source_event_id = $6,
            processing_attempt_count = 0, next_processing_at = NULL,
            processing_lease_owner = NULL, processing_lease_token = NULL,
            processing_lease_expires_at = NULL, processing_fence = processing_fence + 1,
            updated_by = $7, updated_at = {timestamp}, version = version + 1
        WHERE tenant_id = $1 AND organization_id = $2 AND id = $3
          AND version = $8 AND status = 1
        RETURNING {PROJECTION_COLUMNS}
        "#,
    );
    let row = sqlx::query(sqlx::AssertSqlSafe(query.as_str()))
        .bind(to_i64("tenant_id", event.scope.tenant_id)?)
        .bind(to_i64("organization_id", event.scope.organization_id)?)
        .bind(to_i64("projection_id", existing.id)?)
        .bind(source_path)
        .bind(to_i64("source_sequence_no", event.sequence_no)?)
        .bind(event.source_event_id.as_str())
        .bind(actor_id)
        .bind(to_i64("expected_version", existing.version)?)
        .bind(revoke_public)
        .bind(now)
        .fetch_optional(&mut **transaction)
        .await
        .map_err(sql_error)?
        .ok_or(WikiPersistenceError::StaleVersion {
            resource: "wiki_source_projection",
            id: existing.id,
            expected: existing.version,
        })?;
    let projection = projection_from_row(&row)?;
    let change = revoke_public.then(|| {
        public_change(
            existing.canonical_route.clone(),
            &projection,
            publication.provider_generation,
            "path_changed",
        )
    });
    Ok((projection, change))
}

async fn mark_eligible(
    store: &SqlxWikiPersistenceStore,
    transaction: &mut Transaction<'_, Any>,
    event: &WikiDriveInboxEvent,
    existing: &WikiSourceProjection,
    actor_id: i64,
    now: &str,
) -> Result<(WikiSourceProjection, Option<WikiPublicRouteChange>), WikiPersistenceError> {
    let timestamp = store.dialect.sql_timestamp_expr("$8");
    let query = format!(
        r#"
        UPDATE kb_source_file_projection
        SET source_state = 'DISCOVERED', source_sequence_no = $4,
            last_source_event_id = $5, processing_attempt_count = 0,
            next_processing_at = NULL, processing_lease_owner = NULL,
            processing_lease_token = NULL, processing_lease_expires_at = NULL,
            processing_fence = processing_fence + 1,
            last_error_code = NULL, last_error_summary = NULL,
            updated_by = $6, updated_at = {timestamp}, version = version + 1
        WHERE tenant_id = $1 AND organization_id = $2 AND id = $3
          AND version = $7 AND status = 1
        RETURNING {PROJECTION_COLUMNS}
        "#,
    );
    let row = sqlx::query(sqlx::AssertSqlSafe(query.as_str()))
        .bind(to_i64("tenant_id", event.scope.tenant_id)?)
        .bind(to_i64("organization_id", event.scope.organization_id)?)
        .bind(to_i64("projection_id", existing.id)?)
        .bind(to_i64("source_sequence_no", event.sequence_no)?)
        .bind(event.source_event_id.as_str())
        .bind(actor_id)
        .bind(to_i64("expected_version", existing.version)?)
        .bind(now)
        .fetch_optional(&mut **transaction)
        .await
        .map_err(sql_error)?
        .ok_or(WikiPersistenceError::StaleVersion {
            resource: "wiki_source_projection",
            id: existing.id,
            expected: existing.version,
        })?;
    Ok((projection_from_row(&row)?, None))
}

#[allow(clippy::too_many_arguments)]
async fn revoke_projection(
    store: &SqlxWikiPersistenceStore,
    transaction: &mut Transaction<'_, Any>,
    event: &WikiDriveInboxEvent,
    publication: &PublicationIdentity,
    existing: &WikiSourceProjection,
    source_state: WikiSourceState,
    publication_state: WikiPagePublicationState,
    reason_code: &str,
    actor_id: i64,
    now: &str,
) -> Result<(WikiSourceProjection, Option<WikiPublicRouteChange>), WikiPersistenceError> {
    let revoke_public = existing.publication_state == WikiPagePublicationState::Published
        && existing.public_drive_version_uuid.is_some();
    let timestamp = store.dialect.sql_timestamp_expr("$12");
    let query = format!(
        r#"
        UPDATE kb_source_file_projection
        SET source_state = $4, publication_state = $5,
            public_drive_version_uuid = CASE WHEN $10 THEN NULL ELSE public_drive_version_uuid END,
            page_public_version = page_public_version + CASE WHEN $10 THEN 1 ELSE 0 END,
            source_sequence_no = $6, last_source_event_id = $7,
            processing_lease_owner = NULL, processing_lease_token = NULL,
            processing_lease_expires_at = NULL, processing_fence = processing_fence + 1,
            last_error_code = $8, last_error_summary = NULL,
            updated_by = $9, updated_at = {timestamp}, version = version + 1
        WHERE tenant_id = $1 AND organization_id = $2 AND id = $3
          AND version = $11 AND status = 1
        RETURNING {PROJECTION_COLUMNS}
        "#,
    );
    let row = sqlx::query(sqlx::AssertSqlSafe(query.as_str()))
        .bind(to_i64("tenant_id", event.scope.tenant_id)?)
        .bind(to_i64("organization_id", event.scope.organization_id)?)
        .bind(to_i64("projection_id", existing.id)?)
        .bind(source_state.as_str())
        .bind(publication_state.as_str())
        .bind(to_i64("source_sequence_no", event.sequence_no)?)
        .bind(event.source_event_id.as_str())
        .bind(reason_code)
        .bind(actor_id)
        .bind(revoke_public)
        .bind(to_i64("expected_version", existing.version)?)
        .bind(now)
        .fetch_optional(&mut **transaction)
        .await
        .map_err(sql_error)?
        .ok_or(WikiPersistenceError::StaleVersion {
            resource: "wiki_source_projection",
            id: existing.id,
            expected: existing.version,
        })?;
    let projection = projection_from_row(&row)?;
    let change = revoke_public.then(|| {
        public_change(
            existing.canonical_route.clone(),
            &projection,
            publication.provider_generation,
            reason_code,
        )
    });
    Ok((projection, change))
}

fn public_change(
    route: Option<String>,
    projection: &WikiSourceProjection,
    provider_generation: u64,
    reason_code: &str,
) -> WikiPublicRouteChange {
    WikiPublicRouteChange {
        event_type: ROUTE_REVOKED_EVENT,
        route,
        page_public_version: projection.page_public_version,
        provider_generation,
        reason_code: reason_code.to_string(),
    }
}

async fn append_public_change_outbox(
    store: &SqlxWikiPersistenceStore,
    transaction: &mut Transaction<'_, Any>,
    event: &WikiDriveInboxEvent,
    publication: &PublicationIdentity,
    change: &WikiPublicRouteChange,
    generations: PublicCollectionGenerations,
    now: &str,
) -> Result<(), WikiPersistenceError> {
    let context = ProviderEventContext {
        source_event: event,
        publication,
        change,
        generations,
        now,
    };
    append_provider_event(store, transaction, &context, change.event_type, "REVOKE").await?;
    append_provider_event(
        store,
        transaction,
        &context,
        NAVIGATION_CHANGED_EVENT,
        "INVALIDATE",
    )
    .await?;
    append_provider_event(
        store,
        transaction,
        &context,
        SEARCH_CHANGED_EVENT,
        "INVALIDATE",
    )
    .await
}

#[derive(Clone, Copy)]
struct PublicCollectionGenerations {
    navigation: u64,
    search: u64,
}

struct ProviderEventContext<'a> {
    source_event: &'a WikiDriveInboxEvent,
    publication: &'a PublicationIdentity,
    change: &'a WikiPublicRouteChange,
    generations: PublicCollectionGenerations,
    now: &'a str,
}

async fn advance_public_collection_generations(
    store: &SqlxWikiPersistenceStore,
    transaction: &mut Transaction<'_, Any>,
    event: &WikiDriveInboxEvent,
    publication_id: u64,
    actor_id: i64,
    now: &str,
) -> Result<PublicCollectionGenerations, WikiPersistenceError> {
    let timestamp = store.dialect.sql_timestamp_expr("$5");
    let query = format!(
        r#"
        UPDATE kb_site_publication
        SET navigation_generation = navigation_generation + 1,
            search_generation = search_generation + 1,
            updated_by = $4, updated_at = {timestamp}, version = version + 1
        WHERE tenant_id = $1 AND organization_id = $2 AND id = $3 AND status = 1
        RETURNING navigation_generation, search_generation
        "#,
    );
    let row = sqlx::query(sqlx::AssertSqlSafe(query.as_str()))
        .bind(to_i64("tenant_id", event.scope.tenant_id)?)
        .bind(to_i64("organization_id", event.scope.organization_id)?)
        .bind(to_i64("publication_id", publication_id)?)
        .bind(actor_id)
        .bind(now)
        .fetch_optional(&mut **transaction)
        .await
        .map_err(sql_error)?
        .ok_or(WikiPersistenceError::NotFound {
            resource: "wiki_publication",
            id: publication_id,
        })?;
    Ok(PublicCollectionGenerations {
        navigation: from_i64(
            "navigation_generation",
            row.try_get("navigation_generation").map_err(row_error)?,
        )?,
        search: from_i64(
            "search_generation",
            row.try_get("search_generation").map_err(row_error)?,
        )?,
    })
}

async fn append_provider_event(
    store: &SqlxWikiPersistenceStore,
    transaction: &mut Transaction<'_, Any>,
    context: &ProviderEventContext<'_>,
    event_type: &str,
    operation: &str,
) -> Result<(), WikiPersistenceError> {
    let outbox_id = store.next_id()?;
    let event_uuid = uuid();
    let event = context.source_event;
    let publication = context.publication;
    let change = context.change;
    let payload = json!({
        "id": event_uuid,
        "type": event_type,
        "source": "sdkwork-knowledgebase",
        "specversion": "1.0",
        "time": context.now,
        "tenantId": event.scope.tenant_id.to_string(),
        "organizationId": event.scope.organization_id.to_string(),
        "subject": format!("wiki-publication:{}", publication.uuid),
        "sequenceNo": outbox_id.to_string(),
        "data": {
            "providerResourceUuid": publication.uuid,
            "providerGeneration": change.provider_generation.to_string(),
            "navigationGeneration": context.generations.navigation.to_string(),
            "searchGeneration": context.generations.search.to_string(),
            "route": change.route,
            "pagePublicVersion": change.page_public_version.to_string(),
            "previousPagePublicVersion": change.page_public_version.saturating_sub(1).to_string(),
            "operation": operation,
            "driveCheckpoint": event.sequence_no.to_string(),
            "reason": change.reason_code,
        }
    });
    let payload_json = serde_json::to_string(&payload)
        .map_err(|error| WikiPersistenceError::Internal(error.to_string()))?;
    let payload_expr = store.dialect.sql_json_expr("$8");
    let timestamp = store.dialect.sql_timestamp_expr("$10");
    let query = format!(
        r#"
        INSERT INTO kb_outbox_event (
            id, uuid, tenant_id, organization_id, aggregate_type, aggregate_id, event_type,
            payload, status, created_at, version
        ) VALUES ($1, $2, $3, $4, $5, $6, $7, {payload_expr}, $9, {timestamp}, 0)
        "#,
    );
    sqlx::query(sqlx::AssertSqlSafe(query.as_str()))
        .bind(outbox_id)
        .bind(event_uuid)
        .bind(to_i64("tenant_id", event.scope.tenant_id)?)
        .bind(to_i64("organization_id", event.scope.organization_id)?)
        .bind("wiki_publication")
        .bind(to_i64("aggregate_id", event.site_publication_id)?)
        .bind(event_type)
        .bind(payload_json)
        .bind(0_i32)
        .bind(context.now)
        .execute(&mut **transaction)
        .await
        .map_err(sql_error)?;
    Ok(())
}

fn validate_metadata(
    metadata: &sdkwork_intelligence_knowledgebase_service::ports::knowledge_wiki_persistence::WikiDriveSourceMetadata,
) -> Result<(), WikiPersistenceError> {
    require_text("drive_version_uuid", &metadata.drive_version_uuid, 64)?;
    validate_relative_path(&metadata.source_path)?;
    require_text("media_type", &metadata.media_type, 255)?;
    require_sha256("content_sha256", &metadata.content_sha256)?;
    Ok(())
}

fn validate_relative_path(path: &str) -> Result<&str, WikiPersistenceError> {
    let path = require_text("source_path", path, 4_096)?;
    if path.starts_with('/')
        || path.contains('\\')
        || path
            .split('/')
            .any(|segment| segment.is_empty() || segment == "." || segment == "..")
    {
        return Err(WikiPersistenceError::InvalidRequest(
            "source_path must be a normalized root-relative path".to_string(),
        ));
    }
    Ok(path)
}

fn validate_revocation(
    source_state: WikiSourceState,
    publication_state: WikiPagePublicationState,
    reason_code: &str,
) -> Result<(), WikiPersistenceError> {
    if !matches!(
        source_state,
        WikiSourceState::Error | WikiSourceState::Quarantined | WikiSourceState::Deleted
    ) || !matches!(
        publication_state,
        WikiPagePublicationState::Unpublished | WikiPagePublicationState::Archived
    ) {
        return Err(WikiPersistenceError::InvalidRequest(
            "revocation must use a non-public terminal or blocked state".to_string(),
        ));
    }
    require_text("reason_code", reason_code, 128)?;
    Ok(())
}
