/**
 * H5 host adapter contract.
 *
 * Feature packages consume this contract only and must never reference
 * `Capacitor.*`, `window.__TAURI__`, or other platform globals
 * (`APP_CLIENT_ARCHITECTURE_ALIGNMENT_SPEC.md` section 9).
 */
export type KnowledgebaseHostCapability =
  | "clipboard"
  | "deepLinks"
  | "filePicker"
  | "networkStatus"
  | "shareSheet"
  | "secureStorage";

export type KnowledgebaseHostOutcome<T> =
  | { status: "ok"; value: T }
  | { status: "unsupported" }
  | { status: "permission-denied" }
  | { status: "unavailable" }
  | { status: "cancelled" }
  | { status: "invalid-state"; reason: string };

export interface KnowledgebaseH5HostAdapter {
  readonly capabilities: ReadonlySet<KnowledgebaseHostCapability>;
  readonly runtimeTarget: string;
  readGroupKnowledgebaseLaunchTicket(): Promise<KnowledgebaseHostOutcome<string>>;
}

export interface CreateKnowledgebaseH5HostAdapterOptions {
  capabilities: KnowledgebaseHostCapability[];
  runtimeTarget: string;
}

/**
 * Build the browser host adapter.
 *
 * Browser-only mode exposes the declared capability set and returns
 * `unsupported` for every native-only operation
 * (alignment spec section 9 degradation rules).
 */
export function createKnowledgebaseH5BrowserHostAdapter(
  options: CreateKnowledgebaseH5HostAdapterOptions,
): KnowledgebaseH5HostAdapter {
  const capabilities = new Set<KnowledgebaseHostCapability>(options.capabilities);
  return {
    capabilities,
    runtimeTarget: options.runtimeTarget,
    async readGroupKnowledgebaseLaunchTicket() {
      if (!capabilities.has("deepLinks")) {
        return { status: "unsupported" };
      }
      if (typeof window === "undefined") {
        return { status: "unavailable" };
      }
      // Browser launch consumes only the opaque ticket in the standalone route
      // fragment; space ids and session tokens are never accepted from a URL.
      const hash = window.location.hash ?? "";
      const marker = "group-launch=";
      const index = hash.indexOf(marker);
      if (index < 0) {
        return { status: "invalid-state", reason: "no launch ticket present" };
      }
      const ticket = hash.slice(index + marker.length).split("&")[0];
      return ticket ? { status: "ok", value: ticket } : { status: "invalid-state", reason: "empty launch ticket" };
    },
  };
}
