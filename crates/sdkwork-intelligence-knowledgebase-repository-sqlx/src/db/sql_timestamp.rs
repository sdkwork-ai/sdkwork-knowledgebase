//! PostgreSQL SQL timestamp bindings.
//!
//! 服务端权威持久化仅支持 PostgreSQL（DATABASE_SPEC：authoritative-server）；
//! SQLite 仅用于 client-local 桌面端，不经过本 crate。

use sdkwork_database_config::DatabaseEngine;
use std::fmt::Display;
use time::{format_description::well_known::Rfc3339, OffsetDateTime};

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum SqlTimestampDialect {
    #[default]
    Postgres,
}

impl SqlTimestampDialect {
    pub fn from_database_engine(_engine: DatabaseEngine) -> Self {
        // 服务端持久化仅支持 PostgreSQL；非 Postgres engine 不参与服务端执行路径。
        Self::Postgres
    }

    pub fn sql_timestamp_expr(self, placeholder: &str) -> String {
        format!("CAST({placeholder} AS TIMESTAMP)")
    }

    pub fn sql_json_expr(self, placeholder: &str) -> String {
        format!("CAST({placeholder} AS JSONB)")
    }

    pub fn sql_timestamp_text_expr(self, column: &str) -> String {
        format!("TO_CHAR({column}, 'YYYY-MM-DD\"T\"HH24:MI:SS.US\"Z\"')")
    }
}

pub fn utc_sql_timestamp_text() -> Result<String, String> {
    OffsetDateTime::now_utc()
        .format(&Rfc3339)
        .map_err(|error| error.to_string())
}

pub fn push_sql_timestamp_bind<DB, Sep>(
    row: &mut sqlx::query_builder::Separated<'_, DB, Sep>,
    _dialect: SqlTimestampDialect,
    value: &str,
) where
    DB: sqlx::Database,
    for<'q> String: sqlx::Encode<'q, DB> + sqlx::Type<DB>,
    Sep: Display,
{
    row.push("CAST(");
    row.push_bind_unseparated(value.to_owned());
    row.push_unseparated(" AS TIMESTAMP)");
}

#[cfg(test)]
mod tests {
    use super::SqlTimestampDialect;

    #[test]
    fn postgres_dialect_casts_text_bindings_to_database_types() {
        assert_eq!(
            SqlTimestampDialect::Postgres.sql_timestamp_expr("$1"),
            "CAST($1 AS TIMESTAMP)"
        );
        assert_eq!(
            SqlTimestampDialect::Postgres.sql_json_expr("$2"),
            "CAST($2 AS JSONB)"
        );
        assert_eq!(
            SqlTimestampDialect::Postgres.sql_timestamp_text_expr("created_at"),
            "TO_CHAR(created_at, 'YYYY-MM-DD\"T\"HH24:MI:SS.US\"Z\"')"
        );
    }
}
