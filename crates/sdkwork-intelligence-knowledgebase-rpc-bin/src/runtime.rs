use std::{
    fs::{self, OpenOptions},
    io::{Read, Seek, SeekFrom, Write},
    path::Path,
    sync::Arc,
    time::Duration,
};

use async_trait::async_trait;
use sdkwork_database_config::DatabaseEngine;
use sdkwork_drive_storage_local::LocalDriveObjectStore;
use sdkwork_intelligence_knowledgebase_repository_sqlx::{
    connect_knowledgebase_and_install_schema, database_config_from_url, knowledgebase_health_check,
    knowledgebase_process_pool_budget_from_url, require_postgres_rls_organization_id,
    require_postgres_rls_tenant_id, PostgresGroupKnowledgeSpaceBindingStore,
    PostgresKnowledgeOkfBundleFileStore, PostgresKnowledgeSpaceStore, SqlxWikiPersistenceStore,
};
use sdkwork_intelligence_knowledgebase_rpc::GroupKnowledgeSpaceLifecycleRuntime;
use sdkwork_intelligence_knowledgebase_service::{
    group_space::{
        GroupKnowledgeSpaceOperation, KnowledgeGroupKnowledgeSpaceService,
        KnowledgeGroupKnowledgeSpaceServiceError,
    },
    okf::{OkfBundleFileRegistryService, OkfBundleInitializerService},
    ports::knowledge_group_space_binding_store::{
        GroupKnowledgeSpaceMembershipChange, GroupKnowledgeSpaceScope,
    },
};
use sdkwork_knowledgebase_contract::group_space::{
    ArchiveGroupKnowledgeSpaceRequest, EnsureGroupKnowledgeSpaceRequest,
    GroupKnowledgeSpaceBinding, SynchronizeGroupKnowledgeSpaceMembersRequest,
};
use sdkwork_knowledgebase_drive::{
    connect_knowledgebase_drive_pool_with_max_connections, knowledgebase_drive_health_check,
    resolve_cloud_knowledgebase_drive_storage, KnowledgebaseDriveRootScopeAdapter,
    KnowledgebaseDriveSpaceProvisionerAdapter, KnowledgebaseDriveStorageAdapter,
    KnowledgebaseDriveWorkspaceAdapter, KnowledgebaseKnowledgeAccessControlAdapter,
};
use thiserror::Error;

use crate::config::DriveStorageRuntimeConfig;

const DRIVE_PROVIDER_ID: &str = "sdkwork-knowledgebase-local";
const DRIVE_BUCKET: &str = "knowledgebase";

/// Concrete, process-owned lifecycle runtime. It creates tenant/organization-scoped adapters for
/// each verified command, so no caller scope can leak through a long-lived mutable runtime.
#[derive(Clone)]
pub struct KnowledgebaseGroupKnowledgeSpaceLifecycleRuntime {
    pool: sqlx::AnyPool,
    drive_pool: sqlx::PgPool,
    database_engine: DatabaseEngine,
    drive_storage: Arc<KnowledgebaseDriveStorageAdapter>,
    tenant_id: u64,
    organization_id: u64,
    operator_id: String,
    system_actor_id: u64,
}

