import {
  createKnowledgebaseSessionTokenManager,
  KnowledgebaseErrorCodes,
  resolveSharedSdkApiBaseUrl,
  throwKnowledgebaseError,
} from 'sdkwork-knowledgebase-pc-core';
import type {
  KnowledgebaseRuntimeConfig,
  KnowledgebaseSessionTokenManager,
  SessionAppContextSnapshot,
  SessionSnapshot,
  SessionStore,
  SessionUserSnapshot,
} from 'sdkwork-knowledgebase-pc-core';
import {
  createClient,
  type SdkworkAppClient,
} from '@sdkwork/iam-app-sdk';
import {
  createSdkworkAppbasePcAuthRuntime,
  type SdkworkAppbasePcAuthRuntimeComposition,
  type SdkworkAppbasePcAuthRuntimeSdkClient,
  type SdkworkAppbasePcAuthSessionBridgeSession,
} from '@sdkwork/auth-runtime-pc-react';
import type {
  IamAppContext,
  IamDeploymentMode,
  IamEnvironment,
} from '@sdkwork/iam-contracts';

const APPBASE_APP_SDK_FAMILY_ID = 'sdkwork-iam-app-sdk';
const APP_API_PREFIX = '/app/v3/api';

export type KnowledgebaseIamRuntime = ReturnType<
  SdkworkAppbasePcAuthRuntimeComposition['getRuntime']
> & {
  onCurrentUserChanged?: (user: KnowledgebaseIamUserLike | undefined) => void;
};

export interface CreateKnowledgebaseIamRuntimeOptions {
  appClient?: unknown;
  config: KnowledgebaseRuntimeConfig;
  localeProvider?: () => string | undefined;
  sdkClients?: Array<{ setTokenManager(manager: KnowledgebaseSessionTokenManager): unknown }>;
  session: SessionStore;
  tokenManager?: KnowledgebaseSessionTokenManager;
}

interface KnowledgebaseIamUserLike {
  avatar?: unknown;
  displayName?: string;
  email?: string;
  id?: string;
  name?: string;
  nickname?: string;
  userId?: string;
  username?: string;
}

interface KnowledgebaseIamSessionLike extends SdkworkAppbasePcAuthSessionBridgeSession {
  context?: IamAppContext;
  sessionId?: string;
  user?: KnowledgebaseIamUserLike;
  userInfo?: KnowledgebaseIamUserLike;
}

export function createKnowledgebaseIamRuntime({
  appClient,
  config,
  localeProvider,
  sdkClients = [],
  session,
  tokenManager,
}: CreateKnowledgebaseIamRuntimeOptions): KnowledgebaseIamRuntime {
  const globalTokenManager = tokenManager ?? createKnowledgebaseSessionTokenManager(session);
  const generatedAppClient =
    appClient ?? createKnowledgebaseGeneratedAppClient({ config, tokenManager: globalTokenManager });
  const composition = createSdkworkAppbasePcAuthRuntime({
    app: {
      appId: config.appKey,
      deploymentMode: resolveIamDeploymentModeForRuntime(config),
      environment: toIamEnvironment(config.environment),
      platform: 'pc',
    },
    baseUrls: {
      appbaseAppApiBaseUrl: resolveAppbaseAppApiBaseUrl(config),
    },
    createAppbaseAppClient: () => generatedAppClient as SdkworkAppClient,
    localeProvider,
    sdkClients: sdkClients as SdkworkAppbasePcAuthRuntimeSdkClient[],
    sessionBridge: {
      clearSession: () => {
        session.clearSession();
      },
      commitSession: (nextSession) =>
        commitKnowledgebaseIamRuntimeSession(session, nextSession as KnowledgebaseIamSessionLike),
      readSession: () => toKnowledgebaseIamBridgeSession(session.getSnapshot()),
    },
    tokenManager: globalTokenManager as never,
  });
  const runtime = composition.runtime as KnowledgebaseIamRuntime;

  patchKnowledgebaseIamContextStore(session, composition.contextStore);

  runtime.onCurrentUserChanged = (user) => {
    mergeKnowledgebaseSession(session, {
      user: user ? toKnowledgebaseSessionUser(user) : undefined,
    });
  };
  bindKnowledgebaseSessionProjection(runtime, session);

  return runtime;
}

function createKnowledgebaseGeneratedAppClient({
  config,
  tokenManager,
}: {
  config: KnowledgebaseRuntimeConfig;
  tokenManager: KnowledgebaseSessionTokenManager;
}): SdkworkAppClient {
  const client = createClient({
    authMode: 'dual-token',
    baseUrl: normalizeGeneratedSdkBaseUrl(
      resolveAppbaseAppApiBaseUrl(config),
      APP_API_PREFIX,
    ),
    tokenManager: tokenManager as never,
  });

  return ensureIamTenantSelectionCompat(client);
}

