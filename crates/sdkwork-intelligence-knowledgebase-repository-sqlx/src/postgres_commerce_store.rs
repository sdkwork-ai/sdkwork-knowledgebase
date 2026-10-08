use async_trait::async_trait;
use sdkwork_database_config::DatabaseEngine;
use sdkwork_intelligence_knowledgebase_service::ports::commerce_store::{
    map_catalog_item, KnowledgeMarketStore, KnowledgeMarketStoreError,
};
use sdkwork_knowledgebase_contract::market::KnowledgeMarketCatalogItem;
use sdkwork_utils_rust::is_blank;
use sqlx::{AnyPool, Row};
use std::sync::Arc;
use time::{format_description::well_known::Rfc3339, OffsetDateTime};

use crate::db::sql_timestamp::SqlTimestampDialect;
use crate::id::{default_knowledge_id_generator, next_i64_id, KnowledgeIdGenerator};

const ACTIVE_STATUS: i64 = 1;
const DELETED_STATUS: i64 = 0;

#[derive(Debug, Clone)]
pub struct PostgresCommerceStore {
    pool: AnyPool,
    organization_id: u64,
    id_generator: Arc<dyn KnowledgeIdGenerator>,
    timestamp_dialect: SqlTimestampDialect,
}

impl PostgresCommerceStore {
    pub fn new(pool: AnyPool, organization_id: u64) -> Self {
        Self {
            pool,
            organization_id,
            id_generator: default_knowledge_id_generator(),
            timestamp_dialect: SqlTimestampDialect::default(),
        }
    }

    pub fn with_database_engine(mut self, database_engine: DatabaseEngine) -> Self {
        self.timestamp_dialect = SqlTimestampDialect::from_database_engine(database_engine);
        self
    }
}

fn now_rfc3339() -> Result<String, KnowledgeMarketStoreError> {
    OffsetDateTime::now_utc()
        .format(&Rfc3339)
        .map_err(|error| KnowledgeMarketStoreError::Internal(error.to_string()))
}

fn map_catalog_row(
    row: &sqlx::any::AnyRow,
) -> Result<KnowledgeMarketCatalogItem, KnowledgeMarketStoreError> {
    Ok(map_catalog_item(
        market_from_i64("listing_id", row.get("id"))?,
        row.get("title"),
        required_catalog_text(row, "icon")?,
        row.try_get("description").unwrap_or_default(),
        required_catalog_text(row, "author")?,
        serde_json::from_str::<Vec<String>>(row.get("tags_json")).map_err(|error| {
            KnowledgeMarketStoreError::Internal(format!(
                "stored market listing tags_json is invalid: {error}"
            ))
        })?,
        required_catalog_text(row, "provider")?,
        required_catalog_text(row, "model_name")?,
        market_from_i64("subscribers_count", row.get("subscribers_count"))?
            .try_into()
            .map_err(|_| {
                KnowledgeMarketStoreError::Internal(
                    "subscribers_count exceeds the API u32 range".to_string(),
                )
            })?,
        market_from_i64("documents_count", row.get("documents_count"))?
            .try_into()
            .map_err(|_| {
                KnowledgeMarketStoreError::Internal(
                    "documents_count exceeds the API u32 range".to_string(),
                )
            })?,
        row.get::<i64, _>("is_subscribed") == 1,
    ))
}

async fn fetch_catalog_rows(
    pool: &AnyPool,
    tenant_id: i64,
    organization_id: i64,
    subscriber_actor_id: Option<i64>,
    cursor: Option<i64>,
    fetch_limit: i64,
) -> Result<Vec<sqlx::any::AnyRow>, KnowledgeMarketStoreError> {
    sqlx::query(
        r#"
        SELECT
            l.id, l.title, l.icon, l.description, l.author,
            CAST(l.tags_json AS TEXT) AS tags_json,
            l.provider, l.model_name, l.subscribers_count, l.documents_count,
            CASE
                WHEN $3 IS NULL THEN 0
                WHEN EXISTS (
                    SELECT 1 FROM kb_market_subscription s
                    WHERE s.tenant_id = l.tenant_id
                      AND s.organization_id = l.organization_id
                      AND s.listing_id = l.id
                      AND s.subscriber_actor_id = $3
                      AND s.status = 1
                ) THEN 1
                ELSE 0
            END AS is_subscribed
        FROM kb_market_listing l
        WHERE l.tenant_id = $1
          AND l.organization_id = $2
          AND l.status = 1
          AND ($4 IS NULL OR l.id < $4)
        ORDER BY l.id DESC
        LIMIT $5
        "#,
    )
    .bind(tenant_id)
    .bind(organization_id)
    .bind(subscriber_actor_id)
    .bind(cursor)
    .bind(fetch_limit)
    .fetch_all(pool)
    .await
    .map_err(|error| KnowledgeMarketStoreError::Internal(error.to_string()))
}

