import { Headphones, Loader2, Pause, Play, SkipBack, SkipForward } from 'lucide-react';
import type { SearchMediaItem } from '../../../types';
import type { MediaPlaylistControls } from '../types';
import { PlaybackProgressBar } from './PlaybackProgressBar';
import { VolumeVerticalControl } from './VolumeVerticalControl';
import { formatTime } from '../useHtmlMediaPlayer';

export interface SearchMediaMiniDockProps {
  item: SearchMediaItem;
  kind: 'audio' | 'music';
  isPlaying: boolean;
  hasEnded: boolean;
  isBuffering: boolean;
  progress: number;
  buffered: number;
  currentTime: number;
  duration: number;
  canPlay: boolean;
  onTogglePlay: () => void;
  onSeek: (ratio: number) => void;
  volume: number;
  isMuted: boolean;
  onVolumeChange: (value: number) => void;
  onMuteToggle: () => void;
  liveCaption?: string;
  playlist?: MediaPlaylistControls;
}

export function SearchMediaMiniDock({
  item,
  kind,
  isPlaying,
  hasEnded,
  isBuffering,
  progress,
  buffered,
  currentTime,
  duration,
  canPlay,
  onTogglePlay,
  onSeek,
  volume,
  isMuted,
  onVolumeChange,
  onMuteToggle,
  liveCaption,
  playlist
}: SearchMediaMiniDockProps) {
  const playLabel = hasEnded ? '重播' : isPlaying ? '暂停' : '播放';
  const fallbackSubtitle = kind === 'music' ? (item.artist ?? '未知艺术家') : (item.snippet ?? '播客 / 有声内容');
  const subtitle = liveCaption ?? fallbackSubtitle;

  return (
    <div className="search-media-mini-dock">
      <div className="search-media-mini-dock__main">
        <div className="search-media-mini-dock__cover">
          {item.thumbnailUrl ? (
            <img src={item.thumbnailUrl} alt={item.title} />
          ) : (
            <Headphones className="w-5 h-5 opacity-50" />
          )}
          {isPlaying && <span className="search-media-mini-dock__cover-pulse" aria-hidden />}
        </div>

        <div className="search-media-mini-dock__meta">
          <p className="search-media-mini-dock__title" title={item.title}>
            {item.title}
          </p>
          <p className={`search-media-mini-dock__subtitle ${liveCaption ? 'search-media-mini-dock__subtitle--live' : ''}`} title={subtitle}>
            {subtitle}
          </p>
        </div>

        <button
          type="button"
          className="search-media-mini-dock__play-btn"
          onClick={onTogglePlay}
          disabled={!canPlay}
          aria-label={playLabel}
        >
          {isBuffering ? (
            <Loader2 className="w-4 h-4 animate-spin" />
          ) : isPlaying ? (
            <Pause className="w-4 h-4" />
          ) : (
            <Play className="w-4 h-4 fill-current ml-0.5" />
          )}
        </button>

        {playlist && (
          <>
            <button
              type="button"
              className="search-media-mini-dock__track-btn"
              onClick={playlist.onPreviousTrack}
              disabled={!playlist.canGoPrevious}
              aria-label="上一首"
            >
              <SkipBack className="w-3.5 h-3.5" />
            </button>
            <button
              type="button"
              className="search-media-mini-dock__track-btn"
              onClick={playlist.onNextTrack}
              disabled={!playlist.canGoNext}
              aria-label="下一首"
            >
              <SkipForward className="w-3.5 h-3.5" />
            </button>
          </>
        )}

        <VolumeVerticalControl
          volume={volume}
          isMuted={isMuted}
          onVolumeChange={onVolumeChange}
          onMuteToggle={onMuteToggle}
          variant="on-dark"
        />
      </div>

      <div className="search-media-mini-dock__timeline">
        <span className="search-media-mini-dock__time">{formatTime(currentTime)}</span>
        <PlaybackProgressBar progress={progress} buffered={buffered} onSeek={onSeek} disabled={!canPlay} variant="on-dark" />
        <span className="search-media-mini-dock__time">{formatTime(duration)}</span>
      </div>
    </div>
  );
}