function ensureIamTenantSelectionCompat(client: SdkworkAppClient): SdkworkAppClient {
  const sessions = (client as unknown as {
    auth?: {
      sessions?: Record<string, unknown>;
    };
  }).auth?.sessions;
  if (!sessions || sessions.tenantSelection) {
    return client;
  }

  const organizationSelection = sessions.organizationSelection as
    | { create?: (...args: unknown[]) => unknown }
    | undefined;
  if (organizationSelection?.create) {
    sessions.tenantSelection = {
      create: organizationSelection.create.bind(organizationSelection),
    };
    return client;
  }

  sessions.tenantSelection = {
    create: async () => {
      throwKnowledgebaseError(KnowledgebaseErrorCodes.IAM_RUNTIME_MISSING);
    },
  };
  return client;
}

function resolveAppbaseAppApiBaseUrl(config: KnowledgebaseRuntimeConfig): string {
  // The shared `SDKWORK_API_BASE_URL` key resolved through `@sdkwork/sdk-common`
  // wins; the per-app keys behind the runtime config only survive as a fallback.
  const configured =
    resolveSharedSdkApiBaseUrl()
    ?? config.sdkBaseUrls.dependencySdkBaseUrls[APPBASE_APP_SDK_FAMILY_ID]?.appApiBaseUrl;
  if (configured !== undefined) {
    return configured;
  }
  if (config.deploymentProfile === 'standalone') {
    return config.appApiBaseUrl;
  }
  return config.platformApiGatewayBaseUrl;
}

function normalizeGeneratedSdkBaseUrl(baseUrl: string, apiPrefix: string): string {
  const normalizedBaseUrl = baseUrl.replace(/\/+$/, '');
  const normalizedApiPrefix = apiPrefix.replace(/\/+$/, '');
  if (normalizedBaseUrl.endsWith(normalizedApiPrefix)) {
    return normalizedBaseUrl.slice(0, -normalizedApiPrefix.length) || normalizedBaseUrl;
  }
  return normalizedBaseUrl;
}

function bindKnowledgebaseSessionProjection(
  runtime: KnowledgebaseIamRuntime,
  session: SessionStore,
): void {
  const auth = runtime.service.auth;
  wrapIamSessionMethod(auth.registrations, 'create', session, () =>
    hydrateKnowledgebaseCurrentSession(runtime, session),
  );
  wrapIamSessionMethod(auth.sessions, 'create', session, () =>
    hydrateKnowledgebaseCurrentSession(runtime, session),
  );
  wrapIamSessionMethod(auth.sessions, 'refresh', session, () =>
    hydrateKnowledgebaseCurrentSession(runtime, session),
  );
  wrapIamSessionMethod(auth.sessions.current, 'retrieve', session);
  wrapIamSessionMethod(auth.sessions.current, 'update', session, () =>
    hydrateKnowledgebaseCurrentSession(runtime, session),
  );

  const oauth = runtime.service.oauth;
  wrapIamSessionMethod(oauth.deviceAuthorizations, 'create', session);
  wrapIamSessionMethod(oauth.deviceAuthorizations, 'retrieve', session);
  wrapIamSessionMethod(oauth.deviceAuthorizations.passwordCompletions, 'create', session, () =>
    hydrateKnowledgebaseCurrentSession(runtime, session),
  );
  wrapIamSessionMethod(oauth.deviceAuthorizations.scans, 'create', session);

  const usersCurrent = runtime.service.iam.users.current as {
    retrieve: () => Promise<KnowledgebaseIamUserLike>;
  };
  const retrieveCurrentUser = usersCurrent.retrieve.bind(usersCurrent);
  usersCurrent.retrieve = async () => {
    const user = await retrieveCurrentUser();
    runtime.onCurrentUserChanged?.(user);
    return user;
  };
}

function wrapIamSessionMethod(
  resource: object,
  methodName: string,
  session: SessionStore,
  hydrateContext?: () => Promise<void>,
): void {
  const mutableResource = resource as Record<string, unknown>;
  const original = mutableResource[methodName];
  if (typeof original !== 'function') {
    return;
  }

  mutableResource[methodName] = async (...args: unknown[]) => {
    const result = await original.apply(resource, args);
    syncKnowledgebaseIamSession(session, result as KnowledgebaseIamSessionLike);
    if (hydrateContext && shouldHydrateKnowledgebaseAppContext(result, session)) {
      await hydrateContext();
    }
    return augmentIamApiResultWithSessionContext(session, result as KnowledgebaseIamSessionLike);
  };
}

