use async_trait::async_trait;
use sdkwork_drive_workspace_service::{
    application::root_scope_subscription_service::{
        DriveRootScopeSubscriptionService, EnsureKnowledgebaseRawScopeCommand,
        GetRootScopeSubscriptionCommand, SqlDriveKnowledgebaseRawScopeService,
    },
    domain::root_scope_subscription::DriveRootScopeSubscription,
    infrastructure::sql::root_scope_subscription_store::SqlRootScopeSubscriptionStore,
    DriveServiceError,
};
use sdkwork_intelligence_knowledgebase_service::ports::knowledge_wiki_drive_source::{
    EnsureKnowledgebaseRawScopeRequest, KnowledgeWikiDriveEventDeliveryMode,
    KnowledgeWikiDriveScope, KnowledgeWikiDriveSourceError, KnowledgebaseRawScope,
    KnowledgebaseRawScopeEventDelivery, RenewKnowledgebaseRawScopeEventDeliveryRequest,
    KNOWLEDGEBASE_RAW_CONSUMER_KIND,
};
use sqlx::PgPool;

use crate::embedded_knowledgebase_raw_channel_id;

#[derive(Clone)]
pub struct KnowledgebaseDriveRootScopeAdapter {
    pool: PgPool,
    tenant_id: String,
    operator_id: String,
}

impl KnowledgebaseDriveRootScopeAdapter {
    pub fn new(pool: PgPool, tenant_id: impl Into<String>, operator_id: impl Into<String>) -> Self {
        Self {
            pool,
            tenant_id: tenant_id.into().trim().to_string(),
            operator_id: operator_id.into().trim().to_string(),
        }
    }

    fn service(&self) -> DriveRootScopeSubscriptionService<SqlRootScopeSubscriptionStore> {
        DriveRootScopeSubscriptionService::new(SqlRootScopeSubscriptionStore::new(
            self.pool.clone(),
        ))
    }
}

#[async_trait]
impl KnowledgeWikiDriveScope for KnowledgebaseDriveRootScopeAdapter {
    async fn ensure_raw_scope(
        &self,
        request: EnsureKnowledgebaseRawScopeRequest,
    ) -> Result<KnowledgebaseRawScope, KnowledgeWikiDriveSourceError> {
        let result = SqlDriveKnowledgebaseRawScopeService::new(self.pool.clone())
            .ensure_knowledgebase_raw_scope(EnsureKnowledgebaseRawScopeCommand {
                tenant_id: self.tenant_id.clone(),
                space_id: request.drive_space_id,
                knowledge_base_id: request.knowledgebase_uuid,
                operator_id: self.operator_id.clone(),
            })
            .await
            .map_err(map_drive_error)?;
        map_subscription(result.subscription)
    }

    async fn retrieve_raw_scope(
        &self,
        subscription_uuid: &str,
    ) -> Result<KnowledgebaseRawScope, KnowledgeWikiDriveSourceError> {
        let subscription = self
            .service()
            .get_subscription(GetRootScopeSubscriptionCommand {
                tenant_id: self.tenant_id.clone(),
                subscription_uuid: subscription_uuid.trim().to_string(),
            })
            .await
            .map_err(map_drive_error)?;
        map_subscription(subscription)
    }

    async fn renew_raw_scope_event_delivery(
        &self,
        request: RenewKnowledgebaseRawScopeEventDeliveryRequest,
    ) -> Result<KnowledgebaseRawScopeEventDelivery, KnowledgeWikiDriveSourceError> {
        let subscription_uuid = request.subscription_uuid.trim();
        if subscription_uuid.is_empty() {
            return Err(KnowledgeWikiDriveSourceError::InvalidRequest(
                "subscription_uuid is required for embedded event delivery renewal".to_string(),
            ));
        }
        let scope = self.retrieve_raw_scope(subscription_uuid).await?;
        Ok(KnowledgebaseRawScopeEventDelivery {
            subscription_uuid: scope.subscription_uuid,
            channel_id: embedded_knowledgebase_raw_channel_id(subscription_uuid),
            expiration_epoch_ms: None,
            mode: KnowledgeWikiDriveEventDeliveryMode::EmbeddedTrustedRelay,
        })
    }
}

