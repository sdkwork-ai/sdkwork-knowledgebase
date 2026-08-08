import { backendApiPath } from './paths';
import type { ApiRequestOptions, HttpClient } from '../http/client';

import type { AnonymizeKnowledgeAuditSubjectRequest, AnonymizeKnowledgeAuditSubjectResult, CreateKnowledgeEngineProviderBindingRequest, CreateKnowledgeEngineProviderCredentialReferenceRequest, CreateKnowledgeEngineProviderMigrationOperationRequest, CreateKnowledgeSourceRequest, ExportKnowledgeAuditEventsRequest, GroupKnowledgebaseLaunchCapability, IngestionJob, KnowledgeAuditEventExport, KnowledgeEngineProviderBinding, KnowledgeEngineProviderBindingPage, KnowledgeEngineProviderBindingState, KnowledgeEngineProviderCredentialReference, KnowledgeEngineProviderCredentialReferencePage, KnowledgeEngineProviderCredentialRotationState, KnowledgeEngineProviderMigrationOperation, KnowledgeEngineProviderMigrationOperationPage, KnowledgeEngineProviderMigrationState, KnowledgeIndex, KnowledgeIndexRequest, KnowledgeOkfBundleFile, KnowledgeOkfProfileRequest, KnowledgeProviderHealth, KnowledgeRetrievalProfile, KnowledgeRetrievalProfileRequest, KnowledgeRetrievalTrace, KnowledgeSource, KnowledgeSpace, KnowledgeSpaceMember, KnowledgeTenantStatus, OkfBundleExportRequest, OkfBundleImportRequest, OkfBundleImportResult, OkfBundleIndexRebuildRequest, OkfCandidateResult, OkfCandidateReviewRequest, OkfCompileJobRequest, OkfConceptPublishRequest, OkfConceptSummary, OkfIndexDocument, OkfLogEntry, OkfQualityRun, OkfQualityRunRequest, PageInfo, ProviderBindingVersionCommandRequest, ProviderMigrationVersionCommandRequest, RevokeKnowledgeEngineProviderCredentialReferenceRequest, RotateKnowledgeEngineProviderCredentialReferenceRequest, SdkWorkCommandData, UpdateKnowledgeEngineProviderBindingRequest } from '../types';


export class KnowledgeTenantsCurrentApi {
  private client: HttpClient;

  constructor(client: HttpClient) {
    this.client = client;
  }


/** Retrieve current tenant knowledgebase status */
  async list(requestOptions?: ApiRequestOptions): Promise<KnowledgeTenantStatus> {
    return this.client.request<KnowledgeTenantStatus>(backendApiPath(`/knowledge/tenants/current`), { signal: requestOptions?.signal, timeout: requestOptions?.timeout, method: 'GET' as any, sdkworkUnwrapKind: 'item' });
  }
}

export class KnowledgeTenantsApi {
  private client: HttpClient;
  public readonly current: KnowledgeTenantsCurrentApi;

  constructor(client: HttpClient) {
    this.client = client;
    this.current = new KnowledgeTenantsCurrentApi(client);
  }

}

export interface KnowledgeSpacesProviderMigrationsListParams {
  operationState?: KnowledgeEngineProviderMigrationState;
  cursor?: string;
  pageSize?: number;
}

export class KnowledgeSpacesProviderMigrationsApi {
  private client: HttpClient;

  constructor(client: HttpClient) {
    this.client = client;
  }


/** List Provider migration operations for a knowledge space */
  async list(spaceId: string, params?: KnowledgeSpacesProviderMigrationsListParams, requestOptions?: ApiRequestOptions): Promise<KnowledgeEngineProviderMigrationOperationPage> {
    const query = buildQueryString([
      { name: 'operation_state', value: params?.operationState, style: 'form', explode: true, allowReserved: false },
      { name: 'cursor', value: params?.cursor, style: 'form', explode: true, allowReserved: false },
      { name: 'page_size', value: params?.pageSize, style: 'form', explode: true, allowReserved: false },
    ]);
    return this.client.request<KnowledgeEngineProviderMigrationOperationPage>(appendQueryString(backendApiPath(`/knowledge/spaces/${serializePathParameter(spaceId, { name: 'spaceId', style: 'simple', explode: false })}/provider_migrations`), query), { signal: requestOptions?.signal, timeout: requestOptions?.timeout, method: 'GET' as any, sdkworkUnwrapKind: 'page' });
  }

/** Create a recoverable Provider migration operation */
  async create(spaceId: string, body: CreateKnowledgeEngineProviderMigrationOperationRequest, requestOptions?: ApiRequestOptions): Promise<KnowledgeEngineProviderMigrationOperation> {
    return this.client.request<KnowledgeEngineProviderMigrationOperation>(backendApiPath(`/knowledge/spaces/${serializePathParameter(spaceId, { name: 'spaceId', style: 'simple', explode: false })}/provider_migrations`), { signal: requestOptions?.signal, timeout: requestOptions?.timeout, method: 'POST' as any, body, contentType: 'application/json', sdkworkUnwrapKind: 'item' });
  }

/** Retrieve a Provider migration operation */
  async retrieve(spaceId: string, migrationOperationId: string, requestOptions?: ApiRequestOptions): Promise<KnowledgeEngineProviderMigrationOperation> {
    return this.client.request<KnowledgeEngineProviderMigrationOperation>(backendApiPath(`/knowledge/spaces/${serializePathParameter(spaceId, { name: 'spaceId', style: 'simple', explode: false })}/provider_migrations/${serializePathParameter(migrationOperationId, { name: 'migrationOperationId', style: 'simple', explode: false })}`), { signal: requestOptions?.signal, timeout: requestOptions?.timeout, method: 'GET' as any, sdkworkUnwrapKind: 'item' });
  }

/** Request rollback of a Provider migration operation */
  async rollback(spaceId: string, migrationOperationId: string, body: ProviderMigrationVersionCommandRequest, requestOptions?: ApiRequestOptions): Promise<SdkWorkCommandData> {
    return this.client.request<SdkWorkCommandData>(backendApiPath(`/knowledge/spaces/${serializePathParameter(spaceId, { name: 'spaceId', style: 'simple', explode: false })}/provider_migrations/${serializePathParameter(migrationOperationId, { name: 'migrationOperationId', style: 'simple', explode: false })}/rollback`), { signal: requestOptions?.signal, timeout: requestOptions?.timeout, method: 'POST' as any, body, contentType: 'application/json', sdkworkUnwrapKind: 'command' });
  }
}

