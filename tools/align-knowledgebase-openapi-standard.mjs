#!/usr/bin/env node
/**
 * Aligns Knowledgebase SDK-owned OpenAPI authorities with SDKWork v3 HTTP rules.
 *
 * The SDK family OpenAPI files under SDK family openapi directories are the local authority.
 * Generated transports and apis/** copies are derived from these files.
 */
import { readFile, writeFile } from 'node:fs/promises';
import path from 'node:path';
import { fileURLToPath } from 'node:url';
import {
  browserListEnvelope,
  commandEnvelope,
  createdResponse,
  jsonResponse,
  listDataEnvelope,
  listEnvelope,
  resourceEnvelope,
} from './lib/openapi-envelope.mjs';

const scriptDir = path.dirname(fileURLToPath(import.meta.url));
const workspaceRoot = path.resolve(scriptDir, '..');
const checkOnly = process.argv.includes('--check');
const pendingChanges = [];

const appOpenApiPath = path.join(
  workspaceRoot,
  'sdks/sdkwork-knowledgebase-app-sdk/openapi/knowledgebase-app-api.openapi.json',
);
const backendOpenApiPath = path.join(
  workspaceRoot,
  'sdks/sdkwork-knowledgebase-backend-sdk/openapi/knowledgebase-backend-api.openapi.json',
);

const appOperations = [
  resource('post', '/app/v3/api/knowledge/group_launches/consume', {
    operationId: 'groupLaunches.consume',
    itemRef: '#/components/schemas/GroupKnowledgebaseLaunchTarget',
  }),
  resource('get', '/app/v3/api/knowledge/documents/{documentId}/content', {
    operationId: 'documents.content.list',
    itemRef: '#/components/schemas/KnowledgeDocumentContent',
  }),
  resource('get', '/app/v3/api/knowledge/spaces/{spaceId}/wiki_publication', {
    operationId: 'wikiPublications.retrieve',
    itemRef: '#/components/schemas/KnowledgeWikiPublication',
  }),
  resource('post', '/app/v3/api/knowledge/spaces/{spaceId}/wiki_publication/activate', {
    operationId: 'wikiPublications.activate',
    itemRef: '#/components/schemas/KnowledgeWikiPublication',
  }),
  resource('post', '/app/v3/api/knowledge/spaces/{spaceId}/wiki_publication/pause', {
    operationId: 'wikiPublications.pause',
    itemRef: '#/components/schemas/KnowledgeWikiPublication',
  }),
  resource(
    'post',
    '/app/v3/api/knowledge/spaces/{spaceId}/wiki_source_files/{sourceFileUuid}/publish',
    {
      operationId: 'wikiSourceFiles.publish',
      itemRef: '#/components/schemas/KnowledgeWikiSourceFileCommandResult',
    },
  ),
  resource(
    'post',
    '/app/v3/api/knowledge/spaces/{spaceId}/wiki_source_files/{sourceFileUuid}/unpublish',
    {
      operationId: 'wikiSourceFiles.unpublish',
      itemRef: '#/components/schemas/KnowledgeWikiSourceFileCommandResult',
    },
  ),
  resource(
    'patch',
    '/app/v3/api/knowledge/spaces/{spaceId}/wiki_source_files/{sourceFileUuid}/visibility',
    {
      operationId: 'wikiSourceFiles.visibility.update',
      itemRef: '#/components/schemas/KnowledgeWikiSourceFileCommandResult',
    },
  ),
  resource('post', '/app/v3/api/knowledge/documents/{documentId}/versions', {
    operationId: 'documents.versions.create',
    status: '200',
    itemRef: '#/components/schemas/KnowledgeDocumentVersion',
  }),
  namedList('get', '/app/v3/api/knowledge/okf/concepts', {
    operationId: 'okf.concepts.list',
    dataRef: '#/components/schemas/OkfConceptSummaryList',
  }),
  namedList('get', '/app/v3/api/knowledge/okf/concepts/{conceptId}/revisions', {
    operationId: 'okf.concepts.revisions.list',
    dataRef: '#/components/schemas/KnowledgeOkfConceptRevisionList',
  }),
  resource('get', '/app/v3/api/knowledge/okf/index', {
    operationId: 'okf.bundle.index.list',
    itemRef: '#/components/schemas/OkfIndexDocument',
  }),
  resource('get', '/app/v3/api/knowledge/okf/log', {
    operationId: 'okf.bundle.log.list',
    itemRef: '#/components/schemas/OkfLogDocument',
  }),
  resource('get', '/app/v3/api/knowledge/okf/profile', {
    operationId: 'okf.bundle.profile.list',
    itemRef: '#/components/schemas/OkfProfileDocument',
  }),
  resource('post', '/app/v3/api/knowledge/okf/queries/{queryId}/file_answer', {
    operationId: 'okf.queries.fileAnswer',
    status: '200',
    itemRef: '#/components/schemas/OkfQueryResult',
  }),
  resource('put', '/app/v3/api/knowledge/okf/concepts/upsert', {
    operationId: 'okf.concepts.update',
    itemRef: '#/components/schemas/OkfConceptSummary',
  }),
  resource('get', '/app/v3/api/knowledge/agent_profiles/{profileId}/bindings', {
    operationId: 'agentProfiles.bindings.list',
    itemRef: '#/components/schemas/KnowledgeAgentBindingList',
  }),
  resource('post', '/app/v3/api/knowledge/agent_profiles/{profileId}/bindings', {
    operationId: 'agentProfiles.bindings.create',
    status: '200',
    itemRef: '#/components/schemas/KnowledgeAgentBinding',
  }),
  resource('post', '/app/v3/api/knowledge/agent_profiles/{profileId}/retrieval_preview', {
    operationId: 'agentProfiles.retrievalPreview.create',
    status: '200',
    itemRef: '#/components/schemas/KnowledgeRetrievalResult',
  }),
  resource('post', '/app/v3/api/knowledge/agent_profiles/{profileId}/chat', {
    operationId: 'agentProfiles.chat.create',
    status: '200',
    itemRef: '#/components/schemas/KnowledgeAgentChatResponse',
  }),
  list('get', '/app/v3/api/knowledge/spaces/{spaceId}/context_bindings', {
    operationId: 'spaces.contextBindings.list',
    itemRef: '#/components/schemas/KnowledgeSpaceContextBinding',
  }),
  resource('post', '/app/v3/api/knowledge/spaces/{spaceId}/context_bindings', {
    operationId: 'spaces.contextBindings.create',
    status: '200',
    itemRef: '#/components/schemas/KnowledgeSpaceContextBinding',
  }),
  list('get', '/app/v3/api/knowledge/spaces/{spaceId}/members', {
    operationId: 'spaces.members.list',
    itemRef: '#/components/schemas/KnowledgeSpaceMember',
  }),
  command('post', '/app/v3/api/knowledge/spaces/{spaceId}/members', {
    operationId: 'spaces.members.create',
    status: '200',
    payloadRef: '#/components/schemas/SdkWorkCommandData',
  }),
  noContent('delete', '/app/v3/api/knowledge/spaces/{spaceId}/members', {
    operationId: 'spaces.members.delete',
  }),
  resource('get', '/app/v3/api/knowledge/wechat/official_accounts', {
    operationId: 'wechat.officialAccounts.list',
    itemRef: '#/components/schemas/KnowledgeWechatOfficialAccountList',
  }),
  resource('put', '/app/v3/api/knowledge/wechat/official_accounts', {
    operationId: 'wechat.officialAccounts.update',
    itemRef: '#/components/schemas/KnowledgeWechatOfficialAccountList',
  }),
  resource('get', '/app/v3/api/knowledge/wechat/applets', {
    operationId: 'wechat.applets.list',
    itemRef: '#/components/schemas/KnowledgeWechatAppletList',
  }),
  resource('put', '/app/v3/api/knowledge/wechat/applets', {
    operationId: 'wechat.applets.update',
    itemRef: '#/components/schemas/KnowledgeWechatAppletList',
  }),
  resource('post', '/app/v3/api/knowledge/wechat/articles/publish', {
    operationId: 'wechat.articles.publish',
    status: '200',
    itemRef: '#/components/schemas/KnowledgeWechatOperationResult',
    command: true,
  }),
  resource('post', '/app/v3/api/knowledge/wechat/articles/preview', {
    operationId: 'wechat.articles.preview',
    status: '200',
    itemRef: '#/components/schemas/KnowledgeWechatOperationResult',
    command: true,
  }),
  command('post', '/app/v3/api/knowledge/git_syncs', {
    operationId: 'gitSyncs.create',
    status: '201',
    payloadRef: '#/components/schemas/KnowledgeGitSyncResult',
  }),
  command('post', '/app/v3/api/knowledge/market/subscriptions', {
    operationId: 'market.subscriptions.create',
    status: '201',
    payloadRef: '#/components/schemas/KnowledgeMarketSubscriptionResult',
  }),
  noContent('delete', '/app/v3/api/knowledge/market/subscriptions/{listingId}', {
    operationId: 'market.subscriptions.delete',
  }),
  command('post', '/app/v3/api/knowledge/media_tasks', {
    operationId: 'mediaTasks.create',
    status: '201',
    payloadRef: '#/components/schemas/KnowledgeMediaTaskResult',
  }),
  browserList('get', '/app/v3/api/knowledge/spaces/{spaceId}/browser', {
    operationId: 'spaces.browser.list',
  }),
];

