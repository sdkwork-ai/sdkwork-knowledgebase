use async_trait::async_trait;
use sdkwork_intelligence_knowledgebase_service::imports::{
    KnowledgeDriveImportService, ResolvedKnowledgeDriveImportRequest,
};
use sdkwork_intelligence_knowledgebase_service::ports::drive_import_metadata_store::{
    DriveImportMetadataStore, DriveImportMetadataStoreError, PrepareDriveImportMetadataRecord,
    PreparedDriveImportMetadata,
};
use sdkwork_intelligence_knowledgebase_service::ports::knowledge_document_store::{
    CreateKnowledgeDocumentRecord, KnowledgeDocumentIdentityScope, KnowledgeDocumentStore,
    KnowledgeDocumentStoreError,
};
use sdkwork_intelligence_knowledgebase_service::ports::knowledge_document_version_store::{
    CreateKnowledgeDocumentVersionRecord, KnowledgeDocumentVersionStore,
    KnowledgeDocumentVersionStoreError,
};
use sdkwork_intelligence_knowledgebase_service::ports::knowledge_drive_object_ref_store::{
    CreateKnowledgeDriveObjectRefRecord, KnowledgeDriveObjectRefStore,
    KnowledgeDriveObjectRefStoreError,
};
use sdkwork_intelligence_knowledgebase_service::ports::knowledge_drive_storage::{
    HeadKnowledgeObjectRequest, KnowledgeDriveStorage, KnowledgeObjectRef, KnowledgeStorageError,
    PutKnowledgeObjectRequest,
};
use sdkwork_intelligence_knowledgebase_service::ports::knowledge_ingestion_job_store::{
    CreateIngestionJobRecord, CreateOrGetIngestionJobResult, DriveImportJobLinkage,
    IngestionJobStore, IngestionJobStoreError,
};
use sdkwork_intelligence_knowledgebase_service::ports::knowledge_source_store::{
    CreateKnowledgeSourceRecord, KnowledgeSourceStore, KnowledgeSourceStoreError,
};
use sdkwork_knowledgebase_contract::document::{
    KnowledgeDocument, KnowledgeDocumentState, KnowledgeDocumentVersion,
    KnowledgeDocumentVersionState, KnowledgeDocumentVisibility,
};
use sdkwork_knowledgebase_contract::ingest::{
    IngestionJob, IngestionJobState, KnowledgeDriveImportRequest,
};
use sdkwork_knowledgebase_contract::source::{KnowledgeSource, KnowledgeSourceType};
use sdkwork_knowledgebase_contract::KnowledgeDriveObjectRef;
use std::collections::HashMap;
use std::sync::{Arc, Mutex};

fn resolved_drive_import_request(
    title: &str,
    drive_node_id: &str,
    drive_object_key: &str,
    idempotency_key: &str,
) -> ResolvedKnowledgeDriveImportRequest {
    ResolvedKnowledgeDriveImportRequest {
        request: KnowledgeDriveImportRequest {
            space_id: 7,
            title: title.to_string(),
            drive_space_id: "drv-kb-001".to_string(),
            drive_node_id: drive_node_id.to_string(),
            idempotency_key: idempotency_key.to_string(),
            language: Some("en".to_string()),
        },
        drive_storage_provider_id: "provider-kb".to_string(),
        drive_bucket: "knowledgebase-source".to_string(),
        drive_object_key: drive_object_key.to_string(),
    }
}

#[tokio::test]
async fn drive_import_heads_drive_object_then_creates_source_document_version_and_job() {
    let drive = RecordingDrive::with_object(
        "knowledgebase-source",
        "incoming/quarterly-report.md",
        "# Report",
    );
    let sources = MemorySourceStore::default();
    let documents = MemoryDocumentStore::default();
    let object_refs = MemoryDriveObjectRefStore::default();
    let versions = MemoryDocumentVersionStore::default();
    let jobs = MemoryIngestionJobStore::default();
    let metadata =
        MemoryDriveImportMetadataStore::new(&sources, &documents, &object_refs, &versions, &jobs);
    let service = KnowledgeDriveImportService::new(&drive, &metadata);

    let result = service
        .import_drive_object(resolved_drive_import_request(
            "Quarterly Report",
            "node-quarterly-report",
            "incoming/quarterly-report.md",
            "drive-quarterly-report",
        ))
        .await
        .unwrap();

    assert_eq!(
        drive.heads(),
        vec![(
            "knowledgebase-source".to_string(),
            "incoming/quarterly-report.md".to_string()
        )]
    );
    assert_eq!(result.source.source_type, KnowledgeSourceType::DriveObject);
    assert_eq!(
        result.source.drive_bucket.as_deref(),
        Some("knowledgebase-source")
    );
    assert_eq!(
        result.source.drive_prefix.as_deref(),
        Some("incoming/quarterly-report.md")
    );
    assert_eq!(result.document.source_id, Some(result.source.id));
    assert_eq!(result.document.title, "Quarterly Report");
    assert_eq!(result.document.content_state, KnowledgeDocumentState::Ready);
    assert_eq!(
        result.document.original_file_drive_node_id.as_deref(),
        Some("node-quarterly-report")
    );
    assert_eq!(result.version.version_no, 1);
    assert_eq!(result.document.current_version_id, Some(result.version.id));
    assert_eq!(result.original_object_ref.object_role, "original_document");
    assert_eq!(
        result.original_object_ref.drive_provider_kind,
        "sdkwork-drive"
    );
    assert_eq!(
        result.original_object_ref.drive_bucket,
        "knowledgebase-source"
    );
    assert_eq!(
        result.original_object_ref.drive_object_key,
        "incoming/quarterly-report.md"
    );
    assert_eq!(
        result.original_object_ref.drive_object_version.as_deref(),
        Some("v1")
    );
    assert_eq!(
        result.original_object_ref.drive_etag.as_deref(),
        Some("etag")
    );
    assert_eq!(
        result.original_object_ref.size_bytes,
        "# Report".len() as u64
    );
    assert_eq!(
        result.version.original_object_ref_id,
        result.original_object_ref.id
    );
    assert_eq!(
        object_refs.created_refs(),
        vec![result.original_object_ref.clone()]
    );
    assert_eq!(result.job.source_type, "drive_object");
}

