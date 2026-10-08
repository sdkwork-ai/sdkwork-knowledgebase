# SDKWork Knowledgebase API Changelog

## App API (`/app/v3/api`)

### 0.1.0 (Phase 0 MVP)

Initial API surface for the SDKWork Knowledgebase App API.

**Spaces**
- `POST /app/v3/api/knowledge/spaces` — `spaces.create`
- `GET /app/v3/api/knowledge/spaces/{spaceId}` — `spaces.retrieve`
- `PATCH /app/v3/api/knowledge/spaces/{spaceId}` — `spaces.update`
- `DELETE /app/v3/api/knowledge/spaces/{spaceId}` — `spaces.delete`

**Space Members**
- `GET /app/v3/api/knowledge/spaces/{spaceId}/members` — `spaces.members.list`
- `POST /app/v3/api/knowledge/spaces/{spaceId}/members` — `spaces.members.members`
- `DELETE /app/v3/api/knowledge/spaces/{spaceId}/members` — `spaces.members.delete`

**Documents**
- `GET /app/v3/api/knowledge/documents?space_id={space_id}` — `documents.list`
- `POST /app/v3/api/knowledge/documents` — `documents.create`
- `GET /app/v3/api/knowledge/documents/{documentId}` — `documents.retrieve`
- `PATCH /app/v3/api/knowledge/documents/{documentId}` — `documents.update`
- `DELETE /app/v3/api/knowledge/documents/{documentId}` — `documents.delete`
- `GET /app/v3/api/knowledge/documents/{documentId}/content` — `documents.content.list`
- `GET /app/v3/api/knowledge/documents/{documentId}/versions` — `documents.versions.list`
- `POST /app/v3/api/knowledge/documents/{documentId}/versions` — `documents.versions.versions`

**Ingestion**
- `POST /app/v3/api/knowledge/ingests` — `ingests.create`
- `GET /app/v3/api/knowledge/ingests/{ingestId}` — `ingests.retrieve`
- `POST /app/v3/api/knowledge/drive_imports` — `driveImports.create`
- `POST /app/v3/api/knowledge/git_imports` — `gitImports.create`
- `POST /app/v3/api/knowledge/git_syncs` — `gitSyncs.create`

**OKF (Open Knowledge Format)**
- `GET /app/v3/api/knowledge/okf/concepts?space_id={space_id}` — `okf.concepts.list`
- `PUT /app/v3/api/knowledge/okf/concepts/upsert` — `okf.concepts.update`
- `GET /app/v3/api/knowledge/okf/concepts/{conceptId}` — `okf.concepts.retrieve`
- `DELETE /app/v3/api/knowledge/okf/concepts/{conceptId}` — `okf.concepts.delete`
- `GET /app/v3/api/knowledge/okf/concepts/{conceptId}/revisions` — `okf.concepts.revisions.list`
- `GET /app/v3/api/knowledge/okf/index` — `okf.bundle.index.list`
- `GET /app/v3/api/knowledge/okf/log` — `okf.bundle.log.list`
- `GET /app/v3/api/knowledge/okf/profile` — `okf.bundle.profile.list`
- `POST /app/v3/api/knowledge/okf/queries` — `okf.queries.create`
- `POST /app/v3/api/knowledge/okf/queries/{queryId}/file_answer` — `okf.queries.fileAnswer`
- `POST /app/v3/api/knowledge/okf/context_packs` — `okf.contextPacks.create`
- `POST /app/v3/api/knowledge/okf/exports` — `okf.bundle.export.create`
- `GET /app/v3/api/knowledge/okf/exports/{exportId}` — `okf.bundle.export.retrieve`
- `POST /app/v3/api/knowledge/okf/imports` — `okf.bundle.import.create`
- `POST /app/v3/api/knowledge/okf/lint_runs` — `okf.lintRuns.create`

**Browser / Navigation**
Browser list response data is `KnowledgeBrowserListData`: standard `items` + `pageInfo`, plus `spaceId`, `driveSpaceId`, resolved `parentId`, `view`, and `pageSize`.
For OKF spaces, `view=files` resolves to `sources/raw` and lists original raw source files; it must not expose the generated `okf/` bundle tree.
For OKF spaces, `view=okf_bundle` resolves to `okf`, and `view=outputs` resolves to `output`.
Clients creating root folders or uploading root files use response `data.parentId` as the Drive parent node id instead of Drive root or a hard-coded logical path.

- `GET /app/v3/api/knowledge/spaces/{spaceId}/browser?view=files` — `spaces.browser.list`

**Retrieval & RAG**
- `POST /app/v3/api/knowledge/retrievals` — `retrievals.create`
- `GET /app/v3/api/knowledge/retrievals/{retrievalId}` — `retrievals.retrieve`
- `POST /app/v3/api/knowledge/context_packs` — `contextPacks.create`

