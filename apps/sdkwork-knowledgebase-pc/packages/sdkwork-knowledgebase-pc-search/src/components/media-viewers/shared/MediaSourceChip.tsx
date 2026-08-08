import { ExternalLink, Library } from 'lucide-react';
import type { SearchMediaItem } from '../../../types';

export function MediaSourceChip({
  item,
  variant = 'default'
}: {
  item: SearchMediaItem;
  variant?: 'default' | 'on-dark';
}) {
  const isKb = item.source === 'kb';

  return (
    <span
      className={`search-media-source-chip ${isKb ? 'search-media-source-chip--kb' : 'search-media-source-chip--web'} ${variant === 'on-dark' ? 'search-media-source-chip--on-dark' : ''}`}
    >
      {isKb ? <Library className="w-3 h-3" /> : <ExternalLink className="w-3 h-3" />}
      {isKb ? '知识库' : '网络来源'}
    </span>
  );
}
