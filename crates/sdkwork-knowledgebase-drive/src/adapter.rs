use async_trait::async_trait;
use sdkwork_drive_storage_contract::{
    CreateBucketRequest, DeleteObjectRequest, DriveByteRange, DriveObjectLocator, DriveObjectStore,
    DriveObjectStoreError, DriveObjectStoreErrorKind, HeadBucketRequest, HeadObjectRequest,
    PutObjectRequest, ReadObjectRangeRequest,
};
use sdkwork_drive_workspace_service::application::space_service::{
    CreateSpaceCommand, DeleteSpaceCommand, GetSpaceCommand, SqlDriveSpaceService,
};
use sdkwork_drive_workspace_service::application::workspace_service::{
    DriveWorkspaceChildrenPage, DriveWorkspaceNode, DriveWorkspaceNodeKind,
    DriveWorkspaceObjectRef, EnsureDriveWorkspaceNode, EnsureDriveWorkspaceNodesCommand,
    GetDriveWorkspaceNodeCommand, ListDriveWorkspaceChildrenCommand,
    ResolveDriveWorkspacePathCommand, SqlDriveWorkspaceService,
};
use sdkwork_drive_workspace_service::domain::space::DriveSpaceType;
use sdkwork_drive_workspace_service::DriveServiceError;
use sdkwork_intelligence_knowledgebase_object_key_service::object_key::KnowledgeObjectKeyPlanner;
use sdkwork_intelligence_knowledgebase_service::ports::knowledge_drive_node_tree::{
    DriveNodeKind, GetKnowledgeDriveNodeRequest, KnowledgeDriveNodeObjectLocator,
    KnowledgeDriveNodePage, KnowledgeDriveNodeSummary, KnowledgeDriveNodeTree,
    KnowledgeDriveNodeTreeError, ListKnowledgeDriveNodeChildrenRequest,
    ResolveKnowledgeDriveNodePathRequest,
};
use sdkwork_intelligence_knowledgebase_service::ports::knowledge_drive_space::{
    CreateKnowledgeDriveSpaceRequest, DeleteKnowledgeDriveSpaceRequest, KnowledgeDriveSpaceBinding,
    KnowledgeDriveSpaceProvisioner, KnowledgeDriveSpaceProvisionerError,
};
use sdkwork_intelligence_knowledgebase_service::ports::knowledge_drive_storage::{
    object_read_limit_error, validate_object_read_size, HeadKnowledgeObjectRequest,
    KnowledgeDriveStorage, KnowledgeObjectRef, KnowledgeStorageError, PutKnowledgeObjectRequest,
    DEFAULT_MAX_KNOWLEDGE_OBJECT_READ_BYTES,
};
use sdkwork_intelligence_knowledgebase_service::ports::knowledge_drive_workspace::{
    EnsureKnowledgeDriveNodeKind, EnsureKnowledgeDriveNodeRequest,
    EnsureKnowledgeDriveNodesRequest, KnowledgeDriveWorkspace, KnowledgeDriveWorkspaceError,
};
use sdkwork_utils_rust::{is_blank, sha256_hash};
use sqlx::PgPool;
use std::collections::{BTreeMap, HashMap};
use std::sync::Arc;

pub struct KnowledgebaseDriveStorageAdapter {
    store: Arc<dyn DriveObjectStore>,
    storage_provider_id: String,
    bucket: String,
    tenant_id: String,
}

#[derive(Debug, Clone)]
pub struct KnowledgebaseDriveSpaceProvisionerAdapter {
    pool: PgPool,
}

impl KnowledgebaseDriveSpaceProvisionerAdapter {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }
}

#[derive(Debug, Clone)]
pub struct KnowledgebaseDriveWorkspaceAdapter {
    pool: PgPool,
    tenant_id: String,
    operator_id: String,
}

impl KnowledgebaseDriveWorkspaceAdapter {
    pub fn new(pool: PgPool, tenant_id: impl Into<String>, operator_id: impl Into<String>) -> Self {
        Self {
            pool,
            tenant_id: tenant_id.into(),
            operator_id: operator_id.into(),
        }
    }
}

#[derive(Debug, Clone)]
pub struct KnowledgebaseDriveNodeTreeAdapter {
    pool: PgPool,
    tenant_id: String,
}

impl KnowledgebaseDriveNodeTreeAdapter {
    pub fn new(pool: PgPool, tenant_id: impl Into<String>) -> Self {
        Self {
            pool,
            tenant_id: tenant_id.into(),
        }
    }
}

impl KnowledgebaseDriveStorageAdapter {
    pub fn new<S>(
        store: Arc<S>,
        storage_provider_id: impl Into<String>,
        bucket: impl Into<String>,
        tenant_id: impl Into<String>,
    ) -> Self
    where
        S: DriveObjectStore + 'static,
    {
        Self::from_object_store(store, storage_provider_id, bucket, tenant_id)
    }

