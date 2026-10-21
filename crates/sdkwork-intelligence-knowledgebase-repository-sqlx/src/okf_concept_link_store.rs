use async_trait::async_trait;
use sdkwork_database_config::DatabaseEngine;
use sdkwork_intelligence_knowledgebase_service::ports::knowledge_okf_concept_link_store::{
    InboundLinkTargetsPage, KnowledgeOkfConceptLinkEdge, KnowledgeOkfConceptLinkStore,
    KnowledgeOkfConceptLinkStoreError, LinkEdgeCursor, LinkEdgePage,
    ReplaceKnowledgeOkfConceptLinksRecord,
};
use sqlx::AnyPool;
use sqlx::Row;
use std::sync::Arc;
use time::{format_description::well_known::Rfc3339, OffsetDateTime};
use uuid::Uuid;

use crate::db::sql_timestamp::SqlTimestampDialect;
use crate::id::{default_knowledge_id_generator, next_i64_id, KnowledgeIdGenerator};

const ACTIVE_STATUS: i64 = 1;
const INITIAL_VERSION: i64 = 0;
const MAX_LINK_SCAN_PAGE_SIZE: u32 = 2_000;
/// Upper bound accepted by the bounded incident-edge expansion; mirrors the
/// service port bound.
const MAX_OKF_INCIDENT_LINK_EDGES: u32 = 2_000;
/// Concept ids folded into one incident-edge statement.
const MAX_OKF_INCIDENT_CONCEPT_IDS: usize = 256;
/// Delimiter used to carry the concept-id list as one text parameter. Concept
/// ids are validated paths over `[A-Za-z0-9_-]`, `.`, and `/`, so `|` cannot
/// occur inside one.
const OKF_INCIDENT_ID_DELIMITER: &str = "|";

#[derive(Debug, Clone)]
pub struct PostgresKnowledgeOkfConceptLinkStore {
    pool: AnyPool,
    tenant_id: u64,
    organization_id: u64,
    id_generator: Arc<dyn KnowledgeIdGenerator>,
    timestamp_dialect: SqlTimestampDialect,
}

impl PostgresKnowledgeOkfConceptLinkStore {
    pub fn new(pool: AnyPool, tenant_id: u64, organization_id: u64) -> Self {
        Self::with_id_generator(
            pool,
            tenant_id,
            organization_id,
            default_knowledge_id_generator(),
        )
    }

    pub fn with_id_generator(
        pool: AnyPool,
        tenant_id: u64,
        organization_id: u64,
        id_generator: Arc<dyn KnowledgeIdGenerator>,
    ) -> Self {
        Self {
            pool,
            tenant_id,
            organization_id,
            id_generator,
            timestamp_dialect: SqlTimestampDialect::default(),
        }
    }

    pub fn with_database_engine(mut self, database_engine: DatabaseEngine) -> Self {
        self.timestamp_dialect = SqlTimestampDialect::from_database_engine(database_engine);
        self
    }
}

