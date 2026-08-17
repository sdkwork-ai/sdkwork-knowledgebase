import { appApiPath } from './paths';
import type { ApiRequestOptions, HttpClient } from '../http/client';

import type { ChangeKnowledgeWikiSourceFileVisibilityRequest, ConsumeGroupKnowledgebaseLaunchTicketRequest, CreateKnowledgeDocumentRequest, CreateKnowledgeDocumentVersionRequest, CreateKnowledgeSpaceContextBindingRequest, CreateKnowledgeSpaceRequest, GrantKnowledgeSpaceMemberRequest, GroupKnowledgebaseLaunchTarget, IngestionJob, KnowledgeAgentBinding, KnowledgeAgentBindingRequest, KnowledgeAgentChatRequest, KnowledgeAgentChatResponse, KnowledgeAgentProfile, KnowledgeAgentProfileRequest, KnowledgeBrowserListData, KnowledgeBrowserView, KnowledgeContextPack, KnowledgeContextPackRequest, KnowledgeDocument, KnowledgeDocumentContent, KnowledgeDocumentVersion, KnowledgeDriveImportRequest, KnowledgeDriveImportResult, KnowledgeGitImportRequest, KnowledgeGitImportResult, KnowledgeGitSyncRequest, KnowledgeGitSyncResult, KnowledgeIngestRequest, KnowledgeMarketCatalogItem, KnowledgeMarketSubscriptionRequest, KnowledgeMarketSubscriptionResult, KnowledgeMediaTaskRequest, KnowledgeMediaTaskResult, KnowledgeOkfBundleFile, KnowledgeOkfConceptRevisionList, KnowledgeRetrievalRequest, KnowledgeRetrievalResult, KnowledgeSpace, KnowledgeSpaceContextBinding, KnowledgeSpaceMember, KnowledgeSpaceMemberSubjectType, KnowledgeWechatAppletList, KnowledgeWechatArticlesPreviewRequest, KnowledgeWechatArticlesPublishRequest, KnowledgeWechatFanTagList, KnowledgeWechatOfficialAccountList, KnowledgeWechatOperationResult, KnowledgeWechatReplaceAppletsRequest, KnowledgeWechatReplaceOfficialAccountsRequest, KnowledgeWikiPublication, KnowledgeWikiPublicationVersionCommandRequest, KnowledgeWikiSourceFileCommandResult, KnowledgeWikiSourceFileVersionCommandRequest, OkfBundleExportRequest, OkfBundleImportRequest, OkfBundleImportResult, OkfConceptSummary, OkfConceptSummaryList, OkfConceptUpsertRequest, OkfContextPackRequest, OkfFileAnswerRequest, OkfIndexDocument, OkfLogDocument, OkfProfileDocument, OkfQualityRun, OkfQualityRunRequest, OkfQueryRequest, OkfQueryResult, PageInfo, PublishKnowledgeWikiSourceFileRequest, SdkWorkCommandData, UpdateKnowledgeSpaceContextBindingRequest, UpdateKnowledgeSpaceRequest } from '../types';


export interface KnowledgeWikiSourceFilesVisibilityUpdateParams {
  idempotencyKey: string;
}

export class KnowledgeWikiSourceFilesVisibilityApi {
  private client: HttpClient;

  constructor(client: HttpClient) {
    this.client = client;
  }


/** Change Wiki source file visibility */
  async update(spaceId: string, sourceFileUuid: string, body: ChangeKnowledgeWikiSourceFileVisibilityRequest, params: KnowledgeWikiSourceFilesVisibilityUpdateParams, requestOptions?: ApiRequestOptions): Promise<KnowledgeWikiSourceFileCommandResult> {
    const requestHeaders = buildRequestHeaders(
      {
        'Idempotency-Key': { value: params.idempotencyKey, style: 'simple', explode: false },
      },
      {}
    );
    return this.client.request<KnowledgeWikiSourceFileCommandResult>(appApiPath(`/knowledge/spaces/${serializePathParameter(spaceId, { name: 'spaceId', style: 'simple', explode: false })}/wiki_source_files/${serializePathParameter(sourceFileUuid, { name: 'sourceFileUuid', style: 'simple', explode: false })}/visibility`), { ...(requestOptions?.signal !== undefined ? { signal: requestOptions.signal } : {}), ...(requestOptions?.timeout !== undefined ? { timeout: requestOptions.timeout } : {}), method: 'PATCH' as any, body, contentType: 'application/json', ...(requestHeaders !== undefined ? { headers: requestHeaders } : {}), sdkworkUnwrapKind: 'item' });
  }
}

export interface KnowledgeWikiSourceFilesPublishParams {
  idempotencyKey: string;
}

export interface KnowledgeWikiSourceFilesUnpublishParams {
  idempotencyKey: string;
}

export class KnowledgeWikiSourceFilesApi {
  private client: HttpClient;
  public readonly visibility: KnowledgeWikiSourceFilesVisibilityApi;

  constructor(client: HttpClient) {
    this.client = client;
    this.visibility = new KnowledgeWikiSourceFilesVisibilityApi(client);
  }


/** Publish Wiki source file */
  async publish(spaceId: string, sourceFileUuid: string, body: PublishKnowledgeWikiSourceFileRequest, params: KnowledgeWikiSourceFilesPublishParams, requestOptions?: ApiRequestOptions): Promise<KnowledgeWikiSourceFileCommandResult> {
    const requestHeaders = buildRequestHeaders(
      {
        'Idempotency-Key': { value: params.idempotencyKey, style: 'simple', explode: false },
      },
      {}
    );
    return this.client.request<KnowledgeWikiSourceFileCommandResult>(appApiPath(`/knowledge/spaces/${serializePathParameter(spaceId, { name: 'spaceId', style: 'simple', explode: false })}/wiki_source_files/${serializePathParameter(sourceFileUuid, { name: 'sourceFileUuid', style: 'simple', explode: false })}/publish`), { ...(requestOptions?.signal !== undefined ? { signal: requestOptions.signal } : {}), ...(requestOptions?.timeout !== undefined ? { timeout: requestOptions.timeout } : {}), method: 'POST' as any, body, contentType: 'application/json', ...(requestHeaders !== undefined ? { headers: requestHeaders } : {}), sdkworkUnwrapKind: 'item' });
  }

/** Unpublish Wiki source file */
  async unpublish(spaceId: string, sourceFileUuid: string, body: KnowledgeWikiSourceFileVersionCommandRequest, params: KnowledgeWikiSourceFilesUnpublishParams, requestOptions?: ApiRequestOptions): Promise<KnowledgeWikiSourceFileCommandResult> {
    const requestHeaders = buildRequestHeaders(
      {
        'Idempotency-Key': { value: params.idempotencyKey, style: 'simple', explode: false },
      },
      {}
    );
    return this.client.request<KnowledgeWikiSourceFileCommandResult>(appApiPath(`/knowledge/spaces/${serializePathParameter(spaceId, { name: 'spaceId', style: 'simple', explode: false })}/wiki_source_files/${serializePathParameter(sourceFileUuid, { name: 'sourceFileUuid', style: 'simple', explode: false })}/unpublish`), { ...(requestOptions?.signal !== undefined ? { signal: requestOptions.signal } : {}), ...(requestOptions?.timeout !== undefined ? { timeout: requestOptions.timeout } : {}), method: 'POST' as any, body, contentType: 'application/json', ...(requestHeaders !== undefined ? { headers: requestHeaders } : {}), sdkworkUnwrapKind: 'item' });
  }
}

export interface KnowledgeWikiPublicationsActivateParams {
  idempotencyKey: string;
}

export interface KnowledgeWikiPublicationsPauseParams {
  idempotencyKey: string;
}

export class KnowledgeWikiPublicationsApi {
  private client: HttpClient;

