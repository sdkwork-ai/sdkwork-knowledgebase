import { useEffect, useMemo, useRef } from 'react';
import { FileText, Mic, Music2 } from 'lucide-react';
import type { MediaTimedLine } from '../../../types';
import { computeLineProgress, findActiveTimedLineIndex } from '../../../utils/mediaTimedText';
import { formatTime } from '../useHtmlMediaPlayer';

export interface MediaTimedTextPanelProps {
  lines: MediaTimedLine[];
  currentTime: number;
  onSeekToTime: (seconds: number) => void;
  variant: 'lyrics' | 'transcript';
  disabled?: boolean;
  /** Override panel header label, e.g. 会议纪要 */
  panelTitle?: string;
  /** Show speaker chips when lines include speaker names */
  showSpeakers?: boolean;
  /** QQ Music / 汽水音乐 style centered lyrics without timestamps */
  lyricsFocus?: boolean;
  isPlaying?: boolean;
}

const PANEL_META = {
  lyrics: { title: '歌词', icon: Music2 },
  transcript: { title: '字幕', icon: Mic }
} as const;

const LYRICS_NEAR_DISTANCE = 2;

export function MediaTimedTextPanel({
  lines,
  currentTime,
  onSeekToTime,
  variant,
  disabled,
  panelTitle,
  showSpeakers,
  lyricsFocus = false,
  isPlaying = false
}: MediaTimedTextPanelProps) {
  const listRef = useRef<HTMLDivElement>(null);
  const activeIndex = useMemo(() => findActiveTimedLineIndex(lines, currentTime), [lines, currentTime]);
  const meta = PANEL_META[variant];
  const HeaderIcon = panelTitle === '会议纪要' ? FileText : meta.icon;
  const hasSpeakers = showSpeakers ?? lines.some((line) => Boolean(line.speaker));
  const isFocusLyrics = variant === 'lyrics' && lyricsFocus;

  useEffect(() => {
    if (activeIndex < 0) return;
    const list = listRef.current;
    if (!list) return;
    const activeEl = list.querySelector<HTMLElement>(`[data-line-index="${activeIndex}"]`);
    activeEl?.scrollIntoView({ behavior: isPlaying ? 'smooth' : 'auto', block: 'center' });
  }, [activeIndex, isPlaying]);

  if (!lines.length) return null;

  return (
    <section
      className={[
        'search-media-timed-text',
        `search-media-timed-text--${variant}`,
        isFocusLyrics && 'search-media-timed-text--focus',
        isPlaying && 'search-media-timed-text--playing'
      ]
        .filter(Boolean)
        .join(' ')}
      aria-label={panelTitle ?? meta.title}
    >
      {!isFocusLyrics && (
        <header className="search-media-timed-text__header">
          <HeaderIcon className="w-3.5 h-3.5" />
          <span>{panelTitle ?? meta.title}</span>
          {hasSpeakers && variant === 'transcript' && (
            <span className="search-media-timed-text__tag">语音转写</span>
          )}
          {activeIndex >= 0 && (
            <span className="search-media-timed-text__now">{formatTime(lines[activeIndex]?.startTime ?? 0)}</span>
          )}
        </header>
      )}

      {isFocusLyrics && (
        <header className="search-media-timed-text__header search-media-timed-text__header--minimal">
          <Music2 className="w-3.5 h-3.5" />
          <span>歌词</span>
          {activeIndex >= 0 && isPlaying && (
            <span className="search-media-timed-text__pulse" aria-hidden />
          )}
        </header>
      )}

      <div ref={listRef} className="search-media-timed-text__list" role="list">
        {isFocusLyrics && <div className="search-media-timed-text__pad search-media-timed-text__pad--top" aria-hidden />}

        {lines.map((line, index) => {
          const isActive = index === activeIndex;
          const isPast = activeIndex >= 0 && index < activeIndex;
          const distance = activeIndex >= 0 ? Math.abs(index - activeIndex) : 999;
          const isNear = distance > 0 && distance <= LYRICS_NEAR_DISTANCE;
          const nextLine = lines[index + 1];
          const lineProgress = isActive ? computeLineProgress(line, nextLine, currentTime) : 0;

          return (
            <button
              key={`${line.startTime}-${index}`}
              type="button"
              data-line-index={index}
              role="listitem"
              className={[
                'search-media-timed-text__line',
                isActive && 'search-media-timed-text__line--active',
                isPast && 'search-media-timed-text__line--past',
                isNear && 'search-media-timed-text__line--near',
                line.speaker && 'search-media-timed-text__line--has-speaker',
                isFocusLyrics && 'search-media-timed-text__line--focus'
              ]
                .filter(Boolean)
                .join(' ')}
              onClick={() => onSeekToTime(line.startTime)}
              disabled={disabled}
            >
              {!isFocusLyrics && (
                <span className="search-media-timed-text__time">{formatTime(line.startTime)}</span>
              )}
              <span className="search-media-timed-text__content">
                {hasSpeakers && line.speaker && (
                  <span className="search-media-timed-text__speaker">{line.speaker}</span>
                )}
                <span className="search-media-timed-text__text">{line.text}</span>
                {isActive && isFocusLyrics && lineProgress > 0 && (
                  <span className="search-media-timed-text__progress" aria-hidden>
                    <span className="search-media-timed-text__progress-fill" style={{ width: `${lineProgress * 100}%` }} />
                  </span>
                )}
                {isActive && !isFocusLyrics && lineProgress > 0 && variant === 'transcript' && (
                  <span className="search-media-timed-text__progress search-media-timed-text__progress--transcript" aria-hidden>
                    <span className="search-media-timed-text__progress-fill" style={{ width: `${lineProgress * 100}%` }} />
                  </span>
                )}
              </span>
            </button>
          );
        })}

        {isFocusLyrics && <div className="search-media-timed-text__pad search-media-timed-text__pad--bottom" aria-hidden />}
      </div>
    </section>
  );
}
