mod support;

static WIKI_TEST_LOCK: std::sync::Mutex<()> = std::sync::Mutex::new(());

fn wiki_test_guard() -> std::sync::MutexGuard<'static, ()> {
    WIKI_TEST_LOCK
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner())
}

use std::fmt;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;

use sdkwork_intelligence_knowledgebase_repository_sqlx::{
    KnowledgeIdGenerator, KnowledgeIdGeneratorError, SqlxWikiPersistenceStore,
};
use sdkwork_intelligence_knowledgebase_service::ports::knowledge_wiki_persistence::{
    AdvanceWikiReconciliationRequest, ApplyWikiDriveEventRequest, BindWikiSourceScopeRequest,
    ClaimWikiDriveEventsRequest, ClaimWikiReconciliationRequest, ClaimWikiRenditionsRequest,
    ClaimWikiSourceProcessingRequest, CompleteWikiDriveEventRequest,
    CompleteWikiReconciliationRequest, CompleteWikiRenditionRequest,
    CompleteWikiSourceProcessingRequest, ListWikiDriveCheckpointsRequest,
    ListWikiPublicationBackfillCandidatesRequest, MarkWikiPublicationReadyRequest,
    ProvisionWikiDriveCheckpointRequest, ProvisionWikiPublicationRequest,
    ReceiveWikiDriveEventRequest, RetryWikiDriveEventRequest, RetryWikiSourceProcessingRequest,
    UpsertWikiRenditionRequest, UpsertWikiSourceProjectionRequest, WikiDriveCheckpointStore,
    WikiDriveEventInboxStore, WikiDriveEventProcessingState, WikiDriveEventReceiveDisposition,
    WikiDriveEventType, WikiDriveProjectionMutation, WikiDriveSourceMetadata, WikiDriveStreamState,
    WikiIndexState, WikiPagePublicationState, WikiPersistenceError, WikiPersistenceScope,
    WikiPublicationBackfillStore, WikiPublicationStatus, WikiPublicationStore, WikiRenditionKind,
    WikiRenditionState, WikiRenditionStore, WikiSourceFileKind, WikiSourceProjectionStore,
    WikiSourceProjectionUpsertDisposition, WikiSourceState,
};
use sdkwork_utils_rust::sha256_hash;

const SCOPE: WikiPersistenceScope = WikiPersistenceScope {
    tenant_id: 101,
    organization_id: 202,
};

#[tokio::test]
async fn publication_and_checkpoint_provisioning_are_idempotent_scoped_and_optimistic() {
    let _guard = wiki_test_guard();
    let Some((pool, store)) = test_store().await else {
        eprintln!("skipping wiki postgres test: set SDKWORK_DATABASE_URL or DATABASE_URL to a postgres URL");
        return;
    };
    insert_space(&pool, SCOPE, 501, "drive-space-501").await;

    let first = store
        .provision_publication(publication_request())
        .await
        .expect("provision publication");
    assert!(first.created);
    assert_eq!(first.publication.wiki_status, WikiPublicationStatus::Draft);

    let replay = store
        .provision_publication(publication_request())
        .await
        .expect("replay publication provisioning");
    assert!(!replay.created);
    assert_eq!(replay.publication.id, first.publication.id);

    let bound = store
        .bind_source_scope(BindWikiSourceScopeRequest {
            scope: SCOPE,
            site_publication_id: first.publication.id,
            source_root_node_uuid: "raw-root-node".to_string(),
            source_scope_uuid: "raw-root-scope".to_string(),
            expected_version: first.publication.version,
            actor_id: 9001,
        })
        .await
        .expect("bind canonical raw source scope");
    assert_eq!(bound.wiki_status, WikiPublicationStatus::Validating);

    let stale = store
        .bind_source_scope(BindWikiSourceScopeRequest {
            scope: SCOPE,
            site_publication_id: first.publication.id,
            source_root_node_uuid: "different-root".to_string(),
            source_scope_uuid: "different-scope".to_string(),
            expected_version: first.publication.version,
            actor_id: 9001,
        })
        .await
        .expect_err("stale source-scope update must fail");
    assert!(matches!(stale, WikiPersistenceError::Conflict(_)));

    let checkpoint = store
        .provision_checkpoint(checkpoint_request(bound.id))
        .await
        .expect("provision checkpoint");
    let checkpoint_replay = store
        .provision_checkpoint(checkpoint_request(bound.id))
        .await
        .expect("replay checkpoint provisioning");
    assert_eq!(checkpoint_replay.id, checkpoint.id);
    let ready = store
        .mark_publication_ready(MarkWikiPublicationReadyRequest {
            scope: SCOPE,
            site_publication_id: bound.id,
            expected_version: bound.version,
            actor_id: 9001,
        })
        .await
        .expect("mark fully initialized publication ready");
    assert_eq!(ready.wiki_status, WikiPublicationStatus::Ready);

    let wrong_scope = WikiPersistenceScope {
        tenant_id: SCOPE.tenant_id,
        organization_id: SCOPE.organization_id + 1,
    };
    assert!(store
        .get_publication_for_space(wrong_scope, 501)
        .await
        .expect("cross-organization lookup")
        .is_none());
    assert!(matches!(
        store.get_checkpoint(wrong_scope, checkpoint.id).await,
        Err(WikiPersistenceError::NotFound { .. })
    ));
}

