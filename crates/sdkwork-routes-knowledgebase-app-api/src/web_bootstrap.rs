use std::sync::Arc;

use axum::Router;
use sdkwork_iam_web_adapter::IamWebRequestContextResolver;
use sdkwork_routes_knowledgebase_backend_api::{
    apply_knowledgebase_web_framework, attach_knowledgebase_audit_emitter,
    knowledgebase_rate_limit_store, resolve_knowledge_organization_id,
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
    let organization_id = resolve_knowledge_organization_id(principal);
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

#[cfg(test)]
mod tests {
    use super::knowledge_app_context_from_web_request;
    use sdkwork_web_core::{
        ServerRequestId, WebApiSurface, WebAuthMode, WebDeploymentMode, WebEnvironment,
        WebLoginScope, WebRequestContext, WebRequestPrincipal, WebTransportFacts,
    };

    fn web_context(
        login_scope: WebLoginScope,
        organization_id: Option<&str>,
    ) -> WebRequestContext {
        WebRequestContext {
            request_id: ServerRequestId("request-tenant-scope".to_string()),
            api_surface: WebApiSurface::AppApi,
            auth_mode: WebAuthMode::DualToken,
            transport: WebTransportFacts {
                path: "/app/v3/api/knowledge/spaces".to_string(),
                method: "POST".to_string(),
                auth_token_present: true,
                access_token_present: true,
                api_key_present: false,
                ingress_token_present: false,
                oauth_bearer_present: false,
                agent_token_present: false,
            },
            principal: Some(
                WebRequestPrincipal::builder()
                    .tenant_id("100001")
                    .organization_id(organization_id.map(str::to_owned))
                    .login_scope(login_scope)
                    .user_id("42")
                    .session_id(Some("session-1".to_owned()))
                    .app_id("sdkwork-knowledgebase")
                    .environment(WebEnvironment::Dev)
                    .deployment_mode(WebDeploymentMode::Local)
                    .build(),
            ),
            locale: None,
            client_kind: None,
            operation: None,
            trace_id: Some("trace-tenant-scope".to_string()),
            idempotency_key: None,
        }
    }

    #[test]
    fn tenant_login_injects_zero_organization_scope_for_blank_claims() {
        for organization_id in [None, Some("0"), Some(" ")] {
            let context = knowledge_app_context_from_web_request(&web_context(
                WebLoginScope::Tenant,
                organization_id,
            ))
            .expect("tenant login should inject app context");
            assert_eq!(context.organization_id, Some(0));
        }
    }

    #[test]
    fn organization_login_preserves_nonzero_organization_scope() {
        let context = knowledge_app_context_from_web_request(&web_context(
            WebLoginScope::Organization,
            Some("100"),
        ))
        .expect("organization login should inject app context");
        assert_eq!(context.organization_id, Some(100));
    }

    #[test]
    fn missing_organization_claim_normalizes_to_zero() {
        let context = knowledge_app_context_from_web_request(&web_context(
            WebLoginScope::Organization,
            None,
        ))
        .expect("missing organization claim should still inject app context");
        assert_eq!(context.organization_id, Some(0));
    }
}