  constructor(client: HttpClient) {
    this.client = client;
  }


/** Retrieve Wiki publication */
  async retrieve(spaceId: string, requestOptions?: ApiRequestOptions): Promise<KnowledgeWikiPublication> {
    return this.client.request<KnowledgeWikiPublication>(appApiPath(`/knowledge/spaces/${serializePathParameter(spaceId, { name: 'spaceId', style: 'simple', explode: false })}/wiki_publication`), { ...(requestOptions?.signal !== undefined ? { signal: requestOptions.signal } : {}), ...(requestOptions?.timeout !== undefined ? { timeout: requestOptions.timeout } : {}), method: 'GET' as any, sdkworkUnwrapKind: 'item' });
  }

/** Activate Wiki publication */
  async activate(spaceId: string, body: KnowledgeWikiPublicationVersionCommandRequest, params: KnowledgeWikiPublicationsActivateParams, requestOptions?: ApiRequestOptions): Promise<KnowledgeWikiPublication> {
    const requestHeaders = buildRequestHeaders(
      {
        'Idempotency-Key': { value: params.idempotencyKey, style: 'simple', explode: false },
      },
      {}
    );
    return this.client.request<KnowledgeWikiPublication>(appApiPath(`/knowledge/spaces/${serializePathParameter(spaceId, { name: 'spaceId', style: 'simple', explode: false })}/wiki_publication/activate`), { ...(requestOptions?.signal !== undefined ? { signal: requestOptions.signal } : {}), ...(requestOptions?.timeout !== undefined ? { timeout: requestOptions.timeout } : {}), method: 'POST' as any, body, contentType: 'application/json', ...(requestHeaders !== undefined ? { headers: requestHeaders } : {}), sdkworkUnwrapKind: 'item' });
  }

/** Pause Wiki publication */
  async pause(spaceId: string, body: KnowledgeWikiPublicationVersionCommandRequest, params: KnowledgeWikiPublicationsPauseParams, requestOptions?: ApiRequestOptions): Promise<KnowledgeWikiPublication> {
    const requestHeaders = buildRequestHeaders(
      {
        'Idempotency-Key': { value: params.idempotencyKey, style: 'simple', explode: false },
      },
      {}
    );
    return this.client.request<KnowledgeWikiPublication>(appApiPath(`/knowledge/spaces/${serializePathParameter(spaceId, { name: 'spaceId', style: 'simple', explode: false })}/wiki_publication/pause`), { ...(requestOptions?.signal !== undefined ? { signal: requestOptions.signal } : {}), ...(requestOptions?.timeout !== undefined ? { timeout: requestOptions.timeout } : {}), method: 'POST' as any, body, contentType: 'application/json', ...(requestHeaders !== undefined ? { headers: requestHeaders } : {}), sdkworkUnwrapKind: 'item' });
  }
}

export class KnowledgeMediaTasksApi {
  private client: HttpClient;

  constructor(client: HttpClient) {
    this.client = client;
  }


/** Create a knowledge media task (image generation or speech-to-text) */
  async create(body: KnowledgeMediaTaskRequest, requestOptions?: ApiRequestOptions): Promise<KnowledgeMediaTaskResult> {
    return this.client.request<KnowledgeMediaTaskResult>(appApiPath(`/knowledge/media_tasks`), { ...(requestOptions?.signal !== undefined ? { signal: requestOptions.signal } : {}), ...(requestOptions?.timeout !== undefined ? { timeout: requestOptions.timeout } : {}), method: 'POST' as any, body, contentType: 'application/json', sdkworkUnwrapKind: 'command' });
  }
}

export class KnowledgeMarketSubscriptionsApi {
  private client: HttpClient;

  constructor(client: HttpClient) {
    this.client = client;
  }


/** Subscribe to a knowledge market listing */
  async create(body: KnowledgeMarketSubscriptionRequest, requestOptions?: ApiRequestOptions): Promise<KnowledgeMarketSubscriptionResult> {
    return this.client.request<KnowledgeMarketSubscriptionResult>(appApiPath(`/knowledge/market/subscriptions`), { ...(requestOptions?.signal !== undefined ? { signal: requestOptions.signal } : {}), ...(requestOptions?.timeout !== undefined ? { timeout: requestOptions.timeout } : {}), method: 'POST' as any, body, contentType: 'application/json', sdkworkUnwrapKind: 'command' });
  }

/** Unsubscribe from a knowledge market listing */
  async delete(listingId: string, requestOptions?: ApiRequestOptions): Promise<void> {
    return this.client.request<void>(appApiPath(`/knowledge/market/subscriptions/${serializePathParameter(listingId, { name: 'listingId', style: 'simple', explode: false })}`), { ...(requestOptions?.signal !== undefined ? { signal: requestOptions.signal } : {}), ...(requestOptions?.timeout !== undefined ? { timeout: requestOptions.timeout } : {}), method: 'DELETE' as any });
  }
}

export interface KnowledgeMarketListingsListParams {
  cursor?: string;
  pageSize?: number;
}

export class KnowledgeMarketListingsApi {
  private client: HttpClient;

  constructor(client: HttpClient) {
    this.client = client;
  }


/** List knowledge market catalog listings */
  async list(params?: KnowledgeMarketListingsListParams, requestOptions?: ApiRequestOptions): Promise<{ items: KnowledgeMarketCatalogItem[]; pageInfo: PageInfo; }> {
    const query = buildQueryString([
      { name: 'cursor', value: params?.cursor, style: 'form', explode: true, allowReserved: false },
      { name: 'page_size', value: params?.pageSize, style: 'form', explode: true, allowReserved: false },
    ]);
    return this.client.request<{ items: KnowledgeMarketCatalogItem[]; pageInfo: PageInfo; }>(appendQueryString(appApiPath(`/knowledge/market/listings`), query), { ...(requestOptions?.signal !== undefined ? { signal: requestOptions.signal } : {}), ...(requestOptions?.timeout !== undefined ? { timeout: requestOptions.timeout } : {}), method: 'GET' as any, sdkworkUnwrapKind: 'page' });
  }
}

export class KnowledgeMarketApi {
  public readonly listings: KnowledgeMarketListingsApi;
  public readonly subscriptions: KnowledgeMarketSubscriptionsApi;

  constructor(client: HttpClient) {
    this.listings = new KnowledgeMarketListingsApi(client);
    this.subscriptions = new KnowledgeMarketSubscriptionsApi(client);
  }

}

export class KnowledgeGitSyncsApi {
  private client: HttpClient;

  constructor(client: HttpClient) {
    this.client = client;
  }


/** Sync knowledge space documents to a Git repository */
  async create(body: KnowledgeGitSyncRequest, requestOptions?: ApiRequestOptions): Promise<KnowledgeGitSyncResult> {
    return this.client.request<KnowledgeGitSyncResult>(appApiPath(`/knowledge/git_syncs`), { ...(requestOptions?.signal !== undefined ? { signal: requestOptions.signal } : {}), ...(requestOptions?.timeout !== undefined ? { timeout: requestOptions.timeout } : {}), method: 'POST' as any, body, contentType: 'application/json', sdkworkUnwrapKind: 'command' });
  }
}

export class KnowledgeWechatArticlesApi {
  private client: HttpClient;

  constructor(client: HttpClient) {
    this.client = client;
  }


/** Publish WeChat articles */
  async publish(body: KnowledgeWechatArticlesPublishRequest, requestOptions?: ApiRequestOptions): Promise<KnowledgeWechatOperationResult> {
    return this.client.request<KnowledgeWechatOperationResult>(appApiPath(`/knowledge/wechat/articles/publish`), { ...(requestOptions?.signal !== undefined ? { signal: requestOptions.signal } : {}), ...(requestOptions?.timeout !== undefined ? { timeout: requestOptions.timeout } : {}), method: 'POST' as any, body, contentType: 'application/json', sdkworkUnwrapKind: 'command' });
  }

/** Preview WeChat articles */
  async preview(body: KnowledgeWechatArticlesPreviewRequest, requestOptions?: ApiRequestOptions): Promise<KnowledgeWechatOperationResult> {
    return this.client.request<KnowledgeWechatOperationResult>(appApiPath(`/knowledge/wechat/articles/preview`), { ...(requestOptions?.signal !== undefined ? { signal: requestOptions.signal } : {}), ...(requestOptions?.timeout !== undefined ? { timeout: requestOptions.timeout } : {}), method: 'POST' as any, body, contentType: 'application/json', sdkworkUnwrapKind: 'command' });
  }
}

export class KnowledgeWechatAppletsApi {
  private client: HttpClient;

  constructor(client: HttpClient) {
    this.client = client;
  }


/** List WeChat applets */
  async list(requestOptions?: ApiRequestOptions): Promise<KnowledgeWechatAppletList> {
    return this.client.request<KnowledgeWechatAppletList>(appApiPath(`/knowledge/wechat/applets`), { ...(requestOptions?.signal !== undefined ? { signal: requestOptions.signal } : {}), ...(requestOptions?.timeout !== undefined ? { timeout: requestOptions.timeout } : {}), method: 'GET' as any, sdkworkUnwrapKind: 'item' });
  }

/** Replace WeChat applets */
  async update(body: KnowledgeWechatReplaceAppletsRequest, requestOptions?: ApiRequestOptions): Promise<KnowledgeWechatAppletList> {
    return this.client.request<KnowledgeWechatAppletList>(appApiPath(`/knowledge/wechat/applets`), { ...(requestOptions?.signal !== undefined ? { signal: requestOptions.signal } : {}), ...(requestOptions?.timeout !== undefined ? { timeout: requestOptions.timeout } : {}), method: 'PUT' as any, body, contentType: 'application/json', sdkworkUnwrapKind: 'item' });
  }
}

