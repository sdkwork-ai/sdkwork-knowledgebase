/** Domain-neutral screen-state vocabulary for mini program pages. */
export type KnowledgebaseMpScreenState = "empty" | "error" | "loading" | "ready" | "unavailable";

export function resolveKnowledgebaseMpScreenStatus(input: {
  error?: string;
  itemCount: number;
  loading: boolean;
}): KnowledgebaseMpScreenState {
  if (input.loading) {
    return "loading";
  }
  if (input.error) {
    return "error";
  }
  return input.itemCount > 0 ? "ready" : "empty";
}