export interface KnowledgeSpacesProviderBindingsListParams {
  lifecycleState?: KnowledgeEngineProviderBindingState;
  cursor?: string;
  pageSize?: number;
}

export class KnowledgeSpacesProviderBindingsApi {
  private client: HttpClient;

  constructor(client: HttpClient) {
    this.client = client;
  }


/** List Provider bindings for a knowledge space */
  async list(spaceId: string, params?: KnowledgeSpacesProviderBindingsListParams, requestOptions?: ApiRequestOptions): Promise<KnowledgeEngineProviderBindingPage> {
    const query = buildQueryString([
      { name: 'lifecycle_state', value: params?.lifecycleState, style: 'form', explode: true, allowReserved: false },
      { name: 'cursor', value: params?.cursor, style: 'form', explode: true, allowReserved: false },
      { name: 'page_size', value: params?.pageSize, style: 'form', explode: true, allowReserved: false },
    ]);
    return this.client.request<KnowledgeEngineProviderBindingPage>(appendQueryString(backendApiPath(`/knowledge/spaces/${serializePathParameter(spaceId, { name: 'spaceId', style: 'simple', explode: false })}/provider_bindings`), query), { signal: requestOptions?.signal, timeout: requestOptions?.timeout, method: 'GET' as any, sdkworkUnwrapKind: 'page' });
  }

/** Create a Provider binding for a knowledge space */
  async create(spaceId: string, body: CreateKnowledgeEngineProviderBindingRequest, requestOptions?: ApiRequestOptions): Promise<KnowledgeEngineProviderBinding> {
    return this.client.request<KnowledgeEngineProviderBinding>(backendApiPath(`/knowledge/spaces/${serializePathParameter(spaceId, { name: 'spaceId', style: 'simple', explode: false })}/provider_bindings`), { signal: requestOptions?.signal, timeout: requestOptions?.timeout, method: 'POST' as any, body, contentType: 'application/json', sdkworkUnwrapKind: 'item' });
  }

/** Retrieve a Provider binding */
  async retrieve(spaceId: string, bindingId: string, requestOptions?: ApiRequestOptions): Promise<KnowledgeEngineProviderBinding> {
    return this.client.request<KnowledgeEngineProviderBinding>(backendApiPath(`/knowledge/spaces/${serializePathParameter(spaceId, { name: 'spaceId', style: 'simple', explode: false })}/provider_bindings/${serializePathParameter(bindingId, { name: 'bindingId', style: 'simple', explode: false })}`), { signal: requestOptions?.signal, timeout: requestOptions?.timeout, method: 'GET' as any, sdkworkUnwrapKind: 'item' });
  }

/** Update a draft Provider binding */
  async update(spaceId: string, bindingId: string, body: UpdateKnowledgeEngineProviderBindingRequest, requestOptions?: ApiRequestOptions): Promise<KnowledgeEngineProviderBinding> {
    return this.client.request<KnowledgeEngineProviderBinding>(backendApiPath(`/knowledge/spaces/${serializePathParameter(spaceId, { name: 'spaceId', style: 'simple', explode: false })}/provider_bindings/${serializePathParameter(bindingId, { name: 'bindingId', style: 'simple', explode: false })}`), { signal: requestOptions?.signal, timeout: requestOptions?.timeout, method: 'PATCH' as any, body, contentType: 'application/json', sdkworkUnwrapKind: 'item' });
  }

/** Activate a Provider binding */
  async activate(spaceId: string, bindingId: string, body: ProviderBindingVersionCommandRequest, requestOptions?: ApiRequestOptions): Promise<SdkWorkCommandData> {
    return this.client.request<SdkWorkCommandData>(backendApiPath(`/knowledge/spaces/${serializePathParameter(spaceId, { name: 'spaceId', style: 'simple', explode: false })}/provider_bindings/${serializePathParameter(bindingId, { name: 'bindingId', style: 'simple', explode: false })}/activate`), { signal: requestOptions?.signal, timeout: requestOptions?.timeout, method: 'POST' as any, body, contentType: 'application/json', sdkworkUnwrapKind: 'command' });
  }

/** Disable a Provider binding */
  async disable(spaceId: string, bindingId: string, body: ProviderBindingVersionCommandRequest, requestOptions?: ApiRequestOptions): Promise<SdkWorkCommandData> {
    return this.client.request<SdkWorkCommandData>(backendApiPath(`/knowledge/spaces/${serializePathParameter(spaceId, { name: 'spaceId', style: 'simple', explode: false })}/provider_bindings/${serializePathParameter(bindingId, { name: 'bindingId', style: 'simple', explode: false })}/disable`), { signal: requestOptions?.signal, timeout: requestOptions?.timeout, method: 'POST' as any, body, contentType: 'application/json', sdkworkUnwrapKind: 'command' });
  }

/** Test a Provider binding */
  async test(spaceId: string, bindingId: string, body: ProviderBindingVersionCommandRequest, requestOptions?: ApiRequestOptions): Promise<SdkWorkCommandData> {
    return this.client.request<SdkWorkCommandData>(backendApiPath(`/knowledge/spaces/${serializePathParameter(spaceId, { name: 'spaceId', style: 'simple', explode: false })}/provider_bindings/${serializePathParameter(bindingId, { name: 'bindingId', style: 'simple', explode: false })}/test`), { signal: requestOptions?.signal, timeout: requestOptions?.timeout, method: 'POST' as any, body, contentType: 'application/json', sdkworkUnwrapKind: 'command' });
  }
}

export interface KnowledgeSpacesMembersListParams {
  cursor?: string;
  pageSize?: number;
}

export class KnowledgeSpacesMembersApi {
  private client: HttpClient;

  constructor(client: HttpClient) {
    this.client = client;
  }


/** List knowledge space members */
  async list(spaceId: string, params?: KnowledgeSpacesMembersListParams, requestOptions?: ApiRequestOptions): Promise<{ items: KnowledgeSpaceMember[]; pageInfo: PageInfo; }> {
    const query = buildQueryString([
      { name: 'cursor', value: params?.cursor, style: 'form', explode: true, allowReserved: false },
      { name: 'page_size', value: params?.pageSize, style: 'form', explode: true, allowReserved: false },
    ]);
    return this.client.request<{ items: KnowledgeSpaceMember[]; pageInfo: PageInfo; }>(appendQueryString(backendApiPath(`/knowledge/spaces/${serializePathParameter(spaceId, { name: 'spaceId', style: 'simple', explode: false })}/members`), query), { signal: requestOptions?.signal, timeout: requestOptions?.timeout, method: 'GET' as any, sdkworkUnwrapKind: 'page' });
  }
}

