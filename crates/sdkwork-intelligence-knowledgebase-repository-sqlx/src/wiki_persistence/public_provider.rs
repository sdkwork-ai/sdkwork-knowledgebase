use async_trait::async_trait;
use sdkwork_intelligence_knowledgebase_service::ports::{
    knowledge_wiki_persistence::{WikiPersistenceError, WikiPersistenceScope},
    knowledge_wiki_public_provider::{
        ListWikiPublicNavigationRequest, SearchWikiPublicPagesRequest, WikiPublicPageKeyset,
        WikiPublicPageProjection, WikiPublicPageWindow, WikiPublicProviderStore,
        WikiPublicPublication, WikiPublicRouteMatch,
    },
};
use sdkwork_utils_rust::MAX_LIST_PAGE_SIZE;
use sqlx::{any::AnyRow, Row};

use super::{
    from_i64, now, parse_enum, require_id, require_text, row_error, sql_error, to_i64,
    validate_scope, SqlxWikiPersistenceStore,
};

const PUBLIC_PAGE_COLUMNS: &str = r#"
    projection.id AS id,
    projection.uuid AS uuid,
    projection.source_path AS source_path,
    projection.canonical_route AS canonical_route,
    projection.file_kind AS file_kind,
    projection.media_type AS media_type,
    projection.size_bytes AS size_bytes,
    projection.content_sha256 AS content_sha256,
    projection.title AS title,
    projection.description AS description,
    projection.locale AS locale,
    projection.nav_order AS nav_order,
    projection.public_drive_version_uuid AS public_drive_version_uuid,
    projection.page_public_version AS page_public_version,
    TO_CHAR(projection.updated_at AT TIME ZONE 'UTC', 'YYYY-MM-DD"T"HH24:MI:SS.US"Z"') AS public_updated_at
"#;

#[async_trait]
impl WikiPublicProviderStore for SqlxWikiPersistenceStore {
    async fn get_active_publication_by_uuid(
        &self,
        scope: WikiPersistenceScope,
        publication_uuid: &str,
    ) -> Result<Option<WikiPublicPublication>, WikiPersistenceError> {
        validate_scope(scope)?;
        let publication_uuid = require_text("publication_uuid", publication_uuid, 64)?;
        let row = sqlx::query(
            r#"
            SELECT
                id, uuid, tenant_id, organization_id, source_scope_uuid,
                title, description, homepage_source_path, default_locale,
                CAST(supported_locales_json AS TEXT) AS supported_locales_json,
                navigation_mode, theme_key, theme_version, renderer_policy_version,
                CASE WHEN search_enabled THEN 1 ELSE 0 END AS search_enabled_value,
                robots_policy,
                CASE WHEN sitemap_enabled THEN 1 ELSE 0 END AS sitemap_enabled_value,
                provider_generation, navigation_generation, search_generation
            FROM kb_site_publication
            WHERE tenant_id = $1
              AND organization_id = $2
              AND uuid = $3
              AND publication_type = 'wiki'
              AND wiki_status = 'ACTIVE'
              AND source_scope_uuid IS NOT NULL
              AND status = 1
            LIMIT 1
            "#,
        )
        .bind(to_i64("tenant_id", scope.tenant_id)?)
        .bind(to_i64("organization_id", scope.organization_id)?)
        .bind(publication_uuid)
        .fetch_optional(&self.pool)
        .await
        .map_err(sql_error)?;
        row.map(|row| public_publication_from_row(&row)).transpose()
    }

    async fn resolve_public_route(
        &self,
        scope: WikiPersistenceScope,
        publication_id: u64,
        canonical_route: &str,
    ) -> Result<Option<WikiPublicRouteMatch>, WikiPersistenceError> {
        validate_scope(scope)?;
        let publication_id = require_id("publication_id", publication_id)?;
        let canonical_route = require_text("canonical_route", canonical_route, 2_048)?;
        let now = now()?;
        let redirect_time = self.dialect.sql_timestamp_expr("$5");
        let query = format!(
            r#"
            SELECT
                {PUBLIC_PAGE_COLUMNS},
                CASE WHEN projection.canonical_route = $4 THEN 0 ELSE 1 END
                    AS matched_previous_route,
                projection.redirect_status AS redirect_status
            FROM kb_source_file_projection AS projection
            INNER JOIN kb_site_publication AS publication
                ON publication.tenant_id = projection.tenant_id
               AND publication.organization_id = projection.organization_id
               AND publication.id = projection.site_publication_id
            WHERE projection.tenant_id = $1
              AND projection.organization_id = $2
              AND projection.site_publication_id = $3
              AND publication.wiki_status = 'ACTIVE'
              AND publication.status = 1
              AND projection.status = 1
              AND projection.source_state = 'READY'
              AND projection.publication_state = 'PUBLISHED'
              AND projection.visibility IN ('PUBLIC', 'UNLISTED')
              AND projection.public_drive_version_uuid IS NOT NULL
              AND projection.page_public_version > 0
              AND (
                    projection.canonical_route = $4
                    OR (
                        projection.previous_canonical_route = $4
                        AND projection.redirect_status IS NOT NULL
                        AND (
                            projection.redirect_expires_at IS NULL
                            OR projection.redirect_expires_at > {redirect_time}
                        )
                    )
              )
            ORDER BY matched_previous_route ASC, projection.id ASC
            LIMIT 2
            "#,
        );
        let rows = sqlx::query(sqlx::AssertSqlSafe(query.as_str()))
            .bind(to_i64("tenant_id", scope.tenant_id)?)
            .bind(to_i64("organization_id", scope.organization_id)?)
            .bind(publication_id)
            .bind(canonical_route)
            .bind(now)
            .fetch_all(&self.pool)
            .await
            .map_err(sql_error)?;
        if rows.len() > 1 {
            return Err(WikiPersistenceError::Conflict(
                "more than one public Wiki projection owns the requested route".to_string(),
            ));
        }
        rows.first().map(public_route_match_from_row).transpose()
    }