#[tokio::test]
async fn backfill_candidates_are_bounded_keyset_ordered_and_exclude_complete_publications() {
    let _guard = wiki_test_guard();
    let Some((pool, store)) = test_store().await else {
        eprintln!("skipping wiki postgres test: set SDKWORK_DATABASE_URL or DATABASE_URL to a postgres URL");
        return;
    };
    for (space_id, drive_space_uuid) in [
        (501, "drive-space-501"),
        (502, "drive-space-502"),
        (503, "drive-space-503"),
    ] {
        insert_space(&pool, SCOPE, space_id, drive_space_uuid).await;
    }

    store
        .provision_publication(publication_request_for(501, "drive-space-501"))
        .await
        .expect("provision incomplete publication");
    let complete = store
        .provision_publication(publication_request_for(503, "drive-space-503"))
        .await
        .expect("provision complete publication")
        .publication;
    let complete = store
        .bind_source_scope(BindWikiSourceScopeRequest {
            scope: SCOPE,
            site_publication_id: complete.id,
            source_root_node_uuid: "raw-root-503".to_string(),
            source_scope_uuid: "raw-scope-503".to_string(),
            expected_version: complete.version,
            actor_id: 9001,
        })
        .await
        .expect("bind complete publication");
    store
        .provision_checkpoint(ProvisionWikiDriveCheckpointRequest {
            scope: SCOPE,
            site_publication_id: complete.id,
            drive_space_uuid: "drive-space-503".to_string(),
            source_scope_uuid: "raw-scope-503".to_string(),
            actor_id: 9001,
        })
        .await
        .expect("provision complete checkpoint");

    let first = store
        .list_backfill_candidates(ListWikiPublicationBackfillCandidatesRequest {
            scope: SCOPE,
            after_space_id: None,
            limit: 1,
        })
        .await
        .expect("list first backfill page");
    assert_eq!(first.candidates.len(), 1);
    assert_eq!(first.candidates[0].space_id, 501);
    assert!(!first.candidates[0].publication_missing);
    assert!(first.candidates[0].source_scope_missing);
    assert!(first.candidates[0].checkpoint_missing);
    assert_eq!(first.next_after_space_id, Some(501));

    let second = store
        .list_backfill_candidates(ListWikiPublicationBackfillCandidatesRequest {
            scope: SCOPE,
            after_space_id: first.next_after_space_id,
            limit: 1,
        })
        .await
        .expect("list second backfill page");
    assert_eq!(second.candidates.len(), 1);
    assert_eq!(second.candidates[0].space_id, 502);
    assert!(second.candidates[0].publication_missing);
    assert!(second.candidates[0].source_scope_missing);
    assert!(second.candidates[0].checkpoint_missing);
    assert_eq!(second.next_after_space_id, None);
}

#[tokio::test]
async fn checkpoint_listing_is_bounded_keyset_ordered_and_scope_isolated() {
    let _guard = wiki_test_guard();
    let Some((pool, store)) = test_store().await else {
        eprintln!("skipping wiki postgres test: set SDKWORK_DATABASE_URL or DATABASE_URL to a postgres URL");
        return;
    };
    let mut expected = Vec::new();
    for space_id in [501, 502, 503] {
        expected.push(
            provision_checkpoint_for_scope(
                &pool,
                &store,
                SCOPE,
                space_id,
                &format!("drive-space-{space_id}"),
            )
            .await,
        );
    }
    let other_scope = WikiPersistenceScope {
        tenant_id: SCOPE.tenant_id,
        organization_id: SCOPE.organization_id + 1,
    };
    let other_pool = support::scoped_pool(other_scope.tenant_id, other_scope.organization_id)
        .await
        .expect("connect other-scope wiki pool");
    let other_store = SqlxWikiPersistenceStore::new(other_pool.clone());
    let other_checkpoint = provision_checkpoint_for_scope(
        &other_pool,
        &other_store,
        other_scope,
        601,
        "drive-space-601",
    )
    .await;

    let first = store
        .list_checkpoints(ListWikiDriveCheckpointsRequest {
            scope: SCOPE,
            after_checkpoint_id: None,
            limit: 2,
        })
        .await
        .expect("list first checkpoint page");
    assert_eq!(
        first
            .checkpoints
            .iter()
            .map(|checkpoint| checkpoint.id)
            .collect::<Vec<_>>(),
        vec![expected[0].id, expected[1].id]
    );
    assert_eq!(first.next_after_checkpoint_id, Some(expected[1].id));

    let second = store
        .list_checkpoints(ListWikiDriveCheckpointsRequest {
            scope: SCOPE,
            after_checkpoint_id: first.next_after_checkpoint_id,
            limit: 2,
        })
        .await
        .expect("list second checkpoint page");
    assert_eq!(second.checkpoints.len(), 1);
    assert_eq!(second.checkpoints[0].id, expected[2].id);
    assert_eq!(second.next_after_checkpoint_id, None);

    let exhausted = store
        .list_checkpoints(ListWikiDriveCheckpointsRequest {
            scope: SCOPE,
            after_checkpoint_id: Some(expected[2].id),
            limit: 2,
        })
        .await
        .expect("list exhausted checkpoint page");
    assert!(exhausted.checkpoints.is_empty());
    assert_eq!(exhausted.next_after_checkpoint_id, None);

    let isolated = other_store
        .list_checkpoints(ListWikiDriveCheckpointsRequest {
            scope: other_scope,
            after_checkpoint_id: None,
            limit: 2,
        })
        .await
        .expect("list isolated checkpoint page");
    assert_eq!(isolated.checkpoints.len(), 1);
    assert_eq!(isolated.checkpoints[0].id, other_checkpoint.id);

    for limit in [0, 201] {
        assert!(matches!(
            store
                .list_checkpoints(ListWikiDriveCheckpointsRequest {
                    scope: SCOPE,
                    after_checkpoint_id: None,
                    limit,
                })
                .await,
            Err(WikiPersistenceError::InvalidRequest(_))
        ));
    }
}