export interface KnowledgeSpacesListParams {
  cursor?: string;
  pageSize?: number;
}

export class KnowledgeSpacesApi {
  private client: HttpClient;
  public readonly members: KnowledgeSpacesMembersApi;
  public readonly providerBindings: KnowledgeSpacesProviderBindingsApi;
  public readonly providerMigrations: KnowledgeSpacesProviderMigrationsApi;

  constructor(client: HttpClient) {
    this.client = client;
    this.members = new KnowledgeSpacesMembersApi(client);
    this.providerBindings = new KnowledgeSpacesProviderBindingsApi(client);
    this.providerMigrations = new KnowledgeSpacesProviderMigrationsApi(client);
  }


/** List knowledge spaces */
  async list(params?: KnowledgeSpacesListParams, requestOptions?: ApiRequestOptions): Promise<{ items: KnowledgeSpace[]; pageInfo: PageInfo; }> {
    const query = buildQueryString([
      { name: 'cursor', value: params?.cursor, style: 'form', explode: true, allowReserved: false },
      { name: 'page_size', value: params?.pageSize, style: 'form', explode: true, allowReserved: false },
    ]);
    return this.client.request<{ items: KnowledgeSpace[]; pageInfo: PageInfo; }>(appendQueryString(backendApiPath(`/knowledge/spaces`), query), { signal: requestOptions?.signal, timeout: requestOptions?.timeout, method: 'GET' as any, sdkworkUnwrapKind: 'page' });
  }
}

export interface KnowledgeSourcesListParams {
  cursor?: string;
  pageSize?: number;
}

export class KnowledgeSourcesApi {
  private client: HttpClient;

  constructor(client: HttpClient) {
    this.client = client;
  }


/** List knowledge sources */
  async list(params?: KnowledgeSourcesListParams, requestOptions?: ApiRequestOptions): Promise<{ items: KnowledgeSource[]; pageInfo: PageInfo; }> {
    const query = buildQueryString([
      { name: 'cursor', value: params?.cursor, style: 'form', explode: true, allowReserved: false },
      { name: 'page_size', value: params?.pageSize, style: 'form', explode: true, allowReserved: false },
    ]);
    return this.client.request<{ items: KnowledgeSource[]; pageInfo: PageInfo; }>(appendQueryString(backendApiPath(`/knowledge/sources`), query), { signal: requestOptions?.signal, timeout: requestOptions?.timeout, method: 'GET' as any, sdkworkUnwrapKind: 'page' });
  }

/** Create a knowledge source */
  async create(body: CreateKnowledgeSourceRequest, requestOptions?: ApiRequestOptions): Promise<KnowledgeSource> {
    return this.client.request<KnowledgeSource>(backendApiPath(`/knowledge/sources`), { signal: requestOptions?.signal, timeout: requestOptions?.timeout, method: 'POST' as any, body, contentType: 'application/json', sdkworkUnwrapKind: 'item' });
  }
}

export interface KnowledgeRetrievalTracesListParams {
  cursor?: string;
  pageSize?: number;
}

export class KnowledgeRetrievalTracesApi {
  private client: HttpClient;

  constructor(client: HttpClient) {
    this.client = client;
  }


/** List retrieval traces */
  async list(params?: KnowledgeRetrievalTracesListParams, requestOptions?: ApiRequestOptions): Promise<{ items: KnowledgeRetrievalTrace[]; pageInfo: PageInfo; }> {
    const query = buildQueryString([
      { name: 'cursor', value: params?.cursor, style: 'form', explode: true, allowReserved: false },
      { name: 'page_size', value: params?.pageSize, style: 'form', explode: true, allowReserved: false },
    ]);
    return this.client.request<{ items: KnowledgeRetrievalTrace[]; pageInfo: PageInfo; }>(appendQueryString(backendApiPath(`/knowledge/retrieval_traces`), query), { signal: requestOptions?.signal, timeout: requestOptions?.timeout, method: 'GET' as any, sdkworkUnwrapKind: 'page' });
  }

/** Retrieve a retrieval trace */
  async retrieve(traceId: string, requestOptions?: ApiRequestOptions): Promise<KnowledgeRetrievalTrace> {
    return this.client.request<KnowledgeRetrievalTrace>(backendApiPath(`/knowledge/retrieval_traces/${serializePathParameter(traceId, { name: 'traceId', style: 'simple', explode: false })}`), { signal: requestOptions?.signal, timeout: requestOptions?.timeout, method: 'GET' as any, sdkworkUnwrapKind: 'item' });
  }
}

export class KnowledgeRetrievalProfilesApi {
  private client: HttpClient;

  constructor(client: HttpClient) {
    this.client = client;
  }


/** Create a retrieval profile */
  async create(body: KnowledgeRetrievalProfileRequest, requestOptions?: ApiRequestOptions): Promise<KnowledgeRetrievalProfile> {
    return this.client.request<KnowledgeRetrievalProfile>(backendApiPath(`/knowledge/retrieval_profiles`), { signal: requestOptions?.signal, timeout: requestOptions?.timeout, method: 'POST' as any, body, contentType: 'application/json', sdkworkUnwrapKind: 'item' });
  }

/** Retrieve a retrieval profile */
  async retrieve(profileId: string, requestOptions?: ApiRequestOptions): Promise<KnowledgeRetrievalProfile> {
    return this.client.request<KnowledgeRetrievalProfile>(backendApiPath(`/knowledge/retrieval_profiles/${serializePathParameter(profileId, { name: 'profileId', style: 'simple', explode: false })}`), { signal: requestOptions?.signal, timeout: requestOptions?.timeout, method: 'GET' as any, sdkworkUnwrapKind: 'item' });
  }

/** Update a retrieval profile */
  async update(profileId: string, body: KnowledgeRetrievalProfileRequest, requestOptions?: ApiRequestOptions): Promise<KnowledgeRetrievalProfile> {
    return this.client.request<KnowledgeRetrievalProfile>(backendApiPath(`/knowledge/retrieval_profiles/${serializePathParameter(profileId, { name: 'profileId', style: 'simple', explode: false })}`), { signal: requestOptions?.signal, timeout: requestOptions?.timeout, method: 'PATCH' as any, body, contentType: 'application/json', sdkworkUnwrapKind: 'item' });
  }
}