    pub fn from_object_store(
        store: Arc<dyn DriveObjectStore>,
        storage_provider_id: impl Into<String>,
        bucket: impl Into<String>,
        tenant_id: impl Into<String>,
    ) -> Self {
        Self {
            store,
            storage_provider_id: storage_provider_id.into(),
            bucket: bucket.into(),
            tenant_id: tenant_id.into(),
        }
    }

    pub fn for_tenant(&self, tenant_id: impl Into<String>) -> Self {
        Self {
            store: self.store.clone(),
            storage_provider_id: self.storage_provider_id.clone(),
            bucket: self.bucket.clone(),
            tenant_id: tenant_id.into(),
        }
    }

    pub async fn ensure_bucket(&self) -> Result<(), KnowledgeStorageError> {
        self.store
            .create_bucket(CreateBucketRequest {
                bucket: self.bucket.clone(),
            })
            .await
            .map_err(map_drive_error)?;
        Ok(())
    }

    pub async fn readiness_check(&self) -> Result<(), KnowledgeStorageError> {
        let response = self
            .store
            .head_bucket(HeadBucketRequest {
                bucket: self.bucket.clone(),
            })
            .await
            .map_err(map_drive_error)?;
        if !response.exists || response.bucket != self.bucket {
            return Err(KnowledgeStorageError::NotFound(
                "knowledge storage bucket is unavailable".to_string(),
            ));
        }
        Ok(())
    }

    fn locator_for(
        &self,
        logical_path: &str,
        space_uuid: Option<&str>,
    ) -> Result<DriveObjectLocator, KnowledgeStorageError> {
        let safe_logical_path = safe_logical_path(logical_path)?;
        let object_key = match space_uuid {
            Some(space_uuid) => KnowledgeObjectKeyPlanner::new(&self.tenant_id, space_uuid)
                .map_err(|error| KnowledgeStorageError::InvalidRequest(error.to_string()))?
                .okf_bundle_file(&safe_logical_path)
                .map_err(|error| KnowledgeStorageError::InvalidRequest(error.to_string()))?,
            None => {
                return Err(KnowledgeStorageError::InvalidRequest(
                    "space_uuid is required for knowledge object storage".to_string(),
                ));
            }
        };

        Ok(DriveObjectLocator {
            bucket: self.bucket.clone(),
            object_key,
        })
    }
}

#[async_trait]
impl KnowledgeDriveSpaceProvisioner for KnowledgebaseDriveSpaceProvisionerAdapter {
    async fn create_knowledge_drive_space(
        &self,
        request: CreateKnowledgeDriveSpaceRequest,
    ) -> Result<KnowledgeDriveSpaceBinding, KnowledgeDriveSpaceProvisionerError> {
        let tenant_id = safe_drive_identifier(&request.tenant_id, "tenant_id")
            .map_err(KnowledgeDriveSpaceProvisionerError::InvalidRequest)?;
        let owner_subject_type = require_drive_owner_subject_type(&request.owner_subject_type)?;
        let owner_subject_id = safe_drive_owner_subject_id(&request.owner_subject_id)?;
        let display_name = require_non_empty(request.display_name, "display_name")?;
        let operator_id = safe_drive_identifier(&request.operator_id, "operator_id")
            .map_err(KnowledgeDriveSpaceProvisionerError::InvalidRequest)?;

        let drive_space_id = drive_space_id_for_knowledge_space(&request.knowledge_space_uuid)?;
        let service = self.space_service();
        if let Ok(existing) = service
            .get_space(GetSpaceCommand {
                tenant_id: tenant_id.clone(),
                space_id: drive_space_id.clone(),
            })
            .await
        {
            return Ok(KnowledgeDriveSpaceBinding {
                drive_space_id: existing.id,
            });
        }

        match service
            .create_space(CreateSpaceCommand {
                id: drive_space_id.clone(),
                tenant_id: tenant_id.clone(),
                owner_subject_type: owner_subject_type.clone(),
                owner_subject_id: owner_subject_id.clone(),
                display_name,
                space_type: DriveSpaceType::KnowledgeBase,
                presentation_icon: None,
                presentation_color: None,
                description: None,
                operator_id,
            })
            .await
        {
            Ok(space) => Ok(KnowledgeDriveSpaceBinding {
                drive_space_id: space.id,
            }),
            Err(DriveServiceError::Conflict(_)) => {
                let existing = service
                    .get_space(GetSpaceCommand {
                        tenant_id,
                        space_id: drive_space_id,
                    })
                    .await
                    .map_err(map_space_service_error)?;
                Ok(KnowledgeDriveSpaceBinding {
                    drive_space_id: existing.id,
                })
            }
            Err(error) => Err(map_space_service_error(error)),
        }
    }

