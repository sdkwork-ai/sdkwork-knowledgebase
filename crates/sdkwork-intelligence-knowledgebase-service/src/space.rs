use crate::okf::DRIVE_WORKSPACE_INIT_DRIVE_SPACE_REQUIRED;
use crate::okf::{OkfBundleInitializerService, OkfBundleInitializerServiceError};
use crate::ports::knowledge_wiki_persistence::WikiPersistenceScope;
use crate::ports::{
    knowledge_access_control::{
        KnowledgeAccessControl, KnowledgeAccessControlError, KnowledgeAccessRole,
        KnowledgeSubjectType,
    },
    knowledge_drive_space::{
        CreateKnowledgeDriveSpaceRequest, DeleteKnowledgeDriveSpaceRequest,
        KnowledgeDriveSpaceProvisioner, KnowledgeDriveSpaceProvisionerError,
    },
    knowledge_space_store::{
        BindKnowledgeDriveSpaceRecord, CreateKnowledgeSpaceRecord, KnowledgeSpaceStore,
        KnowledgeSpaceStoreError, UpdateKnowledgeSpaceRecord,
    },
};
use crate::wiki_initialization::{
    InitializeKnowledgeWikiRequest, KnowledgeWikiInitializationService,
};
use sdkwork_knowledgebase_contract::rag::KnowledgeAgentKnowledgeMode;
use sdkwork_knowledgebase_contract::space::{
    CreateKnowledgeSpaceRequest, KnowledgeSpace, UpdateKnowledgeSpaceRequest,
};
use sdkwork_utils_rust::{is_blank, DEFAULT_LIST_PAGE_SIZE, MAX_LIST_PAGE_SIZE};
use thiserror::Error;

fn normalize_space_members_page_size(
    page_size: Option<u32>,
) -> Result<u32, KnowledgeSpaceServiceError> {
    let page_size = page_size.unwrap_or(DEFAULT_LIST_PAGE_SIZE as u32);
    if !(1..=MAX_LIST_PAGE_SIZE as u32).contains(&page_size) {
        return Err(KnowledgeSpaceServiceError::InvalidRequest(format!(
            "page_size must be between 1 and {MAX_LIST_PAGE_SIZE}"
        )));
    }
    Ok(page_size)
}

fn knowledge_drive_space_owner(knowledge_space_uuid: &str) -> (String, String) {
    (
        "app".to_string(),
        format!("sdkwork-knowledgebase:{}", knowledge_space_uuid.trim()),
    )
}

pub struct KnowledgeSpaceService<'a> {
    store: &'a dyn KnowledgeSpaceStore,
    okf_bundle_initializer: &'a OkfBundleInitializerService<'a>,
    drive_space_provisioner: Option<&'a dyn KnowledgeDriveSpaceProvisioner>,
    access_control: Option<&'a dyn KnowledgeAccessControl>,
    drive_context: Option<KnowledgeSpaceDriveContext>,
    wiki_context: Option<KnowledgeWikiContext>,
    wiki_initializer: Option<&'a KnowledgeWikiInitializationService<'a>>,
}

impl<'a> KnowledgeSpaceService<'a> {
    pub fn new(
        store: &'a dyn KnowledgeSpaceStore,
        okf_bundle_initializer: &'a OkfBundleInitializerService<'a>,
    ) -> Self {
        Self {
            store,
            okf_bundle_initializer,
            drive_space_provisioner: None,
            access_control: None,
            drive_context: None,
            wiki_context: None,
            wiki_initializer: None,
        }
    }

    pub fn with_drive_context(
        mut self,
        tenant_id: impl Into<String>,
        operator_id: impl Into<String>,
    ) -> Self {
        self.drive_context = Some(KnowledgeSpaceDriveContext {
            tenant_id: tenant_id.into().trim().to_string(),
            operator_id: operator_id.into().trim().to_string(),
        });
        self
    }