    async fn get_public_content_projection(
        &self,
        scope: WikiPersistenceScope,
        publication_id: u64,
        projection_uuid: &str,
        page_public_version: u64,
    ) -> Result<Option<WikiPublicPageProjection>, WikiPersistenceError> {
        validate_scope(scope)?;
        let publication_id = require_id("publication_id", publication_id)?;
        let projection_uuid = require_text("projection_uuid", projection_uuid, 64)?;
        let page_public_version = require_id("page_public_version", page_public_version)?;
        let query = format!(
            r#"
            SELECT {PUBLIC_PAGE_COLUMNS}
            FROM kb_source_file_projection AS projection
            INNER JOIN kb_site_publication AS publication
                ON publication.tenant_id = projection.tenant_id
               AND publication.organization_id = projection.organization_id
               AND publication.id = projection.site_publication_id
            WHERE projection.tenant_id = $1
              AND projection.organization_id = $2
              AND projection.site_publication_id = $3
              AND projection.uuid = $4
              AND projection.page_public_version = $5
              AND publication.wiki_status = 'ACTIVE'
              AND publication.status = 1
              AND projection.status = 1
              AND projection.source_state = 'READY'
              AND projection.publication_state = 'PUBLISHED'
              AND projection.visibility IN ('PUBLIC', 'UNLISTED')
              AND projection.public_drive_version_uuid IS NOT NULL
            LIMIT 1
            "#,
        );
        let row = sqlx::query(sqlx::AssertSqlSafe(query.as_str()))
            .bind(to_i64("tenant_id", scope.tenant_id)?)
            .bind(to_i64("organization_id", scope.organization_id)?)
            .bind(publication_id)
            .bind(projection_uuid)
            .bind(page_public_version)
            .fetch_optional(&self.pool)
            .await
            .map_err(sql_error)?;
        row.map(|row| public_page_from_row(&row)).transpose()
    }

    async fn list_public_navigation(
        &self,
        request: ListWikiPublicNavigationRequest,
    ) -> Result<WikiPublicPageWindow, WikiPersistenceError> {
        validate_scope(request.scope)?;
        let publication_id = require_id("publication_id", request.publication_id)?;
        let fetch_limit = public_fetch_limit(request.limit)?;
        let locale = normalize_locale(request.locale.as_deref())?;
        let (after_route, after_id) = page_keyset(request.after)?;
        let query = format!(
            r#"
            SELECT {PUBLIC_PAGE_COLUMNS}
            FROM kb_source_file_projection AS projection
            INNER JOIN kb_site_publication AS publication
                ON publication.tenant_id = projection.tenant_id
               AND publication.organization_id = projection.organization_id
               AND publication.id = projection.site_publication_id
            WHERE projection.tenant_id = $1
              AND projection.organization_id = $2
              AND projection.site_publication_id = $3
              AND publication.wiki_status = 'ACTIVE'
              AND publication.status = 1
              AND projection.status = 1
              AND projection.source_state = 'READY'
              AND projection.publication_state = 'PUBLISHED'
              AND projection.visibility = 'PUBLIC'
              AND projection.nav_hidden = FALSE
              AND projection.public_drive_version_uuid IS NOT NULL
              AND ($4 = '' OR projection.locale IS NULL OR projection.locale = $4)
              AND (
                    $5 = ''
                    OR projection.canonical_route > $5
                    OR (projection.canonical_route = $5 AND projection.id > $6)
              )
            ORDER BY projection.canonical_route ASC, projection.id ASC
            LIMIT $7
            "#,
        );
        let rows = sqlx::query(sqlx::AssertSqlSafe(query.as_str()))
            .bind(to_i64("tenant_id", request.scope.tenant_id)?)
            .bind(to_i64("organization_id", request.scope.organization_id)?)
            .bind(publication_id)
            .bind(locale)
            .bind(after_route)
            .bind(after_id)
            .bind(fetch_limit)
            .fetch_all(&self.pool)
            .await
            .map_err(sql_error)?;
        page_window(rows, request.limit)
    }