#[tokio::test]
async fn drive_import_replay_reuses_metadata_for_same_idempotency_key() {
    let drive = RecordingDrive::with_object(
        "knowledgebase-source",
        "incoming/quarterly-report.md",
        "# Report",
    );
    let sources = MemorySourceStore::default();
    let documents = MemoryDocumentStore::default();
    let object_refs = MemoryDriveObjectRefStore::default();
    let versions = MemoryDocumentVersionStore::default();
    let jobs = MemoryIngestionJobStore::default();
    let metadata =
        MemoryDriveImportMetadataStore::new(&sources, &documents, &object_refs, &versions, &jobs);
    let service = KnowledgeDriveImportService::new(&drive, &metadata);

    let request = resolved_drive_import_request(
        "Quarterly Report",
        "node-quarterly-report",
        "incoming/quarterly-report.md",
        "drive-quarterly-report",
    );

    let first = service.import_drive_object(request.clone()).await.unwrap();
    let replay = service.import_drive_object(request).await.unwrap();

    assert_eq!(first.job.id, replay.job.id);
    assert_eq!(first.source.id, replay.source.id);
    assert_eq!(first.document.id, replay.document.id);
    assert_eq!(first.version.id, replay.version.id);
    assert_eq!(first.original_object_ref.id, replay.original_object_ref.id);
    assert_eq!(sources.create_count(), 1);
    assert_eq!(documents.create_count(), 1);
    assert_eq!(versions.create_count(), 1);
    assert_eq!(object_refs.create_count(), 1);
}

#[tokio::test]
async fn drive_import_trims_idempotency_key_before_lookup() {
    let drive = RecordingDrive::with_object(
        "knowledgebase-source",
        "incoming/quarterly-report.md",
        "# Report",
    );
    let sources = MemorySourceStore::default();
    let documents = MemoryDocumentStore::default();
    let object_refs = MemoryDriveObjectRefStore::default();
    let versions = MemoryDocumentVersionStore::default();
    let jobs = MemoryIngestionJobStore::default();
    let metadata =
        MemoryDriveImportMetadataStore::new(&sources, &documents, &object_refs, &versions, &jobs);
    let service = KnowledgeDriveImportService::new(&drive, &metadata);

    let request = resolved_drive_import_request(
        "Quarterly Report",
        "node-quarterly-report",
        "incoming/quarterly-report.md",
        "drive-quarterly-report",
    );
    let mut replay_request = request.clone();
    replay_request.request.idempotency_key = " drive-quarterly-report ".to_string();

    let first = service.import_drive_object(request).await.unwrap();
    let replay = service.import_drive_object(replay_request).await.unwrap();

    assert_eq!(first.job.id, replay.job.id);
    assert_eq!(replay.job.idempotency_key, "drive-quarterly-report");
    assert_eq!(sources.create_count(), 1);
    assert_eq!(documents.create_count(), 1);
    assert_eq!(versions.create_count(), 1);
    assert_eq!(object_refs.create_count(), 1);
}

#[tokio::test]
async fn drive_import_rejects_same_idempotency_key_for_different_drive_object_before_side_effects()
{
    let drive = RecordingDrive::with_object(
        "knowledgebase-source",
        "incoming/quarterly-report.md",
        "# Report",
    );
    drive.add_object(
        "knowledgebase-source",
        "incoming/other-report.md",
        "# Other Report",
    );
    let sources = MemorySourceStore::default();
    let documents = MemoryDocumentStore::default();
    let object_refs = MemoryDriveObjectRefStore::default();
    let versions = MemoryDocumentVersionStore::default();
    let jobs = MemoryIngestionJobStore::default();
    let metadata =
        MemoryDriveImportMetadataStore::new(&sources, &documents, &object_refs, &versions, &jobs);
    let service = KnowledgeDriveImportService::new(&drive, &metadata);

    let first = service
        .import_drive_object(resolved_drive_import_request(
            "Quarterly Report",
            "node-quarterly-report",
            "incoming/quarterly-report.md",
            "drive-quarterly-report",
        ))
        .await
        .unwrap();

    let error = service
        .import_drive_object(resolved_drive_import_request(
            "Other Report",
            "node-other-report",
            "incoming/other-report.md",
            "drive-quarterly-report",
        ))
        .await
        .unwrap_err();

    assert!(error.to_string().contains("idempotency_key"));
    assert_eq!(first.job.id, 1);
    assert_eq!(
        drive.heads(),
        vec![(
            "knowledgebase-source".to_string(),
            "incoming/quarterly-report.md".to_string()
        )]
    );
    assert_eq!(sources.create_count(), 1);
    assert_eq!(documents.create_count(), 1);
    assert_eq!(versions.create_count(), 1);
    assert_eq!(object_refs.create_count(), 1);
    assert_eq!(jobs.create_count(), 1);
}

