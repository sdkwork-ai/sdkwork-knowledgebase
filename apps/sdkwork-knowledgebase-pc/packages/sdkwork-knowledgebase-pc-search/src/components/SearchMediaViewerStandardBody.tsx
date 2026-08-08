import React, { useEffect, useRef } from 'react';
import { ChevronLeft, ChevronRight } from 'lucide-react';
import type { SearchMediaCategory, SearchMediaItem } from '../types';
import { SearchMediaViewerContentRouter } from './media-viewers/SearchMediaViewerContentRouter';

export interface SearchMediaViewerStandardBodyProps {
  item: SearchMediaItem;
  items: SearchMediaItem[];
  category: SearchMediaCategory;
  activeIndex: number;
  isMinimized: boolean;
  contentSlideClass: string;
  thumbFallback: React.ReactNode;
  onIndexChange: (index: number) => void;
  onOpenWebLink?: (url: string, title?: string) => void;
}

/** Gallery-style body for image / video / product — isolated from audio/music playlist layout. */
export function SearchMediaViewerStandardBody({
  item,
  items,
  category,
  activeIndex,
  isMinimized,
  contentSlideClass,
  thumbFallback,
  onIndexChange,
  onOpenWebLink
}: SearchMediaViewerStandardBodyProps) {
  const activeThumbRef = useRef<HTMLButtonElement>(null);
  const canGoPrev = activeIndex > 0;
  const canGoNext = activeIndex < items.length - 1;

  useEffect(() => {
    activeThumbRef.current?.scrollIntoView({ behavior: 'smooth', block: 'nearest', inline: 'center' });
  }, [activeIndex]);

  return (
    <>
      <div className={`search-media-viewer-body ${isMinimized ? 'search-media-viewer-body--mini' : ''}`}>
        <div
          className={`search-media-viewer-content search-media-viewer-content--${category} ${contentSlideClass}`.trim()}
          key={item.id}
        >
          <SearchMediaViewerContentRouter
            item={item}
            category={category}
            layoutMode={isMinimized ? 'minimized' : 'expanded'}
            onOpenWebLink={onOpenWebLink}
          />
        </div>

        {!isMinimized && items.length > 1 && canGoPrev && (
          <button
            type="button"
            className="search-media-viewer-nav search-media-viewer-nav--prev"
            onClick={() => onIndexChange(activeIndex - 1)}
            aria-label="上一项"
          >
            <ChevronLeft className="w-5 h-5" />
          </button>
        )}

        {!isMinimized && items.length > 1 && canGoNext && (
          <button
            type="button"
            className="search-media-viewer-nav search-media-viewer-nav--next"
            onClick={() => onIndexChange(activeIndex + 1)}
            aria-label="下一项"
          >
            <ChevronRight className="w-5 h-5" />
          </button>
        )}
      </div>

      {!isMinimized && items.length > 1 && (
        <footer className="search-media-viewer-footer">
          <div className="search-media-viewer-thumbs">
            {items.map((thumbItem, index) => (
              <button
                key={thumbItem.id}
                ref={index === activeIndex ? activeThumbRef : undefined}
                type="button"
                className={`search-media-viewer-thumb ${index === activeIndex ? 'search-media-viewer-thumb--active' : ''}`}
                onClick={() => onIndexChange(index)}
                aria-label={`查看第 ${index + 1} 项：${thumbItem.title}`}
                title={thumbItem.title}
              >
                {thumbItem.thumbnailUrl ? (
                  <img src={thumbItem.thumbnailUrl} alt={thumbItem.title} loading="lazy" />
                ) : (
                  <span className="search-media-viewer-thumb-fallback">{thumbFallback}</span>
                )}
              </button>
            ))}
          </div>
          <p className="search-media-viewer-kbd-hint">
            <kbd>←</kbd>
            <kbd>→</kbd>
            切换
            <span className="search-media-viewer-kbd-sep" />
            <kbd>Esc</kbd>
            关闭
          </p>
        </footer>
      )}
    </>
  );
}
