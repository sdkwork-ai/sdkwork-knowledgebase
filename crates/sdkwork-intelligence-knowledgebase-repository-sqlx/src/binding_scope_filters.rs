//! Applies `KnowledgeRetrievalBinding` source/document filters to chunk search SQL.
//!
//! 方言契约：服务端权威持久化仅支持 PostgreSQL（DATABASE_SPEC：authoritative-server），
//! 因此本模块统一生成 PostgreSQL JSON 语法（`metadata::jsonb ->>`），不再提供 SQLite 方言。
//! 泛型 `DB` 仅用于让 `QueryBuilder<Any>`（兼容池）与 `QueryBuilder<Postgres>`（pgvector 池）
//! 复用同一实现，消除双份重复代码。

use sdkwork_intelligence_knowledgebase_service::ports::knowledge_retrieval_backend::KnowledgeRetrievalBackendError;
use sdkwork_knowledgebase_contract::rag::{KnowledgeFilter, KnowledgeRetrievalBinding};
use sqlx::{Database, Encode, QueryBuilder, Type};

const ACTIVE_STATUS: i64 = 1;

/// 单一实现：按 binding 过滤条件追加 WHERE 片段（PostgreSQL 语法）。
pub fn push_binding_scope_filters<DB>(
    query: &mut QueryBuilder<DB>,
    tenant_id: i64,
    organization_id: i64,
    space_id: i64,
    binding: &KnowledgeRetrievalBinding,
) -> Result<(), KnowledgeRetrievalBackendError>
where
    DB: Database,
    for<'q> i64: Encode<'q, DB> + Type<DB>,
    for<'q> String: Encode<'q, DB> + Type<DB>,
{
    if let Some(filters) = binding.source_filter.as_ref() {
        for filter in filters {
            push_source_scope_filter(query, tenant_id, organization_id, space_id, filter)?;
        }
    }
    if let Some(filters) = binding.document_filter.as_ref() {
        for filter in filters {
            push_document_scope_filter(query, filter)?;
        }
    }
    Ok(())
}

fn push_source_scope_filter<DB>(
    query: &mut QueryBuilder<DB>,
    tenant_id: i64,
    organization_id: i64,
    space_id: i64,
    filter: &KnowledgeFilter,
) -> Result<(), KnowledgeRetrievalBackendError>
where
    DB: Database,
    for<'q> i64: Encode<'q, DB> + Type<DB>,
    for<'q> String: Encode<'q, DB> + Type<DB>,
{
    match classify_source_filter_key(&filter.key) {
        SourceFilterKey::SourceType => {
            query.push(
                " AND d.source_id IN (SELECT id FROM kb_source WHERE (tenant_id, organization_id) = (",
            );
            query.push_bind(tenant_id);
            query.push(", ");
            query.push_bind(organization_id);
            query.push(") AND space_id = ");
            query.push_bind(space_id);
            query.push(" AND source_type = ");
            query.push_bind(filter.value.clone());
            query.push(" AND status = ");
            query.push_bind(ACTIVE_STATUS);
            query.push(")");
        }
        SourceFilterKey::Provider => {
            query.push(
                " AND d.source_id IN (SELECT id FROM kb_source WHERE (tenant_id, organization_id) = (",
            );
            query.push_bind(tenant_id);
            query.push(", ");
            query.push_bind(organization_id);
            query.push(") AND space_id = ");
            query.push_bind(space_id);
            query.push(" AND provider = ");
            query.push_bind(filter.value.clone());
            query.push(" AND status = ");
            query.push_bind(ACTIVE_STATUS);
            query.push(")");
        }
        SourceFilterKey::SourceId => {
            let source_id = parse_filter_u64("sourceId", &filter.value)?;
            query.push(" AND d.source_id = ");
            query.push_bind(source_id);
        }
        SourceFilterKey::Unsupported(key) => {
            return Err(unsupported_binding_filter_key(key));
        }
    }
    Ok(())
}

fn push_document_scope_filter<DB>(
    query: &mut QueryBuilder<DB>,
    filter: &KnowledgeFilter,
) -> Result<(), KnowledgeRetrievalBackendError>
where
    DB: Database,
    for<'q> i64: Encode<'q, DB> + Type<DB>,
    for<'q> String: Encode<'q, DB> + Type<DB>,
{
    match classify_document_filter_key(&filter.key) {
        DocumentFilterKey::DocumentId => {
            let document_id = parse_filter_u64("documentId", &filter.value)?;
            query.push(" AND d.id = ");
            query.push_bind(document_id);
        }
        DocumentFilterKey::Language => {
            query.push(" AND d.language = ");
            query.push_bind(filter.value.clone());
        }
        DocumentFilterKey::MimeType => {
            query.push(" AND d.mime_type = ");
            query.push_bind(filter.value.clone());
        }
        DocumentFilterKey::Visibility => {
            let visibility = parse_filter_u64("visibility", &filter.value)?;
            query.push(" AND d.visibility = ");
            query.push_bind(visibility);
        }
        // PostgreSQL JSON 语义：按顶层 key 提取文本后比较。
        // 绑定裸 key（而非 "$.path" 路径前缀），与 pgvector 检索后端行为一致。
        DocumentFilterKey::Metadata(path) => {
            query.push(" AND d.metadata::jsonb ->> ");
            query.push_bind(path);
            query.push(" = ");
            query.push_bind(filter.value.clone());
        }
        DocumentFilterKey::Unsupported(key) => {
            return Err(unsupported_binding_filter_key(key));
        }
    }
    Ok(())
}