export class KnowledgeProviderHealthApi {
  private client: HttpClient;

  constructor(client: HttpClient) {
    this.client = client;
  }


/** Retrieve provider health status */
  async list(requestOptions?: ApiRequestOptions): Promise<KnowledgeProviderHealth> {
    return this.client.request<KnowledgeProviderHealth>(backendApiPath(`/knowledge/provider_health`), { signal: requestOptions?.signal, timeout: requestOptions?.timeout, method: 'GET' as any, sdkworkUnwrapKind: 'item' });
  }
}

export interface KnowledgeProviderCredentialReferencesListParams {
  implementationId?: string;
  rotationState?: KnowledgeEngineProviderCredentialRotationState;
  cursor?: string;
  pageSize?: number;
}

export class KnowledgeProviderCredentialReferencesApi {
  private client: HttpClient;

  constructor(client: HttpClient) {
    this.client = client;
  }


/** List Provider credential references */
  async list(params?: KnowledgeProviderCredentialReferencesListParams, requestOptions?: ApiRequestOptions): Promise<KnowledgeEngineProviderCredentialReferencePage> {
    const query = buildQueryString([
      { name: 'implementation_id', value: params?.implementationId, style: 'form', explode: true, allowReserved: false },
      { name: 'rotation_state', value: params?.rotationState, style: 'form', explode: true, allowReserved: false },
      { name: 'cursor', value: params?.cursor, style: 'form', explode: true, allowReserved: false },
      { name: 'page_size', value: params?.pageSize, style: 'form', explode: true, allowReserved: false },
    ]);
    return this.client.request<KnowledgeEngineProviderCredentialReferencePage>(appendQueryString(backendApiPath(`/knowledge/provider_credential_references`), query), { signal: requestOptions?.signal, timeout: requestOptions?.timeout, method: 'GET' as any, sdkworkUnwrapKind: 'page' });
  }

/** Create a Provider credential reference */
  async create(body: CreateKnowledgeEngineProviderCredentialReferenceRequest, requestOptions?: ApiRequestOptions): Promise<KnowledgeEngineProviderCredentialReference> {
    return this.client.request<KnowledgeEngineProviderCredentialReference>(backendApiPath(`/knowledge/provider_credential_references`), { signal: requestOptions?.signal, timeout: requestOptions?.timeout, method: 'POST' as any, body, contentType: 'application/json', sdkworkUnwrapKind: 'item' });
  }

/** Retrieve a Provider credential reference */
  async retrieve(credentialReferenceId: string, requestOptions?: ApiRequestOptions): Promise<KnowledgeEngineProviderCredentialReference> {
    return this.client.request<KnowledgeEngineProviderCredentialReference>(backendApiPath(`/knowledge/provider_credential_references/${serializePathParameter(credentialReferenceId, { name: 'credentialReferenceId', style: 'simple', explode: false })}`), { signal: requestOptions?.signal, timeout: requestOptions?.timeout, method: 'GET' as any, sdkworkUnwrapKind: 'item' });
  }

/** Revoke a Provider credential reference */
  async revoke(credentialReferenceId: string, body: RevokeKnowledgeEngineProviderCredentialReferenceRequest, requestOptions?: ApiRequestOptions): Promise<SdkWorkCommandData> {
    return this.client.request<SdkWorkCommandData>(backendApiPath(`/knowledge/provider_credential_references/${serializePathParameter(credentialReferenceId, { name: 'credentialReferenceId', style: 'simple', explode: false })}/revoke`), { signal: requestOptions?.signal, timeout: requestOptions?.timeout, method: 'POST' as any, body, contentType: 'application/json', sdkworkUnwrapKind: 'command' });
  }

/** Rotate a Provider credential reference */
  async rotate(credentialReferenceId: string, body: RotateKnowledgeEngineProviderCredentialReferenceRequest, requestOptions?: ApiRequestOptions): Promise<SdkWorkCommandData> {
    return this.client.request<SdkWorkCommandData>(backendApiPath(`/knowledge/provider_credential_references/${serializePathParameter(credentialReferenceId, { name: 'credentialReferenceId', style: 'simple', explode: false })}/rotate`), { signal: requestOptions?.signal, timeout: requestOptions?.timeout, method: 'POST' as any, body, contentType: 'application/json', sdkworkUnwrapKind: 'command' });
  }
}

export class KnowledgeOkfProfileApi {
  private client: HttpClient;

  constructor(client: HttpClient) {
    this.client = client;
  }


/** Create an OKF profile */
  async create(body: KnowledgeOkfProfileRequest, requestOptions?: ApiRequestOptions): Promise<KnowledgeOkfBundleFile> {
    return this.client.request<KnowledgeOkfBundleFile>(backendApiPath(`/knowledge/okf/profile`), { signal: requestOptions?.signal, timeout: requestOptions?.timeout, method: 'POST' as any, body, contentType: 'application/json', sdkworkUnwrapKind: 'item' });
  }

/** Update an OKF profile */
  async update(profileId: number, body: KnowledgeOkfProfileRequest, requestOptions?: ApiRequestOptions): Promise<KnowledgeOkfBundleFile> {
    return this.client.request<KnowledgeOkfBundleFile>(backendApiPath(`/knowledge/okf/profile/${serializePathParameter(profileId, { name: 'profileId', style: 'simple', explode: false })}`), { signal: requestOptions?.signal, timeout: requestOptions?.timeout, method: 'PATCH' as any, body, contentType: 'application/json', sdkworkUnwrapKind: 'item' });
  }
}

export class KnowledgeOkfLogEntriesApi {
  private client: HttpClient;

  constructor(client: HttpClient) {
    this.client = client;
  }


/** Create an OKF log entry */
  async create(body: OkfLogEntry, requestOptions?: ApiRequestOptions): Promise<OkfLogEntry> {
    return this.client.request<OkfLogEntry>(backendApiPath(`/knowledge/okf/log_entries`), { signal: requestOptions?.signal, timeout: requestOptions?.timeout, method: 'POST' as any, body, contentType: 'application/json', sdkworkUnwrapKind: 'item' });
  }
}