    pub fn with_drive_space_provisioner(
        mut self,
        drive_space_provisioner: &'a dyn KnowledgeDriveSpaceProvisioner,
    ) -> Self {
        self.drive_space_provisioner = Some(drive_space_provisioner);
        self
    }

    pub fn with_access_control(mut self, access_control: &'a dyn KnowledgeAccessControl) -> Self {
        self.access_control = Some(access_control);
        self
    }

    pub fn with_wiki_context(mut self, scope: WikiPersistenceScope, actor_id: u64) -> Self {
        self.wiki_context = Some(KnowledgeWikiContext { scope, actor_id });
        self
    }

    pub fn with_wiki_initializer(
        mut self,
        wiki_initializer: &'a KnowledgeWikiInitializationService<'a>,
    ) -> Self {
        self.wiki_initializer = Some(wiki_initializer);
        self
    }

    pub async fn create_space(
        &self,
        request: CreateKnowledgeSpaceRequest,
    ) -> Result<KnowledgeSpace, KnowledgeSpaceServiceError> {
        if is_blank(Some(request.name.as_str())) {
            return Err(KnowledgeSpaceServiceError::InvalidRequest(
                "name is required".to_string(),
            ));
        }

        let owner_subject_type = request
            .owner_subject_type
            .unwrap_or_else(|| "user".to_string());
        let owner_subject_id = request
            .owner_subject_id
            .unwrap_or_else(|| "unknown".to_string());

        let drive_context = if self.drive_space_provisioner.is_some() {
            Some(self.require_drive_context()?.clone())
        } else {
            if self.okf_bundle_initializer.requires_drive_space_binding() {
                return Err(KnowledgeSpaceServiceError::InvalidRequest(
                    DRIVE_WORKSPACE_INIT_DRIVE_SPACE_REQUIRED.to_string(),
                ));
            }
            None
        };

        let space = self
            .store
            .create_space(CreateKnowledgeSpaceRecord {
                name: request.name,
                description: request.description,
                okf_bundle_initialized: false,
                knowledge_mode: request.knowledge_mode,
            })
            .await
            .map_err(KnowledgeSpaceServiceError::Store)?;

        let space_id = space.id;
        let space = match self
            .initialize_created_space(
                space,
                drive_context.as_ref(),
                &owner_subject_type,
                &owner_subject_id,
            )
            .await
        {
            Ok(space) => space,
            Err(error) => return Err(self.cleanup_created_space(space_id, error).await),
        };

        Ok(space)
    }

    /// Initializes a pre-reserved group-managed space. The reservation record is intentionally
    /// hidden from generic routes until the group aggregate finishes Drive, OKF, and ACL setup.
    pub async fn initialize_group_managed_space(
        &self,
        space_id: u64,
        owner_subject_type: &str,
        owner_subject_id: &str,
    ) -> Result<KnowledgeSpace, KnowledgeSpaceServiceError> {
        if is_blank(Some(owner_subject_type)) || is_blank(Some(owner_subject_id)) {
            return Err(KnowledgeSpaceServiceError::InvalidRequest(
                "group-managed space owner subject is required".to_string(),
            ));
        }
        let drive_context = if self.drive_space_provisioner.is_some() {
            Some(self.require_drive_context()?.clone())
        } else {
            if self.okf_bundle_initializer.requires_drive_space_binding() {
                return Err(KnowledgeSpaceServiceError::InvalidRequest(
                    DRIVE_WORKSPACE_INIT_DRIVE_SPACE_REQUIRED.to_string(),
                ));
            }
            None
        };
        let space = self
            .store
            .get_group_provisioning_space(space_id)
            .await
            .map_err(KnowledgeSpaceServiceError::Store)?;
        match self
            .initialize_created_space(
                space,
                drive_context.as_ref(),
                owner_subject_type,
                owner_subject_id,
            )
            .await
        {
            Ok(space) => Ok(space),
            Err(error) => Err(self.cleanup_created_space(space_id, error).await),
        }
    }