#[tokio::test]
async fn drive_import_rejects_unsafe_idempotency_key_before_side_effects() {
    let drive = RecordingDrive::with_object(
        "knowledgebase-source",
        "incoming/quarterly-report.md",
        "# Report",
    );
    let sources = MemorySourceStore::default();
    let documents = MemoryDocumentStore::default();
    let object_refs = MemoryDriveObjectRefStore::default();
    let versions = MemoryDocumentVersionStore::default();
    let jobs = MemoryIngestionJobStore::default();
    let metadata =
        MemoryDriveImportMetadataStore::new(&sources, &documents, &object_refs, &versions, &jobs);
    let service = KnowledgeDriveImportService::new(&drive, &metadata);

    let error = service
        .import_drive_object(resolved_drive_import_request(
            "Quarterly Report",
            "node-quarterly-report",
            "incoming/quarterly-report.md",
            "../escape",
        ))
        .await
        .unwrap_err();

    assert!(error.to_string().contains("idempotency_key"));
    assert!(drive.heads().is_empty());
    assert_eq!(sources.create_count(), 0);
    assert_eq!(documents.create_count(), 0);
    assert_eq!(versions.create_count(), 0);
    assert_eq!(object_refs.create_count(), 0);
    assert_eq!(jobs.create_count(), 0);
}

#[tokio::test]
async fn drive_import_rejects_provider_id_mismatch_before_import_writes() {
    let drive = RecordingDrive::with_object(
        "knowledgebase-source",
        "incoming/quarterly-report.md",
        "# Report",
    );
    drive.set_object_storage_provider_id("incoming/quarterly-report.md", "provider-other");
    let sources = MemorySourceStore::default();
    let documents = MemoryDocumentStore::default();
    let object_refs = MemoryDriveObjectRefStore::default();
    let versions = MemoryDocumentVersionStore::default();
    let jobs = MemoryIngestionJobStore::default();
    let metadata =
        MemoryDriveImportMetadataStore::new(&sources, &documents, &object_refs, &versions, &jobs);
    let service = KnowledgeDriveImportService::new(&drive, &metadata);

    let error = service
        .import_drive_object(resolved_drive_import_request(
            "Quarterly Report",
            "node-quarterly-report",
            "incoming/quarterly-report.md",
            "drive-quarterly-report",
        ))
        .await
        .unwrap_err();

    assert!(error
        .to_string()
        .contains("drive_storage_provider_id does not match"));
    assert_eq!(
        drive.heads(),
        vec![(
            "knowledgebase-source".to_string(),
            "incoming/quarterly-report.md".to_string()
        )]
    );
    assert_eq!(sources.create_count(), 0);
    assert_eq!(documents.create_count(), 0);
    assert_eq!(versions.create_count(), 0);
    assert_eq!(object_refs.create_count(), 0);
    assert_eq!(jobs.create_count(), 0);
}

#[tokio::test]
async fn drive_import_preserves_drive_node_binding_for_browser_projection() {
    let drive = RecordingDrive::with_object(
        "knowledgebase-source",
        "incoming/quarterly-report.md",
        "# Report",
    );
    let sources = MemorySourceStore::default();
    let documents = MemoryDocumentStore::default();
    let object_refs = MemoryDriveObjectRefStore::default();
    let versions = MemoryDocumentVersionStore::default();
    let jobs = MemoryIngestionJobStore::default();
    let metadata =
        MemoryDriveImportMetadataStore::new(&sources, &documents, &object_refs, &versions, &jobs);
    let service = KnowledgeDriveImportService::new(&drive, &metadata);

    let result = service
        .import_drive_object(resolved_drive_import_request(
            "Quarterly Report",
            "node-report",
            "incoming/quarterly-report.md",
            "drive-quarterly-report",
        ))
        .await
        .unwrap();

    assert_eq!(
        result.original_object_ref.drive_space_id.as_deref(),
        Some("drv-kb-001")
    );
    assert_eq!(
        result.original_object_ref.drive_node_id.as_deref(),
        Some("node-report")
    );
    assert_eq!(
        result.document.original_file_drive_node_id.as_deref(),
        Some("node-report")
    );
    assert_eq!(
        object_refs.created_refs()[0].drive_node_id.as_deref(),
        Some("node-report")
    );
}