    async fn delete_knowledge_drive_space(
        &self,
        request: DeleteKnowledgeDriveSpaceRequest,
    ) -> Result<(), KnowledgeDriveSpaceProvisionerError> {
        let tenant_id = safe_drive_identifier(&request.tenant_id, "tenant_id")
            .map_err(KnowledgeDriveSpaceProvisionerError::InvalidRequest)?;
        let drive_space_id = safe_drive_identifier(&request.drive_space_id, "drive_space_id")
            .map_err(KnowledgeDriveSpaceProvisionerError::InvalidRequest)?;
        let owner_subject_type = require_drive_owner_subject_type(&request.owner_subject_type)?;
        let owner_subject_id = safe_drive_owner_subject_id(&request.owner_subject_id)?;
        let operator_id = safe_drive_identifier(&request.operator_id, "operator_id")
            .map_err(KnowledgeDriveSpaceProvisionerError::InvalidRequest)?;

        let service = self.space_service();
        let drive_space = match service
            .get_space(GetSpaceCommand {
                tenant_id: tenant_id.clone(),
                space_id: drive_space_id.clone(),
            })
            .await
        {
            Ok(space) => space,
            Err(DriveServiceError::NotFound(_)) => return Ok(()),
            Err(error) => return Err(map_space_service_error(error)),
        };

        if drive_space.space_type != DriveSpaceType::KnowledgeBase
            || drive_space.owner_subject_type != owner_subject_type
            || drive_space.owner_subject_id != owner_subject_id
        {
            return Err(KnowledgeDriveSpaceProvisionerError::InvalidRequest(
                "drive space does not belong to the requested knowledge space owner".to_string(),
            ));
        }

        match service
            .delete_space(DeleteSpaceCommand {
                tenant_id,
                space_id: drive_space_id,
                operator_id,
            })
            .await
        {
            Ok(_) | Err(DriveServiceError::NotFound(_)) => Ok(()),
            Err(error) => Err(map_space_service_error(error)),
        }
    }
}

impl KnowledgebaseDriveSpaceProvisionerAdapter {
    fn space_service(&self) -> SqlDriveSpaceService {
        SqlDriveSpaceService::new(self.pool.clone())
    }
}

#[async_trait]
impl KnowledgeDriveStorage for KnowledgebaseDriveStorageAdapter {
    async fn put_object(
        &self,
        request: PutKnowledgeObjectRequest,
    ) -> Result<KnowledgeObjectRef, KnowledgeStorageError> {
        safe_logical_path(&request.logical_path)?;
        let locator = self.locator_for(&request.logical_path, request.space_uuid.as_deref())?;
        let size_bytes = request.body.len() as u64;
        let computed_checksum_sha256_hex = sha256_hash(&request.body);
        let checksum_sha256_hex = verified_request_checksum(
            request.checksum_sha256_hex.as_deref(),
            &computed_checksum_sha256_hex,
        )?;
        let mut metadata = BTreeMap::new();
        metadata.insert("logical_path".to_string(), request.logical_path.clone());
        metadata.insert("object_role".to_string(), request.object_role.clone());

        let response = self
            .store
            .put_object(PutObjectRequest {
                locator: locator.clone(),
                content_type: Some(request.content_type.clone()),
                metadata,
                body: request.body,
                checksum_sha256_hex: Some(checksum_sha256_hex.clone()),
            })
            .await
            .map_err(map_drive_error)?;
        let version_id = content_version_id(response.version_id, &checksum_sha256_hex);

        Ok(KnowledgeObjectRef {
            storage_provider_id: self.storage_provider_id.clone(),
            bucket: response.locator.bucket,
            object_key: response.locator.object_key,
            logical_path: request.logical_path,
            object_role: request.object_role,
            content_type: request.content_type,
            size_bytes,
            checksum_sha256_hex: Some(checksum_sha256_hex),
            etag: response.etag,
            version_id: Some(version_id),
        })
    }

