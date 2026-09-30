use std::str::FromStr;

use async_trait::async_trait;
use thiserror::Error;

macro_rules! database_enum {
    ($name:ident { $($variant:ident => $value:literal),+ $(,)? }) => {
        #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
        pub enum $name {
            $($variant),+
        }

        impl $name {
            pub const fn as_str(self) -> &'static str {
                match self {
                    $(Self::$variant => $value),+
                }
            }
        }

        impl FromStr for $name {
            type Err = ();

            fn from_str(value: &str) -> Result<Self, Self::Err> {
                match value {
                    $($value => Ok(Self::$variant),)+
                    _ => Err(()),
                }
            }
        }
    };
}

database_enum!(WikiPublicationStatus {
    Draft => "DRAFT",
    Validating => "VALIDATING",
    Ready => "READY",
    Active => "ACTIVE",
    Degraded => "DEGRADED",
    Paused => "PAUSED",
    Archived => "ARCHIVED",
    Failed => "FAILED",
});

database_enum!(WikiPublicationMode {
    ReviewRequired => "REVIEW_REQUIRED",
    AutoPublicAfterChecks => "AUTO_PUBLIC_AFTER_CHECKS",
});

database_enum!(WikiVisibility {
    Private => "PRIVATE",
    Unlisted => "UNLISTED",
    Public => "PUBLIC",
});

database_enum!(WikiUpdatePolicy {
    KeepLastPublicUntilReady => "KEEP_LAST_PUBLIC_UNTIL_READY",
    UnpublishDuringProcessing => "UNPUBLISH_DURING_PROCESSING",
});

database_enum!(WikiSourceFileKind {
    Page => "PAGE",
    Document => "DOCUMENT",
    Presentation => "PRESENTATION",
    Spreadsheet => "SPREADSHEET",
    Code => "CODE",
    Media => "MEDIA",
    Asset => "ASSET",
    Archive => "ARCHIVE",
});

database_enum!(WikiSourceState {
    Discovered => "DISCOVERED",
    Queued => "QUEUED",
    Processing => "PROCESSING",
    Ready => "READY",
    Error => "ERROR",
    Quarantined => "QUARANTINED",
    Deleted => "DELETED",
});

database_enum!(WikiPagePublicationState {
    Draft => "DRAFT",
    InReview => "IN_REVIEW",
    Scheduled => "SCHEDULED",
    Published => "PUBLISHED",
    Unpublished => "UNPUBLISHED",
    Archived => "ARCHIVED",
});

database_enum!(WikiIndexState {
    NotRequired => "NOT_REQUIRED",
    Pending => "PENDING",
    Indexing => "INDEXING",
    Ready => "READY",
    Error => "ERROR",
});

database_enum!(WikiRenditionKind {
    SanitizedHtml => "SANITIZED_HTML",
    Pdf => "PDF",
    PageImage => "PAGE_IMAGE",
    Thumbnail => "THUMBNAIL",
    Poster => "POSTER",
    PlainText => "PLAIN_TEXT",
    SlideText => "SLIDE_TEXT",
    SheetPreview => "SHEET_PREVIEW",
    ArchiveManifest => "ARCHIVE_MANIFEST",
    MediaMetadata => "MEDIA_METADATA",
});

database_enum!(WikiRenditionState {
    Pending => "PENDING",
    Processing => "PROCESSING",
    Ready => "READY",
    Error => "ERROR",
    Quarantined => "QUARANTINED",
    Expired => "EXPIRED",
});

database_enum!(WikiDriveStreamState {
    Healthy => "HEALTHY",
    GapDetected => "GAP_DETECTED",
    Reconciling => "RECONCILING",
    Paused => "PAUSED",
    Failed => "FAILED",
});

database_enum!(WikiDriveEventType {
    VersionCommitted => "drive.node.version.committed.v1",
    PathChanged => "drive.node.path.changed.v1",
    EligibilityChanged => "drive.node.eligibility.changed.v1",
    Deleted => "drive.node.deleted.v1",
});

