use std::sync::Mutex;

use async_trait::async_trait;
use sdkwork_intelligence_knowledgebase_service::{
    ports::{
        knowledge_wiki_drive_source::{
            EnsureKnowledgebaseRawScopeRequest, KnowledgeWikiDriveScope, KnowledgeWikiDriveSource,
            KnowledgeWikiDriveSourceError, KnowledgeWikiSourceResource, KnowledgebaseRawScope,
            KnowledgebaseRawScopeEventDelivery, ReadKnowledgeWikiSourceRequest,
            RenewKnowledgebaseRawScopeEventDeliveryRequest, ResolveKnowledgeWikiSourceRequest,
            ROOT_SCOPE_SUBSCRIPTION_TYPE,
        },
        knowledge_wiki_persistence::{
            AdvanceWikiReconciliationRequest, BindWikiSourceScopeRequest,
            ClaimWikiReconciliationRequest, ClaimWikiSourceProcessingRequest,
            CompleteWikiReconciliationRequest, CompleteWikiSourceProcessingRequest,
            ListWikiDriveCheckpointsRequest, ListWikiSourceProjectionsRequest,
            MarkWikiPublicationReadyRequest, ProvisionWikiDriveCheckpointRequest,
            ProvisionWikiPublicationRequest, RetryWikiSourceProcessingRequest,
            UpsertWikiSourceProjectionRequest, WikiDriveCheckpoint, WikiDriveCheckpointPage,
            WikiDriveCheckpointStore, WikiDriveStreamState, WikiIndexState,
            WikiPagePublicationState, WikiPersistenceError, WikiPersistenceScope,
            WikiPublication, WikiPublicationMode, WikiPublicationProvisioningResult,
            WikiPublicationStatus, WikiPublicationStore, WikiSourceFileKind,
            WikiSourceProjection, WikiSourceProjectionPage, WikiSourceProjectionStore,
            WikiSourceProjectionUpsertResult, WikiSourceState, WikiUpdatePolicy, WikiVisibility,
        },
        knowledge_wiki_publication_lifecycle::{
            ChangeWikiPageVisibilityRequest, ChangeWikiPublicationStatusRequest,
            PublishWikiPageRequest, UnpublishWikiPageRequest, WikiLifecycleDisposition,
            WikiPageLifecycleResult, WikiPublicationLifecycleResult, WikiPublicationLifecycleStore,
        },
    },
    wiki_representation::{render_wiki_page, WIKI_HTML_MEDIA_TYPE},
    wiki_source_processor::{
        KnowledgeWikiSourceProcessorService, ProcessKnowledgeWikiSourceCheckpointPageRequest,
    },
};
use sdkwork_utils_rust::sha256_hash;

const SCOPE: WikiPersistenceScope = WikiPersistenceScope {
    tenant_id: 101,
    organization_id: 202,
};
const PUBLICATION_ID: u64 = 501;
const SPACE_ID: u64 = 601;
const PROJECTION_ID: u64 = 701;
const SOURCE_SCOPE_UUID: &str = "11111111-1111-4111-8111-111111111801";
const DRIVE_SPACE_UUID: &str = "11111111-1111-4111-8111-111111111802";
const DRIVE_NODE_UUID: &str = "11111111-1111-4111-8111-111111111803";
const DRIVE_VERSION_UUID: &str = "11111111-1111-4111-8111-111111111804";

