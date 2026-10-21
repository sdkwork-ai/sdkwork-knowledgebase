use sdkwork_intelligence_knowledgebase_service::{
    ports::knowledge_wiki_persistence::WikiPersistenceScope,
    wiki_backfill::{
        RunWikiPublicationBackfillRequest, WikiPublicationBackfillDisposition,
        WikiPublicationBackfillOutcome, MAX_WIKI_BACKFILL_PAGE_SIZE,
    },
};
use sdkwork_knowledgebase_contract::{
    parse_canonical_nonnegative_signed_i64, parse_canonical_positive_signed_i64,
};
use sdkwork_routes_knowledgebase_app_api::{bootstrap, KnowledgebaseRuntime};
use sdkwork_utils_rust::DEFAULT_LIST_PAGE_SIZE;
use serde::Serialize;
use thiserror::Error;
use time::{format_description::well_known::Rfc3339, OffsetDateTime};

const REPORT_KIND: &str = "sdkwork.knowledgebase.wiki-publication-backfill-report";
const REPORT_SCHEMA_VERSION: &str = "1.0.0";

#[tokio::main]
async fn main() {
    match run(std::env::args().skip(1)).await {
        Ok(false) => {}
        Ok(true) => {
            eprintln!("Wiki publication backfill stopped before the failed knowledge space");
            std::process::exit(3);
        }
        Err(BackfillCommandError::HelpRequested) => println!("{}", usage()),
        Err(error) => {
            eprintln!("Wiki publication backfill failed: {error}");
            std::process::exit(2);
        }
    }
}

