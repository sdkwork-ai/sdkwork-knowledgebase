use std::sync::Arc;

use axum::Router;
use sdkwork_iam_web_adapter::IamWebRequestContextResolver;
use sdkwork_routes_knowledgebase_backend_api::{
    apply_knowledgebase_web_framework, attach_knowledgebase_audit_emitter,
    knowledgebase_rate_limit_store,
};
use sdkwork_web_axum::{with_web_request_context, WebFrameworkLayer};
use sdkwork_web_core::{
    DefaultRateLimitPolicyResolver, DomainContextInjector, ManifestAuthorizationPolicy,
    WebRequestContext, WebRequestContextProfile,
};

use crate::http_route_manifest::app_route_manifest;
use crate::paths;
use crate::KnowledgeAppRequestContext;

pub fn knowledgebase_public_path_prefixes() -> Vec<String> {
    vec![
        paths::LIVEZ.to_owned(),
        paths::READYZ.to_owned(),
        paths::HEALTHZ.to_owned(),
    ]
}

#[derive(Clone, Default)]
struct KnowledgeAppContextInjector;

pub fn knowledgebase_app_context_injector() -> Arc<dyn DomainContextInjector> {
    Arc::new(KnowledgeAppContextInjector)
}

impl DomainContextInjector for KnowledgeAppContextInjector {
    fn inject(&self, request: &mut axum::extract::Request, context: &WebRequestContext) {
        if let Some(app_context) = knowledge_app_context_from_web_request(context) {
            request.extensions_mut().insert(app_context);
        }
    }
}

fn knowledge_app_context_from_web_request(
    context: &WebRequestContext,
) -> Option<KnowledgeAppRequestContext> {
    let principal = context.principal.as_ref()?;
    let tenant_id = principal.tenant_id().parse().ok()?;
    let actor_id = principal.user_id().parse().ok();
    let organization_id = principal
        .organization_id()
        .and_then(|value| value.parse().ok());
    let session_id = principal.session_id().map(str::to_owned);
    Some(KnowledgeAppRequestContext {
        tenant_id,
        actor_id,
        organization_id,
        session_id,
        request_id: context.request_id.0.clone(),
        trace_id: context.trace_id.clone(),
        idempotency_key: context.idempotency_key().map(str::to_owned),
    })
}

pub fn wrap_router_with_web_framework(
    resolver: IamWebRequestContextResolver,
    router: Router,
) -> Router {
    with_web_request_context(router, build_app_web_framework_layer(resolver))
}

fn build_app_web_framework_layer(
    resolver: IamWebRequestContextResolver,
) -> WebFrameworkLayer<IamWebRequestContextResolver> {
    let route_manifest = app_route_manifest();
    route_manifest
        .validate_public_path_prefixes(&knowledgebase_public_path_prefixes())
        .expect("knowledgebase app-api public prefixes must not cover protected manifest routes");

    apply_knowledgebase_web_framework(
        WebFrameworkLayer::new(resolver)
            .with_profile(WebRequestContextProfile {
                public_path_prefixes: knowledgebase_public_path_prefixes(),
                ..WebRequestContextProfile::default()
            })
            .with_route_manifest(route_manifest.clone())
            .with_authorization_policy(Arc::new(ManifestAuthorizationPolicy::new(route_manifest)))
            .with_domain_injector(knowledgebase_app_context_injector())
            .with_rate_limit_store(knowledgebase_rate_limit_store())
            .with_rate_limit_resolver(Arc::new(DefaultRateLimitPolicyResolver)),
    )
}

pub async fn wrap_router_with_web_framework_from_env(router: Router) -> Router {
    let resolver = sdkwork_iam_web_adapter::iam_web_request_context_resolver_from_env().await;
    let layer = attach_knowledgebase_audit_emitter(build_app_web_framework_layer(resolver)).await;
    with_web_request_context(router, layer)
}