#[tokio::test]
async fn markdown_source_becomes_ready_and_auto_published() {
    let bytes = b"# Start\n\n<script>alert(1)</script>\n";
    let projection = projection(
        "guide/start.md",
        WikiSourceFileKind::Page,
        "text/markdown",
        bytes,
    );
    let persistence = FakePersistence::new(
        publication(
            WikiPublicationMode::AutoPublicAfterChecks,
            WikiVisibility::Public,
        ),
        projection.clone(),
    );
    let drive = FakeDriveSource::success(&projection, bytes);

    let result = processor(&persistence, &drive)
        .process_checkpoint_page(process_request(3))
        .await
        .expect("process Markdown source");

    assert_eq!(result.checkpoints_processed, 1);
    assert_eq!(result.sources_claimed, 1);
    assert_eq!(result.sources_ready, 1);
    assert_eq!(result.sources_auto_published, 1);
    assert_eq!(result.sources_retried, 0);
    assert_eq!(result.sources_quarantined, 0);
    let snapshot = persistence.snapshot();
    assert_eq!(snapshot.projection.source_state, WikiSourceState::Ready);
    assert_eq!(
        snapshot.projection.publication_state,
        WikiPagePublicationState::Published
    );
    assert_eq!(
        snapshot.projection.canonical_route.as_deref(),
        Some("/guide/start/")
    );
    assert_eq!(snapshot.projection.index_state, WikiIndexState::Ready);
    assert_eq!(snapshot.projection.visibility, WikiVisibility::Public);
    assert_eq!(
        snapshot.projection.public_drive_version_uuid.as_deref(),
        Some(DRIVE_VERSION_UUID)
    );
    assert_eq!(snapshot.publish_requests.len(), 1);
    assert_eq!(snapshot.completed_routes, ["/guide/start/"]);
    assert_eq!(drive.resolve_count(), 1);
    assert_eq!(drive.read_count(), 1);

    let rendered = render_wiki_page("guide/start.md", WikiSourceFileKind::Page, bytes)
        .expect("render Markdown")
        .expect("page representation");
    assert_eq!(rendered.media_type, WIKI_HTML_MEDIA_TYPE);
    assert_eq!(
        rendered.content_sha256,
        format!("sha256:{}", sha256_hash(&rendered.bytes))
    );
    let html = String::from_utf8(rendered.bytes).expect("rendered HTML");
    assert!(html.contains("<h1>Start</h1>"));
    assert!(!html.to_ascii_lowercase().contains("<script"));
}

#[tokio::test]
async fn review_and_private_modes_remain_ready_without_publication() {
    for (mode, visibility) in [
        (WikiPublicationMode::ReviewRequired, WikiVisibility::Public),
        (
            WikiPublicationMode::AutoPublicAfterChecks,
            WikiVisibility::Private,
        ),
    ] {
        let bytes = b"# Review";
        let projection = projection(
            "review.md",
            WikiSourceFileKind::Page,
            "text/markdown",
            bytes,
        );
        let persistence = FakePersistence::new(publication(mode, visibility), projection.clone());
        let drive = FakeDriveSource::success(&projection, bytes);

        let result = processor(&persistence, &drive)
            .process_checkpoint_page(process_request(3))
            .await
            .expect("process source requiring explicit publication");

        assert_eq!(result.sources_ready, 1);
        assert_eq!(result.sources_auto_published, 0);
        let snapshot = persistence.snapshot();
        assert_eq!(snapshot.projection.source_state, WikiSourceState::Ready);
        assert_eq!(
            snapshot.projection.publication_state,
            WikiPagePublicationState::Draft
        );
        assert!(snapshot.projection.public_drive_version_uuid.is_none());
        assert!(snapshot.publish_requests.is_empty());
    }
}

#[tokio::test]
async fn active_content_is_quarantined_before_drive_resolution() {
    for (path, media_type) in [
        ("app.js", "application/javascript"),
        ("icon.svg", "image/svg+xml"),
        ("module.wasm", "application/wasm"),
    ] {
        let bytes = b"active-content";
        let projection = projection(path, WikiSourceFileKind::Asset, media_type, bytes);
        let persistence = FakePersistence::new(
            publication(WikiPublicationMode::ReviewRequired, WikiVisibility::Private),
            projection.clone(),
        );
        let drive = FakeDriveSource::success(&projection, bytes);

        let result = processor(&persistence, &drive)
            .process_checkpoint_page(process_request(1))
            .await
            .expect("quarantine active content");

        assert_eq!(result.sources_claimed, 1);
        assert_eq!(result.sources_ready, 0);
        assert_eq!(result.sources_quarantined, 1);
        let snapshot = persistence.snapshot();
        assert_eq!(
            snapshot.projection.source_state,
            WikiSourceState::Quarantined
        );
        assert_eq!(
            snapshot.retry_error_codes,
            ["wiki_source_processing_unsupported"]
        );
        assert_eq!(drive.resolve_count(), 0);
        assert_eq!(drive.read_count(), 0);
    }
}

