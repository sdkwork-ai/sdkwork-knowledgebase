> Owner: SDKWork maintainers

Date: 2026-06-19  
Status: **approved direction — clean-slate, no legacy compatibility**  
Owner: sdkwork-knowledgebase  
Normative format: `external/knowledge-catalog/okf/SPEC.md` (OKF v0.1)  
Local contract: `specs/okf-knowledge-bundle.spec.json`

---

## 1. 设计立场

应用 **尚未上线**。本设计以 **OKF v0.1** 为唯一知识表示标准，对现有 LLM Wiki 实现做 **完整替换**，不保留：

- `wiki/` Drive 目录树与 `current.md` / `revisions/` 页面模型
- `WikiPageType` 枚举与 slug 目录规则
- `wiki_schema.yaml`、`LlmWikiPaths`、`llm_wiki_initialized`
- `kb_wiki_*` 数据库表与 `wiki.*` API operationId
- `provider.knowledge.llm-wiki` 与 `wiki:{space}:{path}` 文档 ID
- 双写、deprecated 路由、迁移工具、兼容别名

**目标**：每个 Knowledge Space = 一个经 `sdkwork-drive` 持久化的 **OKF Knowledge Bundle**，外加 SDKWork 治理元数据（审核、修订、溯源、ACL）。导出包在 `okf_strict` 模式下必须是 **可直接被外部 OKF 消费者使用的标准 Bundle**。

### 1.1 零遗留原则（硬性）

实施完成的判定标准：**仓库内不存在任何 LLM Wiki 实现痕迹**。不允许 rename alias、deprecated 路由、注释掉的旧代码、`kb_wiki_*` 表、或「暂时保留」的 `wiki/` Drive 种子。

必须删除的文档与代码类别见 **§16 Legacy Purge Checklist**。CI 必须通过 `tools/check_okf_knowledge_bundle_standard.mjs`（实现期新增），对禁止符号与路径 **零容忍**。

---

## 2. 标准引用层次

```text
OKF v0.1 (external/knowledge-catalog/okf/SPEC.md)     ← 格式权威
        ↓ narrows
specs/okf-knowledge-bundle.spec.json                    ← SDKWork 绑定（路径、扩展、治理）
        ↓ implements
Rust services + SQL + OpenAPI + Agent provider
        ↓ stores bytes via
sdkwork-drive (cloud or local adapter)
```

---

## 3. 架构

### 3.1 三层内容模型

| 层 | Drive 路径 | 性质 | OKF 合规 |
|----|-----------|------|----------|
| **Sources** | `sources/raw/` | 不可变原始资料 | 非 Bundle 内容 |
| **OKF Bundle** | `okf/` | 持久化知识概念树 | **必须合规** |
| **Governance** | `.sdkwork/governance/` | 修订草稿、候选、审计快照 | **永不导出** |

OKF Bundle 与 Sources、Governance **严格分离**。Lint、导出、Agent 消费仅针对 `okf/` 子树（链接使用 bundle-relative 路径，不含 `okf/` 前缀）。

### 3.2 逻辑组件

```text
┌──────────────────────────────────────────────────────────┐
│ Agents: chat / enrichment / external OKF tooling        │
└─────────────────────────┬────────────────────────────────┘
                          │
┌─────────────────────────▼────────────────────────────────┐
│ OkfBundleService                                          │
│  ├─ OkfDocumentParser      (frontmatter + body)           │
│  ├─ OkfConformanceValidator (OKF v0.1 §9)                 │
│  ├─ OkfConceptService      (publish / upsert / read)      │
│  ├─ OkfIndexSynthesizer    (index.md)                     │
│  ├─ OkfLogSynthesizer      (log.md)                       │
│  ├─ OkfLinkIndexer         (→ kb_okf_concept_link)       │
│  ├─ OkfBundleLinter                                        │
│  ├─ OkfBundleImporter      (外部 Bundle → Drive)         │
│  ├─ OkfBundleExporter      (mirror / tarball)            │
│  └─ OkfBundleInitializer   (space bootstrap)             │
├──────────────────────────────────────────────────────────┤
│ OkfKnowledgeProvider       (agent kernel search/read)     │
└───────┬────────────────────────────┬─────────────────────┘
        │ SQL metadata               │ object bytes + workspace tree
┌───────▼────────────┐     ┌─────────▼──────────────────────┐
│ kb_okf_concept*    │     │ sdkwork-drive                   │
│ kb_drive_object_ref│     │  DriveObjectStore + Workspace   │
│ kb_source, …       │     └────────────────────────────────┘
└────────────────────┘
```

