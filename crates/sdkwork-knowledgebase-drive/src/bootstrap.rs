use sdkwork_database_config::workspace_database::normalize_workspace_postgres_url;
use sdkwork_drive_config::DatabaseConfig as DriveDatabaseConfig;
use sdkwork_drive_workspace_service::infrastructure::sql::connect_postgres_database_and_install_schema;
use sdkwork_knowledgebase_database_host::postgres_url_with_statement_timeout;
use sqlx::PgPool;

const DEFAULT_DRIVE_PROVIDER_ID: &str = "sdkwork-knowledgebase-local";
const DEFAULT_DRIVE_BUCKET: &str = "knowledgebase";
const DEPLOYMENT_PROFILE_ENV: &str = "SDKWORK_KNOWLEDGEBASE_DEPLOYMENT_PROFILE";

const KNOWLEDGEBASE_DRIVE_POOL_MAX_CONNECTIONS: u32 = 5;

pub async fn connect_knowledgebase_drive_pool(database_url: &str) -> Result<PgPool, sqlx::Error> {
    connect_knowledgebase_drive_pool_with_max_connections(
        database_url,
        KNOWLEDGEBASE_DRIVE_POOL_MAX_CONNECTIONS,
    )
    .await
}

pub async fn connect_knowledgebase_drive_pool_with_max_connections(
    database_url: &str,
    max_connections: u32,
) -> Result<PgPool, sqlx::Error> {
    if max_connections == 0 {
        return Err(sqlx::Error::Configuration(
            "Knowledgebase Drive pool max_connections must be greater than zero".into(),
        ));
    }
    let normalized = normalize_workspace_postgres_url(database_url.trim())
        .map_err(|error| sqlx::Error::Configuration(error.to_string().into()))?;
    // The drive schema ships no RLS policies, so the pool needs the deployment
    // statement guard only; the tenant/organization session scope stays
    // knowledgebase-pool-specific.
    let guarded = postgres_url_with_statement_timeout(normalized.as_str())
        .map_err(|error| sqlx::Error::Configuration(error.to_string().into()))?;
    let drive_config =
        DriveDatabaseConfig::from_url_with_max_connections(guarded.as_str(), max_connections)
            .map_err(|error| sqlx::Error::Configuration(error.to_string().into()))?;
    let pool = connect_postgres_database_and_install_schema(&drive_config).await?;
    if should_seed_standalone_local_provider()? {
        seed_default_drive_storage_provider(&pool).await?;
    }
    Ok(pool)
}

pub async fn knowledgebase_drive_health_check(pool: &PgPool) -> Result<(), sqlx::Error> {
    sqlx::query_scalar::<_, i64>("SELECT 1::bigint")
        .fetch_one(pool)
        .await
        .map(|_| ())
}

fn should_seed_standalone_local_provider() -> Result<bool, sqlx::Error> {
    match std::env::var(DEPLOYMENT_PROFILE_ENV) {
        Ok(value) if value.trim().eq_ignore_ascii_case("standalone") => Ok(true),
        Ok(value) if value.trim().eq_ignore_ascii_case("cloud") => Ok(false),
        Ok(_) => Err(sqlx::Error::Configuration(
            format!("{DEPLOYMENT_PROFILE_ENV} must be standalone or cloud").into(),
        )),
        Err(std::env::VarError::NotPresent) => Ok(true),
        Err(error) => Err(sqlx::Error::Configuration(
            format!("{DEPLOYMENT_PROFILE_ENV} could not be read: {error}").into(),
        )),
    }
}

async fn seed_default_drive_storage_provider(pool: &PgPool) -> Result<(), sqlx::Error> {
    let exists: Option<i64> =
        sqlx::query_scalar("SELECT 1::bigint FROM dr_drive_storage_provider WHERE id = $1")
            .bind(DEFAULT_DRIVE_PROVIDER_ID)
            .fetch_optional(pool)
            .await?;
    if exists.is_some() {
        return Ok(());
    }

    let sql = "INSERT INTO dr_drive_storage_provider (
            id, provider_kind, name, endpoint_url, region, bucket, path_style,
            strict_tls, credential_ref, server_side_encryption_mode, default_storage_class,
            status, version, created_by, updated_by
        ) VALUES (
            $1, 'local_filesystem', $2, 'file://localhost', 'local', $2, TRUE, TRUE,
            'plain:local:local', NULL, NULL, 'active', 1, 'system', 'system'
        )";

    sqlx::query(sql)
        .bind(DEFAULT_DRIVE_PROVIDER_ID)
        .bind(DEFAULT_DRIVE_BUCKET)
        .execute(pool)
        .await?;
    Ok(())
}