#[tokio::test]
async fn transient_drive_failure_retries_then_quarantines_at_the_bound() {
    let bytes = b"# Retry";
    let projection = projection("retry.md", WikiSourceFileKind::Page, "text/markdown", bytes);
    let persistence = FakePersistence::new(
        publication(WikiPublicationMode::ReviewRequired, WikiVisibility::Private),
        projection.clone(),
    );
    let drive = FakeDriveSource::failure(
        &projection,
        bytes,
        KnowledgeWikiDriveSourceError::Upstream("temporary Drive failure".to_string()),
    );

    let first = processor(&persistence, &drive)
        .process_checkpoint_page(process_request(2))
        .await
        .expect("schedule bounded retry");
    assert_eq!(first.sources_retried, 1);
    assert_eq!(first.sources_quarantined, 0);
    assert_eq!(
        persistence.snapshot().projection.source_state,
        WikiSourceState::Error
    );

    let second = processor(&persistence, &drive)
        .process_checkpoint_page(process_request(2))
        .await
        .expect("quarantine after maximum attempts");
    assert_eq!(second.sources_retried, 0);
    assert_eq!(second.sources_quarantined, 1);
    let snapshot = persistence.snapshot();
    assert_eq!(
        snapshot.projection.source_state,
        WikiSourceState::Quarantined
    );
    assert_eq!(snapshot.projection.processing_attempt_count, 2);
    assert_eq!(
        snapshot.retry_error_codes,
        [
            "wiki_drive_source_upstream_failed",
            "wiki_drive_source_upstream_failed"
        ]
    );
}

#[tokio::test]
async fn deferred_auto_publication_is_reclaimed_without_a_source_change() {
    let bytes = b"# Deferred";
    let projection = projection(
        "deferred.md",
        WikiSourceFileKind::Page,
        "text/markdown",
        bytes,
    );
    let persistence = FakePersistence::new(
        publication(
            WikiPublicationMode::AutoPublicAfterChecks,
            WikiVisibility::Unlisted,
        ),
        projection.clone(),
    );
    persistence.set_publish_failure(true);
    let drive = FakeDriveSource::success(&projection, bytes);

    let first = processor(&persistence, &drive)
        .process_checkpoint_page(process_request(3))
        .await
        .expect("defer failed automatic publication");
    assert_eq!(first.sources_ready, 1);
    assert_eq!(first.sources_auto_published, 0);
    assert_eq!(first.auto_publications_deferred, 1);
    assert_eq!(
        persistence.snapshot().projection.source_state,
        WikiSourceState::Ready
    );

    persistence.set_publish_failure(false);
    let second = processor(&persistence, &drive)
        .process_checkpoint_page(process_request(3))
        .await
        .expect("reclaim deferred automatic publication");
    assert_eq!(second.sources_claimed, 1);
    assert_eq!(second.sources_auto_published, 1);
    assert_eq!(
        persistence.snapshot().projection.publication_state,
        WikiPagePublicationState::Published
    );
}

fn processor<'a>(
    persistence: &'a FakePersistence,
    drive: &'a FakeDriveSource,
) -> KnowledgeWikiSourceProcessorService<'a> {
    KnowledgeWikiSourceProcessorService::new(
        persistence,
        persistence,
        persistence,
        persistence,
        drive,
    )
}

fn process_request(max_attempts: u32) -> ProcessKnowledgeWikiSourceCheckpointPageRequest {
    ProcessKnowledgeWikiSourceCheckpointPageRequest {
        scope: SCOPE,
        after_checkpoint_id: None,
        worker_id: "wiki-source-test-worker".to_string(),
        actor_id: 9001,
        lease_seconds: 30,
        checkpoint_limit: 10,
        source_limit_per_checkpoint: 10,
        retry_delay_seconds: 1,
        max_attempts,
    }
}