#[tokio::test]
async fn stable_drive_node_upsert_preserves_identity_and_fences_processing() {
    let _guard = wiki_test_guard();
    let Some((pool, store)) = test_store().await else {
        eprintln!("skipping wiki postgres test: set SDKWORK_DATABASE_URL or DATABASE_URL to a postgres URL");
        return;
    };
    insert_space(&pool, SCOPE, 501, "drive-space-501").await;
    let publication = provision_bound_publication(&store).await;

    let first_request = source_request(
        publication.id,
        1,
        "drive-version-1",
        "sha256:aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa",
    );
    let first = store
        .upsert_source_projection(first_request.clone())
        .await
        .expect("create source projection");
    assert_eq!(
        first.disposition,
        WikiSourceProjectionUpsertDisposition::Created
    );

    let replay = store
        .upsert_source_projection(first_request.clone())
        .await
        .expect("replay source projection");
    assert_eq!(
        replay.disposition,
        WikiSourceProjectionUpsertDisposition::UnchangedReplay
    );
    assert_eq!(replay.projection.id, first.projection.id);

    let mut conflict = first_request.clone();
    conflict.content_sha256 =
        "sha256:bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb".to_string();
    assert!(matches!(
        store.upsert_source_projection(conflict).await,
        Err(WikiPersistenceError::Conflict(_))
    ));

    let updated = store
        .upsert_source_projection(source_request(
            publication.id,
            3,
            "drive-version-3",
            "sha256:cccccccccccccccccccccccccccccccccccccccccccccccccccccccccccccccc",
        ))
        .await
        .expect("advance source projection");
    assert_eq!(updated.projection.id, first.projection.id);
    assert_eq!(
        updated.disposition,
        WikiSourceProjectionUpsertDisposition::Updated
    );

    let stale = store
        .upsert_source_projection(source_request(
            publication.id,
            2,
            "drive-version-2",
            "sha256:dddddddddddddddddddddddddddddddddddddddddddddddddddddddddddddddd",
        ))
        .await
        .expect("ignore stale source version");
    assert_eq!(
        stale.disposition,
        WikiSourceProjectionUpsertDisposition::IgnoredStale
    );
    assert_eq!(stale.projection.drive_version_uuid, "drive-version-3");

    let claimed = store
        .claim_source_processing(ClaimWikiSourceProcessingRequest {
            scope: SCOPE,
            site_publication_id: publication.id,
            claim_owner: "wiki-worker-1".to_string(),
            lease_seconds: 60,
            after_id: None,
            limit: 10,
        })
        .await
        .expect("claim source processing");
    assert_eq!(claimed.len(), 1);
    assert_eq!(claimed[0].source_state, WikiSourceState::Processing);
    let lease_token = claimed[0]
        .processing_lease_token
        .clone()
        .expect("processing lease token");
    let completed = store
        .complete_source_processing(CompleteWikiSourceProcessingRequest {
            scope: SCOPE,
            site_publication_id: publication.id,
            projection_id: claimed[0].id,
            lease_token,
            processing_fence: claimed[0].processing_fence,
            canonical_route: "/guide/getting-started".to_string(),
            index_state: WikiIndexState::Ready,
            actor_id: 9001,
        })
        .await
        .expect("complete source processing");
    assert_eq!(completed.source_state, WikiSourceState::Ready);
    assert_eq!(
        completed.canonical_route.as_deref(),
        Some("/guide/getting-started")
    );

    let rendition_request = UpsertWikiRenditionRequest {
        scope: SCOPE,
        site_publication_id: publication.id,
        source_file_projection_id: completed.id,
        drive_version_uuid: completed.drive_version_uuid.clone(),
        source_content_sha256: completed.content_sha256.clone(),
        processor_id: "sdkwork.markdown".to_string(),
        processor_version: "1".to_string(),
        policy_version: "1".to_string(),
        rendition_kind: WikiRenditionKind::SanitizedHtml,
        rendition_key_sha256:
            "sha256:eeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeee".to_string(),
        actor_id: 9001,
    };
    let rendition = store
        .upsert_rendition(rendition_request.clone())
        .await
        .expect("create rendition");
    let rendition_replay = store
        .upsert_rendition(rendition_request)
        .await
        .expect("replay rendition");
    assert_eq!(rendition_replay.id, rendition.id);

    let claimed_renditions = store
        .claim_renditions(ClaimWikiRenditionsRequest {
            scope: SCOPE,
            site_publication_id: publication.id,
            claim_owner: "wiki-rendition-worker-1".to_string(),
            lease_seconds: 60,
            after_id: None,
            limit: 10,
        })
        .await
        .expect("claim rendition");
    assert_eq!(claimed_renditions.len(), 1);
    assert_eq!(
        claimed_renditions[0].rendition_state,
        WikiRenditionState::Processing
    );
    let completed_rendition = store
        .complete_rendition(CompleteWikiRenditionRequest {
            scope: SCOPE,
            rendition_id: claimed_renditions[0].id,
            lease_token: claimed_renditions[0]
                .processing_lease_token
                .clone()
                .expect("rendition lease token"),
            processing_fence: claimed_renditions[0].processing_fence,
            rendition_drive_space_uuid: "generated-drive-space".to_string(),
            rendition_drive_node_uuid: "generated-node".to_string(),
            rendition_drive_version_uuid: "generated-version".to_string(),
            content_sha256:
                "sha256:ffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffff"
                    .to_string(),
            media_type: "text/html".to_string(),
            size_bytes: 1_024,
            actor_id: 9001,
        })
        .await
        .expect("complete rendition");
    assert_eq!(
        completed_rendition.rendition_state,
        WikiRenditionState::Ready
    );

    sqlx::query(
        r#"
        UPDATE kb_source_file_projection
        SET publication_state = 'PUBLISHED', visibility = 'PUBLIC',
            public_drive_version_uuid = drive_version_uuid, page_public_version = 1
        WHERE id = $1
        "#,
    )
    .bind(i64::try_from(completed.id).unwrap())
    .execute(&pool)
    .await
    .expect("publish verified source snapshot");
    let next_version = store
        .upsert_source_projection(source_request(
            publication.id,
            4,
            "drive-version-4",
            "sha256:abababababababababababababababababababababababababababababababab",
        ))
        .await
        .expect("start processing next immutable Drive version")
        .projection;
    assert_eq!(next_version.source_state, WikiSourceState::Discovered);
    assert_eq!(
        next_version.publication_state,
        WikiPagePublicationState::Published
    );
    assert_eq!(
        next_version.public_drive_version_uuid.as_deref(),
        Some("drive-version-3")
    );
    assert_eq!(
        next_version.canonical_route.as_deref(),
        Some("/guide/getting-started")
    );
    let public_lookup_count: i64 = sqlx::query_scalar(
        r#"
        SELECT COUNT(*) FROM kb_source_file_projection
        WHERE site_publication_id = $1 AND canonical_route = $2
          AND publication_state = 'PUBLISHED' AND visibility = 'PUBLIC'
        "#,
    )
    .bind(i64::try_from(publication.id).unwrap())
    .bind("/guide/getting-started")
    .fetch_one(&pool)
    .await
    .expect("resolve pinned public snapshot while next version processes");
    assert_eq!(public_lookup_count, 1);

    let retried_claim = store
        .claim_source_processing(ClaimWikiSourceProcessingRequest {
            scope: SCOPE,
            site_publication_id: publication.id,
            claim_owner: "wiki-worker-2".to_string(),
            lease_seconds: 60,
            after_id: None,
            limit: 10,
        })
        .await
        .expect("claim next source version");
    assert_eq!(retried_claim.len(), 1);
    let quarantined = store
        .retry_source_processing(RetryWikiSourceProcessingRequest {
            scope: SCOPE,
            projection_id: retried_claim[0].id,
            lease_token: retried_claim[0]
                .processing_lease_token
                .clone()
                .expect("source retry lease token"),
            processing_fence: retried_claim[0].processing_fence,
            error_code: "wiki_source_rendering_failed".to_string(),
            error_summary: "Wiki source processing failed".to_string(),
            retry_delay_seconds: 30,
            max_attempts: 1,
            actor_id: 9001,
        })
        .await
        .expect("quarantine exhausted source processing");
    assert_eq!(quarantined.source_state, WikiSourceState::Quarantined);
}

