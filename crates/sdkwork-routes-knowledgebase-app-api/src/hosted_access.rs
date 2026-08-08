use axum::http::StatusCode;
use sdkwork_intelligence_knowledgebase_service::ports::knowledge_access_control::{
    KnowledgeAccessControl, KnowledgeAccessRole, KnowledgeSpaceMember as ServiceSpaceMember,
    KnowledgeSubjectType,
};
use sdkwork_intelligence_knowledgebase_service::{
    group_space_access::GroupKnowledgeSpaceAccessAuthorizer,
    okf::{OkfBundleFileRegistryService, OkfBundleInitializerService},
    ports::{
        knowledge_access_control::KnowledgeAccessCheckRequest,
        knowledge_group_space_binding_store::GroupKnowledgeSpaceScope,
        knowledge_ingestion_job_store::IngestionJobStore,
        knowledge_space_store::KnowledgeSpaceStore,
    },
    space::KnowledgeSpaceService,
};
use sdkwork_knowledgebase_contract::rag::{
    KnowledgeAgentBinding, KnowledgeAgentProfile, KnowledgeRetrievalBinding,
};
use sdkwork_knowledgebase_contract::{
    ingest::IngestionJob, GrantKnowledgeSpaceMemberRequest, KnowledgeDocument, KnowledgeSpace,
    KnowledgeSpaceMember, KnowledgeSpaceMemberRole,
    KnowledgeSpaceMemberSubjectType, UpdateKnowledgeSpaceRequest,
};
use sdkwork_utils_rust::{is_blank, SdkWorkPageData};
use std::collections::HashSet;

use crate::{
    error::ApiError, hosted::map_okf_concept_store_error, ports::KnowledgeAppRequestContext,
    runtime::KnowledgebaseRuntime, ApiResult,
};

pub(crate) fn ensure_runtime_tenant(
    runtime: &KnowledgebaseRuntime,
    context: &KnowledgeAppRequestContext,
) -> ApiResult<()> {
    if context.tenant_id != runtime.tenant_id() {
        return Err(ApiError::new(
            StatusCode::FORBIDDEN,
            "tenant_id_mismatch",
            "authenticated tenant does not match configured runtime tenant",
        ));
    }
    ensure_runtime_organization(runtime, context)?;
    Ok(())
}

pub(crate) fn ensure_runtime_organization(
    runtime: &KnowledgebaseRuntime,
    context: &KnowledgeAppRequestContext,
) -> ApiResult<()> {
    let runtime_org = runtime.organization_id();
    let context_org = context.organization_id.unwrap_or(0);
    if runtime_org != 0 && context_org == 0 {
        return Err(ApiError::new(
            StatusCode::FORBIDDEN,
            "missing_organization_id",
            "organization context is required for this operation",
        ));
    };
    if context_org != runtime_org {
        return Err(ApiError::new(
            StatusCode::FORBIDDEN,
            "organization_id_mismatch",
            "authenticated organization does not match configured runtime organization",
        ));
    }
    Ok(())
}

pub(crate) fn require_actor_id(context: &KnowledgeAppRequestContext) -> ApiResult<String> {
    require_numeric_actor_id(context).map(|value| value.to_string())
}

pub(crate) fn require_numeric_actor_id(context: &KnowledgeAppRequestContext) -> ApiResult<u64> {
    context.actor_id.filter(|value| *value != 0).ok_or_else(|| {
        ApiError::new(
            StatusCode::UNAUTHORIZED,
            "missing_actor_id",
            "authenticated actor_id is required for this operation",
        )
    })
}

/// Determines group ownership before generic space services can consult Drive ACLs. A binding in
/// another organization is surfaced as a denial by the group authorizer, never as an ordinary
/// space, which keeps every generic route fail-closed for group-managed resources.
async fn is_group_managed_space(
    runtime: &KnowledgebaseRuntime,
    scope: GroupKnowledgeSpaceScope,
    space_id: u64,
) -> ApiResult<bool> {
    GroupKnowledgeSpaceAccessAuthorizer::new(runtime.group_space_binding_store())
        .resolve_group_managed_space(scope, space_id)
        .await
        .map_err(ApiError::from)
        .map(|binding| binding.is_some())
}

