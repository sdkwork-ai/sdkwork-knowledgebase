import React, { useEffect, useMemo, useRef, useState } from 'react';
import {
  Camera,
  ExternalLink,
  Fullscreen,
  Gauge,
  ListMusic,
  Loader2,
  Pause,
  PictureInPicture2,
  Play,
  RotateCcw,
  RotateCw,
  SkipBack,
  SkipForward,
  Video
} from 'lucide-react';
import {
  buildContainAspectStyle,
  getMediaOrientation,
  orientationClass,
  resolveVideoDimensions
} from '../../utils/mediaAspect';
import { resolveVideoPlayback } from '../../utils/searchMediaViewerBridge';
import { openExternalUrl } from './openExternalUrl';
import { PlaybackProgressBar } from './shared/PlaybackProgressBar';
import { VolumeVerticalControl } from './shared/VolumeVerticalControl';
import type { SearchMediaViewerContentProps } from './types';
import { formatTime, getPlaybackRates, useHtmlVideoPlayer } from './useHtmlVideoPlayer';

function buildStageAspectStyle(
  width: number,
  height: number,
  options?: { mini?: boolean }
): React.CSSProperties {
  if (options?.mini) {
    return {
      aspectRatio: `${width} / ${height}`,
      width: '100%',
      maxHeight: getMediaOrientation(width, height) === 'portrait' ? '240px' : '180px'
    };
  }

  const orientation = getMediaOrientation(width, height);
  const maxHeight =
    orientation === 'portrait' || orientation === 'ultratall'
      ? 'min(72vh, 640px)'
      : orientation === 'ultrawide'
        ? 'min(52vh, 420px)'
        : orientation === 'square'
          ? 'min(68vh, 560px)'
          : 'min(62vh, 560px)';

  return {
    aspectRatio: `${width} / ${height}`,
    width: orientation === 'portrait' || orientation === 'ultratall' ? 'auto' : '100%',
    maxWidth: '100%',
    maxHeight,
    margin: '0 auto'
  };
}

const SKIP_SECONDS = 10;