    async fn head_object(
        &self,
        request: HeadKnowledgeObjectRequest,
    ) -> Result<KnowledgeObjectRef, KnowledgeStorageError> {
        if let Some(storage_provider_id) = request.storage_provider_id.as_deref() {
            if storage_provider_id != self.storage_provider_id {
                return Err(KnowledgeStorageError::InvalidRequest(format!(
                    "storage_provider_id does not match adapter provider: {storage_provider_id}"
                )));
            }
        }
        let logical_path = request
            .logical_path
            .clone()
            .unwrap_or_else(|| request.object_key.clone());
        let locator = if request.bucket.is_empty() {
            self.locator_for(&logical_path, request.space_uuid.as_deref())?
        } else {
            DriveObjectLocator {
                bucket: request.bucket,
                object_key: request.object_key,
            }
        };
        let response = self
            .store
            .head_object(HeadObjectRequest { locator })
            .await
            .map_err(map_drive_error)?;
        let version_id = content_version_id_from_head(
            response.version_id,
            response.checksum_sha256_hex.as_deref(),
        );

        Ok(KnowledgeObjectRef {
            storage_provider_id: self.storage_provider_id.clone(),
            bucket: response.locator.bucket,
            object_key: response.locator.object_key,
            logical_path,
            object_role: request.object_role,
            content_type: response
                .content_type
                .unwrap_or_else(|| "application/octet-stream".to_string()),
            size_bytes: response.content_length,
            checksum_sha256_hex: response.checksum_sha256_hex,
            etag: response.etag,
            version_id,
        })
    }

    async fn delete_object(
        &self,
        object_ref: &KnowledgeObjectRef,
    ) -> Result<(), KnowledgeStorageError> {
        if object_ref.storage_provider_id != self.storage_provider_id {
            return Err(KnowledgeStorageError::InvalidRequest(
                "storage_provider_id does not match adapter provider".to_string(),
            ));
        }
        match self
            .store
            .delete_object(DeleteObjectRequest {
                locator: DriveObjectLocator {
                    bucket: object_ref.bucket.clone(),
                    object_key: object_ref.object_key.clone(),
                },
            })
            .await
        {
            Ok(_) => Ok(()),
            Err(error) if error.kind == DriveObjectStoreErrorKind::NotFound => Ok(()),
            Err(error) => Err(map_drive_error(error)),
        }
    }

    async fn get_object_text(
        &self,
        object_ref: &KnowledgeObjectRef,
    ) -> Result<String, KnowledgeStorageError> {
        self.get_object_text_bounded(object_ref, DEFAULT_MAX_KNOWLEDGE_OBJECT_READ_BYTES)
            .await
    }

    async fn get_object_bytes(
        &self,
        object_ref: &KnowledgeObjectRef,
    ) -> Result<Vec<u8>, KnowledgeStorageError> {
        self.get_object_bytes_bounded(object_ref, DEFAULT_MAX_KNOWLEDGE_OBJECT_READ_BYTES)
            .await
    }

    async fn get_object_bytes_bounded(
        &self,
        object_ref: &KnowledgeObjectRef,
        max_bytes: u64,
    ) -> Result<Vec<u8>, KnowledgeStorageError> {
        validate_object_read_size(object_ref, max_bytes)?;
        if object_ref.size_bytes == 0 {
            return Ok(Vec::new());
        }

        let end_inclusive = object_ref.size_bytes.saturating_sub(1);
        let (_, mut stream) = self
            .store
            .read_object_range(ReadObjectRangeRequest {
                locator: DriveObjectLocator {
                    bucket: object_ref.bucket.clone(),
                    object_key: object_ref.object_key.clone(),
                },
                range: DriveByteRange {
                    start_inclusive: 0,
                    end_inclusive,
                },
            })
            .await
            .map_err(map_drive_error)?;

        let initial_capacity = usize::try_from(object_ref.size_bytes).unwrap_or(0);
        let mut bytes = Vec::with_capacity(initial_capacity);
        while let Some(chunk) = stream.next_chunk().await.map_err(map_drive_error)? {
            let next_size = bytes.len().saturating_add(chunk.len()) as u64;
            if next_size > max_bytes {
                return Err(object_read_limit_error(next_size, max_bytes));
            }
            bytes.extend_from_slice(&chunk);
        }

        Ok(bytes)
    }
}

#[async_trait]
impl KnowledgeDriveWorkspace for KnowledgebaseDriveWorkspaceAdapter {
    async fn ensure_nodes(
        &self,
        request: EnsureKnowledgeDriveNodesRequest,
    ) -> Result<(), KnowledgeDriveWorkspaceError> {
        let drive_space_id = safe_drive_id(&request.drive_space_id, "drive_space_id")?;
        if request.nodes.is_empty() {
            return Ok(());
        }

        let nodes = request
            .nodes
            .into_iter()
            .map(knowledge_node_to_drive_node)
            .collect::<Result<Vec<_>, _>>()?;
        self.workspace_service()
            .ensure_nodes(EnsureDriveWorkspaceNodesCommand {
                tenant_id: self.tenant_id.clone(),
                space_id: drive_space_id,
                operator_id: self.operator_id.clone(),
                nodes,
            })
            .await
            .map_err(map_workspace_service_error)
    }
}