fn group_managed_space_controlled_by_im() -> ApiError {
    ApiError::new(
        StatusCode::FORBIDDEN,
        "group_knowledge_space_managed_by_im",
        "group knowledge space membership and lifecycle are managed by IM",
    )
}

pub(crate) async fn require_bindings_space_access(
    runtime: &KnowledgebaseRuntime,
    context: &KnowledgeAppRequestContext,
    bindings: &[KnowledgeRetrievalBinding],
) -> ApiResult<()> {
    require_bindings_space_access_with_role(runtime, context, bindings, KnowledgeAccessRole::Reader)
        .await
}

pub(crate) async fn require_bindings_space_access_with_role(
    runtime: &KnowledgebaseRuntime,
    context: &KnowledgeAppRequestContext,
    bindings: &[KnowledgeRetrievalBinding],
    required_role: KnowledgeAccessRole,
) -> ApiResult<()> {
    let mut seen = HashSet::new();
    for binding in bindings {
        if seen.insert(binding.space_id) {
            require_space_access_with_role(runtime, context, binding.space_id, required_role)
                .await?;
        }
    }
    Ok(())
}

pub(crate) async fn require_space_access(
    runtime: &KnowledgebaseRuntime,
    context: &KnowledgeAppRequestContext,
    space_id: u64,
) -> ApiResult<KnowledgeSpace> {
    require_space_access_with_role(runtime, context, space_id, KnowledgeAccessRole::Reader).await
}

pub(crate) async fn require_space_access_with_role(
    runtime: &KnowledgebaseRuntime,
    context: &KnowledgeAppRequestContext,
    space_id: u64,
    required_role: KnowledgeAccessRole,
) -> ApiResult<KnowledgeSpace> {
    ensure_runtime_tenant(runtime, context)?;
    let actor_id = require_actor_id(context)?;
    let group_authorizer =
        GroupKnowledgeSpaceAccessAuthorizer::new(runtime.group_space_binding_store());
    if group_authorizer
        .authorize(
            GroupKnowledgeSpaceScope {
                tenant_id: context.tenant_id,
                organization_id: context.organization_id.unwrap_or(0),
            },
            space_id,
            &actor_id,
            required_role,
        )
        .await
        .map_err(ApiError::from)?
        .is_some()
    {
        let space = runtime
            .space_store()
            .get_group_managed_space(space_id)
            .await
            .map_err(ApiError::from)?;
        let drive_space_id = space.drive_space_id.clone().ok_or_else(|| {
            ApiError::new(
                StatusCode::FORBIDDEN,
                "group_knowledge_space_access_denied",
                "group knowledge space is not bound to a Drive space",
            )
        })?;
        let drive_grant = runtime
            .access_control()
            .check_space_access(KnowledgeAccessCheckRequest {
                tenant_id: context.tenant_id.to_string(),
                actor_id: actor_id.clone(),
                drive_space_id,
                required_role,
            })
            .await
            .map_err(|_| {
                ApiError::new(
                    StatusCode::SERVICE_UNAVAILABLE,
                    "group_knowledge_space_access_check_unavailable",
                    "group knowledge space access is temporarily unavailable",
                )
            })?;
        if !drive_grant.allowed {
            return Err(ApiError::new(
                StatusCode::FORBIDDEN,
                "group_knowledge_space_access_denied",
                "group knowledge space access is denied",
            ));
        }
        return Ok(space);
    }
    let file_registry = OkfBundleFileRegistryService::new(runtime.okf_bundle_file_store());
    let okf_initializer = OkfBundleInitializerService::new(runtime.drive_storage())
        .with_registry(&file_registry)
        .with_drive_workspace(runtime.drive_workspace());
    KnowledgeSpaceService::new(runtime.space_store(), &okf_initializer)
        .with_drive_context(runtime.tenant_id_str(), runtime.operator_id())
        .with_drive_space_provisioner(runtime.drive_space_provisioner())
        .with_access_control(runtime.access_control())
        .get_space_with_role_check(
            space_id,
            &context.tenant_id.to_string(),
            &actor_id,
            required_role,
        )
        .await
        .map_err(ApiError::from)
}