### 3.3 存储原则（不变）

1. 所有文件字节经 `sdkwork-drive`；Knowledgebase 不直连 S3/OSS/本地 FS。
2. SQL 仅存元数据与稳定 `kb_drive_object_ref`；不存 presigned URL。
3. 无原生 object version 时合成 `sha256:<digest>`；带 checksum 的写入必须校验。
4. Space 创建失败：补偿释放 Drive space + 软删除 `kb_space`。
5. 批处理 `IN (...)` ≤ 200。

---

## 4. Drive 目录标准

每个 Knowledge Space 的 logical path 根：

```text
{space_root}/
├── sources/
│   └── raw/                         # 不可变源文件
├── okf/                             # OKF Bundle 根（bundleRoot）
│   ├── index.md                     # 保留：目录索引
│   ├── log.md                       # 保留：变更日志
│   ├── schema/                      # SDKWork Agent 配置（非 OKF 概念）
│   │   ├── AGENTS.md
│   │   └── okf_profile.yaml
│   └── <domain>/                    # 领域目录，producer 自由组织
│       ├── index.md                 # 可选层级索引
│       └── <concept>.md             # OKF Concept
├── .sdkwork/
│   └── governance/
│       ├── revisions/{concept_id}/r{N}.md
│       ├── candidates/{candidate_id}.md
│       └── bundle_state.json
├── inbox/                           # 摄取入口（非 Bundle）
├── parsed/                          # 解析产物（非 Bundle）
├── mirror/                          # 导出产物
└── …
```

**禁止出现在 `okf/` 内的路径模式：**

- `**/current.md`
- `**/revisions/**`（修订只属于 governance）
- 任何不含 OKF frontmatter（`type` 非空）的非保留 `.md` 文件

**`okf/schema/`** 是 SDKWork 扩展目录；其中文件 **不是** OKF Concept（无强制 `type`），不参与 concept 枚举与合规计数。

---

## 5. OKF Concept 标准

### 5.1 文件格式

每个 Concept = UTF-8 Markdown，结构遵循 OKF §4：

```markdown
---
type: BigQuery Table
title: Users
description: One row per registered user.
resource: https://example.com/asset
tags: [users, schema]
timestamp: 2026-06-19T10:00:00Z
---

Body in standard Markdown…

See [posts](/tables/posts.md).

# Schema
| Column | Type | Description |
| …

# Citations
[1] [Official docs](https://…)
```

### 5.2 标识符

| 名称 | 规则 | 示例 |
|------|------|------|
| **Drive logical path** | `okf/` + 相对路径 | `okf/tables/users.md` |
| **Concept ID** | `okf/` 之后、去掉 `.md` | `tables/users` |
| **Bundle-relative path** | Concept ID + `.md`，用于链接 | `/tables/users.md` |
| **Agent document ID** | `okf:{space_id}:{concept_id}` | `okf:42:tables/users` |

文件名与目录名：`[a-z0-9][a-z0-9_-]*`（小写 ASCII）。

保留文件名（任意层级）：`index.md`、`log.md`。不得用于 Concept 正文文件。

### 5.3 SDKWork 治理扩展（可选 frontmatter）

```yaml
sdkwork:
  revisionNo: 7
  publishState: published
  sourceCount: 3
```

- 位于 frontmatter，OKF 消费者必须保留（§4.1 Extensions）。
- `okf_strict` 导出 **剥离** `sdkwork` 键。
- **禁止** 在 `sdkwork` 中存放 secrets、tokens、PII。