export class KnowledgeWechatOfficialAccountsFanTagsApi {
  private client: HttpClient;

  constructor(client: HttpClient) {
    this.client = client;
  }


/** List WeChat official account fan tags */
  async list(accountId: string, requestOptions?: ApiRequestOptions): Promise<KnowledgeWechatFanTagList> {
    return this.client.request<KnowledgeWechatFanTagList>(appApiPath(`/knowledge/wechat/official_accounts/${serializePathParameter(accountId, { name: 'accountId', style: 'simple', explode: false })}/fan_tags`), { ...(requestOptions?.signal !== undefined ? { signal: requestOptions.signal } : {}), ...(requestOptions?.timeout !== undefined ? { timeout: requestOptions.timeout } : {}), method: 'GET' as any, sdkworkUnwrapKind: 'item' });
  }
}

export class KnowledgeWechatOfficialAccountsApi {
  private client: HttpClient;
  public readonly fanTags: KnowledgeWechatOfficialAccountsFanTagsApi;

  constructor(client: HttpClient) {
    this.client = client;
    this.fanTags = new KnowledgeWechatOfficialAccountsFanTagsApi(client);
  }


/** List WeChat official accounts */
  async list(requestOptions?: ApiRequestOptions): Promise<KnowledgeWechatOfficialAccountList> {
    return this.client.request<KnowledgeWechatOfficialAccountList>(appApiPath(`/knowledge/wechat/official_accounts`), { ...(requestOptions?.signal !== undefined ? { signal: requestOptions.signal } : {}), ...(requestOptions?.timeout !== undefined ? { timeout: requestOptions.timeout } : {}), method: 'GET' as any, sdkworkUnwrapKind: 'item' });
  }

/** Replace WeChat official accounts */
  async update(body: KnowledgeWechatReplaceOfficialAccountsRequest, requestOptions?: ApiRequestOptions): Promise<KnowledgeWechatOfficialAccountList> {
    return this.client.request<KnowledgeWechatOfficialAccountList>(appApiPath(`/knowledge/wechat/official_accounts`), { ...(requestOptions?.signal !== undefined ? { signal: requestOptions.signal } : {}), ...(requestOptions?.timeout !== undefined ? { timeout: requestOptions.timeout } : {}), method: 'PUT' as any, body, contentType: 'application/json', sdkworkUnwrapKind: 'item' });
  }
}

export class KnowledgeWechatApi {
  public readonly officialAccounts: KnowledgeWechatOfficialAccountsApi;
  public readonly applets: KnowledgeWechatAppletsApi;
  public readonly articles: KnowledgeWechatArticlesApi;

  constructor(client: HttpClient) {
    this.officialAccounts = new KnowledgeWechatOfficialAccountsApi(client);
    this.applets = new KnowledgeWechatAppletsApi(client);
    this.articles = new KnowledgeWechatArticlesApi(client);
  }

}

export class KnowledgeContextBindingsApi {
  private client: HttpClient;

  constructor(client: HttpClient) {
    this.client = client;
  }


/** Retrieve a knowledge space context binding */
  async retrieve(bindingId: string, requestOptions?: ApiRequestOptions): Promise<KnowledgeSpaceContextBinding> {
    return this.client.request<KnowledgeSpaceContextBinding>(appApiPath(`/knowledge/context_bindings/${serializePathParameter(bindingId, { name: 'bindingId', style: 'simple', explode: false })}`), { ...(requestOptions?.signal !== undefined ? { signal: requestOptions.signal } : {}), ...(requestOptions?.timeout !== undefined ? { timeout: requestOptions.timeout } : {}), method: 'GET' as any, sdkworkUnwrapKind: 'item' });
  }

/** Update a knowledge space context binding */
  async update(bindingId: string, body: UpdateKnowledgeSpaceContextBindingRequest, requestOptions?: ApiRequestOptions): Promise<KnowledgeSpaceContextBinding> {
    return this.client.request<KnowledgeSpaceContextBinding>(appApiPath(`/knowledge/context_bindings/${serializePathParameter(bindingId, { name: 'bindingId', style: 'simple', explode: false })}`), { ...(requestOptions?.signal !== undefined ? { signal: requestOptions.signal } : {}), ...(requestOptions?.timeout !== undefined ? { timeout: requestOptions.timeout } : {}), method: 'PATCH' as any, body, contentType: 'application/json', sdkworkUnwrapKind: 'item' });
  }

/** Delete a knowledge space context binding */
  async delete(bindingId: string, requestOptions?: ApiRequestOptions): Promise<void> {
    return this.client.request<void>(appApiPath(`/knowledge/context_bindings/${serializePathParameter(bindingId, { name: 'bindingId', style: 'simple', explode: false })}`), { ...(requestOptions?.signal !== undefined ? { signal: requestOptions.signal } : {}), ...(requestOptions?.timeout !== undefined ? { timeout: requestOptions.timeout } : {}), method: 'DELETE' as any });
  }
}

export class KnowledgeAgentProfilesChatApi {
  private client: HttpClient;

  constructor(client: HttpClient) {
    this.client = client;
  }


/** Chat with a knowledge-backed agent profile */
  async create(profileId: string, body: KnowledgeAgentChatRequest, requestOptions?: ApiRequestOptions): Promise<KnowledgeAgentChatResponse> {
    return this.client.request<KnowledgeAgentChatResponse>(appApiPath(`/knowledge/agent_profiles/${serializePathParameter(profileId, { name: 'profileId', style: 'simple', explode: false })}/chat`), { ...(requestOptions?.signal !== undefined ? { signal: requestOptions.signal } : {}), ...(requestOptions?.timeout !== undefined ? { timeout: requestOptions.timeout } : {}), method: 'POST' as any, body, contentType: 'application/json', sdkworkUnwrapKind: 'item' });
  }
}

export class KnowledgeAgentProfilesRetrievalPreviewApi {
  private client: HttpClient;

  constructor(client: HttpClient) {
    this.client = client;
  }


/** Preview retrieval for an agent profile */
  async create(profileId: string, body: KnowledgeRetrievalRequest, requestOptions?: ApiRequestOptions): Promise<KnowledgeRetrievalResult> {
    return this.client.request<KnowledgeRetrievalResult>(appApiPath(`/knowledge/agent_profiles/${serializePathParameter(profileId, { name: 'profileId', style: 'simple', explode: false })}/retrieval_preview`), { ...(requestOptions?.signal !== undefined ? { signal: requestOptions.signal } : {}), ...(requestOptions?.timeout !== undefined ? { timeout: requestOptions.timeout } : {}), method: 'POST' as any, body, contentType: 'application/json', sdkworkUnwrapKind: 'item' });
  }
}

export class KnowledgeAgentProfilesBindingsApi {
  private client: HttpClient;

  constructor(client: HttpClient) {
    this.client = client;
  }


/** List agent profile bindings */
  async list(profileId: string, requestOptions?: ApiRequestOptions): Promise<unknown> {
    return this.client.request<unknown>(appApiPath(`/knowledge/agent_profiles/${serializePathParameter(profileId, { name: 'profileId', style: 'simple', explode: false })}/bindings`), { ...(requestOptions?.signal !== undefined ? { signal: requestOptions.signal } : {}), ...(requestOptions?.timeout !== undefined ? { timeout: requestOptions.timeout } : {}), method: 'GET' as any, sdkworkUnwrapKind: 'item' });
  }

/** Create an agent profile binding */
  async create(profileId: string, body: KnowledgeAgentBindingRequest, requestOptions?: ApiRequestOptions): Promise<KnowledgeAgentBinding> {
    return this.client.request<KnowledgeAgentBinding>(appApiPath(`/knowledge/agent_profiles/${serializePathParameter(profileId, { name: 'profileId', style: 'simple', explode: false })}/bindings`), { ...(requestOptions?.signal !== undefined ? { signal: requestOptions.signal } : {}), ...(requestOptions?.timeout !== undefined ? { timeout: requestOptions.timeout } : {}), method: 'POST' as any, body, contentType: 'application/json', sdkworkUnwrapKind: 'item' });
  }

/** Update an agent profile binding */
  async update(profileId: string, bindingId: string, body: KnowledgeAgentBindingRequest, requestOptions?: ApiRequestOptions): Promise<KnowledgeAgentBinding> {
    return this.client.request<KnowledgeAgentBinding>(appApiPath(`/knowledge/agent_profiles/${serializePathParameter(profileId, { name: 'profileId', style: 'simple', explode: false })}/bindings/${serializePathParameter(bindingId, { name: 'bindingId', style: 'simple', explode: false })}`), { ...(requestOptions?.signal !== undefined ? { signal: requestOptions.signal } : {}), ...(requestOptions?.timeout !== undefined ? { timeout: requestOptions.timeout } : {}), method: 'PATCH' as any, body, contentType: 'application/json', sdkworkUnwrapKind: 'item' });
  }

/** Delete an agent profile binding */
  async delete(profileId: string, bindingId: string, requestOptions?: ApiRequestOptions): Promise<void> {
    return this.client.request<void>(appApiPath(`/knowledge/agent_profiles/${serializePathParameter(profileId, { name: 'profileId', style: 'simple', explode: false })}/bindings/${serializePathParameter(bindingId, { name: 'bindingId', style: 'simple', explode: false })}`), { ...(requestOptions?.signal !== undefined ? { signal: requestOptions.signal } : {}), ...(requestOptions?.timeout !== undefined ? { timeout: requestOptions.timeout } : {}), method: 'DELETE' as any });
  }
}