#[derive(Clone)]
struct StoredObject {
    object_ref: KnowledgeObjectRef,
}

#[derive(Default)]
struct RecordingDrive {
    objects: Mutex<HashMap<String, StoredObject>>,
    heads: Arc<Mutex<Vec<(String, String)>>>,
}

impl RecordingDrive {
    fn with_object(bucket: &str, object_key: &str, body: &str) -> Self {
        let drive = Self::default();
        drive.add_object(bucket, object_key, body);
        drive
    }

    fn add_object(&self, bucket: &str, object_key: &str, body: &str) {
        self.objects.lock().unwrap().insert(
            object_key.to_string(),
            StoredObject {
                object_ref: KnowledgeObjectRef {
                    storage_provider_id: "provider-kb".to_string(),
                    bucket: bucket.to_string(),
                    object_key: object_key.to_string(),
                    logical_path: object_key.to_string(),
                    object_role: "original_document".to_string(),
                    content_type: "text/markdown; charset=utf-8".to_string(),
                    size_bytes: body.len() as u64,
                    checksum_sha256_hex: None,
                    etag: Some("etag".to_string()),
                    version_id: Some("v1".to_string()),
                },
            },
        );
    }

    fn set_object_storage_provider_id(&self, object_key: &str, storage_provider_id: &str) {
        if let Some(stored) = self.objects.lock().unwrap().get_mut(object_key) {
            stored.object_ref.storage_provider_id = storage_provider_id.to_string();
        }
    }

    fn heads(&self) -> Vec<(String, String)> {
        self.heads.lock().unwrap().clone()
    }
}

#[async_trait]
impl KnowledgeDriveStorage for RecordingDrive {
    async fn put_object(
        &self,
        _request: PutKnowledgeObjectRequest,
    ) -> Result<KnowledgeObjectRef, KnowledgeStorageError> {
        Err(KnowledgeStorageError::internal("not needed"))
    }

    async fn head_object(
        &self,
        request: HeadKnowledgeObjectRequest,
    ) -> Result<KnowledgeObjectRef, KnowledgeStorageError> {
        self.heads
            .lock()
            .unwrap()
            .push((request.bucket.clone(), request.object_key.clone()));
        self.objects
            .lock()
            .unwrap()
            .get(&request.object_key)
            .map(|stored| stored.object_ref.clone())
            .ok_or_else(|| KnowledgeStorageError::NotFound(request.object_key))
    }

    async fn get_object_text(
        &self,
        _object_ref: &KnowledgeObjectRef,
    ) -> Result<String, KnowledgeStorageError> {
        Err(KnowledgeStorageError::internal("not needed"))
    }
}

struct MemoryDriveImportMetadataStore<'a> {
    sources: &'a MemorySourceStore,
    documents: &'a MemoryDocumentStore,
    object_refs: &'a MemoryDriveObjectRefStore,
    versions: &'a MemoryDocumentVersionStore,
    jobs: &'a MemoryIngestionJobStore,
}

impl<'a> MemoryDriveImportMetadataStore<'a> {
    fn new(
        sources: &'a MemorySourceStore,
        documents: &'a MemoryDocumentStore,
        object_refs: &'a MemoryDriveObjectRefStore,
        versions: &'a MemoryDocumentVersionStore,
        jobs: &'a MemoryIngestionJobStore,
    ) -> Self {
        Self {
            sources,
            documents,
            object_refs,
            versions,
            jobs,
        }
    }
}

#[async_trait]
impl DriveImportMetadataStore for MemoryDriveImportMetadataStore<'_> {
    async fn validate_drive_import_idempotency(
        &self,
        record: CreateIngestionJobRecord,
    ) -> Result<(), DriveImportMetadataStoreError> {
        self.jobs
            .validate_idempotency(&record)
            .map_err(ingestion_job_metadata_error)
    }

    async fn create_or_prepare_drive_import_metadata(
        &self,
        record: PrepareDriveImportMetadataRecord,
    ) -> Result<PreparedDriveImportMetadata, DriveImportMetadataStoreError> {
        let job = self
            .jobs
            .create_or_get_job(record.job)
            .await
            .map_err(ingestion_job_metadata_error)?;

        let original_object_ref = self
            .object_refs
            .create_or_get_object_ref(record.object_ref)
            .await
            .map_err(|error| DriveImportMetadataStoreError::Internal(error.to_string()))?;

        let source = self
            .sources
            .create_or_get_source(record.source)
            .await
            .map_err(|error| DriveImportMetadataStoreError::Internal(error.to_string()))?;

        let mut document_record = record.document;
        document_record.source_id = Some(source.id);
        let mut document = self
            .documents
            .create_or_get_document(document_record)
            .await
            .map_err(|error| DriveImportMetadataStoreError::Internal(error.to_string()))?;

        let version = self
            .versions
            .create_or_get_document_version(CreateKnowledgeDocumentVersionRecord {
                document_id: document.id,
                version_no: record.version.version_no,
                original_object_ref_id: original_object_ref.id,
                checksum_sha256_hex: record.version.checksum_sha256_hex,
                size_bytes: record.version.size_bytes,
                mime_type: record.version.mime_type,
            })
            .await
            .map_err(|error| DriveImportMetadataStoreError::Internal(error.to_string()))?;
        document.current_version_id = Some(version.id);

        self.jobs
            .attach_drive_import_linkage(
                job.job.id,
                DriveImportJobLinkage {
                    source_id: source.id,
                    document_id: document.id,
                    document_version_id: version.id,
                    original_object_ref: original_object_ref.clone(),
                },
            )
            .await
            .map_err(ingestion_job_metadata_error)?;

        Ok(PreparedDriveImportMetadata {
            job: job.job,
            source,
            document,
            version,
            original_object_ref,
        })
    }
}