    /// The group aggregate calls this only after its direct-user ACL projection has succeeded.
    pub async fn activate_group_managed_space(
        &self,
        space_id: u64,
    ) -> Result<KnowledgeSpace, KnowledgeSpaceServiceError> {
        self.store
            .activate_group_managed_space(space_id)
            .await
            .map_err(KnowledgeSpaceServiceError::Store)
    }

    pub async fn update_space(
        &self,
        space_id: u64,
        tenant_id: &str,
        actor_id: &str,
        request: UpdateKnowledgeSpaceRequest,
    ) -> Result<KnowledgeSpace, KnowledgeSpaceServiceError> {
        self.get_space_with_role_check(space_id, tenant_id, actor_id, KnowledgeAccessRole::Owner)
            .await?;

        if request
            .name
            .as_ref()
            .is_some_and(|name| is_blank(Some(name.as_str())))
        {
            return Err(KnowledgeSpaceServiceError::InvalidRequest(
                "name must not be blank".to_string(),
            ));
        }

        if request.name.is_none() && request.description.is_none() {
            return Err(KnowledgeSpaceServiceError::InvalidRequest(
                "at least one of name or description is required".to_string(),
            ));
        }

        self.store
            .update_space(
                space_id,
                UpdateKnowledgeSpaceRecord {
                    name: request.name,
                    description: request.description,
                },
            )
            .await
            .map_err(KnowledgeSpaceServiceError::Store)
    }

    /// Updates the description of a group-managed space after the caller has already completed
    /// IM snapshot and projected Drive owner authorization. The narrower signature prevents an
    /// App API caller from changing IM-owned group names through the generic space update path.
    pub async fn update_group_managed_space_description(
        &self,
        space_id: u64,
        description: String,
    ) -> Result<KnowledgeSpace, KnowledgeSpaceServiceError> {
        self.store
            .update_group_managed_space_description(space_id, description)
            .await
            .map_err(KnowledgeSpaceServiceError::Store)
    }

    pub async fn delete_space(
        &self,
        space_id: u64,
        tenant_id: &str,
        actor_id: &str,
    ) -> Result<(), KnowledgeSpaceServiceError> {
        self.get_space_with_role_check(space_id, tenant_id, actor_id, KnowledgeAccessRole::Owner)
            .await?;
        self.store
            .mark_space_deleted(space_id)
            .await
            .map_err(KnowledgeSpaceServiceError::Store)
    }

    pub async fn get_space_with_access_check(
        &self,
        space_id: u64,
        tenant_id: &str,
        actor_id: &str,
    ) -> Result<KnowledgeSpace, KnowledgeSpaceServiceError> {
        self.get_space_with_role_check(space_id, tenant_id, actor_id, KnowledgeAccessRole::Reader)
            .await
    }

    pub async fn get_space_with_role_check(
        &self,
        space_id: u64,
        tenant_id: &str,
        actor_id: &str,
        required_role: KnowledgeAccessRole,
    ) -> Result<KnowledgeSpace, KnowledgeSpaceServiceError> {
        let space = self
            .store
            .get_space(space_id)
            .await
            .map_err(KnowledgeSpaceServiceError::Store)?;

        // Read paths are fail-closed: without a configured access control the operation is
        // rejected instead of silently degrading to tenant-scoped visibility only.
        let access = self.require_access_control()?;
        let drive_space_id = space.drive_space_id.as_ref().ok_or_else(|| {
            KnowledgeSpaceServiceError::AccessDenied(format!(
                "space {space_id} is not bound to a drive space for access control"
            ))
        })?;
        let grant = access
            .check_space_access(
                crate::ports::knowledge_access_control::KnowledgeAccessCheckRequest {
                    tenant_id: tenant_id.to_string(),
                    actor_id: actor_id.to_string(),
                    drive_space_id: drive_space_id.clone(),
                    required_role,
                },
            )
            .await
            .map_err(KnowledgeSpaceServiceError::AccessControl)?;
        if !grant.allowed {
            return Err(KnowledgeSpaceServiceError::AccessDenied(format!(
                "actor {actor_id} does not have access to space {space_id}"
            )));
        }

        Ok(space)
    }