const backendOperations = [
  resource('post', '/backend/v3/api/knowledge/okf/candidates/{candidateId}/approve', {
    operationId: 'okf.candidates.approve',
    status: '200',
    itemRef: '#/components/schemas/OkfCandidateResult',
  }),
  resource('post', '/backend/v3/api/knowledge/okf/candidates/{candidateId}/reject', {
    operationId: 'okf.candidates.reject',
    status: '200',
    itemRef: '#/components/schemas/OkfCandidateResult',
  }),
  resource('post', '/backend/v3/api/knowledge/okf/concepts/{conceptId}/publish', {
    operationId: 'okf.concepts.publish',
    status: '200',
    itemRef: '#/components/schemas/OkfConceptSummary',
  }),
  resource('post', '/backend/v3/api/knowledge/okf/index/rebuild', {
    operationId: 'okf.bundle.index.rebuild',
    status: '200',
    itemRef: '#/components/schemas/OkfIndexDocument',
  }),
  resource('get', '/backend/v3/api/knowledge/provider_health', {
    operationId: 'providerHealth.list',
    itemRef: '#/components/schemas/KnowledgeProviderHealth',
  }),
  namedList('get', '/backend/v3/api/knowledge/provider_credential_references', {
    operationId: 'providerCredentialReferences.list',
    dataRef: '#/components/schemas/KnowledgeEngineProviderCredentialReferencePage',
  }),
  resource('post', '/backend/v3/api/knowledge/provider_credential_references', {
    operationId: 'providerCredentialReferences.create',
    status: '201',
    itemRef: '#/components/schemas/KnowledgeEngineProviderCredentialReference',
  }),
  resource(
    'get',
    '/backend/v3/api/knowledge/provider_credential_references/{credentialReferenceId}',
    {
      operationId: 'providerCredentialReferences.retrieve',
      itemRef: '#/components/schemas/KnowledgeEngineProviderCredentialReference',
    },
  ),
  command(
    'post',
    '/backend/v3/api/knowledge/provider_credential_references/{credentialReferenceId}/rotate',
    {
      operationId: 'providerCredentialReferences.rotate',
      payloadRef: '#/components/schemas/SdkWorkCommandData',
    },
  ),
  command(
    'post',
    '/backend/v3/api/knowledge/provider_credential_references/{credentialReferenceId}/revoke',
    {
      operationId: 'providerCredentialReferences.revoke',
      payloadRef: '#/components/schemas/SdkWorkCommandData',
    },
  ),
  namedList('get', '/backend/v3/api/knowledge/spaces/{spaceId}/provider_bindings', {
    operationId: 'spaces.providerBindings.list',
    dataRef: '#/components/schemas/KnowledgeEngineProviderBindingPage',
  }),
  resource('post', '/backend/v3/api/knowledge/spaces/{spaceId}/provider_bindings', {
    operationId: 'spaces.providerBindings.create',
    status: '201',
    itemRef: '#/components/schemas/KnowledgeEngineProviderBinding',
  }),
  resource(
    'get',
    '/backend/v3/api/knowledge/spaces/{spaceId}/provider_bindings/{bindingId}',
    {
      operationId: 'spaces.providerBindings.retrieve',
      itemRef: '#/components/schemas/KnowledgeEngineProviderBinding',
    },
  ),
  resource(
    'patch',
    '/backend/v3/api/knowledge/spaces/{spaceId}/provider_bindings/{bindingId}',
    {
      operationId: 'spaces.providerBindings.update',
      itemRef: '#/components/schemas/KnowledgeEngineProviderBinding',
    },
  ),
  command(
    'post',
    '/backend/v3/api/knowledge/spaces/{spaceId}/provider_bindings/{bindingId}/test',
    {
      operationId: 'spaces.providerBindings.test',
      payloadRef: '#/components/schemas/SdkWorkCommandData',
    },
  ),
  command(
    'post',
    '/backend/v3/api/knowledge/spaces/{spaceId}/provider_bindings/{bindingId}/activate',
    {
      operationId: 'spaces.providerBindings.activate',
      payloadRef: '#/components/schemas/SdkWorkCommandData',
    },
  ),
  command(
    'post',
    '/backend/v3/api/knowledge/spaces/{spaceId}/provider_bindings/{bindingId}/disable',
    {
      operationId: 'spaces.providerBindings.disable',
      payloadRef: '#/components/schemas/SdkWorkCommandData',
    },
  ),
  namedList('get', '/backend/v3/api/knowledge/spaces/{spaceId}/provider_migrations', {
    operationId: 'spaces.providerMigrations.list',
    dataRef: '#/components/schemas/KnowledgeEngineProviderMigrationOperationPage',
  }),
  resource('post', '/backend/v3/api/knowledge/spaces/{spaceId}/provider_migrations', {
    operationId: 'spaces.providerMigrations.create',
    status: '201',
    itemRef: '#/components/schemas/KnowledgeEngineProviderMigrationOperation',
  }),
  resource(
    'get',
    '/backend/v3/api/knowledge/spaces/{spaceId}/provider_migrations/{migrationOperationId}',
    {
      operationId: 'spaces.providerMigrations.retrieve',
      itemRef: '#/components/schemas/KnowledgeEngineProviderMigrationOperation',
    },
  ),
  command(
    'post',
    '/backend/v3/api/knowledge/spaces/{spaceId}/provider_migrations/{migrationOperationId}/rollback',
    {
      operationId: 'spaces.providerMigrations.rollback',
      payloadRef: '#/components/schemas/SdkWorkCommandData',
    },
  ),
  resource('get', '/backend/v3/api/knowledge/tenants/current', {
    operationId: 'tenants.current.list',
    itemRef: '#/components/schemas/KnowledgeTenantStatus',
  }),
  resource('post', '/backend/v3/api/knowledge/compliance/audit_events/export', {
    operationId: 'compliance.auditEvents.export.create',
    status: '201',
    itemRef: '#/components/schemas/KnowledgeAuditEventExport',
  }),
  resource('post', '/backend/v3/api/knowledge/compliance/audit_events/anonymize_actor', {
    operationId: 'compliance.auditEvents.anonymizeActor.create',
    status: '201',
    itemRef: '#/components/schemas/AnonymizeKnowledgeAuditSubjectResult',
  }),
];