#[async_trait]
impl KnowledgeOkfConceptLinkStore for PostgresKnowledgeOkfConceptLinkStore {
    async fn replace_outbound_links(
        &self,
        record: ReplaceKnowledgeOkfConceptLinksRecord,
    ) -> Result<(), KnowledgeOkfConceptLinkStoreError> {
        let tenant_id = to_i64("tenant_id", self.tenant_id)?;
        let organization_id = to_i64("organization_id", self.organization_id)?;
        let space_id = to_i64("space_id", record.space_id)?;
        let now = now_rfc3339()?;

        let mut transaction = self
            .pool
            .begin()
            .await
            .map_err(|error| KnowledgeOkfConceptLinkStoreError::Internal(error.to_string()))?;

        let updated_at_expr = self.timestamp_dialect.sql_timestamp_expr("$1");
        let update_query = format!(
            r#"
            UPDATE kb_okf_concept_link
            SET status = 0, updated_at = {updated_at_expr}, version = version + 1
            WHERE tenant_id = $2 AND organization_id = $3 AND space_id = $4 AND from_concept_id = $5 AND status = $6
            "#,
        );
        sqlx::query(sqlx::AssertSqlSafe(update_query.as_str()))
            .bind(&now)
            .bind(tenant_id)
            .bind(organization_id)
            .bind(space_id)
            .bind(&record.from_concept_id)
            .bind(ACTIVE_STATUS)
            .execute(&mut *transaction)
            .await
            .map_err(|error| KnowledgeOkfConceptLinkStoreError::Internal(error.to_string()))?;

        for link in record.links {
            let id = next_i64_id(&self.id_generator).map_err(id_error)?;
            let created_at_expr = self.timestamp_dialect.sql_timestamp_expr("$10");
            let updated_at_expr = self.timestamp_dialect.sql_timestamp_expr("$11");
            let insert_query = format!(
                r#"
                INSERT INTO kb_okf_concept_link (
                    id, uuid, tenant_id, organization_id, space_id, from_concept_id, to_concept_id,
                    anchor_text, status, created_at, updated_at, version
                ) VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, {created_at_expr}, {updated_at_expr}, $12)
                "#,
            );
            sqlx::query(sqlx::AssertSqlSafe(insert_query.as_str()))
                .bind(id)
                .bind(Uuid::new_v4().to_string())
                .bind(tenant_id)
                .bind(organization_id)
                .bind(space_id)
                .bind(&record.from_concept_id)
                .bind(&link.to_concept_id)
                .bind(&link.anchor_text)
                .bind(ACTIVE_STATUS)
                .bind(&now)
                .bind(&now)
                .bind(INITIAL_VERSION)
                .execute(&mut *transaction)
                .await
                .map_err(|error| KnowledgeOkfConceptLinkStoreError::Internal(error.to_string()))?;
        }