export class KnowledgeAgentProfilesApi {
  private client: HttpClient;
  public readonly bindings: KnowledgeAgentProfilesBindingsApi;
  public readonly retrievalPreview: KnowledgeAgentProfilesRetrievalPreviewApi;
  public readonly chat: KnowledgeAgentProfilesChatApi;

  constructor(client: HttpClient) {
    this.client = client;
    this.bindings = new KnowledgeAgentProfilesBindingsApi(client);
    this.retrievalPreview = new KnowledgeAgentProfilesRetrievalPreviewApi(client);
    this.chat = new KnowledgeAgentProfilesChatApi(client);
  }


/** Create a knowledge agent profile */
  async create(body: KnowledgeAgentProfileRequest, requestOptions?: ApiRequestOptions): Promise<KnowledgeAgentProfile> {
    return this.client.request<KnowledgeAgentProfile>(appApiPath(`/knowledge/agent_profiles`), { ...(requestOptions?.signal !== undefined ? { signal: requestOptions.signal } : {}), ...(requestOptions?.timeout !== undefined ? { timeout: requestOptions.timeout } : {}), method: 'POST' as any, body, contentType: 'application/json', sdkworkUnwrapKind: 'item' });
  }

/** Retrieve a knowledge agent profile */
  async retrieve(profileId: string, requestOptions?: ApiRequestOptions): Promise<KnowledgeAgentProfile> {
    return this.client.request<KnowledgeAgentProfile>(appApiPath(`/knowledge/agent_profiles/${serializePathParameter(profileId, { name: 'profileId', style: 'simple', explode: false })}`), { ...(requestOptions?.signal !== undefined ? { signal: requestOptions.signal } : {}), ...(requestOptions?.timeout !== undefined ? { timeout: requestOptions.timeout } : {}), method: 'GET' as any, sdkworkUnwrapKind: 'item' });
  }

/** Update a knowledge agent profile */
  async update(profileId: string, body: KnowledgeAgentProfileRequest, requestOptions?: ApiRequestOptions): Promise<KnowledgeAgentProfile> {
    return this.client.request<KnowledgeAgentProfile>(appApiPath(`/knowledge/agent_profiles/${serializePathParameter(profileId, { name: 'profileId', style: 'simple', explode: false })}`), { ...(requestOptions?.signal !== undefined ? { signal: requestOptions.signal } : {}), ...(requestOptions?.timeout !== undefined ? { timeout: requestOptions.timeout } : {}), method: 'PATCH' as any, body, contentType: 'application/json', sdkworkUnwrapKind: 'item' });
  }

/** Delete a knowledge agent profile */
  async delete(profileId: string, requestOptions?: ApiRequestOptions): Promise<void> {
    return this.client.request<void>(appApiPath(`/knowledge/agent_profiles/${serializePathParameter(profileId, { name: 'profileId', style: 'simple', explode: false })}`), { ...(requestOptions?.signal !== undefined ? { signal: requestOptions.signal } : {}), ...(requestOptions?.timeout !== undefined ? { timeout: requestOptions.timeout } : {}), method: 'DELETE' as any });
  }
}

export class KnowledgeContextPacksApi {
  private client: HttpClient;

  constructor(client: HttpClient) {
    this.client = client;
  }


/** Create a knowledge context pack */
  async create(body: KnowledgeContextPackRequest, requestOptions?: ApiRequestOptions): Promise<KnowledgeContextPack> {
    return this.client.request<KnowledgeContextPack>(appApiPath(`/knowledge/context_packs`), { ...(requestOptions?.signal !== undefined ? { signal: requestOptions.signal } : {}), ...(requestOptions?.timeout !== undefined ? { timeout: requestOptions.timeout } : {}), method: 'POST' as any, body, contentType: 'application/json', sdkworkUnwrapKind: 'item' });
  }
}

export class KnowledgeRetrievalsApi {
  private client: HttpClient;

  constructor(client: HttpClient) {
    this.client = client;
  }


/** Create a knowledge retrieval */
  async create(body: KnowledgeRetrievalRequest, requestOptions?: ApiRequestOptions): Promise<KnowledgeRetrievalResult> {
    return this.client.request<KnowledgeRetrievalResult>(appApiPath(`/knowledge/retrievals`), { ...(requestOptions?.signal !== undefined ? { signal: requestOptions.signal } : {}), ...(requestOptions?.timeout !== undefined ? { timeout: requestOptions.timeout } : {}), method: 'POST' as any, body, contentType: 'application/json', sdkworkUnwrapKind: 'item' });
  }

/** Retrieve a knowledge retrieval result */
  async retrieve(retrievalId: string, requestOptions?: ApiRequestOptions): Promise<KnowledgeRetrievalResult> {
    return this.client.request<KnowledgeRetrievalResult>(appApiPath(`/knowledge/retrievals/${serializePathParameter(retrievalId, { name: 'retrievalId', style: 'simple', explode: false })}`), { ...(requestOptions?.signal !== undefined ? { signal: requestOptions.signal } : {}), ...(requestOptions?.timeout !== undefined ? { timeout: requestOptions.timeout } : {}), method: 'GET' as any, sdkworkUnwrapKind: 'item' });
  }
}

export class KnowledgeOkfLintRunsApi {
  private client: HttpClient;

  constructor(client: HttpClient) {
    this.client = client;
  }


/** Create an OKF bundle lint run */
  async create(body: OkfQualityRunRequest, requestOptions?: ApiRequestOptions): Promise<OkfQualityRun> {
    return this.client.request<OkfQualityRun>(appApiPath(`/knowledge/okf/lint_runs`), { ...(requestOptions?.signal !== undefined ? { signal: requestOptions.signal } : {}), ...(requestOptions?.timeout !== undefined ? { timeout: requestOptions.timeout } : {}), method: 'POST' as any, body, contentType: 'application/json', sdkworkUnwrapKind: 'item' });
  }
}

export class KnowledgeOkfContextPacksApi {
  private client: HttpClient;

  constructor(client: HttpClient) {
    this.client = client;
  }


/** Create an OKF context pack */
  async create(body: OkfContextPackRequest, requestOptions?: ApiRequestOptions): Promise<KnowledgeOkfBundleFile> {
    return this.client.request<KnowledgeOkfBundleFile>(appApiPath(`/knowledge/okf/context_packs`), { ...(requestOptions?.signal !== undefined ? { signal: requestOptions.signal } : {}), ...(requestOptions?.timeout !== undefined ? { timeout: requestOptions.timeout } : {}), method: 'POST' as any, body, contentType: 'application/json', sdkworkUnwrapKind: 'item' });
  }
}

export class KnowledgeOkfQueriesApi {
  private client: HttpClient;

  constructor(client: HttpClient) {
    this.client = client;
  }


/** Create an OKF query */
  async create(body: OkfQueryRequest, requestOptions?: ApiRequestOptions): Promise<OkfQueryResult> {
    return this.client.request<OkfQueryResult>(appApiPath(`/knowledge/okf/queries`), { ...(requestOptions?.signal !== undefined ? { signal: requestOptions.signal } : {}), ...(requestOptions?.timeout !== undefined ? { timeout: requestOptions.timeout } : {}), method: 'POST' as any, body, contentType: 'application/json', sdkworkUnwrapKind: 'item' });
  }

/** File an answer for an OKF query */
  async fileAnswer(queryId: string, body: OkfFileAnswerRequest, requestOptions?: ApiRequestOptions): Promise<OkfQueryResult> {
    return this.client.request<OkfQueryResult>(appApiPath(`/knowledge/okf/queries/${serializePathParameter(queryId, { name: 'queryId', style: 'simple', explode: false })}/file_answer`), { ...(requestOptions?.signal !== undefined ? { signal: requestOptions.signal } : {}), ...(requestOptions?.timeout !== undefined ? { timeout: requestOptions.timeout } : {}), method: 'POST' as any, body, contentType: 'application/json', sdkworkUnwrapKind: 'item' });
  }
}