function resource(method, routePath, options) {
  return {
    method,
    routePath,
    operationId: options.operationId,
    status: options.status ?? '200',
    schema: options.command
      ? commandEnvelope(options.itemRef)
      : resourceEnvelope(options.itemRef),
  };
}

function list(method, routePath, options) {
  return {
    method,
    routePath,
    operationId: options.operationId,
    status: '200',
    schema: listEnvelope(options.itemRef),
  };
}

function namedList(method, routePath, options) {
  return {
    method,
    routePath,
    operationId: options.operationId,
    status: '200',
    schema: listDataEnvelope(options.dataRef),
  };
}

function browserList(method, routePath, options) {
  return {
    method,
    routePath,
    operationId: options.operationId,
    status: '200',
    schema: browserListEnvelope(),
  };
}

function command(method, routePath, options) {
  return {
    method,
    routePath,
    operationId: options.operationId,
    status: options.status ?? '200',
    schema: commandEnvelope(options.payloadRef),
  };
}

function noContent(method, routePath, options) {
  return {
    method,
    routePath,
    operationId: options.operationId,
    status: '204',
    noContent: true,
  };
}

function ensureWikiPublicationContracts(spec) {
  const schemas = spec.components?.schemas;
  if (!schemas || typeof schemas !== 'object') {
    throw new Error('App OpenAPI components.schemas is required');
  }

  schemas.KnowledgeWikiPublicationStatus = stringEnum([
    'draft',
    'validating',
    'ready',
    'active',
    'degraded',
    'paused',
    'archived',
    'failed',
  ]);
  schemas.KnowledgeWikiPublicationMode = stringEnum([
    'review_required',
    'auto_public_after_checks',
  ]);
  schemas.KnowledgeWikiVisibility = stringEnum(['private', 'unlisted', 'public']);
  schemas.KnowledgeWikiUpdatePolicy = stringEnum([
    'keep_last_public_until_ready',
    'unpublish_during_processing',
  ]);
  schemas.KnowledgeWikiSourceFileKind = stringEnum([
    'page',
    'document',
    'presentation',
    'spreadsheet',
    'code',
    'media',
    'asset',
    'archive',
  ]);
  schemas.KnowledgeWikiSourceState = stringEnum([
    'discovered',
    'queued',
    'processing',
    'ready',
    'error',
    'quarantined',
    'deleted',
  ]);
  schemas.KnowledgeWikiPagePublicationState = stringEnum([
    'draft',
    'in_review',
    'scheduled',
    'published',
    'unpublished',
    'archived',
  ]);
  schemas.KnowledgeWikiIndexState = stringEnum([
    'not_required',
    'pending',
    'indexing',
    'ready',
    'error',
  ]);
  schemas.KnowledgeWikiPublication = objectSchema(
    [
      'uuid',
      'spaceId',
      'driveSpaceUuid',
      'sourceRootNodeUuid',
      'status',
      'title',
      'homepageSourcePath',
      'publicationMode',
      'defaultVisibility',
      'updatePolicy',
      'providerGeneration',
      'navigationGeneration',
      'searchGeneration',
      'lastProjectedDriveCheckpoint',
      'version',
    ],
    {
      uuid: boundedString(1, 64),
      spaceId: int64StringSchema(),
      driveSpaceUuid: boundedString(1, 64),
      sourceRootNodeUuid: nullableSchema(boundedString(1, 64)),
      status: { $ref: '#/components/schemas/KnowledgeWikiPublicationStatus' },
      title: boundedString(1, 256),
      homepageSourcePath: boundedString(1, 1024),
      publicationMode: { $ref: '#/components/schemas/KnowledgeWikiPublicationMode' },
      defaultVisibility: { $ref: '#/components/schemas/KnowledgeWikiVisibility' },
      updatePolicy: { $ref: '#/components/schemas/KnowledgeWikiUpdatePolicy' },
      providerGeneration: int64StringSchema(),
      navigationGeneration: int64StringSchema(),
      searchGeneration: int64StringSchema(),
      lastProjectedDriveCheckpoint: int64StringSchema(),
      version: int64StringSchema(),
    },
    'Canonical Wiki publication state for one Knowledgebase.',
  );
  schemas.KnowledgeWikiSourceFile = objectSchema(
    [
      'uuid',
      'driveNodeUuid',
      'driveVersionUuid',
      'sourcePath',
      'canonicalRoute',
      'fileKind',
      'mediaType',
      'sizeBytes',
      'contentSha256',
      'sourceState',
      'publicationState',
      'visibility',
      'indexState',
      'publicDriveVersionUuid',
      'pagePublicVersion',
      'version',
    ],
    {
      uuid: boundedString(1, 64),
      driveNodeUuid: boundedString(1, 64),
      driveVersionUuid: boundedString(1, 64),
      sourcePath: boundedString(1, 2048),
      canonicalRoute: nullableSchema(boundedString(1, 2048)),
      fileKind: { $ref: '#/components/schemas/KnowledgeWikiSourceFileKind' },
      mediaType: boundedString(1, 255),
      sizeBytes: int64StringSchema(),
      contentSha256: boundedString(1, 128),
      sourceState: { $ref: '#/components/schemas/KnowledgeWikiSourceState' },
      publicationState: { $ref: '#/components/schemas/KnowledgeWikiPagePublicationState' },
      visibility: { $ref: '#/components/schemas/KnowledgeWikiVisibility' },
      indexState: { $ref: '#/components/schemas/KnowledgeWikiIndexState' },
      publicDriveVersionUuid: nullableSchema(boundedString(1, 64)),
      pagePublicVersion: int64StringSchema(),
      version: int64StringSchema(),
    },
    'Projected sources/raw file state and its pinned public version.',
  );
  schemas.KnowledgeWikiPublicationVersionCommandRequest = objectSchema(
    ['expectedVersion'],
    { expectedVersion: int64StringSchema() },
    'Optimistic Wiki publication status command.',
  );
  schemas.PublishKnowledgeWikiSourceFileRequest = objectSchema(
    ['visibility', 'expectedPublicationVersion', 'expectedPageVersion'],
    {
      visibility: stringEnum(['unlisted', 'public']),
      expectedPublicationVersion: int64StringSchema(),
      expectedPageVersion: int64StringSchema(),
    },
    'Publish the exact current Drive version as PUBLIC or UNLISTED.',
  );
  schemas.KnowledgeWikiSourceFileVersionCommandRequest = objectSchema(
    ['expectedPublicationVersion', 'expectedPageVersion'],
    {
      expectedPublicationVersion: int64StringSchema(),
      expectedPageVersion: int64StringSchema(),
    },
    'Optimistic Wiki source-file publication command.',
  );
  schemas.ChangeKnowledgeWikiSourceFileVisibilityRequest = objectSchema(
    ['visibility', 'expectedPublicationVersion', 'expectedPageVersion'],
    {
      visibility: { $ref: '#/components/schemas/KnowledgeWikiVisibility' },
      expectedPublicationVersion: int64StringSchema(),
      expectedPageVersion: int64StringSchema(),
    },
    'Change a published Wiki source file visibility with version fencing.',
  );
  schemas.KnowledgeWikiSourceFileCommandResult = objectSchema(
    ['publication', 'sourceFile'],
    {
      publication: { $ref: '#/components/schemas/KnowledgeWikiPublication' },
      sourceFile: { $ref: '#/components/schemas/KnowledgeWikiSourceFile' },
    },
    'Updated publication and source-file state after a Wiki command.',
  );

  const spaceId = pathIdParameter('spaceId');
  const sourceFileUuid = {
    name: 'sourceFileUuid',
    in: 'path',
    required: true,
    schema: boundedString(1, 64),
  };
  registerWikiOperation(spec, 'get', '/app/v3/api/knowledge/spaces/{spaceId}/wiki_publication', {
    operationId: 'wikiPublications.retrieve',
    summary: 'Retrieve Wiki publication',
    parameters: [spaceId],
    permission: 'knowledge.spaces.read',
    resource: 'wiki-publication',
  });
  registerWikiOperation(
    spec,
    'post',
    '/app/v3/api/knowledge/spaces/{spaceId}/wiki_publication/activate',
    {
      operationId: 'wikiPublications.activate',
      summary: 'Activate Wiki publication',
      parameters: [spaceId],
      requestSchema: '#/components/schemas/KnowledgeWikiPublicationVersionCommandRequest',
      permission: 'knowledge.spaces.write',
      resource: 'wiki-publication',
      auditEvent: 'knowledge.wiki.publication.activated',
    },
  );
  registerWikiOperation(
    spec,
    'post',
    '/app/v3/api/knowledge/spaces/{spaceId}/wiki_publication/pause',
    {
      operationId: 'wikiPublications.pause',
      summary: 'Pause Wiki publication',
      parameters: [spaceId],
      requestSchema: '#/components/schemas/KnowledgeWikiPublicationVersionCommandRequest',
      permission: 'knowledge.spaces.write',
      resource: 'wiki-publication',
      auditEvent: 'knowledge.wiki.publication.paused',
    },
  );
  registerWikiOperation(
    spec,
    'post',
    '/app/v3/api/knowledge/spaces/{spaceId}/wiki_source_files/{sourceFileUuid}/publish',
    {
      operationId: 'wikiSourceFiles.publish',
      summary: 'Publish Wiki source file',
      parameters: [spaceId, sourceFileUuid],
      requestSchema: '#/components/schemas/PublishKnowledgeWikiSourceFileRequest',
      permission: 'knowledge.spaces.write',
      resource: 'wiki-source-file',
      auditEvent: 'knowledge.wiki.source_file.published',
    },
  );
  registerWikiOperation(
    spec,
    'post',
    '/app/v3/api/knowledge/spaces/{spaceId}/wiki_source_files/{sourceFileUuid}/unpublish',
    {
      operationId: 'wikiSourceFiles.unpublish',
      summary: 'Unpublish Wiki source file',
      parameters: [spaceId, sourceFileUuid],
      requestSchema: '#/components/schemas/KnowledgeWikiSourceFileVersionCommandRequest',
      permission: 'knowledge.spaces.write',
      resource: 'wiki-source-file',
      auditEvent: 'knowledge.wiki.source_file.unpublished',
    },
  );
  registerWikiOperation(
    spec,
    'patch',
    '/app/v3/api/knowledge/spaces/{spaceId}/wiki_source_files/{sourceFileUuid}/visibility',
    {
      operationId: 'wikiSourceFiles.visibility.update',
      summary: 'Change Wiki source file visibility',
      parameters: [spaceId, sourceFileUuid],
      requestSchema: '#/components/schemas/ChangeKnowledgeWikiSourceFileVisibilityRequest',
      permission: 'knowledge.spaces.write',
      resource: 'wiki-source-file',
      auditEvent: 'knowledge.wiki.source_file.visibility_changed',
    },
  );
}

