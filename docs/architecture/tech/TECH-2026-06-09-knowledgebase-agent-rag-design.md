> Owner: SDKWork maintainers

- Date: 2026-06-09
- Owner: sdkwork-knowledgebase
- Domain: intelligence
- Capability: knowledgebase
- Status: implementation source design

## Goal

SDKWork Knowledgebase is the product authority for knowledge spaces, documents, chunks, indexes, retrieval, citations, retrieval traces, and knowledge-backed agent profiles. Agent runtimes consume it through standard SDK/API contracts and through a thin `sdkwork-agent-kernel::KnowledgeProvider` adapter.

This design intentionally does not preserve pre-launch compatibility. The system is not online, so new public contracts, database objects, and SPI boundaries should use the standard shape directly.

## Boundaries

`sdkwork-agent-kernel` owns provider-neutral agent SPIs only. It defines `KnowledgeProvider`, `ModelProvider`, context frames, policy, protocol, and runtime diagnostics. It must not depend on `sdkwork-knowledgebase`, vector databases, embedding providers, or Rig-specific implementation objects.

`sdkwork-knowledgebase` owns knowledge product behavior. It stores metadata and indexes for spaces, collections, sources, Drive object references, documents, document versions, chunks, embeddings, retrieval profiles, retrieval traces, retrieval hits, and knowledge-agent profiles. Large source files and generated artifacts remain Drive-owned.

Agent business/runtime code owns chat orchestration. It resolves the model provider, memory, tool policy, knowledge bindings, context assembly, non-SSE responses, SSE events, and RPC chat service behavior.

## Knowledge SPI

The kernel SPI remains retrieval-provider-neutral:

- request: query, tenant, namespace, topK, retrieval methods, filters, trace context, timeout, metadata.
- response: result id, document kind, title, snippet, score, method, source URI, trust, redaction, metadata.
- document read/list: stable document content and provenance for context assembly.

`sdkwork-knowledgebase-agent-provider` maps SDKWork Knowledgebase retrieval responses into this SPI. It exposes provider id `provider.knowledge.sdkwork-knowledgebase` and capabilities `knowledge.search`, `knowledge.read`, and `knowledge.list`.

## RAG Product Model

The product model is:

```text
space -> collection -> source -> document -> document_version -> chunk
                                                         |
                                                        index -> embedding

retrieval_profile -> retrieval_trace -> retrieval_hit -> citation/provenance

agent_profile -> agent_knowledge_binding -> space/collection filters
agent_profile -> model_provider_id/model_id/model_parameters
```

Retrieval is not model generation. Knowledgebase returns retrieval results and context packs. Agent runtime decides how to combine system instructions, memory, tools, retrieved context, and model invocation.

## Knowledge-Agent Profile

`KnowledgeAgentProfile` is a reusable definition for knowledge-backed agents:

- `profileId`, `tenantId`, `name`, `description`, `status`.
- `systemInstruction`.
- `modelProviderId`, `modelId`, `modelParameters`.
- `retrievalProfileId`.
- `citationPolicy`.
- `memoryPolicyRef`, `toolPolicyRef`, `answerPolicy`.

`KnowledgeAgentBinding` links one profile to one space and optional collection filter. A profile may have many bindings, enabling one agent to query one or more knowledge bases with priority, `topK`, minimum score, source filters, and document filters.

## Retrieval Flow

1. App or Agent runtime calls `retrievals.create` or the kernel `KnowledgeProvider`.
2. Knowledgebase validates tenant, actor, space bindings, retrieval profile, and ACL.
3. Query normalization, optional rewrite, hybrid retrieval, and optional rerank produce candidate chunks.
4. Results are filtered by tenant, space, collection, document status, chunk status, and permissions.
5. `kb_retrieval_trace` and `kb_retrieval_hit` preserve diagnostics without storing raw secrets.
6. Context pack assembly returns bounded snippets with citations and token estimates.
7. Agent runtime invokes `ModelProvider` and returns citations plus trace ids in chat output.

## HTTP API

App API adds:

