#!/usr/bin/env node
import fs from 'node:fs';
import path from 'node:path';
import { fileURLToPath } from 'node:url';

const repoRoot = path.resolve(path.dirname(fileURLToPath(import.meta.url)), '..');
const target = path.join(
  repoRoot,
  'crates/sdkwork-routes-knowledgebase-app-api/src/http_route_manifest.rs',
);

export const APP_API_PERMISSION_BY_OPERATION = {
  'spaces.create': 'knowledge.spaces.write',
  'spaces.retrieve': 'knowledge.spaces.read',
  'spaces.update': 'knowledge.spaces.write',
  'spaces.delete': 'knowledge.spaces.write',
  'spaces.browser.list': 'knowledge.spaces.read',
  'spaces.contextBindings.list': 'knowledge.spaces.read',
  'spaces.contextBindings.create': 'knowledge.spaces.write',
  'spaces.members.list': 'knowledge.spaces.read',
  'spaces.members.create': 'knowledge.spaces.write',
  'spaces.members.delete': 'knowledge.spaces.write',
  'driveImports.create': 'knowledge.imports.write',
  'gitImports.create': 'knowledge.imports.write',
  'gitSyncs.create': 'knowledge.imports.write',
  'wechat.officialAccounts.list': 'knowledge.wechat.manage',
  'wechat.officialAccounts.update': 'knowledge.wechat.manage',
  'wechat.applets.list': 'knowledge.wechat.manage',
  'wechat.applets.update': 'knowledge.wechat.manage',
  'wechat.articles.publish': 'knowledge.wechat.manage',
  'wechat.articles.preview': 'knowledge.wechat.manage',
  'ingests.create': 'knowledge.ingests.write',
  'ingests.retrieve': 'knowledge.ingests.read',
  'documents.list': 'knowledge.documents.read',
  'documents.create': 'knowledge.documents.write',
  'documents.retrieve': 'knowledge.documents.read',
  'documents.update': 'knowledge.documents.write',
  'documents.delete': 'knowledge.documents.write',
  'documents.content.list': 'knowledge.documents.read',
  'documents.versions.list': 'knowledge.documents.read',
  'documents.versions.create': 'knowledge.documents.write',
  'okf.concepts.list': 'knowledge.okf.read',
  'okf.concepts.update': 'knowledge.okf.write',
  'okf.concepts.retrieve': 'knowledge.okf.read',
  'okf.concepts.delete': 'knowledge.okf.write',
  'okf.concepts.revisions.list': 'knowledge.okf.read',
  'okf.bundle.index.list': 'knowledge.okf.read',
  'okf.bundle.log.list': 'knowledge.okf.read',
  'okf.bundle.profile.list': 'knowledge.okf.read',
  'okf.queries.create': 'knowledge.okf.write',
  'okf.queries.fileAnswer': 'knowledge.okf.write',
  'okf.contextPacks.create': 'knowledge.okf.write',
  'okf.bundle.export.create': 'knowledge.okf.write',
  'okf.bundle.export.retrieve': 'knowledge.okf.read',
  'okf.bundle.import.create': 'knowledge.okf.write',
  'okf.lintRuns.create': 'knowledge.okf.write',
  'retrievals.create': 'knowledge.retrievals.write',
  'retrievals.retrieve': 'knowledge.retrievals.read',
  'contextPacks.create': 'knowledge.context_packs.write',
  'agentProfiles.create': 'knowledge.agent_profiles.write',
  'agentProfiles.retrieve': 'knowledge.agent_profiles.read',
  'agentProfiles.update': 'knowledge.agent_profiles.write',
  'agentProfiles.delete': 'knowledge.agent_profiles.write',
  'agentProfiles.bindings.list': 'knowledge.agent_profiles.read',
  'agentProfiles.bindings.create': 'knowledge.agent_profiles.write',
  'agentProfiles.bindings.update': 'knowledge.agent_profiles.write',
  'agentProfiles.bindings.delete': 'knowledge.agent_profiles.write',
  'agentProfiles.retrievalPreview.create': 'knowledge.agent_profiles.write',
  'agentProfiles.chat.create': 'knowledge.agent_profiles.write',
  'contextBindings.retrieve': 'knowledge.context_bindings.read',
  'contextBindings.update': 'knowledge.context_bindings.write',
  'contextBindings.delete': 'knowledge.context_bindings.write',
  'market.listings.list': 'knowledge.market.read',
  'market.subscriptions.create': 'knowledge.market.write',
  'market.subscriptions.delete': 'knowledge.market.write',
  'mediaTasks.create': 'knowledge.media.write',
  'groupLaunches.consume': 'knowledge.spaces.read',
  'wikiPublications.retrieve': 'knowledge.spaces.read',
  'wikiPublications.activate': 'knowledge.spaces.write',
  'wikiPublications.pause': 'knowledge.spaces.write',
  'wikiSourceFiles.publish': 'knowledge.spaces.write',
  'wikiSourceFiles.unpublish': 'knowledge.spaces.write',
  'wikiSourceFiles.visibility.update': 'knowledge.spaces.write',
  'wechat.officialAccounts.fanTags.list': 'knowledge.wechat.manage',
};