### 5.4 链接

| 类型 | 格式 | 要求 |
|------|------|------|
| 概念间（推荐） | `/tables/users.md` | OKF §5.1 |
| 概念间（允许） | `./users.md` | 同目录 |
| 外部 | `https://…` | Citations |
| 源文件 | 不得作为 concept 链接目标 | 使用 `# Citations` |

`OkfLinkIndexer` 在每次 `published` 投影后解析链接，写入 `kb_okf_concept_link`。断链仅产生 lint warning，不阻断读写（OKF §5.3）。

### 5.5 约定章节（非强制）

| Heading | 用途 |
|---------|------|
| `# Schema` | 字段/列结构 |
| `# Examples` | 示例 |
| `# Citations` | 外部来源 |

---

## 6. `okf_profile.yaml`

Bundle 级 Agent 配置，路径固定：`okf/schema/okf_profile.yaml`。

```yaml
okfVersion: "0.1"
schemaVersion: "1.0"
bundleRoot: "okf"
standardFiles:
  index: "index.md"              # 相对 bundleRoot
  log: "log.md"
  agentInstructions: "schema/AGENTS.md"
  profile: "schema/okf_profile.yaml"
layers:
  sources:
    immutable: true
    driveRoot: "sources/raw"
  bundle:
    driveRoot: "okf"
workflows:
  ingest:
    steps:
      - read_sources
      - upsert_concepts
      - rebuild_index
      - append_log
  query:
    readFirst: ["index.md"]
    mayFileAnswer: true
  lint:
    checks:
      - okf_conformance
      - broken_links
      - orphan_concepts
      - missing_citations
      - stale_claims
typeExamples:                    # 文档建议，非枚举
  - "BigQuery Table"
  - "API Endpoint"
  - "Metric"
  - "Playbook"
  - "Reference"
```

根 `okf/index.md` **可** 含 frontmatter `okf_version: "0.1"`（OKF §11）。

---

## 7. 数据库标准

### 7.1 命名

- 表前缀：`kb_`
- OKF 域表前缀：`kb_okf_`
- 索引：`idx_kb_okf_*`；唯一：`uk_kb_okf_*`
- **删除** 所有 `kb_wiki_*` 表定义与迁移

### 7.2 核心表

```sql
-- kb_space 字段
okf_bundle_initialized BOOLEAN NOT NULL DEFAULT FALSE

-- 概念主表
kb_okf_concept (
  id, space_id,
  concept_id VARCHAR NOT NULL,          -- canonical: tables/users
  type VARCHAR NOT NULL,                -- OKF frontmatter type
  title, description,
  logical_path VARCHAR NOT NULL,        -- okf/tables/users.md
  tags JSON,
  source_count INT,
  publish_state VARCHAR NOT NULL,
  current_revision_id BIGINT,
  created_at, updated_at,
  UNIQUE (space_id, concept_id)
)

-- 修订历史（字节在 Drive governance 路径）
kb_okf_concept_revision (
  id, concept_id, revision_no,
  markdown_object_ref_id BIGINT NOT NULL,
  content_hash VARCHAR NOT NULL,
  review_state VARCHAR NOT NULL,
  created_at
)

-- 概念链接图
kb_okf_concept_link (
  id, space_id,
  from_concept_id VARCHAR NOT NULL,
  to_concept_id VARCHAR NOT NULL,
  anchor_text VARCHAR,
  UNIQUE (space_id, from_concept_id, to_concept_id, anchor_text)
)

-- Bundle 标准文件登记
kb_okf_bundle_file (
  id, space_id,
  logical_path VARCHAR NOT NULL,
  file_kind VARCHAR NOT NULL,           -- bundle_index | bundle_log | profile | agents
  drive_object_ref_id BIGINT NOT NULL
)

-- 结构化日志（投影到 okf/log.md）
kb_okf_log_entry (
  id, space_id, occurred_at, event_type,
  title, actor, affected_concepts JSON,
  audit_event_id, warnings JSON
)

-- 候选（治理）
kb_okf_candidate (
  id, space_id, concept_id, candidate_type,
  state, markdown_object_ref_id, reviewer_id, …
)
```

