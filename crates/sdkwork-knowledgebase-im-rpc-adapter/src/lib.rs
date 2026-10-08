//! Trusted IM RPC composition for Knowledgebase group knowledge-space launches.
//!
//! This crate is intentionally outside the Knowledgebase domain service. It converts an
//! already-authenticated HTTP request context into framework-signed RPC metadata and invokes the
//! generated IM client over mTLS. It never serializes caller authority into the ticket payload.

use std::time::Duration;

use async_trait::async_trait;
use sdkwork_im_rpc_sdk_rust::sdkwork::communication::internal::v1::{
    group_knowledgebase_launch_ticket_service_client::GroupKnowledgebaseLaunchTicketServiceClient,
    ConsumeGroupKnowledgebaseLaunchTicketRequest, ConsumeGroupKnowledgebaseLaunchTicketResponse,
};
use sdkwork_intelligence_knowledgebase_service::ports::group_launch_ticket_consumer::{
    ConsumeGroupLaunchTicketCommand, ConsumedGroupLaunchTicket, GroupLaunchTicketCallerContext,
    GroupLaunchTicketConsumer, GroupLaunchTicketConsumerError,
};
use sdkwork_knowledgebase_contract::{
    group_space::{
        GroupKnowledgeSpaceLifecycleState, GroupKnowledgeSpaceMemberRole,
        GroupKnowledgeSpacePrincipalKind,
    },
    parse_canonical_nonnegative_signed_i64, parse_canonical_positive_signed_i64,
};
use sdkwork_rpc_client::{
    connect_grpc_channel_with_config, GrpcChannelConfig, RpcServiceCredentialProvider,
    RpcTlsConfig, SignedRpcServiceCredentialProvider,
};
use sdkwork_rpc_framework_core::{
    RpcCallerActorKind, RpcCallerContext, RpcCallerContextSigner, RpcCallerContextSigningKey,
    RpcFrameworkError,
};
use sdkwork_utils_rust::is_blank;
use thiserror::Error;
use tonic::{transport::Channel, Code, Status};
use url::Url;

const KNOWLEDGEBASE_SERVICE_ID: &str = "sdkwork-knowledgebase";
const IM_SERVICE_ID: &str = "sdkwork-im";
const MIN_REQUEST_TIMEOUT_MS: u64 = 100;
const MAX_REQUEST_TIMEOUT_MS: u64 = 60_000;

/// Typed, bootstrap-owned configuration for the outbound IM ticket consumer.
///
/// All production fields are mandatory. The executable parses environment/secrets into this
/// type before creating the adapter so shared adapter and domain code do not read process state.
#[derive(Clone, Debug)]
pub struct KnowledgebaseImGroupLaunchTicketConsumerConfig {
    endpoint: String,
    tls: RpcTlsConfig,
    caller_context_signing_key: RpcCallerContextSigningKey,
    credential_ttl: Duration,
    request_timeout: Duration,
}

impl KnowledgebaseImGroupLaunchTicketConsumerConfig {
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        endpoint: impl Into<String>,
        server_ca_certificate_path: impl Into<std::path::PathBuf>,
        client_certificate_path: impl Into<std::path::PathBuf>,
        client_private_key_path: impl Into<std::path::PathBuf>,
        tls_domain: impl Into<String>,
        caller_context_signing_key: RpcCallerContextSigningKey,
        credential_ttl: Duration,
        request_timeout: Duration,
    ) -> Result<Self, KnowledgebaseImRpcAdapterError> {
        let endpoint = validate_secure_endpoint(endpoint.into())?;
        let tls_domain = tls_domain.into();
        if is_blank(Some(tls_domain.as_str())) {
            return Err(KnowledgebaseImRpcAdapterError::Configuration(
                "IM RPC TLS domain is required".to_string(),
            ));
        }
        if credential_ttl.is_zero() || credential_ttl > Duration::from_secs(300) {
            return Err(KnowledgebaseImRpcAdapterError::Configuration(
                "IM RPC caller-context credential TTL must be between 1 and 300 seconds"
                    .to_string(),
            ));
        }
        let request_timeout_ms = u64::try_from(request_timeout.as_millis()).unwrap_or(u64::MAX);
        if !(MIN_REQUEST_TIMEOUT_MS..=MAX_REQUEST_TIMEOUT_MS).contains(&request_timeout_ms) {
            return Err(KnowledgebaseImRpcAdapterError::Configuration(format!(
                "IM RPC request timeout must be between {MIN_REQUEST_TIMEOUT_MS} and {MAX_REQUEST_TIMEOUT_MS} milliseconds"
            )));
        }

        let tls = RpcTlsConfig::server_verified()
            .with_server_ca(server_ca_certificate_path)
            .with_client_identity(client_certificate_path, client_private_key_path)
            .with_domain(tls_domain.trim());
        tls.validate()
            .map_err(KnowledgebaseImRpcAdapterError::RpcFramework)?;

        Ok(Self {
            endpoint,
            tls,
            caller_context_signing_key,
            credential_ttl,
            request_timeout,
        })
    }
}

