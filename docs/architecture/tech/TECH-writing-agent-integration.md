# Knowledgebase Writing Agent Integration

- Status: active
- Owner: `knowledgebase-platform`
- Consumers: Knowledgebase PC notes right panel and the WeChat writing editor
- Producer: `sdkwork-agents` official writing agent catalog

## 1. Scope

The Knowledgebase PC assistant panel can run one of the official writing agents
published by `sdkwork-agents`. The same panel renders in both note editing
surfaces:

- the notes workspace right sidebar (`sdkwork-knowledgebase-pc-knowledgebase`
  `AiAssistantPanel`, mounted by `EditorPanel`);
- the WeChat article writing editor (`WechatPublishPage`).

When the user has not selected an agent, the panel keeps the existing
Knowledgebase RAG chat path. Selecting an agent switches the panel to the
Agents session path described below; the choice is remembered per browser.

## 2. Backend Contract

The integration consumes the generated Agents app SDK
(`@sdkwork/agents-app-sdk`) and calls only the Agents app API. Knowledgebase
owns no agent orchestration and adds no frontend proxy route.

| Step | App API operation | Purpose |
| --- | --- | --- |
| List | `GET /app/v3/api/ai/agents?scope=market` | Read public agents and keep `managementProfile.categoryId === "writing"` |
| Reuse | `GET /app/v3/api/ai/agents/{agentId}/sessions/{sessionId}` | Continue the open note's live session |
| Create | `POST /app/v3/api/ai/agents/{agentId}/sessions` | Start a server-generated session with Knowledgebase source context |
| Turn | `POST /app/v3/api/ai/agents/{agentId}/sessions/{sessionId}/turns?stream=true&event_protocol=kernel-v1` | Stream the assistant answer, reasoning, and MCP tool events |

Session and turn requests carry the idempotency key and payload hash the app
API requires. The note title and body (bounded to 6000 characters) travel as the
turn `systemPrompt` so the agent answers against the open document.

One session belongs to one agent and one note. Switching the agent or the open
document resets the session and transcript so context never crosses documents.

## 3. Dependency Wiring

- `pnpm-workspace.yaml` registers the sibling Agents SDK workspace package through
  `sdkwork-specs/workspace/consumers/sdkwork-knowledgebase.json`.
- `sdkwork-knowledgebase-pc-core` owns the client factory
  (`src/sdk/agentsAppSdkClient.ts`), the availability registry
  (`src/api/knowledgebaseAgentsApiRegistry.ts`), and the turn-helper re-exports
  (`src/sdk/agentsAppSdkTurns.ts`). Feature packages import the SDK only through
  those core exports.
- `KnowledgebaseRuntimeConfig` exposes the dependency base URL from
  `VITE_SDKWORK_AGENTS_APP_API_BASE_URL`, falling back to the platform API
  gateway. The Agents app API is a dependency surface, so dev same-origin mode
  keeps the gateway origin instead of the application ingress
  (`APP_SDK_INTEGRATION_SPEC.md` section 2.4).
- The Knowledgebase standalone ingress mounts Knowledgebase routes, not Agents
  routes. A standalone deployment therefore sets
  `VITE_SDKWORK_AGENTS_APP_API_BASE_URL` (or the shared `SDKWORK_API_BASE_URL`
  edge) to a gateway that serves `/app/v3/api/ai/agents`; the checked-in
  `.env.example` points it at the development platform gateway on port 3900.
  Cloud deployments use the platform API gateway by default.
- The PC core component spec declares the `sdkwork-agents-app-sdk` dependency,
  the `agents` permission module inheritance, and the client inventory entry.

## 4. Writing Agents And MCP Capability

The `sdkwork-agents` catalog defines six `writing` agents and two writing MCP
catalog entries. See
[`sdkwork-agents/docs/guides/operator/agent-catalog.md`](../../../../sdkwork-agents/docs/guides/operator/agent-catalog.md).

| MCP server | Declared tools | Tool policy |
| --- | --- | --- |
| `mcp.sdkwork.writing-sources` | `lookup_style_guide`, `verify_fact`, `check_citation` | `read-only` |
| `mcp.sdkwork.knowledgebase-notes` | `search_notes`, `read_note`, `create_note`, `update_note`, `append_note` | `read-write` |

Agent streams emit kernel-v1 tool events
(`agent.stream.tool.call.start`, `agent.stream.tool.result`). The panel folds
them into per-call cards that show a running spinner, then success or failure
with the tool result. Connectors stay in catalog draft until an operator
publishes an endpoint and credentials; until then the declared tool names
describe intent and the runtime exposes no unchecked tool.

## 5. Failure Behavior

- Missing or unconfigured Agents SDK: the picker shows the typed
  `api.unavailable.agents` message with a retry action.
- Unparseable or server-rejected sessions: a fresh server-generated session is
  created on the next send.
- Terminal stream errors: the SDK problem is rendered through
  `resolveUserFacingErrorMessage`; partial streamed text is kept.
- Stop: the panel invalidates the generation token so late deltas cannot append
  to the transcript.

## 6. Verification

```powershell
cd sdkwork-knowledgebase/apps/sdkwork-knowledgebase-pc
pnpm lint
cd packages/sdkwork-knowledgebase-pc-core && pnpm test
cd ../sdkwork-knowledgebase-pc-knowledgebase && pnpm test
```

The feature test suite covers catalog filtering, session reuse and creation,
turn payload construction, MCP tool-event reduction, and streamed completion.
From the repository root, `pnpm check:app-composition`, `pnpm check:client-env`,
and `pnpm check:pc-app-hygiene` cover workspace registration, runtime env
materialization, and PC package hygiene.
