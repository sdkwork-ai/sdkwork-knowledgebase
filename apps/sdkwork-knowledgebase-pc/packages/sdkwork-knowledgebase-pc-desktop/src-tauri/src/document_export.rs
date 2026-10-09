use pulldown_cmark::{CodeBlockKind, Event, Options, Parser, Tag, TagEnd};

pub fn markdown_to_typst(markdown: &str) -> String {
    let mut output = String::new();
    let parser = Parser::new_ext(markdown, Options::all());
    let mut ordered_list_index = 1usize;
    let mut in_ordered_list = false;

    for event in parser {
        match event {
            Event::Start(Tag::Heading { level, .. }) => {
                let level = level as usize;
                for _ in 0..level {
                    output.push('=');
                }
                output.push(' ');
            }
            Event::End(TagEnd::Heading(_)) => output.push_str("\n\n"),
            Event::Start(Tag::Paragraph) => {}
            Event::End(TagEnd::Paragraph) => output.push_str("\n\n"),
            Event::Start(Tag::BlockQuote(_)) => output.push_str("#quote[\n"),
            Event::End(TagEnd::BlockQuote(_)) => output.push_str("\n]\n\n"),
            Event::Start(Tag::List(Some(_))) => {
                in_ordered_list = true;
                ordered_list_index = 1;
            }
            Event::Start(Tag::List(None)) => {
                in_ordered_list = false;
            }
            Event::End(TagEnd::List(_)) => output.push('\n'),
            Event::Start(Tag::Item) => {
                if in_ordered_list {
                    output.push_str(&format!("{ordered_list_index}. "));
                    ordered_list_index += 1;
                } else {
                    output.push_str("- ");
                }
            }
            Event::End(TagEnd::Item) => output.push('\n'),
            Event::Start(Tag::Emphasis) => output.push_str("#emph["),
            Event::End(TagEnd::Emphasis) => output.push(']'),
            Event::Start(Tag::Strong) => output.push_str("#strong["),
            Event::End(TagEnd::Strong) => output.push(']'),
            Event::Start(Tag::Link { dest_url, .. }) => {
                output.push_str("#link(\"");
                output.push_str(&escape_typst_string(dest_url.as_ref()));
                output.push_str("\")[");
            }
            Event::End(TagEnd::Link) => output.push(']'),
            Event::Start(Tag::CodeBlock(kind)) => match kind {
                CodeBlockKind::Fenced(lang) if !lang.is_empty() => {
                    output.push_str("#raw(block: true, lang: \"");
                    output.push_str(&escape_typst_string(lang.as_ref()));
                    output.push_str("\")[");
                }
                _ => output.push_str("#raw(block: true)["),
            },
            Event::End(TagEnd::CodeBlock) => output.push_str("]\n\n"),
            Event::Code(text) => {
                // Raw literal content is verbatim, so backslash-escaping does
                // not protect the single-backtick delimiter. Use the
                // double-backtick form (tolerates embedded single backticks);
                // embedded double backticks become backtick+space (lossy but
                // bounded) instead of truncating the raw span.
                let code = text.replace("``", "` `");
                output.push_str("#raw(``");
                output.push_str(&code);
                output.push_str("``)");
            }
            Event::Text(text) => output.push_str(&escape_typst_text(text.as_ref())),
            Event::SoftBreak | Event::HardBreak => output.push('\n'),
            Event::Rule => output.push_str("#line(length: 100%)\n\n"),
            Event::Html(_) | Event::InlineHtml(_) => {}
            Event::FootnoteReference(_) | Event::TaskListMarker(_) => {}
            Event::Start(Tag::Strikethrough) => output.push_str("#strike["),
            Event::End(TagEnd::Strikethrough) => output.push(']'),
            Event::Start(Tag::Table(_)) | Event::End(TagEnd::Table) => {}
            Event::Start(Tag::TableHead) | Event::End(TagEnd::TableHead) => {}
            Event::Start(Tag::TableRow) | Event::End(TagEnd::TableRow) => {}
            Event::Start(Tag::TableCell) => output.push('|'),
            Event::End(TagEnd::TableCell) => output.push(' '),
            Event::Start(Tag::Image { dest_url, .. }) => {
                // The export world has no file/network resolver, so image
                // references cannot be embedded; emit a visible placeholder
                // instead of failing the whole compile.
                output.push_str(&format!(
                    "#text(fill: gray)[[图片: {}]]\n\n",
                    escape_typst_text(dest_url.as_ref())
                ));
            }
            Event::InlineMath(text) => {
                // Keep formulas VISIBLE in the export instead of silently
                // dropping them: emit as Typst math content. The dialects
                // differ (LaTeX-ish vs Typst), so this is best-effort.
                output.push_str(&format!("${}${}", escape_typst_math(text.as_ref()), ""));
            }
            Event::DisplayMath(text) => {
                output.push_str(&format!("\n$ ${}$ $\n\n", escape_typst_math(text.as_ref())));
            }
            _ => {}
        }
    }

    output.trim().to_string()
}

