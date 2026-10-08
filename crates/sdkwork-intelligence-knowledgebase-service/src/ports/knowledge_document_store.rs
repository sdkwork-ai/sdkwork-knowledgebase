use async_trait::async_trait;
use sdkwork_knowledgebase_contract::document::KnowledgeDocument;
use thiserror::Error;

use crate::tenant_quota::TenantQuotaExceeded;

#[async_trait]
pub trait KnowledgeDocumentStore: Send + Sync {
    async fn create_document(
        &self,
        record: CreateKnowledgeDocumentRecord,
    ) -> Result<KnowledgeDocument, KnowledgeDocumentStoreError>;

    async fn create_or_get_document(
        &self,
        record: CreateKnowledgeDocumentRecord,
    ) -> Result<KnowledgeDocument, KnowledgeDocumentStoreError> {
        self.create_document(record).await
    }

    async fn get_document_by_id(
        &self,
        document_id: u64,
    ) -> Result<KnowledgeDocument, KnowledgeDocumentStoreError> {
        let _ = document_id;
        Err(KnowledgeDocumentStoreError::Unsupported(
            "get_document_by_id is unsupported by this knowledge document store".to_string(),
        ))
    }

    async fn list_documents_for_space(
        &self,
        space_id: u64,
        limit: u32,
    ) -> Result<Vec<KnowledgeDocument>, KnowledgeDocumentStoreError> {
        let (items, _, _) = self.list_documents_page(space_id, None, limit).await?;
        Ok(items)
    }

    async fn list_documents_page(
        &self,
        space_id: u64,
        cursor: Option<u64>,
        page_size: u32,
    ) -> Result<(Vec<KnowledgeDocument>, Option<String>, bool), KnowledgeDocumentStoreError> {
        let _ = (space_id, cursor, page_size);
        Err(KnowledgeDocumentStoreError::Unsupported(
            "list_documents_page is unsupported by this knowledge document store".to_string(),
        ))
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CreateKnowledgeDocumentRecord {
    pub space_id: u64,
    pub source_id: Option<u64>,
    pub identity_scope: KnowledgeDocumentIdentityScope,
    pub original_file_drive_node_id: Option<String>,
    pub title: String,
    pub mime_type: Option<String>,
    pub language: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum KnowledgeDocumentIdentityScope {
    SourceOnly,
    SourceAndOriginalDriveNode,
}

impl KnowledgeDocumentIdentityScope {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::SourceOnly => "source_only",
            Self::SourceAndOriginalDriveNode => "source_and_original_drive_node",
        }
    }
}

#[derive(Debug, Error, Clone, PartialEq, Eq)]
pub enum KnowledgeDocumentStoreError {
    #[error("knowledge document store invalid record: {0}")]
    InvalidRecord(String),
    #[error("knowledge document store unsupported operation: {0}")]
    Unsupported(String),
    #[error(transparent)]
    QuotaExceeded(#[from] TenantQuotaExceeded),
    #[error("knowledge document was not found: {0}")]
    NotFound(String),
    #[error("knowledge document store internal error: {0}")]
    Internal(String),
}

#[cfg(test)]
mod tests {
    use super::*;

    struct MinimalDocumentStore;

    #[async_trait]
    impl KnowledgeDocumentStore for MinimalDocumentStore {
        async fn create_document(
            &self,
            _record: CreateKnowledgeDocumentRecord,
        ) -> Result<KnowledgeDocument, KnowledgeDocumentStoreError> {
            Err(KnowledgeDocumentStoreError::Unsupported(
                "document creation is unsupported by this store".to_string(),
            ))
        }
    }

    #[tokio::test]
    async fn default_document_lookup_reports_unsupported_instead_of_internal_placeholder() {
        let error = MinimalDocumentStore
            .get_document_by_id(42)
            .await
            .expect_err("default document lookup should be unsupported");

        match error {
            KnowledgeDocumentStoreError::Unsupported(detail) => {
                assert!(detail.contains("get_document_by_id"));
                assert!(detail.contains("unsupported"));
            }
            other => panic!("expected unsupported default document lookup error, got {other:?}"),
        }
    }

    #[tokio::test]
    async fn default_document_page_reports_unsupported_instead_of_internal_placeholder() {
        let error = MinimalDocumentStore
            .list_documents_page(7, None, 20)
            .await
            .expect_err("default document paging should be unsupported");

        match error {
            KnowledgeDocumentStoreError::Unsupported(detail) => {
                assert!(detail.contains("list_documents_page"));
                assert!(detail.contains("unsupported"));
            }
            other => panic!("expected unsupported default document page error, got {other:?}"),
        }
    }
}