impl KnowledgebaseDriveWorkspaceAdapter {
    fn workspace_service(&self) -> SqlDriveWorkspaceService {
        SqlDriveWorkspaceService::new(self.pool.clone())
    }
}

#[async_trait]
impl KnowledgeDriveNodeTree for KnowledgebaseDriveNodeTreeAdapter {
    async fn resolve_path(
        &self,
        request: ResolveKnowledgeDriveNodePathRequest,
    ) -> Result<Option<KnowledgeDriveNodeSummary>, KnowledgeDriveNodeTreeError> {
        let drive_space_id = safe_drive_id(&request.drive_space_id, "drive_space_id")
            .map_err(|error| KnowledgeDriveNodeTreeError::InvalidRequest(error.to_string()))?;
        let logical_path = safe_logical_path(&request.logical_path)
            .map_err(|error| KnowledgeDriveNodeTreeError::InvalidRequest(error.to_string()))?;
        let node = self
            .workspace_service()
            .resolve_path(ResolveDriveWorkspacePathCommand {
                tenant_id: self.tenant_id.clone(),
                space_id: drive_space_id,
                logical_path,
            })
            .await
            .map_err(map_tree_service_error)?;
        let Some(node) = node else {
            return Ok(None);
        };
        let locator = self
            .resolve_object_locator(&node.id)
            .await
            .map_err(map_tree_service_error)?;
        Ok(Some(knowledge_summary_from_drive_node(node, locator)?))
    }

    async fn get_node(
        &self,
        request: GetKnowledgeDriveNodeRequest,
    ) -> Result<Option<KnowledgeDriveNodeSummary>, KnowledgeDriveNodeTreeError> {
        let drive_space_id = safe_drive_id(&request.drive_space_id, "drive_space_id")
            .map_err(|error| KnowledgeDriveNodeTreeError::InvalidRequest(error.to_string()))?;
        let drive_node_id = safe_drive_id(&request.drive_node_id, "drive_node_id")
            .map_err(|error| KnowledgeDriveNodeTreeError::InvalidRequest(error.to_string()))?;
        let node = self
            .workspace_service()
            .get_node(GetDriveWorkspaceNodeCommand {
                tenant_id: self.tenant_id.clone(),
                space_id: drive_space_id,
                node_id: drive_node_id,
            })
            .await
            .map_err(map_tree_service_error)?;
        let Some(node) = node else {
            return Ok(None);
        };
        let locator = self
            .resolve_object_locator(&node.id)
            .await
            .map_err(map_tree_service_error)?;
        Ok(Some(knowledge_summary_from_drive_node(node, locator)?))
    }

    async fn list_children(
        &self,
        request: ListKnowledgeDriveNodeChildrenRequest,
    ) -> Result<KnowledgeDriveNodePage, KnowledgeDriveNodeTreeError> {
        let drive_space_id = safe_drive_id(&request.drive_space_id, "drive_space_id")
            .map_err(|error| KnowledgeDriveNodeTreeError::InvalidRequest(error.to_string()))?;
        let page_size = request.page_size.clamp(1, 200);
        let offset = decode_cursor(request.cursor.as_deref())?;

        let page = self
            .workspace_service()
            .list_children(ListDriveWorkspaceChildrenCommand {
                tenant_id: self.tenant_id.clone(),
                space_id: drive_space_id,
                parent_node_id: request.parent_drive_node_id,
                offset,
                page_size: i64::from(page_size),
            })
            .await
            .map_err(map_tree_service_error)?;
        let file_node_ids = page
            .nodes
            .iter()
            .filter(|node| node.kind == DriveWorkspaceNodeKind::File)
            .map(|node| node.id.clone())
            .collect::<Vec<_>>();
        let locators = self
            .batch_resolve_object_locators(&file_node_ids)
            .await
            .map_err(map_tree_service_error)?;
        knowledge_page_from_drive_page(page, &locators)
    }
}

impl KnowledgebaseDriveNodeTreeAdapter {
    fn workspace_service(&self) -> SqlDriveWorkspaceService {
        SqlDriveWorkspaceService::new(self.pool.clone())
    }

    async fn resolve_object_locator(
        &self,
        node_id: &str,
    ) -> Result<Option<KnowledgeDriveNodeObjectLocator>, DriveServiceError> {
        let locators = self
            .batch_resolve_object_locators(&[node_id.to_string()])
            .await?;
        Ok(locators.get(node_id).cloned())
    }