pub(crate) async fn require_document_access(
    runtime: &KnowledgebaseRuntime,
    context: &KnowledgeAppRequestContext,
    document_id: u64,
) -> ApiResult<KnowledgeDocument> {
    require_document_access_with_role(runtime, context, document_id, KnowledgeAccessRole::Reader)
        .await
}

pub(crate) async fn require_document_access_with_role(
    runtime: &KnowledgebaseRuntime,
    context: &KnowledgeAppRequestContext,
    document_id: u64,
    required_role: KnowledgeAccessRole,
) -> ApiResult<KnowledgeDocument> {
    ensure_runtime_tenant(runtime, context)?;
    let document = runtime
        .document_store()
        .get_document_by_id(document_id)
        .await
        .map_err(ApiError::from)?;
    require_space_access_with_role(runtime, context, document.space_id, required_role).await?;
    Ok(document)
}

pub(crate) async fn require_ingest_access(
    runtime: &KnowledgebaseRuntime,
    context: &KnowledgeAppRequestContext,
    ingest_id: u64,
) -> ApiResult<IngestionJob> {
    require_ingest_access_with_role(runtime, context, ingest_id, KnowledgeAccessRole::Reader).await
}

pub(crate) async fn require_ingest_access_with_role(
    runtime: &KnowledgebaseRuntime,
    context: &KnowledgeAppRequestContext,
    ingest_id: u64,
    required_role: KnowledgeAccessRole,
) -> ApiResult<IngestionJob> {
    ensure_runtime_tenant(runtime, context)?;
    let job = runtime
        .ingestion_job_store()
        .get_job(ingest_id)
        .await
        .map_err(ApiError::from)?;
    require_space_access_with_role(runtime, context, job.space_id, required_role).await?;
    Ok(job)
}

pub(crate) async fn require_okf_concept_space_access(
    runtime: &KnowledgebaseRuntime,
    context: &KnowledgeAppRequestContext,
    concept_row_id: u64,
) -> ApiResult<KnowledgeSpace> {
    require_okf_concept_space_access_with_role(
        runtime,
        context,
        concept_row_id,
        KnowledgeAccessRole::Reader,
    )
    .await
}

pub(crate) async fn require_okf_concept_space_access_with_role(
    runtime: &KnowledgebaseRuntime,
    context: &KnowledgeAppRequestContext,
    concept_row_id: u64,
    required_role: KnowledgeAccessRole,
) -> ApiResult<KnowledgeSpace> {
    ensure_runtime_tenant(runtime, context)?;
    let concept = runtime
        .okf_concept_store()
        .get_concept_by_row_id(concept_row_id)
        .await
        .map_err(map_okf_concept_store_error)?;
    require_space_access_with_role(runtime, context, concept.space_id, required_role).await
}

pub(crate) async fn create_space_with_context(
    runtime: &KnowledgebaseRuntime,
    context: &KnowledgeAppRequestContext,
    request: sdkwork_knowledgebase_contract::CreateKnowledgeSpaceRequest,
) -> ApiResult<KnowledgeSpace> {
    ensure_runtime_tenant(runtime, context)?;
    let actor_id = require_numeric_actor_id(context)?;
    let file_registry = OkfBundleFileRegistryService::new(runtime.okf_bundle_file_store());
    let okf_initializer = OkfBundleInitializerService::new(runtime.drive_storage())
        .with_registry(&file_registry)
        .with_drive_workspace(runtime.drive_workspace());
    let wiki_initializer =
        sdkwork_intelligence_knowledgebase_service::wiki_initialization::KnowledgeWikiInitializationService::new(
            runtime.wiki_store(),
            runtime.wiki_store(),
            runtime.wiki_drive_scope(),
        );
    KnowledgeSpaceService::new(runtime.space_store(), &okf_initializer)
        .with_drive_context(runtime.tenant_id_str(), runtime.operator_id())
        .with_drive_space_provisioner(runtime.drive_space_provisioner())
        .with_access_control(runtime.access_control())
        .with_wiki_context(
            sdkwork_intelligence_knowledgebase_service::ports::knowledge_wiki_persistence::WikiPersistenceScope {
                tenant_id: context.tenant_id,
                organization_id: context.organization_id.unwrap_or(0),
            },
            actor_id,
        )
        .with_wiki_initializer(&wiki_initializer)
        .create_space(request)
        .await
        .map_err(ApiError::from)
}