**`concept_id` 是唯一业务主键**（per space）。不存在 `slug` 列。

### 7.3 发布状态机

```text
draft → candidate_ready → needs_review → published
                      ↘ rejected | stale | failed
```

- 仅 `published` 内容投影到 `okf/**/*.md`
- 非 published 内容只存在于 `.sdkwork/governance/`

---

## 8. 服务行为

### 8.1 Space 初始化

```text
create_space
  → create kb_space (okf_bundle_initialized=false)
  → provision sdkwork-drive space
  → write okf/schema/AGENTS.md, okf/schema/okf_profile.yaml
  → write okf/index.md (empty OKF index), okf/log.md (header only)
  → ensure drive workspace folder tree
  → register kb_okf_bundle_file rows
  → okf_bundle_initialized=true
  → on any failure: release drive + soft-delete kb_space
```

### 8.2 概念写入

```text
upsert_concept(space_id, concept_id, markdown, actor)
  → parse OkfDocument; validate conformance (type non-empty)
  → write governance revision .sdkwork/governance/revisions/{id}/r{N}.md
  → insert kb_okf_concept_revision
  → if auto_publish OR review approved:
       put okf/{concept_id}.md
       upsert kb_okf_concept
       rebuild okf/index.md
       append okf/log.md
       reindex kb_okf_concept_link
```

### 8.3 Index / Log 合成

**index.md**（OKF §6）：

- 无 frontmatter（或仅 `okf_version` 在根 index）
- 按目录分组，条目格式：`* [title](relative-path) - description`
- `description` 取自 concept frontmatter

**log.md**（OKF §7）：

- 无 frontmatter
- `## YYYY-MM-DD` 分组，新日期在上
- 条目：`* **Update**: …` / `* **Creation**: …`

### 8.4 Bundle Lint

| Check | 行为 |
|-------|------|
| `okf_conformance` | 每个非保留 `.md` 在 `okf/` 内可解析且 `type` 非空 |
| `broken_links` | warning |
| `orphan_concepts` | warning（无入链且非 index 列举的叶节点） |
| `missing_citations` | warning（有 factual claims 启发式检测） |
| `stale_claims` | warning（对比 `kb_source` lineage） |

### 8.5 外部 Bundle 导入

```text
import_okf_bundle(tarball | drive_import)
  → extract to temp
  → OkfConformanceValidator on all concepts
  → reject if any required violation (missing type)
  → write files to okf/ via drive
  → bulk upsert kb_okf_concept from frontmatter
  → rebuild index (或保留导入的 index 若合规)
```

---

## 9. API 标准

**仅 OKF 路由族**。不存在 `wiki.*` operationId。

### 9.1 App API

| operationId | Method | Path |
|-------------|--------|------|
| `okf.concepts.list` | GET | `/app/v3/api/knowledge/okf/concepts?spaceId={spaceId}` |
| `okf.concepts.retrieve` | GET | `/app/v3/api/knowledge/okf/concepts/{conceptId}` |
| `okf.concepts.update` | PUT | `/app/v3/api/knowledge/okf/concepts/upsert` |
| `okf.concepts.publish` | POST | `/backend/v3/api/knowledge/okf/concepts/{conceptId}/publish` |
| `okf.concepts.revisions.list` | GET | `/app/v3/api/knowledge/okf/concepts/{conceptId}/revisions` |
| `okf.bundle.index.list` | GET | `/app/v3/api/knowledge/okf/index?spaceId={spaceId}` |
| `okf.bundle.log.list` | GET | `/app/v3/api/knowledge/okf/log?spaceId={spaceId}` |
| `okf.bundle.profile.list` | GET | `/app/v3/api/knowledge/okf/profile?spaceId={spaceId}` |
| `okf.lintRuns.create` | POST | `/app/v3/api/knowledge/okf/lint_runs` |
| `okf.bundle.export.create` | POST | `/app/v3/api/knowledge/okf/exports` |
| `okf.bundle.import.create` | POST | `/app/v3/api/knowledge/okf/imports` |
| `okf.candidates.list` | GET | `…/okf/candidates` |
| `okf.candidates.approve` | POST | `…/okf/candidates/{candidateId}/approve` |
| `okf.candidates.reject` | POST | `…/okf/candidates/{candidateId}/reject` |
| `okf.compileJobs.create` | POST | `…/okf/compile-jobs` |
| `okf.qualityRuns.create` | POST | `…/okf/quality-runs` |

