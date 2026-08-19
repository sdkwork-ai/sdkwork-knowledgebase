use sdkwork_web_core::WebRequestPrincipal;

/// Personal/tenant scope is represented as `organization_id = 0`.
pub const PERSONAL_ORGANIZATION_ID: u64 = 0;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OrganizationScopeViolation {
    MissingOrganizationId,
}

/// Request-scoped tenant/organization pair used for repository predicates.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct KnowledgeDataScope {
    pub tenant_id: u64,
    pub organization_id: u64,
}

pub fn knowledge_data_scope(tenant_id: u64, organization_id: Option<u64>) -> KnowledgeDataScope {
    KnowledgeDataScope {
        tenant_id,
        organization_id: effective_organization_id(organization_id),
    }
}

/// Parses an IAM organization claim. Blank and `"0"` values map to personal scope.
pub fn parse_organization_id_claim(raw: Option<&str>) -> Option<u64> {
    let raw = raw?.trim();
    if raw.is_empty() {
        return Some(PERSONAL_ORGANIZATION_ID);
    }
    raw.parse::<u64>().ok()
}

/// Normalizes request context organization id. `None`, `Some(0)`, and missing claims
/// all mean tenant/personal scope.
pub fn effective_organization_id(organization_id: Option<u64>) -> u64 {
    organization_id.unwrap_or(PERSONAL_ORGANIZATION_ID)
}

pub fn is_personal_organization_scope(organization_id: u64) -> bool {
    organization_id == PERSONAL_ORGANIZATION_ID
}

/// Resolves the organization scope carried into Knowledgebase handlers from IAM tokens.
///
/// Organization scope is derived exclusively from the authenticated principal claim.
/// Blank, missing, and `"0"` values normalize to tenant/personal scope (`0`).
pub fn resolve_knowledge_organization_id(principal: &WebRequestPrincipal) -> Option<u64> {
    Some(
        principal
            .organization_id()
            .and_then(|value| parse_organization_id_claim(Some(value)))
            .unwrap_or(PERSONAL_ORGANIZATION_ID),
    )
}

/// Requires a non-personal organization id for organization-only flows such as group launches.
pub fn require_organization_bound_scope(
    organization_id: Option<u64>,
) -> Result<u64, OrganizationScopeViolation> {
    let effective = effective_organization_id(organization_id);
    if is_personal_organization_scope(effective) {
        return Err(OrganizationScopeViolation::MissingOrganizationId);
    }
    Ok(effective)
}

#[cfg(test)]
mod tests {
    use super::*;
    use sdkwork_web_core::{
        WebDeploymentMode, WebEnvironment, WebLoginScope, WebRequestPrincipal,
    };

    fn principal(
        login_scope: WebLoginScope,
        organization_id: Option<&str>,
    ) -> WebRequestPrincipal {
        WebRequestPrincipal::builder()
            .tenant_id("1")
            .organization_id(organization_id.map(str::to_owned))
            .login_scope(login_scope)
            .user_id("42")
            .session_id(Some("session-1".to_owned()))
            .app_id("sdkwork-knowledgebase")
            .environment(WebEnvironment::Dev)
            .deployment_mode(WebDeploymentMode::Local)
            .build()
    }

    #[test]
    fn effective_organization_id_treats_none_and_zero_as_personal_scope() {
        assert_eq!(effective_organization_id(None), PERSONAL_ORGANIZATION_ID);
        assert_eq!(
            effective_organization_id(Some(PERSONAL_ORGANIZATION_ID)),
            PERSONAL_ORGANIZATION_ID
        );
        assert_eq!(effective_organization_id(Some(100)), 100);
    }

    #[test]
    fn knowledge_data_scope_normalizes_missing_organization_to_zero() {
        assert_eq!(
            knowledge_data_scope(100_001, None).organization_id,
            PERSONAL_ORGANIZATION_ID
        );
        assert_eq!(knowledge_data_scope(100_001, Some(42)).organization_id, 42);
    }

    #[test]
    fn parse_organization_id_claim_normalizes_blank_and_zero() {
        assert_eq!(parse_organization_id_claim(None), None);
        assert_eq!(parse_organization_id_claim(Some("")), Some(0));
        assert_eq!(parse_organization_id_claim(Some("  ")), Some(0));
        assert_eq!(parse_organization_id_claim(Some("0")), Some(0));
        assert_eq!(parse_organization_id_claim(Some("100")), Some(100));
    }

    #[test]
    fn resolve_organization_id_from_token_claim_only() {
        assert_eq!(
            resolve_knowledge_organization_id(&principal(WebLoginScope::Tenant, None)),
            Some(0)
        );
        assert_eq!(
            resolve_knowledge_organization_id(&principal(WebLoginScope::Tenant, Some("0"))),
            Some(0)
        );
        assert_eq!(
            resolve_knowledge_organization_id(&principal(WebLoginScope::Tenant, Some(" "))),
            Some(0)
        );
        assert_eq!(
            resolve_knowledge_organization_id(&principal(
                WebLoginScope::Organization,
                Some("100")
            )),
            Some(100)
        );
        assert_eq!(
            resolve_knowledge_organization_id(&principal(WebLoginScope::Organization, None)),
            Some(0)
        );
        assert_eq!(
            resolve_knowledge_organization_id(&principal(WebLoginScope::Organization, Some("0"))),
            Some(0)
        );
    }

    #[test]
    fn require_organization_bound_scope_rejects_personal_scope() {
        assert_eq!(
            require_organization_bound_scope(None),
            Err(OrganizationScopeViolation::MissingOrganizationId)
        );
        assert_eq!(
            require_organization_bound_scope(Some(0)),
            Err(OrganizationScopeViolation::MissingOrganizationId)
        );
        assert_eq!(require_organization_bound_scope(Some(2)), Ok(2));
    }
}