export class KnowledgeOkfLogApi {
  private client: HttpClient;
  public readonly entries: KnowledgeOkfLogEntriesApi;

  constructor(client: HttpClient) {
    this.client = client;
    this.entries = new KnowledgeOkfLogEntriesApi(client);
  }

}

export class KnowledgeOkfLintRunsApi {
  private client: HttpClient;

  constructor(client: HttpClient) {
    this.client = client;
  }


/** Create an OKF lint run */
  async create(body: OkfQualityRunRequest, requestOptions?: ApiRequestOptions): Promise<OkfQualityRun> {
    return this.client.request<OkfQualityRun>(backendApiPath(`/knowledge/okf/lint_runs`), { signal: requestOptions?.signal, timeout: requestOptions?.timeout, method: 'POST' as any, body, contentType: 'application/json', sdkworkUnwrapKind: 'item' });
  }
}

export class KnowledgeOkfEvalRunsApi {
  private client: HttpClient;

  constructor(client: HttpClient) {
    this.client = client;
  }


/** Create an OKF eval run */
  async create(body: OkfQualityRunRequest, requestOptions?: ApiRequestOptions): Promise<OkfQualityRun> {
    return this.client.request<OkfQualityRun>(backendApiPath(`/knowledge/okf/eval_runs`), { signal: requestOptions?.signal, timeout: requestOptions?.timeout, method: 'POST' as any, body, contentType: 'application/json', sdkworkUnwrapKind: 'item' });
  }
}

export class KnowledgeOkfConceptsApi {
  private client: HttpClient;

  constructor(client: HttpClient) {
    this.client = client;
  }


/** Publish an OKF concept */
  async publish(conceptId: number, body: OkfConceptPublishRequest, requestOptions?: ApiRequestOptions): Promise<OkfConceptSummary> {
    return this.client.request<OkfConceptSummary>(backendApiPath(`/knowledge/okf/concepts/${serializePathParameter(conceptId, { name: 'conceptId', style: 'simple', explode: false })}/publish`), { signal: requestOptions?.signal, timeout: requestOptions?.timeout, method: 'POST' as any, body, contentType: 'application/json', sdkworkUnwrapKind: 'item' });
  }
}

export class KnowledgeOkfCompileJobsApi {
  private client: HttpClient;

  constructor(client: HttpClient) {
    this.client = client;
  }


/** Create an OKF compile job */
  async create(body: OkfCompileJobRequest, requestOptions?: ApiRequestOptions): Promise<IngestionJob> {
    return this.client.request<IngestionJob>(backendApiPath(`/knowledge/okf/compile_jobs`), { signal: requestOptions?.signal, timeout: requestOptions?.timeout, method: 'POST' as any, body, contentType: 'application/json', sdkworkUnwrapKind: 'item' });
  }
}

export interface KnowledgeOkfCandidatesListParams {
  spaceId: number;
  cursor?: string;
  pageSize?: number;
}

export class KnowledgeOkfCandidatesApi {
  private client: HttpClient;

  constructor(client: HttpClient) {
    this.client = client;
  }


/** List OKF candidates */
  async list(params: KnowledgeOkfCandidatesListParams, requestOptions?: ApiRequestOptions): Promise<{ items: OkfCandidateResult[]; pageInfo: PageInfo; }> {
    const query = buildQueryString([
      { name: 'spaceId', value: params.spaceId, style: 'form', explode: true, allowReserved: false },
      { name: 'cursor', value: params.cursor, style: 'form', explode: true, allowReserved: false },
      { name: 'page_size', value: params.pageSize, style: 'form', explode: true, allowReserved: false },
    ]);
    return this.client.request<{ items: OkfCandidateResult[]; pageInfo: PageInfo; }>(appendQueryString(backendApiPath(`/knowledge/okf/candidates`), query), { signal: requestOptions?.signal, timeout: requestOptions?.timeout, method: 'GET' as any, sdkworkUnwrapKind: 'page' });
  }

/** Approve an OKF candidate */
  async approve(candidateId: number, body: OkfCandidateReviewRequest, requestOptions?: ApiRequestOptions): Promise<OkfCandidateResult> {
    return this.client.request<OkfCandidateResult>(backendApiPath(`/knowledge/okf/candidates/${serializePathParameter(candidateId, { name: 'candidateId', style: 'simple', explode: false })}/approve`), { signal: requestOptions?.signal, timeout: requestOptions?.timeout, method: 'POST' as any, body, contentType: 'application/json', sdkworkUnwrapKind: 'item' });
  }

/** Reject an OKF candidate */
  async reject(candidateId: number, body: OkfCandidateReviewRequest, requestOptions?: ApiRequestOptions): Promise<OkfCandidateResult> {
    return this.client.request<OkfCandidateResult>(backendApiPath(`/knowledge/okf/candidates/${serializePathParameter(candidateId, { name: 'candidateId', style: 'simple', explode: false })}/reject`), { signal: requestOptions?.signal, timeout: requestOptions?.timeout, method: 'POST' as any, body, contentType: 'application/json', sdkworkUnwrapKind: 'item' });
  }
}

export class KnowledgeOkfBundleIndexApi {
  private client: HttpClient;

  constructor(client: HttpClient) {
    this.client = client;
  }


/** Rebuild the OKF bundle index */
  async rebuild(body: OkfBundleIndexRebuildRequest, requestOptions?: ApiRequestOptions): Promise<OkfIndexDocument> {
    return this.client.request<OkfIndexDocument>(backendApiPath(`/knowledge/okf/index/rebuild`), { signal: requestOptions?.signal, timeout: requestOptions?.timeout, method: 'POST' as any, body, contentType: 'application/json', sdkworkUnwrapKind: 'item' });
  }
}

export class KnowledgeOkfBundleImportApi {
  private client: HttpClient;

  constructor(client: HttpClient) {
    this.client = client;
  }


/** Import an OKF bundle from drive staging */
  async create(body: OkfBundleImportRequest, requestOptions?: ApiRequestOptions): Promise<OkfBundleImportResult> {
    return this.client.request<OkfBundleImportResult>(backendApiPath(`/knowledge/okf/imports`), { signal: requestOptions?.signal, timeout: requestOptions?.timeout, method: 'POST' as any, body, contentType: 'application/json', sdkworkUnwrapKind: 'item' });
  }
}

export class KnowledgeOkfBundleExportApi {
  private client: HttpClient;

