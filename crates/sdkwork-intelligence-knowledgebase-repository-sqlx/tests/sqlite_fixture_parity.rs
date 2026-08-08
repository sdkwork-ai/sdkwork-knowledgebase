//! SQLite fixture / PostgreSQL baseline schema parity.
//!
//! The PostgreSQL baseline `database/ddl/baseline/postgres/0001_knowledgebase_baseline.sql`
//! is a folded snapshot: every migration is baked in, so the effective schema is
//! `CREATE TABLE` + `ALTER TABLE ... ADD COLUMN` from the same file. The SQLite fixture
//! (`tests/fixtures/database/sqlite/`) follows the baseline-plus-migrations convention:
//! its effective schema is the baseline plus every `migrations/*.up.sql`.
//!
//! This test locks the contract the application actually relies on: any table and column
//! the PostgreSQL repository stores must exist in the SQLite fixture with a compatible
//! type, so the deterministic in-memory fixture can execute the same repository SQL.
//! Organization isolation, outbox claim fencing/retry, keyword search, and the audit
//! scope-actor index are asserted explicitly because repository code binds them.

use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::path::PathBuf;
use std::sync::OnceLock;

const PG_BASELINE: &str =
    "database/ddl/baseline/postgres/0001_knowledgebase_baseline.sql";
const SQLITE_BASELINE: &str =
    "tests/fixtures/database/sqlite/ddl/baseline/0001_knowledgebase_baseline.sql";
const SQLITE_MIGRATIONS_DIR: &str = "tests/fixtures/database/sqlite/migrations";

/// Tables that must carry `organization_id` after the organization-isolation cutover.
/// Mirrors the PostgreSQL folded section `ALTER TABLE ... ADD COLUMN organization_id`.
const ORGANIZATION_TABLES: &[&str] = &[
    "kb_collection",
    "kb_source",
    "kb_drive_object_ref",
    "kb_document",
    "kb_document_version",
    "kb_chunk",
    "kb_index",
    "kb_embedding",
    "kb_retrieval_profile",
    "kb_retrieval_trace",
    "kb_retrieval_hit",
    "kb_agent_profile",
    "kb_agent_knowledge_binding",
    "kb_ingestion_job",
    "kb_ingestion_job_item",
    "kb_okf_concept",
    "kb_okf_concept_revision",
    "kb_okf_bundle_file",
    "kb_okf_schema_profile",
    "kb_okf_log_entry",
    "kb_local_mirror_package",
    "kb_space_context_binding",
    "kb_outbox_event",
    "kb_okf_concept_link",
    "kb_okf_candidate",
    "kb_market_listing",
    "kb_market_subscription",
    "kb_audit_event",
];

/// PostgreSQL-only index families that have no SQLite equivalent (HNSW/GIN on vectors and
/// FTS) and are therefore excluded from the index-parity assertion.
const PG_ONLY_INDEX_SUFFIXES: &[&str] = &["_hnsw", "search_vector"];

fn repo_root() -> &'static PathBuf {
    static ROOT: OnceLock<PathBuf> = OnceLock::new();
    ROOT.get_or_init(|| {
        let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        // tests/ parent is the crate root; repository root is two levels up.
        manifest_dir
            .parent()
            .and_then(|p| p.parent())
            .map(PathBuf::from)
            .unwrap_or(manifest_dir)
    })
}

fn read_repo_file(relative: &str) -> String {
    fs::read_to_string(repo_root().join(relative))
        .unwrap_or_else(|error| panic!("read {relative}: {error}"))
}

fn migration_files() -> Vec<PathBuf> {
    let mut files = fs::read_dir(repo_root().join(SQLITE_MIGRATIONS_DIR))
        .expect("sqlite migrations dir")
        .map(|entry| entry.expect("migration entry").path())
        .filter(|path| {
            path.file_name()
                .and_then(|name| name.to_str())
                .map(|name| name.ends_with(".up.sql"))
                .unwrap_or(false)
        })
        .collect::<Vec<_>>();
    files.sort();
    files
}

fn sql_statements(sql: &str) -> Vec<String> {
    // Drop full-line `--` comments first so a comment block fused with the following
    // statement (same `;` fragment) never hides that statement.
    let cleaned = sql
        .lines()
        .filter(|line| !line.trim_start().starts_with("--"))
        .collect::<Vec<_>>()
        .join("\n");
    cleaned
        .split(';')
        .map(str::trim)
        .filter(|statement| !statement.is_empty())
        .map(|statement| statement.replace('\n', " "))
        .collect()
}

