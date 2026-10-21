use sdkwork_intelligence_knowledgebase_repository_sqlx::{
    db::connect_knowledgebase_any_pool_from_url,
    SqlxKnowledgeEngineProviderBindingReadinessStore,
};
use sdkwork_intelligence_knowledgebase_service::ports::knowledge_provider_binding_readiness_store::{
    KnowledgeEngineProviderBindingReadinessGap,
    KnowledgeEngineProviderBindingReadinessStore,
    ListKnowledgeEngineProviderBindingReadinessGapsRequest,
};
use sdkwork_intelligence_knowledgebase_service::ports::knowledge_provider_binding_store::KnowledgeEngineProviderScope;
use sdkwork_knowledgebase_contract::{
    parse_canonical_nonnegative_signed_i64, parse_canonical_positive_signed_i64,
};
use sdkwork_utils_rust::{is_blank, DEFAULT_LIST_PAGE_SIZE, MAX_LIST_PAGE_SIZE};
use serde::Serialize;
use thiserror::Error;
use time::{format_description::well_known::Rfc3339, OffsetDateTime};

const REPORT_KIND: &str = "sdkwork.knowledgebase.provider-binding-prelaunch-report";
const REPORT_SCHEMA_VERSION: &str = "1.0.0";

#[tokio::main]
async fn main() {
    match run(std::env::args().skip(1)).await {
        Ok(()) => {}
        Err(ReportCommandError::HelpRequested) => {
            println!("{}", usage());
        }
        Err(error) => {
            eprintln!("provider Binding prelaunch report failed: {error}");
            std::process::exit(2);
        }
    }
}