### 9.2 核心 DTO

```yaml
OkfConcept:
  conceptId: string
  type: string
  title: string
  description: string
  logicalPath: string
  bundleRelativePath: string      # tables/users.md
  tags: string[]
  sourceCount: integer
  publishState: OkfConceptPublishState
  updatedAt: date-time

OkfConceptUpsertRequest:
  markdown: string                # full document with frontmatter
  actor: string
  publish: boolean                # default false → candidate

OkfBundleLintResult:
  conformance: pass | fail
  issues: OkfLintIssue[]
```

### 9.3 Agent 集成

| 项 | 值 |
|----|-----|
| Provider ID | `provider.knowledge.okf` |
| Namespace | `space:{space_id}` |
| Document ID | `okf:{space_id}:{concept_id}` |
| Knowledge mode | `okf_bundle`（删除 `llm_wiki`） |

---

## 10. 导出与本地镜像

### 10.1 `okf_strict`（默认）

镜像根 **= OKF Bundle 内容**（`okf/` 的内部视图）：

```text
export-root/
├── index.md
├── log.md
├── schema/
│   ├── AGENTS.md
│   └── okf_profile.yaml
└── tables/
    └── users.md
```

不含：`sources/`、`.sdkwork/`、`inbox/`、`parsed/`。不含 frontmatter `sdkwork` 键。

### 10.2 `okf_with_sources`

在 `okf_strict` 基础上增加 `raw/`（`sources/raw` 副本）。

### 10.3 Manifest

`OkfBundleCompatibility`（取代 `LlmWikiCompatibility`）：

```yaml
okfVersion: "0.1"
bundleRoot: "."
standardFiles:
  index: "index.md"
  log: "log.md"
  agentInstructions: "schema/AGENTS.md"
  profile: "schema/okf_profile.yaml"
sourcesRoot: "raw"
```

---

## 11. Rust 模块标准

### 11.1 删除

| 路径/符号 | 动作 |
|-----------|------|
| `service/src/wiki/` | 删除，替换为 `service/src/okf/` |
| `contract/src/wiki.rs` | 删除，替换为 `contract/src/okf.rs` |
| `WikiPageType`, `LlmWikiPaths` | 删除 |
| `llm_wiki.rs` | 删除，替换为 `okf.rs` |
| `kb_wiki_*` stores | 删除，替换为 `kb_okf_*` |
| `wiki_*` tests | 删除，替换为 `okf_*` |

### 11.2 新增

```text
contract/src/okf.rs
  OkfBundlePaths, OkfConcept, OkfDocument, OkfConformanceError, …

service/src/okf/
  mod.rs, paths.rs, document.rs, validator.rs,
  concept_service.rs, index_synthesizer.rs, log_synthesizer.rs,
  link_indexer.rs, linter.rs, importer.rs, exporter.rs,
  initializer.rs, profile.rs

agent-provider/src/okf.rs
  OkfKnowledgeProvider, OKF_KNOWLEDGE_PROVIDER_ID

repository-sqlx/src/okf_concept_store.rs
repository-sqlx/migrations/* — 仅含 kb_okf_* 定义
```

### 11.3 AGENTS.md 模板

`okf/schema/AGENTS.md` 必须明确：

- Bundle 根为 `okf/`；Concept 使用 OKF frontmatter
- 链接使用 `/concept/path.md`
- ingest / query / lint 工作流引用 `okf_profile.yaml`
- 所有写入经 Knowledgebase API 或 Drive 抽象，禁止绕过 `sdkwork-drive`