function registerWikiOperation(spec, method, routePath, options) {
  spec.paths ??= {};
  spec.paths[routePath] ??= {};
  removeNonCanonicalHttpOperations(spec.paths[routePath], method);
  const template = spec.paths?.['/app/v3/api/knowledge/spaces/{spaceId}']?.get;
  if (!template) {
    throw new Error('Knowledge space retrieve operation is required as App API metadata template');
  }
  const responses = Object.fromEntries(
    Object.entries(template.responses ?? {})
      .filter(([status]) => !/^2[0-9][0-9]$/u.test(status))
      .map(([status, response]) => [status, structuredClone(response)]),
  );
  spec.paths[routePath][method] = {
    operationId: options.operationId,
    tags: ['knowledge'],
    summary: options.summary,
    description: options.summary,
    parameters: [
      ...(method === 'get'
        ? []
        : [{ $ref: '#/components/parameters/IdempotencyKeyHeader' }]),
      ...structuredClone(options.parameters),
    ],
    ...(options.requestSchema
      ? {
          requestBody: {
            required: true,
            content: {
              'application/json': { schema: { $ref: options.requestSchema } },
            },
          },
        }
      : {}),
    responses,
    security: structuredClone(template.security),
    'x-sdkwork-owner': 'sdkwork-knowledgebase',
    'x-sdkwork-api-authority': 'sdkwork-knowledgebase-app-api',
    'x-sdkwork-request-context': 'WebRequestContext',
    'x-sdkwork-api-surface': 'app-api',
    'x-sdkwork-source-route-crate': 'sdkwork-routes-knowledgebase-app-api',
    'x-sdkwork-auth-mode': 'dual-token',
    'x-sdkwork-domain': 'intelligence',
    'x-sdkwork-resource': options.resource,
    'x-sdkwork-permission': options.permission,
    'x-sdkwork-tenant-scope': 'tenant',
    'x-sdkwork-data-scope': 'organization',
    ...(method !== 'get'
      ? {
          'x-sdkwork-rate-limit-tier': 'auth-critical',
          'x-sdkwork-idempotent': true,
          'x-sdkwork-audit-event': options.auditEvent,
        }
      : {}),
  };
}

function removeNonCanonicalHttpOperations(pathItem, canonicalMethod) {
  for (const method of ['get', 'put', 'post', 'delete', 'options', 'head', 'patch', 'trace']) {
    if (method !== canonicalMethod) {
      delete pathItem[method];
    }
  }
}

function stringEnum(values) {
  return { type: 'string', enum: values };
}

function boundedString(minLength, maxLength) {
  return { type: 'string', minLength, maxLength };
}

function objectSchema(required, properties, description) {
  return {
    type: 'object',
    additionalProperties: false,
    description,
    required,
    properties,
  };
}