async fn run(arguments: impl IntoIterator<Item = String>) -> Result<(), ReportCommandError> {
    let arguments = ReportArguments::parse(arguments)?;
    let database_url = std::env::var("SDKWORK_DATABASE_URL")
        .map_err(|_| ReportCommandError::DatabaseUrlMissing)?;
    let tenant_id = required_tenant_id()?;
    let scope = KnowledgeEngineProviderScope {
        tenant_id,
        organization_id: arguments.organization_id,
    };

    let pool = connect_knowledgebase_any_pool_from_url(&database_url)
        .await
        .map_err(|_| ReportCommandError::DatabaseConnectionFailed)?;
    let store = SqlxKnowledgeEngineProviderBindingReadinessStore::new(pool.clone());
    let page = store
        .list_spaces_missing_active_binding(
            scope,
            ListKnowledgeEngineProviderBindingReadinessGapsRequest {
                cursor: arguments.cursor,
                page_size: Some(arguments.page_size),
            },
        )
        .await
        .map_err(|error| ReportCommandError::ReadinessQuery(error.to_string()))?;
    pool.close().await;

    let report = ProviderBindingPrelaunchReport {
        kind: REPORT_KIND,
        schema_version: REPORT_SCHEMA_VERSION,
        classification: "informational-read-only",
        generated_at: OffsetDateTime::now_utc()
            .format(&Rfc3339)
            .map_err(|_| ReportCommandError::ReportSerializationFailed)?,
        tenant_id: scope.tenant_id,
        organization_id: scope.organization_id,
        criteria: ReportCriteria {
            knowledge_mode: "external",
            space_status: "active",
            required_binding_lifecycle: "active",
        },
        safety: ReportSafety {
            read_only: true,
            binding_inference_applied: false,
            source_order_considered: false,
            credential_material_included: false,
            remote_resource_identifiers_included: false,
        },
        items: page.items,
        page_info: ReportPageInfo {
            mode: "cursor",
            page_size: arguments.page_size,
            has_more: page.next_cursor.is_some(),
            next_cursor: page.next_cursor,
        },
    };
    let json = serde_json::to_string_pretty(&report)
        .map_err(|_| ReportCommandError::ReportSerializationFailed)?;
    println!("{json}");
    Ok(())
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct ReportArguments {
    organization_id: u64,
    page_size: u32,
    cursor: Option<String>,
}

impl ReportArguments {
    fn parse(arguments: impl IntoIterator<Item = String>) -> Result<Self, ReportCommandError> {
        let mut organization_id = None;
        let mut page_size = DEFAULT_LIST_PAGE_SIZE as u32;
        let mut page_size_seen = false;
        let mut cursor = None;
        let mut arguments = arguments.into_iter();
        while let Some(argument) = arguments.next() {
            match argument.as_str() {
                "--help" | "-h" => return Err(ReportCommandError::HelpRequested),
                "--organization-id" if organization_id.is_none() => {
                    let value =
                        arguments
                            .next()
                            .ok_or(ReportCommandError::MissingArgumentValue(
                                "--organization-id",
                            ))?;
                    organization_id = Some(
                        parse_canonical_nonnegative_signed_i64(&value)
                            .map_err(|_| ReportCommandError::InvalidOrganizationId)?,
                    );
                }
                "--page-size" if !page_size_seen => {
                    let value = arguments
                        .next()
                        .ok_or(ReportCommandError::MissingArgumentValue("--page-size"))?;
                    page_size = value
                        .parse::<u32>()
                        .ok()
                        .filter(|value| (1..=MAX_LIST_PAGE_SIZE as u32).contains(value))
                        .ok_or(ReportCommandError::InvalidPageSize)?;
                    page_size_seen = true;
                }
                "--cursor" if cursor.is_none() => {
                    let value = arguments
                        .next()
                        .ok_or(ReportCommandError::MissingArgumentValue("--cursor"))?;
                    if is_blank(Some(value.as_str())) {
                        return Err(ReportCommandError::InvalidCursor);
                    }
                    cursor = Some(value);
                }
                _ => return Err(ReportCommandError::UnexpectedArgument(argument)),
            }
        }
        Ok(Self {
            organization_id: organization_id.ok_or(ReportCommandError::OrganizationIdMissing)?,
            page_size,
            cursor,
        })
    }
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct ProviderBindingPrelaunchReport {
    kind: &'static str,
    schema_version: &'static str,
    classification: &'static str,
    generated_at: String,
    #[serde(with = "sdkwork_utils_rust::serde_uint64")]
    tenant_id: u64,
    #[serde(with = "sdkwork_utils_rust::serde_uint64")]
    organization_id: u64,
    criteria: ReportCriteria,
    safety: ReportSafety,
    items: Vec<KnowledgeEngineProviderBindingReadinessGap>,
    page_info: ReportPageInfo,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct ReportCriteria {
    knowledge_mode: &'static str,
    space_status: &'static str,
    required_binding_lifecycle: &'static str,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct ReportSafety {
    read_only: bool,
    binding_inference_applied: bool,
    source_order_considered: bool,
    credential_material_included: bool,
    remote_resource_identifiers_included: bool,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct ReportPageInfo {
    mode: &'static str,
    page_size: u32,
    has_more: bool,
    next_cursor: Option<String>,
}

#[derive(Debug, Error, PartialEq, Eq)]
enum ReportCommandError {
    #[error("help requested")]
    HelpRequested,
    #[error("--organization-id is required\n{}", usage())]
    OrganizationIdMissing,
    #[error("--organization-id must be a canonical nonnegative signed BIGINT")]
    InvalidOrganizationId,
    #[error("--page-size must be between 1 and {MAX_LIST_PAGE_SIZE}")]
    InvalidPageSize,
    #[error("--cursor must not be empty")]
    InvalidCursor,
    #[error("{0} requires a value")]
    MissingArgumentValue(&'static str),
    #[error("unexpected or repeated argument: {0}\n{usage}", usage = usage())]
    UnexpectedArgument(String),
    #[error("SDKWORK_DATABASE_URL is required")]
    DatabaseUrlMissing,
    #[error("SDKWORK_KNOWLEDGEBASE_TENANT_ID is required")]
    TenantIdMissing,
    #[error("SDKWORK_KNOWLEDGEBASE_TENANT_ID must be a canonical positive signed BIGINT")]
    InvalidTenantId,
    #[error("database connection failed")]
    DatabaseConnectionFailed,
    #[error("{0}")]
    ReadinessQuery(String),
    #[error("report serialization failed")]
    ReportSerializationFailed,
}

fn required_tenant_id() -> Result<u64, ReportCommandError> {
    let value = std::env::var("SDKWORK_KNOWLEDGEBASE_TENANT_ID")
        .map_err(|_| ReportCommandError::TenantIdMissing)?;
    parse_canonical_positive_signed_i64(&value).map_err(|_| ReportCommandError::InvalidTenantId)
}

const fn usage() -> &'static str {
    "usage: sdkwork-knowledgebase-provider-binding-prelaunch-report --organization-id <id> [--page-size <1..200>] [--cursor <opaque-token>]"
}

#[cfg(test)]
mod tests {
    use super::{ReportArguments, ReportCommandError};

    fn strings(values: &[&str]) -> Vec<String> {
        values.iter().map(|value| (*value).to_string()).collect()
    }

    #[test]
    fn arguments_require_an_explicit_organization_scope() {
        assert_eq!(
            ReportArguments::parse(Vec::<String>::new()),
            Err(ReportCommandError::OrganizationIdMissing)
        );
        assert_eq!(
            ReportArguments::parse(strings(&["--organization-id", "7"])),
            Ok(ReportArguments {
                organization_id: 7,
                page_size: 20,
                cursor: None,
            })
        );
    }

    #[test]
    fn arguments_reject_ambiguous_or_unbounded_inputs() {
        for arguments in [
            strings(&["--organization-id", "01"]),
            strings(&["--organization-id", "7", "--page-size", "0"]),
            strings(&["--organization-id", "7", "--page-size", "201"]),
            strings(&["--organization-id", "7", "--cursor", ""]),
            strings(&[
                "--organization-id",
                "7",
                "--page-size",
                "20",
                "--page-size",
                "20",
            ]),
            strings(&[
                "--organization-id",
                "7",
                "--cursor",
                "opaque-a",
                "--cursor",
                "opaque-b",
            ]),
            strings(&["--organization-id", "7", "--organization-id", "8"]),
        ] {
            assert!(ReportArguments::parse(arguments).is_err());
        }
    }

    #[test]
    fn arguments_accept_a_bounded_page_and_opaque_cursor() {
        assert_eq!(
            ReportArguments::parse(strings(&[
                "--organization-id",
                "0",
                "--page-size",
                "50",
                "--cursor",
                "opaque-token",
            ])),
            Ok(ReportArguments {
                organization_id: 0,
                page_size: 50,
                cursor: Some("opaque-token".to_string()),
            })
        );
    }
}