/// Concurrent, generated-client implementation of the Knowledgebase ticket-consumer port.
#[derive(Clone)]
pub struct KnowledgebaseImGroupLaunchTicketConsumer {
    client: GroupKnowledgebaseLaunchTicketServiceClient<Channel>,
    credential_provider: SignedRpcServiceCredentialProvider,
    request_timeout: Duration,
}

impl KnowledgebaseImGroupLaunchTicketConsumer {
    pub async fn connect(
        config: KnowledgebaseImGroupLaunchTicketConsumerConfig,
    ) -> Result<Self, KnowledgebaseImRpcAdapterError> {
        let signer = RpcCallerContextSigner::new(
            KNOWLEDGEBASE_SERVICE_ID,
            config.caller_context_signing_key,
        )
        .map_err(KnowledgebaseImRpcAdapterError::RpcFramework)?;
        let credential_provider =
            SignedRpcServiceCredentialProvider::new(signer, config.credential_ttl)
                .map_err(KnowledgebaseImRpcAdapterError::RpcFramework)?;
        let channel = connect_grpc_channel_with_config(
            &config.endpoint,
            &GrpcChannelConfig {
                connect_timeout: config.request_timeout,
                tls: Some(config.tls),
                ..GrpcChannelConfig::default()
            },
        )
        .await
        .map_err(KnowledgebaseImRpcAdapterError::RpcFramework)?;

        Ok(Self {
            client: GroupKnowledgebaseLaunchTicketServiceClient::new(channel),
            credential_provider,
            request_timeout: config.request_timeout,
        })
    }

    fn signed_caller_context(
        caller: &GroupLaunchTicketCallerContext,
    ) -> Result<RpcCallerContext, GroupLaunchTicketConsumerError> {
        if caller.principal_kind != GroupKnowledgeSpacePrincipalKind::User {
            return Err(GroupLaunchTicketConsumerError::Unauthorized);
        }
        let session_id = caller
            .session_id
            .as_deref()
            .filter(|value| !is_blank(Some(value)))
            .ok_or(GroupLaunchTicketConsumerError::Unauthorized)?;
        if !is_positive_signed_bigint(caller.tenant_id)
            || !is_nonnegative_signed_bigint(caller.organization_id)
            || is_blank(Some(caller.actor_id.as_str()))
            || is_blank(Some(caller.request_id.as_str()))
            || is_blank(Some(caller.trace_id.as_str()))
            || is_blank(Some(caller.idempotency_key.as_str()))
        {
            return Err(GroupLaunchTicketConsumerError::Unauthorized);
        }

        RpcCallerContext::builder()
            .tenant_id(caller.tenant_id.to_string())
            .organization_id(caller.organization_id.to_string())
            .actor_id(caller.actor_id.clone())
            .actor_kind(RpcCallerActorKind::User)
            .session_id(session_id)
            .request_id(caller.request_id.clone())
            .trace_id(caller.trace_id.clone())
            .idempotency_key(caller.idempotency_key.clone())
            .audience_service_id(IM_SERVICE_ID)
            .build()
            .map_err(|_| GroupLaunchTicketConsumerError::Unauthorized)
    }
}

#[async_trait]
impl GroupLaunchTicketConsumer for KnowledgebaseImGroupLaunchTicketConsumer {
    async fn consume_group_launch_ticket(
        &self,
        command: ConsumeGroupLaunchTicketCommand,
    ) -> Result<ConsumedGroupLaunchTicket, GroupLaunchTicketConsumerError> {
        let caller_context = Self::signed_caller_context(&command.caller)?;
        let credential = self
            .credential_provider
            .issue(caller_context)
            .map_err(|_| {
                GroupLaunchTicketConsumerError::Upstream(
                    "could not issue trusted IM RPC caller context".to_string(),
                )
            })?;
        let mut request = tonic::Request::new(ConsumeGroupKnowledgebaseLaunchTicketRequest {
            ticket: command.ticket,
            metadata: None,
        });
        request.set_timeout(self.request_timeout);
        credential.apply_to(request.metadata_mut()).map_err(|_| {
            GroupLaunchTicketConsumerError::Upstream(
                "could not apply trusted IM RPC caller context".to_string(),
            )
        })?;

        let mut client = self.client.clone();
        let response = tokio::time::timeout(
            self.request_timeout,
            client.consume_group_knowledgebase_launch_ticket(request),
        )
        .await
        .map_err(|_| GroupLaunchTicketConsumerError::Unavailable)?
        .map_err(map_ticket_consumer_status)?
        .into_inner();

        consumed_ticket_from_response(command.caller, response)
    }
}