---

## 12. 测试与合规

### 12.1 合规测试

- 加载 `external/knowledge-catalog/okf/bundles/stackoverflow`，经 importer 写入 fake drive，断言 `OkfConformanceValidator` 通过
- 每个 service 测试产出的 bundle 必须满足 OKF §9 三条硬性要求

### 12.2 验证命令

```bash
cargo test -p sdkwork-knowledgebase-contract okf
cargo test -p sdkwork-intelligence-knowledgebase-service okf
cargo test -p sdkwork-intelligence-knowledgebase-repository-sqlx okf
cargo test -p sdkwork-knowledgebase-agent-provider okf
pnpm test
pnpm verify
```

### 12.3 静态检查（实现期新增）

`tools/check_okf_knowledge_bundle_standard.mjs`：

- 禁止新增 `wiki_` / `llm_wiki` / `WikiPage` 符号
- OpenAPI 禁止 `wiki.` operationId 前缀
- 迁移 SQL 禁止 `kb_wiki_` 表名

---

## 13. 实施顺序

应用未上线，按 **删除 → 重建** 顺序执行，无迁移阶段。

| Step | 内容 |
|------|------|
| 1 | 更新 `specs/okf-knowledge-bundle.spec.json`；本设计定稿 |
| 2 | 替换 SQL 迁移：`kb_wiki_*` → `kb_okf_*`；`okf_bundle_initialized` |
| 3 | `contract/src/okf.rs` + 删除 `wiki.rs` |
| 4 | `service/src/okf/*` + 删除 `wiki/*` |
| 5 | Drive initializer 新目录树；删除 `wiki/pages/**` 种子 |
| 6 | OpenAPI：删除 `wiki.*`，新增 `okf.*`；重新生成 SDK |
| 7 | `OkfKnowledgeProvider`；删除 `LlmWikiKnowledgeProvider` |
| 8 | Router handlers / runtime 接线 |
| 9 | 测试 + `check_okf_knowledge_bundle_standard.mjs` |
| 10 | 更新 `docs/llm-wiki.md` SDKWork 章节 → 指向 OKF 绑定 |

---

## 14. 可观测性

| 指标 | 说明 |
|------|------|
| `kb_okf_concept_publish_total` | 发布 |
| `kb_okf_concept_upsert_total` | 写入 |
| `kb_okf_bundle_lint_issues` | lint 计数 |
| `kb_okf_conformance_failures` | 不合规 |
| `kb_okf_bundle_import_total` | 导入 |

审计：`okf.concept.published`、`okf.bundle.imported`、`okf.bundle.exported`、`okf.bundle.lint.completed`

---

## 15. 已锁定决策

| 决策 | 选择 |
|------|------|
| Bundle Drive 根 | `okf/` |
| 数据库表 | `kb_okf_*` only |
| 页面类型 | OKF `type` 字符串，无枚举 |
| API | `okf.*` only |
| 兼容 | **无** |
| 链接图 | `kb_okf_concept_link`，Phase 1 核心 |
| 外部 Bundle 导入 | Phase 1 核心 |
| 修订存储 | `.sdkwork/governance/revisions/` + SQL |
| 导出 | `okf_strict` 默认剥离 `sdkwork` frontmatter |
| LLM Wiki 遗留 | **全部删除**，见 §16 |

---

## 16. Legacy Purge Checklist（实施完成必达）

以下清单 **逐项清零** 后方可声称 OKF 标准实现完成。

### 16.1 文档（已完成）

| 动作 | 路径 | 状态 |
|------|------|------|
| DELETE | `docs/llm-wiki.md` | done |
| REPLACE | [TECH-okf-knowledge-bundle.md](TECH-okf-knowledge-bundle.md) | done |
| REVISE | [TECH-2026-06-01-knowledgebase-backend-design.md](TECH-2026-06-01-knowledgebase-backend-design.md) | done |
| REVISE | [TECH-2026-06-01-knowledgebase-backend-phase1-implementation.md](TECH-2026-06-01-knowledgebase-backend-phase1-implementation.md) | done |
| REVISE | `README.md` | done |