        transaction
            .commit()
            .await
            .map_err(|error| KnowledgeOkfConceptLinkStoreError::Internal(error.to_string()))?;
        Ok(())
    }

    async fn list_inbound_concept_ids(
        &self,
        space_id: u64,
        to_concept_id: &str,
    ) -> Result<Vec<String>, KnowledgeOkfConceptLinkStoreError> {
        let tenant_id = to_i64("tenant_id", self.tenant_id)?;
        let organization_id = to_i64("organization_id", self.organization_id)?;
        let space_id = to_i64("space_id", space_id)?;
        let rows = sqlx::query_scalar::<_, String>(
            r#"
            SELECT DISTINCT from_concept_id
            FROM kb_okf_concept_link
            WHERE tenant_id = $1 AND organization_id = $2 AND space_id = $3 AND to_concept_id = $4 AND status = $5
            ORDER BY from_concept_id ASC
            LIMIT 200
            "#,
        )
        .bind(tenant_id)
        .bind(organization_id)
        .bind(space_id)
        .bind(to_concept_id)
        .bind(ACTIVE_STATUS)
        .fetch_all(&self.pool)
        .await
        .map_err(|error| KnowledgeOkfConceptLinkStoreError::Internal(error.to_string()))?;
        Ok(rows)
    }

    async fn list_inbound_link_targets_page(
        &self,
        space_id: u64,
        after_concept_id: Option<&str>,
        limit: u32,
    ) -> Result<InboundLinkTargetsPage, KnowledgeOkfConceptLinkStoreError> {
        let tenant_id = to_i64("tenant_id", self.tenant_id)?;
        let organization_id = to_i64("organization_id", self.organization_id)?;
        let space_id = to_i64("space_id", space_id)?;
        let limit = i64::from(limit.clamp(1, MAX_LINK_SCAN_PAGE_SIZE));
        let rows = sqlx::query(
            r#"
            SELECT DISTINCT to_concept_id
            FROM kb_okf_concept_link
            WHERE tenant_id = $1 AND organization_id = $2 AND space_id = $3 AND status = $4
              AND ($5 IS NULL OR to_concept_id > $5)
            ORDER BY to_concept_id ASC
            LIMIT $6
            "#,
        )
        .bind(tenant_id)
        .bind(organization_id)
        .bind(space_id)
        .bind(ACTIVE_STATUS)
        .bind(after_concept_id)
        .bind(limit + 1)
        .fetch_all(&self.pool)
        .await
        .map_err(|error| KnowledgeOkfConceptLinkStoreError::Internal(error.to_string()))?;

        let has_more = rows.len() as i64 > limit;
        let targets = rows
            .into_iter()
            .take(limit as usize)
            .map(|row| row.try_get::<String, _>("to_concept_id"))
            .collect::<Result<Vec<_>, _>>()
            .map_err(|error| KnowledgeOkfConceptLinkStoreError::Internal(error.to_string()))?;
        let next_cursor = if has_more {
            targets.last().cloned()
        } else {
            None
        };
        Ok(InboundLinkTargetsPage {
            targets,
            next_cursor,
            has_more,
        })
    }

    async fn list_active_link_edges_page(
        &self,
        space_id: u64,
        after: Option<LinkEdgeCursor>,
        limit: u32,
    ) -> Result<LinkEdgePage, KnowledgeOkfConceptLinkStoreError> {
        let tenant_id = to_i64("tenant_id", self.tenant_id)?;
        let organization_id = to_i64("organization_id", self.organization_id)?;
        let space_id = to_i64("space_id", space_id)?;
        let limit = i64::from(limit.clamp(1, MAX_LINK_SCAN_PAGE_SIZE));
        let rows = sqlx::query(
            r#"
            SELECT from_concept_id, to_concept_id, anchor_text
            FROM kb_okf_concept_link
            WHERE tenant_id = $1 AND organization_id = $2 AND space_id = $3 AND status = $4
              AND (
                  $5 IS NULL
                  OR from_concept_id > $5
                  OR (from_concept_id = $5 AND to_concept_id > $6)
                  OR (from_concept_id = $5 AND to_concept_id = $6 AND anchor_text > $7)
              )
            ORDER BY from_concept_id ASC, to_concept_id ASC, anchor_text ASC
            LIMIT $8
            "#,
        )
        .bind(tenant_id)
        .bind(organization_id)
        .bind(space_id)
        .bind(ACTIVE_STATUS)
        .bind(after.as_ref().map(|cursor| cursor.from_concept_id.as_str()))
        .bind(after.as_ref().map(|cursor| cursor.to_concept_id.as_str()))
        .bind(after.as_ref().map(|cursor| cursor.anchor_text.as_str()))
        .bind(limit + 1)
        .fetch_all(&self.pool)
        .await
        .map_err(|error| KnowledgeOkfConceptLinkStoreError::Internal(error.to_string()))?;

        let has_more = rows.len() as i64 > limit;
        let edges = rows
            .into_iter()
            .take(limit as usize)
            .map(|row| {
                Ok(KnowledgeOkfConceptLinkEdge {
                    from_concept_id: row.try_get("from_concept_id").map_err(|error| {
                        KnowledgeOkfConceptLinkStoreError::Internal(error.to_string())
                    })?,
                    to_concept_id: row.try_get("to_concept_id").map_err(|error| {
                        KnowledgeOkfConceptLinkStoreError::Internal(error.to_string())
                    })?,
                    anchor_text: row.try_get("anchor_text").map_err(|error| {
                        KnowledgeOkfConceptLinkStoreError::Internal(error.to_string())
                    })?,
                })
            })
            .collect::<Result<Vec<_>, KnowledgeOkfConceptLinkStoreError>>()?;
        let next_cursor = if has_more {
            edges.last().map(|edge| LinkEdgeCursor {
                from_concept_id: edge.from_concept_id.clone(),
                to_concept_id: edge.to_concept_id.clone(),
                anchor_text: edge.anchor_text.clone(),
            })
        } else {
            None
        };
        Ok(LinkEdgePage {
            edges,
            next_cursor,
            has_more,
        })
    }

    async fn list_active_link_edges_incident_to(
        &self,
        space_id: u64,
        concept_ids: &[String],
        limit: u32,
    ) -> Result<Vec<KnowledgeOkfConceptLinkEdge>, KnowledgeOkfConceptLinkStoreError> {
        let limit = i64::from(limit.clamp(1, MAX_OKF_INCIDENT_LINK_EDGES));
        let Some(concept_id_filter) = incident_concept_id_filter(concept_ids) else {
            return Ok(Vec::new());
        };
        let tenant_id = to_i64("tenant_id", self.tenant_id)?;
        let organization_id = to_i64("organization_id", self.organization_id)?;
        let space_id = to_i64("space_id", space_id)?;

        // SQLx's `Any` driver cannot encode a PostgreSQL text array, so the
        // concept-id list travels as one delimited text parameter and is turned
        // back into an array server-side. Both endpoints are matched so edge
        // expansion sees outbound and inbound links of the candidate set.
        let rows = sqlx::query(
            r#"
            SELECT from_concept_id, to_concept_id, anchor_text
            FROM kb_okf_concept_link
            WHERE tenant_id = $1
              AND organization_id = $2
              AND space_id = $3
              AND status = $4
              AND (
                  from_concept_id = ANY(string_to_array($5, '|'))
                  OR to_concept_id = ANY(string_to_array($5, '|'))
              )
            ORDER BY from_concept_id ASC, to_concept_id ASC, anchor_text ASC
            LIMIT $6
            "#,
        )
        .bind(tenant_id)
        .bind(organization_id)
        .bind(space_id)
        .bind(ACTIVE_STATUS)
        .bind(concept_id_filter)
        .bind(limit)
        .fetch_all(&self.pool)
        .await
        .map_err(|error| KnowledgeOkfConceptLinkStoreError::Internal(error.to_string()))?;

        rows.into_iter()
            .map(|row| {
                Ok(KnowledgeOkfConceptLinkEdge {
                    from_concept_id: row.try_get("from_concept_id").map_err(|error| {
                        KnowledgeOkfConceptLinkStoreError::Internal(error.to_string())
                    })?,
                    to_concept_id: row.try_get("to_concept_id").map_err(|error| {
                        KnowledgeOkfConceptLinkStoreError::Internal(error.to_string())
                    })?,
                    anchor_text: row.try_get("anchor_text").map_err(|error| {
                        KnowledgeOkfConceptLinkStoreError::Internal(error.to_string())
                    })?,
                })
            })
            .collect()
    }
}

