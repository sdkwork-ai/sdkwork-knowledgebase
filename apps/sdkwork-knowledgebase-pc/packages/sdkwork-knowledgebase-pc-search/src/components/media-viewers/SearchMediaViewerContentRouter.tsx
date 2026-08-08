import type { SearchMediaCategory, SearchMediaItem } from '../../types';
import type { MediaPlaylistControls } from './types';
import { SearchAudioPlayerContent } from './SearchAudioPlayerContent';
import { SearchImageViewerContent } from './SearchImageViewerContent';
import { SearchMusicPlayerContent } from './SearchMusicPlayerContent';
import { SearchProductViewerContent } from './SearchProductViewerContent';
import { SearchVideoPlayerContent } from './SearchVideoPlayerContent';
import type { MediaViewerLayoutMode } from './types';

export interface SearchMediaViewerContentRouterProps {
  item: SearchMediaItem;
  category: SearchMediaCategory;
  layoutMode?: MediaViewerLayoutMode;
  onOpenWebLink?: (url: string, title?: string) => void;
  playlist?: MediaPlaylistControls;
}

export function SearchMediaViewerContentRouter({
  item,
  category,
  layoutMode = 'expanded',
  onOpenWebLink,
  playlist
}: SearchMediaViewerContentRouterProps) {
  switch (category) {
    case 'image':
      return <SearchImageViewerContent item={item} layoutMode={layoutMode} onOpenWebLink={onOpenWebLink} />;
    case 'video':
      return <SearchVideoPlayerContent item={item} layoutMode={layoutMode} onOpenWebLink={onOpenWebLink} />;
    case 'audio':
      return (
        <SearchAudioPlayerContent
          item={item}
          layoutMode={layoutMode}
          onOpenWebLink={onOpenWebLink}
          playlist={playlist}
        />
      );
    case 'music':
      return (
        <SearchMusicPlayerContent
          item={item}
          layoutMode={layoutMode}
          onOpenWebLink={onOpenWebLink}
          playlist={playlist}
        />
      );
    case 'product':
      return <SearchProductViewerContent item={item} layoutMode={layoutMode} onOpenWebLink={onOpenWebLink} />;
    default:
      return null;
  }
}