    async fn search_public_pages(
        &self,
        request: SearchWikiPublicPagesRequest,
    ) -> Result<WikiPublicPageWindow, WikiPersistenceError> {
        validate_scope(request.scope)?;
        let publication_id = require_id("publication_id", request.publication_id)?;
        let query_text = require_text("query", &request.query, 256)?;
        let search_pattern = escaped_like_pattern(query_text);
        let fetch_limit = public_fetch_limit(request.limit)?;
        let locale = normalize_locale(request.locale.as_deref())?;
        let (after_route, after_id) = page_keyset(request.after)?;
        let query = format!(
            r#"
            SELECT {PUBLIC_PAGE_COLUMNS}
            FROM kb_source_file_projection AS projection
            INNER JOIN kb_site_publication AS publication
                ON publication.tenant_id = projection.tenant_id
               AND publication.organization_id = projection.organization_id
               AND publication.id = projection.site_publication_id
            WHERE projection.tenant_id = $1
              AND projection.organization_id = $2
              AND projection.site_publication_id = $3
              AND publication.wiki_status = 'ACTIVE'
              AND publication.search_enabled = TRUE
              AND publication.status = 1
              AND projection.status = 1
              AND projection.source_state = 'READY'
              AND projection.publication_state = 'PUBLISHED'
              AND projection.visibility = 'PUBLIC'
              AND projection.index_state = 'READY'
              AND projection.public_drive_version_uuid IS NOT NULL
              AND ($4 = '' OR projection.locale IS NULL OR projection.locale = $4)
              AND (
                    LOWER(COALESCE(projection.title, '')) LIKE $5 ESCAPE '\'
                    OR LOWER(projection.canonical_route) LIKE $5 ESCAPE '\'
                    OR LOWER(projection.source_path) LIKE $5 ESCAPE '\'
              )
              AND (
                    $6 = ''
                    OR projection.canonical_route > $6
                    OR (projection.canonical_route = $6 AND projection.id > $7)
              )
            ORDER BY projection.canonical_route ASC, projection.id ASC
            LIMIT $8
            "#,
        );
        let rows = sqlx::query(sqlx::AssertSqlSafe(query.as_str()))
            .bind(to_i64("tenant_id", request.scope.tenant_id)?)
            .bind(to_i64("organization_id", request.scope.organization_id)?)
            .bind(publication_id)
            .bind(locale)
            .bind(search_pattern)
            .bind(after_route)
            .bind(after_id)
            .bind(fetch_limit)
            .fetch_all(&self.pool)
            .await
            .map_err(sql_error)?;
        page_window(rows, request.limit)
    }
}

fn public_publication_from_row(
    row: &AnyRow,
) -> Result<WikiPublicPublication, WikiPersistenceError> {
    let supported_locales_json: String =
        row.try_get("supported_locales_json").map_err(row_error)?;
    let supported_locales =
        serde_json::from_str::<Vec<String>>(&supported_locales_json).map_err(|error| {
            WikiPersistenceError::Internal(format!(
                "database returned invalid supported_locales_json: {error}"
            ))
        })?;
    if supported_locales.is_empty()
        || supported_locales
            .iter()
            .any(|locale| locale.is_empty() || locale.len() > 35)
    {
        return Err(WikiPersistenceError::Internal(
            "database returned invalid supported locales".to_string(),
        ));
    }
    Ok(WikiPublicPublication {
        id: from_i64("id", row.try_get("id").map_err(row_error)?)?,
        uuid: row.try_get("uuid").map_err(row_error)?,
        scope: WikiPersistenceScope {
            tenant_id: from_i64("tenant_id", row.try_get("tenant_id").map_err(row_error)?)?,
            organization_id: from_i64(
                "organization_id",
                row.try_get("organization_id").map_err(row_error)?,
            )?,
        },
        source_scope_uuid: row.try_get("source_scope_uuid").map_err(row_error)?,
        title: row.try_get("title").map_err(row_error)?,
        description: row.try_get("description").map_err(row_error)?,
        homepage_source_path: row.try_get("homepage_source_path").map_err(row_error)?,
        default_locale: row.try_get("default_locale").map_err(row_error)?,
        supported_locales,
        navigation_mode: row.try_get("navigation_mode").map_err(row_error)?,
        theme_key: row.try_get("theme_key").map_err(row_error)?,
        theme_version: row.try_get("theme_version").map_err(row_error)?,
        renderer_policy_version: row.try_get("renderer_policy_version").map_err(row_error)?,
        search_enabled: row
            .try_get::<i64, _>("search_enabled_value")
            .map_err(row_error)?
            == 1,
        robots_policy: row.try_get("robots_policy").map_err(row_error)?,
        sitemap_enabled: row
            .try_get::<i64, _>("sitemap_enabled_value")
            .map_err(row_error)?
            == 1,
        provider_generation: from_i64(
            "provider_generation",
            row.try_get("provider_generation").map_err(row_error)?,
        )?,
        navigation_generation: from_i64(
            "navigation_generation",
            row.try_get("navigation_generation").map_err(row_error)?,
        )?,
        search_generation: from_i64(
            "search_generation",
            row.try_get("search_generation").map_err(row_error)?,
        )?,
    })
}