function patchKnowledgebaseIamContextStore(
  session: SessionStore,
  contextStore: SdkworkAppbasePcAuthRuntimeComposition['contextStore'],
): void {
  if (!contextStore?.clear) {
    return;
  }

  contextStore.clear = async () => {
    const snapshot = session.getSnapshot();
    if (!snapshot.context) {
      return;
    }

    const nextSession = { ...snapshot };
    delete nextSession.context;
    replaceKnowledgebaseSession(session, nextSession);
  };
}

function augmentIamApiResultWithSessionContext(
  session: SessionStore,
  apiResult: KnowledgebaseIamSessionLike,
): KnowledgebaseIamSessionLike {
  if (readIamContextTenantId(apiResult.context)) {
    return apiResult;
  }

  const snapshot = session.getSnapshot();
  const context = toIamAppContext(snapshot.context);
  if (!context) {
    return apiResult;
  }

  return {
    ...apiResult,
    context,
    sessionId: apiResult.sessionId ?? context.sessionId ?? snapshot.sessionId,
  };
}

function readIamContextTenantId(context: unknown): string | undefined {
  if (!context || typeof context !== 'object') {
    return undefined;
  }

  const record = context as Record<string, unknown>;
  return normalizeScalar(record.tenantId) ?? normalizeScalar(record.tenant_id);
}

function commitKnowledgebaseIamRuntimeSession(
  session: SessionStore,
  iamSession: KnowledgebaseIamSessionLike,
): KnowledgebaseIamSessionLike | undefined {
  const nextSession: SessionSnapshot = {
    ...session.getSnapshot(),
    accessToken: normalizeIamSessionToken(iamSession.accessToken),
    authToken: normalizeIamSessionToken(iamSession.authToken),
    refreshToken: iamSession.refreshToken,
    sessionId: iamSession.sessionId ?? iamSession.context?.sessionId,
  };

  if (iamSession.context) {
    nextSession.context = toKnowledgebaseSessionContext(iamSession.context);
  } else if (!iamSession.accessToken && !iamSession.authToken && !iamSession.refreshToken) {
    delete nextSession.context;
  }

  replaceKnowledgebaseSession(session, nextSession);

  return toKnowledgebaseIamBridgeSession(session.getSnapshot()) ?? undefined;
}

function toKnowledgebaseIamBridgeSession(
  snapshot: SessionSnapshot,
): KnowledgebaseIamSessionLike | null {
  if (!snapshot.authToken && !snapshot.accessToken && !snapshot.refreshToken) {
    return null;
  }

  const context = toIamAppContext(snapshot.context);
  return {
    ...(snapshot.accessToken ? { accessToken: snapshot.accessToken } : {}),
    ...(snapshot.authToken ? { authToken: snapshot.authToken } : {}),
    ...(snapshot.refreshToken ? { refreshToken: snapshot.refreshToken } : {}),
    ...(snapshot.sessionId ? { sessionId: snapshot.sessionId } : {}),
    ...(context ? { context } : {}),
  };
}

async function hydrateKnowledgebaseCurrentSession(
  runtime: KnowledgebaseIamRuntime,
  session: SessionStore,
): Promise<void> {
  if (session.getSnapshot().context?.tenantId) {
    return;
  }
  await runtime.service.auth.sessions.current.retrieve();
}

function shouldHydrateKnowledgebaseAppContext(
  value: unknown,
  session: SessionStore,
): boolean {
  if (session.getSnapshot().context?.tenantId) {
    return false;
  }
  const sessionLike = value as KnowledgebaseIamSessionLike | undefined;
  return Boolean(sessionLike?.authToken && sessionLike.accessToken && !sessionLike.context);
}

function syncKnowledgebaseIamSession(
  session: SessionStore,
  iamSession: KnowledgebaseIamSessionLike,
): void {
  mergeKnowledgebaseSession(session, {
    accessToken: normalizeIamSessionToken(iamSession.accessToken),
    authToken: normalizeIamSessionToken(iamSession.authToken),
    context: iamSession.context
      ? toKnowledgebaseSessionContext(iamSession.context)
      : undefined,
    refreshToken: iamSession.refreshToken,
    sessionId: iamSession.sessionId ?? iamSession.context?.sessionId,
    user: iamSession.user || iamSession.userInfo
      ? toKnowledgebaseSessionUser((iamSession.user ?? iamSession.userInfo)!)
      : undefined,
  });
}

