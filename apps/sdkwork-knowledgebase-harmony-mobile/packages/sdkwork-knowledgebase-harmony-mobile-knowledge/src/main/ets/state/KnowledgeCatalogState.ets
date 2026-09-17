import { type KnowledgebaseListItem } from '../models/KnowledgeModels';

/**
 * Package-local state slice for the knowledgebase catalog.
 *
 * Sensitive state must clear on logout and account/tenant switch.
 */
export interface KnowledgeCatalogState {
  readonly page: number;
  readonly items: KnowledgebaseListItem[];
  readonly hasMore: boolean;
  readonly loading: boolean;
  readonly errorMessage: string;
}

export function initialKnowledgeCatalogState(): KnowledgeCatalogState {
  return {
    page: 1,
    items: [],
    hasMore: false,
    loading: true,
    errorMessage: '',
  };
}