export class KnowledgeOkfBundleImportApi {
  private client: HttpClient;

  constructor(client: HttpClient) {
    this.client = client;
  }


/** Import an OKF bundle from drive staging */
  async create(body: OkfBundleImportRequest, requestOptions?: ApiRequestOptions): Promise<OkfBundleImportResult> {
    return this.client.request<OkfBundleImportResult>(appApiPath(`/knowledge/okf/imports`), { ...(requestOptions?.signal !== undefined ? { signal: requestOptions.signal } : {}), ...(requestOptions?.timeout !== undefined ? { timeout: requestOptions.timeout } : {}), method: 'POST' as any, body, contentType: 'application/json', sdkworkUnwrapKind: 'item' });
  }
}

export class KnowledgeOkfBundleExportApi {
  private client: HttpClient;

  constructor(client: HttpClient) {
    this.client = client;
  }


/** Create an OKF bundle export */
  async create(body: OkfBundleExportRequest, requestOptions?: ApiRequestOptions): Promise<KnowledgeOkfBundleFile> {
    return this.client.request<KnowledgeOkfBundleFile>(appApiPath(`/knowledge/okf/exports`), { ...(requestOptions?.signal !== undefined ? { signal: requestOptions.signal } : {}), ...(requestOptions?.timeout !== undefined ? { timeout: requestOptions.timeout } : {}), method: 'POST' as any, body, contentType: 'application/json', sdkworkUnwrapKind: 'item' });
  }

/** Retrieve an OKF bundle export */
  async retrieve(exportId: string, requestOptions?: ApiRequestOptions): Promise<KnowledgeOkfBundleFile> {
    return this.client.request<KnowledgeOkfBundleFile>(appApiPath(`/knowledge/okf/exports/${serializePathParameter(exportId, { name: 'exportId', style: 'simple', explode: false })}`), { ...(requestOptions?.signal !== undefined ? { signal: requestOptions.signal } : {}), ...(requestOptions?.timeout !== undefined ? { timeout: requestOptions.timeout } : {}), method: 'GET' as any, sdkworkUnwrapKind: 'item' });
  }
}

export class KnowledgeOkfBundleProfileApi {
  private client: HttpClient;

  constructor(client: HttpClient) {
    this.client = client;
  }


/** Retrieve the OKF bundle profile */
  async list(requestOptions?: ApiRequestOptions): Promise<OkfProfileDocument> {
    return this.client.request<OkfProfileDocument>(appApiPath(`/knowledge/okf/profile`), { ...(requestOptions?.signal !== undefined ? { signal: requestOptions.signal } : {}), ...(requestOptions?.timeout !== undefined ? { timeout: requestOptions.timeout } : {}), method: 'GET' as any, sdkworkUnwrapKind: 'item' });
  }
}

export class KnowledgeOkfBundleLogApi {
  private client: HttpClient;

  constructor(client: HttpClient) {
    this.client = client;
  }


/** Retrieve the OKF bundle log */
  async list(requestOptions?: ApiRequestOptions): Promise<OkfLogDocument> {
    return this.client.request<OkfLogDocument>(appApiPath(`/knowledge/okf/log`), { ...(requestOptions?.signal !== undefined ? { signal: requestOptions.signal } : {}), ...(requestOptions?.timeout !== undefined ? { timeout: requestOptions.timeout } : {}), method: 'GET' as any, sdkworkUnwrapKind: 'item' });
  }
}

export class KnowledgeOkfBundleIndexApi {
  private client: HttpClient;

  constructor(client: HttpClient) {
    this.client = client;
  }


/** Retrieve the OKF bundle index */
  async list(requestOptions?: ApiRequestOptions): Promise<OkfIndexDocument> {
    return this.client.request<OkfIndexDocument>(appApiPath(`/knowledge/okf/index`), { ...(requestOptions?.signal !== undefined ? { signal: requestOptions.signal } : {}), ...(requestOptions?.timeout !== undefined ? { timeout: requestOptions.timeout } : {}), method: 'GET' as any, sdkworkUnwrapKind: 'item' });
  }
}

export class KnowledgeOkfBundleApi {
  public readonly index: KnowledgeOkfBundleIndexApi;
  public readonly log: KnowledgeOkfBundleLogApi;
  public readonly profile: KnowledgeOkfBundleProfileApi;
  public readonly export: KnowledgeOkfBundleExportApi;
  public readonly import: KnowledgeOkfBundleImportApi;

  constructor(client: HttpClient) {
    this.index = new KnowledgeOkfBundleIndexApi(client);
    this.log = new KnowledgeOkfBundleLogApi(client);
    this.profile = new KnowledgeOkfBundleProfileApi(client);
    this.export = new KnowledgeOkfBundleExportApi(client);
    this.import = new KnowledgeOkfBundleImportApi(client);
  }

}

export interface KnowledgeOkfConceptsRevisionsListParams {
  cursor?: string;
  pageSize?: number;
}

export class KnowledgeOkfConceptsRevisionsApi {
  private client: HttpClient;

  constructor(client: HttpClient) {
    this.client = client;
  }


/** List OKF concept revisions */
  async list(conceptId: string, params?: KnowledgeOkfConceptsRevisionsListParams, requestOptions?: ApiRequestOptions): Promise<KnowledgeOkfConceptRevisionList> {
    const query = buildQueryString([
      { name: 'cursor', value: params?.cursor, style: 'form', explode: true, allowReserved: false },
      { name: 'page_size', value: params?.pageSize, style: 'form', explode: true, allowReserved: false },
    ]);
    return this.client.request<KnowledgeOkfConceptRevisionList>(appendQueryString(appApiPath(`/knowledge/okf/concepts/${serializePathParameter(conceptId, { name: 'conceptId', style: 'simple', explode: false })}/revisions`), query), { ...(requestOptions?.signal !== undefined ? { signal: requestOptions.signal } : {}), ...(requestOptions?.timeout !== undefined ? { timeout: requestOptions.timeout } : {}), method: 'GET' as any, sdkworkUnwrapKind: 'page' });
  }
}

export interface KnowledgeOkfConceptsListParams {
  spaceId: string;
  cursor?: string;
  pageSize?: number;
}

export class KnowledgeOkfConceptsApi {
  private client: HttpClient;
  public readonly revisions: KnowledgeOkfConceptsRevisionsApi;

  constructor(client: HttpClient) {
    this.client = client;
    this.revisions = new KnowledgeOkfConceptsRevisionsApi(client);
  }


/** List OKF concepts */
  async list(params: KnowledgeOkfConceptsListParams, requestOptions?: ApiRequestOptions): Promise<OkfConceptSummaryList> {
    const query = buildQueryString([
      { name: 'spaceId', value: params.spaceId, style: 'form', explode: true, allowReserved: false },
      { name: 'cursor', value: params.cursor, style: 'form', explode: true, allowReserved: false },
      { name: 'page_size', value: params.pageSize, style: 'form', explode: true, allowReserved: false },
    ]);
    return this.client.request<OkfConceptSummaryList>(appendQueryString(appApiPath(`/knowledge/okf/concepts`), query), { ...(requestOptions?.signal !== undefined ? { signal: requestOptions.signal } : {}), ...(requestOptions?.timeout !== undefined ? { timeout: requestOptions.timeout } : {}), method: 'GET' as any, sdkworkUnwrapKind: 'page' });
  }

/** Retrieve an OKF concept */
  async retrieve(conceptId: string, requestOptions?: ApiRequestOptions): Promise<OkfConceptSummary> {
    return this.client.request<OkfConceptSummary>(appApiPath(`/knowledge/okf/concepts/${serializePathParameter(conceptId, { name: 'conceptId', style: 'simple', explode: false })}`), { ...(requestOptions?.signal !== undefined ? { signal: requestOptions.signal } : {}), ...(requestOptions?.timeout !== undefined ? { timeout: requestOptions.timeout } : {}), method: 'GET' as any, sdkworkUnwrapKind: 'item' });
  }

/** Delete an OKF concept */
  async delete(conceptId: string, requestOptions?: ApiRequestOptions): Promise<void> {
    return this.client.request<void>(appApiPath(`/knowledge/okf/concepts/${serializePathParameter(conceptId, { name: 'conceptId', style: 'simple', explode: false })}`), { ...(requestOptions?.signal !== undefined ? { signal: requestOptions.signal } : {}), ...(requestOptions?.timeout !== undefined ? { timeout: requestOptions.timeout } : {}), method: 'DELETE' as any });
  }

/** Upsert an OKF concept revision */
  async update(body: OkfConceptUpsertRequest, requestOptions?: ApiRequestOptions): Promise<OkfConceptSummary> {
    return this.client.request<OkfConceptSummary>(appApiPath(`/knowledge/okf/concepts/upsert`), { ...(requestOptions?.signal !== undefined ? { signal: requestOptions.signal } : {}), ...(requestOptions?.timeout !== undefined ? { timeout: requestOptions.timeout } : {}), method: 'PUT' as any, body, contentType: 'application/json', sdkworkUnwrapKind: 'item' });
  }
}

