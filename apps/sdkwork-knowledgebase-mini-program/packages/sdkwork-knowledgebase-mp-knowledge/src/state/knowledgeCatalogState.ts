import type { KnowledgebaseMpSpace } from "../models/knowledgeModels";

export interface KnowledgebaseMpCatalogState {
  error?: string;
  loading: boolean;
  spaces: KnowledgebaseMpSpace[];
}

export const INITIAL_KNOWLEDGEBASE_MP_CATALOG_STATE: KnowledgebaseMpCatalogState = {
  loading: false,
  spaces: [],
};

export type KnowledgebaseMpCatalogAction =
  | { type: "load-failed"; error: string }
  | { type: "load-started" }
  | { type: "load-succeeded"; spaces: KnowledgebaseMpSpace[] };

/** Package-local reducer; view state stays package-local. */
export function knowledgebaseMpCatalogReducer(
  state: KnowledgebaseMpCatalogState,
  action: KnowledgebaseMpCatalogAction,
): KnowledgebaseMpCatalogState {
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