  constructor(client: HttpClient) {
    this.client = client;
  }


/** Create an OKF bundle export */
  async create(body: OkfBundleExportRequest, requestOptions?: ApiRequestOptions): Promise<KnowledgeOkfBundleFile> {
    return this.client.request<KnowledgeOkfBundleFile>(backendApiPath(`/knowledge/okf/exports`), { signal: requestOptions?.signal, timeout: requestOptions?.timeout, method: 'POST' as any, body, contentType: 'application/json', sdkworkUnwrapKind: 'item' });
  }

/** Retrieve an OKF bundle export */
  async retrieve(exportId: number, requestOptions?: ApiRequestOptions): Promise<KnowledgeOkfBundleFile> {
    return this.client.request<KnowledgeOkfBundleFile>(backendApiPath(`/knowledge/okf/exports/${serializePathParameter(exportId, { name: 'exportId', style: 'simple', explode: false })}`), { signal: requestOptions?.signal, timeout: requestOptions?.timeout, method: 'GET' as any, sdkworkUnwrapKind: 'item' });
  }
}

export interface KnowledgeOkfBundleFilesListParams {
  cursor?: string;
  pageSize?: number;
}

export class KnowledgeOkfBundleFilesApi {
  private client: HttpClient;

  constructor(client: HttpClient) {
    this.client = client;
  }


/** List OKF bundle files */
  async list(params?: KnowledgeOkfBundleFilesListParams, requestOptions?: ApiRequestOptions): Promise<{ items: KnowledgeOkfBundleFile[]; pageInfo: PageInfo; }> {
    const query = buildQueryString([
      { name: 'cursor', value: params?.cursor, style: 'form', explode: true, allowReserved: false },
      { name: 'page_size', value: params?.pageSize, style: 'form', explode: true, allowReserved: false },
    ]);
    return this.client.request<{ items: KnowledgeOkfBundleFile[]; pageInfo: PageInfo; }>(appendQueryString(backendApiPath(`/knowledge/okf/bundle/files`), query), { signal: requestOptions?.signal, timeout: requestOptions?.timeout, method: 'GET' as any, sdkworkUnwrapKind: 'page' });
  }
}

export class KnowledgeOkfBundleApi {
  private client: HttpClient;
  public readonly files: KnowledgeOkfBundleFilesApi;
  public readonly export: KnowledgeOkfBundleExportApi;
  public readonly import: KnowledgeOkfBundleImportApi;
  public readonly index: KnowledgeOkfBundleIndexApi;

  constructor(client: HttpClient) {
    this.client = client;
    this.files = new KnowledgeOkfBundleFilesApi(client);
    this.export = new KnowledgeOkfBundleExportApi(client);
    this.import = new KnowledgeOkfBundleImportApi(client);
    this.index = new KnowledgeOkfBundleIndexApi(client);
  }

}

export class KnowledgeOkfApi {
  private client: HttpClient;
  public readonly bundle: KnowledgeOkfBundleApi;
  public readonly candidates: KnowledgeOkfCandidatesApi;
  public readonly compileJobs: KnowledgeOkfCompileJobsApi;
  public readonly concepts: KnowledgeOkfConceptsApi;
  public readonly evalRuns: KnowledgeOkfEvalRunsApi;
  public readonly lintRuns: KnowledgeOkfLintRunsApi;
  public readonly log: KnowledgeOkfLogApi;
  public readonly profile: KnowledgeOkfProfileApi;

  constructor(client: HttpClient) {
    this.client = client;
    this.bundle = new KnowledgeOkfBundleApi(client);
    this.candidates = new KnowledgeOkfCandidatesApi(client);
    this.compileJobs = new KnowledgeOkfCompileJobsApi(client);
    this.concepts = new KnowledgeOkfConceptsApi(client);
    this.evalRuns = new KnowledgeOkfEvalRunsApi(client);
    this.lintRuns = new KnowledgeOkfLintRunsApi(client);
    this.log = new KnowledgeOkfLogApi(client);
    this.profile = new KnowledgeOkfProfileApi(client);
  }

}

export interface KnowledgeIndexesListParams {
  cursor?: string;
  pageSize?: number;
}

export class KnowledgeIndexesApi {
  private client: HttpClient;

  constructor(client: HttpClient) {
    this.client = client;
  }


/** Create a knowledge index */
  async create(body: KnowledgeIndexRequest, requestOptions?: ApiRequestOptions): Promise<KnowledgeIndex> {
    return this.client.request<KnowledgeIndex>(backendApiPath(`/knowledge/indexes`), { signal: requestOptions?.signal, timeout: requestOptions?.timeout, method: 'POST' as any, body, contentType: 'application/json', sdkworkUnwrapKind: 'item' });
  }

/** List knowledge indexes */
  async list(params?: KnowledgeIndexesListParams, requestOptions?: ApiRequestOptions): Promise<{ items: KnowledgeIndex[]; pageInfo: PageInfo; }> {
    const query = buildQueryString([
      { name: 'cursor', value: params?.cursor, style: 'form', explode: true, allowReserved: false },
      { name: 'page_size', value: params?.pageSize, style: 'form', explode: true, allowReserved: false },
    ]);
    return this.client.request<{ items: KnowledgeIndex[]; pageInfo: PageInfo; }>(appendQueryString(backendApiPath(`/knowledge/indexes`), query), { signal: requestOptions?.signal, timeout: requestOptions?.timeout, method: 'GET' as any, sdkworkUnwrapKind: 'page' });
  }

/** Retrieve a knowledge index */
  async retrieve(indexId: string, requestOptions?: ApiRequestOptions): Promise<KnowledgeIndex> {
    return this.client.request<KnowledgeIndex>(backendApiPath(`/knowledge/indexes/${serializePathParameter(indexId, { name: 'indexId', style: 'simple', explode: false })}`), { signal: requestOptions?.signal, timeout: requestOptions?.timeout, method: 'GET' as any, sdkworkUnwrapKind: 'item' });
  }

/** Rebuild a knowledge index */
  async rebuild(indexId: string, body: OkfBundleIndexRebuildRequest, requestOptions?: ApiRequestOptions): Promise<OkfIndexDocument> {
    return this.client.request<OkfIndexDocument>(backendApiPath(`/knowledge/indexes/${serializePathParameter(indexId, { name: 'indexId', style: 'simple', explode: false })}/rebuild`), { signal: requestOptions?.signal, timeout: requestOptions?.timeout, method: 'POST' as any, body, contentType: 'application/json', sdkworkUnwrapKind: 'item' });
  }
}

