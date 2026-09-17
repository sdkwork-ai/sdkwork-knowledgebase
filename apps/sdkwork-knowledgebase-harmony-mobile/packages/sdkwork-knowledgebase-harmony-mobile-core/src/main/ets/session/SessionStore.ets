/**
 * Session and context store.
 *
 * Authority: `APP_SDK_INTEGRATION_SPEC.md`. Logout, refresh failure, tenant
 * switch, and account switch must clear this store together with the token
 * manager, secure platform storage, and realtime/session bridges.
 */

export interface KnowledgebaseHarmonySession {
  readonly accessToken?: string;
  readonly authToken?: string;
  readonly tenantId?: string;
}

let currentSession: KnowledgebaseHarmonySession | null = null;

export function readKnowledgebaseHarmonySession(): KnowledgebaseHarmonySession | null {
  return currentSession;
}

export function writeKnowledgebaseHarmonySession(
  session: KnowledgebaseHarmonySession | null,
): void {
  currentSession = session;
}

export function clearKnowledgebaseHarmonySession(): void {
  currentSession = null;
}