function ensureProviderManagementContracts(spec) {
  const schemas = spec.components?.schemas;
  if (!schemas || typeof schemas !== 'object') {
    throw new Error('Backend OpenAPI components.schemas is required');
  }

  schemas.KnowledgeEngineProviderBindingState ??= {
    type: 'string',
    enum: ['draft', 'testing', 'active', 'degraded', 'disabled', 'failed'],
  };
  schemas.KnowledgeEngineProviderMigrationState ??= {
    type: 'string',
    enum: [
      'dry_run',
      'preparing',
      'validating',
      'cutover',
      'observing',
      'completed',
      'rolling_back',
      'rolled_back',
      'failed',
    ],
  };
  schemas.KnowledgeEngineProviderCredentialRotationState ??= {
    type: 'string',
    enum: ['current', 'rotation_due', 'revoked'],
  };
  schemas.KnowledgeEngineCapability ??= {
    type: 'string',
    enum: ['health', 'search', 'read_document', 'list_documents', 'ingest', 'sync_sources'],
  };
  schemas.KnowledgeEngineProviderErrorCategory ??= {
    type: 'string',
    enum: [
      'authentication',
      'permission_denied',
      'rate_limited',
      'timeout',
      'unavailable',
      'circuit_open',
      'bulkhead_saturated',
      'invalid_response',
      'response_too_large',
      'invalid_target',
      'not_found',
      'validation',
      'unsupported',
      'internal',
    ],
  };
  schemas.KnowledgeEngineProviderCredentialReference ??= {
    type: 'object',
    additionalProperties: false,
    required: [
      'id',
      'uuid',
      'tenantId',
      'organizationId',
      'implementationId',
      'displayName',
      'rotationState',
      'createdBy',
      'updatedBy',
      'createdAt',
      'updatedAt',
      'version',
    ],
    properties: {
      id: int64StringSchema(),
      uuid: { type: 'string', format: 'uuid' },
      tenantId: int64StringSchema(),
      organizationId: int64StringSchema(),
      implementationId: { type: 'string', maxLength: 128 },
      displayName: { type: 'string', maxLength: 256 },
      rotationState: {
        $ref: '#/components/schemas/KnowledgeEngineProviderCredentialRotationState',
      },
      lastRotatedAt: nullableSchema({ type: 'string', format: 'date-time' }),
      createdBy: { type: 'string', maxLength: 128 },
      updatedBy: { type: 'string', maxLength: 128 },
      createdAt: { type: 'string', format: 'date-time' },
      updatedAt: { type: 'string', format: 'date-time' },
      version: int64StringSchema(),
    },
  };
  schemas.CreateKnowledgeEngineProviderCredentialReferenceRequest ??= {
    type: 'object',
    additionalProperties: false,
    required: ['implementationId', 'displayName', 'referenceLocator'],
    properties: {
      implementationId: { type: 'string', minLength: 1, maxLength: 128 },
      displayName: { type: 'string', minLength: 1, maxLength: 256 },
      referenceLocator: {
        type: 'string',
        minLength: 1,
        maxLength: 2048,
        writeOnly: true,
      },
    },
  };
  schemas.RotateKnowledgeEngineProviderCredentialReferenceRequest ??= {
    type: 'object',
    additionalProperties: false,
    required: ['referenceLocator', 'expectedVersion'],
    properties: {
      referenceLocator: {
        type: 'string',
        minLength: 1,
        maxLength: 2048,
        writeOnly: true,
      },
      expectedVersion: int64StringSchema(),
    },
  };
  schemas.RevokeKnowledgeEngineProviderCredentialReferenceRequest ??= {
    type: 'object',
    additionalProperties: false,
    required: ['expectedVersion'],
    properties: { expectedVersion: int64StringSchema() },
  };
  schemas.CreateKnowledgeEngineProviderBindingRequest ??= {
    type: 'object',
    additionalProperties: false,
    required: ['implementationId', 'remoteResourceType', 'remoteResourceId'],
    properties: {
      implementationId: { type: 'string', minLength: 1, maxLength: 128 },
      remoteResourceType: { type: 'string', minLength: 1, maxLength: 64 },
      remoteResourceId: { type: 'string', minLength: 1, maxLength: 512 },
      credentialReferenceId: nullableSchema(int64StringSchema()),
    },
  };
  schemas.UpdateKnowledgeEngineProviderBindingRequest ??= {
    type: 'object',
    additionalProperties: false,
    required: ['clearCredentialReference', 'expectedVersion'],
    properties: {
      remoteResourceType: nullableSchema({ type: 'string', minLength: 1, maxLength: 64 }),
      remoteResourceId: nullableSchema({ type: 'string', minLength: 1, maxLength: 512 }),
      credentialReferenceId: nullableSchema(int64StringSchema()),
      clearCredentialReference: { type: 'boolean' },
      expectedVersion: int64StringSchema(),
    },
  };
  schemas.ProviderBindingVersionCommandRequest ??= {
    type: 'object',
    additionalProperties: false,
    required: ['expectedVersion'],
    properties: { expectedVersion: int64StringSchema() },
  };
  schemas.ProviderMigrationVersionCommandRequest ??= {
    type: 'object',
    additionalProperties: false,
    required: ['expectedVersion'],
    properties: { expectedVersion: int64StringSchema() },
  };
  schemas.CreateKnowledgeEngineProviderMigrationOperationRequest ??= {
    type: 'object',
    additionalProperties: false,
    required: [
      'sourceBindingId',
      'targetBindingId',
      'idempotencyKey',
      'expectedSourceVersion',
      'expectedTargetVersion',
      'observationSeconds',
    ],
    properties: {
      sourceBindingId: int64StringSchema(),
      targetBindingId: int64StringSchema(),
      idempotencyKey: { type: 'string', minLength: 1, maxLength: 128 },
      expectedSourceVersion: int64StringSchema(),
      expectedTargetVersion: int64StringSchema(),
      observationSeconds: {
        type: 'integer',
        format: 'int32',
        minimum: 60,
        maximum: 604800,
      },
    },
  };
  schemas.KnowledgeEngineProviderBinding ??= {
    type: 'object',
    additionalProperties: false,
    required: [
      'id',
      'uuid',
      'tenantId',
      'organizationId',
      'spaceId',
      'implementationId',
      'remoteResourceType',
      'remoteResourceId',
      'lifecycleState',
      'capabilitySnapshot',
      'capabilitySnapshotVersion',
      'createdBy',
      'updatedBy',
      'createdAt',
      'updatedAt',
      'version',
    ],
    properties: {
      id: int64StringSchema(),
      uuid: { type: 'string', format: 'uuid' },
      tenantId: int64StringSchema(),
      organizationId: int64StringSchema(),
      spaceId: int64StringSchema(),
      implementationId: { type: 'string', maxLength: 128 },
      remoteResourceType: { type: 'string', maxLength: 64 },
      remoteResourceId: { type: 'string', maxLength: 512 },
      credentialReferenceId: nullableSchema(int64StringSchema()),
      lifecycleState: { $ref: '#/components/schemas/KnowledgeEngineProviderBindingState' },
      capabilitySnapshot: {
        type: 'array',
        items: { $ref: '#/components/schemas/KnowledgeEngineCapability' },
      },
      capabilitySnapshotVersion: int64StringSchema(),
      lastTestedAt: nullableSchema({ type: 'string', format: 'date-time' }),
      activatedAt: nullableSchema({ type: 'string', format: 'date-time' }),
      disabledAt: nullableSchema({ type: 'string', format: 'date-time' }),
      lastErrorCategory: nullableSchema({
        $ref: '#/components/schemas/KnowledgeEngineProviderErrorCategory',
      }),
      createdBy: { type: 'string', maxLength: 128 },
      updatedBy: { type: 'string', maxLength: 128 },
      createdAt: { type: 'string', format: 'date-time' },
      updatedAt: { type: 'string', format: 'date-time' },
      version: int64StringSchema(),
    },
  };
  schemas.KnowledgeEngineProviderMigrationOperation ??= {
    type: 'object',
    additionalProperties: false,
    required: [
      'id',
      'uuid',
      'tenantId',
      'organizationId',
      'spaceId',
      'sourceBindingId',
      'targetBindingId',
      'operationState',
      'requestedBy',
      'attemptCount',
      'createdAt',
      'updatedAt',
      'version',
    ],
    properties: {
      id: int64StringSchema(),
      uuid: { type: 'string', format: 'uuid' },
      tenantId: int64StringSchema(),
      organizationId: int64StringSchema(),
      spaceId: int64StringSchema(),
      sourceBindingId: int64StringSchema(),
      targetBindingId: int64StringSchema(),
      operationState: { $ref: '#/components/schemas/KnowledgeEngineProviderMigrationState' },
      requestedBy: { type: 'string', minLength: 1, maxLength: 128 },
      attemptCount: { type: 'integer', format: 'int32', minimum: 0 },
      cutoverAt: nullableSchema({ type: 'string', format: 'date-time' }),
      observationUntil: nullableSchema({ type: 'string', format: 'date-time' }),
      completedAt: nullableSchema({ type: 'string', format: 'date-time' }),
      lastErrorCategory: nullableSchema({
        $ref: '#/components/schemas/KnowledgeEngineProviderErrorCategory',
      }),
      createdAt: { type: 'string', format: 'date-time' },
      updatedAt: { type: 'string', format: 'date-time' },
      version: int64StringSchema(),
    },
  };
  schemas.KnowledgeEngineProviderCredentialReferencePage ??= listDataSchema(
    '#/components/schemas/KnowledgeEngineProviderCredentialReference',
    'One bounded cursor page of Provider credential references.',
  );
  schemas.KnowledgeEngineProviderBindingPage ??= listDataSchema(
    '#/components/schemas/KnowledgeEngineProviderBinding',
    'One bounded cursor page of Provider bindings.',
  );
  schemas.KnowledgeEngineProviderMigrationOperationPage ??= listDataSchema(
    '#/components/schemas/KnowledgeEngineProviderMigrationOperation',
    'One bounded cursor page of Provider migration operations.',
  );

  const credentialId = pathIdParameter('credentialReferenceId');
  const spaceId = pathIdParameter('spaceId');
  const bindingId = pathIdParameter('bindingId');
  const migrationOperationId = pathIdParameter('migrationOperationId');
  const cursor = queryParameter('cursor', { type: 'string' });
  const pageSize = queryParameter('page_size', {
    type: 'integer',
    format: 'int32',
    minimum: 1,
    maximum: 200,
    default: 20,
  });
  registerProviderOperation(spec, 'get', '/backend/v3/api/knowledge/provider_credential_references', {
    operationId: 'providerCredentialReferences.list',
    summary: 'List Provider credential references',
    parameters: [
      queryParameter('implementation_id', { type: 'string', maxLength: 128 }),
      queryParameter('rotation_state', {
        $ref: '#/components/schemas/KnowledgeEngineProviderCredentialRotationState',
      }),
      cursor,
      pageSize,
    ],
  });
  registerProviderOperation(spec, 'post', '/backend/v3/api/knowledge/provider_credential_references', {
    operationId: 'providerCredentialReferences.create',
    summary: 'Create a Provider credential reference',
    requestSchema: '#/components/schemas/CreateKnowledgeEngineProviderCredentialReferenceRequest',
  });
  registerProviderOperation(
    spec,
    'get',
    '/backend/v3/api/knowledge/provider_credential_references/{credentialReferenceId}',
    {
      operationId: 'providerCredentialReferences.retrieve',
      summary: 'Retrieve a Provider credential reference',
      parameters: [credentialId],
    },
  );
  for (const action of ['rotate', 'revoke']) {
    registerProviderOperation(
      spec,
      'post',
      `/backend/v3/api/knowledge/provider_credential_references/{credentialReferenceId}/${action}`,
      {
        operationId: `providerCredentialReferences.${action}`,
        summary: `${action === 'rotate' ? 'Rotate' : 'Revoke'} a Provider credential reference`,
        parameters: [credentialId],
        requestSchema:
          action === 'rotate'
            ? '#/components/schemas/RotateKnowledgeEngineProviderCredentialReferenceRequest'
            : '#/components/schemas/RevokeKnowledgeEngineProviderCredentialReferenceRequest',
      },
    );
  }
  const bindingCollection = '/backend/v3/api/knowledge/spaces/{spaceId}/provider_bindings';
  const bindingResource = `${bindingCollection}/{bindingId}`;
  registerProviderOperation(spec, 'get', bindingCollection, {
    operationId: 'spaces.providerBindings.list',
    summary: 'List Provider bindings for a knowledge space',
    parameters: [
      spaceId,
      queryParameter('lifecycle_state', {
        $ref: '#/components/schemas/KnowledgeEngineProviderBindingState',
      }),
      cursor,
      pageSize,
    ],
  });
  registerProviderOperation(spec, 'post', bindingCollection, {
    operationId: 'spaces.providerBindings.create',
    summary: 'Create a Provider binding for a knowledge space',
    parameters: [spaceId],
    requestSchema: '#/components/schemas/CreateKnowledgeEngineProviderBindingRequest',
  });
  registerProviderOperation(spec, 'get', bindingResource, {
    operationId: 'spaces.providerBindings.retrieve',
    summary: 'Retrieve a Provider binding',
    parameters: [spaceId, bindingId],
  });
  registerProviderOperation(spec, 'patch', bindingResource, {
    operationId: 'spaces.providerBindings.update',
    summary: 'Update a draft Provider binding',
    parameters: [spaceId, bindingId],
    requestSchema: '#/components/schemas/UpdateKnowledgeEngineProviderBindingRequest',
  });
  for (const action of ['test', 'activate', 'disable']) {
    registerProviderOperation(spec, 'post', `${bindingResource}/${action}`, {
      operationId: `spaces.providerBindings.${action}`,
      summary: `${action[0].toUpperCase()}${action.slice(1)} a Provider binding`,
      parameters: [spaceId, bindingId],
      requestSchema: '#/components/schemas/ProviderBindingVersionCommandRequest',
    });
  }
  const migrationCollection = '/backend/v3/api/knowledge/spaces/{spaceId}/provider_migrations';
  const migrationResource = `${migrationCollection}/{migrationOperationId}`;
  registerProviderOperation(spec, 'get', migrationCollection, {
    operationId: 'spaces.providerMigrations.list',
    summary: 'List Provider migration operations for a knowledge space',
    parameters: [
      spaceId,
      queryParameter('operation_state', {
        $ref: '#/components/schemas/KnowledgeEngineProviderMigrationState',
      }),
      cursor,
      pageSize,
    ],
  });
  registerProviderOperation(spec, 'post', migrationCollection, {
    operationId: 'spaces.providerMigrations.create',
    summary: 'Create a recoverable Provider migration operation',
    parameters: [spaceId],
    requestSchema: '#/components/schemas/CreateKnowledgeEngineProviderMigrationOperationRequest',
  });
  registerProviderOperation(spec, 'get', migrationResource, {
    operationId: 'spaces.providerMigrations.retrieve',
    summary: 'Retrieve a Provider migration operation',
    parameters: [spaceId, migrationOperationId],
  });
  registerProviderOperation(spec, 'post', `${migrationResource}/rollback`, {
    operationId: 'spaces.providerMigrations.rollback',
    summary: 'Request rollback of a Provider migration operation',
    parameters: [spaceId, migrationOperationId],
    requestSchema: '#/components/schemas/ProviderMigrationVersionCommandRequest',
  });
}

