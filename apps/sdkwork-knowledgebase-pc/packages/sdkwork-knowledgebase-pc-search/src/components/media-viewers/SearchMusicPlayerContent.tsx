import { useCallback, useEffect, useMemo, useState } from 'react';
import {
  Disc3,
  ExternalLink,
  ListMusic,
  Loader2,
  Pause,
  Play,
  RotateCcw,
  RotateCw,
  SkipBack,
  SkipForward
} from 'lucide-react';
import { resolveAudioPlayback } from '../../utils/searchMediaViewerBridge';
import { findActiveTimedLineIndex, hasSyncedTimedText } from '../../utils/mediaTimedText';
import { openExternalUrl } from './openExternalUrl';
import { MediaSourceChip } from './shared/MediaSourceChip';
import { MediaTimedTextPanel } from './shared/MediaTimedTextPanel';
import { PlaybackProgressBar } from './shared/PlaybackProgressBar';
import { SearchMediaMiniDock } from './shared/SearchMediaMiniDock';
import { VolumeVerticalControl } from './shared/VolumeVerticalControl';
import type { SearchMediaViewerContentProps } from './types';
import { formatTime, useHtmlMediaPlayer } from './useHtmlMediaPlayer';

function MusicTurntable({
  thumbnailUrl,
  title,
  isPlaying,
  size = 'default'
}: {
  thumbnailUrl?: string;
  title: string;
  isPlaying: boolean;
  size?: 'default' | 'large';
}) {
  return (
    <div
      className={`search-music-player-shell__turntable ${size === 'large' ? 'search-music-player-shell__turntable--large' : ''}`}
    >
      <div className={`search-music-player-shell__disc ${isPlaying ? 'search-music-player-shell__disc--spinning' : ''}`}>
        <div className="search-music-player-shell__disc-groove" aria-hidden />
        <div className="search-music-player-shell__disc-inner">
          {thumbnailUrl ? (
            <img src={thumbnailUrl} alt={title} />
          ) : (
            <Disc3 className="w-14 h-14 opacity-50" />
          )}
        </div>
        <div className="search-music-player-shell__disc-hole" aria-hidden />
      </div>
      <div
        className={`search-music-player-shell__tonearm ${isPlaying ? 'search-music-player-shell__tonearm--playing' : ''}`}
        aria-hidden
      />
    </div>
  );
}