export class KnowledgeOkfApi {
  public readonly concepts: KnowledgeOkfConceptsApi;
  public readonly bundle: KnowledgeOkfBundleApi;
  public readonly queries: KnowledgeOkfQueriesApi;
  public readonly contextPacks: KnowledgeOkfContextPacksApi;
  public readonly lintRuns: KnowledgeOkfLintRunsApi;

  constructor(client: HttpClient) {
    this.concepts = new KnowledgeOkfConceptsApi(client);
    this.bundle = new KnowledgeOkfBundleApi(client);
    this.queries = new KnowledgeOkfQueriesApi(client);
    this.contextPacks = new KnowledgeOkfContextPacksApi(client);
    this.lintRuns = new KnowledgeOkfLintRunsApi(client);
  }

}

export interface KnowledgeDocumentsVersionsListParams {
  cursor?: string;
  pageSize?: number;
}

export class KnowledgeDocumentsVersionsApi {
  private client: HttpClient;

  constructor(client: HttpClient) {
    this.client = client;
  }


/** List document versions */
  async list(documentId: string, params?: KnowledgeDocumentsVersionsListParams, requestOptions?: ApiRequestOptions): Promise<{ items: KnowledgeDocumentVersion[]; pageInfo: PageInfo; }> {
    const query = buildQueryString([
      { name: 'cursor', value: params?.cursor, style: 'form', explode: true, allowReserved: false },
      { name: 'page_size', value: params?.pageSize, style: 'form', explode: true, allowReserved: false },
    ]);
    return this.client.request<{ items: KnowledgeDocumentVersion[]; pageInfo: PageInfo; }>(appendQueryString(appApiPath(`/knowledge/documents/${serializePathParameter(documentId, { name: 'documentId', style: 'simple', explode: false })}/versions`), query), { ...(requestOptions?.signal !== undefined ? { signal: requestOptions.signal } : {}), ...(requestOptions?.timeout !== undefined ? { timeout: requestOptions.timeout } : {}), method: 'GET' as any, sdkworkUnwrapKind: 'page' });
  }

/** Create a document version */
  async create(documentId: string, body: CreateKnowledgeDocumentVersionRequest, requestOptions?: ApiRequestOptions): Promise<KnowledgeDocumentVersion> {
    return this.client.request<KnowledgeDocumentVersion>(appApiPath(`/knowledge/documents/${serializePathParameter(documentId, { name: 'documentId', style: 'simple', explode: false })}/versions`), { ...(requestOptions?.signal !== undefined ? { signal: requestOptions.signal } : {}), ...(requestOptions?.timeout !== undefined ? { timeout: requestOptions.timeout } : {}), method: 'POST' as any, body, contentType: 'application/json', sdkworkUnwrapKind: 'item' });
  }
}

export class KnowledgeDocumentsContentApi {
  private client: HttpClient;

  constructor(client: HttpClient) {
    this.client = client;
  }


/** Retrieve authoritative knowledge document content */
  async list(documentId: string, requestOptions?: ApiRequestOptions): Promise<KnowledgeDocumentContent> {
    return this.client.request<KnowledgeDocumentContent>(appApiPath(`/knowledge/documents/${serializePathParameter(documentId, { name: 'documentId', style: 'simple', explode: false })}/content`), { ...(requestOptions?.signal !== undefined ? { signal: requestOptions.signal } : {}), ...(requestOptions?.timeout !== undefined ? { timeout: requestOptions.timeout } : {}), method: 'GET' as any, sdkworkUnwrapKind: 'item' });
  }
}

export interface KnowledgeDocumentsListParams {
  spaceId: string;
  cursor?: string;
  pageSize?: number;
}

export class KnowledgeDocumentsApi {
  private client: HttpClient;
  public readonly content: KnowledgeDocumentsContentApi;
  public readonly versions: KnowledgeDocumentsVersionsApi;

  constructor(client: HttpClient) {
    this.client = client;
    this.content = new KnowledgeDocumentsContentApi(client);
    this.versions = new KnowledgeDocumentsVersionsApi(client);
  }


/** List knowledge documents */
  async list(params: KnowledgeDocumentsListParams, requestOptions?: ApiRequestOptions): Promise<{ items: KnowledgeDocument[]; pageInfo: PageInfo; }> {
    const query = buildQueryString([
      { name: 'spaceId', value: params.spaceId, style: 'form', explode: true, allowReserved: false },
      { name: 'cursor', value: params.cursor, style: 'form', explode: true, allowReserved: false },
      { name: 'page_size', value: params.pageSize, style: 'form', explode: true, allowReserved: false },
    ]);
    return this.client.request<{ items: KnowledgeDocument[]; pageInfo: PageInfo; }>(appendQueryString(appApiPath(`/knowledge/documents`), query), { ...(requestOptions?.signal !== undefined ? { signal: requestOptions.signal } : {}), ...(requestOptions?.timeout !== undefined ? { timeout: requestOptions.timeout } : {}), method: 'GET' as any, sdkworkUnwrapKind: 'page' });
  }

/** Create a knowledge document */
  async create(body: CreateKnowledgeDocumentRequest, requestOptions?: ApiRequestOptions): Promise<KnowledgeDocument> {
    return this.client.request<KnowledgeDocument>(appApiPath(`/knowledge/documents`), { ...(requestOptions?.signal !== undefined ? { signal: requestOptions.signal } : {}), ...(requestOptions?.timeout !== undefined ? { timeout: requestOptions.timeout } : {}), method: 'POST' as any, body, contentType: 'application/json', sdkworkUnwrapKind: 'item' });
  }

/** Retrieve a knowledge document */
  async retrieve(documentId: string, requestOptions?: ApiRequestOptions): Promise<KnowledgeDocument> {
    return this.client.request<KnowledgeDocument>(appApiPath(`/knowledge/documents/${serializePathParameter(documentId, { name: 'documentId', style: 'simple', explode: false })}`), { ...(requestOptions?.signal !== undefined ? { signal: requestOptions.signal } : {}), ...(requestOptions?.timeout !== undefined ? { timeout: requestOptions.timeout } : {}), method: 'GET' as any, sdkworkUnwrapKind: 'item' });
  }

/** Update a knowledge document */
  async update(documentId: string, body: CreateKnowledgeDocumentRequest, requestOptions?: ApiRequestOptions): Promise<KnowledgeDocument> {
    return this.client.request<KnowledgeDocument>(appApiPath(`/knowledge/documents/${serializePathParameter(documentId, { name: 'documentId', style: 'simple', explode: false })}`), { ...(requestOptions?.signal !== undefined ? { signal: requestOptions.signal } : {}), ...(requestOptions?.timeout !== undefined ? { timeout: requestOptions.timeout } : {}), method: 'PATCH' as any, body, contentType: 'application/json', sdkworkUnwrapKind: 'item' });
  }

/** Delete a knowledge document */
  async delete(documentId: string, requestOptions?: ApiRequestOptions): Promise<void> {
    return this.client.request<void>(appApiPath(`/knowledge/documents/${serializePathParameter(documentId, { name: 'documentId', style: 'simple', explode: false })}`), { ...(requestOptions?.signal !== undefined ? { signal: requestOptions.signal } : {}), ...(requestOptions?.timeout !== undefined ? { timeout: requestOptions.timeout } : {}), method: 'DELETE' as any });
  }
}

export class KnowledgeIngestsApi {
  private client: HttpClient;

