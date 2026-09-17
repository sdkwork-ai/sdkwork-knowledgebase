/**
 * AuthGate integration for the mini program shell.
 *
 * Route guards are shell/runtime responsibilities; capability packages declare
 * auth mode and permission hints only
 * (`APP_CLIENT_ARCHITECTURE_ALIGNMENT_SPEC.md` section 7).
 */
export interface KnowledgebaseMpAuthGateDecision {
  readonly allowed: boolean;
  readonly redirectPagePath?: string;
  readonly reason?: string;
}

export function evaluateKnowledgebaseMpAuthGate(
  auth: "public" | "required",
  isAuthenticated: boolean,
  loginPagePath = "pages/home/index",
): KnowledgebaseMpAuthGateDecision {
  if (auth === "public" || isAuthenticated) {
    return { allowed: true };
  }
  return { allowed: false, redirectPagePath: loginPagePath, reason: "authentication-required" };
}
