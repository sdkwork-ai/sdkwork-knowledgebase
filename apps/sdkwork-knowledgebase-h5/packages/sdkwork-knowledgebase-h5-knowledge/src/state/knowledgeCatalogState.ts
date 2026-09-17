import type { KnowledgeSpaceSummary } from "../models/knowledgeModels";

export interface KnowledgeCatalogState {
  error?: string;
  loading: boolean;
  spaces: KnowledgeSpaceSummary[];
}

export const INITIAL_KNOWLEDGE_CATALOG_STATE: KnowledgeCatalogState = {
  loading: false,
  spaces: [],
};

export type KnowledgeCatalogAction =
  | { type: "load-failed"; error: string }
  | { type: "load-started" }
  | { type: "load-succeeded"; spaces: KnowledgeSpaceSummary[] };

/** Package-local reducer; view models stay package-local (alignment spec section 6). */
export function knowledgeCatalogReducer(
  state: KnowledgeCatalogState,
  action: KnowledgeCatalogAction,
): KnowledgeCatalogState {
  switch (action.type) {
    case "load-started":
      return { ...state, error: undefined, loading: true };
    case "load-succeeded":
      return { loading: false, spaces: action.spaces };
    case "load-failed":
      return { error: action.error, loading: false, spaces: state.spaces };
    default:
      return state;
  }
}