fn validate_secure_endpoint(endpoint: String) -> Result<String, KnowledgebaseImRpcAdapterError> {
    let endpoint = endpoint.trim().to_string();
    if is_blank(Some(endpoint.as_str())) {
        return Err(KnowledgebaseImRpcAdapterError::Configuration(
            "IM RPC endpoint is required".to_string(),
        ));
    }
    let parsed = Url::parse(&endpoint).map_err(|_| {
        KnowledgebaseImRpcAdapterError::Configuration(
            "IM RPC endpoint must be an absolute URL".to_string(),
        )
    })?;
    if !matches!(parsed.scheme(), "https" | "grpcs") || parsed.host_str().is_none() {
        return Err(KnowledgebaseImRpcAdapterError::Configuration(
            "IM RPC endpoint must use https:// or grpcs:// with a host".to_string(),
        ));
    }
    if !parsed.username().is_empty()
        || parsed.password().is_some()
        || parsed.query().is_some()
        || parsed.fragment().is_some()
    {
        return Err(KnowledgebaseImRpcAdapterError::Configuration(
            "IM RPC endpoint must not contain credentials, query parameters, or fragments"
                .to_string(),
        ));
    }
    Ok(endpoint)
}

fn map_ticket_consumer_status(status: Status) -> GroupLaunchTicketConsumerError {
    match status.code() {
        Code::Unauthenticated => GroupLaunchTicketConsumerError::InvalidOrExpired,
        Code::PermissionDenied => GroupLaunchTicketConsumerError::Unauthorized,
        Code::Unavailable | Code::DeadlineExceeded | Code::Cancelled => {
            GroupLaunchTicketConsumerError::Unavailable
        }
        _ => GroupLaunchTicketConsumerError::Upstream(
            "IM ticket consumer returned an unexpected RPC status".to_string(),
        ),
    }
}

fn consumed_ticket_from_response(
    caller: GroupLaunchTicketCallerContext,
    response: ConsumeGroupKnowledgebaseLaunchTicketResponse,
) -> Result<ConsumedGroupLaunchTicket, GroupLaunchTicketConsumerError> {
    if response.lifecycle_state != GroupKnowledgeSpaceLifecycleState::Active.as_str()
        || is_blank(Some(response.conversation_id.as_str()))
        || is_blank(Some(response.knowledgebase_binding_uuid.as_str()))
        || is_blank(Some(response.space_uuid.as_str()))
        || is_blank(Some(response.expires_at.as_str()))
    {
        return Err(GroupLaunchTicketConsumerError::Upstream(
            "IM ticket consumer returned an invalid launch target".to_string(),
        ));
    }
    let knowledgebase_binding_id = parse_response_u64(
        "knowledgebase_binding_id",
        &response.knowledgebase_binding_id,
    )?;
    let space_id = parse_response_u64("space_id", &response.space_id)?;
    // Epochs and link generations are nonnegative fences (0 is a valid initial value), unlike
    // resource ids, which must stay positive.
    let membership_epoch = parse_response_nonnegative_u64(
        "membership_epoch",
        &response.membership_epoch,
    )?;
    let upstream_link_generation = parse_response_nonnegative_u64(
        "upstream_link_generation",
        &response.upstream_link_generation,
    )?;
    let membership_role = response
        .membership_role
        .parse::<GroupKnowledgeSpaceMemberRole>()
        .map_err(|_| {
            GroupLaunchTicketConsumerError::Upstream(
                "IM ticket consumer returned an invalid membership role".to_string(),
            )
        })?;

    Ok(ConsumedGroupLaunchTicket {
        tenant_id: caller.tenant_id,
        organization_id: caller.organization_id,
        principal_kind: caller.principal_kind,
        actor_id: caller.actor_id,
        conversation_id: response.conversation_id,
        knowledgebase_binding_id,
        knowledgebase_binding_uuid: response.knowledgebase_binding_uuid,
        space_id,
        space_uuid: response.space_uuid,
        membership_epoch,
        upstream_link_generation,
        membership_role,
        expires_at: response.expires_at,
    })
}

