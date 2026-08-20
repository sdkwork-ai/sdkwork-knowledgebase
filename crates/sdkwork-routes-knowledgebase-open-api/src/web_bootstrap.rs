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

use crate::http_route_manifest::open_route_manifest;
use crate::paths;
use crate::KnowledgeOpenApiRequestContext;

pub fn knowledgebase_open_api_public_path_prefixes() -> Vec<String> {
    sdkwork_routes_knowledgebase_backend_api::health::knowledgebase_infra_public_path_prefixes()
}

pub fn knowledgebase_open_api_prefixes() -> Vec<String> {
    vec![paths::PREFIX.to_owned()]
}

#[derive(Clone, Default)]
struct KnowledgeOpenApiContextInjector;

pub fn knowledgebase_open_api_context_injector() -> Arc<dyn DomainContextInjector> {
    Arc::new(KnowledgeOpenApiContextInjector)
}

impl DomainContextInjector for KnowledgeOpenApiContextInjector {
    fn inject(&self, request: &mut axum::extract::Request, context: &WebRequestContext) {
        if let Some(open_context) = knowledge_open_api_context_from_web_request(context) {
            request.extensions_mut().insert(open_context);
        }
    }
}

fn knowledge_open_api_context_from_web_request(
    context: &WebRequestContext,
) -> Option<KnowledgeOpenApiRequestContext> {
    let principal = context.principal.as_ref()?;
    let tenant_id = principal.tenant_id().parse().ok()?;
    let actor_id = principal.user_id().parse().ok();
    let organization_id = principal
        .organization_id()
        .and_then(|value| value.parse().ok());
    let credential_id = principal
        .api_key_id()
        .map(str::to_owned)
        .or_else(|| principal.session_id().map(str::to_owned))
        .unwrap_or_else(|| principal.user_id().to_owned());
    Some(KnowledgeOpenApiRequestContext {
        api_key_id: credential_id,
        tenant_id,
        actor_id,
        organization_id,
        request_id: context.request_id.0.clone(),
        trace_id: context.trace_id.clone(),
        idempotency_key: context.idempotency_key().map(str::to_owned),
    })
}

pub fn wrap_router_with_web_framework(
    resolver: IamWebRequestContextResolver,
    router: Router,
) -> Router {
    with_web_request_context(router, build_open_web_framework_layer(resolver))
}

fn build_open_web_framework_layer(
    resolver: IamWebRequestContextResolver,
) -> WebFrameworkLayer<IamWebRequestContextResolver> {
    let route_manifest = open_route_manifest();
    route_manifest
        .validate_public_path_prefixes(&knowledgebase_open_api_public_path_prefixes())
        .expect("knowledgebase open-api public prefixes must not cover protected manifest routes");

    apply_knowledgebase_web_framework(
        WebFrameworkLayer::new(resolver)
            .with_profile(WebRequestContextProfile {
                open_api_prefixes: knowledgebase_open_api_prefixes(),
                public_path_prefixes: knowledgebase_open_api_public_path_prefixes(),
                ..WebRequestContextProfile::default()
            })
            .with_route_manifest(route_manifest.clone())
            .with_authorization_policy(Arc::new(ManifestAuthorizationPolicy::new(route_manifest)))
            .with_domain_injector(knowledgebase_open_api_context_injector())
            .with_rate_limit_store(knowledgebase_rate_limit_store())
            .with_rate_limit_resolver(Arc::new(DefaultRateLimitPolicyResolver)),
    )
}

pub async fn wrap_router_with_web_framework_from_env(router: Router) -> Router {
    let resolver = sdkwork_iam_web_adapter::iam_web_request_context_resolver_from_env().await;
    let layer = attach_knowledgebase_audit_emitter(build_open_web_framework_layer(resolver)).await;
    with_web_request_context(router, layer)
}