#[derive(Clone)]
struct FakeState {
    publication: WikiPublication,
    checkpoint: WikiDriveCheckpoint,
    projection: WikiSourceProjection,
    completed_routes: Vec<String>,
    retry_error_codes: Vec<String>,
    publish_requests: Vec<PublishWikiPageRequest>,
    fail_publish: bool,
}

struct FakePersistence {
    state: Mutex<FakeState>,
}

impl FakePersistence {
    fn new(publication: WikiPublication, projection: WikiSourceProjection) -> Self {
        Self {
            state: Mutex::new(FakeState {
                checkpoint: checkpoint(),
                publication,
                projection,
                completed_routes: Vec::new(),
                retry_error_codes: Vec::new(),
                publish_requests: Vec::new(),
                fail_publish: false,
            }),
        }
    }

    fn snapshot(&self) -> FakeState {
        self.state.lock().expect("fake persistence lock").clone()
    }

    fn set_publish_failure(&self, fail: bool) {
        self.state
            .lock()
            .expect("fake persistence lock")
            .fail_publish = fail;
    }
}

#[async_trait]
impl WikiPublicationStore for FakePersistence {
    async fn provision_publication(
        &self,
        _request: ProvisionWikiPublicationRequest,
    ) -> Result<WikiPublicationProvisioningResult, WikiPersistenceError> {
        Err(unused("provision_publication"))
    }

    async fn get_publication(
        &self,
        scope: WikiPersistenceScope,
        site_publication_id: u64,
    ) -> Result<WikiPublication, WikiPersistenceError> {
        let state = self.state.lock().expect("fake persistence lock");
        if scope == state.publication.scope && site_publication_id == state.publication.id {
            Ok(state.publication.clone())
        } else {
            Err(WikiPersistenceError::NotFound {
                resource: "wiki_publication",
                id: site_publication_id,
            })
        }
    }

    async fn get_publication_for_space(
        &self,
        scope: WikiPersistenceScope,
        space_id: u64,
    ) -> Result<Option<WikiPublication>, WikiPersistenceError> {
        let state = self.state.lock().expect("fake persistence lock");
        Ok(
            (scope == state.publication.scope && space_id == state.publication.space_id)
                .then(|| state.publication.clone()),
        )
    }

    async fn bind_source_scope(
        &self,
        _request: BindWikiSourceScopeRequest,
    ) -> Result<WikiPublication, WikiPersistenceError> {
        Err(unused("bind_source_scope"))
    }

    async fn mark_publication_ready(
        &self,
        _request: MarkWikiPublicationReadyRequest,
    ) -> Result<WikiPublication, WikiPersistenceError> {
        Err(unused("mark_publication_ready"))
    }
}

#[async_trait]
impl WikiSourceProjectionStore for FakePersistence {
    async fn upsert_source_projection(
        &self,
        _request: UpsertWikiSourceProjectionRequest,
    ) -> Result<WikiSourceProjectionUpsertResult, WikiPersistenceError> {
        Err(unused("upsert_source_projection"))
    }

    async fn get_source_projection_by_node(
        &self,
        scope: WikiPersistenceScope,
        site_publication_id: u64,
        drive_node_uuid: &str,
    ) -> Result<Option<WikiSourceProjection>, WikiPersistenceError> {
        let state = self.state.lock().expect("fake persistence lock");
        Ok((scope == state.projection.scope
            && site_publication_id == state.projection.site_publication_id
            && drive_node_uuid == state.projection.drive_node_uuid)
            .then(|| state.projection.clone()))
    }

    async fn list_source_projections(
        &self,
        request: ListWikiSourceProjectionsRequest,
    ) -> Result<WikiSourceProjectionPage, WikiPersistenceError> {
        let state = self.state.lock().expect("fake persistence lock");
        let matches = request.scope == state.projection.scope
            && request.site_publication_id == state.projection.site_publication_id
            && request
                .after_projection_id
                .is_none_or(|after| state.projection.id > after);
        let projections = matches
            .then(|| vec![state.projection.clone()])
            .unwrap_or_default();
        Ok(WikiSourceProjectionPage {
            projections,
            next_after_projection_id: None,
        })
    }