fn parse_response_u64(field: &str, value: &str) -> Result<u64, GroupLaunchTicketConsumerError> {
    parse_canonical_positive_signed_i64(value).map_err(|_| {
        GroupLaunchTicketConsumerError::Upstream(format!(
            "IM ticket consumer returned a noncanonical {field}"
        ))
    })
}

fn parse_response_nonnegative_u64(
    field: &str,
    value: &str,
) -> Result<u64, GroupLaunchTicketConsumerError> {
    parse_canonical_nonnegative_signed_i64(value).map_err(|_| {
        GroupLaunchTicketConsumerError::Upstream(format!(
            "IM ticket consumer returned a noncanonical {field}"
        ))
    })
}

fn is_positive_signed_bigint(value: u64) -> bool {
    (1..=i64::MAX as u64).contains(&value)
}

fn is_nonnegative_signed_bigint(value: u64) -> bool {
    value <= i64::MAX as u64
}

#[derive(Debug, Error)]
pub enum KnowledgebaseImRpcAdapterError {
    #[error("invalid Knowledgebase IM RPC configuration: {0}")]
    Configuration(String),
    #[error(transparent)]
    RpcFramework(#[from] RpcFrameworkError),
}

#[cfg(test)]
mod tests {
    use super::*;

    fn caller() -> GroupLaunchTicketCallerContext {
        GroupLaunchTicketCallerContext {
            tenant_id: 1,
            organization_id: 2,
            principal_kind: GroupKnowledgeSpacePrincipalKind::User,
            actor_id: "7".to_string(),
            session_id: Some("session-1".to_string()),
            request_id: "request-1".to_string(),
            trace_id: "trace-1".to_string(),
            idempotency_key: "idempotency-1".to_string(),
        }
    }

    #[test]
    fn rejects_plaintext_or_ambiguous_endpoints() {
        for endpoint in [
            "http://im.internal:7443",
            "grpc://im.internal:7443",
            "im.internal",
        ] {
            assert!(matches!(
                validate_secure_endpoint(endpoint.to_string()),
                Err(KnowledgebaseImRpcAdapterError::Configuration(_))
            ));
        }
        assert_eq!(
            validate_secure_endpoint("grpcs://im.internal:7443".to_string())
                .expect("secure endpoint"),
            "grpcs://im.internal:7443"
        );
    }

    #[test]
    fn trusted_context_keeps_only_authenticated_caller_values() {
        let context = KnowledgebaseImGroupLaunchTicketConsumer::signed_caller_context(&caller())
            .expect("caller context");
        assert_eq!(context.tenant_id, "1");
        assert_eq!(context.organization_id, "2");
        assert_eq!(context.actor_id, "7");
        assert_eq!(context.session_id.as_deref(), Some("session-1"));
        assert_eq!(context.trace_id.as_deref(), Some("trace-1"));
        assert_eq!(context.idempotency_key.as_deref(), Some("idempotency-1"));
        assert_eq!(context.audience_service_id, IM_SERVICE_ID);
    }

    #[test]
    fn rejects_missing_or_invalid_required_caller_scope() {
        let mut missing_trace = caller();
        missing_trace.trace_id.clear();
        assert!(matches!(
            KnowledgebaseImGroupLaunchTicketConsumer::signed_caller_context(&missing_trace),
            Err(GroupLaunchTicketConsumerError::Unauthorized)
        ));

        let mut missing_idempotency_key = caller();
        missing_idempotency_key.idempotency_key.clear();
        assert!(matches!(
            KnowledgebaseImGroupLaunchTicketConsumer::signed_caller_context(
                &missing_idempotency_key
            ),
            Err(GroupLaunchTicketConsumerError::Unauthorized)
        ));

        let mut tenant_scope = caller();
        tenant_scope.organization_id = 0;
        let tenant_context =
            KnowledgebaseImGroupLaunchTicketConsumer::signed_caller_context(&tenant_scope)
                .expect("tenant-scoped caller context");
        assert_eq!(tenant_context.organization_id, "0");

        let mut tenant_overflow = caller();
        tenant_overflow.tenant_id = i64::MAX as u64 + 1;
        assert!(matches!(
            KnowledgebaseImGroupLaunchTicketConsumer::signed_caller_context(&tenant_overflow),
            Err(GroupLaunchTicketConsumerError::Unauthorized)
        ));

        let mut organization_overflow = caller();
        organization_overflow.organization_id = i64::MAX as u64 + 1;
        assert!(matches!(
            KnowledgebaseImGroupLaunchTicketConsumer::signed_caller_context(&organization_overflow),
            Err(GroupLaunchTicketConsumerError::Unauthorized)
        ));
    }