export class KnowledgeGroupLaunchApi {
  private client: HttpClient;

  constructor(client: HttpClient) {
    this.client = client;
  }


/** Retrieve the group knowledgebase launch capability state for the runtime deployment */
  async capability(requestOptions?: ApiRequestOptions): Promise<GroupKnowledgebaseLaunchCapability> {
    return this.client.request<GroupKnowledgebaseLaunchCapability>(backendApiPath(`/knowledge/group_launch_capability`), { signal: requestOptions?.signal, timeout: requestOptions?.timeout, method: 'GET' as any, sdkworkUnwrapKind: 'item' });
  }
}

export class KnowledgeGroupApi {
  private client: HttpClient;
  public readonly launch: KnowledgeGroupLaunchApi;

  constructor(client: HttpClient) {
    this.client = client;
    this.launch = new KnowledgeGroupLaunchApi(client);
  }

}

export class KnowledgeComplianceAuditEventsExportApi {
  private client: HttpClient;

  constructor(client: HttpClient) {
    this.client = client;
  }


/** Export knowledge audit events for a subject */
  async create(body: ExportKnowledgeAuditEventsRequest, requestOptions?: ApiRequestOptions): Promise<KnowledgeAuditEventExport> {
    return this.client.request<KnowledgeAuditEventExport>(backendApiPath(`/knowledge/compliance/audit_events/export`), { signal: requestOptions?.signal, timeout: requestOptions?.timeout, method: 'POST' as any, body, contentType: 'application/json', sdkworkUnwrapKind: 'item' });
  }
}

export class KnowledgeComplianceAuditEventsAnonymizeActorApi {
  private client: HttpClient;

  constructor(client: HttpClient) {
    this.client = client;
  }


/** Anonymize audit events for a subject */
  async create(body: AnonymizeKnowledgeAuditSubjectRequest, requestOptions?: ApiRequestOptions): Promise<AnonymizeKnowledgeAuditSubjectResult> {
    return this.client.request<AnonymizeKnowledgeAuditSubjectResult>(backendApiPath(`/knowledge/compliance/audit_events/anonymize_actor`), { signal: requestOptions?.signal, timeout: requestOptions?.timeout, method: 'POST' as any, body, contentType: 'application/json', sdkworkUnwrapKind: 'item' });
  }
}

export class KnowledgeComplianceAuditEventsApi {
  private client: HttpClient;
  public readonly anonymizeActor: KnowledgeComplianceAuditEventsAnonymizeActorApi;
  public readonly export: KnowledgeComplianceAuditEventsExportApi;

  constructor(client: HttpClient) {
    this.client = client;
    this.anonymizeActor = new KnowledgeComplianceAuditEventsAnonymizeActorApi(client);
    this.export = new KnowledgeComplianceAuditEventsExportApi(client);
  }

}

export class KnowledgeComplianceApi {
  private client: HttpClient;
  public readonly auditEvents: KnowledgeComplianceAuditEventsApi;

  constructor(client: HttpClient) {
    this.client = client;
    this.auditEvents = new KnowledgeComplianceAuditEventsApi(client);
  }

}

export class KnowledgeApi {
  private client: HttpClient;
  public readonly compliance: KnowledgeComplianceApi;
  public readonly group: KnowledgeGroupApi;
  public readonly indexes: KnowledgeIndexesApi;
  public readonly okf: KnowledgeOkfApi;
  public readonly providerCredentialReferences: KnowledgeProviderCredentialReferencesApi;
  public readonly providerHealth: KnowledgeProviderHealthApi;
  public readonly retrievalProfiles: KnowledgeRetrievalProfilesApi;
  public readonly retrievalTraces: KnowledgeRetrievalTracesApi;
  public readonly sources: KnowledgeSourcesApi;
  public readonly spaces: KnowledgeSpacesApi;
  public readonly tenants: KnowledgeTenantsApi;

  constructor(client: HttpClient) {
    this.client = client;
    this.compliance = new KnowledgeComplianceApi(client);
    this.group = new KnowledgeGroupApi(client);
    this.indexes = new KnowledgeIndexesApi(client);
    this.okf = new KnowledgeOkfApi(client);
    this.providerCredentialReferences = new KnowledgeProviderCredentialReferencesApi(client);
    this.providerHealth = new KnowledgeProviderHealthApi(client);
    this.retrievalProfiles = new KnowledgeRetrievalProfilesApi(client);
    this.retrievalTraces = new KnowledgeRetrievalTracesApi(client);
    this.sources = new KnowledgeSourcesApi(client);
    this.spaces = new KnowledgeSpacesApi(client);
    this.tenants = new KnowledgeTenantsApi(client);
  }

}

export function createKnowledgeApi(client: HttpClient): KnowledgeApi {
  return new KnowledgeApi(client);
}

function appendQueryString(path: string, rawQueryString: string): string {
  const query = rawQueryString.replace(/^\?+/, '');
  if (!query) {
    return path;
  }
  return path.includes('?') ? `${path}&${query}` : `${path}?${query}`;
}

interface PathParameterSpec {
  name: string;
  style: string;
  explode: boolean;
}

function serializePathParameter(value: unknown, spec: PathParameterSpec): string {
  if (value === undefined || value === null) {
    return '';
  }

  const style = spec.style || 'simple';
  if (Array.isArray(value)) {
    return serializePathArray(spec.name, value, style, spec.explode);
  }
  if (typeof value === 'object') {
    return serializePathObject(spec.name, value as Record<string, unknown>, style, spec.explode);
  }
  return pathPrefix(spec.name, style, false) + encodePathValue(serializePathPrimitive(value));
}

function serializePathArray(name: string, values: unknown[], style: string, explode: boolean): string {
  const serialized = values
    .filter((item) => item !== undefined && item !== null)
    .map((item) => encodePathValue(serializePathPrimitive(item)));
  if (serialized.length === 0) {
    return pathPrefix(name, style, false);
  }
  if (style === 'matrix') {
    return explode
      ? serialized.map((item) => `;${name}=${item}`).join('')
      : `;${name}=${serialized.join(',')}`;
  }
  return pathPrefix(name, style, false) + serialized.join(explode ? '.' : ',');
}