function registerProviderOperation(spec, method, routePath, options) {
  spec.paths ??= {};
  spec.paths[routePath] ??= {};
  if (spec.paths[routePath][method]) {
    return;
  }
  const template = spec.paths?.['/backend/v3/api/knowledge/provider_health']?.get;
  if (!template) {
    throw new Error('Provider health operation is required as backend metadata template');
  }
  const responses = Object.fromEntries(
    Object.entries(template.responses ?? {})
      .filter(([status]) => !/^2[0-9][0-9]$/u.test(status))
      .map(([status, response]) => [status, structuredClone(response)]),
  );
  spec.paths[routePath][method] = {
    operationId: options.operationId,
    tags: ['knowledge'],
    summary: options.summary,
    description: options.summary,
    parameters: structuredClone(options.parameters ?? []),
    ...(options.requestSchema
      ? {
          requestBody: {
            required: true,
            content: {
              'application/json': { schema: { $ref: options.requestSchema } },
            },
          },
        }
      : {}),
    responses,
    security: structuredClone(template.security ?? []),
    'x-sdkwork-owner': 'sdkwork-knowledgebase',
    'x-sdkwork-api-authority': 'sdkwork-knowledgebase-backend-api',
    'x-sdkwork-request-context': 'WebRequestContext',
    'x-sdkwork-api-surface': 'backend-api',
    'x-sdkwork-source-route-crate': 'sdkwork-routes-knowledgebase-backend-api',
    'x-sdkwork-auth-mode': 'dual-token',
    'x-sdkwork-rate-limit-tier': method === 'get' ? 'standard' : 'auth-critical',
  };
}