enum SourceFilterKey {
    SourceType,
    Provider,
    SourceId,
    Unsupported(String),
}

enum DocumentFilterKey {
    DocumentId,
    Language,
    MimeType,
    Visibility,
    Metadata(String),
    Unsupported(String),
}

fn classify_source_filter_key(key: &str) -> SourceFilterKey {
    match normalize_filter_key(key).as_str() {
        "source_type" | "sourcetype" => SourceFilterKey::SourceType,
        "provider" => SourceFilterKey::Provider,
        "source_id" | "sourceid" => SourceFilterKey::SourceId,
        _ => SourceFilterKey::Unsupported(key.to_string()),
    }
}

fn classify_document_filter_key(key: &str) -> DocumentFilterKey {
    if let Some(path) = key.strip_prefix("metadata.") {
        if path.is_empty() {
            return DocumentFilterKey::Unsupported(key.to_string());
        }
        return DocumentFilterKey::Metadata(path.to_string());
    }

    match normalize_filter_key(key).as_str() {
        "document_id" | "documentid" => DocumentFilterKey::DocumentId,
        "language" => DocumentFilterKey::Language,
        "mime_type" | "mimetype" => DocumentFilterKey::MimeType,
        "visibility" => DocumentFilterKey::Visibility,
        _ => DocumentFilterKey::Unsupported(key.to_string()),
    }
}

fn normalize_filter_key(key: &str) -> String {
    key.trim().to_ascii_lowercase().replace('-', "_")
}

fn parse_filter_u64(field: &str, value: &str) -> Result<i64, KnowledgeRetrievalBackendError> {
    let parsed = value.trim().parse::<u64>().map_err(|_| {
        KnowledgeRetrievalBackendError::Internal(format!(
            "binding filter {field} expects a numeric value"
        ))
    })?;
    i64::try_from(parsed).map_err(|_| {
        KnowledgeRetrievalBackendError::Internal(format!(
            "binding filter {field} exceeds i64 range"
        ))
    })
}

fn unsupported_binding_filter_key(key: String) -> KnowledgeRetrievalBackendError {
    KnowledgeRetrievalBackendError::Internal(format!(
        "unsupported knowledge retrieval binding filter key: {key}"
    ))
}

#[cfg(test)]
mod tests {
    use super::*;
    use sqlx::Any;

    fn binding_with_document_filter(key: &str, value: &str) -> KnowledgeRetrievalBinding {
        serde_json::from_value(serde_json::json!({
            "spaceId": "1",
            "priority": 0,
            "documentFilter": [{ "key": key, "value": value }]
        }))
        .expect("binding fixture")
    }

    fn built_sql(binding: &KnowledgeRetrievalBinding) -> String {
        let mut query = QueryBuilder::<Any>::new("SELECT 1 FROM kb_document d WHERE 1 = 1");
        push_binding_scope_filters(&mut query, 1, 1, 1, binding).expect("filters");
        query.into_sql().as_str().to_string()
    }

    #[test]
    fn classifies_contract_source_and_document_filter_keys() {
        assert!(matches!(
            classify_source_filter_key("sourceType"),
            SourceFilterKey::SourceType
        ));
        assert!(matches!(
            classify_document_filter_key("language"),
            DocumentFilterKey::Language
        ));
        assert!(matches!(
            classify_document_filter_key("metadata.locale"),
            DocumentFilterKey::Metadata(path) if path == "locale"
        ));
    }

    /// 回归测试（P0）：Any 兼容池路径必须生成 PostgreSQL JSON 语法。
    /// 历史缺陷：该路径曾生成 SQLite 方言 `json_extract(...)`，在 PostgreSQL 上
    /// 对带 metadata 过滤的检索产生确定性 `function json_extract(...) does not exist`。
    #[test]
    fn any_path_metadata_filter_uses_postgres_json_syntax() {
        let binding = binding_with_document_filter("metadata.locale", "zh-CN");
        let sql = built_sql(&binding);
        assert!(
            sql.contains("d.metadata::jsonb ->> "),
            "expected PostgreSQL jsonb extraction, got: {sql}"
        );
        assert!(
            !sql.contains("json_extract"),
            "SQLite/MySQL dialect leaked into retrieval SQL: {sql}"
        );
        assert!(!sql.contains("$."), "JSON path prefix leaked into binds: {sql}");
    }

    #[test]
    fn any_path_document_filter_binds_value() {
        let binding = binding_with_document_filter("language", "zh-CN");
        let sql = built_sql(&binding);
        assert!(sql.contains(" AND d.language = "), "got: {sql}");
    }

    #[test]
    fn unsupported_filter_keys_fail_closed() {
        let binding = binding_with_document_filter("metadata.", "x");
        let mut query = QueryBuilder::<Any>::new("SELECT 1");
        let error = push_binding_scope_filters(&mut query, 1, 1, 1, &binding)
            .expect_err("empty metadata path must fail closed");
        assert!(error.to_string().contains("unsupported"));
    }
}