    pub async fn grant_space_member(
        &self,
        space_id: u64,
        tenant_id: &str,
        subject_type: KnowledgeSubjectType,
        subject_id: &str,
        role: KnowledgeAccessRole,
        operator_id: &str,
    ) -> Result<(), KnowledgeSpaceServiceError> {
        self.get_space_with_role_check(
            space_id,
            tenant_id,
            operator_id,
            KnowledgeAccessRole::Owner,
        )
        .await?;

        let space = self
            .store
            .get_space(space_id)
            .await
            .map_err(KnowledgeSpaceServiceError::Store)?;

        let drive_space_id = space.drive_space_id.as_ref().ok_or_else(|| {
            KnowledgeSpaceServiceError::InvalidRequest(
                "space is not bound to a drive space".to_string(),
            )
        })?;

        let access = self.require_access_control()?;
        access
            .grant_space_access(
                crate::ports::knowledge_access_control::GrantKnowledgeSpaceAccessRequest {
                    tenant_id: tenant_id.to_string(),
                    drive_space_id: drive_space_id.clone(),
                    drive_node_id: None,
                    subject_type,
                    subject_id: subject_id.to_string(),
                    role,
                    operator_id: operator_id.to_string(),
                },
            )
            .await
            .map_err(KnowledgeSpaceServiceError::AccessControl)?;

        Ok(())
    }

    pub async fn revoke_space_member(
        &self,
        space_id: u64,
        tenant_id: &str,
        subject_type: KnowledgeSubjectType,
        subject_id: &str,
        operator_id: &str,
    ) -> Result<(), KnowledgeSpaceServiceError> {
        self.get_space_with_role_check(
            space_id,
            tenant_id,
            operator_id,
            KnowledgeAccessRole::Owner,
        )
        .await?;

        let space = self
            .store
            .get_space(space_id)
            .await
            .map_err(KnowledgeSpaceServiceError::Store)?;

        let drive_space_id = space.drive_space_id.as_ref().ok_or_else(|| {
            KnowledgeSpaceServiceError::InvalidRequest(
                "space is not bound to a drive space".to_string(),
            )
        })?;

        let access = self.require_access_control()?;
        access
            .revoke_space_access(
                crate::ports::knowledge_access_control::RevokeKnowledgeSpaceAccessRequest {
                    tenant_id: tenant_id.to_string(),
                    drive_space_id: drive_space_id.clone(),
                    drive_node_id: None,
                    subject_type,
                    subject_id: subject_id.to_string(),
                    operator_id: operator_id.to_string(),
                },
            )
            .await
            .map_err(KnowledgeSpaceServiceError::AccessControl)?;

        Ok(())
    }

    pub async fn list_space_members(
        &self,
        space_id: u64,
        tenant_id: &str,
        actor_id: &str,
        cursor: Option<String>,
        page_size: Option<u32>,
    ) -> Result<
        crate::ports::knowledge_access_control::KnowledgeSpaceMemberList,
        KnowledgeSpaceServiceError,
    > {
        self.get_space_with_access_check(space_id, tenant_id, actor_id)
            .await?;

        let space = self
            .store
            .get_space(space_id)
            .await
            .map_err(KnowledgeSpaceServiceError::Store)?;

        let drive_space_id = space.drive_space_id.as_ref().ok_or_else(|| {
            KnowledgeSpaceServiceError::InvalidRequest(
                "space is not bound to a drive space".to_string(),
            )
        })?;

        let access = self.require_access_control()?;
        let members = access
            .list_space_members(
                crate::ports::knowledge_access_control::ListKnowledgeSpaceMembersRequest {
                    tenant_id: tenant_id.to_string(),
                    drive_space_id: drive_space_id.clone(),
                    drive_node_id: None,
                    cursor,
                    page_size: Some(normalize_space_members_page_size(page_size)?),
                },
            )
            .await
            .map_err(KnowledgeSpaceServiceError::AccessControl)?;

        Ok(members)
    }

