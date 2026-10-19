//! Bounded PDF/DOCX/PPTX/XLSX text extraction for Knowledgebase ingestion.
//!
//! The ingestion pipeline reads markdown/plain text directly and delegates
//! binary documents to this adapter through
//! [`sdkwork_intelligence_knowledgebase_service::document_extraction::DocumentTextExtractor`].
//!
//! Every decoder is bounded:
//!
//! - the request size is validated before parsing;
//! - OOXML archives cap each decompressed entry and the number of slides;
//! - the PDF parser runs inside `catch_unwind`, because malformed PDFs can
//!   panic a pure-Rust decoder;
//! - extracted text is normalized and capped by the service port.
//!
//! The adapter intentionally does not support legacy `.doc`/`.ppt`/`.xls`,
//! encrypted PDFs, or scanned/image-only PDFs (no OCR). Those fail with a typed
//! error so the ingestion job records an actionable reason instead of indexing
//! garbage.

use std::io::{Cursor, Read};

use sdkwork_intelligence_knowledgebase_service::document_extraction::{
    file_extension, DocumentExtractionError, DocumentExtractionRequest, DocumentTextExtractor,
    ExtractedDocument, MAX_DOCUMENT_EXTRACTION_INPUT_BYTES, MAX_DOCUMENT_EXTRACTION_PAGES,
};
use zip::ZipArchive;

/// Maximum decompressed bytes read from one OOXML entry.
pub const MAX_OOXML_ENTRY_BYTES: u64 = 16 * 1024 * 1024;
/// Maximum number of PPTX slides read.
pub const MAX_PPTX_SLIDES: u32 = MAX_DOCUMENT_EXTRACTION_PAGES;
/// Maximum number of XLSX worksheets read.
pub const MAX_XLSX_WORKSHEETS: u32 = 100;
/// Maximum number of XLSX rows read across one worksheet.
pub const MAX_XLSX_ROWS: usize = 50_000;

/// Default extractor for the ingestion pipeline.
#[derive(Debug, Default, Clone, Copy)]
pub struct DefaultDocumentTextExtractor;

impl DefaultDocumentTextExtractor {
    /// Creates the extractor.
    #[must_use]
    pub fn new() -> Self {
        Self
    }
}

impl DocumentTextExtractor for DefaultDocumentTextExtractor {
    fn supports(&self, content_type: &str, file_name: Option<&str>) -> bool {
        document_kind(content_type, file_name).is_some()
    }

