import { isBlank, trim } from '@sdkwork/utils';

import type { SessionAppContextSnapshot, SessionSnapshot } from './sessionStore';

/**
 * Session context derived from the SDKWork session tokens themselves.
 * Host-managed sessions forward credentials only; identity claims come from
 * the token payload (IAM_SPEC "token-derived context"), so the embedded
 * surface never requires the host to pass tenant or user ids explicitly.
 */
export interface KnowledgebaseSessionTokenClaims {
  appId?: string;
  authLevel?: string;
  dataScope?: string[];
  deploymentMode?: string;
  environment?: string;
  organizationId?: string;
  permissionScope?: string[];
  sessionId?: string;
  tenantId?: string;
  userId?: string;
}

const JWT_PAYLOAD_SEGMENT_INDEX = 1;

function readClaimString(
  claims: Record<string, unknown>,
  keys: readonly string[],
): string | undefined {
  for (const key of keys) {
    const value = claims[key];
    if (typeof value === 'string' && !isBlank(value)) {
      return trim(value);
    }
  }
  return undefined;
}

function readClaimStringArray(
  claims: Record<string, unknown>,
  keys: readonly string[],
): string[] | undefined {
  for (const key of keys) {
    const value = claims[key];
    if (Array.isArray(value) && value.every((entry) => typeof entry === 'string')) {
      return value as string[];
    }
  }
  return undefined;
}

function decodeJwtPayload(token: string): Record<string, unknown> | undefined {
  const segments = token.split('.');
  const payloadSegment = segments[JWT_PAYLOAD_SEGMENT_INDEX];
  if (!payloadSegment) {
    return undefined;
  }

  const base64 = payloadSegment.replace(/-/g, '+').replace(/_/g, '/');
  const padded = base64 + '='.repeat((4 - (base64.length % 4)) % 4);
  if (typeof atob !== 'function') {
    return undefined;
  }

  let binary: string;
  try {
    binary = atob(padded);
  } catch {
    // Malformed or signed-opaque token: no claims can be derived from it.
    return undefined;
  }

  try {
    const json = new TextDecoder().decode(Uint8Array.from(binary, (char) => char.charCodeAt(0)));
    const payload = JSON.parse(json) as unknown;
    if (payload === null || typeof payload !== 'object' || Array.isArray(payload)) {
      return undefined;
    }
    return payload as Record<string, unknown>;
  } catch {
    return undefined;
  }
}

/**
 * Read the identity claims carried by the session snapshot's tokens.
 * @param snapshot - session snapshot holding the SDKWork dual tokens.
 * @returns the payload claims of the access token (auth token as fallback), or
 * undefined when neither token is a decodable JWT.
 */
export function readKnowledgebaseSessionTokenClaims(
  snapshot: Pick<SessionSnapshot, 'accessToken' | 'authToken'>,
): Record<string, unknown> | undefined {
  for (const token of [snapshot.accessToken, snapshot.authToken]) {
    if (!token || isBlank(token)) {
      continue;
    }
    const claims = decodeJwtPayload(token);
    if (claims) {
      return claims;
    }
  }
  return undefined;
}

/**
 * Map the canonical SDKWork token claims onto a session app context.
 * @param claims - raw token payload claims (`tenant_id`, `user_id`, ...).
 * @returns the context, or undefined when the claims lack the required
 * tenant and user identity; the knowledgebase guards fail closed without it.
 */
export function knowledgebaseSessionContextFromClaims(
  claims: Record<string, unknown>,
): SessionAppContextSnapshot | undefined {
  const claimsLike: KnowledgebaseSessionTokenClaims = {
    appId: readClaimString(claims, ['app_id', 'appId']),
    authLevel: readClaimString(claims, ['auth_level', 'authLevel']),
    dataScope: readClaimStringArray(claims, ['data_scope', 'dataScope']),
    deploymentMode: readClaimString(claims, ['deployment_mode', 'deploymentMode']),
    environment: readClaimString(claims, ['environment']),
    organizationId: readClaimString(claims, ['organization_id', 'organizationId']),
    permissionScope: readClaimStringArray(claims, ['permission_scope', 'permissionScope']),
    sessionId: readClaimString(claims, ['session_id', 'sessionId']),
    tenantId: readClaimString(claims, ['tenant_id', 'tenantId']),
    userId: readClaimString(claims, ['user_id', 'userId']),
  };

  if (!claimsLike.tenantId || !claimsLike.userId) {
    return undefined;
  }

  return {
    tenantId: claimsLike.tenantId,
    userId: claimsLike.userId,
    ...(claimsLike.organizationId === undefined ? {} : { organizationId: claimsLike.organizationId }),
    ...(claimsLike.sessionId === undefined ? {} : { sessionId: claimsLike.sessionId }),
    ...(claimsLike.appId === undefined ? {} : { appId: claimsLike.appId }),
    ...(claimsLike.environment === undefined ? {} : { environment: claimsLike.environment }),
    ...(claimsLike.deploymentMode === undefined ? {} : { iamDeploymentMode: claimsLike.deploymentMode }),
    ...(claimsLike.authLevel === undefined ? {} : { authLevel: claimsLike.authLevel }),
    ...(claimsLike.dataScope === undefined ? {} : { dataScope: claimsLike.dataScope }),
    ...(claimsLike.permissionScope === undefined ? {} : { permissionScope: claimsLike.permissionScope }),
    actorId: claimsLike.userId,
    actorKind: 'user',
  };
}

/**
 * Derive the session context from the snapshot's own tokens.
 * @param snapshot - session snapshot holding the SDKWork dual tokens.
 * @returns the token-derived context, or undefined when the tokens carry no
 * usable identity claims (non-JWT tokens, or claims missing tenant/user).
 */
export function deriveKnowledgebaseSessionContext(
  snapshot: Pick<SessionSnapshot, 'accessToken' | 'authToken'>,
): SessionAppContextSnapshot | undefined {
  const claims = readKnowledgebaseSessionTokenClaims(snapshot);
  if (!claims) {
    return undefined;
  }
  return knowledgebaseSessionContextFromClaims(claims);
}

/**
 * Fill the session context from the snapshot's tokens when the snapshot has no
 * usable context of its own. Token claims are the server-minted authority
 * (IAM_SPEC "token-derived context"): a derivable context replaces the
 * incoming one, an underivable one leaves the snapshot untouched.
 * @param snapshot - session snapshot to normalize (host-bridged or stored).
 * @returns the snapshot carrying a complete context whenever the tokens allow.
 */
export function withKnowledgebaseTokenDerivedContext(
  snapshot: SessionSnapshot,
): SessionSnapshot {
  const derived = deriveKnowledgebaseSessionContext(snapshot);
  if (!derived) {
    return snapshot;
  }
  return { ...snapshot, context: derived };
}