  constructor(client: HttpClient) {
    this.client = client;
  }


/** Create an ingestion job */
  async create(body: KnowledgeIngestRequest, requestOptions?: ApiRequestOptions): Promise<IngestionJob> {
    return this.client.request<IngestionJob>(appApiPath(`/knowledge/ingests`), { ...(requestOptions?.signal !== undefined ? { signal: requestOptions.signal } : {}), ...(requestOptions?.timeout !== undefined ? { timeout: requestOptions.timeout } : {}), method: 'POST' as any, body, contentType: 'application/json', sdkworkUnwrapKind: 'item' });
  }

/** Retrieve an ingestion job */
  async retrieve(ingestId: string, requestOptions?: ApiRequestOptions): Promise<IngestionJob> {
    return this.client.request<IngestionJob>(appApiPath(`/knowledge/ingests/${serializePathParameter(ingestId, { name: 'ingestId', style: 'simple', explode: false })}`), { ...(requestOptions?.signal !== undefined ? { signal: requestOptions.signal } : {}), ...(requestOptions?.timeout !== undefined ? { timeout: requestOptions.timeout } : {}), method: 'GET' as any, sdkworkUnwrapKind: 'item' });
  }
}

export class KnowledgeGitImportsApi {
  private client: HttpClient;

  constructor(client: HttpClient) {
    this.client = client;
  }


/** Import a Git repository into knowledgebase */
  async create(body: KnowledgeGitImportRequest, requestOptions?: ApiRequestOptions): Promise<KnowledgeGitImportResult> {
    return this.client.request<KnowledgeGitImportResult>(appApiPath(`/knowledge/git_imports`), { ...(requestOptions?.signal !== undefined ? { signal: requestOptions.signal } : {}), ...(requestOptions?.timeout !== undefined ? { timeout: requestOptions.timeout } : {}), method: 'POST' as any, body, contentType: 'application/json', sdkworkUnwrapKind: 'item' });
  }
}

export class KnowledgeDriveImportsApi {
  private client: HttpClient;

  constructor(client: HttpClient) {
    this.client = client;
  }


/** Import a drive object into knowledgebase */
  async create(body: KnowledgeDriveImportRequest, requestOptions?: ApiRequestOptions): Promise<KnowledgeDriveImportResult> {
    return this.client.request<KnowledgeDriveImportResult>(appApiPath(`/knowledge/drive_imports`), { ...(requestOptions?.signal !== undefined ? { signal: requestOptions.signal } : {}), ...(requestOptions?.timeout !== undefined ? { timeout: requestOptions.timeout } : {}), method: 'POST' as any, body, contentType: 'application/json', sdkworkUnwrapKind: 'item' });
  }
}

export interface KnowledgeSpacesMembersListParams {
  cursor?: string;
  pageSize?: number;
}

export interface KnowledgeSpacesMembersDeleteParams {
  subjectType: KnowledgeSpaceMemberSubjectType;
  subjectId: string;
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
    return this.client.request<{ items: KnowledgeSpaceMember[]; pageInfo: PageInfo; }>(appendQueryString(appApiPath(`/knowledge/spaces/${serializePathParameter(spaceId, { name: 'spaceId', style: 'simple', explode: false })}/members`), query), { ...(requestOptions?.signal !== undefined ? { signal: requestOptions.signal } : {}), ...(requestOptions?.timeout !== undefined ? { timeout: requestOptions.timeout } : {}), method: 'GET' as any, sdkworkUnwrapKind: 'page' });
  }

/** Grant knowledge space member access */
  async create(spaceId: string, body: GrantKnowledgeSpaceMemberRequest, requestOptions?: ApiRequestOptions): Promise<SdkWorkCommandData> {
    return this.client.request<SdkWorkCommandData>(appApiPath(`/knowledge/spaces/${serializePathParameter(spaceId, { name: 'spaceId', style: 'simple', explode: false })}/members`), { ...(requestOptions?.signal !== undefined ? { signal: requestOptions.signal } : {}), ...(requestOptions?.timeout !== undefined ? { timeout: requestOptions.timeout } : {}), method: 'POST' as any, body, contentType: 'application/json', sdkworkUnwrapKind: 'command' });
  }

/** Revoke knowledge space member access */
  async delete(spaceId: string, params: KnowledgeSpacesMembersDeleteParams, requestOptions?: ApiRequestOptions): Promise<void> {
    const query = buildQueryString([
      { name: 'subjectType', value: params.subjectType, style: 'form', explode: true, allowReserved: false },
      { name: 'subjectId', value: params.subjectId, style: 'form', explode: true, allowReserved: false },
    ]);
    return this.client.request<void>(appendQueryString(appApiPath(`/knowledge/spaces/${serializePathParameter(spaceId, { name: 'spaceId', style: 'simple', explode: false })}/members`), query), { ...(requestOptions?.signal !== undefined ? { signal: requestOptions.signal } : {}), ...(requestOptions?.timeout !== undefined ? { timeout: requestOptions.timeout } : {}), method: 'DELETE' as any });
  }
}

export interface KnowledgeSpacesContextBindingsListParams {
  cursor?: string;
  pageSize?: number;
}

export class KnowledgeSpacesContextBindingsApi {
  private client: HttpClient;

  constructor(client: HttpClient) {
    this.client = client;
  }


/** List knowledge space context bindings */
  async list(spaceId: string, params?: KnowledgeSpacesContextBindingsListParams, requestOptions?: ApiRequestOptions): Promise<{ items: KnowledgeSpaceContextBinding[]; pageInfo: PageInfo; }> {
    const query = buildQueryString([
      { name: 'cursor', value: params?.cursor, style: 'form', explode: true, allowReserved: false },
      { name: 'page_size', value: params?.pageSize, style: 'form', explode: true, allowReserved: false },
    ]);
    return this.client.request<{ items: KnowledgeSpaceContextBinding[]; pageInfo: PageInfo; }>(appendQueryString(appApiPath(`/knowledge/spaces/${serializePathParameter(spaceId, { name: 'spaceId', style: 'simple', explode: false })}/context_bindings`), query), { ...(requestOptions?.signal !== undefined ? { signal: requestOptions.signal } : {}), ...(requestOptions?.timeout !== undefined ? { timeout: requestOptions.timeout } : {}), method: 'GET' as any, sdkworkUnwrapKind: 'page' });
  }

/** Create a knowledge space context binding */
  async create(spaceId: string, body: CreateKnowledgeSpaceContextBindingRequest, requestOptions?: ApiRequestOptions): Promise<KnowledgeSpaceContextBinding> {
    return this.client.request<KnowledgeSpaceContextBinding>(appApiPath(`/knowledge/spaces/${serializePathParameter(spaceId, { name: 'spaceId', style: 'simple', explode: false })}/context_bindings`), { ...(requestOptions?.signal !== undefined ? { signal: requestOptions.signal } : {}), ...(requestOptions?.timeout !== undefined ? { timeout: requestOptions.timeout } : {}), method: 'POST' as any, body, contentType: 'application/json', sdkworkUnwrapKind: 'item' });
  }
}

export interface KnowledgeSpacesBrowserListParams {
  view: KnowledgeBrowserView;
  parentId?: string | null;
  cursor?: string | null;
  pageSize?: number;
}

export class KnowledgeSpacesBrowserApi {
  private client: HttpClient;

  constructor(client: HttpClient) {
    this.client = client;
  }


/** List knowledge browser view */
  async list(spaceId: string, params: KnowledgeSpacesBrowserListParams, requestOptions?: ApiRequestOptions): Promise<KnowledgeBrowserListData> {
    const query = buildQueryString([
      { name: 'view', value: params.view, style: 'form', explode: true, allowReserved: false },
      { name: 'parentId', value: params.parentId, style: 'form', explode: true, allowReserved: false },
      { name: 'cursor', value: params.cursor, style: 'form', explode: true, allowReserved: false },
      { name: 'page_size', value: params.pageSize, style: 'form', explode: true, allowReserved: false },
    ]);
    return this.client.request<KnowledgeBrowserListData>(appendQueryString(appApiPath(`/knowledge/spaces/${serializePathParameter(spaceId, { name: 'spaceId', style: 'simple', explode: false })}/browser`), query), { ...(requestOptions?.signal !== undefined ? { signal: requestOptions.signal } : {}), ...(requestOptions?.timeout !== undefined ? { timeout: requestOptions.timeout } : {}), method: 'GET' as any, sdkworkUnwrapKind: 'page' });
  }
}

export class KnowledgeSpacesApi {
  private client: HttpClient;
  public readonly browser: KnowledgeSpacesBrowserApi;
  public readonly contextBindings: KnowledgeSpacesContextBindingsApi;
  public readonly members: KnowledgeSpacesMembersApi;

