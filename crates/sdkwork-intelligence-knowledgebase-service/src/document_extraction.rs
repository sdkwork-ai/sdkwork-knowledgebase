//! Content extraction SPI for binary Drive documents.
//!
//! Markdown and plain-text payloads are read as UTF-8 directly. PDF, DOCX, and
//! PPTX uploads need a bounded text extraction step before chunking. The
//! extraction implementation lives in a separate adapter crate; this port keeps
//! the ingestion pipeline free of parser dependencies and lets a deployment
//! swap the extractor without touching the pipeline.
//!
//! Contract:
//!
//! - `supports` is a pure predicate over the declared content type and optional
//!   file name; it MUST NOT read the payload.
//! - `extract` is synchronous CPU work. The pipeline runs it on a blocking
//!   worker with a deadline and must never hold a database transaction.
//! - Decoders MUST be bounded: input size, decompressed size, page count, and
//!   extracted character count all have explicit ceilings.
//! - Failures are typed and sanitized; the ingestion job records a bounded
//!   reason, never raw document content.

use std::time::Duration;

use thiserror::Error;

/// Maximum accepted input bytes for one extraction request.
pub const MAX_DOCUMENT_EXTRACTION_INPUT_BYTES: u64 = 32 * 1024 * 1024;
/// Maximum characters accepted from one extraction result.
pub const MAX_EXTRACTED_TEXT_CHARS: usize = 2_000_000;
/// Maximum pages read from a paginated document.
pub const MAX_DOCUMENT_EXTRACTION_PAGES: u32 = 500;
/// Default extraction deadline applied by the pipeline.
pub const DEFAULT_DOCUMENT_EXTRACTION_TIMEOUT: Duration = Duration::from_secs(30);

/// One bounded binary document handed to an extractor.
#[derive(Clone)]
pub struct DocumentExtractionRequest {
    pub content_type: String,
    pub file_name: Option<String>,
    pub bytes: Vec<u8>,
}

impl std::fmt::Debug for DocumentExtractionRequest {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("DocumentExtractionRequest")
            .field("content_type", &self.content_type)
            .field("file_name", &self.file_name)
            .field("bytes", &self.bytes.len())
            .finish()
    }
}

/// Text extracted from one binary document.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ExtractedDocument {
    pub text: String,
    /// Stable format label (`pdf`, `docx`, `pptx`, `text`).
    pub format: String,
    pub pages: Option<u32>,
    /// True when the authoritative text exceeded
    /// [`MAX_EXTRACTED_TEXT_CHARS`] and was cut on a character boundary.
    pub truncated: bool,
}

/// Text extraction boundary for binary documents.
pub trait DocumentTextExtractor: Send + Sync {
    /// Returns whether this extractor can decode the declared document.
    fn supports(&self, content_type: &str, file_name: Option<&str>) -> bool;

    /// Decodes one bounded document into plain text.
    fn extract(
        &self,
        request: DocumentExtractionRequest,
    ) -> Result<ExtractedDocument, DocumentExtractionError>;
}

/// Classifies a payload as UTF-8 text that does not need a binary extractor.
#[must_use]
pub fn is_textual_content_type(content_type: &str, file_name: Option<&str>) -> bool {
    let content_type = content_type.trim().to_ascii_lowercase();
    if content_type.starts_with("text/") {
        return true;
    }
    matches!(
        content_type.as_str(),
        "application/json"
            | "application/xml"
            | "application/x-yaml"
            | "application/yaml"
            | "application/x-ndjson"
            | "application/javascript"
            | "application/typescript"
            | "application/sql"
    ) || file_extension(file_name).is_some_and(|extension| {
        matches!(
            extension.as_str(),
            "md" | "markdown"
                | "txt"
                | "csv"
                | "tsv"
                | "json"
                | "jsonl"
                | "yaml"
                | "yml"
                | "xml"
                | "html"
                | "htm"
                | "log"
                | "rs"
                | "ts"
                | "tsx"
                | "js"
                | "jsx"
                | "py"
                | "go"
                | "java"
                | "kt"
                | "sql"
                | "toml"
                | "ini"
                | "conf"
                | "sh"
                | "ps1"
        )
    })
}

/// Returns the lowercase extension without the dot.
#[must_use]
pub fn file_extension(file_name: Option<&str>) -> Option<String> {
    let name = file_name?.trim();
    let (_, extension) = name.rsplit_once('.')?;
    let extension = extension.trim().to_ascii_lowercase();
    (!extension.is_empty()).then_some(extension)
}

/// Bounded failure raised by a document extractor.
#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub enum DocumentExtractionError {
    #[error("unsupported document content type: {0}")]
    Unsupported(String),
    #[error("document is corrupt or malformed: {0}")]
    Corrupt(String),
    #[error("document extraction failed: {0}")]
    Failed(String),
    #[error("document exceeds the {limit}-byte extraction input limit")]
    InputTooLarge { limit: u64 },
}

impl DocumentExtractionError {
    /// Sanitizes a decoder error into a bounded, content-free message.
    #[must_use]
    pub fn corrupt(error: impl std::fmt::Display) -> Self {
        Self::Corrupt(bounded_reason(error.to_string()))
    }

    /// Sanitizes a decoder failure into a bounded, content-free message.
    #[must_use]
    pub fn failed(error: impl std::fmt::Display) -> Self {
        Self::Failed(bounded_reason(error.to_string()))
    }
}

/// Truncates extracted text on a character boundary to the global ceiling.
#[must_use]
pub fn bound_extracted_text(document: ExtractedDocument) -> ExtractedDocument {
    if document.text.chars().count() <= MAX_EXTRACTED_TEXT_CHARS {
        return document;
    }
    let text = document
        .text
        .chars()
        .take(MAX_EXTRACTED_TEXT_CHARS)
        .collect::<String>();
    ExtractedDocument {
        text,
        truncated: true,
        ..document
    }
}

fn bounded_reason(message: String) -> String {
    const MAX_REASON_CHARS: usize = 256;
    if message.chars().count() <= MAX_REASON_CHARS {
        return message;
    }
    message.chars().take(MAX_REASON_CHARS).collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn textual_content_types_and_extensions_are_recognized() {
        assert!(is_textual_content_type("text/markdown", None));
        assert!(is_textual_content_type(
            "application/octet-stream",
            Some("notes.md")
        ));
        assert!(is_textual_content_type("application/json", None));
        assert!(!is_textual_content_type("application/pdf", Some("a.pdf")));
        assert!(!is_textual_content_type(
            "application/vnd.openxmlformats-officedocument.wordprocessingml.document",
            Some("a.docx")
        ));
    }

    #[test]
    fn extracted_text_is_bounded_on_a_character_boundary() {
        let document = ExtractedDocument {
            text: "知".repeat(MAX_EXTRACTED_TEXT_CHARS + 10),
            format: "text".to_string(),
            pages: None,
            truncated: false,
        };
        let bounded = bound_extracted_text(document);
        assert!(bounded.truncated);
        assert_eq!(bounded.text.chars().count(), MAX_EXTRACTED_TEXT_CHARS);
    }

    #[test]
    fn extraction_reasons_are_bounded_and_never_echo_payload() {
        let error = DocumentExtractionError::corrupt("x".repeat(1_000));
        assert!(error.to_string().chars().count() < 300);
    }
}