    pub async fn list_space_members_admin(
        &self,
        space_id: u64,
        tenant_id: &str,
        cursor: Option<String>,
        page_size: Option<u32>,
    ) -> Result<
        crate::ports::knowledge_access_control::KnowledgeSpaceMemberList,
        KnowledgeSpaceServiceError,
    > {
        let space = self
            .store
            .get_space(space_id)
            .await
            .map_err(KnowledgeSpaceServiceError::Store)?;

        let drive_space_id = space.drive_space_id.as_ref().ok_or_else(|| {
            KnowledgeSpaceServiceError::InvalidRequest(
                "space is not bound to a drive space".to_string(),
            )
        })?;

        let access = self.require_access_control()?;
        access
            .list_space_members(
                crate::ports::knowledge_access_control::ListKnowledgeSpaceMembersRequest {
                    tenant_id: tenant_id.to_string(),
                    drive_space_id: drive_space_id.clone(),
                    drive_node_id: None,
                    cursor,
                    page_size: Some(normalize_space_members_page_size(page_size)?),
                },
            )
            .await
            .map_err(KnowledgeSpaceServiceError::AccessControl)
    }

    async fn initialize_created_space(
        &self,
        mut space: KnowledgeSpace,
        drive_context: Option<&KnowledgeSpaceDriveContext>,
        owner_subject_type: &str,
        owner_subject_id: &str,
    ) -> Result<KnowledgeSpace, KnowledgeSpaceServiceError> {
        let mut drive_cleanup = None;
        if let Some(provisioner) = self.drive_space_provisioner {
            let drive_context = drive_context.ok_or_else(|| {
                KnowledgeSpaceServiceError::InvalidRequest(
                    "drive tenant_id and operator_id are required when drive space provisioning is enabled"
                        .to_string(),
                )
            })?;
            let wiki_context = self.require_wiki_context()?;
            if space.drive_space_id.is_none() {
                let (drive_owner_subject_type, drive_owner_subject_id) =
                    knowledge_drive_space_owner(&space.uuid);
                let binding = provisioner
                    .create_knowledge_drive_space(CreateKnowledgeDriveSpaceRequest {
                        tenant_id: drive_context.tenant_id.clone(),
                        knowledge_space_id: space.id,
                        knowledge_space_uuid: space.uuid.clone(),
                        display_name: space.name.clone(),
                        owner_subject_type: drive_owner_subject_type.clone(),
                        owner_subject_id: drive_owner_subject_id.clone(),
                        operator_id: drive_context.operator_id.clone(),
                    })
                    .await?;
                drive_cleanup = Some(DeleteKnowledgeDriveSpaceRequest {
                    tenant_id: drive_context.tenant_id.clone(),
                    drive_space_id: binding.drive_space_id.clone(),
                    owner_subject_type: drive_owner_subject_type,
                    owner_subject_id: drive_owner_subject_id,
                    operator_id: drive_context.operator_id.clone(),
                });

                space = match self
                    .store
                    .mark_drive_space_bound(
                        space.id,
                        BindKnowledgeDriveSpaceRecord {
                            drive_space_id: binding.drive_space_id,
                            actor_id: wiki_context.actor_id,
                        },
                    )
                    .await
                    .map_err(KnowledgeSpaceServiceError::Store)
                {
                    Ok(space) => space,
                    Err(error) => {
                        return Err(self
                            .cleanup_created_drive_space(drive_cleanup.as_ref(), error)
                            .await)
                    }
                };
            }
        }

        if space.knowledge_mode == KnowledgeAgentKnowledgeMode::OkfBundle {
            if let Err(error) = self
                .okf_bundle_initializer
                .initialize_standard_files(space.id, &space.name, space.drive_space_id.as_deref())
                .await
                .map_err(KnowledgeSpaceServiceError::OkfBundleInitializer)
            {
                return Err(self
                    .cleanup_created_drive_space(drive_cleanup.as_ref(), error)
                    .await);
            }

            space = match self
                .store
                .mark_okf_bundle_initialized(space.id)
                .await
                .map_err(KnowledgeSpaceServiceError::Store)
            {
                Ok(space) => space,
                Err(error) => {
                    return Err(self
                        .cleanup_created_drive_space(drive_cleanup.as_ref(), error)
                        .await)
                }
            };
        } else if self.okf_bundle_initializer.requires_drive_space_binding() {
            if let Err(error) = self
                .okf_bundle_initializer
                .ensure_drive_permission_anchor(space.drive_space_id.as_deref())
                .await
                .map_err(KnowledgeSpaceServiceError::OkfBundleInitializer)
            {
                return Err(self
                    .cleanup_created_drive_space(drive_cleanup.as_ref(), error)
                    .await);
            }
        }

        if let Err(error) = self
            .grant_created_space_owner_access(
                &space,
                drive_context,
                owner_subject_type,
                owner_subject_id,
            )
            .await
        {
            let error = self
                .cleanup_created_drive_space(drive_cleanup.as_ref(), error)
                .await;
            return Err(self.cleanup_created_space(space.id, error).await);
        }

        self.try_initialize_wiki(&space).await;

        Ok(space)
    }