#[tokio::test]
async fn drive_inbox_detects_gaps_deduplicates_and_advances_strictly_in_order() {
    let _guard = wiki_test_guard();
    let Some((pool, store)) = test_store().await else {
        eprintln!("skipping wiki postgres test: set SDKWORK_DATABASE_URL or DATABASE_URL to a postgres URL");
        return;
    };
    insert_space(&pool, SCOPE, 501, "drive-space-501").await;
    let publication = provision_bound_publication(&store).await;
    let checkpoint = store
        .provision_checkpoint(checkpoint_request(publication.id))
        .await
        .expect("provision checkpoint");

    let second = drive_event(publication.id, checkpoint.id, 2, "event-2");
    let deferred = store
        .receive_event(second.clone())
        .await
        .expect("record out-of-order event");
    assert_eq!(
        deferred.disposition,
        WikiDriveEventReceiveDisposition::DeferredGap
    );
    let gap_checkpoint = store
        .get_checkpoint(SCOPE, checkpoint.id)
        .await
        .expect("read gap checkpoint");
    assert_eq!(gap_checkpoint.gap_from_sequence_no, Some(1));
    assert_eq!(gap_checkpoint.gap_to_sequence_no, Some(1));

    let first = drive_event(publication.id, checkpoint.id, 1, "event-1");
    let ready = store
        .receive_event(first.clone())
        .await
        .expect("record missing event");
    assert_eq!(ready.disposition, WikiDriveEventReceiveDisposition::Ready);
    let duplicate = store
        .receive_event(first.clone())
        .await
        .expect("deduplicate exact replay");
    assert_eq!(
        duplicate.disposition,
        WikiDriveEventReceiveDisposition::Duplicate
    );

    let mut sequence_conflict = first.clone();
    sequence_conflict.source_event_id = "event-1-conflict".to_string();
    sequence_conflict.payload_json = "{\"different\":true}".to_string();
    sequence_conflict.payload_sha256 = payload_hash(&sequence_conflict.payload_json);
    assert!(matches!(
        store.receive_event(sequence_conflict).await,
        Err(WikiPersistenceError::Conflict(_))
    ));

    let first_claim = store
        .claim_events(claim_events_request(checkpoint.id))
        .await
        .expect("claim first sequence");
    assert_eq!(first_claim.len(), 1);
    assert_eq!(first_claim[0].sequence_no, 1);
    store
        .complete_event(CompleteWikiDriveEventRequest {
            scope: SCOPE,
            event_id: first_claim[0].id,
            lease_token: first_claim[0]
                .lease_token
                .clone()
                .expect("event lease token"),
            actor_id: 9001,
        })
        .await
        .expect("complete first event");

    let second_claim = store
        .claim_events(claim_events_request(checkpoint.id))
        .await
        .expect("claim second sequence");
    assert_eq!(second_claim.len(), 1);
    assert_eq!(second_claim[0].sequence_no, 2);
    store
        .complete_event(CompleteWikiDriveEventRequest {
            scope: SCOPE,
            event_id: second_claim[0].id,
            lease_token: second_claim[0]
                .lease_token
                .clone()
                .expect("event lease token"),
            actor_id: 9001,
        })
        .await
        .expect("complete second event");

    let completed_checkpoint = store
        .get_checkpoint(SCOPE, checkpoint.id)
        .await
        .expect("read completed checkpoint");
    assert_eq!(completed_checkpoint.last_sequence_no, 2);
    assert_eq!(completed_checkpoint.gap_from_sequence_no, None);
    assert_eq!(completed_checkpoint.gap_to_sequence_no, None);
    let advanced_publication = store
        .get_publication(SCOPE, publication.id)
        .await
        .expect("read advanced publication");
    assert_eq!(advanced_publication.last_projected_drive_checkpoint, 2);
    assert_eq!(advanced_publication.provider_generation, 1);
}

