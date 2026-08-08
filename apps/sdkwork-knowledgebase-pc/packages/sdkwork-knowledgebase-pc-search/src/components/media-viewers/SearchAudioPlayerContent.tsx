import { useCallback, useEffect, useMemo, useState } from 'react';
import {
  ExternalLink,
  Headphones,
  Loader2,
  Mic,
  Pause,
  Play,
  RotateCcw,
  RotateCw
} from 'lucide-react';
import { resolveAudioPlayback } from '../../utils/searchMediaViewerBridge';
import { findActiveTimedLineIndex, hasSyncedTimedText } from '../../utils/mediaTimedText';
import {
  buildAspectRatioStyle,
  getCoverShapeLabel,
  getMediaOrientation,
  orientationClass,
  resolveCoverDimensions
} from '../../utils/mediaAspect';
import { openExternalUrl } from './openExternalUrl';
import { MediaSourceChip } from './shared/MediaSourceChip';
import { MediaTimedTextPanel } from './shared/MediaTimedTextPanel';
import { PlaybackProgressBar } from './shared/PlaybackProgressBar';
import { SearchMediaMiniDock } from './shared/SearchMediaMiniDock';
import { VolumeVerticalControl } from './shared/VolumeVerticalControl';
import type { SearchMediaViewerContentProps } from './types';
import { formatTime, useHtmlMediaPlayer } from './useHtmlMediaPlayer';

const SPEED_OPTIONS = [1, 1.25, 1.5, 2] as const;

const AUDIO_KIND_LABEL: Record<string, string> = {
  podcast: '播客',
  recording: '录音',
  speech: '语音'
};

