import { type KnowledgebaseListItem } from '../../models/KnowledgeModels';
import { type KnowledgeCatalogState } from '../../state/KnowledgeCatalogState';

/**
 * Presentation view model for the knowledgebase catalog.
 *
 * Owns UI state mapping and calls services only.
 */
export interface KnowledgeCatalogViewModel {
  readonly title: string;
  readonly loading: boolean;
  readonly items: KnowledgebaseListItem[];
  readonly hasMore: boolean;
  readonly errorMessage: string;
}

export function toKnowledgeCatalogViewModel(
  state: KnowledgeCatalogState,
): KnowledgeCatalogViewModel {
  return {
    title: 'Knowledgebase',
    loading: state.loading,
    items: state.items,
    hasMore: state.hasMore,
    errorMessage: state.errorMessage,
  };
}