**Agent Profiles**
- `POST /app/v3/api/knowledge/agent_profiles` — `agentProfiles.create`
- `GET /app/v3/api/knowledge/agent_profiles/{profileId}` — `agentProfiles.retrieve`
- `PATCH /app/v3/api/knowledge/agent_profiles/{profileId}` — `agentProfiles.update`
- `DELETE /app/v3/api/knowledge/agent_profiles/{profileId}` — `agentProfiles.delete`
- `GET /app/v3/api/knowledge/agent_profiles/{profileId}/bindings` — `agentProfiles.bindings.list`
- `POST /app/v3/api/knowledge/agent_profiles/{profileId}/bindings` — `agentProfiles.bindings.bindings`
- `PATCH /app/v3/api/knowledge/agent_profiles/{profileId}/bindings/{bindingId}` — `agentProfiles.bindings.update`
- `DELETE /app/v3/api/knowledge/agent_profiles/{profileId}/bindings/{bindingId}` — `agentProfiles.bindings.delete`
- `POST /app/v3/api/knowledge/agent_profiles/{profileId}/retrieval_preview` — `agentProfiles.retrievalPreview.retrievalPreview`
- `POST /app/v3/api/knowledge/agent_profiles/{profileId}/chat` — `agentProfiles.chat.chat`

**Context Bindings**
- `GET /app/v3/api/knowledge/spaces/{spaceId}/context_bindings` — `spaces.contextBindings.list`
- `POST /app/v3/api/knowledge/spaces/{spaceId}/context_bindings` — `spaces.contextBindings.contextBindings`
- `GET /app/v3/api/knowledge/context_bindings/{bindingId}` — `contextBindings.retrieve`
- `PATCH /app/v3/api/knowledge/context_bindings/{bindingId}` — `contextBindings.update`
- `DELETE /app/v3/api/knowledge/context_bindings/{bindingId}` — `contextBindings.delete`

**Upload Sessions**
- `POST /app/v3/api/knowledge/upload_sessions` — `uploadSessions.create`
- `POST /app/v3/api/knowledge/upload_sessions/{sessionId}/complete` — `uploadSessions.complete`

**WeChat Integration**
- `GET /app/v3/api/knowledge/wechat/official_accounts` — `wechat.officialAccounts.list`
- `PUT /app/v3/api/knowledge/wechat/official_accounts` — `wechat.officialAccounts.update`
- `GET /app/v3/api/knowledge/wechat/applets` — `wechat.applets.list`
- `PUT /app/v3/api/knowledge/wechat/applets` — `wechat.applets.update`
- `POST /app/v3/api/knowledge/wechat/articles/publish` — `wechat.articles.publish`
- `POST /app/v3/api/knowledge/wechat/articles/preview` — `wechat.articles.preview`

**Market / Commerce**
- `GET /app/v3/api/knowledge/market/listings` — `market.listings.list`
- `POST /app/v3/api/knowledge/market/subscriptions` — `market.subscriptions.create`
- `DELETE /app/v3/api/knowledge/market/subscriptions/{listingId}` — `market.subscriptions.delete`

**Media Tasks**
- `POST /app/v3/api/knowledge/media_tasks` — `mediaTasks.create`

## Backend API (`/backend/v3/api`)

### 0.1.0 (Phase 0 MVP)

Initial backend admin API surface.

### 0.2.0 (Phase 3 — Tenant Status)

Tenant status endpoint for multi-tenant architecture.

**Tenant Status**
- `GET /backend/v3/api/knowledge/tenants/current` — `tenants.current.list`
  - Retrieves the caller's own tenant knowledgebase status.
  - **Security**: Tenant identity is derived from the authenticated principal's
    access token claims (`WebRequestPrincipal.tenant_id()`); the handler fails
    closed with `tenant_id_mismatch` when the principal tenant differs from the
    runtime tenant. No `tenant_id` is accepted in the request body or path
    parameter.
  - Response (SDKWork envelope): `data.item` shaped
    `{ "tenantName": string?, "status": "ACTIVE"|"SUSPENDED"|"ARCHIVED", "spaceCount": "<int64 as string>", "documentCount": "<int64 as string>", "createdAt": string? }`

**Note**: Tenant creation and management is handled by the IAM layer.
Knowledgebase only reports tenant-level statistics derived from the authenticated
principal's token claims.

### Pre-release contract alignment (2026-10)

Corrections applied before first launch; no consumer migration is required.

- **Query parameters are lower_snake_case** (API_SPEC §13): `space_id`,
  `parent_id`, `subject_type`, `subject_id` replace the earlier camelCase
  spellings on `documents.list`, `okf.concepts.list`,
  `spaces/{spaceId}/browser` (`parent_id`), `spaces.members.delete`, and the
  backend `okf.candidates.list`.
- **Int64 wire contract** (API_SPEC §13.6): every snowflake/BIGINT id and byte
  size is serialized as a canonical decimal string (`x-sdkwork-int64-string`),
  in all three surfaces, for both responses and request bodies.
- **Command envelope** (API_SPEC §15.4): backend lifecycle commands
  (`okf.candidates.approve/reject`, `okf.concepts.publish`,
  `okf.index.rebuild`, `indexes.rebuild`) return `data.accepted`
  (`SdkWorkCommandData`) instead of `data.item`.
- **Small fixed lists** (`wechat` official accounts/applets/fan tags,
  `agents.bindings.list`) return the list envelope (`data.items` +
  `data.pageInfo`, `hasMore: false`) instead of bare arrays.
- **Removed the dead `collection` concept**: `collectionId` no longer appears
  on any document, chunk, index, or retrieval binding schema; the
  `kb_collection` table and its ghost columns were dropped from the baseline
  schema before first launch.

## Open API (`/knowledge/v3/api`)

### 0.1.0 (Phase 0 MVP)

Initial public Open API surface.