#[tokio::test]
async fn drive_event_application_atomically_revokes_public_route_and_advances_checkpoint() {
    let _guard = wiki_test_guard();
    let Some((pool, store)) = test_store().await else {
        eprintln!("skipping wiki postgres test: set SDKWORK_DATABASE_URL or DATABASE_URL to a postgres URL");
        return;
    };
    insert_space(&pool, SCOPE, 501, "drive-space-501").await;
    let publication = provision_bound_publication(&store).await;
    let checkpoint = store
        .provision_checkpoint(checkpoint_request(publication.id))
        .await
        .expect("provision checkpoint");

    store
        .receive_event(drive_event(publication.id, checkpoint.id, 1, "event-1"))
        .await
        .expect("receive source event");
    let first = store
        .claim_events(claim_events_request(checkpoint.id))
        .await
        .expect("claim source event")
        .pop()
        .expect("source event");
    let applied = store
        .apply_event(ApplyWikiDriveEventRequest {
            complete: CompleteWikiDriveEventRequest {
                scope: SCOPE,
                event_id: first.id,
                lease_token: first.lease_token.expect("event lease"),
                actor_id: 9001,
            },
            mutation: WikiDriveProjectionMutation::Upsert(WikiDriveSourceMetadata {
                drive_version_uuid: "drive-version-1".to_string(),
                source_path: "guide/getting-started.md".to_string(),
                file_kind: WikiSourceFileKind::Page,
                media_type: "text/markdown".to_string(),
                size_bytes: 512,
                content_sha256:
                    "sha256:aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa"
                        .to_string(),
            }),
        })
        .await
        .expect("apply source event");
    let projection = applied.projection.expect("source projection");
    assert!(applied.public_route_change.is_none());

    sqlx::query(
        r#"
        UPDATE kb_source_file_projection
        SET source_state = 'READY', canonical_route = '/guide/getting-started/',
            publication_state = 'PUBLISHED', visibility = 'PUBLIC',
            public_drive_version_uuid = drive_version_uuid, page_public_version = 1
        WHERE id = $1
        "#,
    )
    .bind(i64::try_from(projection.id).unwrap())
    .execute(&pool)
    .await
    .expect("publish verified route");

    store
        .receive_event(drive_event(publication.id, checkpoint.id, 2, "event-2"))
        .await
        .expect("receive revocation event");
    let second = store
        .claim_events(claim_events_request(checkpoint.id))
        .await
        .expect("claim revocation event")
        .pop()
        .expect("revocation event");
    let revoked = store
        .apply_event(ApplyWikiDriveEventRequest {
            complete: CompleteWikiDriveEventRequest {
                scope: SCOPE,
                event_id: second.id,
                lease_token: second.lease_token.expect("event lease"),
                actor_id: 9001,
            },
            mutation: WikiDriveProjectionMutation::Revoke {
                source_state: WikiSourceState::Quarantined,
                publication_state: WikiPagePublicationState::Unpublished,
                reason_code: "drive_quarantined".to_string(),
            },
        })
        .await
        .expect("atomically revoke route");
    let projection = revoked.projection.expect("revoked projection");
    let public_change = revoked.public_route_change.expect("public revocation");
    assert_eq!(projection.source_state, WikiSourceState::Quarantined);
    assert_eq!(
        projection.publication_state,
        WikiPagePublicationState::Unpublished
    );
    assert_eq!(projection.public_drive_version_uuid, None);
    assert_eq!(projection.page_public_version, 2);
    assert_eq!(
        public_change.route.as_deref(),
        Some("/guide/getting-started/")
    );

    let checkpoint = store
        .get_checkpoint(SCOPE, checkpoint.id)
        .await
        .expect("advanced checkpoint");
    assert_eq!(checkpoint.last_sequence_no, 2);
    let outbox: Vec<(String, String, String)> = sqlx::query_as(
        "SELECT uuid, event_type, CAST(payload AS TEXT) FROM kb_outbox_event WHERE aggregate_id = $1 ORDER BY id ASC",
    )
    .bind(i64::try_from(publication.id).unwrap())
    .fetch_all(&pool)
    .await
    .expect("transactional public revocation outbox");
    assert_eq!(
        outbox
            .iter()
            .map(|event| event.1.as_str())
            .collect::<Vec<_>>(),
        [
            "knowledgebase.wiki.route.revoked.v1",
            "knowledgebase.wiki.navigation.changed.v1",
            "knowledgebase.wiki.search.changed.v1",
        ]
    );
    for (uuid, _, payload) in &outbox {
        let payload: serde_json::Value =
            serde_json::from_str(payload).expect("provider event JSON");
        assert_eq!(payload["id"], uuid.as_str());
        assert!(payload["sequenceNo"].as_str().is_some());
        assert_eq!(payload["data"]["navigationGeneration"], "2");
        assert_eq!(payload["data"]["searchGeneration"], "2");
        assert_eq!(payload["data"]["pagePublicVersion"], "2");
        assert_eq!(payload["data"]["previousPagePublicVersion"], "1");
        assert!(payload.to_string().contains("drive_quarantined"));
        assert!(!payload.to_string().contains("objectKey"));
    }
    let generations: (i64, i64) = sqlx::query_as(
        "SELECT navigation_generation, search_generation FROM kb_site_publication WHERE id = $1",
    )
    .bind(i64::try_from(publication.id).unwrap())
    .fetch_one(&pool)
    .await
    .expect("advanced public collection generations");
    assert_eq!(generations, (2, 2));
}