function serializePathObject(name: string, value: Record<string, unknown>, style: string, explode: boolean): string {
  const entries = Object.entries(value).filter(([, entryValue]) => entryValue !== undefined && entryValue !== null);
  if (entries.length === 0) {
    return pathPrefix(name, style, true);
  }
  if (style === 'matrix') {
    return explode
      ? entries.map(([key, entryValue]) => `;${encodePathValue(key)}=${encodePathValue(serializePathPrimitive(entryValue))}`).join('')
      : `;${name}=${entries.flatMap(([key, entryValue]) => [encodePathValue(key), encodePathValue(serializePathPrimitive(entryValue))]).join(',')}`;
  }
  const serialized = explode
    ? entries.map(([key, entryValue]) => `${encodePathValue(key)}=${encodePathValue(serializePathPrimitive(entryValue))}`).join(style === 'label' ? '.' : ',')
    : entries.flatMap(([key, entryValue]) => [encodePathValue(key), encodePathValue(serializePathPrimitive(entryValue))]).join(',');
  return pathPrefix(name, style, true) + serialized;
}

function pathPrefix(name: string, style: string, _objectValue: boolean): string {
  if (style === 'label') return '.';
  if (style === 'matrix') return `;${name}`;
  return '';
}

function encodePathValue(value: string): string {
  return encodeURIComponent(value);
}

function serializePathPrimitive(value: unknown): string {
  if (value instanceof Date) {
    return value.toISOString();
  }
  if (typeof value === 'object') {
    return JSON.stringify(value);
  }
  return String(value);
}
interface QueryParameterSpec {
  name: string;
  value: unknown;
  style: string;
  explode: boolean;
  allowReserved: boolean;
  contentType?: string;
}

function buildQueryString(parameters: QueryParameterSpec[]): string {
  const pairs: string[] = [];
  for (const parameter of parameters) {
    appendSerializedParameter(pairs, parameter);
  }
  return pairs.join('&');
}

function appendSerializedParameter(pairs: string[], parameter: QueryParameterSpec): void {
  if (parameter.value === undefined || parameter.value === null) {
    return;
  }

  if (parameter.contentType) {
    pairs.push(`${encodeQueryComponent(parameter.name)}=${encodeQueryValue(JSON.stringify(parameter.value), parameter.allowReserved)}`);
    return;
  }

  const style = parameter.style || 'form';
  if (style === 'deepObject') {
    appendDeepObjectParameter(pairs, parameter.name, parameter.value, parameter.allowReserved);
    return;
  }

  if (Array.isArray(parameter.value)) {
    appendArrayParameter(pairs, parameter.name, parameter.value, style, parameter.explode, parameter.allowReserved);
    return;
  }

  if (typeof parameter.value === 'object') {
    appendObjectParameter(pairs, parameter.name, parameter.value as Record<string, unknown>, style, parameter.explode, parameter.allowReserved);
    return;
  }

  pairs.push(`${encodeQueryComponent(parameter.name)}=${encodeQueryValue(serializePrimitive(parameter.value), parameter.allowReserved)}`);
}

function appendArrayParameter(
  pairs: string[],
  name: string,
  value: unknown[],
  style: string,
  explode: boolean,
  allowReserved: boolean,
): void {
  const values = value
    .filter((item) => item !== undefined && item !== null)
    .map((item) => serializePrimitive(item));
  if (values.length === 0) {
    return;
  }

  if (style === 'form' && explode) {
    for (const item of values) {
      pairs.push(`${encodeQueryComponent(name)}=${encodeQueryValue(item, allowReserved)}`);
    }
    return;
  }

  pairs.push(`${encodeQueryComponent(name)}=${encodeQueryValue(values.join(','), allowReserved)}`);
}

function appendObjectParameter(
  pairs: string[],
  name: string,
  value: Record<string, unknown>,
  style: string,
  explode: boolean,
  allowReserved: boolean,
): void {
  const entries = Object.entries(value).filter(([, entryValue]) => entryValue !== undefined && entryValue !== null);
  if (entries.length === 0) {
    return;
  }

  if (style === 'form' && explode) {
    for (const [key, entryValue] of entries) {
      pairs.push(`${encodeQueryComponent(key)}=${encodeQueryValue(serializePrimitive(entryValue), allowReserved)}`);
    }
    return;
  }

  const serialized = entries.flatMap(([key, entryValue]) => [key, serializePrimitive(entryValue)]).join(',');
  pairs.push(`${encodeQueryComponent(name)}=${encodeQueryValue(serialized, allowReserved)}`);
}

function appendDeepObjectParameter(
  pairs: string[],
  name: string,
  value: unknown,
  allowReserved: boolean,
): void {
  if (!value || typeof value !== 'object' || Array.isArray(value)) {
    pairs.push(`${encodeQueryComponent(name)}=${encodeQueryValue(serializePrimitive(value), allowReserved)}`);
    return;
  }

  for (const [key, entryValue] of Object.entries(value as Record<string, unknown>)) {
    if (entryValue === undefined || entryValue === null) {
      continue;
    }
    pairs.push(`${encodeQueryComponent(`${name}[${key}]`)}=${encodeQueryValue(serializePrimitive(entryValue), allowReserved)}`);
  }
}

function serializePrimitive(value: unknown): string {
  if (value instanceof Date) {
    return value.toISOString();
  }
  if (typeof value === 'object') {
    return JSON.stringify(value);
  }
  return String(value);
}

function encodeQueryComponent(value: string): string {
  return encodeURIComponent(value);
}

function encodeQueryValue(value: string, allowReserved: boolean): string {
  const encoded = encodeURIComponent(value);
  if (!allowReserved) {
    return encoded;
  }
  return encoded.replace(/%3A/gi, ':')
    .replace(/%2F/gi, '/')
    .replace(/%3F/gi, '?')
    .replace(/%23/gi, '#')
    .replace(/%5B/gi, '[')
    .replace(/%5D/gi, ']')
    .replace(/%40/gi, '@')
    .replace(/%21/gi, '!')
    .replace(/%24/gi, '$')
    .replace(/%26/gi, '&')
    .replace(/%27/gi, "'")
    .replace(/%28/gi, '(')
    .replace(/%29/gi, ')')
    .replace(/%2A/gi, '*')
    .replace(/%2B/gi, '+')
    .replace(/%2C/gi, ',')
    .replace(/%3B/gi, ';')
    .replace(/%3D/gi, '=');
}