    async fn claim_source_processing(
        &self,
        request: ClaimWikiSourceProcessingRequest,
    ) -> Result<Vec<WikiSourceProjection>, WikiPersistenceError> {
        let mut state = self.state.lock().expect("fake persistence lock");
        let auto_publish_ready = state.projection.source_state == WikiSourceState::Ready
            && state.publication.publication_mode == WikiPublicationMode::AutoPublicAfterChecks
            && state.publication.default_visibility != WikiVisibility::Private
            && (state.projection.publication_state != WikiPagePublicationState::Published
                || state.projection.public_drive_version_uuid.as_deref()
                    != Some(state.projection.drive_version_uuid.as_str()));
        let processable = matches!(
            state.projection.source_state,
            WikiSourceState::Discovered | WikiSourceState::Queued | WikiSourceState::Error
        ) || auto_publish_ready;
        if request.scope != state.projection.scope
            || request.site_publication_id != state.projection.site_publication_id
            || !processable
            || request.after_id.is_some_and(|id| state.projection.id <= id)
        {
            return Ok(Vec::new());
        }
        state.projection.source_state = WikiSourceState::Processing;
        state.projection.processing_attempt_count += 1;
        state.projection.processing_fence += 1;
        state.projection.processing_lease_token = Some(format!(
            "{}:{}",
            request.claim_owner, state.projection.processing_fence
        ));
        state.projection.version += 1;
        Ok(vec![state.projection.clone()])
    }

    async fn complete_source_processing(
        &self,
        request: CompleteWikiSourceProcessingRequest,
    ) -> Result<WikiSourceProjection, WikiPersistenceError> {
        let mut state = self.state.lock().expect("fake persistence lock");
        if request.scope != state.projection.scope
            || request.site_publication_id != state.projection.site_publication_id
            || request.projection_id != state.projection.id
            || request.processing_fence != state.projection.processing_fence
            || state.projection.processing_lease_token.as_deref() != Some(&request.lease_token)
            || state.projection.source_state != WikiSourceState::Processing
        {
            return Err(WikiPersistenceError::Conflict(
                "source processing lease is stale".to_string(),
            ));
        }
        state.projection.source_state = WikiSourceState::Ready;
        state.projection.canonical_route = Some(request.canonical_route.clone());
        state.projection.index_state = request.index_state;
        state.projection.processing_lease_token = None;
        state.projection.version += 1;
        state.completed_routes.push(request.canonical_route);
        Ok(state.projection.clone())
    }

    async fn retry_source_processing(
        &self,
        request: RetryWikiSourceProcessingRequest,
    ) -> Result<WikiSourceProjection, WikiPersistenceError> {
        let mut state = self.state.lock().expect("fake persistence lock");
        if request.scope != state.projection.scope
            || request.projection_id != state.projection.id
            || request.processing_fence != state.projection.processing_fence
            || state.projection.processing_lease_token.as_deref() != Some(&request.lease_token)
            || state.projection.source_state != WikiSourceState::Processing
        {
            return Err(WikiPersistenceError::Conflict(
                "source processing lease is stale".to_string(),
            ));
        }
        state.projection.source_state =
            if state.projection.processing_attempt_count >= request.max_attempts {
                WikiSourceState::Quarantined
            } else {
                WikiSourceState::Error
            };
        state.projection.processing_lease_token = None;
        state.projection.version += 1;
        state.retry_error_codes.push(request.error_code);
        Ok(state.projection.clone())
    }
}

#[async_trait]
impl WikiDriveCheckpointStore for FakePersistence {
    async fn provision_checkpoint(
        &self,
        _request: ProvisionWikiDriveCheckpointRequest,
    ) -> Result<WikiDriveCheckpoint, WikiPersistenceError> {
        Err(unused("provision_checkpoint"))
    }