    async fn try_initialize_wiki(&self, space: &KnowledgeSpace) {
        let (Some(initializer), Some(context), Some(drive_space_uuid)) = (
            self.wiki_initializer,
            self.wiki_context.as_ref(),
            space.drive_space_id.as_ref(),
        ) else {
            return;
        };
        if let Err(error) = initializer
            .initialize(InitializeKnowledgeWikiRequest {
                scope: context.scope,
                space_id: space.id,
                knowledgebase_uuid: space.uuid.clone(),
                drive_space_uuid: drive_space_uuid.clone(),
                actor_id: context.actor_id,
            })
            .await
        {
            tracing::warn!(
                knowledge_space_id = space.id,
                error = %error,
                "Wiki publication initialization remains pending for bounded compensation"
            );
        }
    }

    async fn grant_created_space_owner_access(
        &self,
        space: &KnowledgeSpace,
        drive_context: Option<&KnowledgeSpaceDriveContext>,
        owner_subject_type: &str,
        owner_subject_id: &str,
    ) -> Result<(), KnowledgeSpaceServiceError> {
        let Some(access) = self.access_control else {
            return Ok(());
        };
        let Some(drive_context) = drive_context else {
            return Ok(());
        };
        let Some(drive_space_id) = space.drive_space_id.as_deref() else {
            return Ok(());
        };

        let subject_type = KnowledgeSubjectType::from_drive_subject_type(owner_subject_type)
            .unwrap_or(KnowledgeSubjectType::User);
        access
            .grant_space_access(
                crate::ports::knowledge_access_control::GrantKnowledgeSpaceAccessRequest {
                    tenant_id: drive_context.tenant_id.clone(),
                    drive_space_id: drive_space_id.to_string(),
                    drive_node_id: None,
                    subject_type,
                    subject_id: owner_subject_id.to_string(),
                    role: KnowledgeAccessRole::Owner,
                    operator_id: drive_context.operator_id.clone(),
                },
            )
            .await
            .map_err(KnowledgeSpaceServiceError::AccessControl)
    }