    fn extract(
        &self,
        request: DocumentExtractionRequest,
    ) -> Result<ExtractedDocument, DocumentExtractionError> {
        if request.bytes.len() as u64 > MAX_DOCUMENT_EXTRACTION_INPUT_BYTES {
            return Err(DocumentExtractionError::InputTooLarge {
                limit: MAX_DOCUMENT_EXTRACTION_INPUT_BYTES,
            });
        }
        let Some(kind) = document_kind(&request.content_type, request.file_name.as_deref()) else {
            return Err(DocumentExtractionError::Unsupported(format!(
                "{}",
                request.content_type
            )));
        };
        match kind {
            DocumentKind::Pdf => extract_pdf(&request.bytes),
            DocumentKind::Docx => extract_docx(&request.bytes),
            DocumentKind::Pptx => extract_pptx(&request.bytes),
            DocumentKind::Xlsx => extract_xlsx(&request.bytes),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum DocumentKind {
    Pdf,
    Docx,
    Pptx,
    Xlsx,
}

fn document_kind(content_type: &str, file_name: Option<&str>) -> Option<DocumentKind> {
    let content_type = content_type.trim().to_ascii_lowercase();
    if content_type == "application/pdf" {
        return Some(DocumentKind::Pdf);
    }
    if content_type == "application/vnd.openxmlformats-officedocument.wordprocessingml.document" {
        return Some(DocumentKind::Docx);
    }
    if content_type == "application/vnd.openxmlformats-officedocument.presentationml.presentation" {
        return Some(DocumentKind::Pptx);
    }
    if content_type == "application/vnd.openxmlformats-officedocument.spreadsheetml.sheet" {
        return Some(DocumentKind::Xlsx);
    }
    match file_extension(file_name)?.as_str() {
        "pdf" => Some(DocumentKind::Pdf),
        "docx" => Some(DocumentKind::Docx),
        "pptx" => Some(DocumentKind::Pptx),
        "xlsx" => Some(DocumentKind::Xlsx),
        _ => None,
    }
}

fn extract_pdf(bytes: &[u8]) -> Result<ExtractedDocument, DocumentExtractionError> {
    // `pdf-extract` parses untrusted input in-process; a panic on a malformed
    // document must become a typed failure, not a poisoned ingestion worker.
    let parsed = std::panic::catch_unwind(|| pdf_extract::extract_text_from_mem(bytes))
        .map_err(|_| DocumentExtractionError::Corrupt("pdf parser aborted".to_string()))?;
    let text = parsed.map_err(DocumentExtractionError::corrupt)?;
    let text = normalize_text(&text);
    if text.trim().is_empty() {
        return Err(DocumentExtractionError::Corrupt(
            "pdf contains no extractable text (scanned or encrypted PDF?)".to_string(),
        ));
    }
    Ok(ExtractedDocument {
        text,
        format: "pdf".to_string(),
        pages: None,
        truncated: false,
    })
}

fn extract_docx(bytes: &[u8]) -> Result<ExtractedDocument, DocumentExtractionError> {
    let xml = read_zip_entry(bytes, "word/document.xml")?;
    let text = normalize_text(&extract_ooxml_text(&xml)?);
    if text.trim().is_empty() {
        return Err(DocumentExtractionError::Corrupt(
            "docx document contains no text".to_string(),
        ));
    }
    Ok(ExtractedDocument {
        text,
        format: "docx".to_string(),
        pages: None,
        truncated: false,
    })
}

fn extract_pptx(bytes: &[u8]) -> Result<ExtractedDocument, DocumentExtractionError> {
    let mut archive = ZipArchive::new(Cursor::new(bytes))
        .map_err(|error| DocumentExtractionError::Corrupt(format!("pptx archive: {error}")))?;
    let mut slides = 0u32;
    let mut sections = Vec::new();
    for index in 1..=MAX_PPTX_SLIDES {
        let name = format!("ppt/slides/slide{index}.xml");
        let mut entry = match archive.by_name(&name) {
            Ok(entry) => entry,
            Err(_) => break,
        };
        if entry.size() > MAX_OOXML_ENTRY_BYTES {
            return Err(DocumentExtractionError::Corrupt(format!(
                "pptx slide {index} exceeds the decompressed entry limit"
            )));
        }
        let mut xml = String::with_capacity(entry.size().min(MAX_OOXML_ENTRY_BYTES) as usize);
        entry.read_to_string(&mut xml).map_err(|error| {
            DocumentExtractionError::Corrupt(format!("pptx slide {index}: {error}"))
        })?;
        let text = normalize_text(&extract_ooxml_text(&xml)?);
        if !text.trim().is_empty() {
            sections.push(text);
            slides += 1;
        }
    }
    if sections.is_empty() {
        return Err(DocumentExtractionError::Corrupt(
            "pptx presentation contains no extractable slide text".to_string(),
        ));
    }
    Ok(ExtractedDocument {
        text: sections.join("\n\n---\n\n"),
        format: "pptx".to_string(),
        pages: Some(slides),
        truncated: false,
    })
}

/// Extracts cell text from SpreadsheetML.
///
/// Shared strings are resolved by index, inline strings are read directly, and
/// numeric/boolean cells keep their raw value. Rows become tab-separated lines so
/// ingestion chunks preserve the table shape instead of gluing every cell
/// together.
fn extract_xlsx(bytes: &[u8]) -> Result<ExtractedDocument, DocumentExtractionError> {
    let mut archive = ZipArchive::new(Cursor::new(bytes))
        .map_err(|error| DocumentExtractionError::Corrupt(format!("xlsx archive: {error}")))?;
    let shared_strings = match read_archive_entry(&mut archive, "xl/sharedStrings.xml")? {
        Some(xml) => parse_shared_strings(&xml)?,
        None => Vec::new(),
    };

    let mut sheets = 0u32;
    let mut sections = Vec::new();
    for index in 1..=MAX_XLSX_WORKSHEETS {
        let name = format!("xl/worksheets/sheet{index}.xml");
        let Some(xml) = read_archive_entry(&mut archive, &name)? else {
            break;
        };
        let rows = parse_worksheet_rows(&xml, &shared_strings)?;
        if !rows.is_empty() {
            sheets += 1;
            sections.push(rows.join("\n"));
        }
    }
    if sections.is_empty() {
        return Err(DocumentExtractionError::Corrupt(
            "xlsx workbook contains no extractable cell text".to_string(),
        ));
    }
    Ok(ExtractedDocument {
        text: normalize_text(&sections.join("\n\n")),
        format: "xlsx".to_string(),
        pages: Some(sheets),
        truncated: false,
    })
}

fn read_archive_entry(
    archive: &mut ZipArchive<Cursor<&[u8]>>,
    name: &str,
) -> Result<Option<String>, DocumentExtractionError> {
    let mut entry = match archive.by_name(name) {
        Ok(entry) => entry,
        Err(zip::result::ZipError::FileNotFound) => return Ok(None),
        Err(error) => {
            return Err(DocumentExtractionError::Corrupt(format!(
                "ooxml part {name}: {error}"
            )))
        }
    };
    if entry.size() > MAX_OOXML_ENTRY_BYTES {
        return Err(DocumentExtractionError::Corrupt(format!(
            "ooxml part {name} exceeds the decompressed entry limit"
        )));
    }
    let mut xml = String::with_capacity(entry.size().min(MAX_OOXML_ENTRY_BYTES) as usize);
    entry
        .read_to_string(&mut xml)
        .map_err(|error| DocumentExtractionError::Corrupt(format!("ooxml part {name}: {error}")))?;
    Ok(Some(xml))
}

fn parse_shared_strings(xml: &str) -> Result<Vec<String>, DocumentExtractionError> {
    use quick_xml::events::Event;

    let mut reader = quick_xml::Reader::from_str(xml);
    reader.config_mut().trim_text(false);
    let mut strings = Vec::new();
    let mut current = String::new();
    let mut in_si = false;
    let mut in_text = false;
    loop {
        match reader.read_event() {
            Ok(Event::Start(element)) => match element.local_name().as_ref() {
                b"si" => {
                    in_si = true;
                    current.clear();
                }
                b"t" if in_si => in_text = true,
                _ => {}
            },
            Ok(Event::Text(text)) if in_text => {
                let decoded = text.unescape().map_err(|error| {
                    DocumentExtractionError::Corrupt(format!("xlsx shared string: {error}"))
                })?;
                current.push_str(&decoded);
            }
            Ok(Event::End(element)) => match element.local_name().as_ref() {
                b"t" => in_text = false,
                b"si" if in_si => {
                    strings.push(std::mem::take(&mut current));
                    in_si = false;
                }
                _ => {}
            },
            Ok(Event::Eof) => break,
            Err(error) => {
                return Err(DocumentExtractionError::Corrupt(format!(
                    "xlsx shared strings: {error}"
                )))
            }
            _ => {}
        }
    }
    Ok(strings)
}

fn parse_worksheet_rows(
    xml: &str,
    shared_strings: &[String],
) -> Result<Vec<String>, DocumentExtractionError> {
    use quick_xml::events::Event;

    let mut reader = quick_xml::Reader::from_str(xml);
    reader.config_mut().trim_text(false);
    let mut rows = Vec::new();
    let mut cells = Vec::new();
    let mut in_row = false;
    let mut in_cell = false;
    let mut in_value = false;
    let mut in_inline_text = false;
    let mut cell_type = String::new();
    let mut cell_value = String::new();
    loop {
        match reader.read_event() {
            Ok(Event::Start(element)) => match element.local_name().as_ref() {
                b"row" => {
                    in_row = true;
                    cells.clear();
                }
                b"c" if in_row => {
                    in_cell = true;
                    in_value = false;
                    in_inline_text = false;
                    cell_value.clear();
                    cell_type.clear();
                    for attribute in element.attributes() {
                        let attribute = attribute.map_err(|error| {
                            DocumentExtractionError::Corrupt(format!("xlsx cell: {error}"))
                        })?;
                        if attribute.key.local_name().as_ref() == b"t" {
                            cell_type = attribute
                                .unescape_value()
                                .map_err(|error| {
                                    DocumentExtractionError::Corrupt(format!(
                                        "xlsx cell type: {error}"
                                    ))
                                })?
                                .into_owned();
                        }
                    }
                }
                b"v" if in_cell => in_value = true,
                b"t" if in_cell => in_inline_text = true,
                _ => {}
            },
            Ok(Event::Text(text)) if in_cell && (in_value || in_inline_text) => {
                let decoded = text.unescape().map_err(|error| {
                    DocumentExtractionError::Corrupt(format!("xlsx cell text: {error}"))
                })?;
                cell_value.push_str(&decoded);
            }
            Ok(Event::End(element)) => match element.local_name().as_ref() {
                b"v" => in_value = false,
                b"t" => in_inline_text = false,
                b"c" if in_cell => {
                    let raw = cell_value.trim();
                    let value = if cell_type == "s" {
                        raw.parse::<usize>()
                            .ok()
                            .and_then(|index| shared_strings.get(index))
                            .cloned()
                            .unwrap_or_default()
                    } else {
                        raw.to_string()
                    };
                    cells.push(value);
                    in_cell = false;
                }
                b"row" => {
                    if cells.iter().any(|value| !value.trim().is_empty()) {
                        if rows.len() >= MAX_XLSX_ROWS {
                            return Err(DocumentExtractionError::Corrupt(format!(
                                "xlsx worksheet exceeds the {MAX_XLSX_ROWS} row limit"
                            )));
                        }
                        rows.push(cells.join("\t"));
                    }
                    in_row = false;
                }
                _ => {}
            },
            Ok(Event::Eof) => break,
            Err(error) => {
                return Err(DocumentExtractionError::Corrupt(format!(
                    "xlsx worksheet: {error}"
                )))
            }
            _ => {}
        }
    }
    Ok(rows)
}

fn read_zip_entry(bytes: &[u8], name: &str) -> Result<String, DocumentExtractionError> {
    let mut archive = ZipArchive::new(Cursor::new(bytes))
        .map_err(|error| DocumentExtractionError::Corrupt(format!("ooxml archive: {error}")))?;
    let mut entry = archive
        .by_name(name)
        .map_err(|_| DocumentExtractionError::Corrupt(format!("ooxml part is missing: {name}")))?;
    if entry.size() > MAX_OOXML_ENTRY_BYTES {
        return Err(DocumentExtractionError::Corrupt(format!(
            "ooxml part {name} exceeds the decompressed entry limit"
        )));
    }
    let mut xml = String::with_capacity(entry.size().min(MAX_OOXML_ENTRY_BYTES) as usize);
    entry
        .read_to_string(&mut xml)
        .map_err(|error| DocumentExtractionError::Corrupt(format!("ooxml part {name}: {error}")))?;
    Ok(xml)
}

/// Extracts visible text from WordprocessingML/PresentationML.
///
/// Text runs (`w:t`/`a:t`) contribute their content, paragraphs end with a
/// newline, tabs and explicit breaks are preserved, and every other element is
/// ignored.
fn extract_ooxml_text(xml: &str) -> Result<String, DocumentExtractionError> {
    use quick_xml::events::Event;

    let mut reader = quick_xml::Reader::from_str(xml);
    reader.config_mut().trim_text(false);
    let mut output = String::new();
    let mut in_text = false;
    loop {
        match reader.read_event() {
            Ok(Event::Start(element)) => match element.local_name().as_ref() {
                b"t" => in_text = true,
                b"tab" => output.push('\t'),
                b"br" => output.push('\n'),
                _ => {}
            },
            Ok(Event::Empty(element)) => match element.local_name().as_ref() {
                b"tab" => output.push('\t'),
                b"br" => output.push('\n'),
                _ => {}
            },
            Ok(Event::Text(text)) => {
                if in_text {
                    let decoded = text.unescape().map_err(|error| {
                        DocumentExtractionError::Corrupt(format!("ooxml text: {error}"))
                    })?;
                    output.push_str(&decoded);
                }
            }
            Ok(Event::End(element)) => match element.local_name().as_ref() {
                b"t" => in_text = false,
                b"p" => output.push('\n'),
                _ => {}
            },
            Ok(Event::Eof) => break,
            Err(error) => {
                return Err(DocumentExtractionError::Corrupt(format!(
                    "ooxml xml: {error}"
                )))
            }
            _ => {}
        }
    }
    Ok(output)
}

fn normalize_text(text: &str) -> String {
    let mut normalized = String::with_capacity(text.len());
    let mut blank_run = 0usize;
    for line in text.replace('\r', "\n").split('\n') {
        let line = line.trim_end();
        if line.trim().is_empty() {
            blank_run += 1;
            if blank_run > 1 {
                continue;
            }
            normalized.push('\n');
            continue;
        }
        blank_run = 0;
        normalized.push_str(line);
        normalized.push('\n');
    }
    normalized.trim().to_string()
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;
    use zip::write::SimpleFileOptions;

    fn request(content_type: &str, name: &str, bytes: Vec<u8>) -> DocumentExtractionRequest {
        DocumentExtractionRequest {
            content_type: content_type.to_string(),
            file_name: Some(name.to_string()),
            bytes,
        }
    }

    fn zip_with(entries: &[(&str, &str)]) -> Vec<u8> {
        let mut buffer = Cursor::new(Vec::new());
        {
            let mut writer = zip::ZipWriter::new(&mut buffer);
            let options =
                SimpleFileOptions::default().compression_method(zip::CompressionMethod::Deflated);
            for (name, contents) in entries {
                writer.start_file(*name, options).expect("start zip entry");
                writer
                    .write_all(contents.as_bytes())
                    .expect("write zip entry");
            }
            writer.finish().expect("finish zip");
        }
        buffer.into_inner()
    }

    #[test]
    fn supports_pdf_docx_pptx_xlsx_by_mime_and_extension() {
        let extractor = DefaultDocumentTextExtractor::new();
        assert!(extractor.supports("application/pdf", None));
        assert!(extractor.supports("", Some("report.PDF")));
        assert!(extractor.supports(
            "application/vnd.openxmlformats-officedocument.wordprocessingml.document",
            None
        ));
        assert!(extractor.supports(
            "application/vnd.openxmlformats-officedocument.presentationml.presentation",
            Some("deck.pptx")
        ));
        assert!(extractor.supports(
            "application/vnd.openxmlformats-officedocument.spreadsheetml.sheet",
            Some("budget.xlsx")
        ));
        assert!(!extractor.supports("text/markdown", Some("notes.md")));
        assert!(!extractor.supports("application/msword", Some("legacy.doc")));
    }

    #[test]
    fn docx_extracts_paragraph_text_and_line_breaks() {
        let document_xml = r#"<?xml version="1.0" encoding="UTF-8"?>
<w:document xmlns:w="http://schemas.openxmlformats.org/wordprocessingml/2006/main">
  <w:body>
    <w:p><w:r><w:t>Refund policy</w:t></w:r></w:p>
    <w:p><w:r><w:t>Refunds are accepted within </w:t></w:r><w:r><w:t>30 days.</w:t></w:r></w:p>
  </w:body>
</w:document>"#;
        let bytes = zip_with(&[("word/document.xml", document_xml)]);
        let extracted = DefaultDocumentTextExtractor::new()
            .extract(request(
                "application/vnd.openxmlformats-officedocument.wordprocessingml.document",
                "policy.docx",
                bytes,
            ))
            .expect("docx extraction");
        assert_eq!(extracted.format, "docx");
        assert!(extracted.text.contains("Refund policy"));
        assert!(extracted
            .text
            .contains("Refunds are accepted within 30 days."));
    }

    #[test]
    fn pptx_extracts_each_slide_in_order() {
        let slide_one = r#"<p:sld xmlns:a="http://schemas.openxmlformats.org/drawingml/2006/main"><a:p><a:r><a:t>Welcome</a:t></a:r></a:p></p:sld>"#;
        let slide_two = r#"<p:sld xmlns:a="http://schemas.openxmlformats.org/drawingml/2006/main"><a:p><a:r><a:t>Roadmap</a:t></a:r></a:p></p:sld>"#;
        let bytes = zip_with(&[
            ("ppt/slides/slide1.xml", slide_one),
            ("ppt/slides/slide2.xml", slide_two),
        ]);
        let extracted = DefaultDocumentTextExtractor::new()
            .extract(request(
                "application/vnd.openxmlformats-officedocument.presentationml.presentation",
                "deck.pptx",
                bytes,
            ))
            .expect("pptx extraction");
        assert_eq!(extracted.pages, Some(2));
        let welcome = extracted.text.find("Welcome").expect("slide one");
        let roadmap = extracted.text.find("Roadmap").expect("slide two");
        assert!(welcome < roadmap);
    }

    #[test]
    fn xlsx_extracts_shared_and_inline_cell_text_per_row() {
        let shared_strings = r#"<sst xmlns="http://schemas.openxmlformats.org/spreadsheetml/2006/main"><si><t>Product</t></si><si><t>Units</t></si></sst>"#;
        let worksheet = r#"<worksheet xmlns="http://schemas.openxmlformats.org/spreadsheetml/2006/main"><sheetData>
<row r="1"><c r="A1" t="s"><v>0</v></c><c r="B1" t="s"><v>1</v></c></row>
<row r="2"><c r="A2" t="inlineStr"><is><t>Widget</t></is></c><c r="B2"><v>42</v></c></row>
</sheetData></worksheet>"#;
        let bytes = zip_with(&[
            ("xl/sharedStrings.xml", shared_strings),
            ("xl/worksheets/sheet1.xml", worksheet),
        ]);
        let extracted = DefaultDocumentTextExtractor::new()
            .extract(request(
                "application/vnd.openxmlformats-officedocument.spreadsheetml.sheet",
                "budget.xlsx",
                bytes,
            ))
            .expect("xlsx extraction");
        assert_eq!(extracted.format, "xlsx");
        assert_eq!(extracted.pages, Some(1));
        assert_eq!(extracted.text, "Product\tUnits\nWidget\t42");
    }

    #[test]
    fn corrupt_archives_fail_with_a_typed_error() {
        let error = DefaultDocumentTextExtractor::new()
            .extract(request("application/pdf", "broken.pdf", vec![0, 1, 2, 3]))
            .expect_err("corrupt pdf must fail");
        assert!(matches!(error, DocumentExtractionError::Corrupt(_)));
    }

    #[test]
    fn oversize_input_is_rejected_before_parsing() {
        let bytes = vec![0u8; MAX_DOCUMENT_EXTRACTION_INPUT_BYTES as usize + 1];
        let error = DefaultDocumentTextExtractor::new()
            .extract(request("application/pdf", "huge.pdf", bytes))
            .expect_err("oversize input must fail");
        assert!(matches!(
            error,
            DocumentExtractionError::InputTooLarge { .. }
        ));
    }
}
