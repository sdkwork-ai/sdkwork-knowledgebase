import React from 'react';
import { isBlank } from '@sdkwork/utils';
import { Send, Sparkles, Globe, Square } from 'lucide-react';
import { useTranslation } from 'react-i18next';
import { AiModelSelector } from '@sdkwork/sdkwork-knowledgebase-pc-commons';
import { CHAT_LAYOUT_MAX, COMPOSER_MAX_HEIGHT } from '../constants';
import { useComposerAutosize } from '../hooks/useComposerAutosize';
import type { SearchComposerVariant } from '../types';

export interface SearchComposerProps {
  inputValue: string;
  onInputChange: (value: string) => void;
  onKeyDown: (e: React.KeyboardEvent<HTMLTextAreaElement>) => void;
  onSend: () => void;
  onStop?: () => void;
  isTyping: boolean;
  webSearchEnabled: boolean;
  deepThinkEnabled: boolean;
  onToggleWeb: () => void;
  onToggleDeep: () => void;
  textareaRef: React.RefObject<HTMLTextAreaElement | null>;
  variant?: SearchComposerVariant;
  placeholder?: string;
}

function toggleChipClass(active: boolean) {
  return active
    ? 'bg-[var(--color-kb-panel-active)] text-[var(--color-kb-accent)] border-[color-mix(in_srgb,var(--color-kb-accent)_25%,transparent)]'
    : 'bg-transparent text-[var(--color-kb-text-muted)] border-transparent hover:bg-[var(--color-kb-panel-hover)] hover:text-[var(--color-kb-text)]';
}

function iconToggleClass(active: boolean) {
  return active
    ? 'text-[var(--color-kb-accent)] bg-[var(--color-kb-panel-active)]'
    : 'text-[var(--color-kb-text-muted)] hover:bg-[var(--color-kb-panel-hover)] hover:text-[var(--color-kb-text)]';
}

