# Component Specs

This directory is the local SDKWork component contract for `sdkwork-knowledgebase-document-extraction`.

- Component root: `sdkwork-knowledgebase-document-extraction`
- Canonical standards: `../../../sdkwork-specs/README.md`
- Machine-readable contract: `specs/component.spec.json`

The crate implements the ingestion `DocumentTextExtractor` SPI for PDF, DOCX, PPTX, and XLSX with
bounded input, decompressed-entry, page, row, character, and deadline limits. Parser dependencies
stay in this adapter so the service layer remains parser-free.

Do not copy root standards into this directory. Link to files under `../../../sdkwork-specs/` instead.