fn map_subscription(
    subscription: DriveRootScopeSubscription,
) -> Result<KnowledgebaseRawScope, KnowledgeWikiDriveSourceError> {
    // Drive retrieves a subscription by uuid without filtering its consumer kind,
    // so the adapter keeps the guard the deleted Internal API transport applied:
    // a root scope registered for another consumer must never enter the Wiki.
    if !subscription
        .consumer_kind
        .eq_ignore_ascii_case(KNOWLEDGEBASE_RAW_CONSUMER_KIND)
    {
        return Err(KnowledgeWikiDriveSourceError::IntegrityFailed(format!(
            "Drive root scope consumer kind must be {KNOWLEDGEBASE_RAW_CONSUMER_KIND}"
        )));
    }
    let version = u64::try_from(subscription.version).map_err(|_| {
        KnowledgeWikiDriveSourceError::IntegrityFailed(
            "Drive root scope version must be nonnegative".to_string(),
        )
    })?;
    Ok(KnowledgebaseRawScope {
        subscription_uuid: subscription.uuid,
        drive_space_id: subscription.space_id,
        consumer_kind: KNOWLEDGEBASE_RAW_CONSUMER_KIND.to_string(),
        knowledgebase_uuid: subscription.consumer_resource_id,
        raw_folder_node_id: subscription.root_node_id,
        scope_status: subscription.scope_status,
        version: version.to_string(),
        created_at: subscription.created_at,
        updated_at: subscription.updated_at,
    })
}

fn map_drive_error(error: DriveServiceError) -> KnowledgeWikiDriveSourceError {
    match error {
        DriveServiceError::Validation(detail) => {
            KnowledgeWikiDriveSourceError::InvalidRequest(detail)
        }
        DriveServiceError::NotFound(detail) => KnowledgeWikiDriveSourceError::NotFound(detail),
        DriveServiceError::Conflict(detail) => KnowledgeWikiDriveSourceError::Conflict(detail),
        DriveServiceError::PermissionDenied(_) => KnowledgeWikiDriveSourceError::Upstream(
            "Drive denied the embedded Knowledgebase scope operation".to_string(),
        ),
        DriveServiceError::Internal(_) => KnowledgeWikiDriveSourceError::Upstream(
            "Drive could not complete the embedded Knowledgebase scope operation".to_string(),
        ),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn subscription(consumer_kind: &str) -> DriveRootScopeSubscription {
        DriveRootScopeSubscription {
            id: "1".to_string(),
            uuid: "scope-uuid".to_string(),
            tenant_id: "1".to_string(),
            space_id: "space-id".to_string(),
            consumer_kind: consumer_kind.to_string(),
            consumer_resource_id: "knowledgebase-uuid".to_string(),
            root_node_id: "raw-folder-node".to_string(),
            scope_status: "ACTIVE".to_string(),
            version: 4,
            created_at: "2026-11-11T00:00:00Z".to_string(),
            updated_at: "2026-11-11T00:00:00Z".to_string(),
        }
    }

    /// The deleted Internal API transport rejected a root scope registered for a
    /// different consumer. Drive's uuid lookup does not filter on consumer kind,
    /// so the adapter must keep that guard on both ensure and retrieve.
    #[test]
    fn root_scope_must_belong_to_the_canonical_knowledgebase_consumer() {
        let scope = map_subscription(subscription(KNOWLEDGEBASE_RAW_CONSUMER_KIND))
            .expect("canonical knowledgebase raw scope maps");
        assert_eq!(scope.consumer_kind, KNOWLEDGEBASE_RAW_CONSUMER_KIND);
        assert_eq!(scope.version, "4");

        for foreign in ["drive_webhook", "workspace_publication", ""] {
            assert_eq!(
                map_subscription(subscription(foreign))
                    .expect_err("a foreign consumer kind must fail integrity")
                    .code(),
                "wiki_drive_source_integrity_failed"
            );
        }
    }
}