#[tokio::test]
async fn invalid_drive_projection_mutation_rolls_back_before_checkpoint_advance() {
    let _guard = wiki_test_guard();
    let Some((pool, store)) = test_store().await else {
        eprintln!("skipping wiki postgres test: set SDKWORK_DATABASE_URL or DATABASE_URL to a postgres URL");
        return;
    };
    insert_space(&pool, SCOPE, 501, "drive-space-501").await;
    let publication = provision_bound_publication(&store).await;
    let checkpoint = store
        .provision_checkpoint(checkpoint_request(publication.id))
        .await
        .expect("provision checkpoint");
    store
        .receive_event(drive_event(publication.id, checkpoint.id, 1, "event-1"))
        .await
        .expect("receive source event");
    let event = store
        .claim_events(claim_events_request(checkpoint.id))
        .await
        .expect("claim source event")
        .pop()
        .expect("source event");
    let error = store
        .apply_event(ApplyWikiDriveEventRequest {
            complete: CompleteWikiDriveEventRequest {
                scope: SCOPE,
                event_id: event.id,
                lease_token: event.lease_token.expect("event lease"),
                actor_id: 9001,
            },
            mutation: WikiDriveProjectionMutation::Upsert(WikiDriveSourceMetadata {
                drive_version_uuid: "drive-version-1".to_string(),
                source_path: "../private.md".to_string(),
                file_kind: WikiSourceFileKind::Page,
                media_type: "text/markdown".to_string(),
                size_bytes: 512,
                content_sha256:
                    "sha256:aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa"
                        .to_string(),
            }),
        })
        .await
        .expect_err("invalid path must roll back the event transaction");
    assert!(matches!(error, WikiPersistenceError::InvalidRequest(_)));
    assert_eq!(
        store
            .get_checkpoint(SCOPE, checkpoint.id)
            .await
            .expect("checkpoint after rollback")
            .last_sequence_no,
        0
    );
    let projection_count: i64 =
        sqlx::query_scalar("SELECT COUNT(*) FROM kb_source_file_projection")
            .fetch_one(&pool)
            .await
            .expect("count projections");
    let outbox_count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM kb_outbox_event")
        .fetch_one(&pool)
        .await
        .expect("count outbox");
    assert_eq!(projection_count, 0);
    assert_eq!(outbox_count, 0);
}