pub(crate) async fn update_space_with_context(
    runtime: &KnowledgebaseRuntime,
    context: &KnowledgeAppRequestContext,
    space_id: u64,
    request: UpdateKnowledgeSpaceRequest,
) -> ApiResult<KnowledgeSpace> {
    ensure_runtime_tenant(runtime, context)?;
    let actor_id = require_actor_id(context)?;
    let scope = GroupKnowledgeSpaceScope {
        tenant_id: context.tenant_id,
        organization_id: context.organization_id.unwrap_or(0),
    };
    if is_group_managed_space(runtime, scope, space_id).await? {
        if request.name.is_some() {
            return Err(group_managed_space_controlled_by_im());
        }
        let Some(description) = request.description else {
            return Err(ApiError::invalid_request(
                "invalid_group_knowledge_space_update",
                "a group knowledge space update must include a description",
            ));
        };
        // Description is KB-owned metadata, but its mutation still requires the current IM
        // group owner snapshot plus the matching projected Drive ACL.
        require_space_access_with_role(runtime, context, space_id, KnowledgeAccessRole::Owner)
            .await?;
        let file_registry = OkfBundleFileRegistryService::new(runtime.okf_bundle_file_store());
        let okf_initializer = OkfBundleInitializerService::new(runtime.drive_storage())
            .with_registry(&file_registry)
            .with_drive_workspace(runtime.drive_workspace());
        return KnowledgeSpaceService::new(runtime.space_store(), &okf_initializer)
            .update_group_managed_space_description(space_id, description)
            .await
            .map_err(ApiError::from);
    }
    let file_registry = OkfBundleFileRegistryService::new(runtime.okf_bundle_file_store());
    let okf_initializer = OkfBundleInitializerService::new(runtime.drive_storage())
        .with_registry(&file_registry)
        .with_drive_workspace(runtime.drive_workspace());
    KnowledgeSpaceService::new(runtime.space_store(), &okf_initializer)
        .with_drive_context(runtime.tenant_id_str(), runtime.operator_id())
        .with_drive_space_provisioner(runtime.drive_space_provisioner())
        .with_access_control(runtime.access_control())
        .update_space(space_id, &context.tenant_id.to_string(), &actor_id, request)
        .await
        .map_err(ApiError::from)
}

pub(crate) async fn delete_space_with_context(
    runtime: &KnowledgebaseRuntime,
    context: &KnowledgeAppRequestContext,
    space_id: u64,
) -> ApiResult<()> {
    ensure_runtime_tenant(runtime, context)?;
    if is_group_managed_space(
        runtime,
        GroupKnowledgeSpaceScope {
            tenant_id: context.tenant_id,
            organization_id: context.organization_id.unwrap_or(0),
        },
        space_id,
    )
    .await?
    {
        return Err(group_managed_space_controlled_by_im());
    }
    let actor_id = require_actor_id(context)?;
    let file_registry = OkfBundleFileRegistryService::new(runtime.okf_bundle_file_store());
    let okf_initializer = OkfBundleInitializerService::new(runtime.drive_storage())
        .with_registry(&file_registry)
        .with_drive_workspace(runtime.drive_workspace());
    KnowledgeSpaceService::new(runtime.space_store(), &okf_initializer)
        .with_drive_context(runtime.tenant_id_str(), runtime.operator_id())
        .with_drive_space_provisioner(runtime.drive_space_provisioner())
        .with_access_control(runtime.access_control())
        .delete_space(space_id, &context.tenant_id.to_string(), &actor_id)
        .await
        .map_err(ApiError::from)
}

fn map_member_role(role: KnowledgeAccessRole) -> KnowledgeSpaceMemberRole {
    match role {
        KnowledgeAccessRole::Reader => KnowledgeSpaceMemberRole::Reader,
        KnowledgeAccessRole::Writer => KnowledgeSpaceMemberRole::Writer,
        KnowledgeAccessRole::Owner => KnowledgeSpaceMemberRole::Owner,
    }
}