async fn run(arguments: impl IntoIterator<Item = String>) -> Result<bool, BackfillCommandError> {
    let arguments = BackfillArguments::parse(arguments)?;
    let database_url = std::env::var("SDKWORK_DATABASE_URL")
        .map_err(|_| BackfillCommandError::DatabaseUrlMissing)?;
    let tenant_id = required_tenant_id()?;

    bootstrap::validate_process_config();
    let runtime = KnowledgebaseRuntime::connect(&database_url, tenant_id)
        .await
        .map_err(|_| BackfillCommandError::RuntimeInitializationFailed)?;
    let result = runtime
        .run_wiki_publication_backfill_page(RunWikiPublicationBackfillRequest {
            scope: WikiPersistenceScope {
                tenant_id,
                organization_id: arguments.organization_id,
            },
            after_space_id: arguments.after_space_id,
            excluded_space_ids: Vec::new(),
            page_size: arguments.page_size,
            actor_id: arguments.actor_id,
            dry_run: arguments.dry_run,
        })
        .await
        .map_err(|_| BackfillCommandError::ExecutionFailed)?;

    let report = WikiPublicationBackfillReport {
        kind: REPORT_KIND,
        schema_version: REPORT_SCHEMA_VERSION,
        generated_at: OffsetDateTime::now_utc()
            .format(&Rfc3339)
            .map_err(|_| BackfillCommandError::ReportSerializationFailed)?,
        tenant_id,
        organization_id: arguments.organization_id,
        actor_id: arguments.actor_id,
        dry_run: arguments.dry_run,
        outcomes: result.outcomes.iter().map(ReportOutcome::from).collect(),
        page_info: ReportPageInfo {
            mode: "keyset",
            page_size: arguments.page_size,
            next_after_space_id: result.next_after_space_id,
            stopped_on_failure: result.stopped_on_failure,
        },
    };
    println!(
        "{}",
        serde_json::to_string_pretty(&report)
            .map_err(|_| BackfillCommandError::ReportSerializationFailed)?
    );
    Ok(result.stopped_on_failure)
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct BackfillArguments {
    organization_id: u64,
    actor_id: u64,
    page_size: u32,
    after_space_id: Option<u64>,
    dry_run: bool,
}

impl BackfillArguments {
    fn parse(arguments: impl IntoIterator<Item = String>) -> Result<Self, BackfillCommandError> {
        let mut organization_id = None;
        let mut actor_id = None;
        let mut page_size = DEFAULT_LIST_PAGE_SIZE as u32;
        let mut page_size_seen = false;
        let mut after_space_id = None;
        let mut dry_run = false;
        let mut arguments = arguments.into_iter();
        while let Some(argument) = arguments.next() {
            match argument.as_str() {
                "--help" | "-h" => return Err(BackfillCommandError::HelpRequested),
                "--organization-id" if organization_id.is_none() => {
                    let value = required_argument(&mut arguments, "--organization-id")?;
                    organization_id = Some(
                        parse_canonical_nonnegative_signed_i64(&value)
                            .map_err(|_| BackfillCommandError::InvalidOrganizationId)?,
                    );
                }
                "--actor-id" if actor_id.is_none() => {
                    let value = required_argument(&mut arguments, "--actor-id")?;
                    actor_id = Some(
                        parse_canonical_positive_signed_i64(&value)
                            .map_err(|_| BackfillCommandError::InvalidActorId)?,
                    );
                }
                "--page-size" if !page_size_seen => {
                    let value = required_argument(&mut arguments, "--page-size")?;
                    page_size = value
                        .parse::<u32>()
                        .ok()
                        .filter(|value| (1..=MAX_WIKI_BACKFILL_PAGE_SIZE).contains(value))
                        .ok_or(BackfillCommandError::InvalidPageSize)?;
                    page_size_seen = true;
                }
                "--after-space-id" if after_space_id.is_none() => {
                    let value = required_argument(&mut arguments, "--after-space-id")?;
                    after_space_id = Some(
                        parse_canonical_positive_signed_i64(&value)
                            .map_err(|_| BackfillCommandError::InvalidAfterSpaceId)?,
                    );
                }
                "--dry-run" if !dry_run => dry_run = true,
                _ => return Err(BackfillCommandError::UnexpectedArgument(argument)),
            }
        }

        Ok(Self {
            organization_id: organization_id.ok_or(BackfillCommandError::OrganizationIdMissing)?,
            actor_id: actor_id.ok_or(BackfillCommandError::ActorIdMissing)?,
            page_size,
            after_space_id,
            dry_run,
        })
    }
}

fn required_argument(
    arguments: &mut impl Iterator<Item = String>,
    name: &'static str,
) -> Result<String, BackfillCommandError> {
    arguments
        .next()
        .ok_or(BackfillCommandError::MissingArgumentValue(name))
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct WikiPublicationBackfillReport {
    kind: &'static str,
    schema_version: &'static str,
    generated_at: String,
    #[serde(with = "sdkwork_utils_rust::serde_uint64")]
    tenant_id: u64,
    #[serde(with = "sdkwork_utils_rust::serde_uint64")]
    organization_id: u64,
    #[serde(with = "sdkwork_utils_rust::serde_uint64")]
    actor_id: u64,
    dry_run: bool,
    outcomes: Vec<ReportOutcome>,
    page_info: ReportPageInfo,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct ReportOutcome {
    #[serde(with = "sdkwork_utils_rust::serde_uint64")]
    space_id: u64,
    disposition: &'static str,
    failure_code: Option<&'static str>,
}

impl From<&WikiPublicationBackfillOutcome> for ReportOutcome {
    fn from(outcome: &WikiPublicationBackfillOutcome) -> Self {
        Self {
            space_id: outcome.space_id,
            disposition: match outcome.disposition {
                WikiPublicationBackfillDisposition::Planned => "planned",
                WikiPublicationBackfillDisposition::Initialized => "initialized",
                WikiPublicationBackfillDisposition::Failed => "failed",
            },
            failure_code: outcome.failure_code,
        }
    }
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct ReportPageInfo {
    mode: &'static str,
    page_size: u32,
    #[serde(with = "sdkwork_utils_rust::serde_uint64::option")]
    next_after_space_id: Option<u64>,
    stopped_on_failure: bool,
}

#[derive(Debug, Error, PartialEq, Eq)]
enum BackfillCommandError {
    #[error("help requested")]
    HelpRequested,
    #[error("--organization-id is required\n{}", usage())]
    OrganizationIdMissing,
    #[error("--organization-id must be a canonical nonnegative signed BIGINT")]
    InvalidOrganizationId,
    #[error("--actor-id is required\n{}", usage())]
    ActorIdMissing,
    #[error("--actor-id must be a canonical positive signed BIGINT")]
    InvalidActorId,
    #[error("--page-size must be between 1 and {MAX_WIKI_BACKFILL_PAGE_SIZE}")]
    InvalidPageSize,
    #[error("--after-space-id must be a canonical positive signed BIGINT")]
    InvalidAfterSpaceId,
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
    #[error("Knowledgebase runtime initialization failed")]
    RuntimeInitializationFailed,
    #[error("Wiki publication backfill execution failed")]
    ExecutionFailed,
    #[error("report serialization failed")]
    ReportSerializationFailed,
}

fn required_tenant_id() -> Result<u64, BackfillCommandError> {
    let value = std::env::var("SDKWORK_KNOWLEDGEBASE_TENANT_ID")
        .map_err(|_| BackfillCommandError::TenantIdMissing)?;
    parse_canonical_positive_signed_i64(&value).map_err(|_| BackfillCommandError::InvalidTenantId)
}

const fn usage() -> &'static str {
    "usage: sdkwork-knowledgebase-wiki-backfill --organization-id <id> --actor-id <id> [--page-size <1..200>] [--after-space-id <id>] [--dry-run]"
}

#[cfg(test)]
mod tests {
    use super::{BackfillArguments, BackfillCommandError};

    fn strings(values: &[&str]) -> Vec<String> {
        values.iter().map(|value| (*value).to_string()).collect()
    }

    #[test]
    fn arguments_require_explicit_scope_and_actor() {
        assert_eq!(
            BackfillArguments::parse(Vec::<String>::new()),
            Err(BackfillCommandError::OrganizationIdMissing)
        );
        assert_eq!(
            BackfillArguments::parse(strings(&["--organization-id", "7"])),
            Err(BackfillCommandError::ActorIdMissing)
        );
    }

    #[test]
    fn arguments_accept_bounded_resume_and_dry_run() {
        assert_eq!(
            BackfillArguments::parse(strings(&[
                "--organization-id",
                "0",
                "--actor-id",
                "42",
                "--page-size",
                "50",
                "--after-space-id",
                "99",
                "--dry-run",
            ])),
            Ok(BackfillArguments {
                organization_id: 0,
                actor_id: 42,
                page_size: 50,
                after_space_id: Some(99),
                dry_run: true,
            })
        );
    }

    #[test]
    fn arguments_reject_ambiguous_or_unbounded_values() {
        for arguments in [
            strings(&["--organization-id", "01", "--actor-id", "42"]),
            strings(&["--organization-id", "0", "--actor-id", "0"]),
            strings(&[
                "--organization-id",
                "0",
                "--actor-id",
                "42",
                "--page-size",
                "201",
            ]),
            strings(&[
                "--organization-id",
                "0",
                "--actor-id",
                "42",
                "--after-space-id",
                "0",
            ]),
            strings(&[
                "--organization-id",
                "0",
                "--actor-id",
                "42",
                "--dry-run",
                "--dry-run",
            ]),
        ] {
            assert!(BackfillArguments::parse(arguments).is_err());
        }
    }
}