function pathIdParameter(name) {
  return {
    name,
    in: 'path',
    required: true,
    schema: int64StringSchema(),
  };
}

function queryParameter(name, schema) {
  return { name, in: 'query', required: false, schema };
}

function nullableSchema(schema) {
  return { anyOf: [schema, { type: 'null' }] };
}

async function alignFile(filePath, operationAlignments) {
  const spec = JSON.parse(await readFile(filePath, 'utf8'));
  if (filePath === appOpenApiPath) {
    ensureWikiPublicationContracts(spec);
  }
  if (filePath === backendOpenApiPath) {
    ensureProviderManagementContracts(spec);
    ensureAuditComplianceContracts(spec);
  }
  normalizeQueryParameterNames(spec);
  for (const alignment of operationAlignments) {
    applyOperationAlignment(spec, alignment);
  }
  alignOkfPaginationSchemas(spec);
  alignCommandResultSchemas(spec);
  alignBrowserContracts(spec);
  await writeJsonIfChanged(filePath, spec);
}

function ensureAuditComplianceContracts(spec) {
  const schemas = spec.components?.schemas;
  if (!schemas || typeof schemas !== 'object') {
    throw new Error('Backend OpenAPI components.schemas is required');
  }

  for (const schemaName of [
    'ExportKnowledgeAuditEventsRequest',
    'AnonymizeKnowledgeAuditSubjectRequest',
  ]) {
    const actorId = schemas[schemaName]?.properties?.actorId;
    if (!actorId) {
      throw new Error(`Missing Backend OpenAPI ${schemaName}.actorId`);
    }
    schemas[schemaName].properties.actorId = {
      type: 'string',
      minLength: 1,
      maxLength: 128,
      pattern: '\\S',
      description: 'Non-blank IAM subject identifier.',
    };
  }

  const auditEventProperties = schemas.KnowledgeAuditEventItem?.properties;
  if (!auditEventProperties) {
    throw new Error('Missing Backend OpenAPI KnowledgeAuditEventItem properties');
  }
  Object.assign(auditEventProperties, {
    id: { type: 'string', minLength: 1, maxLength: 64 },
    eventType: { type: 'string', minLength: 1, maxLength: 128 },
    actorType: { type: 'string', minLength: 1, maxLength: 64 },
    actorId: { type: 'string', minLength: 1, maxLength: 128 },
    resourceType: { type: 'string', minLength: 1, maxLength: 64 },
    resourceId: nullableSchema(int64StringSchema()),
    result: { type: 'string', minLength: 1, maxLength: 64 },
    traceId: nullableSchema({ type: 'string', maxLength: 128 }),
    createdAt: { type: 'string', format: 'date-time', maxLength: 64 },
  });

  const exportItems = schemas.KnowledgeAuditEventExport?.properties?.items;
  if (!exportItems) {
    throw new Error('Missing Backend OpenAPI KnowledgeAuditEventExport.items');
  }
  exportItems.maxItems = 5_000;
  exportItems.description =
    'Complete synchronous result for the subject. Requests matching more than 5,000 events fail with HTTP 413 and never return a truncated array.';

  const operation =
    spec.paths?.['/backend/v3/api/knowledge/compliance/audit_events/export']?.post;
  if (!operation) {
    throw new Error('Missing Backend OpenAPI compliance audit export operation');
  }
  operation.description =
    'Export at most 5,000 tenant-scoped kb_audit_event rows for a GDPR data-subject request. Results above the synchronous bound fail with HTTP 413 and are never silently truncated.';
  const problemResponse = operation.responses?.['400'];
  if (!problemResponse) {
    throw new Error('Compliance audit export requires the standard 400 problem response template');
  }
  operation.responses['413'] = {
    ...structuredClone(problemResponse),
    description: 'Audit export exceeds the 5,000-event synchronous response bound',
  };
}

function alignOkfPaginationSchemas(spec) {
  const schemas = spec.components?.schemas;
  if (!schemas || typeof schemas !== 'object') {
    return;
  }

  schemas.OkfConceptSummaryList = listDataSchema(
    '#/components/schemas/OkfConceptSummary',
    'One bounded cursor page of published OKF concept summaries.',
  );
  schemas.KnowledgeOkfConceptRevisionList = listDataSchema(
    '#/components/schemas/KnowledgeOkfConceptRevision',
    'One bounded cursor page of OKF concept revisions.',
  );
}

function listDataSchema(itemSchemaRef, description) {
  return {
    type: 'object',
    additionalProperties: false,
    description,
    required: ['items', 'pageInfo'],
    properties: {
      items: {
        type: 'array',
        items: { $ref: itemSchemaRef },
      },
      pageInfo: { $ref: '#/components/schemas/PageInfo' },
    },
  };
}

function alignBrowserContracts(spec) {
  const operation = spec.paths?.['/app/v3/api/knowledge/spaces/{spaceId}/browser']?.get;
  if (!operation) {
    removeStaleBrowserSchemas(spec);
    return;
  }

  const schemas = spec.components?.schemas;
  if (!schemas || typeof schemas !== 'object') {
    return;
  }

  alignBrowserSchemas(schemas);
  alignBrowserOperation(operation);
}

function removeStaleBrowserSchemas(spec) {
  const schemas = spec.components?.schemas;
  if (!schemas || typeof schemas !== 'object') {
    return;
  }
  delete schemas.KnowledgeBrowserListData;
  delete schemas.KnowledgeBrowserPage;
  delete schemas.KnowledgeBrowserView;
}

function alignBrowserOperation(operation) {
  operation.summary = 'List knowledge browser view';
  operation.description = [
    'Lists one cursor page from a bounded knowledge browser view.',
    'For OKF knowledge spaces, view=files resolves to the original raw source file root sources/raw and must not expose the generated okf/ bundle directory tree.',
    'view=okf_bundle resolves to the generated OKF bundle root okf, and view=outputs resolves to output.',
    'When parentId is omitted, the response data.parentId is the resolved Drive folder for the current view root; clients must use it for root uploads and root folder creation.',
    'When parentId is provided, it must stay inside the selected view root.',
  ].join(' ');

  for (const parameter of operation.parameters ?? []) {
    if (!parameter || typeof parameter !== 'object') {
      continue;
    }
    if (parameter.name === 'view') {
      parameter.description =
        'Browser view selector. For OKF spaces, files lists original files under sources/raw; okf_bundle lists generated bundle files under okf; outputs lists generated output files under output.';
    }
    if (parameter.name === 'parentId') {
      parameter.description =
        'Drive node id of a folder inside the selected browser view root. Omit it to resolve the current view root; the resolved folder id is returned as data.parentId.';
    }
    if (parameter.name === 'page_size') {
      parameter.description = 'Maximum number of browser nodes to return. Defaults to 20 and is capped at 200.';
    }
    if (parameter.name === 'cursor') {
      parameter.description = 'Opaque cursor returned by data.pageInfo.nextCursor.';
    }
  }
}