database_enum!(WikiDriveEventProcessingState {
    Received => "RECEIVED",
    Deferred => "DEFERRED",
    Applied => "APPLIED",
    Retry => "RETRY",
    DeadLetter => "DEAD_LETTER",
    Ignored => "IGNORED",
});

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct WikiPersistenceScope {
    pub tenant_id: u64,
    pub organization_id: u64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WikiPublication {
    pub id: u64,
    pub uuid: String,
    pub scope: WikiPersistenceScope,
    pub space_id: u64,
    pub drive_space_uuid: String,
    pub source_root_node_uuid: Option<String>,
    pub source_scope_uuid: Option<String>,
    pub wiki_status: WikiPublicationStatus,
    pub title: String,
    pub homepage_source_path: String,
    pub publication_mode: WikiPublicationMode,
    pub default_visibility: WikiVisibility,
    pub update_policy: WikiUpdatePolicy,
    pub provider_generation: u64,
    pub navigation_generation: u64,
    pub search_generation: u64,
    pub last_projected_drive_checkpoint: u64,
    pub version: u64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WikiSourceProjection {
    pub id: u64,
    pub uuid: String,
    pub scope: WikiPersistenceScope,
    pub site_publication_id: u64,
    pub space_id: u64,
    pub drive_space_uuid: String,
    pub drive_node_uuid: String,
    pub drive_version_uuid: String,
    pub source_path: String,
    pub canonical_route: Option<String>,
    pub file_kind: WikiSourceFileKind,
    pub media_type: String,
    pub size_bytes: u64,
    pub content_sha256: String,
    pub source_state: WikiSourceState,
    pub publication_state: WikiPagePublicationState,
    pub visibility: WikiVisibility,
    pub index_state: WikiIndexState,
    pub public_drive_version_uuid: Option<String>,
    pub page_public_version: u64,
    pub source_sequence_no: u64,
    pub last_source_event_id: Option<String>,
    pub processing_attempt_count: u32,
    pub processing_lease_token: Option<String>,
    pub processing_fence: u64,
    pub version: u64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WikiSourceRendition {
    pub id: u64,
    pub uuid: String,
    pub scope: WikiPersistenceScope,
    pub site_publication_id: u64,
    pub source_file_projection_id: u64,
    pub drive_version_uuid: String,
    pub source_content_sha256: String,
    pub processor_id: String,
    pub processor_version: String,
    pub policy_version: String,
    pub rendition_kind: WikiRenditionKind,
    pub rendition_key_sha256: String,
    pub rendition_state: WikiRenditionState,
    pub rendition_drive_space_uuid: Option<String>,
    pub rendition_drive_node_uuid: Option<String>,
    pub rendition_drive_version_uuid: Option<String>,
    pub content_sha256: Option<String>,
    pub media_type: Option<String>,
    pub size_bytes: Option<u64>,
    pub processing_attempt_count: u32,
    pub processing_lease_token: Option<String>,
    pub processing_fence: u64,
    pub version: u64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WikiDriveCheckpoint {
    pub id: u64,
    pub uuid: String,
    pub scope: WikiPersistenceScope,
    pub site_publication_id: u64,
    pub drive_space_uuid: String,
    pub source_scope_uuid: String,
    pub last_sequence_no: u64,
    pub last_event_id: Option<String>,
    pub stream_state: WikiDriveStreamState,
    pub gap_from_sequence_no: Option<u64>,
    pub gap_to_sequence_no: Option<u64>,
    pub reconciliation_cursor: Option<String>,
    pub lease_token: Option<String>,
    pub fence_token: u64,
    pub version: u64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ListWikiDriveCheckpointsRequest {
    pub scope: WikiPersistenceScope,
    pub after_checkpoint_id: Option<u64>,
    pub limit: u32,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WikiDriveCheckpointPage {
    pub checkpoints: Vec<WikiDriveCheckpoint>,
    pub next_after_checkpoint_id: Option<u64>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WikiDriveInboxEvent {
    pub id: u64,
    pub uuid: String,
    pub scope: WikiPersistenceScope,
    pub site_publication_id: u64,
    pub checkpoint_id: u64,
    pub source_event_id: String,
    pub event_type: WikiDriveEventType,
    pub sequence_no: u64,
    pub drive_node_uuid: String,
    pub drive_version_uuid: Option<String>,
    pub payload_sha256: String,
    pub payload_json: String,
    pub source_event_time: String,
    pub processing_state: WikiDriveEventProcessingState,
    pub attempt_count: u32,
    pub lease_token: Option<String>,
    pub version: u64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WikiPublicationBackfillCandidate {
    pub space_id: u64,
    pub knowledgebase_uuid: String,
    pub title: String,
    pub drive_space_uuid: String,
    pub publication_missing: bool,
    pub source_scope_missing: bool,
    pub checkpoint_missing: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ListWikiPublicationBackfillCandidatesRequest {
    pub scope: WikiPersistenceScope,
    pub after_space_id: Option<u64>,
    pub limit: u32,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WikiPublicationBackfillCandidatePage {
    pub candidates: Vec<WikiPublicationBackfillCandidate>,
    pub next_after_space_id: Option<u64>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProvisionWikiPublicationRequest {
    pub scope: WikiPersistenceScope,
    pub space_id: u64,
    pub drive_space_uuid: String,
    pub title: String,
    pub actor_id: u64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WikiPublicationProvisioningResult {
    pub publication: WikiPublication,
    pub created: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BindWikiSourceScopeRequest {
    pub scope: WikiPersistenceScope,
    pub site_publication_id: u64,
    pub source_root_node_uuid: String,
    pub source_scope_uuid: String,
    pub expected_version: u64,
    pub actor_id: u64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MarkWikiPublicationReadyRequest {
    pub scope: WikiPersistenceScope,
    pub site_publication_id: u64,
    pub expected_version: u64,
    pub actor_id: u64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UpsertWikiSourceProjectionRequest {
    pub scope: WikiPersistenceScope,
    pub site_publication_id: u64,
    pub space_id: u64,
    pub drive_space_uuid: String,
    pub drive_node_uuid: String,
    pub drive_version_uuid: String,
    pub source_path: String,
    pub file_kind: WikiSourceFileKind,
    pub media_type: String,
    pub size_bytes: u64,
    pub content_sha256: String,
    pub source_sequence_no: u64,
    pub source_event_id: String,
    pub actor_id: u64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WikiSourceProjectionUpsertDisposition {
    Created,
    Updated,
    UnchangedReplay,
    IgnoredStale,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WikiSourceProjectionUpsertResult {
    pub projection: WikiSourceProjection,
    pub disposition: WikiSourceProjectionUpsertDisposition,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ClaimWikiSourceProcessingRequest {
    pub scope: WikiPersistenceScope,
    pub site_publication_id: u64,
    pub claim_owner: String,
    pub lease_seconds: u64,
    pub after_id: Option<u64>,
    pub limit: u32,
}

/// Read-only, cursor-paginated listing of source-file projections for one
/// Wiki publication. Powers the app-api `wikiSourceFiles.list` inspection
/// surface; ordering is stable by projection row id.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ListWikiSourceProjectionsRequest {
    pub scope: WikiPersistenceScope,
    pub site_publication_id: u64,
    pub after_projection_id: Option<u64>,
    pub limit: u32,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WikiSourceProjectionPage {
    pub projections: Vec<WikiSourceProjection>,
    pub next_after_projection_id: Option<u64>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CompleteWikiSourceProcessingRequest {
    pub scope: WikiPersistenceScope,
    pub site_publication_id: u64,
    pub projection_id: u64,
    pub lease_token: String,
    pub processing_fence: u64,
    pub canonical_route: String,
    pub index_state: WikiIndexState,
    pub actor_id: u64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RetryWikiSourceProcessingRequest {
    pub scope: WikiPersistenceScope,
    pub projection_id: u64,
    pub lease_token: String,
    pub processing_fence: u64,
    pub error_code: String,
    pub error_summary: String,
    pub retry_delay_seconds: u64,
    pub max_attempts: u32,
    pub actor_id: u64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UpsertWikiRenditionRequest {
    pub scope: WikiPersistenceScope,
    pub site_publication_id: u64,
    pub source_file_projection_id: u64,
    pub drive_version_uuid: String,
    pub source_content_sha256: String,
    pub processor_id: String,
    pub processor_version: String,
    pub policy_version: String,
    pub rendition_kind: WikiRenditionKind,
    pub rendition_key_sha256: String,
    pub actor_id: u64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ClaimWikiRenditionsRequest {
    pub scope: WikiPersistenceScope,
    pub site_publication_id: u64,
    pub claim_owner: String,
    pub lease_seconds: u64,
    pub after_id: Option<u64>,
    pub limit: u32,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CompleteWikiRenditionRequest {
    pub scope: WikiPersistenceScope,
    pub rendition_id: u64,
    pub lease_token: String,
    pub processing_fence: u64,
    pub rendition_drive_space_uuid: String,
    pub rendition_drive_node_uuid: String,
    pub rendition_drive_version_uuid: String,
    pub content_sha256: String,
    pub media_type: String,
    pub size_bytes: u64,
    pub actor_id: u64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProvisionWikiDriveCheckpointRequest {
    pub scope: WikiPersistenceScope,
    pub site_publication_id: u64,
    pub drive_space_uuid: String,
    pub source_scope_uuid: String,
    pub actor_id: u64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ClaimWikiReconciliationRequest {
    pub scope: WikiPersistenceScope,
    pub checkpoint_id: u64,
    pub claim_owner: String,
    pub lease_seconds: u64,
    pub expected_version: u64,
    pub actor_id: u64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AdvanceWikiReconciliationRequest {
    pub scope: WikiPersistenceScope,
    pub checkpoint_id: u64,
    pub lease_token: String,
    pub fence_token: u64,
    pub reconciliation_cursor: String,
    pub actor_id: u64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CompleteWikiReconciliationRequest {
    pub scope: WikiPersistenceScope,
    pub checkpoint_id: u64,
    pub lease_token: String,
    pub fence_token: u64,
    pub reconciled_sequence_no: u64,
    pub last_event_id: Option<String>,
    pub actor_id: u64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ReceiveWikiDriveEventRequest {
    pub scope: WikiPersistenceScope,
    pub site_publication_id: u64,
    pub checkpoint_id: u64,
    pub source_event_id: String,
    pub event_type: WikiDriveEventType,
    pub sequence_no: u64,
    pub drive_node_uuid: String,
    pub drive_version_uuid: Option<String>,
    pub payload_sha256: String,
    pub payload_json: String,
    pub source_event_time: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WikiDriveEventReceiveDisposition {
    Ready,
    DeferredGap,
    Duplicate,
    IgnoredStale,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WikiDriveEventReceipt {
    pub event: WikiDriveInboxEvent,
    pub disposition: WikiDriveEventReceiveDisposition,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ClaimWikiDriveEventsRequest {
    pub scope: WikiPersistenceScope,
    pub checkpoint_id: u64,
    pub claim_owner: String,
    pub lease_seconds: u64,
    pub after_id: Option<u64>,
    pub limit: u32,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CompleteWikiDriveEventRequest {
    pub scope: WikiPersistenceScope,
    pub event_id: u64,
    pub lease_token: String,
    pub actor_id: u64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WikiDriveSourceMetadata {
    pub drive_version_uuid: String,
    pub source_path: String,
    pub file_kind: WikiSourceFileKind,
    pub media_type: String,
    pub size_bytes: u64,
    pub content_sha256: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum WikiDriveProjectionMutation {
    None,
    Upsert(WikiDriveSourceMetadata),
    MoveWithin {
        source_path: String,
    },
    MarkEligible,
    Revoke {
        source_state: WikiSourceState,
        publication_state: WikiPagePublicationState,
        reason_code: String,
    },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ApplyWikiDriveEventRequest {
    pub complete: CompleteWikiDriveEventRequest,
    pub mutation: WikiDriveProjectionMutation,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WikiPublicRouteChange {
    pub event_type: &'static str,
    pub route: Option<String>,
    pub page_public_version: u64,
    pub provider_generation: u64,
    pub reason_code: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WikiDriveEventApplicationResult {
    pub event: WikiDriveInboxEvent,
    pub projection: Option<WikiSourceProjection>,
    pub public_route_change: Option<WikiPublicRouteChange>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RetryWikiDriveEventRequest {
    pub scope: WikiPersistenceScope,
    pub event_id: u64,
    pub lease_token: String,
    pub error_code: String,
    pub error_summary: String,
    pub retry_delay_seconds: u64,
    pub max_attempts: u32,
}

#[async_trait]
pub trait WikiPublicationStore: Send + Sync {
    async fn provision_publication(
        &self,
        request: ProvisionWikiPublicationRequest,
    ) -> Result<WikiPublicationProvisioningResult, WikiPersistenceError>;

    async fn get_publication(
        &self,
        scope: WikiPersistenceScope,
        site_publication_id: u64,
    ) -> Result<WikiPublication, WikiPersistenceError>;

    async fn get_publication_for_space(
        &self,
        scope: WikiPersistenceScope,
        space_id: u64,
    ) -> Result<Option<WikiPublication>, WikiPersistenceError>;

    async fn bind_source_scope(
        &self,
        request: BindWikiSourceScopeRequest,
    ) -> Result<WikiPublication, WikiPersistenceError>;

    async fn mark_publication_ready(
        &self,
        request: MarkWikiPublicationReadyRequest,
    ) -> Result<WikiPublication, WikiPersistenceError>;
}

#[async_trait]
pub trait WikiSourceProjectionStore: Send + Sync {
    async fn upsert_source_projection(
        &self,
        request: UpsertWikiSourceProjectionRequest,
    ) -> Result<WikiSourceProjectionUpsertResult, WikiPersistenceError>;

    async fn get_source_projection_by_node(
        &self,
        scope: WikiPersistenceScope,
        site_publication_id: u64,
        drive_node_uuid: &str,
    ) -> Result<Option<WikiSourceProjection>, WikiPersistenceError>;

    async fn list_source_projections(
        &self,
        request: ListWikiSourceProjectionsRequest,
    ) -> Result<WikiSourceProjectionPage, WikiPersistenceError>;

    async fn claim_source_processing(
        &self,
        request: ClaimWikiSourceProcessingRequest,
    ) -> Result<Vec<WikiSourceProjection>, WikiPersistenceError>;

    async fn complete_source_processing(
        &self,
        request: CompleteWikiSourceProcessingRequest,
    ) -> Result<WikiSourceProjection, WikiPersistenceError>;

    async fn retry_source_processing(
        &self,
        request: RetryWikiSourceProcessingRequest,
    ) -> Result<WikiSourceProjection, WikiPersistenceError>;
}

#[async_trait]
pub trait WikiRenditionStore: Send + Sync {
    async fn upsert_rendition(
        &self,
        request: UpsertWikiRenditionRequest,
    ) -> Result<WikiSourceRendition, WikiPersistenceError>;

    async fn claim_renditions(
        &self,
        request: ClaimWikiRenditionsRequest,
    ) -> Result<Vec<WikiSourceRendition>, WikiPersistenceError>;

    async fn complete_rendition(
        &self,
        request: CompleteWikiRenditionRequest,
    ) -> Result<WikiSourceRendition, WikiPersistenceError>;
}

#[async_trait]
pub trait WikiDriveCheckpointStore: Send + Sync {
    async fn provision_checkpoint(
        &self,
        request: ProvisionWikiDriveCheckpointRequest,
    ) -> Result<WikiDriveCheckpoint, WikiPersistenceError>;

    async fn get_checkpoint(
        &self,
        scope: WikiPersistenceScope,
        checkpoint_id: u64,
    ) -> Result<WikiDriveCheckpoint, WikiPersistenceError>;

    async fn find_checkpoint_by_drive_scope(
        &self,
        scope: WikiPersistenceScope,
        drive_space_uuid: &str,
        source_scope_uuid: &str,
    ) -> Result<Option<WikiDriveCheckpoint>, WikiPersistenceError>;

    async fn list_checkpoints(
        &self,
        request: ListWikiDriveCheckpointsRequest,
    ) -> Result<WikiDriveCheckpointPage, WikiPersistenceError>;

    async fn claim_reconciliation(
        &self,
        request: ClaimWikiReconciliationRequest,
    ) -> Result<WikiDriveCheckpoint, WikiPersistenceError>;

    async fn advance_reconciliation(
        &self,
        request: AdvanceWikiReconciliationRequest,
    ) -> Result<WikiDriveCheckpoint, WikiPersistenceError>;

    async fn complete_reconciliation(
        &self,
        request: CompleteWikiReconciliationRequest,
    ) -> Result<WikiDriveCheckpoint, WikiPersistenceError>;
}

#[async_trait]
pub trait WikiDriveEventInboxStore: Send + Sync {
    async fn receive_event(
        &self,
        request: ReceiveWikiDriveEventRequest,
    ) -> Result<WikiDriveEventReceipt, WikiPersistenceError>;

    async fn claim_events(
        &self,
        request: ClaimWikiDriveEventsRequest,
    ) -> Result<Vec<WikiDriveInboxEvent>, WikiPersistenceError>;

    async fn complete_event(
        &self,
        request: CompleteWikiDriveEventRequest,
    ) -> Result<WikiDriveInboxEvent, WikiPersistenceError>;

    async fn apply_event(
        &self,
        request: ApplyWikiDriveEventRequest,
    ) -> Result<WikiDriveEventApplicationResult, WikiPersistenceError>;

    async fn retry_event(
        &self,
        request: RetryWikiDriveEventRequest,
    ) -> Result<WikiDriveInboxEvent, WikiPersistenceError>;
}

#[async_trait]
pub trait WikiPublicationBackfillStore: Send + Sync {
    async fn list_backfill_candidates(
        &self,
        request: ListWikiPublicationBackfillCandidatesRequest,
    ) -> Result<WikiPublicationBackfillCandidatePage, WikiPersistenceError>;
}

#[derive(Debug, Error, Clone, PartialEq, Eq)]
pub enum WikiPersistenceError {
    #[error("Wiki persistence invalid request: {0}")]
    InvalidRequest(String),
    #[error("Wiki persistence resource not found: {resource}={id}")]
    NotFound { resource: &'static str, id: u64 },
    #[error("Wiki persistence conflict: {0}")]
    Conflict(String),
    #[error("Wiki persistence stale version for {resource}={id}; expected {expected}")]
    StaleVersion {
        resource: &'static str,
        id: u64,
        expected: u64,
    },
    #[error("Wiki persistence internal error: {0}")]
    Internal(String),
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn database_enums_round_trip_canonical_values() {
        for (value, expected) in [
            ("ACTIVE", WikiPublicationStatus::Active),
            ("DEGRADED", WikiPublicationStatus::Degraded),
        ] {
            assert_eq!(value.parse::<WikiPublicationStatus>(), Ok(expected));
            assert_eq!(expected.as_str(), value);
        }
        assert!("active".parse::<WikiPublicationStatus>().is_err());
        assert_eq!(
            WikiDriveEventType::VersionCommitted.as_str(),
            "drive.node.version.committed.v1"
        );
    }
}
