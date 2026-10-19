//! Resolves the PostgreSQL schema that owns the `pgvector` extension.
//!
//! The Knowledgebase baseline installs `vector` in whatever schema is current
//! during bootstrap (normally `public`), while application connections usually
//! pin `search_path` to the application schema. Type casts and operators must
//! therefore be schema-qualified. The default stays `public` for existing
//! deployments; a deployment that installs the extension elsewhere sets
//! [`PGVECTOR_SCHEMA_ENV`].
//!
//! The value is validated as an unquoted SQL identifier before interpolation,
//! and configuration errors fail closed instead of silently resolving to a
//! schema that may not own the extension.

/// Environment variable selecting the `pgvector` extension schema.
pub const PGVECTOR_SCHEMA_ENV: &str = "SDKWORK_KNOWLEDGEBASE_PGVECTOR_SCHEMA";
/// Schema used when the environment variable is absent.
pub const DEFAULT_PGVECTOR_SCHEMA: &str = "public";
/// Maximum PostgreSQL identifier length in bytes.
const MAX_IDENTIFIER_BYTES: usize = 63;

/// Resolves the configured extension schema from an optional raw value.
///
/// `None` and blank values select [`DEFAULT_PGVECTOR_SCHEMA`]. A value that is
/// not a simple unquoted identifier (63 bytes maximum, ASCII letters, digits,
/// and underscores, not starting with a digit) is rejected.
pub fn resolve_pgvector_schema(value: Option<&str>) -> Result<String, String> {
    let Some(value) = value.map(str::trim).filter(|value| !value.is_empty()) else {
        return Ok(DEFAULT_PGVECTOR_SCHEMA.to_string());
    };
    if value.len() > MAX_IDENTIFIER_BYTES {
        return Err(format!(
            "{PGVECTOR_SCHEMA_ENV} must be at most {MAX_IDENTIFIER_BYTES} bytes"
        ));
    }
    let mut characters = value.chars();
    let Some(first) = characters.next() else {
        return Ok(DEFAULT_PGVECTOR_SCHEMA.to_string());
    };
    if !(first.is_ascii_alphabetic() || first == '_')
        || !characters.all(|character| character.is_ascii_alphanumeric() || character == '_')
    {
        return Err(format!(
            "{PGVECTOR_SCHEMA_ENV} must be an unquoted SQL identifier"
        ));
    }
    Ok(value.to_string())
}

/// Reads [`PGVECTOR_SCHEMA_ENV`] and resolves the extension schema.
pub fn pgvector_schema() -> Result<String, String> {
    let configured = std::env::var(PGVECTOR_SCHEMA_ENV).ok();
    resolve_pgvector_schema(configured.as_deref())
}

#[cfg(test)]
mod tests {
    use super::{resolve_pgvector_schema, DEFAULT_PGVECTOR_SCHEMA};

    #[test]
    fn absent_or_blank_values_use_the_default_schema() {
        assert_eq!(
            resolve_pgvector_schema(None).expect("default"),
            DEFAULT_PGVECTOR_SCHEMA
        );
        assert_eq!(
            resolve_pgvector_schema(Some("   ")).expect("blank default"),
            DEFAULT_PGVECTOR_SCHEMA
        );
    }

    #[test]
    fn plain_identifiers_are_accepted_and_trimmed() {
        assert_eq!(
            resolve_pgvector_schema(Some(" extensions ")).expect("identifier"),
            "extensions"
        );
        assert_eq!(
            resolve_pgvector_schema(Some("_pgvector_1")).expect("identifier"),
            "_pgvector_1"
        );
    }

    #[test]
    fn unsafe_identifiers_fail_closed() {
        let overlong = "v".repeat(64);
        for value in [
            "public; DROP SCHEMA public",
            "public.vector",
            "1vector",
            "vector-schema",
            "vector\"",
            overlong.as_str(),
        ] {
            assert!(
                resolve_pgvector_schema(Some(value)).is_err(),
                "expected rejection for {value:?}"
            );
        }
    }
}
