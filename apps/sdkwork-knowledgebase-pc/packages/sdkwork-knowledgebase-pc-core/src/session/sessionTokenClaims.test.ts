import { describe, expect, it } from 'vitest';

import { createSessionStore } from './sessionStore';
import {
  bindKnowledgebaseSessionStore,
  getKnowledgebaseTenantId,
} from '../api/knowledgebaseSpaceRegistry';
import {
  deriveKnowledgebaseSessionContext,
  readKnowledgebaseSessionTokenClaims,
  withKnowledgebaseTokenDerivedContext,
} from './sessionTokenClaims';

function encodeJwtPayload(payload: Record<string, unknown>): string {
  const bytes = new TextEncoder().encode(JSON.stringify(payload));
  const base64 = btoa(String.fromCharCode(...bytes));
  return base64.replace(/\+/g, '-').replace(/\//g, '_').replace(/=+$/, '');
}

function signSessionToken(payload: Record<string, unknown>): string {
  return `header.${encodeJwtPayload(payload)}.signature`;
}

const SDKWORK_TOKEN_CLAIMS = {
  app_id: 'sdkwork-knowledgebase-pc',
  auth_level: 'standard',
  data_scope: ['tenant'],
  deployment_mode: 'saas',
  environment: 'cloud',
  exp: 4102444800,
  organization_id: '0',
  permission_scope: ['knowledge.read'],
  session_id: 'session-1',
  tenant_id: '100001',
  token_type: 'auth_token',
  user_id: 'user-1',
};

describe('sessionTokenClaims', () => {
  it('derives the session context from the access token claims', () => {
    const derived = deriveKnowledgebaseSessionContext({
      accessToken: signSessionToken(SDKWORK_TOKEN_CLAIMS),
    });

    expect(derived).toMatchObject({
      tenantId: '100001',
      userId: 'user-1',
      sessionId: 'session-1',
      appId: 'sdkwork-knowledgebase-pc',
      environment: 'cloud',
      iamDeploymentMode: 'saas',
      authLevel: 'standard',
      dataScope: ['tenant'],
      permissionScope: ['knowledge.read'],
      actorId: 'user-1',
      actorKind: 'user',
    });
  });

  it('falls back to the auth token when no access token is present', () => {
    const derived = deriveKnowledgebaseSessionContext({
      authToken: signSessionToken(SDKWORK_TOKEN_CLAIMS),
    });
    expect(derived?.tenantId).toBe('100001');
    expect(derived?.userId).toBe('user-1');
  });

  it('prefers the access token over the auth token', () => {
    const claims = readKnowledgebaseSessionTokenClaims({
      accessToken: signSessionToken({ tenant_id: '100002', user_id: 'user-2' }),
      authToken: signSessionToken({ tenant_id: '100003', user_id: 'user-3' }),
    });
    expect(claims).toMatchObject({ tenant_id: '100002', user_id: 'user-2' });
  });

  it('accepts camelCase claim spellings', () => {
    const derived = deriveKnowledgebaseSessionContext({
      accessToken: signSessionToken({ tenantId: '100001', userId: 'user-1' }),
    });
    expect(derived).toMatchObject({ tenantId: '100001', userId: 'user-1' });
  });

  it('returns no context when tokens are not SDKWork JWTs', () => {
    expect(deriveKnowledgebaseSessionContext({ accessToken: 'opaque-token' })).toBeUndefined();
    expect(deriveKnowledgebaseSessionContext({})).toBeUndefined();
  });

  it('returns no context when claims miss the tenant or user identity', () => {
    expect(
      deriveKnowledgebaseSessionContext({ accessToken: signSessionToken({ tenant_id: '100001' }) }),
    ).toBeUndefined();
    expect(
      deriveKnowledgebaseSessionContext({ accessToken: signSessionToken({ user_id: 'user-1' }) }),
    ).toBeUndefined();
  });

  it('fills the context of a host session that forwards credentials only', () => {
    const hostSession = {
      accessToken: signSessionToken(SDKWORK_TOKEN_CLAIMS),
      authToken: signSessionToken(SDKWORK_TOKEN_CLAIMS),
      refreshToken: 'refresh-1',
      sessionId: 'session-1',
    };

    expect(withKnowledgebaseTokenDerivedContext(hostSession).context).toMatchObject({
      tenantId: '100001',
      userId: 'user-1',
    });
    expect(withKnowledgebaseTokenDerivedContext(hostSession)).toMatchObject({
      authToken: hostSession.authToken,
      refreshToken: 'refresh-1',
    });
  });

  it('lets token claims supersede a stale host-provided context', () => {
    const snapshot = {
      accessToken: signSessionToken(SDKWORK_TOKEN_CLAIMS),
      context: {
        tenantId: '999999',
        userId: 'stale-user',
      },
    };

    expect(withKnowledgebaseTokenDerivedContext(snapshot).context).toMatchObject({
      tenantId: '100001',
      userId: 'user-1',
    });
  });

  it('leaves the snapshot untouched when the tokens carry no identity claims', () => {
    const snapshot = {
      accessToken: 'opaque-token',
      context: {
        tenantId: '100001',
        userId: 'user-1',
      },
    };

    expect(withKnowledgebaseTokenDerivedContext(snapshot)).toBe(snapshot);
  });

  it('satisfies the knowledgebase tenant registry from token-derived contexts', () => {
    const session = createSessionStore();
    bindKnowledgebaseSessionStore(session);
    try {
      session.setSession(withKnowledgebaseTokenDerivedContext({
        accessToken: signSessionToken(SDKWORK_TOKEN_CLAIMS),
      }));
      expect(getKnowledgebaseTenantId()).toBe('100001');
    } finally {
      bindKnowledgebaseSessionStore(createSessionStore());
    }
  });
});
