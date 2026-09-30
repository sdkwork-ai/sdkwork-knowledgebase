use sdkwork_drive_object_runtime::DriveObjectStoreRuntime;
use sdkwork_drive_storage_contract::{
    DriveObjectStoreError, DriveObjectStoreErrorKind, DriveStorageProviderKind, HeadBucketRequest,
};
use sdkwork_drive_workspace_service::infrastructure::sql::storage_provider_store::SqlStorageProviderStore;
use sdkwork_drive_workspace_service::ports::storage_provider_store::DriveStorageProviderStore;
use sqlx::PgPool;

use crate::KnowledgebaseDriveStorageAdapter;

pub async fn resolve_cloud_knowledgebase_drive_storage(
    pool: PgPool,
    provider_id: &str,
    tenant_id: &str,
) -> Result<KnowledgebaseDriveStorageAdapter, DriveObjectStoreError> {
    let provider_id = provider_id.trim();
    if provider_id.is_empty() {
        return Err(configuration_error(
            "cloud Knowledgebase storage provider id must not be empty",
        ));
    }

    let provider = SqlStorageProviderStore::new(pool.clone())
        .find_storage_provider(provider_id)
        .await
        .map_err(|_| {
            DriveObjectStoreError::new(
                DriveObjectStoreErrorKind::Internal,
                "cloud Knowledgebase storage provider lookup failed",
            )
        })?
        .ok_or_else(|| {
            DriveObjectStoreError::new(
                DriveObjectStoreErrorKind::NotFound,
                "cloud Knowledgebase storage provider was not found",
            )
        })?;
    if provider.status != "active" {
        return Err(DriveObjectStoreError::new(
            DriveObjectStoreErrorKind::Conflict,
            "cloud Knowledgebase storage provider is not active",
        ));
    }
    if provider.provider_kind == DriveStorageProviderKind::LocalFilesystem {
        return Err(configuration_error(
            "cloud Knowledgebase storage must use a shared object-storage provider",
        ));
    }

    // Cloud Knowledgebase storage is the tenant's active write surface; the
    // Write intent requires an active provider (also enforced explicitly above).
    let object_store = DriveObjectStoreRuntime::new(pool)
        .resolve(
            &provider.id,
            sdkwork_drive_object_runtime::ProviderAccessIntent::Write,
        )
        .await?;
    let bucket_health = object_store
        .head_bucket(HeadBucketRequest {
            bucket: provider.bucket.clone(),
        })
        .await?;
    if !bucket_health.exists {
        return Err(DriveObjectStoreError::new(
            DriveObjectStoreErrorKind::NotFound,
            "cloud Knowledgebase storage bucket was not found",
        ));
    }
    Ok(KnowledgebaseDriveStorageAdapter::from_object_store(
        object_store,
        provider.id,
        provider.bucket,
        tenant_id,
    ))
}

fn configuration_error(message: &'static str) -> DriveObjectStoreError {
    DriveObjectStoreError::new(DriveObjectStoreErrorKind::InvalidRequest, message)
}

#[cfg(test)]
mod tests {
    use super::configuration_error;
    use sdkwork_drive_storage_contract::DriveObjectStoreErrorKind;

    #[test]
    fn configuration_errors_are_typed() {
        let error = configuration_error("cloud storage configuration is invalid");
        assert_eq!(error.kind, DriveObjectStoreErrorKind::InvalidRequest);
        assert_eq!(error.message, "cloud storage configuration is invalid");
    }
}