- `POST /app/v3/api/knowledge/retrievals` -> `retrievals.create`
- `GET /app/v3/api/knowledge/retrievals/{retrievalId}` -> `retrievals.retrieve`
- `POST /app/v3/api/knowledge/context_packs` -> `contextPacks.create`
- `POST /app/v3/api/knowledge/agent_profiles` -> `agentProfiles.create`
- `GET /app/v3/api/knowledge/agent_profiles/{profileId}` -> `agentProfiles.retrieve`
- `PATCH /app/v3/api/knowledge/agent_profiles/{profileId}` -> `agentProfiles.update`
- `DELETE /app/v3/api/knowledge/agent_profiles/{profileId}` -> `agentProfiles.delete`
- `GET /app/v3/api/knowledge/agent_profiles/{profileId}/bindings` -> `agentProfiles.bindings.list`
- `POST /app/v3/api/knowledge/agent_profiles/{profileId}/bindings` -> `agentProfiles.bindings.create`
- `PATCH /app/v3/api/knowledge/agent_profiles/{profileId}/bindings/{bindingId}` -> `agentProfiles.bindings.update`
- `DELETE /app/v3/api/knowledge/agent_profiles/{profileId}/bindings/{bindingId}` -> `agentProfiles.bindings.delete`
- `POST /app/v3/api/knowledge/agent_profiles/{profileId}/retrieval_preview` -> `agentProfiles.retrievalPreview.create`

Backend API adds:

- `POST /backend/v3/api/knowledge/indexes` -> `indexes.create`
- `GET /backend/v3/api/knowledge/indexes/{indexId}` -> `indexes.retrieve`
- `POST /backend/v3/api/knowledge/indexes/{indexId}/rebuild` -> `indexes.rebuild`
- `POST /backend/v3/api/knowledge/retrieval_profiles` -> `retrievalProfiles.create`
- `GET /backend/v3/api/knowledge/retrieval_profiles/{profileId}` -> `retrievalProfiles.retrieve`
- `PATCH /backend/v3/api/knowledge/retrieval_profiles/{profileId}` -> `retrievalProfiles.update`
- `GET /backend/v3/api/knowledge/retrieval_traces` -> `retrievalTraces.list`
- `GET /backend/v3/api/knowledge/retrieval_traces/{traceId}` -> `retrievalTraces.retrieve`
- `GET /backend/v3/api/knowledge/provider_health` -> `providerHealth.list`

The OpenAPI authority remains owner-only:

- `sdkwork-knowledgebase.app`
- `sdkwork-knowledgebase.backend`

SDK families remain:

- `sdkwork-knowledgebase-app-sdk`
- `sdkwork-knowledgebase-backend-sdk`

Generated SDK output must be regenerated from OpenAPI and must not be hand-edited.

## Database

New database objects use the `kb_` prefix and runtime-generated Snowflake ids. Required new tables:

- `kb_chunk`
- `kb_index`
- `kb_embedding`
- `kb_retrieval_profile`
- `kb_retrieval_trace`
- `kb_retrieval_hit`
- `kb_agent_profile`
- `kb_agent_knowledge_binding`

The database stores retrieval metadata, indexes, and traceability. It does not store provider credentials, presigned URLs, raw auth tokens, or Drive-owned file content.

## Chat, SSE, And RPC

Knowledgebase does not own full chat sessions. Agent runtime exposes:

- non-SSE complete chat responses with answer, citations, trace ids, tool calls, and usage.
- SSE events for retrieval start/completion, context assembly, model deltas, tool calls, citations, completion, and errors.
- RPC service methods for `AgentChatService.complete`, `AgentChatService.stream`, `AgentChatService.retrieveKnowledge`, and trace inspection.

RPC and HTTP operation semantics must match. RPC is not a bypass for missing app/backend API behavior.

## Verification

Required checks:

- contract tests for DTOs and operation ids.
- app/backend route tests proving every OpenAPI path is mounted.
- migration tests proving all required `kb_` objects and indexes exist.
- `cargo fmt --all --check`.
- `cargo test --workspace`.
- `powershell -ExecutionPolicy Bypass -File tools/verify_phase1.ps1`.