fn ingestion_job_metadata_error(error: IngestionJobStoreError) -> DriveImportMetadataStoreError {
    match error {
        IngestionJobStoreError::Conflict(detail) => DriveImportMetadataStoreError::Conflict(detail),
        other => DriveImportMetadataStoreError::Internal(other.to_string()),
    }
}

#[derive(Default)]
struct MemorySourceStore {
    next_id: Mutex<u64>,
    by_locator: Mutex<HashMap<(u64, String, String), KnowledgeSource>>,
}

impl MemorySourceStore {
    fn create_count(&self) -> usize {
        self.by_locator.lock().unwrap().len()
    }
}

#[async_trait]
impl KnowledgeSourceStore for MemorySourceStore {
    async fn create_source(
        &self,
        record: CreateKnowledgeSourceRecord,
    ) -> Result<KnowledgeSource, KnowledgeSourceStoreError> {
        self.insert_source(record)
    }

    async fn create_or_get_source(
        &self,
        record: CreateKnowledgeSourceRecord,
    ) -> Result<KnowledgeSource, KnowledgeSourceStoreError> {
        let key = source_key(&record);
        if let Some(source) = self.by_locator.lock().unwrap().get(&key).cloned() {
            return Ok(source);
        }
        self.insert_source(record)
    }
}

impl MemorySourceStore {
    fn insert_source(
        &self,
        record: CreateKnowledgeSourceRecord,
    ) -> Result<KnowledgeSource, KnowledgeSourceStoreError> {
        let mut next_id = self.next_id.lock().unwrap();
        *next_id += 1;
        let source = KnowledgeSource {
            id: *next_id,
            space_id: record.space_id,
            source_type: record.source_type,
            provider: record.provider,
            drive_bucket: record.drive_bucket,
            drive_prefix: record.drive_prefix,
            connector_metadata_json: record.connector_metadata_json,
        };
        if let (Some(bucket), Some(prefix)) = (&source.drive_bucket, &source.drive_prefix) {
            self.by_locator.lock().unwrap().insert(
                (source.space_id, bucket.clone(), prefix.clone()),
                source.clone(),
            );
        }
        Ok(source)
    }
}

fn source_key(record: &CreateKnowledgeSourceRecord) -> (u64, String, String) {
    (
        record.space_id,
        record.drive_bucket.clone().unwrap_or_default(),
        record.drive_prefix.clone().unwrap_or_default(),
    )
}

#[derive(Default)]
struct MemoryDocumentStore {
    next_id: Mutex<u64>,
    by_identity: Mutex<HashMap<DocumentIdentityKey, KnowledgeDocument>>,
}

impl MemoryDocumentStore {
    fn create_count(&self) -> usize {
        self.by_identity.lock().unwrap().len()
    }
}

#[async_trait]
impl KnowledgeDocumentStore for MemoryDocumentStore {
    async fn create_document(
        &self,
        record: CreateKnowledgeDocumentRecord,
    ) -> Result<KnowledgeDocument, KnowledgeDocumentStoreError> {
        self.insert_document(record)
    }

    async fn create_or_get_document(
        &self,
        record: CreateKnowledgeDocumentRecord,
    ) -> Result<KnowledgeDocument, KnowledgeDocumentStoreError> {
        let key = document_key(&record);
        if let Some(mut document) = self.by_identity.lock().unwrap().get(&key).cloned() {
            if document.original_file_drive_node_id.is_none()
                && record.original_file_drive_node_id.is_some()
            {
                document.original_file_drive_node_id = record.original_file_drive_node_id;
                self.by_identity
                    .lock()
                    .unwrap()
                    .insert(key, document.clone());
            }
            return Ok(document);
        }
        self.insert_document(record)
    }

    async fn get_document_by_id(
        &self,
        document_id: u64,
    ) -> Result<KnowledgeDocument, KnowledgeDocumentStoreError> {
        let mut document = None;
        for value in self.by_identity.lock().unwrap().values() {
            if value.id == document_id {
                document = Some(value.clone());
                break;
            }
        }
        document.ok_or_else(|| {
            KnowledgeDocumentStoreError::Internal(format!(
                "missing knowledge document: {document_id}"
            ))
        })
    }

    async fn list_documents_for_space(
        &self,
        space_id: u64,
        limit: u32,
    ) -> Result<Vec<KnowledgeDocument>, KnowledgeDocumentStoreError> {
        Ok(self
            .by_identity
            .lock()
            .unwrap()
            .values()
            .filter(|document| document.space_id == space_id)
            .take(limit.max(1) as usize)
            .cloned()
            .collect())
    }
}