let source = fs.readFileSync(target, 'utf8');

source = source.replace(
  /const fn abuse_sensitive_route\([\s\S]*?\n\}\n\nconst HTTP_ROUTES/,
  `const fn knowledge_route(
    method: HttpMethod,
    path: &'static str,
    operation_id: &'static str,
    permission: &'static str,
) -> HttpRoute {
    HttpRoute::dual_token(method, path, "knowledge", operation_id)
        .with_required_permission(permission)
}

const fn knowledge_read_route(
    method: HttpMethod,
    path: &'static str,
    operation_id: &'static str,
) -> HttpRoute {
    HttpRoute::dual_token(method, path, "knowledge", operation_id)
}

const fn knowledge_abuse_route(
    method: HttpMethod,
    path: &'static str,
    operation_id: &'static str,
    permission: &'static str,
) -> HttpRoute {
    knowledge_route(method, path, operation_id, permission)
        .with_rate_limit_tier(RateLimitTier::AuthCritical)
}

const HTTP_ROUTES`,
);

source = source.replace(
  /HttpRoute::dual_token\(\s*HttpMethod::(\w+),\s*"([^"]+)",\s*"knowledge",\s*"([^"]+)",\s*\)/g,
  (_, method, routePath, operationId) => {
    if (method === 'Get') {
      return `knowledge_read_route(
        HttpMethod::${method},
        "${routePath}",
        "${operationId}",
    )`;
    }
    const permission = APP_API_PERMISSION_BY_OPERATION[operationId];
    if (!permission) {
      throw new Error(`missing permission mapping for ${operationId}`);
    }
    return `knowledge_route(
        HttpMethod::${method},
        "${routePath}",
        "${operationId}",
        "${permission}",
    )`;
  },
);

source = source.replace(
  /abuse_sensitive_route\(\s*HttpMethod::(\w+),\s*"([^"]+)",\s*"knowledge",\s*"([^"]+)",\s*\)/g,
  (_, method, routePath, operationId) => {
    if (method === 'Get') {
      return `knowledge_read_route(
        HttpMethod::${method},
        "${routePath}",
        "${operationId}",
    )`;
    }
    const permission = APP_API_PERMISSION_BY_OPERATION[operationId];
    if (!permission) {
      throw new Error(`missing permission mapping for ${operationId}`);
    }
    return `knowledge_abuse_route(
        HttpMethod::${method},
        "${routePath}",
        "${operationId}",
        "${permission}",
    )`;
  },
);

fs.writeFileSync(target, source);
console.log('patched knowledge app route permissions');
