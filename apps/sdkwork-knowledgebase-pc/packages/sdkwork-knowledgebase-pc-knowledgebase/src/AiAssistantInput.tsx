import React, { useRef, useEffect } from 'react';
import { isBlank } from '@sdkwork/utils';
import { Send, Mic, Plus } from 'lucide-react';
import { DocumentMeta } from './services/document';

interface AiAssistantInputProps {
  inputValue: string;
  setInputValue: (value: string) => void;
  isTyping: boolean;
  selectedReferences: DocumentMeta[];
  isDocSelectorOpen: boolean;
  setIsDocSelectorOpen: (open: boolean) => void;
  handleSend: () => void;
  handleAbort: () => void;
  handleKeyDown: (e: React.KeyboardEvent<HTMLTextAreaElement>) => void;
  t: (key: string, options?: any) => string;
}

export function AiAssistantInput({
  inputValue,
  setInputValue,
  isTyping,
  selectedReferences,
  isDocSelectorOpen,
  setIsDocSelectorOpen,
  handleSend,
  handleAbort,
  handleKeyDown,
  t,
}: AiAssistantInputProps) {
  const textareaRef = useRef<HTMLTextAreaElement>(null);

  // Auto-grow height of textarea dynamically based on content with scrollbar hidden
  useEffect(() => {
    const textarea = textareaRef.current;
    if (textarea) {
      textarea.style.height = '20px';
      // Set to current scrollHeight to fit all content precisely
      const scrollHeight = textarea.scrollHeight;
      textarea.style.height = `${Math.min(scrollHeight, 160)}px`;
    }
  }, [inputValue]);

  return (
    <div className="px-2.5 py-1.5 bg-white dark:bg-[var(--color-kb-query-bg,var(--color-kb-editor))] shrink-0 z-10 relative">
      <div className="flex flex-col bg-zinc-50/80 dark:bg-zinc-900/40 rounded-[6px] px-2 py-1.5 transition-all border border-zinc-200/80 dark:border-zinc-800/80 focus-within:border-zinc-300 dark:focus-within:border-zinc-700 focus-within:ring-1 focus-within:ring-zinc-200/50 dark:focus-within:ring-zinc-800/50 shadow-sm">
        <textarea
          ref={textareaRef}
          value={inputValue}
          onChange={(e) => setInputValue(e.target.value)}
          onKeyDown={handleKeyDown}
          className="w-full max-h-[160px] min-h-[20px] bg-transparent text-[13px] px-0.5 outline-none resize-none text-zinc-900 dark:text-zinc-100 py-0 leading-5 font-medium placeholder-zinc-400 focus:placeholder-zinc-500 dark:placeholder-zinc-500 dark:focus:placeholder-zinc-400 transition-colors overflow-y-auto no-scrollbar"
          style={{ height: '20px', scrollbarWidth: 'none' }}
          placeholder={selectedReferences.length > 0 ? (t('askAboutSelectedDocs', { ns: 'mcp' }) || "关于选档的指引...") : t('aiPlaceholder')}
          rows={1}
          disabled={isTyping}
        />
        
        <div className="flex justify-between items-center mt-1 select-none">
          <div className="flex items-center text-[10px] text-zinc-400 dark:text-[var(--color-kb-text-muted)] font-medium pl-0.5">
            {selectedReferences.length > 0 && (
              <span>已关联 {selectedReferences.length} 个文件</span>
            )}
          </div>
          
          <div className="flex items-center gap-1.5">
            <button 
              type="button"
              className="w-[26px] h-[26px] flex items-center justify-center rounded-[6px] text-zinc-400 hover:text-zinc-600 dark:text-zinc-500 dark:hover:text-zinc-300 hover:bg-zinc-200/50 dark:hover:bg-zinc-800/40 transition-all active:scale-95 cursor-pointer border-0 bg-transparent"
              title={t('voiceInput', { defaultValue: '语音输入' })}
            >
              <Mic size={11} strokeWidth={1.8} />
            </button>

            <button 
              type="button"
              onClick={() => setIsDocSelectorOpen(!isDocSelectorOpen)}
              className={`w-[26px] h-[26px] flex items-center justify-center rounded-[6px] transition-all cursor-pointer border-0 ${
                isDocSelectorOpen || selectedReferences.length > 0 
                  ? 'text-indigo-600 bg-indigo-50/80 dark:text-zinc-100 dark:bg-zinc-800 font-bold' 
                  : 'text-zinc-400 hover:text-zinc-600 dark:text-zinc-500 dark:hover:text-zinc-300 hover:bg-zinc-200/50 dark:hover:bg-zinc-800/40 bg-transparent'
              } active:scale-95`}
              title={t('referToFiles', { ns: 'mcp' })}
            >
              <Plus size={11} strokeWidth={1.8} />
            </button>

            <button 
              disabled={!isTyping && isBlank(inputValue)}
              onClick={isTyping ? handleAbort : handleSend}
              className={`w-[26px] h-[26px] flex items-center justify-center rounded-[6px] transition-all flex-shrink-0 disabled:opacity-40 disabled:scale-100 active:scale-95 cursor-pointer border-0 ${
                isTyping 
                  ? 'bg-zinc-950 dark:bg-zinc-900 shadow-[0_0_8px_rgba(255,255,255,0.15)] flex items-center justify-center' 
                  : 'text-white bg-[var(--color-kb-accent)] hover:bg-[var(--color-kb-accent)]/90 shadow-sm'
              }`}
            >
              {isTyping ? (
                <span className="w-1.5 h-1.5 bg-white rounded-[1.5px]" />
              ) : (
                <Send size={11} strokeWidth={1.8} className="ml-[0.5px] text-white" />
              )}
            </button>
          </div>
        </div>
      </div>
    </div>
  );
}