pub fn build_typst_document(_title: &str, body: &str) -> String {
    format!(
        r#"#set page(paper: "a4", margin: (x: 2cm, y: 2.2cm))
#set text(
  font: ("Segoe UI", "SimSun", "Noto Sans CJK SC", "Libertinus Serif"),
  size: 11pt,
  lang: "zh",
)
#set par(justify: true, leading: 0.65em, spacing: 0.65em)
#set heading(numbering: none)

{body}
"#
    )
}

pub fn compile_typst_to_pdf(source: &str) -> Result<Vec<u8>, String> {
    let engine = typst_as_lib::TypstEngine::builder()
        .main_file(source)
        .build();
    engine
        .with_world(|world| {
            let warned = typst::compile(world);
            let document = warned
                .output
                .map_err(|diagnostics| format!("typst compile failed: {diagnostics:?}"))?;
            typst_pdf::pdf(&document, &typst_pdf::PdfOptions::default())
                .map_err(|diagnostics| format!("typst pdf export failed: {diagnostics:?}"))
        })
        .map_err(|error| format!("typst export failed: {error}"))?
}

pub fn export_markdown_to_pdf(title: &str, markdown: &str) -> Result<Vec<u8>, String> {
    let trimmed = markdown.trim();
    if trimmed.is_empty() {
        return Err("markdown content is empty".to_string());
    }

    let body = markdown_to_typst(trimmed);
    let source = build_typst_document(&escape_typst_text(title), &body);
    compile_typst_to_pdf(&source)
}

fn escape_typst_text(input: &str) -> String {
    let mut escaped = String::with_capacity(input.len());
    for ch in input.chars() {
        match ch {
            '#' | '$' | '@' | '\\' | '*' | '_' | '`' | '<' | '>' | '~' | '[' | ']' => {
                escaped.push('\\');
                escaped.push(ch);
            }
            '/' => {
                // `//` starts a Typst line comment: escape the slash so plain
                // text URLs and code lines survive.
                escaped.push_str("\\/");
            }
            _ => escaped.push(ch),
        }
    }
    escaped
}

/// Math content goes into Typst math mode where the markup charset differs;
/// escape the characters that would break the math delimiters or start
/// markup, accepting that LaTeX-specific commands render as text.
fn escape_typst_math(input: &str) -> String {
    let mut escaped = String::with_capacity(input.len());
    for ch in input.chars() {
        match ch {
            '$' | '\\' | '#' | '@' => {
                escaped.push('\\');
                escaped.push(ch);
            }
            _ => escaped.push(ch),
        }
    }
    escaped
}

fn escape_typst_string(input: &str) -> String {
    input.replace('\\', "\\\\").replace('"', "\\\"")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn converts_heading_and_paragraph() {
        let typst = markdown_to_typst("# Title\n\nHello world.");
        assert!(typst.contains("= Title"));
        assert!(typst.contains("Hello world."));
    }

    #[test]
    fn exports_markdown_pdf_bytes() {
        let pdf = export_markdown_to_pdf(
            "Test Note",
            "# Hello\n\nThis is a **native** PDF export test.",
        )
        .expect("pdf export should succeed");
        assert!(pdf.starts_with(b"%PDF"));
    }
}