export function SearchAudioPlayerContent({ item, layoutMode = 'expanded', onOpenWebLink }: SearchMediaViewerContentProps) {
  const playbackUrl = resolveAudioPlayback(item);
  const externalUrl = item.url ?? item.previewUrl;
  const isMini = layoutMode === 'minimized';
  const hasTranscript = hasSyncedTimedText(item.transcript);
  const isRecording = item.audioKind === 'recording' || item.audioKind === 'speech';
  const coverDims = useMemo(() => resolveCoverDimensions(item), [item]);
  const coverOrientation = useMemo(
    () => getMediaOrientation(coverDims.width, coverDims.height),
    [coverDims.height, coverDims.width]
  );
  const coverShapeLabel = useMemo(
    () => getCoverShapeLabel(coverDims.width, coverDims.height),
    [coverDims.height, coverDims.width]
  );
  const coverStyle = useMemo(
    () => buildAspectRatioStyle(coverDims.width, coverDims.height),
    [coverDims.height, coverDims.width]
  );
  const [speedIndex, setSpeedIndex] = useState(0);
  const [volume, setVolume] = useState(0.9);
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
  } = useHtmlMediaPlayer(playbackUrl, item.id);

  const playbackRate = SPEED_OPTIONS[speedIndex];

  useEffect(() => {
    const audio = audioRef.current;
    if (audio) {
      audio.playbackRate = playbackRate;
    }
  }, [audioRef, playbackRate, item.id]);

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
  const statusLabel = hasEnded
    ? '播放结束'
    : isBuffering
      ? '缓冲中…'
      : isPlaying
        ? isRecording
          ? '录音播放中'
          : '正在播放'
        : isRecording
          ? '录音 / 会议音频'
          : '播客 / 有声内容';

  const activeCaption = useMemo(() => {
    if (!item.transcript?.length) return undefined;
    const index = findActiveTimedLineIndex(item.transcript, currentTime);
    return index >= 0 ? item.transcript[index].text : undefined;
  }, [currentTime, item.transcript]);

  const shellClass = [
    'search-audio-player',
    isMini ? 'search-audio-player--mini' : 'search-audio-player--pro',
    !isMini && hasTranscript && 'search-audio-player--with-transcript',
    !isMini && orientationClass('search-audio-player', coverOrientation)
  ]
    .filter(Boolean)
    .join(' ');

  const sidebar = (
    <aside className={`search-audio-player__sidebar ${hasTranscript ? 'search-audio-player__sidebar--split' : ''}`}>
      <div
        className={`search-audio-player__cover search-audio-player__cover--${coverOrientation} ${isPlaying ? 'search-audio-player__cover--playing' : ''}`}
        style={coverStyle}
      >
        {item.thumbnailUrl ? (
          <img src={item.thumbnailUrl} alt={item.title} />
        ) : isRecording ? (
          <Mic className="w-12 h-12 opacity-45" />
        ) : (
          <Headphones className="w-12 h-12 opacity-45" />
        )}
        <span className="search-audio-player__cover-ring" aria-hidden />
        {isPlaying && <span className="search-audio-player__cover-glow" aria-hidden />}
      </div>

      <div className="search-audio-player__main">
        <div className="search-audio-player__badges">
          <MediaSourceChip item={item} variant={hasTranscript ? 'on-dark' : undefined} />
          {item.audioKind && (
            <span className="search-audio-player__kind-badge">{AUDIO_KIND_LABEL[item.audioKind] ?? item.audioKind}</span>
          )}
          {hasTranscript && (
            <span className="search-audio-player__kind-badge search-audio-player__kind-badge--caption">
              {isRecording ? '会议纪要' : '字幕'}
            </span>
          )}
          {!hasTranscript && (
            <span className="search-audio-player__kind-badge search-audio-player__kind-badge--shape">{coverShapeLabel}</span>
          )}
          {item.duration && <span className="search-audio-player__duration-badge">{item.duration}</span>}
        </div>
        <p className="search-audio-player__label">{statusLabel}</p>
        <h3 className="search-audio-player__title">{item.title}</h3>
        {item.snippet && !hasTranscript && <p className="search-audio-player__snippet">{item.snippet}</p>}
        {hasTranscript && activeCaption && isPlaying && (
          <p className="search-audio-player__live-caption" aria-live="polite">
            {activeCaption}
          </p>
        )}

        {!hasTranscript && (
          <div className={`search-audio-player__wave ${isPlaying ? 'search-audio-player__wave--active' : ''}`} aria-hidden>
            {Array.from({ length: 18 }, (_, index) => (
              <span key={index} className="search-audio-player__bar" style={{ animationDelay: `${index * 0.04}s` }} />
            ))}
          </div>
        )}
      </div>
    </aside>
  );

  return (
    <div className={shellClass}>
      {playbackUrl && <audio ref={audioRef} src={playbackUrl} preload="metadata" className="sr-only" />}

      {isMini && playbackUrl ? (
        <SearchMediaMiniDock
          item={item}
          kind="audio"
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
          liveCaption={activeCaption}
        />
      ) : (
        <div className="search-audio-player__card">
          {hasTranscript && item.thumbnailUrl && (
            <div
              className="search-audio-player__backdrop"
              style={{ backgroundImage: `url(${item.thumbnailUrl})` }}
              aria-hidden
            />
          )}

          <div className="search-audio-player__card-inner">
            <div className={`search-audio-player__body ${hasTranscript ? 'search-audio-player__body--split' : ''}`}>
              {hasTranscript ? (
                <>
                  {sidebar}
                  {item.transcript && (
                    <MediaTimedTextPanel
                      lines={item.transcript}
                      currentTime={currentTime}
                      onSeekToTime={seekToTime}
                      variant="transcript"
                      panelTitle={isRecording ? '会议纪要' : undefined}
                      showSpeakers={isRecording}
                      isPlaying={isPlaying}
                      disabled={!canPlay}
                    />
                  )}
                </>
              ) : (
                <div className="search-audio-player__hero">{sidebar}</div>
              )}
            </div>

            {playbackUrl ? (
              <div className="search-audio-player__dock">
                <div className="search-audio-player__timeline">
                  <span className="search-audio-player__time">{formatTime(currentTime)}</span>
                  <PlaybackProgressBar
                    progress={progress}
                    buffered={buffered}
                    onSeek={seek}
                    disabled={!canPlay}
                    variant={hasTranscript ? 'on-dark' : undefined}
                  />
                  <span className="search-audio-player__time">{formatTime(duration)}</span>
                </div>

                <div className="search-audio-player__transport">
                  <div className="search-audio-player__controls">
                    <button type="button" className="search-audio-player__skip-btn" onClick={() => skip(-15)} aria-label="后退 15 秒">
                      <RotateCcw className="w-4 h-4" />
                      <span>15</span>
                    </button>
                    <button
                      type="button"
                      className="search-audio-player__play-btn search-audio-player__play-btn--large"
                      onClick={togglePlay}
                      disabled={!canPlay}
                      aria-label={playLabel}
                    >
                      {isBuffering ? (
                        <Loader2 className="w-6 h-6 animate-spin" />
                      ) : isPlaying ? (
                        <Pause className="w-6 h-6" />
                      ) : (
                        <Play className="w-6 h-6 fill-current ml-0.5" />
                      )}
                    </button>
                    <button type="button" className="search-audio-player__skip-btn" onClick={() => skip(15)} aria-label="前进 15 秒">
                      <RotateCw className="w-4 h-4" />
                      <span>15</span>
                    </button>
                  </div>
                  <VolumeVerticalControl
                    volume={volume}
                    isMuted={isMuted}
                    onVolumeChange={(next) => {
                      setIsMuted(false);
                      setVolume(next);
                    }}
                    onMuteToggle={() => setIsMuted((value) => !value)}
                    variant={hasTranscript ? 'on-dark' : undefined}
                  />
                </div>

                <div className="search-audio-player__speeds" role="group" aria-label="播放倍速">
                  {SPEED_OPTIONS.map((speed, index) => (
                    <button
                      key={speed}
                      type="button"
                      className={`search-audio-player__speed ${index === speedIndex ? 'search-audio-player__speed--active' : ''}`}
                      onClick={() => setSpeedIndex(index)}
                    >
                      {speed}x
                    </button>
                  ))}
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
              <p className="search-media-viewer-empty-inline">暂无可用音频流</p>
            )}
          </div>
        </div>
      )}
    </div>
  );
}