    async fn batch_resolve_object_locators(
        &self,
        node_ids: &[String],
    ) -> Result<HashMap<String, KnowledgeDriveNodeObjectLocator>, DriveServiceError> {
        if node_ids.is_empty() {
            return Ok(HashMap::new());
        }

        let workspace = self.workspace_service();
        let mut locators = HashMap::with_capacity(node_ids.len());
        for node_id in node_ids {
            let object = workspace
                .find_latest_active_storage_object_by_node(&self.tenant_id, node_id)
                .await?;
            let Some(object) = object else {
                continue;
            };
            locators.insert(
                node_id.clone(),
                KnowledgeDriveNodeObjectLocator {
                    storage_provider_id: object.storage_provider_id,
                    bucket: object.bucket,
                    object_key: object.object_key,
                },
            );
        }
        Ok(locators)
    }
}

fn safe_logical_path(value: &str) -> Result<String, KnowledgeStorageError> {
    let trimmed = value.trim();
    if trimmed.is_empty()
        || trimmed.starts_with('/')
        || trimmed.starts_with('\\')
        || trimmed.contains(':')
    {
        return Err(KnowledgeStorageError::InvalidRequest(format!(
            "unsafe logical_path: {value}"
        )));
    }

    let normalized = trimmed.replace('\\', "/");
    let mut segments = Vec::new();
    for segment in normalized.split('/') {
        if segment.is_empty()
            || segment == "."
            || segment == ".."
            || !segment
                .chars()
                .all(|ch| ch.is_ascii_alphanumeric() || ch == '-' || ch == '_' || ch == '.')
        {
            return Err(KnowledgeStorageError::InvalidRequest(format!(
                "unsafe logical_path: {value}"
            )));
        }
        segments.push(segment);
    }

    Ok(segments.join("/"))
}

fn map_drive_error(error: DriveObjectStoreError) -> KnowledgeStorageError {
    match error.kind {
        DriveObjectStoreErrorKind::NotFound => KnowledgeStorageError::NotFound(error.message),
        DriveObjectStoreErrorKind::InvalidRequest => {
            KnowledgeStorageError::InvalidRequest(error.message)
        }
        DriveObjectStoreErrorKind::IntegrityFailed => {
            KnowledgeStorageError::IntegrityFailed(error.message)
        }
        DriveObjectStoreErrorKind::PermissionDenied => {
            KnowledgeStorageError::PermissionDenied(error.message)
        }
        DriveObjectStoreErrorKind::Conflict => KnowledgeStorageError::Conflict(error.message),
        // Timeout / RateLimited / Unavailable are transient provider states with
        // no definitive Knowledge semantics; UpstreamError is already generic.
        // They stay collapsed so callers retry them instead of misclassifying.
        DriveObjectStoreErrorKind::Timeout
        | DriveObjectStoreErrorKind::Unavailable
        | DriveObjectStoreErrorKind::RateLimited
        | DriveObjectStoreErrorKind::UpstreamError
        | DriveObjectStoreErrorKind::NotSupported => KnowledgeStorageError::Upstream(error.message),
        DriveObjectStoreErrorKind::Internal => KnowledgeStorageError::Internal(error.message),
    }
}

fn safe_drive_id(value: &str, field_name: &str) -> Result<String, KnowledgeDriveWorkspaceError> {
    safe_drive_identifier(value, field_name).map_err(KnowledgeDriveWorkspaceError::InvalidRequest)
}

fn require_non_empty(
    value: String,
    field_name: &str,
) -> Result<String, KnowledgeDriveSpaceProvisionerError> {
    let value = value.trim().to_string();
    if value.is_empty() {
        return Err(KnowledgeDriveSpaceProvisionerError::InvalidRequest(
            format!("{field_name} is required"),
        ));
    }
    Ok(value)
}

fn require_drive_owner_subject_type(
    value: &str,
) -> Result<String, KnowledgeDriveSpaceProvisionerError> {
    let value = value.trim();
    match value {
        "app" | "user" | "group" | "organization" => Ok(value.to_string()),
        _ => Err(KnowledgeDriveSpaceProvisionerError::InvalidRequest(
            "owner_subject_type must be app, user, group, or organization".to_string(),
        )),
    }
}

fn safe_drive_owner_subject_id(value: &str) -> Result<String, KnowledgeDriveSpaceProvisionerError> {
    let value = value.trim();
    if value.is_empty()
        || value.len() > 128
        || !value.chars().all(|ch| {
            ch.is_ascii_alphanumeric() || ch == '-' || ch == '_' || ch == '.' || ch == ':'
        })
    {
        return Err(KnowledgeDriveSpaceProvisionerError::InvalidRequest(
            "invalid owner_subject_id".to_string(),
        ));
    }
    Ok(value.to_string())
}