    async fn get_checkpoint(
        &self,
        scope: WikiPersistenceScope,
        checkpoint_id: u64,
    ) -> Result<WikiDriveCheckpoint, WikiPersistenceError> {
        let state = self.state.lock().expect("fake persistence lock");
        if scope == state.checkpoint.scope && checkpoint_id == state.checkpoint.id {
            Ok(state.checkpoint.clone())
        } else {
            Err(WikiPersistenceError::NotFound {
                resource: "wiki_drive_checkpoint",
                id: checkpoint_id,
            })
        }
    }

    async fn find_checkpoint_by_drive_scope(
        &self,
        scope: WikiPersistenceScope,
        drive_space_uuid: &str,
        source_scope_uuid: &str,
    ) -> Result<Option<WikiDriveCheckpoint>, WikiPersistenceError> {
        let state = self.state.lock().expect("fake persistence lock");
        Ok((scope == state.checkpoint.scope
            && drive_space_uuid == state.checkpoint.drive_space_uuid
            && source_scope_uuid == state.checkpoint.source_scope_uuid)
            .then(|| state.checkpoint.clone()))
    }

    async fn list_checkpoints(
        &self,
        request: ListWikiDriveCheckpointsRequest,
    ) -> Result<WikiDriveCheckpointPage, WikiPersistenceError> {
        let state = self.state.lock().expect("fake persistence lock");
        let checkpoints = (request.scope == state.checkpoint.scope
            && request
                .after_checkpoint_id
                .is_none_or(|id| state.checkpoint.id > id))
        .then(|| state.checkpoint.clone())
        .into_iter()
        .take(request.limit as usize)
        .collect();
        Ok(WikiDriveCheckpointPage {
            checkpoints,
            next_after_checkpoint_id: None,
        })
    }

    async fn claim_reconciliation(
        &self,
        _request: ClaimWikiReconciliationRequest,
    ) -> Result<WikiDriveCheckpoint, WikiPersistenceError> {
        Err(unused("claim_reconciliation"))
    }

    async fn advance_reconciliation(
        &self,
        _request: AdvanceWikiReconciliationRequest,
    ) -> Result<WikiDriveCheckpoint, WikiPersistenceError> {
        Err(unused("advance_reconciliation"))
    }

    async fn complete_reconciliation(
        &self,
        _request: CompleteWikiReconciliationRequest,
    ) -> Result<WikiDriveCheckpoint, WikiPersistenceError> {
        Err(unused("complete_reconciliation"))
    }
}

#[async_trait]
impl WikiPublicationLifecycleStore for FakePersistence {
    async fn change_publication_status(
        &self,
        _request: ChangeWikiPublicationStatusRequest,
    ) -> Result<WikiPublicationLifecycleResult, WikiPersistenceError> {
        Err(unused("change_publication_status"))
    }

    async fn publish_page(
        &self,
        request: PublishWikiPageRequest,
    ) -> Result<WikiPageLifecycleResult, WikiPersistenceError> {
        let mut state = self.state.lock().expect("fake persistence lock");
        state.publish_requests.push(request.clone());
        if state.fail_publish {
            return Err(WikiPersistenceError::Conflict(
                "simulated optimistic publication conflict".to_string(),
            ));
        }
        if request.scope != state.publication.scope
            || request.space_id != state.publication.space_id
            || request.source_file_uuid != state.projection.uuid
            || request.expected_publication_version != state.publication.version
            || request.expected_page_version != state.projection.version
            || state.projection.source_state != WikiSourceState::Ready
        {
            return Err(WikiPersistenceError::Conflict(
                "automatic publication version is stale".to_string(),
            ));
        }
        state.projection.publication_state = WikiPagePublicationState::Published;
        state.projection.visibility = request.visibility;
        state.projection.public_drive_version_uuid =
            Some(state.projection.drive_version_uuid.clone());
        state.projection.page_public_version += 1;
        state.projection.version += 1;
        state.publication.provider_generation += 1;
        state.publication.version += 1;
        Ok(WikiPageLifecycleResult {
            publication: state.publication.clone(),
            page: state.projection.clone(),
            disposition: WikiLifecycleDisposition::Changed,
        })
    }