export function SearchComposer({
  inputValue,
  onInputChange,
  onKeyDown,
  onSend,
  onStop,
  isTyping,
  webSearchEnabled,
  deepThinkEnabled,
  onToggleWeb,
  onToggleDeep,
  textareaRef,
  variant = 'hero',
  placeholder,
}: SearchComposerProps) {
  const { t } = useTranslation('search');
  const resolvedPlaceholder =
    placeholder ?? t('composerPlaceholderDefault');
  const isChat = variant === 'chat';
  const maxHeight = isChat ? COMPOSER_MAX_HEIGHT.chat : COMPOSER_MAX_HEIGHT.hero;
  const canSend = !isTyping && !isBlank(inputValue);

  useComposerAutosize(textareaRef, inputValue, maxHeight);

  const sendButton = (
    <button
      type="button"
      disabled={isTyping && !onStop}
      onClick={isTyping && onStop ? onStop : onSend}
      className={`flex items-center justify-center transition-all cursor-pointer border-0 active:scale-95 disabled:opacity-40 ${
        isChat ? 'w-8 h-8 rounded-full' : 'w-9 h-9 rounded-lg'
      } ${
        isTyping && onStop
          ? 'bg-[var(--color-kb-text-heading)] text-[var(--color-kb-editor)]'
          : canSend
            ? 'bg-[var(--color-kb-accent)] hover:bg-[var(--color-kb-accent-hover)] text-white shadow-sm'
            : 'bg-[var(--color-kb-panel-hover)] text-[var(--color-kb-text-muted)]'
      }`}
      title={isTyping ? t('stopGeneration') : t('send')}
    >
      {isTyping && onStop ? (
        <Square className={`fill-current ${isChat ? 'w-2.5 h-2.5' : 'w-3 h-3'}`} />
      ) : (
        <Send className={`${isChat ? 'w-3.5 h-3.5' : 'w-4 h-4'}`} strokeWidth={2.5} />
      )}
    </button>
  );

  if (isChat) {
    return (
      <div className="search-composer-chat w-full rounded-[22px] border border-[var(--color-kb-panel-border)] bg-[var(--color-kb-input-bg)] shadow-sm focus-within:border-[color-mix(in_srgb,var(--color-kb-accent)_30%,var(--color-kb-panel-border))] focus-within:shadow-md transition-all px-3.5 py-2">
        <textarea
          ref={textareaRef}
          value={inputValue}
          onChange={(e) => onInputChange(e.target.value)}
          onKeyDown={onKeyDown}
          placeholder={resolvedPlaceholder}
          rows={1}
          className="w-full min-h-[20px] max-h-[100px] py-1 px-0.5 text-[15px] leading-6 bg-transparent text-[var(--color-kb-text-heading)] outline-none resize-none placeholder-[var(--color-kb-text-muted)] overflow-y-auto no-scrollbar"
        />
        <div className="flex items-center justify-between gap-2 mt-0.5 select-none">
          <div className="flex items-center gap-0.5 min-w-0">
            <AiModelSelector variant="compact" popoverPlacement="top" />
            <button
              type="button"
              onClick={onToggleWeb}
              className={`p-1.5 rounded-full transition-colors cursor-pointer ${iconToggleClass(webSearchEnabled)}`}
              title={t('webSearch')}
            >
              <Globe className="w-3.5 h-3.5" />
            </button>
            <button
              type="button"
              onClick={onToggleDeep}
              className={`p-1.5 rounded-full transition-colors cursor-pointer ${iconToggleClass(deepThinkEnabled)}`}
              title={t('deepThink')}
            >
              <Sparkles className="w-3.5 h-3.5" />
            </button>
          </div>
          {sendButton}
        </div>
      </div>
    );
  }

  return (
    <div className="w-full flex flex-col items-center">
      <div
        className={`w-full ${CHAT_LAYOUT_MAX} mx-auto rounded-2xl bg-[var(--color-kb-input-bg)] border border-[var(--color-kb-panel-border)] shadow-md focus-within:border-[color-mix(in_srgb,var(--color-kb-accent)_35%,var(--color-kb-panel-border))] focus-within:ring-2 focus-within:ring-[color-mix(in_srgb,var(--color-kb-accent)_12%,transparent)] overflow-hidden transition-all`}
      >
        <textarea
          ref={textareaRef}
          value={inputValue}
          onChange={(e) => onInputChange(e.target.value)}
          onKeyDown={onKeyDown}
          placeholder={resolvedPlaceholder}
          rows={3}
          className="w-full min-h-[120px] max-h-[280px] px-5 pt-5 pb-2 text-[15px] bg-transparent text-[var(--color-kb-text-heading)] outline-none resize-none placeholder-[var(--color-kb-text-muted)] leading-relaxed overflow-y-auto no-scrollbar font-medium"
        />
        <div className="flex items-center justify-between select-none px-4 pb-4 pt-1">
          <div className="flex items-center gap-2 flex-wrap min-w-0">
            <AiModelSelector variant="compact" popoverPlacement="top" />
            <button
              type="button"
              onClick={onToggleWeb}
              className={`flex items-center gap-1.5 py-1.5 px-3 rounded-lg text-xs font-semibold border transition-colors cursor-pointer ${toggleChipClass(webSearchEnabled)}`}
            >
              <Globe className="w-3.5 h-3.5" />
              <span>{t('webSearch')}</span>
            </button>
            <button
              type="button"
              onClick={onToggleDeep}
              className={`flex items-center gap-1.5 py-1.5 px-3 rounded-lg text-xs font-semibold border transition-colors cursor-pointer ${toggleChipClass(deepThinkEnabled)}`}
            >
              <Sparkles className="w-3.5 h-3.5" />
              <span>{t('deepThink')}</span>
            </button>
          </div>
          {sendButton}
        </div>
      </div>
    </div>
  );
}