impl KnowledgebaseGroupKnowledgeSpaceLifecycleRuntime {
    pub async fn connect(
        database_url: &str,
        drive_storage: DriveStorageRuntimeConfig,
        operator_id: String,
        system_actor_id: u64,
    ) -> Result<Self, KnowledgebaseGroupKnowledgeSpaceLifecycleRuntimeError> {
        let tenant_id = require_postgres_rls_tenant_id()
            .map_err(|_| dependency_unavailable("knowledgebase-tenant-scope"))?;
        let organization_id = require_postgres_rls_organization_id()
            .map_err(|_| dependency_unavailable("knowledgebase-organization-scope"))?;
        let pool = connect_knowledgebase_and_install_schema(database_url)
            .await
            .map_err(|error| dependency_failure("knowledgebase-schema-connect", error))?;
        let database_config = database_config_from_url(database_url)
            .map_err(|_| dependency_unavailable("knowledgebase-database-config"))?;
        let pool_budget = knowledgebase_process_pool_budget_from_url(database_url)
            .map_err(|_| dependency_unavailable("knowledgebase-pool-budget"))?;
        let postgres_max_connections = pool_budget
            .postgres_max_connections
            .ok_or_else(|| dependency_unavailable("knowledgebase-postgres-required"))?;
        let drive_pool = connect_knowledgebase_drive_pool_with_max_connections(
            &database_config.url,
            postgres_max_connections,
        )
        .await
        .map_err(|_| dependency_unavailable("drive-schema-connect"))?;
        knowledgebase_health_check(&pool)
            .await
            .map_err(|_| dependency_unavailable("knowledgebase-health"))?;
        knowledgebase_drive_health_check(&drive_pool)
            .await
            .map_err(|_| dependency_unavailable("drive-health"))?;
        let drive_storage = match drive_storage {
            DriveStorageRuntimeConfig::StandaloneLocal(drive_storage_root) => {
                verify_drive_storage_root(&drive_storage_root)?;
                let adapter = KnowledgebaseDriveStorageAdapter::new(
                    Arc::new(LocalDriveObjectStore::new(drive_storage_root)),
                    DRIVE_PROVIDER_ID,
                    DRIVE_BUCKET,
                    "deployment",
                );
                adapter
                    .ensure_bucket()
                    .await
                    .map_err(|_| dependency_unavailable("local-object-store-bucket"))?;
                adapter
            }
            DriveStorageRuntimeConfig::CloudProvider(provider_id) => {
                resolve_cloud_knowledgebase_drive_storage(
                    drive_pool.clone(),
                    &provider_id,
                    "deployment",
                )
                .await
                .map_err(|_| dependency_unavailable("cloud-object-store-provider"))?
            }
        };
        tokio::time::timeout(Duration::from_secs(5), drive_storage.readiness_check())
            .await
            .map_err(|_| dependency_unavailable("object-store-readiness-timeout"))?
            .map_err(|_| dependency_unavailable("object-store-readiness"))?;

        Ok(Self {
            pool,
            drive_pool,
            database_engine: DatabaseEngine::Postgres,
            drive_storage: Arc::new(drive_storage),
            tenant_id,
            organization_id,
            operator_id,
            system_actor_id,
        })
    }

    pub async fn readiness_check(
        &self,
    ) -> Result<(), KnowledgebaseGroupKnowledgeSpaceLifecycleRuntimeError> {
        knowledgebase_health_check(&self.pool)
            .await
            .map_err(|_| dependency_unavailable("knowledgebase-readiness"))?;
        knowledgebase_drive_health_check(&self.drive_pool)
            .await
            .map_err(|_| dependency_unavailable("drive-readiness"))?;
        tokio::time::timeout(Duration::from_secs(5), self.drive_storage.readiness_check())
            .await
            .map_err(|_| dependency_unavailable("object-store-readiness-timeout"))?
            .map_err(|_| dependency_unavailable("object-store-readiness"))
    }

    fn dependencies_for_scope(
        &self,
        scope: GroupKnowledgeSpaceScope,
    ) -> GroupKnowledgeSpaceLifecycleDependencies {
        let tenant_id = scope.tenant_id.to_string();
        GroupKnowledgeSpaceLifecycleDependencies {
            binding_store: Arc::new(
                PostgresGroupKnowledgeSpaceBindingStore::new(self.pool.clone())
                    .with_database_engine(self.database_engine),
            ),
            space_store: Arc::new(
                PostgresKnowledgeSpaceStore::new(
                    self.pool.clone(),
                    scope.tenant_id,
                    scope.organization_id,
                )
                .with_database_engine(self.database_engine),
            ),
            wiki_store: Arc::new(
                SqlxWikiPersistenceStore::new(self.pool.clone())
                    .with_database_engine(self.database_engine),
            ),
            bundle_file_store: Arc::new(
                PostgresKnowledgeOkfBundleFileStore::new(
                    self.pool.clone(),
                    scope.tenant_id,
                    scope.organization_id,
                )
                .with_database_engine(self.database_engine),
            ),
            drive_storage: Arc::new(self.drive_storage.for_tenant(tenant_id.clone())),
            drive_space_provisioner: Arc::new(KnowledgebaseDriveSpaceProvisionerAdapter::new(
                self.drive_pool.clone(),
            )),
            drive_workspace: Arc::new(KnowledgebaseDriveWorkspaceAdapter::new(
                self.drive_pool.clone(),
                tenant_id,
                self.operator_id.clone(),
            )),
            wiki_drive_scope: Arc::new(KnowledgebaseDriveRootScopeAdapter::new(
                self.drive_pool.clone(),
                scope.tenant_id.to_string(),
                self.operator_id.clone(),
            )),
            access_control: Arc::new(KnowledgebaseKnowledgeAccessControlAdapter::new(
                self.drive_pool.clone(),
            )),
        }
    }