function alignBrowserSchemas(schemas) {
  schemas.KnowledgeBrowserView = {
    type: 'string',
    description:
      'Browser view selector. files lists original source files; for OKF spaces this is sources/raw. okf_bundle lists generated OKF bundle files under okf. outputs lists generated output files under output.',
    enum: ['files', 'okf_bundle', 'outputs'],
  };

  if (schemas.ListKnowledgeBrowserRequest?.properties) {
    schemas.ListKnowledgeBrowserRequest.description =
      'Browser list request. parentId is a Drive folder id within the selected view root; omit parentId to resolve the view root.';
    schemas.ListKnowledgeBrowserRequest.properties.parentId = {
      type: ['string', 'null'],
      maxLength: 128,
      description:
        'Drive folder id within the selected browser view root. For OKF files view, it must be under sources/raw; for okf_bundle, under okf; for outputs, under output.',
    };
    schemas.ListKnowledgeBrowserRequest.properties.view = {
      $ref: '#/components/schemas/KnowledgeBrowserView',
      description:
        'files shows original files, okf_bundle shows generated OKF bundle content, outputs shows generated outputs.',
    };
    if (schemas.ListKnowledgeBrowserRequest.properties.pageSize) {
      schemas.ListKnowledgeBrowserRequest.properties.pageSize.description =
        'JSON request body page size. HTTP GET uses page_size on the wire.';
    }
  }

  schemas.KnowledgeBrowserListData = {
    type: 'object',
    additionalProperties: false,
    description:
      'Standard browser list response data. It follows SDKWork list semantics with items and pageInfo, and also returns the resolved Drive view context needed by clients.',
    required: ['spaceId', 'driveSpaceId', 'view', 'pageSize', 'items', 'pageInfo'],
    properties: {
      spaceId: int64StringSchema(),
      driveSpaceId: {
        type: 'string',
        minLength: 1,
        description: 'Drive space id bound to the knowledge space.',
      },
      parentId: {
        type: ['string', 'null'],
        maxLength: 128,
        description:
          'Resolved Drive folder id for the current browser view page. When request parentId is omitted, this is the view root folder id; OKF files view resolves to sources/raw.',
      },
      view: {
        $ref: '#/components/schemas/KnowledgeBrowserView',
      },
      pageSize: {
        type: 'integer',
        format: 'uint32',
        minimum: 1,
        maximum: 200,
      },
      items: {
        type: 'array',
        items: {
          $ref: '#/components/schemas/KnowledgeBrowserNode',
        },
      },
      pageInfo: {
        $ref: '#/components/schemas/PageInfo',
      },
    },
  };

  delete schemas.KnowledgeBrowserPage;

  const mode = schemas.KnowledgeAgentKnowledgeMode;
  if (mode && Array.isArray(mode.enum) && !mode.enum.includes('external')) {
    mode.enum = [...mode.enum, 'external'];
  }
  if (mode && typeof mode.description !== 'string') {
    mode.description =
      'Knowledge execution mode. okf_bundle uses native OKF bundle knowledge, rag uses native RAG retrieval, and external delegates to an external knowledge engine integration.';
  }
}

function alignCommandResultSchemas(spec) {
  const schemas = spec.components?.schemas;
  if (!schemas || typeof schemas !== 'object') {
    return;
  }

  const replacements = {
    KnowledgeWechatOperationResult: commandResultSchema(),
    KnowledgeGitSyncResult: {
      ...commandResultSchema(),
      required: ['accepted', 'status', 'hash', 'syncedCount'],
      properties: {
        ...commandResultProperties(),
        hash: { type: 'string', minLength: 1 },
        syncedCount: { type: 'integer', format: 'uint32', minimum: 0 },
      },
    },
    KnowledgeMarketSubscriptionResult: commandResultSchema(),
    KnowledgeMediaTaskResult: {
      ...commandResultSchema(),
      required: ['accepted', 'status', 'suggestions', 'similars'],
      properties: {
        ...commandResultProperties(),
        url: { type: ['string', 'null'] },
        resolution: { type: ['string', 'null'] },
        text: { type: ['string', 'null'] },
        suggestions: { type: 'array', items: { type: 'string' } },
        similars: { type: 'array', items: { type: 'string' } },
      },
    },
  };

  for (const [schemaName, schema] of Object.entries(replacements)) {
    if (Object.hasOwn(schemas, schemaName)) {
      schemas[schemaName] = schema;
    }
  }
}

function commandResultSchema() {
  return {
    type: 'object',
    required: ['accepted', 'status'],
    properties: commandResultProperties(),
  };
}

function commandResultProperties() {
  return {
    accepted: { type: 'boolean', const: true },
    status: { type: 'string', enum: ['completed'] },
  };
}

function int64StringSchema() {
  return {
    type: 'string',
    format: 'uint64',
    pattern: '^[0-9]+$',
    'x-sdkwork-int64-string': true,
  };
}

function normalizeQueryParameterNames(spec) {
  for (const pathItem of Object.values(spec.paths ?? {})) {
    if (!pathItem || typeof pathItem !== 'object') {
      continue;
    }
    for (const operation of Object.values(pathItem)) {
      if (!operation || typeof operation !== 'object' || !Array.isArray(operation.parameters)) {
        continue;
      }
      let hasPageSize = false;
      operation.parameters = operation.parameters.filter((parameter) => {
        if (parameter?.in !== 'query') {
          return true;
        }
        if (parameter.name === 'pageSize') {
          parameter.name = 'page_size';
        }
        if (parameter.name !== 'page_size') {
          return true;
        }
        if (hasPageSize) {
          return false;
        }
        hasPageSize = true;
        return true;
      });
    }
  }
}

function applyOperationAlignment(spec, alignment) {
  const operation = spec.paths?.[alignment.routePath]?.[alignment.method];
  if (!operation) {
    throw new Error(
      `Missing OpenAPI operation: ${alignment.method.toUpperCase()} ${alignment.routePath}`,
    );
  }

  operation.operationId = alignment.operationId;
  operation.responses = operation.responses && typeof operation.responses === 'object'
    ? operation.responses
    : {};
  removeSuccessResponses(operation.responses);

  if (alignment.noContent) {
    operation.responses['204'] = { description: 'No Content' };
    return;
  }

  const response = alignment.status === '201'
    ? createdResponse(alignment.schema)
    : jsonResponse(alignment.schema);
  operation.responses[alignment.status] = response;
}

function removeSuccessResponses(responses) {
  for (const status of Object.keys(responses)) {
    if (/^2[0-9][0-9]$/u.test(status)) {
      delete responses[status];
    }
  }
}

async function writeJsonIfChanged(filePath, value) {
  const desired = `${JSON.stringify(value, null, 2)}\n`;
  const current = await readFile(filePath, 'utf8');
  if (current === desired) {
    return;
  }

  const relativePath = path.relative(workspaceRoot, filePath).replaceAll('\\', '/');
  if (checkOnly) {
    pendingChanges.push(relativePath);
    return;
  }

  await writeFile(filePath, desired, 'utf8');
  console.log(`Aligned ${relativePath}`);
}

await alignFile(appOpenApiPath, appOperations);
await alignFile(backendOpenApiPath, backendOperations);

if (checkOnly && pendingChanges.length > 0) {
  console.error(
    JSON.stringify(
      {
        ok: false,
        pendingChanges,
      },
      null,
      2,
    ),
  );
  process.exit(1);
}

console.log(
  JSON.stringify(
    {
      ok: true,
      mode: checkOnly ? 'check' : 'apply',
    },
    null,
    2,
  ),
);
