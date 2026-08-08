import type { SearchMediaItem } from '../types';
import { SearchMediaViewerContentRouter } from './media-viewers/SearchMediaViewerContentRouter';
import { MediaPlaylistPanel } from './media-viewers/shared/MediaPlaylistPanel';
import type { MediaPlaylistControls } from './media-viewers/types';
import type { MediaPlayMode } from '../utils/mediaPlaylist';

export interface SearchMediaViewerPlaylistBodyProps {
  item: SearchMediaItem;
  items: SearchMediaItem[];
  category: 'music' | 'audio' | 'video';
  activeIndex: number;
  isMinimized: boolean;
  isPlaylistOpen: boolean;
  isPlaying: boolean;
  playMode: MediaPlayMode;
  shuffleEnabled: boolean;
  hasLyrics: boolean;
  hasTranscript: boolean;
  contentSlideClass: string;
  playlistControls?: MediaPlaylistControls;
  onOpenWebLink?: (url: string, title?: string) => void;
  onPlayModeChange: (mode: MediaPlayMode) => void;
  onShuffleToggle: () => void;
  onPlaylistClose: () => void;
  onPlaylistSelect: (index: number) => void;
}

/** Playlist body for music / audio / video — gallery image & product keep the standard body. */
export function SearchMediaViewerPlaylistBody({
  item,
  items,
  category,
  activeIndex,
  isMinimized,
  isPlaylistOpen,
  isPlaying,
  playMode,
  shuffleEnabled,
  hasLyrics,
  hasTranscript,
  contentSlideClass,
  playlistControls,
  onOpenWebLink,
  onPlayModeChange,
  onShuffleToggle,
  onPlaylistClose,
  onPlaylistSelect
}: SearchMediaViewerPlaylistBodyProps) {
  const showPlaylist = items.length > 1 && isPlaylistOpen && !isMinimized;

  return (
    <>
      <div
        className={[
          'search-media-viewer-body',
          isMinimized && 'search-media-viewer-body--mini',
          showPlaylist && 'search-media-viewer-body--with-playlist',
          showPlaylist && category === 'video' && 'search-media-viewer-body--video-playlist'
        ]
          .filter(Boolean)
          .join(' ')}
      >
        <div className="search-media-viewer-player-pane">
          <div
            className={[
              `search-media-viewer-content search-media-viewer-content--${category}`,
              hasTranscript && 'search-media-viewer-content--with-transcript',
              hasLyrics && 'search-media-viewer-content--with-lyrics',
              contentSlideClass
            ]
              .filter(Boolean)
              .join(' ')}
            key={item.id}
          >
            <SearchMediaViewerContentRouter
              item={item}
              category={category}
              layoutMode={isMinimized ? 'minimized' : 'expanded'}
              onOpenWebLink={onOpenWebLink}
              playlist={playlistControls}
            />
          </div>
        </div>

        {showPlaylist && (
          <MediaPlaylistPanel
            items={items}
            activeIndex={activeIndex}
            category={category}
            playMode={playMode}
            shuffleEnabled={shuffleEnabled}
            isPlaying={isPlaying}
            onSelect={onPlaylistSelect}
            onPlayModeChange={onPlayModeChange}
            onShuffleToggle={onShuffleToggle}
            onClose={onPlaylistClose}
          />
        )}
      </div>

      {!isMinimized && items.length > 1 && (
        <footer className="search-media-viewer-footer search-media-viewer-footer--playlist">
          <p className="search-media-viewer-kbd-hint">
            <kbd>Shift</kbd>+<kbd>←</kbd>
            <kbd>→</kbd>
            {category === 'video' ? '上/下一个视频' : '上/下一首'}
            {category === 'video' && (
              <>
                <span className="search-media-viewer-kbd-sep" />
                <kbd>F</kbd>
                全屏
              </>
            )}
            <span className="search-media-viewer-kbd-sep" />
            <kbd>Esc</kbd>
            关闭
          </p>
        </footer>
      )}
    </>
  );
}
