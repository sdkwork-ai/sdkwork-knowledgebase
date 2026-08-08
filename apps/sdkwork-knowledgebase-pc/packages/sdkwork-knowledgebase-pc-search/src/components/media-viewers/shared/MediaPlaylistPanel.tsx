import { useEffect, useRef } from 'react';
import { Disc3, Headphones, ListMusic, Repeat, Repeat1, Shuffle, Video } from 'lucide-react';
import type { SearchMediaItem } from '../../../types';
import { hasSyncedTimedText } from '../../../utils/mediaTimedText';
import type { MediaPlayMode } from '../../../utils/mediaPlaylist';
import { cyclePlayMode, playModeLabel } from '../../../utils/mediaPlaylist';

export interface MediaPlaylistPanelProps {
  items: SearchMediaItem[];
  activeIndex: number;
  category: 'music' | 'audio' | 'video';
  playMode: MediaPlayMode;
  shuffleEnabled: boolean;
  isPlaying?: boolean;
  onSelect: (index: number) => void;
  onPlayModeChange: (mode: MediaPlayMode) => void;
  onShuffleToggle: () => void;
  onClose?: () => void;
}

function trackSubtitle(item: SearchMediaItem, category: 'music' | 'audio' | 'video'): string {
  if (category === 'music') return item.artist ?? item.snippet ?? '未知艺术家';
  if (category === 'video') return item.snippet ?? item.duration ?? '视频';
  return item.snippet ?? item.audioKind ?? '音频';
}

function trackBadge(item: SearchMediaItem, category: 'music' | 'audio' | 'video'): string | null {
  if (category === 'music' && hasSyncedTimedText(item.lyrics)) return '歌词';
  if (category === 'audio' && hasSyncedTimedText(item.transcript)) {
    return item.audioKind === 'recording' || item.audioKind === 'speech' ? '纪要' : '字幕';
  }
  if (category === 'video' && item.videoWidth && item.videoHeight) {
    return `${item.videoWidth}×${item.videoHeight}`;
  }
  return null;
}

function playlistTitle(category: 'music' | 'audio' | 'video'): string {
  if (category === 'video') return '视频列表';
  return '播放列表';
}

function playlistCountLabel(category: 'music' | 'audio' | 'video', count: number): string {
  if (category === 'video') return `${count} 个`;
  return `${count} 首`;
}

export function MediaPlaylistPanel({
  items,
  activeIndex,
  category,
  playMode,
  shuffleEnabled,
  isPlaying = false,
  onSelect,
  onPlayModeChange,
  onShuffleToggle,
  onClose
}: MediaPlaylistPanelProps) {
  const activeRowRef = useRef<HTMLButtonElement>(null);
  const FallbackIcon = category === 'music' ? Disc3 : category === 'video' ? Video : Headphones;

  useEffect(() => {
    activeRowRef.current?.scrollIntoView({ behavior: 'smooth', block: 'nearest' });
  }, [activeIndex]);

  const repeatActive = playMode === 'loop-all' || playMode === 'loop-one';

  return (
    <aside
      className={[
        'search-media-playlist',
        category === 'video' && 'search-media-playlist--video',
        category === 'music' && 'search-media-playlist--music',
        category === 'audio' && 'search-media-playlist--audio'
      ]
        .filter(Boolean)
        .join(' ')}
      aria-label={playlistTitle(category)}
    >
      <header className="search-media-playlist__header">
        <div className="search-media-playlist__title-row">
          {category === 'video' ? <Video className="w-4 h-4" /> : <ListMusic className="w-4 h-4" />}
          <span className="search-media-playlist__title">{playlistTitle(category)}</span>
          <span className="search-media-playlist__count">{playlistCountLabel(category, items.length)}</span>
        </div>
        <div className="search-media-playlist__modes">
          <button
            type="button"
            className={`search-media-playlist__mode-btn ${shuffleEnabled ? 'search-media-playlist__mode-btn--active' : ''}`}
            onClick={onShuffleToggle}
            title={shuffleEnabled ? '关闭随机' : '随机播放'}
            aria-label={shuffleEnabled ? '关闭随机播放' : '随机播放'}
            aria-pressed={shuffleEnabled}
          >
            <Shuffle className="w-3.5 h-3.5" />
          </button>
          <button
            type="button"
            className={`search-media-playlist__mode-btn ${repeatActive ? 'search-media-playlist__mode-btn--active' : ''}`}
            onClick={() => onPlayModeChange(cyclePlayMode(playMode))}
            title={playModeLabel(playMode)}
            aria-label={`播放模式：${playModeLabel(playMode)}`}
          >
            {playMode === 'loop-one' ? <Repeat1 className="w-3.5 h-3.5" /> : <Repeat className="w-3.5 h-3.5" />}
          </button>
          {onClose && (
            <button type="button" className="search-media-playlist__mode-btn" onClick={onClose} title="收起列表" aria-label="收起播放列表">
              ×
            </button>
          )}
        </div>
      </header>

      <div className="search-media-playlist__list" role="list">
        {items.map((track, index) => {
          const isActive = index === activeIndex;
          const badge = trackBadge(track, category);
          return (
            <button
              key={track.id}
              ref={isActive ? activeRowRef : undefined}
              type="button"
              role="listitem"
              className={[
                'search-media-playlist__row',
                isActive && 'search-media-playlist__row--active',
                isActive && isPlaying && 'search-media-playlist__row--playing'
              ]
                .filter(Boolean)
                .join(' ')}
              onClick={() => onSelect(index)}
              aria-current={isActive ? 'true' : undefined}
            >
              <span className="search-media-playlist__index">
                {isActive && isPlaying ? (
                  <span className="search-media-playlist__eq" aria-hidden>
                    <span />
                    <span />
                    <span />
                  </span>
                ) : (
                  index + 1
                )}
              </span>
              <span className="search-media-playlist__cover">
                {track.thumbnailUrl ? (
                  <img src={track.thumbnailUrl} alt="" loading="lazy" />
                ) : (
                  <FallbackIcon className="w-4 h-4 opacity-45" />
                )}
              </span>
              <span className="search-media-playlist__meta">
                <span className="search-media-playlist__track-title" title={track.title}>
                  {track.title}
                </span>
                <span className="search-media-playlist__track-sub" title={trackSubtitle(track, category)}>
                  {trackSubtitle(track, category)}
                  {badge && <span className="search-media-playlist__track-badge">{badge}</span>}
                </span>
              </span>
              {track.duration && <span className="search-media-playlist__duration">{track.duration}</span>}
            </button>
          );
        })}
      </div>
    </aside>
  );
}
