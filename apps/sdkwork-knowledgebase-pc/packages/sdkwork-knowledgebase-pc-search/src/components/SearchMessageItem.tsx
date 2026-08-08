import React, { useState } from 'react';
import { isBlank } from '@sdkwork/utils';
import { Copy, PlusCircle, RotateCcw } from 'lucide-react';
import type {
  SearchMediaTab,
  SearchMessage,
  SearchNavigateToFilePayload,
  SearchNavigateToKbPayload
} from '../types';
import { formatMarkdownHtml, parseFollowUpSuggestions, scrollToCitation } from '../utils/markdown';
import { hasRelatedMedia } from '../utils/mediaResults';
import { MessageSourcesPanel } from './MessageSourcesPanel';
import { SearchMessageMediaSwitcher } from './SearchMediaPanel';
import { SearchStepsPanel } from './SearchStepsPanel';

export interface SearchMessageItemProps {
  message: SearchMessage;
  prevUserQuery?: string;
  isStreaming: boolean;
  isTyping: boolean;
  stepsExpanded: boolean;
  onToggleSteps: () => void;
  onRegenerate: (query: string) => void;
  onExport: (text: string) => void;
  onCopy: (text: string) => void;
  onFollowUp: (text: string) => void;
  onGoToKb: (payload: SearchNavigateToKbPayload) => void;
  onGoToFile: (payload: SearchNavigateToFilePayload) => void;
  onOpenWebLink?: (url: string, title?: string) => void;
}

export function SearchMessageItem({
  message,
  prevUserQuery,
  isStreaming,
  isTyping,
  stepsExpanded,
  onToggleSteps,
  onRegenerate,
  onExport,
  onCopy,
  onFollowUp,
  onGoToKb,
  onGoToFile,
  onOpenWebLink
}: SearchMessageItemProps) {
  const isUser = message.role === 'user';
  const [activeMediaTab, setActiveMediaTab] = useState<SearchMediaTab>('answer');
  const showAnswerContent = activeMediaTab === 'answer';

  if (isUser) {
    return (
      <div className="flex justify-end">
        <p className="max-w-[92%] text-sm font-medium text-[var(--color-kb-text-heading)] leading-relaxed whitespace-pre-wrap bg-[var(--color-kb-panel)]/80 px-4 py-3 rounded-2xl">
          {message.content}
        </p>
      </div>
    );
  }

  const followUps =
    !message.isSearching && message.content ? parseFollowUpSuggestions(message.content) : [];

  /** Citations & follow-ups only after streaming finishes — avoids flicker and partial parsing */
  const isResponseComplete =
    !message.isSearching && !isStreaming && !isBlank(message.content);

  const handleCitationClick = (e: React.MouseEvent<HTMLDivElement>) => {
    const target = (e.target as HTMLElement).closest('[data-citation]') as HTMLElement | null;
    if (!target) return;
    const num = target.dataset.citation;
    const msgId = target.dataset.msg;
    if (!num || !msgId) return;
    scrollToCitation(msgId, num);
  };

  return (
    <div className="space-y-4">
      <SearchStepsPanel message={message} expanded={stepsExpanded} onToggle={onToggleSteps} />

      {message.isSearching ? (
        <div className="flex flex-col gap-2.5 py-2">
          <div className="h-3.5 bg-[var(--color-kb-panel)] rounded animate-pulse w-[78%]" />
          <div className="h-3.5 bg-[var(--color-kb-panel)] rounded animate-pulse w-[92%]" />
          <div className="h-3.5 bg-[var(--color-kb-panel)] rounded animate-pulse w-[64%]" />
        </div>
      ) : (
        <div className="space-y-4">
          {isResponseComplete && hasRelatedMedia(message.relatedMedia) && (
            <SearchMessageMediaSwitcher
              relatedMedia={message.relatedMedia}
              activeTab={activeMediaTab}
              onTabChange={setActiveMediaTab}
              onGoToFile={onGoToFile}
              onOpenWebLink={onOpenWebLink}
            />
          )}

          {showAnswerContent && (
            <>
              <div
                className="markdown-body search-markdown text-[15px] leading-[1.75] whitespace-normal break-words text-[var(--color-kb-text)] selection:bg-[color-mix(in_srgb,var(--color-kb-accent)_20%,transparent)]"
                onClick={handleCitationClick}
                dangerouslySetInnerHTML={{
                  __html: formatMarkdownHtml(message.content, message.id)
                }}
              />
              {isStreaming && <span className="search-stream-cursor" aria-hidden />}
            </>
          )}

          {isResponseComplete && message.sources && message.sources.length > 0 && showAnswerContent && (
            <MessageSourcesPanel
              sources={message.sources}
              messageId={message.id}
              onGoToKb={onGoToKb}
              onGoToFile={onGoToFile}
              onOpenWebLink={onOpenWebLink}
            />
          )}

          {isResponseComplete && followUps.length > 0 && showAnswerContent && (
            <div className="pt-1 space-y-2 animate-in fade-in duration-300">
              <p className="text-[11px] font-semibold text-[var(--color-kb-text-muted)]">推荐追问</p>
              <div className="flex flex-wrap gap-2">
                {followUps.map((suggestion) => (
                  <button
                    key={suggestion}
                    type="button"
                    onClick={() => onFollowUp(suggestion)}
                    disabled={isTyping}
                    className="text-left text-xs px-3 py-1.5 rounded-full border border-[var(--color-kb-panel-border)] bg-[var(--color-kb-panel)]/50 hover:border-[color-mix(in_srgb,var(--color-kb-accent)_40%,var(--color-kb-panel-border))] hover:bg-[var(--color-kb-panel-active)] text-[var(--color-kb-text)] transition-all disabled:opacity-50"
                  >
                    {suggestion}
                  </button>
                ))}
              </div>
            </div>
          )}

          {isResponseComplete && showAnswerContent && (
          <div className="flex flex-wrap items-center justify-between gap-3 pt-2 text-xs text-[var(--color-kb-text-muted)] select-none animate-in fade-in duration-300">
            <span>{message.timestamp}</span>
            <div className="flex items-center gap-1">
              {prevUserQuery && (
                <button
                  type="button"
                  onClick={() => onRegenerate(prevUserQuery)}
                  disabled={isTyping}
                  className="flex items-center gap-1 px-2 py-1.5 rounded-md hover:bg-[var(--color-kb-panel-hover)] text-[var(--color-kb-text-muted)] hover:text-[var(--color-kb-text)] transition-all disabled:opacity-40"
                  title="重新生成"
                >
                  <RotateCcw className="w-3.5 h-3.5" />
                  <span>重新生成</span>
                </button>
              )}
              <button
                type="button"
                onClick={() => onExport(message.content)}
                className="flex items-center gap-1 px-2 py-1.5 rounded-md hover:bg-[var(--color-kb-panel-hover)] text-[var(--color-kb-text-muted)] hover:text-[var(--color-kb-accent)] transition-all font-medium"
              >
                <PlusCircle className="w-3.5 h-3.5" />
                <span>导入知识库</span>
              </button>
              <button
                type="button"
                onClick={() => void onCopy(message.content)}
                className="p-1.5 rounded-md hover:bg-[var(--color-kb-panel-hover)] text-[var(--color-kb-text-muted)] hover:text-[var(--color-kb-text)]"
                title="复制全文"
              >
                <Copy className="w-3.5 h-3.5" />
              </button>
            </div>
          </div>
          )}
        </div>
      )}
    </div>
  );
}
