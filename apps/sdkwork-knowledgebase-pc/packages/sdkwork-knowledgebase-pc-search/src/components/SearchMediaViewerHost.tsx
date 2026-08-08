import { useEffect, useState } from 'react';
import type { SearchNavigateToFilePayload } from '../types';
import {
  SEARCH_OPEN_MEDIA_VIEWER_EVENT,
  type SearchMediaViewerOpenDetail
} from '../utils/searchMediaViewerBridge';
import { SearchMediaViewerModal } from './SearchMediaViewerModal';

export interface SearchMediaViewerHostProps {
  onGoToFile?: (payload: SearchNavigateToFilePayload) => void;
  onOpenWebLink?: (url: string, title?: string) => void;
}

export function SearchMediaViewerHost({ onGoToFile, onOpenWebLink }: SearchMediaViewerHostProps) {
  const [state, setState] = useState<SearchMediaViewerOpenDetail | null>(null);

  useEffect(() => {
    const handler = (event: Event) => {
      const detail = (event as CustomEvent<SearchMediaViewerOpenDetail>).detail;
      if (!detail?.items?.length) return;
      setState({
        items: detail.items,
        activeIndex: Math.min(Math.max(detail.activeIndex, 0), detail.items.length - 1),
        category: detail.category
      });
    };

    window.addEventListener(SEARCH_OPEN_MEDIA_VIEWER_EVENT, handler);
    return () => window.removeEventListener(SEARCH_OPEN_MEDIA_VIEWER_EVENT, handler);
  }, []);

  if (!state) return null;

  return (
    <SearchMediaViewerModal
      items={state.items}
      activeIndex={state.activeIndex}
      category={state.category}
      onClose={() => setState(null)}
      onIndexChange={(activeIndex) => setState((prev) => (prev ? { ...prev, activeIndex } : prev))}
      onGoToFile={onGoToFile}
      onOpenWebLink={onOpenWebLink}
    />
  );
}