export function SearchMusicPlayerContent({ item, layoutMode = 'expanded', onOpenWebLink, playlist }: SearchMediaViewerContentProps) {
  const playbackUrl = resolveAudioPlayback(item);
  const externalUrl = item.url ?? item.previewUrl;
  const isMini = layoutMode === 'minimized';
  const hasLyrics = hasSyncedTimedText(item.lyrics);
  const [volume, setVolume] = useState(0.85);
  const [isMuted, setIsMuted] = useState(false);

  const {
    audioRef,
    isPlaying,
    hasEnded,
    isBuffering,
    currentTime,
    duration,
    progress,
    buffered,
    togglePlay,
    seek,
    canPlay
  } = useHtmlMediaPlayer(playbackUrl, item.id, {
    loop: playlist?.playMode === 'loop-one',
    autoPlay: playlist?.autoPlay,
    onEnded: playlist?.onTrackEnded,
    onPlayingChange: (playing) => {
      playlist?.onPlaybackActiveChange?.(playing);
      if (playing) playlist?.onAutoPlayConsumed?.();
    }
  });

  useEffect(() => {
    const audio = audioRef.current;
    if (!audio) return;
    audio.volume = isMuted ? 0 : volume;
  }, [audioRef, isMuted, volume]);

  const seekToTime = useCallback(
    (seconds: number) => {
      if (!duration) return;
      seek(Math.max(0, Math.min(1, seconds / duration)));
    },
    [duration, seek]
  );

  const skip = (delta: number) => {
    const audio = audioRef.current;
    if (!audio) return;
    audio.currentTime = Math.max(0, Math.min(audio.duration || 0, audio.currentTime + delta));
  };

  const playLabel = hasEnded ? '重播' : isPlaying ? '暂停' : '播放';
  const statusLabel = hasEnded ? '播放结束' : isBuffering ? '缓冲中…' : isPlaying ? '正在播放' : '音乐';

  const activeLyric = useMemo(() => {
    if (!item.lyrics?.length) return undefined;
    const index = findActiveTimedLineIndex(item.lyrics, currentTime);
    return index >= 0 ? item.lyrics[index].text : undefined;
  }, [currentTime, item.lyrics]);

  const shellClass = [
    'search-music-player-shell',
    isMini ? 'search-music-player-shell--mini' : 'search-music-player-shell--pro',
    !isMini && hasLyrics && 'search-music-player-shell--with-lyrics'
  ]
    .filter(Boolean)
    .join(' ');

  const metaBlock = (
    <div className="search-music-player-shell__meta">
      <p className="search-music-player-shell__label">
        {isPlaying && !hasEnded && !isBuffering && <span className="search-music-player-shell__live-dot" aria-hidden />}
        {statusLabel}
      </p>
      <h3 className="search-music-player-shell__title">{item.title}</h3>
      <p className="search-music-player-shell__artist">{item.artist ?? '未知艺术家'}</p>
      {item.snippet && <p className="search-music-player-shell__album">{item.snippet}</p>}
      {hasLyrics && activeLyric && isPlaying && (
        <p className="search-music-player-shell__live-lyric" aria-live="polite">
          {activeLyric}
        </p>
      )}
    </div>
  );

  return (
    <div className={shellClass}>
      {playbackUrl && <audio ref={audioRef} src={playbackUrl} preload="metadata" className="sr-only" />}

      {isMini && playbackUrl ? (
        <SearchMediaMiniDock
          item={item}
          kind="music"
          isPlaying={isPlaying}
          hasEnded={hasEnded}
          isBuffering={isBuffering}
          progress={progress}
          buffered={buffered}
          currentTime={currentTime}
          duration={duration}
          canPlay={canPlay}
          onTogglePlay={togglePlay}
          onSeek={seek}
          volume={volume}
          isMuted={isMuted}
          onVolumeChange={(next) => {
            setIsMuted(false);
            setVolume(next);
          }}
          onMuteToggle={() => setIsMuted((value) => !value)}
          liveCaption={activeLyric}
          playlist={playlist}
        />
      ) : (
        <>
          {item.thumbnailUrl && (
            <div
              className="search-music-player-shell__backdrop"
              style={{ backgroundImage: `url(${item.thumbnailUrl})` }}
              aria-hidden
            />
          )}
          <div className="search-music-player-shell__mesh" aria-hidden />
          <div className="search-music-player-shell__glow" aria-hidden />

          <div className="search-music-player-shell__content">
            <div className="search-music-player-shell__header">
              <MediaSourceChip item={item} variant="on-dark" />
              {hasLyrics && (
                <span className="search-music-player-shell__badge search-music-player-shell__badge--lyrics">同步歌词</span>
              )}
              {item.duration && <span className="search-music-player-shell__badge">{item.duration}</span>}
            </div>

            {hasLyrics ? (
              <div className="search-music-player-shell__body search-music-player-shell__body--with-lyrics">
                <aside className="search-music-player-shell__left-pane">
                  <MusicTurntable
                    thumbnailUrl={item.thumbnailUrl}
                    title={item.title}
                    isPlaying={isPlaying}
                    size="large"
                  />
                  {metaBlock}
                </aside>
                {item.lyrics && (
                  <MediaTimedTextPanel
                    lines={item.lyrics}
                    currentTime={currentTime}
                    onSeekToTime={seekToTime}
                    variant="lyrics"
                    lyricsFocus
                    isPlaying={isPlaying}
                    disabled={!canPlay}
                  />
                )}
              </div>
            ) : (
              <div className="search-music-player-shell__stage">
                <MusicTurntable thumbnailUrl={item.thumbnailUrl} title={item.title} isPlaying={isPlaying} />
                {metaBlock}
                <div
                  className={`search-music-player-shell__visualizer ${isPlaying ? 'search-music-player-shell__visualizer--active' : ''}`}
                  aria-hidden
                >
                  {Array.from({ length: 14 }, (_, index) => (
                    <span key={index} className="search-music-player-shell__viz-bar" style={{ animationDelay: `${index * 0.055}s` }} />
                  ))}
                </div>
              </div>
            )}
          </div>

          {playbackUrl ? (
            <div className="search-music-player-shell__dock">
              <div className="search-music-player-shell__timeline">
                <span className="search-music-player-shell__time">{formatTime(currentTime)}</span>
                <PlaybackProgressBar
                  progress={progress}
                  buffered={buffered}
                  onSeek={seek}
                  disabled={!canPlay}
                  variant="on-dark"
                />
                <span className="search-music-player-shell__time">{formatTime(duration)}</span>
              </div>

              <div className="search-music-player-shell__transport">
                <div className="search-music-player-shell__controls">
                  {playlist && (
                    <button
                      type="button"
                      className="search-music-player-shell__icon-btn search-music-player-shell__icon-btn--track"
                      onClick={playlist.onPreviousTrack}
                      disabled={!playlist.canGoPrevious}
                      aria-label="上一首"
                    >
                      <SkipBack className="w-4 h-4" />
                    </button>
                  )}
                  <button
                    type="button"
                    className="search-music-player-shell__icon-btn search-music-player-shell__icon-btn--seek"
                    onClick={() => skip(-10)}
                    aria-label="后退 10 秒"
                  >
                    <RotateCcw className="w-4 h-4" />
                    <span>10</span>
                  </button>
                  <button
                    type="button"
                    className="search-music-player-shell__play-btn search-music-player-shell__play-btn--large"
                    onClick={togglePlay}
                    disabled={!canPlay}
                    aria-label={playLabel}
                  >
                    {isBuffering ? (
                      <Loader2 className="w-7 h-7 animate-spin" />
                    ) : isPlaying ? (
                      <Pause className="w-7 h-7" />
                    ) : (
                      <Play className="w-7 h-7 fill-current ml-0.5" />
                    )}
                  </button>
                  <button
                    type="button"
                    className="search-music-player-shell__icon-btn search-music-player-shell__icon-btn--seek"
                    onClick={() => skip(10)}
                    aria-label="前进 10 秒"
                  >
                    <RotateCw className="w-4 h-4" />
                    <span>10</span>
                  </button>
                  {playlist && (
                    <button
                      type="button"
                      className="search-music-player-shell__icon-btn search-music-player-shell__icon-btn--track"
                      onClick={playlist.onNextTrack}
                      disabled={!playlist.canGoNext}
                      aria-label="下一首"
                    >
                      <SkipForward className="w-4 h-4" />
                    </button>
                  )}
                </div>
                <div className="search-music-player-shell__transport-side">
                  {playlist && (
                    <button
                      type="button"
                      className={`search-music-player-shell__icon-btn ${playlist.isPlaylistOpen ? 'search-music-player-shell__icon-btn--active' : ''}`}
                      onClick={playlist.onPlaylistToggle}
                      aria-label={playlist.isPlaylistOpen ? '收起播放列表' : '展开播放列表'}
                      title="播放列表"
                    >
                      <ListMusic className="w-4 h-4" />
                    </button>
                  )}
                  <VolumeVerticalControl
                  volume={volume}
                  isMuted={isMuted}
                  onVolumeChange={(next) => {
                    setIsMuted(false);
                    setVolume(next);
                  }}
                  onMuteToggle={() => setIsMuted((value) => !value)}
                  variant="on-dark"
                />
                </div>
              </div>
            </div>
          ) : externalUrl ? (
            <button
              type="button"
              className="search-media-viewer-link-btn"
              onClick={() => openExternalUrl(externalUrl, item.title, onOpenWebLink)}
            >
              <ExternalLink className="w-3.5 h-3.5" />
              在浏览器中播放
            </button>
          ) : (
            <p className="search-media-viewer-empty-inline">暂无可用音乐流</p>
          )}
        </>
      )}
    </div>
  );
}
