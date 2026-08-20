use async_trait::async_trait;
use axum::http::StatusCode;
use sdkwork_intelligence_knowledgebase_service::{
    group_launch::{GroupKnowledgebaseLaunchResolver, GroupKnowledgebaseLaunchResolverError},
    ports::group_launch_ticket_consumer::{
        GroupLaunchTicketCallerContext, GroupLaunchTicketConsumerError,
    },
};
use sdkwork_knowledgebase_contract::group_space::{
    ConsumeGroupKnowledgebaseLaunchTicketRequest, GroupKnowledgeSpacePrincipalKind,
    GroupKnowledgebaseLaunchTarget,
};
use sdkwork_utils_rust::is_blank;

use crate::{
    hosted_access::{ensure_runtime_tenant, require_actor_id, require_space_access},
    ApiError, ApiResult, KnowledgeAppRequestContext, KnowledgeGroupLaunchAppService,
    KnowledgebaseRuntime,
};

pub(crate) struct HostedGroupLaunchService {
    runtime: KnowledgebaseRuntime,
}

impl HostedGroupLaunchService {
    pub(crate) fn new(runtime: KnowledgebaseRuntime) -> Self {
        Self { runtime }
    }
}

#[async_trait]
impl KnowledgeGroupLaunchAppService for HostedGroupLaunchService {
    async fn consume_group_launch_ticket(
        &self,
        context: KnowledgeAppRequestContext,
        request: ConsumeGroupKnowledgebaseLaunchTicketRequest,
    ) -> ApiResult<GroupKnowledgebaseLaunchTarget> {
        ensure_runtime_tenant(&self.runtime, &context)?;
        let actor_id = require_actor_id(&context)?;
        let session_id = context
            .session_id
            .clone()
            .filter(|value| !is_blank(Some(value.as_str())))
            .ok_or_else(|| {
                ApiError::new(
                    StatusCode::UNAUTHORIZED,
                    "missing_group_launch_session",
                    "an authenticated session is required to consume a group launch ticket",
                )
            })?;
        let organization_id = require_group_launch_organization_id(&context)?;
        let trace_id = require_group_launch_trace_id(&context)?;
        let idempotency_key = require_group_launch_idempotency_key(&context)?;
        let consumer = self.runtime.group_launch_ticket_consumer().ok_or_else(|| {
            ApiError::new(
                StatusCode::SERVICE_UNAVAILABLE,
                "group_launch_ticket_consumer_unavailable",
                "group launch is temporarily unavailable",
            )
        })?;
        let target = GroupKnowledgebaseLaunchResolver::new(
            consumer,
            self.runtime.group_space_binding_store(),
        )
        .consume(
            GroupLaunchTicketCallerContext {
                tenant_id: context.tenant_id,
                organization_id,
                principal_kind: GroupKnowledgeSpacePrincipalKind::User,
                actor_id,
                session_id: Some(session_id),
                request_id: context.request_id.clone(),
                trace_id,
                idempotency_key,
            },
            request,
        )
        .await
        .map_err(map_group_launch_error)?;

        // Launch resolution is not itself content access. Re-run the normal group snapshot plus
        // direct Drive check before returning the exact space target to the client.
        require_space_access(&self.runtime, &context, target.space_id).await?;
        Ok(target)
    }
}

fn require_group_launch_organization_id(context: &KnowledgeAppRequestContext) -> ApiResult<u64> {
    context
        .organization_id
        .filter(|organization_id| *organization_id != 0)
        .ok_or_else(|| {
            ApiError::new(
                StatusCode::FORBIDDEN,
                "group_launch_organization_required",
                "an active organization is required to consume a group launch ticket",
            )
        })
}

fn require_group_launch_trace_id(context: &KnowledgeAppRequestContext) -> ApiResult<String> {
    context
        .trace_id
        .as_deref()
        .filter(|value| !is_blank(Some(value)))
        .map(str::to_owned)
        .ok_or_else(|| {
            ApiError::new(
                StatusCode::SERVICE_UNAVAILABLE,
                "group_launch_request_correlation_unavailable",
                "group launch is temporarily unavailable",
            )
        })
}