#[tokio::test]
async fn dead_lettered_gap_can_be_reconciled_with_a_fenced_bounded_checkpoint() {
    let _guard = wiki_test_guard();
    let Some((pool, store)) = test_store().await else {
        eprintln!("skipping wiki postgres test: set SDKWORK_DATABASE_URL or DATABASE_URL to a postgres URL");
        return;
    };
    insert_space(&pool, SCOPE, 501, "drive-space-501").await;
    let publication = provision_bound_publication(&store).await;
    let checkpoint = store
        .provision_checkpoint(checkpoint_request(publication.id))
        .await
        .expect("provision checkpoint");
    store
        .receive_event(drive_event(publication.id, checkpoint.id, 2, "event-2"))
        .await
        .expect("record gap event");
    store
        .receive_event(drive_event(publication.id, checkpoint.id, 1, "event-1"))
        .await
        .expect("record first event");

    let claimed = store
        .claim_events(claim_events_request(checkpoint.id))
        .await
        .expect("claim first event");
    let dead_lettered = store
        .retry_event(RetryWikiDriveEventRequest {
            scope: SCOPE,
            event_id: claimed[0].id,
            lease_token: claimed[0].lease_token.clone().expect("event lease token"),
            error_code: "DRIVE_READ_FAILED".to_string(),
            error_summary: "bounded Drive read failed".to_string(),
            retry_delay_seconds: 30,
            max_attempts: 1,
        })
        .await
        .expect("dead letter exhausted event");
    assert_eq!(
        dead_lettered.processing_state,
        WikiDriveEventProcessingState::DeadLetter
    );

    let gap_checkpoint = store
        .get_checkpoint(SCOPE, checkpoint.id)
        .await
        .expect("read gap checkpoint");
    let reconciliation = store
        .claim_reconciliation(ClaimWikiReconciliationRequest {
            scope: SCOPE,
            checkpoint_id: checkpoint.id,
            claim_owner: "wiki-reconciler-1".to_string(),
            lease_seconds: 300,
            expected_version: gap_checkpoint.version,
            actor_id: 9001,
        })
        .await
        .expect("claim reconciliation");
    assert_eq!(
        reconciliation.stream_state,
        WikiDriveStreamState::Reconciling
    );
    let advanced = store
        .advance_reconciliation(AdvanceWikiReconciliationRequest {
            scope: SCOPE,
            checkpoint_id: checkpoint.id,
            lease_token: reconciliation
                .lease_token
                .clone()
                .expect("reconciliation lease token"),
            fence_token: reconciliation.fence_token,
            reconciliation_cursor: "drive-node-page-1".to_string(),
            actor_id: 9001,
        })
        .await
        .expect("advance reconciliation cursor");
    assert_eq!(
        advanced.reconciliation_cursor.as_deref(),
        Some("drive-node-page-1")
    );
    let completed = store
        .complete_reconciliation(CompleteWikiReconciliationRequest {
            scope: SCOPE,
            checkpoint_id: checkpoint.id,
            lease_token: advanced
                .lease_token
                .clone()
                .expect("reconciliation lease token"),
            fence_token: advanced.fence_token,
            reconciled_sequence_no: 2,
            last_event_id: Some("event-2".to_string()),
            actor_id: 9001,
        })
        .await
        .expect("complete reconciliation");
    assert_eq!(completed.stream_state, WikiDriveStreamState::Healthy);
    assert_eq!(completed.last_sequence_no, 2);
    assert_eq!(completed.gap_from_sequence_no, None);
    assert_eq!(completed.lease_token, None);

    let ignored_count: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM kb_drive_event_inbox WHERE checkpoint_id = $1 AND processing_state = 'IGNORED'",
    )
    .bind(i64::try_from(checkpoint.id).unwrap())
    .fetch_one(&pool)
    .await
    .expect("count reconciled inbox events");
    assert_eq!(ignored_count, 2);
}

async fn test_store() -> Option<(sqlx::AnyPool, SqlxWikiPersistenceStore)> {
    let pool = support::scoped_pool(101, 202).await?;
    support::clean_wiki_scope(&pool, 101, 202).await;
    let generator = Arc::new(TestIdGenerator::new(10_000));
    let store = SqlxWikiPersistenceStore::with_id_generator(pool.clone(), generator);
    Some((pool, store))
}

async fn insert_space(
    pool: &sqlx::AnyPool,
    scope: WikiPersistenceScope,
    space_id: u64,
    drive_space_uuid: &str,
) {
    sqlx::query(
        r#"
        INSERT INTO kb_space (
            id, uuid, tenant_id, organization_id, name, drive_space_id,
            status, created_at, updated_at
        ) VALUES ($1, $2, $3, $4, $5, $6, 1, $7::timestamptz, $7::timestamptz)
        ON CONFLICT (id) DO NOTHING
        "#,
    )
    .bind(i64::try_from(space_id).unwrap())
    .bind(format!("space-{space_id}"))
    .bind(i64::try_from(scope.tenant_id).unwrap())
    .bind(i64::try_from(scope.organization_id).unwrap())
    .bind(format!("Knowledge Space {space_id}"))
    .bind(drive_space_uuid)
    .bind("2026-07-21T00:00:00Z")
    .execute(pool)
    .await
    .expect("insert knowledge space");
}

fn publication_request() -> ProvisionWikiPublicationRequest {
    publication_request_for(501, "drive-space-501")
}

fn publication_request_for(
    space_id: u64,
    drive_space_uuid: &str,
) -> ProvisionWikiPublicationRequest {
    ProvisionWikiPublicationRequest {
        scope: SCOPE,
        space_id,
        drive_space_uuid: drive_space_uuid.to_string(),
        title: "SDKWork Wiki".to_string(),
        actor_id: 9001,
    }
}

