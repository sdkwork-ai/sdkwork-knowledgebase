/**
 * Appbase IAM runtime wiring for the HarmonyOS root.
 *
 * Authority: `APP_SDK_INTEGRATION_SPEC.md` and `IAM_LOGIN_INTEGRATION_SPEC.md`.
 *
 * The global token-manager equivalent is shared by the appbase app SDK, every
 * authenticated app-api SDK client, and explicit backend-admin clients.
 * Logout and refresh failure must clear the token manager, session/context
 * stores, secure platform storage, and realtime bridges.
 */

export interface KnowledgebaseHarmonyIamRuntime {
  isAuthenticated(): boolean;
  clearOnLogout(): void;
}

export class KnowledgebaseHarmonyIamRuntimeImpl implements KnowledgebaseHarmonyIamRuntime {
  private authenticated: boolean = false;

  isAuthenticated(): boolean {
    return this.authenticated;
  }

  clearOnLogout(): void {
    this.authenticated = false;
  }
}

let iamRuntime: KnowledgebaseHarmonyIamRuntime | null = null;

export function createKnowledgebaseHarmonyIamRuntime(): KnowledgebaseHarmonyIamRuntime {
  iamRuntime = new KnowledgebaseHarmonyIamRuntimeImpl();
  return iamRuntime;
}

export function getKnowledgebaseHarmonyIamRuntime(): KnowledgebaseHarmonyIamRuntime {
  if (iamRuntime === null) {
    return createKnowledgebaseHarmonyIamRuntime();
  }
  return iamRuntime;
}
