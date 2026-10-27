# Standalone Docker 快速部署能力（参考 sdkwork-cloudrouter）

## 方案概述

采用 cloudrouter 已验证的模式：**宿主预编译产物 → 分级打包（staged context）→ 单 Dockerfile → compose 多服务 → WSL 宿主机 nginx 多域名反代 → Windows hosts 绑定**。

- 身份：`deploymentProfile=standalone`、`runtimeTarget=container`、包 ID `linux-x64-standalone-container-oci`
- 镜像 = 预编译 release 二进制（gateway+worker）+ PC 前端 dist + 数据库模块（自身/iam/drive/web-framework），镜像内无源码、无 node_modules、无 target（PACKAGING_SPEC §2.5/§4）
- 快速测试部署使用 `SDKWORK_KNOWLEDGEBASE_ENVIRONMENT=development`（cloudrouter 的 `SDKWORK_ENV=development` 同款做法）——production/staging 会强制要求 IM RPC 集群 + Redis，单机无法满足；dev 模式 IAM 自动生成 bootstrap access-token，登录链路可用
- 前端以 **同源相对路径** 构建（`VITE_SDKWORK_KNOWLEDGEBASE_DEV_SAME_ORIGIN_API=true`，已确认 runtimeConfig.ts:270 显式开启即生效），nginx 同时托管静态 + 反代 API 前缀
- 端口：容器内 18081（gateway 默认单端口承载全部 5 个 API 面 + /healthz /readyz /livez /metrics /openapi.json），宿主映射 **3903:18081**（避开 pnpm dev 的 18081）；postgres/redis 仅在 compose 网络内

## 域名链路

```
Windows 浏览器 → hosts: 127.0.0.1 testapidocker.sdkwork.com testapidocker.birdcoder.com testapidocker.dtupay.com
→ Windows localhost:80 → WSL 宿主 nginx :80（server_name 三域名并列）
→ 静态: /opt/sdkwork/knowledgebase/portal（dist，SPA fallback /index.html）
→ API:  /app/v3/api/* /backend/v3/api/* /knowledge/v3/api/* /internal/v3/api/* → proxy http://127.0.0.1:3903
→ 探针: /healthz /readyz /openapi.json → 同上
```

## 一、仓库内新增/修改（打包能力，均在 sdkwork-knowledgebase）

| 文件 | 内容 |
|---|---|
| `Dockerfile`（根，新增） | cloudrouter 式单阶段：`debian:bookworm-slim` + `libssl3 ca-certificates curl`（`--no-install-recommends`）；非 root `sdkwork` 用户；安装根 `/opt/sdkwork/knowledgebase`；ENV 注入 `SDKWORK_KNOWLEDGEBASE_APP_ROOT`、`SDKWORK_IAM_APP_ROOT`、`SDKWORK_DRIVE_APP_ROOT`、`SDKWORK_WEB_STORE_APP_ROOT`、`SDKWORK_APP_ROOT`、部署身份 env；`LABEL org.opencontainers.image.*`；EXPOSE 18081；ENTRYPOINT gateway，compose 侧 worker 服务覆盖 entrypoint |
| `.dockerignore`（根，新增） | 排除 target/node_modules/.git/docs/specs/密钥等（cloudrouter 同款） |
| `docker-compose.yml`（根，新增） | 4 服务：`postgres:16-alpine`（init SQL 建 schema、healthcheck、named volume）、`redis:7-alpine`（healthcheck、volume）、`api`（3903:18081、/readyz healthcheck、资源 limits、restart unless-stopped）、`worker`（无宿主端口）；全部密钥走 env 注入，镜像无真实 secret |
| `docker/.env.example` | 全部可调项：端口、PG/Redis、密钥 dev 默认值（IAM signing secret、secrets encryption key）、RUST_LOG、CORS 三域名白名单、资源限制 |
| `docker/postgres/init/001-create-schema.sql` | `CREATE SCHEMA IF NOT EXISTS sdkwork_ai_prod; GRANT ...`（与生命周期 search_path 对齐） |
| `docker/nginx/testapidocker-knowledgebase.conf` | 三域名 server 块 + 静态 root + SPA fallback + 4 个 API 前缀反代 + 探针反代；`client_max_body_size 32m`（对齐 k8s ingress 既有值） |
| `scripts/build-knowledgebase-container.mjs` | `pnpm build:container`：前置校验（release 二进制、dist、docker daemon）→ 组装 `dist/install-package-staging`（bin/ 经 strip、portal/dist、database 模块、agent.app.config.json、entrypoint、install-manifest.json 含内容清单+sha256）→ 生成安装包 tar.gz → 解包到 `dist/container-image-build` → `docker build` → 证据 `dist/container-image.json`（imageId/digest，RELEASE_SPEC §4.1）+ 输入快照缓存（cloudrouter 同款，重复构建秒级） |
| `scripts/deploy-wsl-docker.mjs`（薄包装）+ `scripts/wsl-deploy.sh`（WSL 内执行） | 幂等编排：安装 docker/nginx/rsync（apt，缺则装）→ 同步工作区到 WSL ext4（排除 target/node_modules/dist/.git，含 16 个必要兄弟仓库）→ rustup + `cargo build --release -p sdkwork-api-knowledgebase-standalone-gateway -p sdkwork-knowledgebase-worker` → pnpm install + `vite build --mode standalone.docker` → `build:container` → `docker compose up -d` → 安装 nginx site + reload → 写入 Windows hosts（失败则给出提权命令）→ 端到端验证输出 |
| `apps/sdkwork-knowledgebase-pc/.env.standalone.docker` | 提交的 Vite 构建环境：standalone 档案、environment=development、`DEV_SAME_ORIGIN_API=true`、browser-local token、dev auth 凭据 |
| `package.json` scripts | `build:container`、`build:container:check`、`deploy:docker:wsl`（名称过 `check:pnpm-script-standard`；`docker:*` 前缀禁用） |
| `agent.workflow.json` | 追加 target `linux-x64-standalone-container-oci`（runtimeTarget=container、deploymentProfile=standalone、format=oci、outputGlobs=release 二进制+dist）——仅追加不改动既有 target，遵循 GITHUB_WORKFLOW_SPEC §5.0 |
| `docs/installation/docker-deployment.md` | 完整部署文档：前置、WSL 安装、构建、compose、nginx、Windows hosts、验证、故障排查（Clash 代理直连、端口冲突、登录） |
| `deployments/docker/Dockerfile.api` + `.worker` | 删除（现状无法构建：依赖 ../sdkwork-* 路径、上下文无兄弟仓库；且 EXPOSE 8080 与真实 18081 不一致，被新根 Dockerfile 取代），`deployments/README.md` 同步更新 |