async fn provision_bound_publication(
    store: &SqlxWikiPersistenceStore,
) -> sdkwork_intelligence_knowledgebase_service::ports::knowledge_wiki_persistence::WikiPublication
{
    let publication = store
        .provision_publication(publication_request())
        .await
        .expect("provision publication")
        .publication;
    store
        .bind_source_scope(BindWikiSourceScopeRequest {
            scope: SCOPE,
            site_publication_id: publication.id,
            source_root_node_uuid: "raw-root-node".to_string(),
            source_scope_uuid: "raw-root-scope".to_string(),
            expected_version: publication.version,
            actor_id: 9001,
        })
        .await
        .expect("bind source scope")
}

async fn provision_checkpoint_for_scope(
    pool: &sqlx::AnyPool,
    store: &SqlxWikiPersistenceStore,
    scope: WikiPersistenceScope,
    space_id: u64,
    drive_space_uuid: &str,
) -> sdkwork_intelligence_knowledgebase_service::ports::knowledge_wiki_persistence::WikiDriveCheckpoint
{
    insert_space(pool, scope, space_id, drive_space_uuid).await;
    let publication = store
        .provision_publication(ProvisionWikiPublicationRequest {
            scope,
            space_id,
            drive_space_uuid: drive_space_uuid.to_string(),
            title: format!("Wiki {space_id}"),
            actor_id: 9001,
        })
        .await
        .expect("provision scoped publication")
        .publication;
    let source_scope_uuid = format!("raw-scope-{space_id}");
    let publication = store
        .bind_source_scope(BindWikiSourceScopeRequest {
            scope,
            site_publication_id: publication.id,
            source_root_node_uuid: format!("raw-root-{space_id}"),
            source_scope_uuid: source_scope_uuid.clone(),
            expected_version: publication.version,
            actor_id: 9001,
        })
        .await
        .expect("bind scoped source root");
    store
        .provision_checkpoint(ProvisionWikiDriveCheckpointRequest {
            scope,
            site_publication_id: publication.id,
            drive_space_uuid: drive_space_uuid.to_string(),
            source_scope_uuid,
            actor_id: 9001,
        })
        .await
        .expect("provision scoped checkpoint")
}

fn checkpoint_request(site_publication_id: u64) -> ProvisionWikiDriveCheckpointRequest {
    ProvisionWikiDriveCheckpointRequest {
        scope: SCOPE,
        site_publication_id,
        drive_space_uuid: "drive-space-501".to_string(),
        source_scope_uuid: "raw-root-scope".to_string(),
        actor_id: 9001,
    }
}

fn source_request(
    site_publication_id: u64,
    sequence_no: u64,
    drive_version_uuid: &str,
    content_sha256: &str,
) -> UpsertWikiSourceProjectionRequest {
    UpsertWikiSourceProjectionRequest {
        scope: SCOPE,
        site_publication_id,
        space_id: 501,
        drive_space_uuid: "drive-space-501".to_string(),
        drive_node_uuid: "drive-node-guide".to_string(),
        drive_version_uuid: drive_version_uuid.to_string(),
        source_path: "guide/getting-started.md".to_string(),
        file_kind: WikiSourceFileKind::Page,
        media_type: "text/markdown".to_string(),
        size_bytes: 512,
        content_sha256: content_sha256.to_string(),
        source_sequence_no: sequence_no,
        source_event_id: format!("source-event-{sequence_no}"),
        actor_id: 9001,
    }
}

fn drive_event(
    site_publication_id: u64,
    checkpoint_id: u64,
    sequence_no: u64,
    source_event_id: &str,
) -> ReceiveWikiDriveEventRequest {
    let payload_json = format!("{{\"sequenceNo\":{sequence_no}}}");
    ReceiveWikiDriveEventRequest {
        scope: SCOPE,
        site_publication_id,
        checkpoint_id,
        source_event_id: source_event_id.to_string(),
        event_type: WikiDriveEventType::VersionCommitted,
        sequence_no,
        drive_node_uuid: "drive-node-guide".to_string(),
        drive_version_uuid: Some(format!("drive-version-{sequence_no}")),
        payload_sha256: payload_hash(&payload_json),
        payload_json,
        source_event_time: format!("2026-07-21T00:00:0{sequence_no}Z"),
    }
}

fn payload_hash(payload_json: &str) -> String {
    format!("sha256:{}", sha256_hash(payload_json.as_bytes()))
}

fn claim_events_request(checkpoint_id: u64) -> ClaimWikiDriveEventsRequest {
    ClaimWikiDriveEventsRequest {
        scope: SCOPE,
        checkpoint_id,
        claim_owner: "wiki-event-worker-1".to_string(),
        lease_seconds: 60,
        after_id: None,
        limit: 10,
    }
}

struct TestIdGenerator {
    next: AtomicU64,
}

impl TestIdGenerator {
    fn new(first: u64) -> Self {
        Self {
            next: AtomicU64::new(first),
        }
    }
}

impl fmt::Debug for TestIdGenerator {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.debug_struct("TestIdGenerator").finish()
    }
}

impl KnowledgeIdGenerator for TestIdGenerator {
    fn next_id(&self) -> Result<u64, KnowledgeIdGeneratorError> {
        Ok(self.next.fetch_add(1, Ordering::Relaxed))
    }
}