fn map_member_subject_type(subject_type: KnowledgeSubjectType) -> KnowledgeSpaceMemberSubjectType {
    match subject_type {
        KnowledgeSubjectType::User => KnowledgeSpaceMemberSubjectType::User,
        KnowledgeSubjectType::Group => KnowledgeSpaceMemberSubjectType::Group,
        KnowledgeSubjectType::Domain => KnowledgeSpaceMemberSubjectType::Domain,
        KnowledgeSubjectType::App => KnowledgeSpaceMemberSubjectType::App,
    }
}

fn map_contract_member(member: ServiceSpaceMember) -> KnowledgeSpaceMember {
    KnowledgeSpaceMember {
        subject_type: map_member_subject_type(member.subject_type),
        subject_id: member.subject_id,
        role: map_member_role(member.role),
        inherited: member.inherited,
    }
}

pub(crate) fn parse_member_subject_type(
    subject_type: KnowledgeSpaceMemberSubjectType,
) -> KnowledgeSubjectType {
    match subject_type {
        KnowledgeSpaceMemberSubjectType::User => KnowledgeSubjectType::User,
        KnowledgeSpaceMemberSubjectType::Group => KnowledgeSubjectType::Group,
        KnowledgeSpaceMemberSubjectType::Domain => KnowledgeSubjectType::Domain,
        KnowledgeSpaceMemberSubjectType::App => KnowledgeSubjectType::App,
    }
}

pub(crate) fn parse_member_role(role: KnowledgeSpaceMemberRole) -> KnowledgeAccessRole {
    match role {
        KnowledgeSpaceMemberRole::Reader => KnowledgeAccessRole::Reader,
        KnowledgeSpaceMemberRole::Writer => KnowledgeAccessRole::Writer,
        KnowledgeSpaceMemberRole::Owner => KnowledgeAccessRole::Owner,
    }
}

fn space_service<'a>(
    runtime: &'a KnowledgebaseRuntime,
    okf_initializer: &'a OkfBundleInitializerService,
) -> KnowledgeSpaceService<'a> {
    KnowledgeSpaceService::new(runtime.space_store(), okf_initializer)
        .with_drive_context(runtime.tenant_id_str(), runtime.operator_id())
        .with_drive_space_provisioner(runtime.drive_space_provisioner())
        .with_access_control(runtime.access_control())
}

pub(crate) async fn list_space_members_with_context(
    runtime: &KnowledgebaseRuntime,
    context: &KnowledgeAppRequestContext,
    space_id: u64,
    cursor: Option<String>,
    page_size: Option<u32>,
) -> ApiResult<SdkWorkPageData<KnowledgeSpaceMember>> {
    ensure_runtime_tenant(runtime, context)?;
    if is_group_managed_space(
        runtime,
        GroupKnowledgeSpaceScope {
            tenant_id: context.tenant_id,
            organization_id: context.organization_id.unwrap_or(0),
        },
        space_id,
    )
    .await?
    {
        // The generic Drive member list is not an authority for group membership. Keep this
        // fail-closed until the IM snapshot list has its own paginated App API surface.
        require_space_access(runtime, context, space_id).await?;
        return Err(group_managed_space_controlled_by_im());
    }
    let actor_id = require_actor_id(context)?;
    let file_registry = OkfBundleFileRegistryService::new(runtime.okf_bundle_file_store());
    let okf_initializer = OkfBundleInitializerService::new(runtime.drive_storage())
        .with_registry(&file_registry)
        .with_drive_workspace(runtime.drive_workspace());
    let normalized_page_size = crate::pagination::normalize_api_page_size(page_size)?;
    let members = space_service(runtime, &okf_initializer)
        .list_space_members(
            space_id,
            &context.tenant_id.to_string(),
            &actor_id,
            cursor,
            Some(normalized_page_size),
        )
        .await
        .map_err(ApiError::from)?;
    Ok(crate::pagination::cursor_page_data(
        members
            .members
            .into_iter()
            .map(map_contract_member)
            .collect(),
        members.next_cursor.clone(),
        members.next_cursor.is_some(),
        normalized_page_size,
    ))
}

