# REQ-2026-1019 Agent RAG And Per-Agent Memory

- Status: implemented (knowledge path); memory provider wiring pending
- Owner: sdkwork-knowledgebase
- Updated: 2026-10-19
- Related:
  - [`../../architecture/tech/TECH-2026-06-09-knowledgebase-agent-rag-design.md`](../../architecture/tech/TECH-2026-06-09-knowledgebase-agent-rag-design.md)
  - [`../../../specs/knowledge-engine-spi.spec.json`](../../../specs/knowledge-engine-spi.spec.json)
  - [`../../../../sdkwork-agents/docs/architecture/tech/TECH-2026-1019-turn-knowledge-memory-grounding.md`](../../../../sdkwork-agents/docs/architecture/tech/TECH-2026-1019-turn-knowledge-memory-grounding.md)

## 1. Goal

After a user uploads files to a Knowledgebase space, a conversation executed by
an Agent must answer from those files (RAG), with citations, and each Agent must
be able to carry its own memory policy.

## 2. Product Contract

1. **Upload to retrieval.** A Drive upload that becomes a document version is
   chunked, indexed, and embedded by the ingestion pipeline; hybrid retrieval
   over the resulting index is the only knowledge source for answers.
   Markdown/text is read directly. PDF, DOCX, and PPTX are decoded by the
   bounded `DocumentTextExtractor` adapter
   (`crates/sdkwork-knowledgebase-document-extraction`) before chunking, so
   office documents and PDFs become answerable without a separate conversion
   step.
2. **Agent-owned conversation.** Chat is executed by an Agent, not by a bare
   retrieval endpoint. The Knowledgebase product's agent profiles and the
   `sdkwork-agents` composition slots both address the same knowledge capability.
3. **Binding.** An Agent associates one or more Knowledgebase spaces; each
   binding may set priority, `topK`, minimum score, and source/document filters.
   A RAG Agent works without an administrator-created retrieval profile: when
   none is set, the platform default hybrid plan applies.
4. **Mode resolution.** The PC assistant profile inherits the space mode. A
   space reports `okf_bundle` as its wire default, but only an OKF-initialized
   space (`okfBundleInitialized = true`) actually has a concept bundle to
   search; any other space resolves to RAG so uploaded files are answerable
   immediately after ingestion.
5. **Memory.** An Agent may set a `memory_policy_ref`. Chat injects bounded
   memory fragments ahead of the answer, labelled untrusted reference data.
   Missing or failing memory degrades to an ungrounded answer; it never fails
   chat.
6. **Citations.** Knowledge answers return the document/chunk citations produced
   by the retrieval trace.
7. **Conversation context.** A chat request may carry up to 20 prior
   user/assistant turns (8,000 characters each, 24,000 total). History shapes
   the conversational prompt; retrieval still runs against the current message,
   so an earlier question can never silently change today's evidence scope.

## 3. Cross-Module Boundary

`sdkwork-agents` consumes knowledge only through the public Knowledgebase App
API and generated transport SDK, forwarding the caller's dual tokens so the
Knowledgebase enforces actor and space ACLs. Agents never stores knowledge
content, chunks, indexes, or traces, and Knowledgebase never stores agent
session state or memory content.

To make that possible, the Knowledgebase App SDK now publishes a Rust transport
target (`sdkwork-knowledgebase-app-sdk-rust`) alongside TypeScript. The OpenAPI
authority also declares the previously missing `KnowledgeAgentBindingList`
schema referenced by `agentProfiles.bindings.list`.

## 4. Acceptance Evidence

```powershell
cargo test -p sdkwork-knowledgebase-agent-provider
cargo test -p sdkwork-intelligence-knowledgebase-service --lib agent_chat
cargo test -p sdkwork-intelligence-knowledgebase-service --test agent_service
cargo test -p sdkwork-knowledgebase-contract
cargo check -p sdkwork-knowledgebase-app-sdk-generated-rust # generated crate directory
node tools/knowledgebase_sdk_generate.mjs --check
node sdks/standardize-knowledgebase-sdk-family.mjs --check
node ../sdkwork-specs/tools/check-api-operation-patterns.mjs --workspace .
node ../sdkwork-specs/tools/check-api-response-envelope.mjs --workspace .
pnpm --dir apps/sdkwork-knowledgebase-pc/packages/sdkwork-knowledgebase-pc-knowledgebase test
pnpm --dir apps/sdkwork-knowledgebase-pc run lint
```

Agents-side evidence is recorded in the cross-linked Agents design document.

## 5. Remaining Gates

- Scanned/image-only PDFs need OCR; the current extractor fails with a typed
  "no extractable text" error. Legacy binary `.doc`/`.ppt`/`.xls` and `.xlsx`
  are not decoded yet (they receive the document upload profile but fail with
  an actionable error instead of indexing garbage).
- A production memory provider that implements
  `KnowledgeMemoryContextProvider` over the memory module's runtime. Today the
  Knowledgebase memory adapter exists but no production retriever is composed
  into the App API runtime, so a profile with `memory_policy_ref` logs a
  warning and answers without memory.
- Wire-level citation delivery in `sdkwork-agents` turn responses/SSE (API and
  SDK contract change).
- Live end-to-end evidence with a real Drive upload, worker ingestion, embedding
  provider, and model provider in a release environment.