function toKnowledgebaseSessionContext(context: IamAppContext): SessionAppContextSnapshot {
  return {
    tenantId: context.tenantId,
    userId: context.userId,
    organizationId: context.organizationId,
    sessionId: context.sessionId,
    appId: context.appId,
    environment: context.environment,
    iamDeploymentMode: context.deploymentMode,
    authLevel: context.authLevel,
    dataScope: [...context.dataScope],
    permissionScope: [...context.permissionScope],
    actorId: context.userId,
    actorKind: 'user',
  };
}

function toIamAppContext(
  context: SessionAppContextSnapshot | undefined,
): IamAppContext | undefined {
  if (!context?.tenantId || !context.userId || !context.sessionId) {
    return undefined;
  }

  return {
    appId: context.appId ?? 'sdkwork-knowledgebase-pc',
    authLevel: toIamAuthLevel(context.authLevel),
    dataScope: [...(context.dataScope ?? [])],
    deploymentMode: normalizeIamDeploymentMode(context.iamDeploymentMode),
    environment: toIamEnvironment(context.environment),
    organizationId: context.organizationId,
    permissionScope: [...(context.permissionScope ?? [])],
    sessionId: context.sessionId,
    tenantId: context.tenantId,
    userId: context.userId,
  };
}

function toKnowledgebaseSessionUser(user: KnowledgebaseIamUserLike): SessionUserSnapshot {
  const id = normalizeScalar(user.id) ?? normalizeScalar(user.userId) ?? 'knowledgebase-user';
  const displayName =
    normalizeScalar(user.displayName)
    ?? normalizeScalar(user.name)
    ?? normalizeScalar(user.nickname)
    ?? normalizeScalar(user.username);

  return {
    id,
    displayName,
    avatarUrl: resolveMediaUrl(user.avatar),
    email: normalizeScalar(user.email),
  };
}

function mergeKnowledgebaseSession(
  session: SessionStore,
  patch: Partial<SessionSnapshot>,
): void {
  replaceKnowledgebaseSession(session, {
    ...session.getSnapshot(),
    ...compactSessionPatch(patch),
  });
}

function replaceKnowledgebaseSession(
  session: SessionStore,
  nextSession: SessionSnapshot,
): void {
  const compact = compactSessionPatch(nextSession) as SessionSnapshot;
  if (!compact.authToken && !compact.accessToken && !compact.refreshToken) {
    session.clearSession();
    return;
  }

  session.setSession(compact);
}

function compactSessionPatch<T extends object>(value: T): Partial<T> {
  return Object.fromEntries(
    Object.entries(value).filter(([, entry]) => entry !== undefined),
  ) as Partial<T>;
}

function normalizeIamSessionToken(token: string | undefined): string | undefined {
  if (!token) {
    return undefined;
  }

  const trimmed = token.trim();
  if (!trimmed) {
    return undefined;
  }

  const withoutBearer = trimmed.replace(/^Bearer\s+/i, '').trim();
  return withoutBearer || undefined;
}

function resolveIamDeploymentModeForRuntime(config: KnowledgebaseRuntimeConfig): IamDeploymentMode {
  if (config.deploymentProfile === 'cloud') {
    return 'saas';
  }
  if (config.runtimeTarget === 'desktop' || config.environment !== 'production') {
    return 'local';
  }
  return 'private';
}

function normalizeIamDeploymentMode(
  value: string | undefined,
): IamDeploymentMode {
  if (value === 'local' || value === 'standalone') {
    return 'local';
  }
  if (value === 'saas' || value === 'cloud') {
    return 'saas';
  }
  if (value === 'private') {
    return 'private';
  }
  return 'private';
}

function toIamEnvironment(value: string | undefined): IamEnvironment {
  const normalized = String(value ?? '').trim().toLowerCase();
  if (normalized === 'prod' || normalized === 'production' || normalized === 'staging') {
    return 'prod';
  }
  if (normalized === 'test' || normalized === 'testing') {
    return 'test';
  }
  return 'dev';
}

function toIamAuthLevel(value: string | undefined): IamAppContext['authLevel'] {
  if (value === 'anonymous' || value === 'password' || value === 'mfa' || value === 'system') {
    return value;
  }
  return 'password';
}

function normalizeScalar(value: unknown): string | undefined {
  const normalized = typeof value === 'number' && Number.isFinite(value)
    ? String(value)
    : typeof value === 'string'
      ? value.trim()
      : '';
  return normalized || undefined;
}

function resolveMediaUrl(value: unknown): string | undefined {
  if (typeof value === 'string') {
    return normalizeScalar(value);
  }
  if (!value || typeof value !== 'object') {
    return undefined;
  }

  const record = value as Record<string, unknown>;
  return normalizeScalar(record.url)
    ?? normalizeScalar(record.deliveryUrl)
    ?? normalizeScalar(record.publicUrl)
    ?? normalizeScalar(record.cdnUrl);
}