pub(crate) async fn list_space_members_admin_with_runtime(
    runtime: &KnowledgebaseRuntime,
    space_id: u64,
    cursor: Option<String>,
    page_size: Option<u32>,
) -> ApiResult<sdkwork_utils_rust::SdkWorkPageData<KnowledgeSpaceMember>> {
    if is_group_managed_space(
        runtime,
        GroupKnowledgeSpaceScope {
            tenant_id: runtime.tenant_id(),
            organization_id: runtime.organization_id(),
        },
        space_id,
    )
    .await?
    {
        return Err(group_managed_space_controlled_by_im());
    }
    let file_registry = OkfBundleFileRegistryService::new(runtime.okf_bundle_file_store());
    let okf_initializer = OkfBundleInitializerService::new(runtime.drive_storage())
        .with_registry(&file_registry)
        .with_drive_workspace(runtime.drive_workspace());
    let normalized_page_size = crate::pagination::normalize_api_page_size(page_size)?;
    let members = space_service(runtime, &okf_initializer)
        .list_space_members_admin(
            space_id,
            runtime.tenant_id_str(),
            cursor,
            Some(normalized_page_size),
        )
        .await
        .map_err(ApiError::from)?;
    Ok(crate::pagination::cursor_page_data(
        members
            .members
            .into_iter()
            .map(map_contract_member)
            .collect(),
        members.next_cursor.clone(),
        members.next_cursor.is_some(),
        normalized_page_size,
    ))
}

pub(crate) async fn grant_space_member_with_context(
    runtime: &KnowledgebaseRuntime,
    context: &KnowledgeAppRequestContext,
    space_id: u64,
    request: GrantKnowledgeSpaceMemberRequest,
) -> ApiResult<()> {
    ensure_runtime_tenant(runtime, context)?;
    if is_group_managed_space(
        runtime,
        GroupKnowledgeSpaceScope {
            tenant_id: context.tenant_id,
            organization_id: context.organization_id.unwrap_or(0),
        },
        space_id,
    )
    .await?
    {
        return Err(group_managed_space_controlled_by_im());
    }
    let actor_id = require_actor_id(context)?;
    if is_blank(Some(request.subject_id.as_str())) {
        return Err(ApiError::new(
            StatusCode::BAD_REQUEST,
            "invalid_subject_id",
            "subjectId must not be blank",
        ));
    }
    let file_registry = OkfBundleFileRegistryService::new(runtime.okf_bundle_file_store());
    let okf_initializer = OkfBundleInitializerService::new(runtime.drive_storage())
        .with_registry(&file_registry)
        .with_drive_workspace(runtime.drive_workspace());
    space_service(runtime, &okf_initializer)
        .grant_space_member(
            space_id,
            &context.tenant_id.to_string(),
            parse_member_subject_type(request.subject_type),
            request.subject_id.trim(),
            parse_member_role(request.role),
            &actor_id,
        )
        .await
        .map_err(ApiError::from)?;
    sdkwork_knowledgebase_observability::audit::record_space_member_granted(
        space_id,
        context.actor_id.unwrap_or(0),
        member_subject_type_label(request.subject_type),
        request.subject_id.trim(),
        member_role_label(request.role),
    )
    .await
    .map_err(ApiError::from)?;
    Ok(())
}

pub(crate) async fn revoke_space_member_with_context(
    runtime: &KnowledgebaseRuntime,
    context: &KnowledgeAppRequestContext,
    space_id: u64,
    subject_type: KnowledgeSpaceMemberSubjectType,
    subject_id: &str,
) -> ApiResult<()> {
    ensure_runtime_tenant(runtime, context)?;
    if is_group_managed_space(
        runtime,
        GroupKnowledgeSpaceScope {
            tenant_id: context.tenant_id,
            organization_id: context.organization_id.unwrap_or(0),
        },
        space_id,
    )
    .await?
    {
        return Err(group_managed_space_controlled_by_im());
    }
    let actor_id = require_actor_id(context)?;
    if is_blank(Some(subject_id)) {
        return Err(ApiError::new(
            StatusCode::BAD_REQUEST,
            "invalid_subject_id",
            "subjectId must not be blank",
        ));
    }
    let file_registry = OkfBundleFileRegistryService::new(runtime.okf_bundle_file_store());
    let okf_initializer = OkfBundleInitializerService::new(runtime.drive_storage())
        .with_registry(&file_registry)
        .with_drive_workspace(runtime.drive_workspace());
    space_service(runtime, &okf_initializer)
        .revoke_space_member(
            space_id,
            &context.tenant_id.to_string(),
            parse_member_subject_type(subject_type),
            subject_id.trim(),
            &actor_id,
        )
        .await
        .map_err(ApiError::from)?;
    sdkwork_knowledgebase_observability::audit::record_space_member_revoked(
        space_id,
        context.actor_id.unwrap_or(0),
        member_subject_type_label(subject_type),
        subject_id.trim(),
    )
    .await
    .map_err(ApiError::from)?;
    Ok(())
}