## 二、本机实际部署（WSL Ubuntu-22.04 + Windows 主机）

1. WSL：安装 docker engine（apt `docker.io` + compose 插件）、nginx、rsync；启动 docker 服务
2. 工作区同步到 WSL ext4（`~/sdkwork-workspace/`，16 个仓库：knowledgebase + iam/web-framework/drive/database/cloudrouter/kernel/memory/id/utils/im/rpc-framework/appbase/ui/core/sdk-commons/app-topology）——避开 9p 挂载的编译性能陷阱
3. WSL 内 `cargo build --release`（gateway + worker，首次约 1–2 小时一次性成本，之后增量）
4. `pnpm install` + `vite build --mode standalone.docker`（同源 API 构建）
5. `build:container` → `sdkwork-knowledgebase:local`（strip 后镜像预计 <300MB，不含任何源码/依赖树）
6. `docker compose up -d` → 4 服务全部 healthy（gateway 启动时经 `SDKWORK_DATABASE_AUTO_MIGRATE=1` 自动跑 baseline）
7. nginx site 安装到 `/etc/nginx/sites-enabled/sdkwork/testapidocker-knowledgebase.conf` + `nginx -t` + reload
8. Windows hosts 追加 `127.0.0.1 testapidocker.sdkwork.com testapidocker.birdcoder.com testapidocker.dtupay.com`（无权限则给出管理员命令）

## 三、端到端验证

- `docker compose ps` 全 healthy；`curl 127.0.0.1:3903/{healthz,readyz,openapi.json}`
- `curl http://testapidocker.sdkwork.com/`（HTML 200）+ API 前缀可达
- 浏览器访问三个域名 + 登录 + 建空间/文档基础功能冒烟（browser-use 验证）
- 输出 `dist/container-image.json` 证据 + install-manifest 内容清单

## 风险与对策

- cargo 首次编译耗时长：一次性成本，已文档化；快照缓存保证后续 build:container 秒级
- dev 登录链路：knowledgebase 与 cloudrouter 共用同一 IAM web adapter 机制，运行时实测登录，失败则显式注入 `SDKWORK_ACCESS_TOKEN` / super-admin 密码 env
- Windows hosts 写入权限：脚本先尝试直写，失败给出提权 PowerShell 命令
- 租户 ID 取 dev 档案值（100001/org 0），与 IAM bootstrap 对齐，运行时验证

## 人审说明

- 删除 2 个旧 Dockerfile、修改 agent.workflow.json（release 治理）属规范要求的人审范围——此计划即提交审核；其余均为新增文件 + package.json 脚本追加