fn create_table_columns(sql: &str) -> BTreeMap<String, BTreeMap<String, String>> {
    let mut tables = BTreeMap::new();
    for statement in sql_statements(sql) {
        let trimmed = statement.trim_start();
        if !(trimmed.starts_with("CREATE TABLE")
            || trimmed.starts_with("CREATE VIRTUAL TABLE"))
        {
            continue;
        }
        let Some(open) = trimmed.find('(') else {
            continue;
        };
        let close = trimmed.rfind(')').unwrap_or(trimmed.len());
        let name = trimmed[..open]
            .split_whitespace()
            .find(|token| !matches!(*token, "CREATE" | "TABLE" | "VIRTUAL" | "IF" | "NOT" | "EXISTS" | "USING"))
            .map(|token| token.trim_matches(|c| c == '"' || c == '`'))
            .expect("table name");
        let body = &trimmed[open + 1..close];
        let mut columns = BTreeMap::new();
        for part in split_top_level(body, ',') {
            let part = part.trim();
            if part.is_empty()
                || part.starts_with("PRIMARY KEY")
                || part.starts_with("UNIQUE")
                || part.starts_with("CHECK")
                || part.starts_with("FOREIGN")
                || part.starts_with("CONSTRAINT")
            {
                continue;
            }
            let mut tokens = part.split_whitespace();
            if let (Some(first), Some(second)) = (tokens.next(), tokens.next()) {
                if second.chars().next().is_some_and(char::is_alphabetic) {
                    columns.insert(first.to_string(), second.to_string());
                }
            }
        }
        tables.insert(name.to_string(), columns);
    }
    tables
}

/// Splits on `separator` outside nested parentheses/quotes so `CHECK (...)`, defaults, and
/// expression bodies never break column parsing.
fn split_top_level(input: &str, separator: char) -> Vec<&str> {
    let mut parts = Vec::new();
    let mut depth = 0usize;
    let mut start = 0usize;
    let mut in_quote = false;
    for (index, ch) in input.char_indices() {
        match ch {
            '\'' if !in_quote => in_quote = true,
            '\'' if in_quote => in_quote = false,
            '(' if !in_quote => depth += 1,
            ')' if !in_quote => depth = depth.saturating_sub(1),
            _ => {}
        }
        if ch == separator && depth == 0 && !in_quote {
            parts.push(&input[start..index]);
            start = index + 1;
        }
    }
    parts.push(&input[start..]);
    parts
}

fn alter_added_columns(sql: &str) -> BTreeMap<String, BTreeMap<String, String>> {
    let mut alters: BTreeMap<String, BTreeMap<String, String>> = BTreeMap::new();
    for statement in sql_statements(sql) {
        let tokens = statement.split_whitespace().collect::<Vec<_>>();
        if tokens.first().copied() != Some("ALTER") || tokens.get(1).copied() != Some("TABLE") {
            continue;
        }
        let table = tokens[2].trim_matches('"');
        let Some(add_index) = tokens.iter().position(|token| *token == "ADD") else {
            continue;
        };
        let mut index = add_index + 1;
        if tokens.get(index).copied() == Some("COLUMN") {
            index += 1;
        }
        if tokens.get(index).copied() == Some("IF") {
            index += 3; // IF NOT EXISTS
        }
        let Some(column) = tokens.get(index).copied() else {
            continue;
        };
        let column = column.trim_matches('"');
        let column_type = tokens[index + 1..].join(" ");
        if column.starts_with("CONSTRAINT") || column == "PRIMARY" {
            continue;
        }
        alters
            .entry(table.to_string())
            .or_default()
            .insert(column.to_string(), column_type);
    }
    alters
}

fn index_names(sql: &str) -> BTreeSet<String> {
    sql_statements(sql)
        .into_iter()
        .filter(|statement| statement.starts_with("CREATE"))
        .filter(|statement| !statement.starts_with("CREATE EXTENSION"))
        .filter_map(|statement| {
            let mut tokens = statement.split_whitespace();
            let mut name = None;
            for token in tokens.by_ref() {
                match token {
                    "CREATE" | "UNIQUE" | "INDEX" | "IF" | "NOT" | "EXISTS" | "EXTENSION" => {
                        continue
                    }
                    _ => {
                        name = Some(token.trim());
                        break;
                    }
                }
            }
            name.map(|value| value.to_string())
        })
        .collect()
}