fn drive_space_id_for_knowledge_space(
    knowledge_space_uuid: &str,
) -> Result<String, KnowledgeDriveSpaceProvisionerError> {
    let safe_uuid = safe_drive_identifier(knowledge_space_uuid, "knowledge_space_uuid")
        .map_err(KnowledgeDriveSpaceProvisionerError::InvalidRequest)?;
    let drive_space_id = format!("kb-{safe_uuid}");
    if drive_space_id.len() > 64 {
        return Err(KnowledgeDriveSpaceProvisionerError::InvalidRequest(
            "knowledge_space_uuid is too long for drive space id".to_string(),
        ));
    }
    Ok(drive_space_id)
}

fn safe_drive_identifier(value: &str, field_name: &str) -> Result<String, String> {
    let value = value.trim();
    if value.is_empty()
        || value.len() > 128
        || !value
            .chars()
            .all(|ch| ch.is_ascii_alphanumeric() || ch == '-' || ch == '_' || ch == '.')
    {
        return Err(format!("invalid {field_name}"));
    }
    Ok(value.to_string())
}

fn knowledge_node_to_drive_node(
    node: EnsureKnowledgeDriveNodeRequest,
) -> Result<EnsureDriveWorkspaceNode, KnowledgeDriveWorkspaceError> {
    let logical_path = safe_logical_path(&node.logical_path)
        .map_err(|error| KnowledgeDriveWorkspaceError::InvalidRequest(error.to_string()))?;
    match node.kind {
        EnsureKnowledgeDriveNodeKind::Folder => Ok(EnsureDriveWorkspaceNode::folder(logical_path)),
        EnsureKnowledgeDriveNodeKind::File => {
            let object_ref = node.object_ref.ok_or_else(|| {
                KnowledgeDriveWorkspaceError::InvalidRequest(format!(
                    "object_ref is required for file node: {logical_path}"
                ))
            })?;
            let content_length = i64::try_from(object_ref.size_bytes).map_err(|_| {
                KnowledgeDriveWorkspaceError::InvalidRequest(format!(
                    "size_bytes is out of range for drive file object: {}",
                    object_ref.logical_path
                ))
            })?;
            let checksum_sha256_hex = object_ref.checksum_sha256_hex.ok_or_else(|| {
                KnowledgeDriveWorkspaceError::InvalidRequest(format!(
                    "checksum_sha256_hex is required for drive file object: {}",
                    object_ref.logical_path
                ))
            })?;
            Ok(EnsureDriveWorkspaceNode::file(
                logical_path,
                DriveWorkspaceObjectRef {
                    storage_provider_id: object_ref.storage_provider_id,
                    bucket: object_ref.bucket,
                    object_key: object_ref.object_key,
                    content_type: object_ref.content_type,
                    content_length,
                    checksum_sha256_hex,
                },
            ))
        }
    }
}

fn knowledge_summary_from_drive_node(
    node: DriveWorkspaceNode,
    object_locator: Option<KnowledgeDriveNodeObjectLocator>,
) -> Result<KnowledgeDriveNodeSummary, KnowledgeDriveNodeTreeError> {
    let kind = match node.kind {
        DriveWorkspaceNodeKind::Folder => DriveNodeKind::Folder,
        DriveWorkspaceNodeKind::File => DriveNodeKind::File,
    };
    let size_bytes = node
        .content_length
        .map(|value| {
            u64::try_from(value).map_err(|_| {
                KnowledgeDriveNodeTreeError::Internal(format!(
                    "drive content_length must be non-negative: {value}"
                ))
            })
        })
        .transpose()?;
    let children_count = u64::try_from(node.children_count).map_err(|_| {
        KnowledgeDriveNodeTreeError::Internal(format!(
            "drive children_count must be non-negative: {}",
            node.children_count
        ))
    })?;
    Ok(KnowledgeDriveNodeSummary {
        drive_node_id: node.id,
        parent_drive_node_id: node.parent_node_id,
        kind,
        name: node.name,
        path: node.path,
        content_type: node.content_type,
        size_bytes,
        children_count: Some(children_count),
        updated_at: node.updated_at,
        object_locator,
    })
}

fn knowledge_page_from_drive_page(
    page: DriveWorkspaceChildrenPage,
    locators: &HashMap<String, KnowledgeDriveNodeObjectLocator>,
) -> Result<KnowledgeDriveNodePage, KnowledgeDriveNodeTreeError> {
    let nodes = page
        .nodes
        .into_iter()
        .map(|node| {
            let locator = locators.get(&node.id).cloned();
            knowledge_summary_from_drive_node(node, locator)
        })
        .collect::<Result<Vec<_>, _>>()?;
    Ok(KnowledgeDriveNodePage {
        nodes,
        next_cursor: page.next_offset.map(|offset| offset.to_string()),
    })
}