fn require_group_launch_idempotency_key(context: &KnowledgeAppRequestContext) -> ApiResult<String> {
    context
        .idempotency_key
        .as_deref()
        .filter(|value| !is_blank(Some(value)))
        .map(str::to_owned)
        .ok_or_else(|| {
            ApiError::new(
                StatusCode::BAD_REQUEST,
                "group_launch_idempotency_key_required",
                "Idempotency-Key header is required to consume a group launch ticket",
            )
        })
}

fn map_group_launch_error(error: GroupKnowledgebaseLaunchResolverError) -> ApiError {
    match error {
        GroupKnowledgebaseLaunchResolverError::InvalidRequest(detail) => {
            ApiError::invalid_request("invalid_group_launch_request", detail)
        }
        GroupKnowledgebaseLaunchResolverError::Denied(_) => ApiError::new(
            StatusCode::FORBIDDEN,
            "group_launch_access_denied",
            "group launch access is denied",
        ),
        GroupKnowledgebaseLaunchResolverError::InvalidBinding(detail) => {
            ApiError::internal("group_launch_binding_invalid", detail)
        }
        GroupKnowledgebaseLaunchResolverError::Ticket(ticket_error) => {
            map_group_launch_ticket_error(ticket_error)
        }
        GroupKnowledgebaseLaunchResolverError::Binding(error) => error.into(),
        GroupKnowledgebaseLaunchResolverError::Authorization(error) => error.into(),
    }
}

fn map_group_launch_ticket_error(error: GroupLaunchTicketConsumerError) -> ApiError {
    match error {
        GroupLaunchTicketConsumerError::InvalidOrExpired
        | GroupLaunchTicketConsumerError::Replayed => ApiError::new(
            StatusCode::UNAUTHORIZED,
            "group_launch_ticket_invalid",
            "group launch ticket is invalid or expired",
        ),
        GroupLaunchTicketConsumerError::Unauthorized => ApiError::new(
            StatusCode::FORBIDDEN,
            "group_launch_access_denied",
            "group launch access is denied",
        ),
        GroupLaunchTicketConsumerError::Unavailable => ApiError::new(
            StatusCode::SERVICE_UNAVAILABLE,
            "group_launch_ticket_consumer_unavailable",
            "group launch is temporarily unavailable",
        ),
        GroupLaunchTicketConsumerError::Upstream(_) => ApiError::internal(
            "group_launch_ticket_consumer_failed",
            "group launch ticket validation could not be completed",
        ),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn context() -> KnowledgeAppRequestContext {
        KnowledgeAppRequestContext {
            tenant_id: 1,
            actor_id: Some(7),
            organization_id: Some(2),
            session_id: Some("session-1".to_string()),
            request_id: "request-1".to_string(),
            trace_id: Some("trace-1".to_string()),
            idempotency_key: Some("idempotency-1".to_string()),
        }
    }

    #[test]
    fn requires_framework_normalized_group_launch_correlation() {
        let mut missing_key = context();
        missing_key.idempotency_key = None;
        let error = require_group_launch_idempotency_key(&missing_key)
            .expect_err("group launch must require an idempotency key");
        assert!(format!("{error:?}").contains("group_launch_idempotency_key_required"));

        let mut missing_trace = context();
        missing_trace.trace_id = Some(" ".to_string());
        let error = require_group_launch_trace_id(&missing_trace)
            .expect_err("group launch must require a framework trace");
        assert!(format!("{error:?}").contains("group_launch_request_correlation_unavailable"));

        assert_eq!(
            require_group_launch_idempotency_key(&context()).expect("key"),
            "idempotency-1"
        );
        assert_eq!(
            require_group_launch_trace_id(&context()).expect("trace"),
            "trace-1"
        );
    }

    #[test]
    fn requires_nonzero_organization_for_group_launches() {
        let mut missing_organization = context();
        missing_organization.organization_id = Some(0);
        let error = require_group_launch_organization_id(&missing_organization)
            .expect_err("group launch must be organization scoped");
        assert!(format!("{error:?}").contains("group_launch_organization_required"));
    }
}