export function SearchVideoPlayerContent({
  item,
  layoutMode = 'expanded',
  onOpenWebLink,
  playlist
}: SearchMediaViewerContentProps) {
  const playback = resolveVideoPlayback(item);
  const externalUrl = item.url ?? item.previewUrl;
  const isDirect = playback.mode === 'direct' && Boolean(playback.url);
  const isMini = layoutMode === 'minimized';
  const [volume, setVolume] = useState(0.85);
  const [isMuted, setIsMuted] = useState(false);
  const [isSpeedMenuOpen, setIsSpeedMenuOpen] = useState(false);
  const [snapshotHint, setSnapshotHint] = useState<string | null>(null);
  const speedMenuRef = useRef<HTMLDivElement>(null);
  const snapshotTimerRef = useRef<ReturnType<typeof setTimeout> | null>(null);
  const playbackRates = useMemo(() => getPlaybackRates(), []);

  const frameDims = useMemo(() => resolveVideoDimensions(item), [item]);
  const videoOrientation = useMemo(
    () => getMediaOrientation(frameDims.width, frameDims.height),
    [frameDims.height, frameDims.width]
  );
  const stageAspectStyle = useMemo(() => {
    if (isMini) {
      return buildStageAspectStyle(frameDims.width, frameDims.height, { mini: true });
    }
    return buildContainAspectStyle(frameDims.width, frameDims.height);
  }, [frameDims.height, frameDims.width, isMini]);
  const playerOrientationClass = orientationClass('search-video-player', videoOrientation);

  const {
    videoRef,
    isPlaying,
    hasEnded,
    isBuffering,
    currentTime,
    duration,
    progress,
    buffered,
    playbackRate,
    togglePlay,
    seek,
    seekBy,
    setPlaybackRate,
    canPlay,
    toggleFullscreen,
    togglePictureInPicture,
    captureSnapshot
  } = useHtmlVideoPlayer(isDirect ? playback.url : undefined, item.id, {
    loop: playlist?.playMode === 'loop-one',
    autoPlay: playlist?.autoPlay,
    onEnded: playlist?.onTrackEnded,
    onPlayingChange: (playing) => {
      playlist?.onPlaybackActiveChange?.(playing);
      if (playing) playlist?.onAutoPlayConsumed?.();
    }
  });

  useEffect(() => {
    const video = videoRef.current;
    if (!video) return;
    video.volume = isMuted ? 0 : volume;
    video.muted = isMuted;
  }, [videoRef, isMuted, volume, item.id]);

  useEffect(() => {
    if (!isSpeedMenuOpen) return;
    const onPointerDown = (event: MouseEvent) => {
      if (!speedMenuRef.current?.contains(event.target as Node)) {
        setIsSpeedMenuOpen(false);
      }
    };
    window.addEventListener('mousedown', onPointerDown);
    return () => window.removeEventListener('mousedown', onPointerDown);
  }, [isSpeedMenuOpen]);

  useEffect(() => {
    return () => {
      if (snapshotTimerRef.current) clearTimeout(snapshotTimerRef.current);
    };
  }, []);

  const handleSnapshot = () => {
    const dataUrl = captureSnapshot();
    if (!dataUrl) {
      setSnapshotHint('截图失败');
    } else {
      const link = document.createElement('a');
      link.href = dataUrl;
      link.download = `${item.title || 'snapshot'}-${Math.round(currentTime)}s.png`;
      link.click();
      setSnapshotHint('已保存截图');
    }
    if (snapshotTimerRef.current) clearTimeout(snapshotTimerRef.current);
    snapshotTimerRef.current = setTimeout(() => setSnapshotHint(null), 1800);
  };

  useEffect(() => {
    if (!isDirect || isMini) return;
    const onKeyDown = (event: KeyboardEvent) => {
      const target = event.target as HTMLElement;
      if (target.tagName === 'INPUT' || target.tagName === 'TEXTAREA' || target.isContentEditable) {
        return;
      }
      switch (event.key.toLowerCase()) {
        case ' ':
        case 'k':
          event.preventDefault();
          togglePlay();
          break;
        case 'arrowleft':
          event.preventDefault();
          seekBy(-SKIP_SECONDS);
          break;
        case 'arrowright':
          event.preventDefault();
          seekBy(SKIP_SECONDS);
          break;
        case 'j':
          event.preventDefault();
          seekBy(-SKIP_SECONDS);
          break;
        case 'l':
          event.preventDefault();
          seekBy(SKIP_SECONDS);
          break;
        case 'f':
          event.preventDefault();
          toggleFullscreen();
          break;
        case 'm':
          event.preventDefault();
          setIsMuted((value) => !value);
          break;
        case ',':
          event.preventDefault();
          setPlaybackRate(Math.max(0.5, Number((playbackRate - 0.25).toFixed(2))));
          break;
        case '.':
          event.preventDefault();
          setPlaybackRate(Math.min(2, Number((playbackRate + 0.25).toFixed(2))));
          break;
        default:
          break;
      }
    };
    window.addEventListener('keydown', onKeyDown);
    return () => window.removeEventListener('keydown', onKeyDown);
  }, [isDirect, isMini, togglePlay, seekBy, toggleFullscreen, setPlaybackRate, playbackRate]);

  if (playback.mode === 'embed' && playback.url) {
    return (
      <div
        className={`search-video-player search-video-player--embed ${playerOrientationClass} ${isMini ? 'search-video-player--mini' : ''} ${playlist ? 'search-video-player--with-playlist' : ''}`}
      >
        <div className="search-video-player__stage search-video-player__stage--adaptive">
          <div
            className={`search-video-player__screen search-video-player__screen--embed search-video-player__screen--adaptive`.trim()}
            style={stageAspectStyle}
          >
            <iframe
              key={item.id}
              src={playback.url}
              title={item.title}
              className="search-video-player__iframe"
              allow="accelerometer; autoplay; clipboard-write; encrypted-media; gyroscope; picture-in-picture; web-share"
              allowFullScreen
            />
          </div>
        </div>
      </div>
    );
  }

  if (isDirect) {
    const isActivelyPlaying = isPlaying && !hasEnded;
    const screenStateClass = isActivelyPlaying
      ? 'search-video-player__screen--playing'
      : hasEnded
        ? 'search-video-player__screen--ended'
        : 'search-video-player__screen--paused';

    return (
      <div
        className={`search-video-player search-video-player--native ${playerOrientationClass} ${isMini ? 'search-video-player--mini' : ''} ${playlist ? 'search-video-player--with-playlist' : ''}`}
      >
        <div className="search-video-player__stage search-video-player__stage--native">
          <div
            className={`search-video-player__screen search-video-player__screen--native search-video-player__screen--adaptive ${screenStateClass}`.trim()}
            style={stageAspectStyle}
          >
            <video
              ref={videoRef}
              key={item.id}
              src={playback.url}
              poster={playback.poster}
              playsInline
              preload="metadata"
              className="search-video-player__video"
              onClick={togglePlay}
              onDoubleClick={toggleFullscreen}
            />
            <div className="search-video-player__shade" aria-hidden />
            {isBuffering && isActivelyPlaying && (
              <div className="search-video-player__buffer-indicator" aria-hidden>
                <Loader2 className="w-8 h-8 animate-spin" />
              </div>
            )}
            {snapshotHint && (
              <div className="search-video-player__snapshot-hint" role="status">
                {snapshotHint}
              </div>
            )}
            <div className={`search-video-player__overlay ${isMini ? 'search-video-player__overlay--mini' : ''}`}>
              {!isMini && !isActivelyPlaying && (
                <div className="search-video-player__center-action">
                  <button
                    type="button"
                    className="search-video-player__center-play"
                    onClick={togglePlay}
                    aria-label={hasEnded ? '重播' : '播放'}
                  >
                    {isBuffering && !hasEnded ? (
                      <Loader2 className="w-8 h-8 animate-spin" />
                    ) : hasEnded ? (
                      <RotateCcw className="w-8 h-8" />
                    ) : (
                      <Play className="w-8 h-8 fill-current ml-0.5" />
                    )}
                  </button>
                  {hasEnded && <span className="search-video-player__replay-label">重播</span>}
                </div>
              )}
              <div className="search-video-player__controls-bar">
                <div className={`search-video-player__controls-bottom ${isMini ? 'search-video-player__controls-bottom--mini' : ''}`}>
                  {playlist && !isMini && (
                    <button
                      type="button"
                      className="search-video-player__control-btn search-video-player__control-btn--track"
                      onClick={playlist.onPreviousTrack}
                      disabled={!playlist.canGoPrevious}
                      aria-label="上一个视频"
                    >
                      <SkipBack className="w-4 h-4" />
                    </button>
                  )}
                  {!isMini && (
                    <button
                      type="button"
                      className="search-video-player__control-btn search-video-player__control-btn--skip"
                      onClick={() => seekBy(-SKIP_SECONDS)}
                      disabled={!canPlay}
                      aria-label={`后退 ${SKIP_SECONDS} 秒`}
                      title={`后退 ${SKIP_SECONDS} 秒 (←/J)`}
                    >
                      <RotateCcw className="w-4 h-4" />
                    </button>
                  )}
                  <button type="button" className="search-video-player__control-btn search-video-player__control-btn--play" onClick={togglePlay} disabled={!canPlay} aria-label={isActivelyPlaying ? '暂停' : '播放'}>
                    {isActivelyPlaying ? <Pause className="w-4 h-4" /> : <Play className="w-4 h-4 fill-current" />}
                  </button>
                  {!isMini && (
                    <button
                      type="button"
                      className="search-video-player__control-btn search-video-player__control-btn--skip"
                      onClick={() => seekBy(SKIP_SECONDS)}
                      disabled={!canPlay}
                      aria-label={`快进 ${SKIP_SECONDS} 秒`}
                      title={`快进 ${SKIP_SECONDS} 秒 (→/L)`}
                    >
                      <RotateCw className="w-4 h-4" />
                    </button>
                  )}
                  {playlist && !isMini && (
                    <button
                      type="button"
                      className="search-video-player__control-btn search-video-player__control-btn--track"
                      onClick={playlist.onNextTrack}
                      disabled={!playlist.canGoNext}
                      aria-label="下一个视频"
                    >
                      <SkipForward className="w-4 h-4" />
                    </button>
                  )}
                  {!isMini && <span className="search-video-player__time">{formatTime(currentTime)}</span>}
                  <PlaybackProgressBar
                    progress={progress}
                    buffered={buffered}
                    onSeek={seek}
                    disabled={!canPlay}
                    variant="on-dark"
                  />
                  {!isMini && <span className="search-video-player__time">{formatTime(duration)}</span>}
                  {!isMini && (
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
                  )}
                  {!isMini && (
                    <div className="search-video-player__speed-wrapper" ref={speedMenuRef}>
                      <button
                        type="button"
                        className={`search-video-player__control-btn search-video-player__control-btn--speed ${isSpeedMenuOpen ? 'search-video-player__control-btn--active' : ''}`}
                        onClick={() => setIsSpeedMenuOpen((value) => !value)}
                        aria-label="播放速度"
                        title="播放速度 (, / .)"
                      >
                        <Gauge className="w-4 h-4" />
                        <span className="search-video-player__speed-label">{playbackRate}x</span>
                      </button>
                      {isSpeedMenuOpen && (
                        <div className="search-video-player__speed-menu" role="menu">
                          {playbackRates.map((rate) => (
                            <button
                              key={rate}
                              type="button"
                              role="menuitemradio"
                              aria-checked={playbackRate === rate}
                              className={`search-video-player__speed-option ${playbackRate === rate ? 'search-video-player__speed-option--active' : ''}`}
                              onClick={() => {
                                setPlaybackRate(rate);
                                setIsSpeedMenuOpen(false);
                              }}
                            >
                              {rate === 1 ? '正常' : `${rate}x`}
                              {playbackRate === rate && <span className="search-video-player__speed-check" />}
                            </button>
                          ))}
                        </div>
                      )}
                    </div>
                  )}
                  {!isMini && (
                    <button
                      type="button"
                      className="search-video-player__control-btn"
                      onClick={handleSnapshot}
                      disabled={!canPlay}
                      aria-label="视频截图"
                      title="视频截图"
                    >
                      <Camera className="w-4 h-4" />
                    </button>
                  )}
                  {!isMini && (
                    <>
                      <button type="button" className="search-video-player__control-btn" onClick={togglePictureInPicture} aria-label="画中画" title="画中画">
                        <PictureInPicture2 className="w-4 h-4" />
                      </button>
                      <button type="button" className="search-video-player__control-btn" onClick={toggleFullscreen} aria-label="视频全屏" title="全屏 (F)">
                        <Fullscreen className="w-4 h-4" />
                      </button>
                      {playlist && (
                        <button
                          type="button"
                          className={`search-video-player__control-btn ${playlist.isPlaylistOpen ? 'search-video-player__control-btn--active' : ''}`}
                          onClick={playlist.onPlaylistToggle}
                          aria-label={playlist.isPlaylistOpen ? '收起视频列表' : '展开视频列表'}
                        >
                          <ListMusic className="w-4 h-4" />
                        </button>
                      )}
                    </>
                  )}
                  {isMini && <span className="search-video-player__time search-video-player__time--mini">{formatTime(currentTime)}</span>}
                </div>
              </div>
            </div>
          </div>
        </div>
      </div>
    );
  }

  return (
    <div
      className={`search-video-player search-video-player--fallback ${playerOrientationClass} ${isMini ? 'search-video-player--mini' : ''} ${playlist ? 'search-video-player--with-playlist' : ''}`}
    >
      <div className="search-video-player__stage search-video-player__stage--adaptive">
        <div
          className={`search-video-player__screen search-video-player__screen--fallback search-video-player__screen--adaptive`.trim()}
          style={stageAspectStyle}
        >
          {playback.poster ? (
            <img src={playback.poster} alt={item.title} className="search-video-player__poster" />
          ) : (
            <div className="search-video-player__empty">
              <Video className="w-12 h-12 opacity-35" />
              <p>暂无视频预览</p>
            </div>
          )}
          {externalUrl && (
            <button
              type="button"
              className="search-video-player__center-play"
              onClick={() => openExternalUrl(externalUrl, item.title, onOpenWebLink)}
              aria-label="在浏览器中播放"
            >
              <Play className="w-8 h-8 fill-current ml-0.5" />
            </button>
          )}
        </div>
      </div>
      {isMini && externalUrl && (
        <button
          type="button"
          className="search-media-viewer-link-btn"
          onClick={() => openExternalUrl(externalUrl, item.title, onOpenWebLink)}
        >
          <ExternalLink className="w-3.5 h-3.5" />
          在浏览器中播放
        </button>
      )}
    </div>
  );
}