/// Joins the concept-id filter into one delimited text parameter, dropping
/// empty ids, delimiter-bearing ids, and duplicates so the server-side
/// `string_to_array` split can never fabricate a concept id.
fn incident_concept_id_filter(concept_ids: &[String]) -> Option<String> {
    let mut ids: Vec<&str> = Vec::new();
    for concept_id in concept_ids {
        if concept_id.is_empty() || concept_id.contains(OKF_INCIDENT_ID_DELIMITER) {
            continue;
        }
        if ids.iter().any(|existing| *existing == concept_id.as_str()) {
            continue;
        }
        ids.push(concept_id.as_str());
        if ids.len() >= MAX_OKF_INCIDENT_CONCEPT_IDS {
            break;
        }
    }
    if ids.is_empty() {
        None
    } else {
        Some(ids.join(OKF_INCIDENT_ID_DELIMITER))
    }
}

fn now_rfc3339() -> Result<String, KnowledgeOkfConceptLinkStoreError> {
    OffsetDateTime::now_utc()
        .format(&Rfc3339)
        .map_err(|error| KnowledgeOkfConceptLinkStoreError::Internal(error.to_string()))
}

fn to_i64(field: &str, value: u64) -> Result<i64, KnowledgeOkfConceptLinkStoreError> {
    i64::try_from(value).map_err(|_| {
        KnowledgeOkfConceptLinkStoreError::Internal(format!("{field} is out of range"))
    })
}

fn id_error(error: crate::KnowledgeIdGeneratorError) -> KnowledgeOkfConceptLinkStoreError {
    KnowledgeOkfConceptLinkStoreError::Internal(error.to_string())
}