#[async_trait]
impl KnowledgeMarketStore for PostgresCommerceStore {
    async fn list_catalog_page(
        &self,
        tenant_id: u64,
        subscriber_actor_id: Option<u64>,
        cursor: Option<u64>,
        page_size: u32,
    ) -> Result<(Vec<KnowledgeMarketCatalogItem>, Option<String>, bool), KnowledgeMarketStoreError>
    {
        let page_size = page_size.clamp(1, 200);
        let fetch_limit = i64::from(page_size.saturating_add(1));
        let tenant_id = market_to_i64("tenant_id", tenant_id)?;
        let organization_id = market_to_i64("organization_id", self.organization_id)?;
        let subscriber_actor_id = subscriber_actor_id
            .map(|value| market_to_i64("subscriber_actor_id", value))
            .transpose()?;
        let cursor = cursor
            .map(|value| market_to_i64("cursor", value))
            .transpose()?;
        let rows = fetch_catalog_rows(
            &self.pool,
            tenant_id,
            organization_id,
            subscriber_actor_id,
            cursor,
            fetch_limit,
        )
        .await?;

        let has_more = rows.len() > page_size as usize;
        let rows = rows
            .into_iter()
            .take(page_size as usize)
            .collect::<Vec<_>>();
        let next_cursor = if has_more {
            rows.last().map(|row| row.get::<i64, _>("id").to_string())
        } else {
            None
        };
        let items = rows
            .iter()
            .map(map_catalog_row)
            .collect::<Result<Vec<_>, _>>()?;
        Ok((items, next_cursor, has_more))
    }

    async fn subscribe(
        &self,
        tenant_id: u64,
        subscriber_actor_id: u64,
        listing_id: u64,
    ) -> Result<(), KnowledgeMarketStoreError> {
        let tenant_id = market_to_i64("tenant_id", tenant_id)?;
        let organization_id = market_to_i64("organization_id", self.organization_id)?;
        let subscriber_actor_id = market_to_i64("subscriber_actor_id", subscriber_actor_id)?;
        let listing_id = market_to_i64("listing_id", listing_id)?;
        let mut transaction = self
            .pool
            .begin()
            .await
            .map_err(|error| KnowledgeMarketStoreError::Internal(error.to_string()))?;
        let listing_exists = sqlx::query_scalar::<_, i64>(
            "SELECT id FROM kb_market_listing WHERE tenant_id = $1 AND organization_id = $2 AND id = $3 AND status = 1",
        )
        .bind(tenant_id)
        .bind(organization_id)
        .bind(listing_id)
        .fetch_optional(&mut *transaction)
        .await
        .map_err(|error| KnowledgeMarketStoreError::Internal(error.to_string()))?;
        if listing_exists.is_none() {
            return Err(KnowledgeMarketStoreError::NotFound);
        }

        let now = now_rfc3339()?;
        let id = next_i64_id(&self.id_generator)
            .map_err(|error| KnowledgeMarketStoreError::Internal(error.to_string()))?;
        let created_at_expr = self.timestamp_dialect.sql_timestamp_expr("$6");
        let query = format!(
            r#"
            INSERT INTO kb_market_subscription (
                id, tenant_id, organization_id, subscriber_actor_id, listing_id, created_at, status
            ) VALUES ($1, $2, $3, $4, $5, {created_at_expr}, $7)
            "#,
        );
        sqlx::query(sqlx::AssertSqlSafe(query.as_str()))
            .bind(id)
            .bind(tenant_id)
            .bind(organization_id)
            .bind(subscriber_actor_id)
            .bind(listing_id)
            .bind(&now)
            .bind(ACTIVE_STATUS)
            .execute(&mut *transaction)
            .await
            .map_err(|error| {
                let message = error.to_string();
                if message.contains("UNIQUE") || message.contains("unique") {
                    KnowledgeMarketStoreError::InvalidRequest(
                        "market listing is already subscribed".to_string(),
                    )
                } else {
                    KnowledgeMarketStoreError::Internal(message)
                }
            })?;

        let updated_at_expr = self.timestamp_dialect.sql_timestamp_expr("$4");
        let query = format!(
            "UPDATE kb_market_listing SET subscribers_count = subscribers_count + 1, updated_at = {updated_at_expr} WHERE tenant_id = $1 AND organization_id = $2 AND id = $3 AND status = 1",
        );
        let result = sqlx::query(sqlx::AssertSqlSafe(query.as_str()))
            .bind(tenant_id)
            .bind(organization_id)
            .bind(listing_id)
            .bind(&now)
            .execute(&mut *transaction)
            .await
            .map_err(|error| KnowledgeMarketStoreError::Internal(error.to_string()))?;
        if result.rows_affected() != 1 {
            return Err(KnowledgeMarketStoreError::NotFound);
        }

        transaction
            .commit()
            .await
            .map_err(|error| KnowledgeMarketStoreError::Internal(error.to_string()))?;

        Ok(())
    }

