/**
 * Platform host contracts for the mini program root.
 *
 * Feature packages consume these contracts only and never reference `wx.*`
 * directly (`APP_CLIENT_ARCHITECTURE_ALIGNMENT_SPEC.md` section 9).
 */
export type KnowledgebaseMpHostOutcome<T> =
  | { status: "ok"; value: T }
  | { status: "unsupported" }
  | { status: "permission-denied" }
  | { status: "unavailable" }
  | { status: "cancelled" }
  | { status: "invalid-state"; reason: string };

export interface KnowledgebaseMpHostAdapter {
  readonly platform: string;
  navigateTo(pagePath: string): Promise<KnowledgebaseMpHostOutcome<void>>;
  readSecureValue(key: string): Promise<KnowledgebaseMpHostOutcome<string>>;
  scanQrCode(): Promise<KnowledgebaseMpHostOutcome<string>>;
  showToast(message: string): Promise<KnowledgebaseMpHostOutcome<void>>;
}