### 16.2 Rust 源码（删除文件）

```text
crates/sdkwork-knowledgebase-contract/src/wiki.rs
crates/sdkwork-knowledgebase-contract/src/wiki_file.rs
crates/sdkwork-knowledgebase-contract/tests/llm_wiki_contract.rs
crates/sdkwork-knowledgebase-agent-provider/src/llm_wiki.rs
crates/sdkwork-intelligence-knowledgebase-service/src/wiki/          # 整个目录
crates/sdkwork-intelligence-knowledgebase-service/src/ports/knowledge_wiki_page_store.rs
crates/sdkwork-intelligence-knowledgebase-service/src/ports/knowledge_wiki_file_entry_store.rs
crates/sdkwork-intelligence-knowledgebase-service/tests/wiki_page_service.rs
crates/sdkwork-intelligence-knowledgebase-service/tests/wiki_file_registry.rs
crates/sdkwork-intelligence-knowledgebase-service/tests/llm_wiki_renderers.rs
crates/sdkwork-intelligence-knowledgebase-repository-sqlx/src/wiki_page_store.rs
```

### 16.3 禁止残留的符号与标识

```text
LlmWikiPaths, LlmWikiCompatibility, LlmWikiStandardFileService
LlmWikiKnowledgeProvider, LLM_WIKI_KNOWLEDGE_PROVIDER_ID
WikiPageType, WikiPageSummary, KnowledgeWikiPage, WikiFileEntryType
KnowledgeWikiInitializerService, KnowledgeWikiPageService
llm_wiki_initialized, wiki_schema.yaml, wiki/pages/
provider.knowledge.llm-wiki, knowledge mode llm_wiki
document id prefix wiki:
operationId prefix wiki.
table prefix kb_wiki_
drive logical path prefix wiki/
object_role wiki_index, wiki_log, wiki_schema, wiki_revision
```

### 16.4 API / SDK / 前端

| 动作 | 范围 |
|------|------|
| DELETE + REGEN | `apis/*` 中全部 `wiki.*` paths 与 schemas |
| DELETE + REGEN | `sdks/*/openapi` 与 generated `wiki-*.ts` |
| REVISE | route manifests 中 wiki 路由 |
| REVISE | `apps/.../knowledgebaseDocumentApiBridge.ts`：`wiki_page` → `okf_concept` |

### 16.5 SQL 迁移

| 动作 | 说明 |
|------|------|
| REMOVE | 所有 `kb_wiki_*` CREATE TABLE |
| REMOVE | `kb_space.llm_wiki_initialized` |
| ADD | `kb_okf_*` 表与 `okf_bundle_initialized` |

应用未上线：**直接改初始迁移**，不做 ALTER 兼容迁移。

### 16.6 完成验证（全部通过才算完成）

```bash
# 1. 禁止符号扫描
node tools/check_okf_knowledge_bundle_standard.mjs

# 2. 全仓无 wiki/llm_wiki 残留（排除 external/okf 样例与 forbiddenLegacy 清单本身）
rg -l 'llm_wiki|LlmWiki|WikiPageType|kb_wiki_|wiki\.pages|wiki_schema' \
  --glob '!external/**' --glob '!target/**' --glob '!docs/architecture/tech/TECH-2026-06-01*'
# 期望：零匹配

# 3. OKF 合规
cargo test -p sdkwork-knowledgebase-contract okf
cargo test -p sdkwork-intelligence-knowledgebase-service okf

# 4. 全量验证
pnpm verify
```

### 16.7 明确保留（非技术债）

| 保留 | 原因 |
|------|------|
| `external/knowledge-catalog/okf/` | OKF 权威规范与样例 Bundle |
| `specs/okf-knowledge-bundle.spec.json` | 新标准契约 |
| `forbiddenLegacy` 配置项 | CI 防回归，不是旧实现 |

---

*End of specification.*
