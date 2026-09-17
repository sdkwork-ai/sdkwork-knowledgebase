/**
 * AuthGate integration for the HarmonyOS mobile shell.
 *
 * Route guards are shell/runtime responsibilities. Capability packages declare
 * auth mode and permission hints only.
 */
export interface SdkworkKnowledgebaseAuthGateDecision {
  readonly allowed: boolean;
  readonly reason?: string;
}

export function evaluateSdkworkKnowledgebaseAuthGate(
  authRequired: boolean,
  isAuthenticated: boolean,
): SdkworkKnowledgebaseAuthGateDecision {
  if (!authRequired || isAuthenticated) {
    return { allowed: true };
  }
  return { allowed: false, reason: 'authentication-required' };
}