    async fn unsubscribe(
        &self,
        tenant_id: u64,
        subscriber_actor_id: u64,
        listing_id: u64,
    ) -> Result<(), KnowledgeMarketStoreError> {
        let tenant_id = market_to_i64("tenant_id", tenant_id)?;
        let organization_id = market_to_i64("organization_id", self.organization_id)?;
        let subscriber_actor_id = market_to_i64("subscriber_actor_id", subscriber_actor_id)?;
        let listing_id = market_to_i64("listing_id", listing_id)?;
        let now = now_rfc3339()?;
        let mut transaction = self
            .pool
            .begin()
            .await
            .map_err(|error| KnowledgeMarketStoreError::Internal(error.to_string()))?;
        let result = sqlx::query(
            r#"
            UPDATE kb_market_subscription
            SET status = $5
            WHERE tenant_id = $1 AND organization_id = $2 AND subscriber_actor_id = $3 AND listing_id = $4 AND status = 1
            "#,
        )
        .bind(tenant_id)
        .bind(organization_id)
        .bind(subscriber_actor_id)
        .bind(listing_id)
        .bind(DELETED_STATUS)
        .execute(&mut *transaction)
        .await
        .map_err(|error| KnowledgeMarketStoreError::Internal(error.to_string()))?;

        if result.rows_affected() == 0 {
            return Err(KnowledgeMarketStoreError::NotFound);
        }

        let updated_at_expr = self.timestamp_dialect.sql_timestamp_expr("$4");
        let query = format!(
            "UPDATE kb_market_listing SET subscribers_count = CASE WHEN subscribers_count > 0 THEN subscribers_count - 1 ELSE 0 END, updated_at = {updated_at_expr} WHERE tenant_id = $1 AND organization_id = $2 AND id = $3",
        );
        let result = sqlx::query(sqlx::AssertSqlSafe(query.as_str()))
            .bind(tenant_id)
            .bind(organization_id)
            .bind(listing_id)
            .bind(&now)
            .execute(&mut *transaction)
            .await
            .map_err(|error| KnowledgeMarketStoreError::Internal(error.to_string()))?;
        if result.rows_affected() != 1 {
            return Err(KnowledgeMarketStoreError::Internal(
                "active market subscription references a missing listing".to_string(),
            ));
        }

        transaction
            .commit()
            .await
            .map_err(|error| KnowledgeMarketStoreError::Internal(error.to_string()))?;

        Ok(())
    }
}

fn market_to_i64(field: &str, value: u64) -> Result<i64, KnowledgeMarketStoreError> {
    i64::try_from(value).map_err(|_| {
        KnowledgeMarketStoreError::InvalidRequest(format!(
            "{field} exceeds the supported signed 64-bit range"
        ))
    })
}

fn market_from_i64(field: &str, value: i64) -> Result<u64, KnowledgeMarketStoreError> {
    u64::try_from(value).map_err(|_| {
        KnowledgeMarketStoreError::Internal(format!(
            "stored {field} is outside the supported unsigned range"
        ))
    })
}

fn required_catalog_text(
    row: &sqlx::any::AnyRow,
    field: &str,
) -> Result<String, KnowledgeMarketStoreError> {
    let value = row
        .try_get::<Option<String>, _>(field)
        .map_err(|error| KnowledgeMarketStoreError::Internal(error.to_string()))?
        .filter(|value| !is_blank(Some(value.as_str())));
    value.ok_or_else(|| {
        KnowledgeMarketStoreError::Internal(format!(
            "stored market listing {field} is required by the API contract"
        ))
    })
}