    async fn unpublish_page(
        &self,
        _request: UnpublishWikiPageRequest,
    ) -> Result<WikiPageLifecycleResult, WikiPersistenceError> {
        Err(unused("unpublish_page"))
    }

    async fn change_page_visibility(
        &self,
        _request: ChangeWikiPageVisibilityRequest,
    ) -> Result<WikiPageLifecycleResult, WikiPersistenceError> {
        Err(unused("change_page_visibility"))
    }
}

struct FakeDriveSource {
    resource: KnowledgeWikiSourceResource,
    bytes: Vec<u8>,
    failure: Option<KnowledgeWikiDriveSourceError>,
    resolve_requests: Mutex<Vec<ResolveKnowledgeWikiSourceRequest>>,
    read_requests: Mutex<Vec<ReadKnowledgeWikiSourceRequest>>,
}

impl FakeDriveSource {
    fn success(projection: &WikiSourceProjection, bytes: &[u8]) -> Self {
        Self::new(projection, bytes, None)
    }

    fn failure(
        projection: &WikiSourceProjection,
        bytes: &[u8],
        error: KnowledgeWikiDriveSourceError,
    ) -> Self {
        Self::new(projection, bytes, Some(error))
    }

    fn new(
        projection: &WikiSourceProjection,
        bytes: &[u8],
        failure: Option<KnowledgeWikiDriveSourceError>,
    ) -> Self {
        Self {
            resource: KnowledgeWikiSourceResource {
                scope_type: ROOT_SCOPE_SUBSCRIPTION_TYPE.to_string(),
                subscription_uuid: SOURCE_SCOPE_UUID.to_string(),
                scope_generation: "7".to_string(),
                normalized_relative_path: projection.source_path.clone(),
                resource_type: "FILE".to_string(),
                drive_node_id: projection.drive_node_uuid.clone(),
                drive_node_version_id: projection.drive_version_uuid.clone(),
                version_no: "3".to_string(),
                checksum_sha256_hex: projection.content_sha256.clone(),
                etag: format!("\"{}\"", projection.content_sha256),
                content_type: projection.media_type.clone(),
                content_length: projection.size_bytes,
                last_modified: "2026-07-23T00:00:00Z".to_string(),
                scope_status: "ACTIVE".to_string(),
                node_status: "ACTIVE".to_string(),
                eligibility: "ELIGIBLE".to_string(),
            },
            bytes: bytes.to_vec(),
            failure,
            resolve_requests: Mutex::new(Vec::new()),
            read_requests: Mutex::new(Vec::new()),
        }
    }

    fn resolve_count(&self) -> usize {
        self.resolve_requests
            .lock()
            .expect("resolve request lock")
            .len()
    }

    fn read_count(&self) -> usize {
        self.read_requests.lock().expect("read request lock").len()
    }
}

#[async_trait]
impl KnowledgeWikiDriveScope for FakeDriveSource {
    async fn ensure_raw_scope(
        &self,
        _request: EnsureKnowledgebaseRawScopeRequest,
    ) -> Result<KnowledgebaseRawScope, KnowledgeWikiDriveSourceError> {
        Err(KnowledgeWikiDriveSourceError::InvalidRequest(
            "not used by source processor tests".to_string(),
        ))
    }

    async fn retrieve_raw_scope(
        &self,
        _subscription_uuid: &str,
    ) -> Result<KnowledgebaseRawScope, KnowledgeWikiDriveSourceError> {
        Err(KnowledgeWikiDriveSourceError::InvalidRequest(
            "not used by source processor tests".to_string(),
        ))
    }

    async fn renew_raw_scope_event_delivery(
        &self,
        _request: RenewKnowledgebaseRawScopeEventDeliveryRequest,
    ) -> Result<KnowledgebaseRawScopeEventDelivery, KnowledgeWikiDriveSourceError> {
        Err(KnowledgeWikiDriveSourceError::InvalidRequest(
            "not used by source processor tests".to_string(),
        ))
    }
}

