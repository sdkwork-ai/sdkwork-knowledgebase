use axum::{
    extract::FromRequestParts,
    http::{request::Parts, StatusCode},
};
use std::ops::Deref;

use crate::{
    organization_scope::effective_organization_id,
    permission::can_access_knowledge_admin,
    routes::BackendState,
    BackendApiProblem, KnowledgeBackendRequestContext,
};

/// Authenticated backend request context injected by `sdkwork-web-framework` middleware.
#[derive(Debug, Clone)]
pub struct RequiredBackendContext(pub KnowledgeBackendRequestContext);

impl Deref for RequiredBackendContext {
    type Target = KnowledgeBackendRequestContext;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl<S> FromRequestParts<S> for RequiredBackendContext
where
    S: Send + Sync,
{
    type Rejection = BackendApiProblem;

    async fn from_request_parts(parts: &mut Parts, _state: &S) -> Result<Self, Self::Rejection> {
        parts
            .extensions
            .get::<KnowledgeBackendRequestContext>()
            .cloned()
            .map(RequiredBackendContext)
            .ok_or_else(|| {
                BackendApiProblem::new(
                    StatusCode::UNAUTHORIZED,
                    "missing_backend_request_context",
                    "authenticated backend request context is required",
                )
            })
    }
}

/// Extracts the authenticated backend request context after extractor validation.
pub fn require_backend_context(
    state: &BackendState,
    context: RequiredBackendContext,
) -> Result<KnowledgeBackendRequestContext, BackendApiProblem> {
    let context = context.0;
    ensure_runtime_tenant(state, &context)?;
    ensure_knowledge_admin_permission(&context)?;
    Ok(context)
}

/// Extracts and validates context for an admin mutation.
pub fn require_backend_mutation_context(
    state: &BackendState,
    context: RequiredBackendContext,
    _operation: &str,
) -> Result<KnowledgeBackendRequestContext, BackendApiProblem> {
    require_backend_context(state, context)
}

pub fn ensure_runtime_tenant(
    state: &BackendState,
    context: &KnowledgeBackendRequestContext,
) -> Result<(), BackendApiProblem> {
    if context.tenant_id != state.runtime_tenant_id {
        return Err(BackendApiProblem::new(
            StatusCode::FORBIDDEN,
            "tenant_id_mismatch",
            "authenticated tenant does not match configured runtime tenant",
        ));
    }
    let _ = effective_organization_id(context.organization_id);
    Ok(())
}

fn ensure_knowledge_admin_permission(
    context: &KnowledgeBackendRequestContext,
) -> Result<(), BackendApiProblem> {
    if can_access_knowledge_admin(context) {
        return Ok(());
    }
    Err(BackendApiProblem::new(
        StatusCode::FORBIDDEN,
        "knowledge_admin_permission_required",
        "knowledge.platform.manage permission is required for backend-api operations",
    ))
}
