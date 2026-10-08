use crate::ports::group_launch_ticket_consumer::{
    ConsumeGroupLaunchTicketCommand, ConsumedGroupLaunchTicket, GroupLaunchTicketCallerContext,
    GroupLaunchTicketConsumer, GroupLaunchTicketConsumerError,
};
use crate::ports::knowledge_group_space_binding_store::{
    GroupKnowledgeSpaceScope, KnowledgeGroupSpaceBindingStore,
    KnowledgeGroupSpaceBindingStoreError,
};
use sdkwork_knowledgebase_contract::group_space::{
    is_valid_group_knowledgebase_launch_ticket, ConsumeGroupKnowledgebaseLaunchTicketRequest,
    GroupKnowledgeSpaceAclProjectionState, GroupKnowledgeSpaceLifecycleState,
    GroupKnowledgeSpaceMemberRole, GroupKnowledgebaseLaunchTarget,
};
use sdkwork_utils_rust::is_blank;
use thiserror::Error;

/// Resolves an opaque, one-time IM launch ticket to exactly one synchronized group KB space.
/// There is intentionally no default/personal-space fallback on any validation failure.
pub struct GroupKnowledgebaseLaunchResolver<'a> {
    ticket_consumer: &'a dyn GroupLaunchTicketConsumer,
    binding_store: &'a dyn KnowledgeGroupSpaceBindingStore,
}

impl<'a> GroupKnowledgebaseLaunchResolver<'a> {
    pub fn new(
        ticket_consumer: &'a dyn GroupLaunchTicketConsumer,
        binding_store: &'a dyn KnowledgeGroupSpaceBindingStore,
    ) -> Self {
        Self {
            ticket_consumer,
            binding_store,
        }
    }

    pub async fn consume(
        &self,
        caller: GroupLaunchTicketCallerContext,
        request: ConsumeGroupKnowledgebaseLaunchTicketRequest,
    ) -> Result<GroupKnowledgebaseLaunchTarget, GroupKnowledgebaseLaunchResolverError> {
        // Pre-consumption validation keeps its own error codes: the remote ticket is untouched
        // here, so a corrected request may still consume the very same ticket.
        if is_blank(Some(request.ticket.as_str())) {
            return Err(GroupKnowledgebaseLaunchResolverError::InvalidRequest(
                "group launch ticket is required".to_string(),
            ));
        }
        if !is_valid_group_knowledgebase_launch_ticket(request.ticket.as_str()) {
            return Err(GroupKnowledgebaseLaunchResolverError::InvalidRequest(
                "group launch ticket has an invalid format".to_string(),
            ));
        }
        let consumed = self
            .ticket_consumer
            .consume_group_launch_ticket(ConsumeGroupLaunchTicketCommand {
                ticket: request.ticket,
                caller: caller.clone(),
            })
            .await?;

        // Everything past this point runs after IM irreversibly burned the one-time ticket, so
        // any failure is terminal for this ticket: it is surfaced as `TicketConsumed` so the
        // route can tell the client to re-issue instead of inviting a doomed replay.
        self.resolve_consumed_target(caller, consumed)
            .await
            .map_err(|error| {
                GroupKnowledgebaseLaunchResolverError::TicketConsumed(error.to_string())
            })
    }

    async fn resolve_consumed_target(
        &self,
        caller: GroupLaunchTicketCallerContext,
        consumed: ConsumedGroupLaunchTicket,
    ) -> Result<GroupKnowledgebaseLaunchTarget, ConsumedLaunchResolutionError> {
        // Actor binding is not re-checked here: the ticket was issued to and consumed under
        // IM's signed RPC caller context (mTLS plus signed metadata), which is the enforcement
        // point for actor identity per the group-knowledgebase boundary contract.
        if consumed.membership_role == GroupKnowledgeSpaceMemberRole::Guest {
            return Err(ConsumedLaunchResolutionError::Denied(
                "group guests cannot launch the group knowledgebase".to_string(),
            ));
        }

        let scope = GroupKnowledgeSpaceScope {
            tenant_id: caller.tenant_id,
            organization_id: caller.organization_id,
        };
        let binding = self
            .binding_store
            .get_group_space(scope, &consumed.conversation_id)
            .await?;
        if binding.lifecycle_state != GroupKnowledgeSpaceLifecycleState::Active
            || binding.id != consumed.knowledgebase_binding_id
            || binding.uuid != consumed.knowledgebase_binding_uuid
            || binding.space_id != Some(consumed.space_id)
            || binding.space_uuid.as_deref() != Some(consumed.space_uuid.as_str())
            || binding.membership_epoch != consumed.membership_epoch
            || binding.upstream_link_generation != consumed.upstream_link_generation
        {
            return Err(ConsumedLaunchResolutionError::Denied(
                "group launch ticket no longer resolves to the current knowledgebase binding"
                    .to_string(),
            ));
        }
        // These two fences mirror `GroupKnowledgeSpaceAccessAuthorizer::authorize`, applied to
        // the binding already resolved above so the member snapshot below is loaded once.
        if binding.acl_projection_state != GroupKnowledgeSpaceAclProjectionState::Active {
            return Err(ConsumedLaunchResolutionError::Denied(
                "group knowledge space is not active with a current ACL projection".to_string(),
            ));
        }
        if self
            .binding_store
            .has_unsettled_group_membership_projection(scope, binding.id)
            .await?
        {
            return Err(ConsumedLaunchResolutionError::Denied(
                "group knowledge space membership ACL projection is not settled".to_string(),
            ));
        }

        let members = self
            .binding_store
            .list_active_group_members(scope, binding.id)
            .await?;
        let current_role = members
            .into_iter()
            .find(|member| member.actor_id == caller.actor_id)
            .map(|member| member.role)
            .ok_or_else(|| {
                ConsumedLaunchResolutionError::Denied(
                    "authenticated actor is no longer in the group knowledgebase snapshot"
                        .to_string(),
                )
            })?;
        if current_role == GroupKnowledgeSpaceMemberRole::Guest
            || current_role != consumed.membership_role
        {
            return Err(ConsumedLaunchResolutionError::Denied(
                "group launch ticket membership role is stale".to_string(),
            ));
        }

        Ok(GroupKnowledgebaseLaunchTarget {
            conversation_id: binding.conversation_id,
            space_id: consumed.space_id,
            space_uuid: binding.space_uuid.ok_or_else(|| {
                ConsumedLaunchResolutionError::InvalidBinding(
                    "active group binding has no space UUID".to_string(),
                )
            })?,
            group_name: binding.group_name,
            lifecycle_state: binding.lifecycle_state,
        })
    }
}

#[derive(Debug, Error)]
pub enum GroupKnowledgebaseLaunchResolverError {
    #[error("group launch request is invalid: {0}")]
    InvalidRequest(String),
    /// A post-consumption failure. The one-time ticket is burned and this exact ticket can
    /// never succeed again; clients must request a fresh launch ticket from IM and retry.
    #[error("group launch ticket was consumed but the launch failed and a new ticket is required: {0}")]
    TicketConsumed(String),
    #[error(transparent)]
    Ticket(#[from] GroupLaunchTicketConsumerError),
}

/// Classification of failures that happen after the remote ticket was consumed.
#[derive(Debug, Error)]
enum ConsumedLaunchResolutionError {
    #[error("group launch ticket is denied: {0}")]
    Denied(String),
    #[error("group launch binding is invalid: {0}")]
    InvalidBinding(String),
    #[error(transparent)]
    Binding(#[from] KnowledgeGroupSpaceBindingStoreError),
}