  constructor(client: HttpClient) {
    this.client = client;
    this.browser = new KnowledgeSpacesBrowserApi(client);
    this.contextBindings = new KnowledgeSpacesContextBindingsApi(client);
    this.members = new KnowledgeSpacesMembersApi(client);
  }


/** Create a knowledge space */
  async create(body: CreateKnowledgeSpaceRequest, requestOptions?: ApiRequestOptions): Promise<KnowledgeSpace> {
    return this.client.request<KnowledgeSpace>(appApiPath(`/knowledge/spaces`), { ...(requestOptions?.signal !== undefined ? { signal: requestOptions.signal } : {}), ...(requestOptions?.timeout !== undefined ? { timeout: requestOptions.timeout } : {}), method: 'POST' as any, body, contentType: 'application/json', sdkworkUnwrapKind: 'item' });
  }

/** Retrieve a knowledge space */
  async retrieve(spaceId: string, requestOptions?: ApiRequestOptions): Promise<KnowledgeSpace> {
    return this.client.request<KnowledgeSpace>(appApiPath(`/knowledge/spaces/${serializePathParameter(spaceId, { name: 'spaceId', style: 'simple', explode: false })}`), { ...(requestOptions?.signal !== undefined ? { signal: requestOptions.signal } : {}), ...(requestOptions?.timeout !== undefined ? { timeout: requestOptions.timeout } : {}), method: 'GET' as any, sdkworkUnwrapKind: 'item' });
  }

/** Update a knowledge space */
  async update(spaceId: string, body: UpdateKnowledgeSpaceRequest, requestOptions?: ApiRequestOptions): Promise<KnowledgeSpace> {
    return this.client.request<KnowledgeSpace>(appApiPath(`/knowledge/spaces/${serializePathParameter(spaceId, { name: 'spaceId', style: 'simple', explode: false })}`), { ...(requestOptions?.signal !== undefined ? { signal: requestOptions.signal } : {}), ...(requestOptions?.timeout !== undefined ? { timeout: requestOptions.timeout } : {}), method: 'PATCH' as any, body, contentType: 'application/json', sdkworkUnwrapKind: 'item' });
  }

/** Delete a knowledge space */
  async delete(spaceId: string, requestOptions?: ApiRequestOptions): Promise<void> {
    return this.client.request<void>(appApiPath(`/knowledge/spaces/${serializePathParameter(spaceId, { name: 'spaceId', style: 'simple', explode: false })}`), { ...(requestOptions?.signal !== undefined ? { signal: requestOptions.signal } : {}), ...(requestOptions?.timeout !== undefined ? { timeout: requestOptions.timeout } : {}), method: 'DELETE' as any });
  }
}

export interface KnowledgeGroupLaunchesConsumeParams {
  idempotencyKey: string;
}

export class KnowledgeGroupLaunchesApi {
  private client: HttpClient;

  constructor(client: HttpClient) {
    this.client = client;
  }


/** Consume a group knowledgebase launch ticket */
  async consume(body: ConsumeGroupKnowledgebaseLaunchTicketRequest, params: KnowledgeGroupLaunchesConsumeParams, requestOptions?: ApiRequestOptions): Promise<GroupKnowledgebaseLaunchTarget> {
    const requestHeaders = buildRequestHeaders(
      {
        'Idempotency-Key': { value: params.idempotencyKey, style: 'simple', explode: false },
      },
      {}
    );
    return this.client.request<GroupKnowledgebaseLaunchTarget>(appApiPath(`/knowledge/group_launches/consume`), { ...(requestOptions?.signal !== undefined ? { signal: requestOptions.signal } : {}), ...(requestOptions?.timeout !== undefined ? { timeout: requestOptions.timeout } : {}), method: 'POST' as any, body, contentType: 'application/json', ...(requestHeaders !== undefined ? { headers: requestHeaders } : {}), sdkworkUnwrapKind: 'item' });
  }
}

export class KnowledgeApi {
  public readonly groupLaunches: KnowledgeGroupLaunchesApi;
  public readonly spaces: KnowledgeSpacesApi;
  public readonly driveImports: KnowledgeDriveImportsApi;
  public readonly gitImports: KnowledgeGitImportsApi;
  public readonly ingests: KnowledgeIngestsApi;
  public readonly documents: KnowledgeDocumentsApi;
  public readonly okf: KnowledgeOkfApi;
  public readonly retrievals: KnowledgeRetrievalsApi;
  public readonly contextPacks: KnowledgeContextPacksApi;
  public readonly agentProfiles: KnowledgeAgentProfilesApi;
  public readonly contextBindings: KnowledgeContextBindingsApi;
  public readonly wechat: KnowledgeWechatApi;
  public readonly gitSyncs: KnowledgeGitSyncsApi;
  public readonly market: KnowledgeMarketApi;
  public readonly mediaTasks: KnowledgeMediaTasksApi;
  public readonly wikiPublications: KnowledgeWikiPublicationsApi;
  public readonly wikiSourceFiles: KnowledgeWikiSourceFilesApi;

  constructor(client: HttpClient) {
    this.groupLaunches = new KnowledgeGroupLaunchesApi(client);
    this.spaces = new KnowledgeSpacesApi(client);
    this.driveImports = new KnowledgeDriveImportsApi(client);
    this.gitImports = new KnowledgeGitImportsApi(client);
    this.ingests = new KnowledgeIngestsApi(client);
    this.documents = new KnowledgeDocumentsApi(client);
    this.okf = new KnowledgeOkfApi(client);
    this.retrievals = new KnowledgeRetrievalsApi(client);
    this.contextPacks = new KnowledgeContextPacksApi(client);
    this.agentProfiles = new KnowledgeAgentProfilesApi(client);
    this.contextBindings = new KnowledgeContextBindingsApi(client);
    this.wechat = new KnowledgeWechatApi(client);
    this.gitSyncs = new KnowledgeGitSyncsApi(client);
    this.market = new KnowledgeMarketApi(client);
    this.mediaTasks = new KnowledgeMediaTasksApi(client);
    this.wikiPublications = new KnowledgeWikiPublicationsApi(client);
    this.wikiSourceFiles = new KnowledgeWikiSourceFilesApi(client);
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
function buildRequestHeaders(
  headers: Record<string, HeaderParameterSpec | undefined>,
  cookies: Record<string, HeaderParameterSpec | undefined> = {},
): Record<string, string> | undefined {
  const requestHeaders: Record<string, string> = {};

  for (const [name, parameter] of Object.entries(headers)) {
    const serialized = serializeParameterValue(parameter);
    if (serialized !== undefined) {
      requestHeaders[name] = serialized;
    }
  }

  const cookieHeader = buildCookieHeader(cookies);
  if (cookieHeader) {
    requestHeaders.Cookie = requestHeaders.Cookie
      ? `${requestHeaders.Cookie}; ${cookieHeader}`
      : cookieHeader;
  }

  return Object.keys(requestHeaders).length > 0 ? requestHeaders : undefined;
}

interface HeaderParameterSpec {
  value: unknown;
  style: string;
  explode: boolean;
  contentType?: string;
}

function buildCookieHeader(cookies: Record<string, HeaderParameterSpec | undefined>): string | undefined {
  const pairs: string[] = [];
  for (const [name, parameter] of Object.entries(cookies)) {
    const serialized = serializeParameterValue(parameter);
    if (serialized !== undefined) {
      pairs.push(`${encodeURIComponent(name)}=${encodeURIComponent(serialized)}`);
    }
  }
  return pairs.length > 0 ? pairs.join('; ') : undefined;
}

function serializeParameterValue(parameter: HeaderParameterSpec | undefined): string | undefined {
  const value = parameter?.value;
  if (value === undefined || value === null) {
    return undefined;
  }
  if (parameter?.contentType) {
    return JSON.stringify(value);
  }
  if (value instanceof Date) {
    return value.toISOString();
  }
  if (Array.isArray(value)) {
    return value.map((item) => serializeHeaderPrimitive(item)).join(',');
  }
  if (typeof value === 'object' && value !== null) {
    return serializeHeaderObject(value as Record<string, unknown>, parameter?.explode === true);
  }
  return serializeHeaderPrimitive(value);
}

function serializeHeaderObject(value: Record<string, unknown>, explode: boolean): string {
  const entries = Object.entries(value).filter(([, entryValue]) => entryValue !== undefined && entryValue !== null);
  if (explode) {
    return entries.map(([key, entryValue]) => `${key}=${serializeHeaderPrimitive(entryValue)}`).join(',');
  }
  return entries.flatMap(([key, entryValue]) => [key, serializeHeaderPrimitive(entryValue)]).join(',');
}

function serializeHeaderPrimitive(value: unknown): string {
  if (value instanceof Date) {
    return value.toISOString();
  }
  return String(value);
}