pub(crate) async fn require_enabled_agent_bindings_space_access(
    runtime: &KnowledgebaseRuntime,
    context: &KnowledgeAppRequestContext,
    bindings: &[KnowledgeAgentBinding],
) -> ApiResult<()> {
    require_enabled_agent_bindings_space_access_with_role(
        runtime,
        context,
        bindings,
        KnowledgeAccessRole::Reader,
    )
    .await
}

pub(crate) async fn require_enabled_agent_bindings_space_access_with_role(
    runtime: &KnowledgebaseRuntime,
    context: &KnowledgeAppRequestContext,
    bindings: &[KnowledgeAgentBinding],
    required_role: KnowledgeAccessRole,
) -> ApiResult<()> {
    let retrieval_bindings: Vec<KnowledgeRetrievalBinding> = bindings
        .iter()
        .filter(|binding| binding.enabled)
        .map(|binding| KnowledgeRetrievalBinding {
            space_id: binding.space_id,
            collection_id: binding.collection_id,
            source_filter: binding.source_filter.clone(),
            document_filter: binding.document_filter.clone(),
            priority: binding.priority,
            top_k: binding.top_k,
            min_score: binding.min_score,
        })
        .collect();
    require_bindings_space_access_with_role(runtime, context, &retrieval_bindings, required_role)
        .await
}

pub(crate) async fn require_agent_profile_space_access(
    runtime: &KnowledgebaseRuntime,
    context: &KnowledgeAppRequestContext,
    profile: &KnowledgeAgentProfile,
) -> ApiResult<()> {
    ensure_agent_profile_tenant(runtime, context, profile)?;
    require_enabled_agent_bindings_space_access(runtime, context, &profile.bindings).await
}

fn ensure_agent_profile_tenant(
    runtime: &KnowledgebaseRuntime,
    context: &KnowledgeAppRequestContext,
    profile: &KnowledgeAgentProfile,
) -> ApiResult<()> {
    ensure_runtime_tenant(runtime, context)?;
    if profile.tenant_id != context.tenant_id {
        return Err(ApiError::new(
            StatusCode::FORBIDDEN,
            "tenant_id_mismatch",
            "agent profile tenant does not match authenticated tenant",
        ));
    }
    Ok(())
}

pub(crate) async fn require_agent_profile_space_access_with_role(
    runtime: &KnowledgebaseRuntime,
    context: &KnowledgeAppRequestContext,
    profile: &KnowledgeAgentProfile,
    required_role: KnowledgeAccessRole,
) -> ApiResult<()> {
    ensure_agent_profile_tenant(runtime, context, profile)?;
    require_enabled_agent_bindings_space_access_with_role(
        runtime,
        context,
        &profile.bindings,
        required_role,
    )
    .await
}

pub(crate) async fn require_agent_binding_space_access_with_role(
    runtime: &KnowledgebaseRuntime,
    context: &KnowledgeAppRequestContext,
    space_id: u64,
    required_role: KnowledgeAccessRole,
) -> ApiResult<()> {
    require_space_access_with_role(runtime, context, space_id, required_role).await?;
    Ok(())
}

fn member_role_label(role: KnowledgeSpaceMemberRole) -> &'static str {
    match role {
        KnowledgeSpaceMemberRole::Reader => "reader",
        KnowledgeSpaceMemberRole::Writer => "writer",
        KnowledgeSpaceMemberRole::Owner => "owner",
    }
}

fn member_subject_type_label(subject_type: KnowledgeSpaceMemberSubjectType) -> &'static str {
    match subject_type {
        KnowledgeSpaceMemberSubjectType::User => "user",
        KnowledgeSpaceMemberSubjectType::Group => "group",
        KnowledgeSpaceMemberSubjectType::Domain => "domain",
        KnowledgeSpaceMemberSubjectType::App => "app",
    }
}