impl MemoryDocumentStore {
    fn insert_document(
        &self,
        record: CreateKnowledgeDocumentRecord,
    ) -> Result<KnowledgeDocument, KnowledgeDocumentStoreError> {
        let key = document_key(&record);
        let mut next_id = self.next_id.lock().unwrap();
        *next_id += 1;
        let document = KnowledgeDocument {
            id: *next_id,
            space_id: record.space_id,
            source_id: record.source_id,
            original_file_drive_node_id: record.original_file_drive_node_id,
            title: record.title,
            mime_type: record.mime_type,
            language: record.language,
            current_version_id: None,
            visibility: KnowledgeDocumentVisibility::Space,
            content_state: KnowledgeDocumentState::Ready,
            index_state: KnowledgeDocumentVersionState::Pending,
        };
        self.by_identity
            .lock()
            .unwrap()
            .insert(key, document.clone());
        Ok(document)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
struct DocumentIdentityKey {
    space_id: u64,
    identity_scope: KnowledgeDocumentIdentityScope,
    source_id: Option<u64>,
    original_file_drive_node_id: Option<String>,
}

fn document_key(record: &CreateKnowledgeDocumentRecord) -> DocumentIdentityKey {
    DocumentIdentityKey {
        space_id: record.space_id,
        identity_scope: record.identity_scope,
        source_id: record.source_id,
        original_file_drive_node_id: match record.identity_scope {
            KnowledgeDocumentIdentityScope::SourceOnly => None,
            KnowledgeDocumentIdentityScope::SourceAndOriginalDriveNode => {
                record.original_file_drive_node_id.clone()
            }
        },
    }
}

#[derive(Default)]
struct MemoryDriveObjectRefStore {
    next_id: Mutex<u64>,
    created_refs: Mutex<Vec<KnowledgeDriveObjectRef>>,
}

impl MemoryDriveObjectRefStore {
    fn created_refs(&self) -> Vec<KnowledgeDriveObjectRef> {
        self.created_refs.lock().unwrap().clone()
    }

    fn create_count(&self) -> usize {
        self.created_refs.lock().unwrap().len()
    }
}

#[async_trait]
impl KnowledgeDriveObjectRefStore for MemoryDriveObjectRefStore {
    async fn create_object_ref(
        &self,
        record: CreateKnowledgeDriveObjectRefRecord,
    ) -> Result<KnowledgeDriveObjectRef, KnowledgeDriveObjectRefStoreError> {
        self.insert_object_ref(record)
    }

    async fn create_or_get_object_ref(
        &self,
        record: CreateKnowledgeDriveObjectRefRecord,
    ) -> Result<KnowledgeDriveObjectRef, KnowledgeDriveObjectRefStoreError> {
        let refs = self.created_refs.lock().unwrap();
        if let Some(object_ref) = refs
            .iter()
            .find(|object_ref| {
                object_ref.space_id == record.space_id
                    && object_ref.drive_storage_provider_id == record.drive_storage_provider_id
                    && object_ref.drive_bucket == record.drive_bucket
                    && object_ref.drive_object_key == record.drive_object_key
                    && object_ref.drive_object_version == record.drive_object_version
                    && object_ref.object_role == record.object_role
            })
            .cloned()
        {
            return Ok(object_ref);
        }
        drop(refs);
        self.insert_object_ref(record)
    }

    async fn list_object_refs_by_logical_path_prefix(
        &self,
        space_id: u64,
        prefix: &str,
    ) -> Result<Vec<KnowledgeDriveObjectRef>, KnowledgeDriveObjectRefStoreError> {
        Ok(self
            .created_refs
            .lock()
            .unwrap()
            .iter()
            .filter(|object_ref| {
                object_ref.space_id == space_id
                    && object_ref
                        .logical_path
                        .as_deref()
                        .is_some_and(|path| path.starts_with(prefix))
            })
            .cloned()
            .collect())
    }

    async fn get_object_ref_by_id(
        &self,
        object_ref_id: u64,
    ) -> Result<KnowledgeDriveObjectRef, KnowledgeDriveObjectRefStoreError> {
        self.created_refs
            .lock()
            .unwrap()
            .iter()
            .find(|object_ref| object_ref.id == object_ref_id)
            .cloned()
            .ok_or_else(|| {
                KnowledgeDriveObjectRefStoreError::Internal(format!(
                    "drive object ref not found: {object_ref_id}"
                ))
            })
    }
}

impl MemoryDriveObjectRefStore {
    fn insert_object_ref(
        &self,
        record: CreateKnowledgeDriveObjectRefRecord,
    ) -> Result<KnowledgeDriveObjectRef, KnowledgeDriveObjectRefStoreError> {
        let mut next_id = self.next_id.lock().unwrap();
        *next_id += 1;
        let object_ref = KnowledgeDriveObjectRef {
            id: *next_id,
            space_id: record.space_id,
            drive_space_id: record.drive_space_id,
            drive_node_id: record.drive_node_id,
            logical_path: record.logical_path,
            drive_provider_kind: record.drive_provider_kind,
            drive_storage_provider_id: record.drive_storage_provider_id,
            drive_bucket: record.drive_bucket,
            drive_object_key: record.drive_object_key,
            drive_object_version: record.drive_object_version,
            drive_etag: record.drive_etag,
            content_type: record.content_type,
            size_bytes: record.size_bytes,
            checksum_sha256_hex: record.checksum_sha256_hex,
            object_role: record.object_role,
            access_mode: record.access_mode,
        };
        self.created_refs.lock().unwrap().push(object_ref.clone());
        Ok(object_ref)
    }
}

#[derive(Default)]
struct MemoryDocumentVersionStore {
    next_id: Mutex<u64>,
    by_document: Mutex<HashMap<(u64, u64), KnowledgeDocumentVersion>>,
}

impl MemoryDocumentVersionStore {
    fn create_count(&self) -> usize {
        self.by_document.lock().unwrap().len()
    }
}

#[async_trait]
impl KnowledgeDocumentVersionStore for MemoryDocumentVersionStore {
    async fn create_document_version(
        &self,
        record: CreateKnowledgeDocumentVersionRecord,
    ) -> Result<KnowledgeDocumentVersion, KnowledgeDocumentVersionStoreError> {
        self.insert_document_version(record)
    }

    async fn create_or_get_document_version(
        &self,
        record: CreateKnowledgeDocumentVersionRecord,
    ) -> Result<KnowledgeDocumentVersion, KnowledgeDocumentVersionStoreError> {
        let key = (record.document_id, record.version_no);
        if let Some(version) = self.by_document.lock().unwrap().get(&key).cloned() {
            return Ok(version);
        }
        self.insert_document_version(record)
    }
}

impl MemoryDocumentVersionStore {
    fn insert_document_version(
        &self,
        record: CreateKnowledgeDocumentVersionRecord,
    ) -> Result<KnowledgeDocumentVersion, KnowledgeDocumentVersionStoreError> {
        let mut next_id = self.next_id.lock().unwrap();
        *next_id += 1;
        let version = KnowledgeDocumentVersion {
            id: *next_id,
            document_id: record.document_id,
            version_no: record.version_no,
            original_object_ref_id: record.original_object_ref_id,
            checksum_sha256_hex: record.checksum_sha256_hex,
            size_bytes: record.size_bytes,
            mime_type: record.mime_type,
            parse_state: KnowledgeDocumentVersionState::Pending,
            index_state: KnowledgeDocumentVersionState::Pending,
        };
        self.by_document
            .lock()
            .unwrap()
            .insert((version.document_id, version.version_no), version.clone());
        Ok(version)
    }
}

#[derive(Default)]
struct MemoryIngestionJobStore {
    next_id: Mutex<u64>,
    by_id: Mutex<HashMap<u64, IngestionJob>>,
    by_key: Mutex<HashMap<(u64, String), u64>>,
    fingerprint_by_id: Mutex<HashMap<u64, Option<String>>>,
    linkages: Mutex<HashMap<u64, DriveImportJobLinkage>>,
}

impl MemoryIngestionJobStore {
    fn create_count(&self) -> usize {
        self.by_id.lock().unwrap().len()
    }

    fn validate_idempotency(
        &self,
        record: &CreateIngestionJobRecord,
    ) -> Result<(), IngestionJobStoreError> {
        let key = (record.space_id, record.idempotency_key.clone());
        let Some(existing_id) = self.by_key.lock().unwrap().get(&key).copied() else {
            return Ok(());
        };
        let job = self
            .by_id
            .lock()
            .unwrap()
            .get(&existing_id)
            .cloned()
            .ok_or_else(|| IngestionJobStoreError::Internal("missing job".to_string()))?;
        if job.source_type != record.source_type {
            return Err(IngestionJobStoreError::Conflict(
                "idempotency_key is already used for a different job_type".to_string(),
            ));
        }
        if let Some(expected_fingerprint) = &record.idempotency_fingerprint_sha256_hex {
            let existing_fingerprint = self
                .fingerprint_by_id
                .lock()
                .unwrap()
                .get(&existing_id)
                .cloned()
                .flatten();
            if existing_fingerprint.as_deref() != Some(expected_fingerprint.as_str()) {
                return Err(IngestionJobStoreError::Conflict(
                    "idempotency_key is already used for a different request".to_string(),
                ));
            }
        }
        Ok(())
    }
}

#[async_trait]
impl IngestionJobStore for MemoryIngestionJobStore {
    async fn create_or_get_job(
        &self,
        record: CreateIngestionJobRecord,
    ) -> Result<CreateOrGetIngestionJobResult, IngestionJobStoreError> {
        let key = (record.space_id, record.idempotency_key.clone());
        if let Some(existing_id) = self.by_key.lock().unwrap().get(&key).copied() {
            let job = self
                .by_id
                .lock()
                .unwrap()
                .get(&existing_id)
                .cloned()
                .ok_or_else(|| IngestionJobStoreError::Internal("missing job".to_string()))?;
            if job.source_type != record.source_type {
                return Err(IngestionJobStoreError::Conflict(
                    "idempotency_key is already used for a different job_type".to_string(),
                ));
            }
            let existing_fingerprint = self
                .fingerprint_by_id
                .lock()
                .unwrap()
                .get(&existing_id)
                .cloned()
                .flatten();
            if let Some(expected_fingerprint) = &record.idempotency_fingerprint_sha256_hex {
                if existing_fingerprint.as_deref() != Some(expected_fingerprint.as_str()) {
                    return Err(IngestionJobStoreError::Conflict(
                        "idempotency_key is already used for a different request".to_string(),
                    ));
                }
            }
            return Ok(CreateOrGetIngestionJobResult {
                job,
                created: false,
            });
        }

        let mut next_id = self.next_id.lock().unwrap();
        *next_id += 1;
        let job = IngestionJob {
            id: *next_id,
            space_id: record.space_id,
            source_type: record.source_type,
            idempotency_key: record.idempotency_key,
            state: IngestionJobState::Queued,
            error_message: None,
        };
        self.by_key.lock().unwrap().insert(key, job.id);
        self.fingerprint_by_id
            .lock()
            .unwrap()
            .insert(job.id, record.idempotency_fingerprint_sha256_hex);
        self.by_id.lock().unwrap().insert(job.id, job.clone());
        Ok(CreateOrGetIngestionJobResult { job, created: true })
    }

    async fn get_job(&self, job_id: u64) -> Result<IngestionJob, IngestionJobStoreError> {
        self.by_id
            .lock()
            .unwrap()
            .get(&job_id)
            .cloned()
            .ok_or(IngestionJobStoreError::NotFound(job_id))
    }

    async fn update_job_state(
        &self,
        job_id: u64,
        expected_state: IngestionJobState,
        state: IngestionJobState,
        error_message: Option<String>,
    ) -> Result<IngestionJob, IngestionJobStoreError> {
        let mut jobs = self.by_id.lock().unwrap();
        let job = jobs
            .get_mut(&job_id)
            .ok_or(IngestionJobStoreError::NotFound(job_id))?;
        if job.state != expected_state {
            return Err(IngestionJobStoreError::NotFound(job_id));
        }
        job.state = state;
        job.error_message = error_message;
        Ok(job.clone())
    }

    async fn attach_drive_import_linkage(
        &self,
        job_id: u64,
        linkage: DriveImportJobLinkage,
    ) -> Result<(), IngestionJobStoreError> {
        if !self.by_id.lock().unwrap().contains_key(&job_id) {
            return Err(IngestionJobStoreError::NotFound(job_id));
        }
        self.linkages.lock().unwrap().insert(job_id, linkage);
        Ok(())
    }

    async fn get_drive_import_linkage(
        &self,
        job_id: u64,
    ) -> Result<Option<DriveImportJobLinkage>, IngestionJobStoreError> {
        if !self.by_id.lock().unwrap().contains_key(&job_id) {
            return Err(IngestionJobStoreError::NotFound(job_id));
        }
        Ok(self.linkages.lock().unwrap().get(&job_id).cloned())
    }

    async fn list_jobs_by_state(
        &self,
        state: IngestionJobState,
        limit: u32,
    ) -> Result<Vec<IngestionJob>, IngestionJobStoreError> {
        let jobs = self.by_id.lock().unwrap();
        Ok(jobs
            .values()
            .filter(|job| job.state == state)
            .take(limit as usize)
            .cloned()
            .collect())
    }

    async fn mark_running_job_succeeded_with_outbox(
        &self,
        job_id: u64,
        _outbox: sdkwork_intelligence_knowledgebase_service::ports::knowledge_outbox_store::AppendOutboxEventRecord,
    ) -> Result<IngestionJob, IngestionJobStoreError> {
        let mut jobs = self.by_id.lock().unwrap();
        let job = jobs
            .get_mut(&job_id)
            .ok_or(IngestionJobStoreError::NotFound(job_id))?;
        if job.state != IngestionJobState::Running {
            return Err(IngestionJobStoreError::Conflict(format!(
                "invalid ingestion job transition: {:?} -> {:?}",
                job.state,
                IngestionJobState::Succeeded
            )));
        }
        job.state = IngestionJobState::Succeeded;
        job.error_message = None;
        Ok(job.clone())
    }

    async fn complete_running_ingestion_with_chunks_and_outbox(
        &self,
        record: sdkwork_intelligence_knowledgebase_service::ports::knowledge_ingestion_job_store::CompleteRunningIngestionRecord,
    ) -> Result<
        sdkwork_intelligence_knowledgebase_service::ports::knowledge_ingestion_job_store::CompletedIngestionResult,
        IngestionJobStoreError,
    >{
        let job = self
            .mark_running_job_succeeded_with_outbox(record.job_id, record.outbox)
            .await?;
        Ok(
            sdkwork_intelligence_knowledgebase_service::ports::knowledge_ingestion_job_store::CompletedIngestionResult {
                job,
                chunk_count: record.chunks.len(),
            },
        )
    }
}