    fn ensure_deployment_scope(
        &self,
        scope: GroupKnowledgeSpaceScope,
    ) -> Result<(), KnowledgeGroupKnowledgeSpaceServiceError> {
        if scope.tenant_id != self.tenant_id || scope.organization_id != self.organization_id {
            return Err(KnowledgeGroupKnowledgeSpaceServiceError::Denied(
                "request scope does not match the fixed Knowledgebase deployment scope".to_string(),
            ));
        }
        Ok(())
    }

    async fn ensure_from_im(
        &self,
        scope: GroupKnowledgeSpaceScope,
        service_actor_id: &str,
        request: EnsureGroupKnowledgeSpaceRequest,
    ) -> Result<GroupKnowledgeSpaceOperation, KnowledgeGroupKnowledgeSpaceServiceError> {
        self.ensure_deployment_scope(scope)?;
        let dependencies = self.dependencies_for_scope(scope);
        let registry = OkfBundleFileRegistryService::new(dependencies.bundle_file_store.as_ref());
        let initializer = OkfBundleInitializerService::new(dependencies.drive_storage.as_ref())
            .with_registry(&registry)
            .with_drive_workspace(dependencies.drive_workspace.as_ref());
        let wiki_initializer =
            sdkwork_intelligence_knowledgebase_service::wiki_initialization::KnowledgeWikiInitializationService::new(
                dependencies.wiki_store.as_ref(),
                dependencies.wiki_store.as_ref(),
                dependencies.wiki_drive_scope.as_ref(),
            );
        let service = KnowledgeGroupKnowledgeSpaceService::new(
            dependencies.binding_store.as_ref(),
            dependencies.space_store.as_ref(),
            &initializer,
            dependencies.drive_space_provisioner.as_ref(),
            dependencies.access_control.as_ref(),
            self.operator_id.clone(),
            Some(self.system_actor_id),
        )
        .with_wiki_initializer(&wiki_initializer);
        service
            .ensure_from_im(scope, service_actor_id, request)
            .await
    }

    async fn synchronize_from_im(
        &self,
        scope: GroupKnowledgeSpaceScope,
        service_actor_id: &str,
        request: SynchronizeGroupKnowledgeSpaceMembersRequest,
    ) -> Result<GroupKnowledgeSpaceMembershipChange, KnowledgeGroupKnowledgeSpaceServiceError> {
        self.ensure_deployment_scope(scope)?;
        let dependencies = self.dependencies_for_scope(scope);
        let registry = OkfBundleFileRegistryService::new(dependencies.bundle_file_store.as_ref());
        let initializer = OkfBundleInitializerService::new(dependencies.drive_storage.as_ref())
            .with_registry(&registry)
            .with_drive_workspace(dependencies.drive_workspace.as_ref());
        let service = KnowledgeGroupKnowledgeSpaceService::new(
            dependencies.binding_store.as_ref(),
            dependencies.space_store.as_ref(),
            &initializer,
            dependencies.drive_space_provisioner.as_ref(),
            dependencies.access_control.as_ref(),
            self.operator_id.clone(),
            Some(self.system_actor_id),
        );
        service
            .synchronize_members_from_im(scope, service_actor_id, request)
            .await
    }