    #[test]
    fn response_uses_trusted_caller_scope_and_validates_im_fields() {
        let response = ConsumeGroupKnowledgebaseLaunchTicketResponse {
            conversation_id: "conversation-1".to_string(),
            space_id: "123".to_string(),
            space_uuid: "space-uuid".to_string(),
            lifecycle_state: "active".to_string(),
            membership_role: "admin".to_string(),
            membership_epoch: "4".to_string(),
            upstream_link_generation: "5".to_string(),
            expires_at: "2026-07-13T00:00:00Z".to_string(),
            knowledgebase_binding_id: "122".to_string(),
            knowledgebase_binding_uuid: "binding-uuid".to_string(),
            metadata: None,
        };
        let consumed = consumed_ticket_from_response(caller(), response).expect("response");
        assert_eq!(consumed.tenant_id, 1);
        assert_eq!(consumed.organization_id, 2);
        assert_eq!(
            consumed.membership_role,
            GroupKnowledgeSpaceMemberRole::Admin
        );
        assert_eq!(consumed.space_id, 123);
        assert_eq!(consumed.knowledgebase_binding_id, 122);
        assert_eq!(consumed.upstream_link_generation, 5);
    }

    #[test]
    fn maps_transport_statuses_without_disclosing_ticket_data() {
        assert_eq!(
            map_ticket_consumer_status(Status::unauthenticated("no")),
            GroupLaunchTicketConsumerError::InvalidOrExpired
        );
        assert_eq!(
            map_ticket_consumer_status(Status::permission_denied("no")),
            GroupLaunchTicketConsumerError::Unauthorized
        );
        assert_eq!(
            map_ticket_consumer_status(Status::unavailable("no")),
            GroupLaunchTicketConsumerError::Unavailable
        );
    }

    #[test]
    fn ticket_response_ids_require_canonical_positive_signed_bigints() {
        for invalid in [
            "0",
            "01",
            "+1",
            " 1",
            "1 ",
            "not-a-number",
            "9223372036854775808",
        ] {
            assert!(
                parse_response_u64("space_id", invalid).is_err(),
                "{invalid:?} must not be accepted from IM"
            );
        }
        assert_eq!(
            parse_response_u64("space_id", "9223372036854775807").expect("signed max"),
            i64::MAX as u64
        );
    }

    #[test]
    fn ticket_response_epochs_are_nonnegative_and_accept_zero() {
        assert_eq!(
            parse_response_nonnegative_u64("membership_epoch", "0").expect("initial epoch"),
            0
        );
        assert_eq!(
            parse_response_nonnegative_u64("upstream_link_generation", "0")
                .expect("initial link generation"),
            0
        );
        assert_eq!(
            parse_response_nonnegative_u64("membership_epoch", "9223372036854775807")
                .expect("signed max"),
            i64::MAX as u64
        );
        for invalid in [
            "01",
            "+0",
            " 0",
            "0 ",
            "-0",
            "not-a-number",
            "9223372036854775808",
        ] {
            assert!(
                parse_response_nonnegative_u64("membership_epoch", invalid).is_err(),
                "{invalid:?} must not be accepted from IM"
            );
        }
    }

    #[test]
    fn consumed_ticket_from_response_accepts_zero_epoch_and_generation() {
        let response = ConsumeGroupKnowledgebaseLaunchTicketResponse {
            conversation_id: "conversation-1".to_string(),
            space_id: "123".to_string(),
            space_uuid: "space-uuid".to_string(),
            lifecycle_state: "active".to_string(),
            membership_role: "member".to_string(),
            membership_epoch: "0".to_string(),
            upstream_link_generation: "0".to_string(),
            expires_at: "2026-07-13T00:00:00Z".to_string(),
            knowledgebase_binding_id: "122".to_string(),
            knowledgebase_binding_uuid: "binding-uuid".to_string(),
            metadata: None,
        };
        let consumed = consumed_ticket_from_response(caller(), response).expect("response");
        assert_eq!(consumed.membership_epoch, 0);
        assert_eq!(consumed.upstream_link_generation, 0);
    }
}