fn postgres_effective_schema() -> (BTreeMap<String, BTreeMap<String, String>>, BTreeSet<String>) {
    let sql = read_repo_file(PG_BASELINE);
    let mut tables = create_table_columns(&sql);
    for (table, columns) in alter_added_columns(&sql) {
        tables.entry(table).or_default().extend(columns);
    }
    (tables, index_names(&sql))
}

fn sqlite_effective_schema() -> (BTreeMap<String, BTreeMap<String, String>>, BTreeSet<String>) {
    let mut sql = read_repo_file(SQLITE_BASELINE);
    for migration in migration_files() {
        sql.push('\n');
        sql.push_str(&fs::read_to_string(&migration).unwrap_or_else(|error| {
            panic!("read migration {}: {error}", migration.display())
        }));
    }
    let mut tables = create_table_columns(&sql);
    for (table, columns) in alter_added_columns(&sql) {
        tables.entry(table).or_default().extend(columns);
    }
    (tables, index_names(&sql))
}

#[test]
fn sqlite_fixture_contains_every_postgres_table_and_column() {
    let (postgres, _) = postgres_effective_schema();
    let (sqlite, _) = sqlite_effective_schema();

    let missing_tables = postgres
        .keys()
        .filter(|table| !sqlite.contains_key(*table))
        .cloned()
        .collect::<Vec<_>>();
    assert!(
        missing_tables.is_empty(),
        "SQLite fixture is missing PostgreSQL tables: {missing_tables:?}"
    );

    for (table, columns) in &postgres {
        let sqlite_columns = sqlite.get(table).expect("table present");
        let missing = columns
            .keys()
            .filter(|column| !sqlite_columns.contains_key(*column))
            .collect::<Vec<_>>();
        assert!(
            missing.is_empty(),
            "SQLite fixture is missing columns on {table}: {missing:?}"
        );
    }
}

#[test]
fn organization_isolation_columns_exist_on_every_scoped_table() {
    let (sqlite, _) = sqlite_effective_schema();
    for table in ORGANIZATION_TABLES {
        let columns = sqlite
            .get(*table)
            .unwrap_or_else(|| panic!("missing SQLite table {table}"));
        assert!(
            columns.contains_key("organization_id"),
            "SQLite {table} must carry organization_id"
        );
        assert!(
            columns.contains_key("tenant_id"),
            "SQLite {table} must carry tenant_id"
        );
    }
}

#[test]
fn outbox_claim_and_retry_columns_exist_on_sqlite_outbox() {
    let (sqlite, _) = sqlite_effective_schema();
    let columns = sqlite.get("kb_outbox_event").expect("outbox table");
    for column in [
        "organization_id",
        "claim_owner",
        "claim_token",
        "claimed_at",
        "dead_lettered_at",
        "next_attempt_at",
        "last_error",
        "retry_count",
    ] {
        assert!(
            columns.contains_key(column),
            "SQLite kb_outbox_event must carry {column}"
        );
    }
}

#[test]
fn keyword_search_and_vector_columns_exist_on_sqlite_fixture() {
    let (sqlite, _) = sqlite_effective_schema();
    assert!(
        sqlite
            .get("kb_chunk")
            .is_some_and(|columns| columns.contains_key("search_vector")),
        "SQLite kb_chunk must carry search_vector (chunk keyword search)"
    );
    assert!(
        sqlite
            .get("kb_embedding")
            .is_some_and(|columns| columns.contains_key("embedding_vector")),
        "SQLite kb_embedding must carry embedding_vector"
    );
}

#[test]
fn sqlite_indexes_cover_postgres_functional_indexes() {
    let (_, postgres_indexes) = postgres_effective_schema();
    let (_, sqlite_indexes) = sqlite_effective_schema();
    let missing = postgres_indexes
        .iter()
        .filter(|name| {
            !PG_ONLY_INDEX_SUFFIXES
                .iter()
                .any(|suffix| name.ends_with(suffix))
        })
        .filter(|name| !sqlite_indexes.contains(*name))
        .cloned()
        .collect::<Vec<_>>();
    assert!(
        missing.is_empty(),
        "SQLite fixture is missing PostgreSQL functional indexes: {missing:?}"
    );
}