    async fn archive_from_im(
        &self,
        scope: GroupKnowledgeSpaceScope,
        service_actor_id: &str,
        archived_by: &str,
        request: ArchiveGroupKnowledgeSpaceRequest,
    ) -> Result<GroupKnowledgeSpaceBinding, KnowledgeGroupKnowledgeSpaceServiceError> {
        self.ensure_deployment_scope(scope)?;
        let dependencies = self.dependencies_for_scope(scope);
        let registry = OkfBundleFileRegistryService::new(dependencies.bundle_file_store.as_ref());
        let initializer = OkfBundleInitializerService::new(dependencies.drive_storage.as_ref())
            .with_registry(&registry)
            .with_drive_workspace(dependencies.drive_workspace.as_ref());
        let service = KnowledgeGroupKnowledgeSpaceService::new(
            dependencies.binding_store.as_ref(),
            dependencies.space_store.as_ref(),
            &initializer,
            dependencies.drive_space_provisioner.as_ref(),
            dependencies.access_control.as_ref(),
            self.operator_id.clone(),
            Some(self.system_actor_id),
        );
        service
            .archive_from_im(scope, service_actor_id, archived_by, request)
            .await
    }
}

#[async_trait]
impl GroupKnowledgeSpaceLifecycleRuntime for KnowledgebaseGroupKnowledgeSpaceLifecycleRuntime {
    async fn ensure_group_knowledge_space(
        &self,
        scope: GroupKnowledgeSpaceScope,
        actor_id: &str,
        request: EnsureGroupKnowledgeSpaceRequest,
    ) -> Result<GroupKnowledgeSpaceOperation, KnowledgeGroupKnowledgeSpaceServiceError> {
        self.ensure_from_im(scope, actor_id, request).await
    }

    async fn synchronize_group_knowledge_space_members(
        &self,
        scope: GroupKnowledgeSpaceScope,
        actor_id: &str,
        request: SynchronizeGroupKnowledgeSpaceMembersRequest,
    ) -> Result<GroupKnowledgeSpaceMembershipChange, KnowledgeGroupKnowledgeSpaceServiceError> {
        self.synchronize_from_im(scope, actor_id, request).await
    }

    async fn archive_group_knowledge_space(
        &self,
        scope: GroupKnowledgeSpaceScope,
        service_actor_id: &str,
        archived_by: &str,
        request: ArchiveGroupKnowledgeSpaceRequest,
    ) -> Result<GroupKnowledgeSpaceBinding, KnowledgeGroupKnowledgeSpaceServiceError> {
        self.archive_from_im(scope, service_actor_id, archived_by, request)
            .await
    }
}

struct GroupKnowledgeSpaceLifecycleDependencies {
    binding_store: Arc<PostgresGroupKnowledgeSpaceBindingStore>,
    space_store: Arc<PostgresKnowledgeSpaceStore>,
    wiki_store: Arc<SqlxWikiPersistenceStore>,
    bundle_file_store: Arc<PostgresKnowledgeOkfBundleFileStore>,
    drive_storage: Arc<KnowledgebaseDriveStorageAdapter>,
    drive_space_provisioner: Arc<KnowledgebaseDriveSpaceProvisionerAdapter>,
    drive_workspace: Arc<KnowledgebaseDriveWorkspaceAdapter>,
    wiki_drive_scope: Arc<KnowledgebaseDriveRootScopeAdapter>,
    access_control: Arc<KnowledgebaseKnowledgeAccessControlAdapter>,
}