fn public_page_from_row(row: &AnyRow) -> Result<WikiPublicPageProjection, WikiPersistenceError> {
    Ok(WikiPublicPageProjection {
        id: from_i64("id", row.try_get("id").map_err(row_error)?)?,
        uuid: row.try_get("uuid").map_err(row_error)?,
        source_path: row.try_get("source_path").map_err(row_error)?,
        canonical_route: row.try_get("canonical_route").map_err(row_error)?,
        file_kind: parse_enum("file_kind", row.try_get("file_kind").map_err(row_error)?)?,
        media_type: row.try_get("media_type").map_err(row_error)?,
        size_bytes: from_i64("size_bytes", row.try_get("size_bytes").map_err(row_error)?)?,
        content_sha256: row.try_get("content_sha256").map_err(row_error)?,
        title: row.try_get("title").map_err(row_error)?,
        description: row.try_get("description").map_err(row_error)?,
        locale: row.try_get("locale").map_err(row_error)?,
        nav_order: row.try_get("nav_order").map_err(row_error)?,
        public_drive_version_uuid: row
            .try_get("public_drive_version_uuid")
            .map_err(row_error)?,
        page_public_version: from_i64(
            "page_public_version",
            row.try_get("page_public_version").map_err(row_error)?,
        )?,
        public_updated_at: row.try_get("public_updated_at").map_err(row_error)?,
    })
}

fn public_route_match_from_row(row: &AnyRow) -> Result<WikiPublicRouteMatch, WikiPersistenceError> {
    let redirect_status = row
        .try_get::<Option<i32>, _>("redirect_status")
        .map_err(row_error)?
        .map(|value| {
            u16::try_from(value).map_err(|_| {
                WikiPersistenceError::Internal(
                    "database returned invalid Wiki redirect status".to_string(),
                )
            })
        })
        .transpose()?;
    Ok(WikiPublicRouteMatch {
        page: public_page_from_row(row)?,
        matched_previous_route: row
            .try_get::<i32, _>("matched_previous_route")
            .map_err(row_error)?
            == 1,
        redirect_status,
    })
}

fn public_fetch_limit(limit: u32) -> Result<i64, WikiPersistenceError> {
    if !(1..=MAX_LIST_PAGE_SIZE as u32).contains(&limit) {
        return Err(WikiPersistenceError::InvalidRequest(format!(
            "limit must be between 1 and {MAX_LIST_PAGE_SIZE}"
        )));
    }
    Ok(i64::from(limit) + 1)
}

fn normalize_locale(locale: Option<&str>) -> Result<String, WikiPersistenceError> {
    match locale {
        None => Ok(String::new()),
        Some(locale) => Ok(require_text("locale", locale, 35)?.to_string()),
    }
}

fn page_keyset(after: Option<WikiPublicPageKeyset>) -> Result<(String, i64), WikiPersistenceError> {
    match after {
        None => Ok((String::new(), 0)),
        Some(after) => Ok((
            require_text("after.canonical_route", &after.canonical_route, 2_048)?.to_string(),
            require_id("after.page_id", after.page_id)?,
        )),
    }
}

fn page_window(
    rows: Vec<AnyRow>,
    page_size: u32,
) -> Result<WikiPublicPageWindow, WikiPersistenceError> {
    let has_more = rows.len() > page_size as usize;
    let items = rows
        .iter()
        .take(page_size as usize)
        .map(public_page_from_row)
        .collect::<Result<Vec<_>, _>>()?;
    let next = has_more
        .then(|| {
            items.last().map(|item| WikiPublicPageKeyset {
                canonical_route: item.canonical_route.clone(),
                page_id: item.id,
            })
        })
        .flatten();
    Ok(WikiPublicPageWindow { items, next })
}

fn escaped_like_pattern(query: &str) -> String {
    let escaped = query
        .to_lowercase()
        .replace('\\', "\\\\")
        .replace('%', "\\%")
        .replace('_', "\\_");
    format!("%{escaped}%")
}
