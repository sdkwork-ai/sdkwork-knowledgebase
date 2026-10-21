use crate::ports::{
    knowledge_access_control::KnowledgeAccessRole,
    knowledge_group_space_binding_store::{
        GroupKnowledgeSpaceScope, KnowledgeGroupSpaceBindingStore,
        KnowledgeGroupSpaceBindingStoreError,
    },
};
use sdkwork_knowledgebase_contract::group_space::{
    GroupKnowledgeSpaceAccessLevel, GroupKnowledgeSpaceAclProjectionState,
    GroupKnowledgeSpaceBinding, GroupKnowledgeSpaceLifecycleState,
};
use sdkwork_utils_rust::is_blank;
use thiserror::Error;

/// Authorizes group-managed knowledge spaces from the synchronized IM member snapshot. Drive ACL
/// records are a projection for provider enforcement, never the authority for group membership.
pub struct GroupKnowledgeSpaceAccessAuthorizer<'a> {
    binding_store: &'a dyn KnowledgeGroupSpaceBindingStore,
}

impl<'a> GroupKnowledgeSpaceAccessAuthorizer<'a> {
    pub fn new(binding_store: &'a dyn KnowledgeGroupSpaceBindingStore) -> Self {
        Self { binding_store }
    }

    /// Resolves whether a tenant-owned space is group-managed before applying exact scope or
    /// membership checks. A group-managed space in another token-derived scope is deliberately an
    /// authorization failure, not an ordinary-space miss, so it cannot fall through to generic
    /// Drive ACL logic.
    pub async fn resolve_group_managed_space(
        &self,
        scope: GroupKnowledgeSpaceScope,
        space_id: u64,
    ) -> Result<Option<GroupKnowledgeSpaceBinding>, GroupKnowledgeSpaceAccessAuthorizerError> {
        if space_id == 0 || scope.tenant_id == 0 {
            return Err(GroupKnowledgeSpaceAccessAuthorizerError::InvalidRequest(
                "tenant_id and space_id are required".to_string(),
            ));
        }

        let Some(binding) = self
            .binding_store
            .find_group_space_for_space_in_tenant(scope.tenant_id, scope.organization_id, space_id)
            .await
            .map_err(GroupKnowledgeSpaceAccessAuthorizerError::Store)?
        else {
            return Ok(None);
        };

        if binding.organization_id != scope.organization_id {
            return Err(GroupKnowledgeSpaceAccessAuthorizerError::Denied(
                "group knowledge space belongs to a different authenticated scope".to_string(),
            ));
        }

        Ok(Some(binding))
    }

    /// Returns `Ok(None)` for an ordinary space. A group-managed space is either explicitly
    /// admitted from its current snapshot or rejected; it never falls through to generic Drive
    /// ACL authorization.
    pub async fn authorize(
        &self,
        scope: GroupKnowledgeSpaceScope,
        space_id: u64,
        actor_id: &str,
        required_role: KnowledgeAccessRole,
    ) -> Result<Option<GroupKnowledgeSpaceBinding>, GroupKnowledgeSpaceAccessAuthorizerError> {
        if is_blank(Some(actor_id)) {
            return Err(GroupKnowledgeSpaceAccessAuthorizerError::InvalidRequest(
                "actor_id is required".to_string(),
            ));
        }

        let Some(binding) = self.resolve_group_managed_space(scope, space_id).await? else {
            return Ok(None);
        };

        if binding.lifecycle_state != GroupKnowledgeSpaceLifecycleState::Active
            || binding.acl_projection_state != GroupKnowledgeSpaceAclProjectionState::Active
        {
            return Err(GroupKnowledgeSpaceAccessAuthorizerError::Denied(
                "group knowledge space is not active with a current ACL projection".to_string(),
            ));
        }

        if self
            .binding_store
            .has_unsettled_group_membership_projection(scope, binding.id)
            .await
            .map_err(GroupKnowledgeSpaceAccessAuthorizerError::Store)?
        {
            return Err(GroupKnowledgeSpaceAccessAuthorizerError::Denied(
                "group knowledge space membership ACL projection is not settled".to_string(),
            ));
        }

        let members = self
            .binding_store
            .list_active_group_members(scope, binding.id)
            .await
            .map_err(GroupKnowledgeSpaceAccessAuthorizerError::Store)?;
        let Some(member) = members
            .into_iter()
            .find(|member| member.actor_id == actor_id)
        else {
            return Err(GroupKnowledgeSpaceAccessAuthorizerError::Denied(
                "actor is not an active group knowledgebase member".to_string(),
            ));
        };

        let Some(access_level) = member.role.access_level() else {
            return Err(GroupKnowledgeSpaceAccessAuthorizerError::Denied(
                "group guest members do not receive knowledgebase access".to_string(),
            ));
        };
        let effective_role = knowledge_access_role(access_level);
        if !effective_role.satisfies(&required_role) {
            return Err(GroupKnowledgeSpaceAccessAuthorizerError::Denied(
                "group membership does not satisfy the required knowledgebase role".to_string(),
            ));
        }

        Ok(Some(binding))
    }
}

pub fn knowledge_access_role(access_level: GroupKnowledgeSpaceAccessLevel) -> KnowledgeAccessRole {
    match access_level {
        GroupKnowledgeSpaceAccessLevel::Reader => KnowledgeAccessRole::Reader,
        GroupKnowledgeSpaceAccessLevel::Writer => KnowledgeAccessRole::Writer,
        GroupKnowledgeSpaceAccessLevel::Owner => KnowledgeAccessRole::Owner,
    }
}

#[derive(Debug, Error)]
pub enum GroupKnowledgeSpaceAccessAuthorizerError {
    #[error("group knowledgebase authorization request is invalid: {0}")]
    InvalidRequest(String),
    #[error("group knowledgebase access denied: {0}")]
    Denied(String),
    #[error(transparent)]
    Store(#[from] KnowledgeGroupSpaceBindingStoreError),
}