#[derive(Debug, Error)]
pub enum KnowledgebaseGroupKnowledgeSpaceLifecycleRuntimeError {
    #[error("Knowledgebase lifecycle runtime dependency is unavailable at stage `{stage}`")]
    DependencyUnavailable { stage: &'static str },
    #[error("Knowledgebase lifecycle runtime dependency failed at stage `{stage}`: {detail}")]
    DependencyFailure { stage: &'static str, detail: String },
}

fn dependency_unavailable(
    stage: &'static str,
) -> KnowledgebaseGroupKnowledgeSpaceLifecycleRuntimeError {
    KnowledgebaseGroupKnowledgeSpaceLifecycleRuntimeError::DependencyUnavailable { stage }
}

fn dependency_failure(
    stage: &'static str,
    error: impl std::fmt::Display,
) -> KnowledgebaseGroupKnowledgeSpaceLifecycleRuntimeError {
    KnowledgebaseGroupKnowledgeSpaceLifecycleRuntimeError::DependencyFailure {
        stage,
        detail: redact_connection_userinfo(&error.to_string()),
    }
}

fn redact_connection_userinfo(detail: &str) -> String {
    let Some(scheme_end) = detail.find("://") else {
        return detail.to_string();
    };
    let authority_start = scheme_end + 3;
    let Some(authority_end) = detail[authority_start..].find('@') else {
        return detail.to_string();
    };
    let authority_end = authority_start + authority_end;
    let userinfo = &detail[authority_start..authority_end];
    if !userinfo.contains(':') {
        return detail.to_string();
    }
    format!(
        "{}[REDACTED]{}",
        &detail[..authority_start],
        &detail[authority_end..]
    )
}

/// Creates the configured root when needed and verifies that it is a directory usable for Drive
/// object writes before the RPC listener accepts lifecycle commands. The probe is unique, read
/// back, and removed immediately; no Knowledgebase data is created or modified.
fn verify_drive_storage_root(
    drive_storage_root: &Path,
) -> Result<(), KnowledgebaseGroupKnowledgeSpaceLifecycleRuntimeError> {
    match fs::metadata(drive_storage_root) {
        Ok(metadata) if !metadata.is_dir() => {
            return Err(dependency_unavailable("drive-storage-root-not-directory"));
        }
        Ok(_) => {}
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
            fs::create_dir_all(drive_storage_root)
                .map_err(|_| dependency_unavailable("drive-storage-root-create"))?;
        }
        Err(_) => return Err(dependency_unavailable("drive-storage-root-metadata")),
    }

    let probe_path = drive_storage_root.join(format!(
        ".sdkwork-knowledgebase-rpc-preflight-{}",
        sdkwork_utils_rust::uuid()
    ));
    let result = (|| -> std::io::Result<()> {
        let mut probe = OpenOptions::new()
            .create_new(true)
            .read(true)
            .write(true)
            .open(&probe_path)?;
        probe.write_all(b"sdkwork-knowledgebase-rpc-preflight")?;
        probe.sync_all()?;
        probe.seek(SeekFrom::Start(0))?;
        const PROBE_CONTENT: &[u8] = b"sdkwork-knowledgebase-rpc-preflight";
        let mut contents = [0_u8; PROBE_CONTENT.len()];
        probe.read_exact(&mut contents)?;
        let mut trailing = [0_u8; 1];
        if contents != PROBE_CONTENT || probe.read(&mut trailing)? != 0 {
            return Err(std::io::Error::other(
                "drive storage probe readback mismatch",
            ));
        }
        drop(probe);
        fs::remove_file(&probe_path)
    })();

    if result.is_err() {
        let _ = fs::remove_file(&probe_path);
        return Err(dependency_unavailable("drive-storage-root-probe"));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn dependency_diagnostics_redact_database_userinfo() {
        let detail = "failed to parse postgresql://gateway:secret@postgres:5432/sdkwork";

        assert_eq!(
            redact_connection_userinfo(detail),
            "failed to parse postgresql://[REDACTED]@postgres:5432/sdkwork"
        );
        assert_eq!(
            redact_connection_userinfo("database connection refused"),
            "database connection refused"
        );
    }

    #[test]
    fn drive_storage_preflight_creates_a_writable_root_without_leaving_a_probe() {
        let temporary = tempfile::tempdir().expect("temporary directory");
        let root = temporary.path().join("drive-storage");

        verify_drive_storage_root(&root).expect("writable drive root");

        assert!(root.is_dir());
        assert!(
            fs::read_dir(&root)
                .expect("read drive root")
                .next()
                .is_none(),
            "the readiness probe must not leave runtime data behind"
        );
    }

    #[test]
    fn drive_storage_preflight_rejects_a_file_instead_of_a_directory() {
        let temporary = tempfile::tempdir().expect("temporary directory");
        let root = temporary.path().join("not-a-directory");
        fs::write(&root, b"not a directory").expect("fixture file");

        assert!(matches!(
            verify_drive_storage_root(&root),
            Err(
                KnowledgebaseGroupKnowledgeSpaceLifecycleRuntimeError::DependencyUnavailable {
                    stage: "drive-storage-root-not-directory"
                }
            )
        ));
    }
}