    async fn cleanup_created_drive_space(
        &self,
        request: Option<&DeleteKnowledgeDriveSpaceRequest>,
        error: KnowledgeSpaceServiceError,
    ) -> KnowledgeSpaceServiceError {
        let Some(request) = request else {
            return error;
        };
        let Some(provisioner) = self.drive_space_provisioner else {
            return error;
        };
        match provisioner
            .delete_knowledge_drive_space(request.clone())
            .await
        {
            Ok(()) => error,
            Err(cleanup) => KnowledgeSpaceServiceError::DriveSpaceCleanup {
                original: error.to_string(),
                cleanup,
            },
        }
    }

    async fn cleanup_created_space(
        &self,
        space_id: u64,
        error: KnowledgeSpaceServiceError,
    ) -> KnowledgeSpaceServiceError {
        match self.store.mark_space_deleted(space_id).await {
            Ok(()) => error,
            Err(cleanup) => KnowledgeSpaceServiceError::InitializationCleanup {
                original: error.to_string(),
                cleanup,
            },
        }
    }

    fn require_drive_context(
        &self,
    ) -> Result<&KnowledgeSpaceDriveContext, KnowledgeSpaceServiceError> {
        let Some(context) = self.drive_context.as_ref() else {
            return Err(KnowledgeSpaceServiceError::InvalidRequest(
                "drive tenant_id and operator_id are required when drive space provisioning is enabled"
                    .to_string(),
            ));
        };
        if is_blank(Some(context.tenant_id.as_str())) {
            return Err(KnowledgeSpaceServiceError::InvalidRequest(
                "drive tenant_id is required".to_string(),
            ));
        }
        if is_blank(Some(context.operator_id.as_str())) {
            return Err(KnowledgeSpaceServiceError::InvalidRequest(
                "drive operator_id is required".to_string(),
            ));
        }
        Ok(context)
    }

    fn require_wiki_context(&self) -> Result<&KnowledgeWikiContext, KnowledgeSpaceServiceError> {
        let context = self.wiki_context.as_ref().ok_or_else(|| {
            KnowledgeSpaceServiceError::InvalidRequest(
                "Wiki tenant, organization, and numeric actor context are required when Drive Space provisioning is enabled"
                    .to_string(),
            )
        })?;
        if context.scope.tenant_id == 0 || context.actor_id == 0 {
            return Err(KnowledgeSpaceServiceError::InvalidRequest(
                "Wiki tenant_id and actor_id must be greater than zero".to_string(),
            ));
        }
        Ok(context)
    }

    fn require_access_control(
        &self,
    ) -> Result<&dyn KnowledgeAccessControl, KnowledgeSpaceServiceError> {
        self.access_control.ok_or_else(|| {
            KnowledgeSpaceServiceError::InvalidRequest(
                "access control is required for this operation".to_string(),
            )
        })
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct KnowledgeSpaceDriveContext {
    pub tenant_id: String,
    pub operator_id: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct KnowledgeWikiContext {
    pub scope: WikiPersistenceScope,
    pub actor_id: u64,
}

#[derive(Debug, Error)]
pub enum KnowledgeSpaceServiceError {
    #[error("invalid knowledge space request: {0}")]
    InvalidRequest(String),
    #[error("knowledge space access denied: {0}")]
    AccessDenied(String),
    #[error(transparent)]
    Store(#[from] KnowledgeSpaceStoreError),
    #[error(transparent)]
    OkfBundleInitializer(#[from] OkfBundleInitializerServiceError),
    #[error(transparent)]
    DriveSpaceProvisioner(#[from] KnowledgeDriveSpaceProvisionerError),
    #[error(transparent)]
    AccessControl(#[from] KnowledgeAccessControlError),
    #[error("knowledge space initialization failed: {original}; cleanup failed: {cleanup}")]
    InitializationCleanup {
        original: String,
        cleanup: KnowledgeSpaceStoreError,
    },
    #[error("knowledge space initialization failed: {original}; drive cleanup failed: {cleanup}")]
    DriveSpaceCleanup {
        original: String,
        cleanup: KnowledgeDriveSpaceProvisionerError,
    },
}