#[async_trait]
impl KnowledgeWikiDriveSource for FakeDriveSource {
    async fn resolve_source(
        &self,
        request: ResolveKnowledgeWikiSourceRequest,
    ) -> Result<KnowledgeWikiSourceResource, KnowledgeWikiDriveSourceError> {
        self.resolve_requests
            .lock()
            .expect("resolve request lock")
            .push(request);
        if let Some(error) = &self.failure {
            return Err(error.clone());
        }
        Ok(self.resource.clone())
    }

    async fn read_pinned_source(
        &self,
        request: ReadKnowledgeWikiSourceRequest,
    ) -> Result<Vec<u8>, KnowledgeWikiDriveSourceError> {
        self.read_requests
            .lock()
            .expect("read request lock")
            .push(request);
        if let Some(error) = &self.failure {
            return Err(error.clone());
        }
        Ok(self.bytes.clone())
    }
}

fn publication(mode: WikiPublicationMode, visibility: WikiVisibility) -> WikiPublication {
    WikiPublication {
        id: PUBLICATION_ID,
        uuid: "11111111-1111-4111-8111-111111111805".to_string(),
        scope: SCOPE,
        space_id: SPACE_ID,
        drive_space_uuid: DRIVE_SPACE_UUID.to_string(),
        source_root_node_uuid: Some("11111111-1111-4111-8111-111111111806".to_string()),
        source_scope_uuid: Some(SOURCE_SCOPE_UUID.to_string()),
        wiki_status: WikiPublicationStatus::Active,
        title: "Source Processor Wiki".to_string(),
        homepage_source_path: "index.md".to_string(),
        publication_mode: mode,
        default_visibility: visibility,
        update_policy: WikiUpdatePolicy::KeepLastPublicUntilReady,
        provider_generation: 1,
        navigation_generation: 1,
        search_generation: 1,
        last_projected_drive_checkpoint: 1,
        version: 3,
    }
}

fn projection(
    path: &str,
    kind: WikiSourceFileKind,
    media_type: &str,
    bytes: &[u8],
) -> WikiSourceProjection {
    WikiSourceProjection {
        id: PROJECTION_ID,
        uuid: "11111111-1111-4111-8111-111111111807".to_string(),
        scope: SCOPE,
        site_publication_id: PUBLICATION_ID,
        space_id: SPACE_ID,
        drive_space_uuid: DRIVE_SPACE_UUID.to_string(),
        drive_node_uuid: DRIVE_NODE_UUID.to_string(),
        drive_version_uuid: DRIVE_VERSION_UUID.to_string(),
        source_path: path.to_string(),
        canonical_route: None,
        file_kind: kind,
        media_type: media_type.to_string(),
        size_bytes: bytes.len() as u64,
        content_sha256: format!("sha256:{}", sha256_hash(bytes)),
        source_state: WikiSourceState::Discovered,
        publication_state: WikiPagePublicationState::Draft,
        visibility: WikiVisibility::Private,
        index_state: WikiIndexState::Pending,
        public_drive_version_uuid: None,
        page_public_version: 0,
        source_sequence_no: 1,
        last_source_event_id: Some("drive-event-1".to_string()),
        processing_attempt_count: 0,
        processing_lease_token: None,
        processing_fence: 0,
        version: 1,
    }
}

fn checkpoint() -> WikiDriveCheckpoint {
    WikiDriveCheckpoint {
        id: 901,
        uuid: "11111111-1111-4111-8111-111111111808".to_string(),
        scope: SCOPE,
        site_publication_id: PUBLICATION_ID,
        drive_space_uuid: DRIVE_SPACE_UUID.to_string(),
        source_scope_uuid: SOURCE_SCOPE_UUID.to_string(),
        last_sequence_no: 1,
        last_event_id: Some("drive-event-1".to_string()),
        stream_state: WikiDriveStreamState::Healthy,
        gap_from_sequence_no: None,
        gap_to_sequence_no: None,
        reconciliation_cursor: None,
        lease_token: None,
        fence_token: 0,
        version: 1,
    }
}

fn unused(operation: &str) -> WikiPersistenceError {
    WikiPersistenceError::InvalidRequest(format!(
        "{operation} is not used by Wiki source processor tests"
    ))
}
