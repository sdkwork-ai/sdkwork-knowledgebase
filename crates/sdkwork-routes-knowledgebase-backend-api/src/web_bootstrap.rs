use std::sync::Arc;

use axum::Router;
use sdkwork_iam_web_adapter::IamWebRequestContextResolver;
use sdkwork_web_axum::{with_web_request_context, WebFrameworkLayer};
use sdkwork_web_core::{
    AuthorizationPolicy, DefaultRateLimitPolicyResolver, DomainContextInjector, WebFrameworkError,
    WebRequestContext, WebRequestContextProfile,
};

use crate::http_route_manifest::backend_route_manifest;
use crate::permission::can_access_knowledge_admin;
use crate::web_framework_assembly::apply_knowledgebase_web_framework;
use crate::KnowledgeBackendRequestContext;

pub fn knowledgebase_backend_public_path_prefixes() -> Vec<String> {
    crate::health::knowledgebase_infra_public_path_prefixes()
}

#[derive(Clone, Default)]
struct KnowledgeBackendAuthorizationPolicy;

impl AuthorizationPolicy for KnowledgeBackendAuthorizationPolicy {
    fn authorize(
        &self,
        ctx: &WebRequestContext,
        _operation_id: Option<&str>,
    ) -> Result<(), WebFrameworkError> {
        let principal = ctx.principal.as_ref().ok_or_else(|| {
            WebFrameworkError::missing_credentials("authenticated principal is required")
        })?;
        let backend_context = knowledge_backend_context_from_web_request(ctx, principal)
            .ok_or_else(|| {
                WebFrameworkError::forbidden("authenticated backend principal is required")
            })?;
        if can_access_knowledge_admin(&backend_context) {
            return Ok(());
        }
        Err(WebFrameworkError::forbidden(
            "knowledge.platform.manage permission is required",
        ))
    }
}

#[derive(Clone, Default)]
struct KnowledgeBackendContextInjector;

pub fn knowledgebase_backend_context_injector() -> Arc<dyn DomainContextInjector> {
    Arc::new(KnowledgeBackendContextInjector)
}

impl DomainContextInjector for KnowledgeBackendContextInjector {
    fn inject(&self, request: &mut axum::extract::Request, context: &WebRequestContext) {
        if let Some(principal) = context.principal.as_ref() {
            if let Some(backend_context) =
                knowledge_backend_context_from_web_request(context, principal)
            {
                request.extensions_mut().insert(backend_context);
            }
        }
    }
}

fn knowledge_backend_context_from_web_request(
    context: &WebRequestContext,
    principal: &sdkwork_web_core::WebRequestPrincipal,
) -> Option<KnowledgeBackendRequestContext> {
    let tenant_id = principal.tenant_id().parse().ok()?;
    let operator_id = principal.user_id().parse().ok();
    let organization_id = principal
        .organization_id()
        .and_then(|value| value.parse().ok());
    Some(KnowledgeBackendRequestContext {
        tenant_id,
        operator_id,
        organization_id,
        permission_scope: principal.scopes.permission_scope.clone(),
        trace_id: context
            .trace_id
            .clone()
            .unwrap_or_else(|| context.request_id.0.clone()),
    })
}

pub fn wrap_router_with_web_framework(
    resolver: IamWebRequestContextResolver,
    router: Router,
) -> Router {
    with_web_request_context(router, build_backend_web_framework_layer(resolver))
}

fn build_backend_web_framework_layer(
    resolver: IamWebRequestContextResolver,
) -> WebFrameworkLayer<IamWebRequestContextResolver> {
    let route_manifest = backend_route_manifest();
    route_manifest
        .validate_public_path_prefixes(&knowledgebase_backend_public_path_prefixes())
        .expect(
            "knowledgebase backend-api public prefixes must not cover protected manifest routes",
        );

    apply_knowledgebase_web_framework(
        WebFrameworkLayer::new(resolver)
            .with_profile(WebRequestContextProfile {
                public_path_prefixes: knowledgebase_backend_public_path_prefixes(),
                ..WebRequestContextProfile::default()
            })
            .with_route_manifest(route_manifest)
            .with_authorization_policy(Arc::new(KnowledgeBackendAuthorizationPolicy))
            .with_domain_injector(knowledgebase_backend_context_injector())
            .with_rate_limit_store(crate::web_rate_limit_store::knowledgebase_rate_limit_store())
            .with_rate_limit_resolver(Arc::new(DefaultRateLimitPolicyResolver)),
    )
}

pub async fn wrap_router_with_web_framework_from_env(router: Router) -> Router {
    let resolver = sdkwork_iam_web_adapter::iam_web_request_context_resolver_from_env().await;
    let layer = crate::web_audit_store::attach_knowledgebase_audit_emitter(
        build_backend_web_framework_layer(resolver),
    )
    .await;
    with_web_request_context(router, layer)
}