fn decode_cursor(cursor: Option<&str>) -> Result<i64, KnowledgeDriveNodeTreeError> {
    let Some(cursor) = cursor else {
        return Ok(0);
    };
    let offset = cursor.trim().parse::<i64>().map_err(|_| {
        KnowledgeDriveNodeTreeError::InvalidRequest("cursor must be a numeric offset".to_string())
    })?;
    if offset < 0 {
        return Err(KnowledgeDriveNodeTreeError::InvalidRequest(
            "cursor must be non-negative".to_string(),
        ));
    }
    Ok(offset)
}

fn verified_request_checksum(
    request_checksum: Option<&str>,
    computed_checksum: &str,
) -> Result<String, KnowledgeStorageError> {
    let Some(request_checksum) = request_checksum else {
        return Ok(computed_checksum.to_string());
    };
    let normalized = normalize_sha256_hex(request_checksum)?;
    if normalized != computed_checksum {
        return Err(KnowledgeStorageError::IntegrityFailed(
            "checksum_sha256_hex does not match request body".to_string(),
        ));
    }
    Ok(normalized)
}

fn normalize_sha256_hex(value: &str) -> Result<String, KnowledgeStorageError> {
    let checksum = value.trim().to_ascii_lowercase();
    let checksum = checksum.strip_prefix("sha256:").unwrap_or(&checksum);
    if checksum.len() != 64 || !checksum.bytes().all(|byte| byte.is_ascii_hexdigit()) {
        return Err(KnowledgeStorageError::InvalidRequest(
            "checksum_sha256_hex must be a 64-character hex SHA-256 digest".to_string(),
        ));
    }
    Ok(checksum.to_string())
}

fn content_version_id(provider_version_id: Option<String>, checksum_sha256_hex: &str) -> String {
    normalized_provider_version_id(provider_version_id)
        .unwrap_or_else(|| synthetic_content_version_id(checksum_sha256_hex))
}

fn content_version_id_from_head(
    provider_version_id: Option<String>,
    checksum_sha256_hex: Option<&str>,
) -> Option<String> {
    normalized_provider_version_id(provider_version_id).or_else(|| {
        checksum_sha256_hex
            .filter(|checksum| !is_blank(Some(checksum)))
            .map(synthetic_content_version_id)
    })
}

fn normalized_provider_version_id(provider_version_id: Option<String>) -> Option<String> {
    provider_version_id
        .map(|version_id| version_id.trim().to_string())
        .filter(|version_id| !version_id.is_empty())
}

fn synthetic_content_version_id(checksum_sha256_hex: &str) -> String {
    let checksum = checksum_sha256_hex.trim();
    if checksum.starts_with("sha256:") {
        checksum.to_string()
    } else {
        format!("sha256:{checksum}")
    }
}

fn map_workspace_service_error(error: DriveServiceError) -> KnowledgeDriveWorkspaceError {
    match error {
        DriveServiceError::Validation(message) => {
            KnowledgeDriveWorkspaceError::InvalidRequest(message)
        }
        DriveServiceError::Conflict(message) => KnowledgeDriveWorkspaceError::Conflict(message),
        DriveServiceError::NotFound(message) => KnowledgeDriveWorkspaceError::NotFound(message),
        DriveServiceError::PermissionDenied(message) => {
            KnowledgeDriveWorkspaceError::PermissionDenied(message)
        }
        DriveServiceError::Internal(message) => KnowledgeDriveWorkspaceError::Internal(message),
    }
}

fn map_space_service_error(error: DriveServiceError) -> KnowledgeDriveSpaceProvisionerError {
    match error {
        DriveServiceError::Validation(message) => {
            KnowledgeDriveSpaceProvisionerError::InvalidRequest(message)
        }
        DriveServiceError::Conflict(message) => {
            KnowledgeDriveSpaceProvisionerError::Conflict(message)
        }
        DriveServiceError::NotFound(message) => {
            KnowledgeDriveSpaceProvisionerError::NotFound(message)
        }
        DriveServiceError::PermissionDenied(message) => {
            KnowledgeDriveSpaceProvisionerError::PermissionDenied(message)
        }
        DriveServiceError::Internal(message) => {
            KnowledgeDriveSpaceProvisionerError::Internal(message)
        }
    }
}

fn map_tree_service_error(error: DriveServiceError) -> KnowledgeDriveNodeTreeError {
    match error {
        DriveServiceError::Validation(message) => {
            KnowledgeDriveNodeTreeError::InvalidRequest(message)
        }
        DriveServiceError::Conflict(message) => KnowledgeDriveNodeTreeError::Conflict(message),
        DriveServiceError::NotFound(message) => KnowledgeDriveNodeTreeError::NotFound(message),
        DriveServiceError::PermissionDenied(message) => {
            KnowledgeDriveNodeTreeError::PermissionDenied(message)
        }
        DriveServiceError::Internal(message) => KnowledgeDriveNodeTreeError::Internal(message),
    }
}
