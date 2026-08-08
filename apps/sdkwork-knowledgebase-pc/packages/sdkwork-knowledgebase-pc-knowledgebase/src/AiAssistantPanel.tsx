import React, { useState, useRef, useEffect } from 'react';
import { isBlank } from '@sdkwork/utils';
import { Sparkles, X, Loader2, File, Check, CheckCircle2, CircleAlert } from 'lucide-react';
import { useTranslation } from 'react-i18next';
import { AiModelSelector } from '@sdkwork/sdkwork-knowledgebase-pc-commons';
import { DocumentMeta, FolderNode } from './services/document';
import { AIService } from './services/ai';
import type { McpToolCall } from './services/mcpAgent';
import {
  resolveUserFacingErrorMessage,
  type ErrorTranslateFn,
} from 'sdkwork-knowledgebase-pc-core';
import DOMPurify from 'dompurify';
import { marked } from 'marked';
import { AiAssistantInput } from './AiAssistantInput';
import {
  isKnowledgebaseWorkspaceAiEnabled,
  resolveKnowledgebaseWorkspaceAiScope,
  type KnowledgebaseWorkspaceMode,
} from './workspaceMode';

export interface ChatMessage {
  role: 'user' | 'assistant';
  content: string;
  references?: DocumentMeta[];
  toolCalls?: McpToolCall[];
}

export interface AiAssistantPanelProps {
  aiWidth: number;
  isDraggingAi: boolean;
  onMouseDownDrag: () => void;
  onClose: () => void;
  docContent?: string;
  docs?: (FolderNode | DocumentMeta)[];
  activeDoc?: DocumentMeta | null;
  activeKbId?: string;
  headerHeightClass?: string;
  onSendMessage?: (msg: string, refs: DocumentMeta[], setMessages: React.Dispatch<React.SetStateAction<ChatMessage[]>>, setIsTyping: React.Dispatch<React.SetStateAction<boolean>>) => void;
  selectedArticle?: any;
  workspaceMode?: KnowledgebaseWorkspaceMode;
}

export function AiAssistantPanel({
  aiWidth, isDraggingAi, onMouseDownDrag, onClose, docContent, docs = [], activeDoc, activeKbId,
  headerHeightClass = 'h-[40px]', onSendMessage, workspaceMode = 'standard',
}: AiAssistantPanelProps) {
  const { t } = useTranslation(['editor', 'mcp']);
  const aiEnabled = isKnowledgebaseWorkspaceAiEnabled(workspaceMode);
  const [messages, setMessages] = useState<ChatMessage[]>([]);
  const [inputValue, setInputValue] = useState('');
  const [isTyping, setIsTyping] = useState(false);
  const scrollRef = useRef<HTMLDivElement>(null);
  const [isDocSelectorOpen, setIsDocSelectorOpen] = useState(false);
  const [selectedReferences, setSelectedReferences] = useState<DocumentMeta[]>([]);
  // Monotonic generation id: bumping it invalidates any in-flight generation so "stop"
  // really stops the streamed reply from appending stale content.
  const generationSeqRef = useRef(0);

  useEffect(() => {
    // Clear references when switching knowledge base
    setSelectedReferences([]);
  }, [activeKbId]);

  useEffect(() => {
    // If there's an active doc, we could automatically suggest referencing it, 
    // or we can allow manual referencing. Let's just update references when user explicitly chooses.
  }, [activeDoc]);

  useEffect(() => {
    if (scrollRef.current) {
      scrollRef.current.scrollTop = scrollRef.current.scrollHeight;
    }
  }, [messages, isTyping]);

  if (!aiEnabled) {
    return null;
  }

  const appendRequestError = (error: unknown) => {
    const content = resolveUserFacingErrorMessage(
      error,
      t as unknown as ErrorTranslateFn,
    );
    setMessages(prev => [...prev, { role: 'assistant', content }]);
  };

  const handleDefaultSend = async (userMessage: string, currentRefs: DocumentMeta[]) => {
    if (onSendMessage && workspaceMode === 'standard') {
       onSendMessage(userMessage, currentRefs, setMessages, setIsTyping);
       return;
    }

    const generation = ++generationSeqRef.current;
    try {
      const scope = resolveKnowledgebaseWorkspaceAiScope(workspaceMode, activeKbId);
      const { result: responseText, toolCalls } = await AIService.generateChatResponse(
        userMessage,
        docContent,
        currentRefs.map((reference) => reference.title).join(','),
        scope,
      );
      if (generation !== generationSeqRef.current) {
        return;
      }
      const resolvedToolCalls = toolCalls?.map((toolCall): McpToolCall => ({
        ...toolCall,
        status: toolCall.status ?? 'failed',
      }));
      setMessages(prev => [...prev, {
        role: 'assistant',
        content: responseText,
        toolCalls: resolvedToolCalls,
      }]);
    } catch (error) {
      if (generation !== generationSeqRef.current) {
        return;
      }
      appendRequestError(error);
    } finally {
      if (generation === generationSeqRef.current) {
        setIsTyping(false);
      }
    }
  };

  const handleAbort = () => {
    // Invalidate the in-flight generation; the completed response is discarded so the
    // assistant reply really stops instead of resuming after the user pressed stop.
    generationSeqRef.current += 1;
    setIsTyping(false);
  };

  const handleSend = async () => {
    if (isBlank(inputValue) || isTyping) return;

    const userMessage = inputValue.trim();
    const currentRefs = [...selectedReferences];
    setInputValue('');
    setSelectedReferences([]);
    setMessages(prev => [...prev, { role: 'user', content: userMessage, references: currentRefs }]);
    setIsTyping(true);

    void handleDefaultSend(userMessage, currentRefs);
  };

  const handleKeyDown = (e: React.KeyboardEvent<HTMLTextAreaElement>) => {
    if (e.key === 'Enter' && !e.shiftKey) {
      e.preventDefault();
      handleSend();
    }
  };

  // Helper to extract flat list of non-folder docs
  const extractFlatDocs = (items: (FolderNode | DocumentMeta)[]): DocumentMeta[] => {
    let result: DocumentMeta[] = [];
    for (const item of items) {
      if (item.type === 'folder') {
        if ((item as FolderNode).children) {
           result = [...result, ...extractFlatDocs((item as FolderNode).children)];
        }
      } else {
        result.push(item as DocumentMeta);
      }
    }
    return result;
  };

  const flatDocs = extractFlatDocs(docs);

  const toggleReference = (doc: DocumentMeta) => {
    setSelectedReferences(prev => {
      if (prev.find(d => d.id === doc.id)) {
        return prev.filter(d => d.id !== doc.id);
      }
      return [...prev, doc];
    });
  };

  const formatMessageContent = (content: string) => {
    try {
      const rawHtml = marked.parse(content, { async: false }) as string;
      return { __html: DOMPurify.sanitize(rawHtml) };
    } catch {
      return { __html: DOMPurify.sanitize(content) };
    }
  };

  return (
    <div 
      className="flex flex-col bg-[#fafafa] dark:bg-[var(--color-kb-editor)] border-l border-zinc-200/80 dark:border-[var(--color-kb-panel-border)] relative flex-shrink-0 shadow-[-8px_0_24px_-12px_rgba(0,0,0,0.08)] z-30"
      style={{ width: `${aiWidth}px` }}
    >
      {/* Resize Handle */}
      <div 
        className="absolute left-0 top-0 bottom-0 w-1 cursor-col-resize hover:bg-[var(--color-kb-accent)]/50 transition-colors z-20 group -ml-[2px]"
        onMouseDown={onMouseDownDrag}
      >
        <div className={`absolute top-1/2 -translate-y-1/2 left-1/2 -translate-x-1/2 w-[3px] h-12 rounded-full transition-colors opacity-0 group-hover:opacity-100 ${isDraggingAi ? 'bg-[var(--color-kb-accent)] opacity-100' : 'bg-[var(--color-kb-accent)]/80'}`}></div>
      </div>
      
      <div className={`${headerHeightClass} border-b border-[var(--color-kb-panel-border)]/80 bg-[var(--color-kb-editor)]/95 dark:bg-[var(--color-kb-panel)] flex items-center justify-between px-4 shrink-0 select-none backdrop-blur-md z-30 shadow-sm relative`}>
        <AiModelSelector variant="header" fallbackLabel={t('aiAssistant')} popoverPlacement="bottom" />

        <button
          type="button"
          onClick={onClose} 
          className="text-zinc-400 dark:text-[var(--color-kb-text-muted)] hover:text-red-500 hover:bg-red-50 dark:hover:text-red-500 dark:hover:bg-red-500/10 p-1 rounded-md transition-all active:scale-95 border border-transparent"
          title={t('closeAiAssistant', { ns: 'mcp' }) || '关闭AI助手'}
        >
          <X size={14} strokeWidth={2.5} />
        </button>
      </div>
      
      <div className="flex-1 min-h-0 search-theme-scrollbar p-4 space-y-4" ref={scrollRef}>
        {messages.length === 0 && (
          <div className="flex flex-col space-y-1.5">
            <div className="text-xs text-zinc-400 dark:text-[var(--color-kb-text-muted)] text-center mb-2 font-medium">{t('aiGreetingTime')}</div>
            <div className="flex justify-start mb-2">
               <div className="bg-white dark:bg-[var(--color-kb-panel-hover)] border-2 border-zinc-200/80 dark:border-[var(--color-kb-panel-border)]/50 rounded-3xl p-5 text-sm text-zinc-800 dark:text-[var(--color-kb-text)] w-full break-words shadow-sm relative overflow-hidden group">
                <div className="absolute top-0 right-0 -mr-6 -mt-6 w-24 h-24 bg-indigo-50 dark:bg-indigo-500/5 rounded-full blur-[20px] transition-all group-hover:bg-indigo-100/80 dark:group-hover:bg-indigo-500/10"></div>
                
                <div className="relative z-10">
                  <div className="markdown-body font-medium leading-relaxed" dangerouslySetInnerHTML={formatMessageContent(t('aiGreetingText'))} />
                  <ul className="list-disc pl-5 mt-3 space-y-2 text-[13px] font-medium text-zinc-500 dark:text-[var(--color-kb-text-muted)] mb-4 marker:text-indigo-300 dark:marker:text-indigo-600">
                    <li>{t('aiHelpSummarize')}</li>
                    <li>{t('aiHelpWrite')}</li>
                    <li>{t('aiHelpAsk')}</li>
                    <li>{t('aiHelpGenerate')}</li>
                  </ul>
                </div>

                <div className="h-px bg-zinc-100 dark:bg-[var(--color-kb-panel-border)]/50 my-4 relative z-10"></div>
                <div className="space-y-3 relative z-10">
                  <span className="text-[11px] font-extrabold text-indigo-500 dark:text-[var(--color-kb-text-muted)] tracking-wider flex items-center gap-1">
                    <Sparkles size={12} strokeWidth={2.5} /> {t('coreSkills', { ns: 'mcp' })}
                  </span>
                  <div className="flex flex-wrap gap-2.5">
                    {[
                      { label: t('chipStyles', { ns: 'mcp' }), cmd: t('quickStyles', { ns: 'mcp' }) },
                      { label: t('chipHeadlines', { ns: 'mcp' }), cmd: t('quickHeadlines', { ns: 'mcp' }) },
                      { label: t('chipInsert', { ns: 'mcp' }), cmd: t('quickInsert', { ns: 'mcp' }) },
                      { label: t('chipDiagnose', { ns: 'mcp' }), cmd: t('quickDiagnose', { ns: 'mcp' }) },
                      { label: t('chipRewrite', { ns: 'mcp' }), cmd: t('quickRewrite', { ns: 'mcp' }) }
                    ].map((chip, i) => (
                      <button
                        key={i}
                        type="button"
                        // Quick-tool commands are not yet wired to a server capability; keep
                        // the entry honest and disabled instead of invoking a guaranteed failure.
                        disabled
                        title={t('comingSoon', { ns: 'mcp' }) || '即将上线'}
                        className="px-3.5 py-1.5 text-[12px] font-bold bg-[#fafafa] dark:bg-[var(--color-kb-editor)] border border-zinc-200/80 dark:border-[var(--color-kb-panel-border)] text-zinc-400 dark:text-[var(--color-kb-text-muted)] rounded-full transition-all cursor-not-allowed opacity-70"
                      >
                        {chip.label}
                      </button>
                    ))}
                  </div>
                </div>
              </div>
            </div>
          </div>
        )}

        {messages.map((msg, idx) => (
          <div key={idx} className={`flex mb-4 animate-in slide-in-from-bottom-2 ${msg.role === 'user' ? 'justify-end' : 'justify-start'}`}>
             <div className={`flex flex-col ${msg.role === 'user' ? 'items-end max-w-[92%]' : 'items-start w-full'}`}>
                {msg.references && msg.references.length > 0 && (
                   <div className="flex flex-wrap gap-1 mb-1 justify-end">
                      {msg.references.map(ref => (
                         <div key={ref.id} className="flex items-center text-[10px] uppercase font-bold bg-indigo-50/50 dark:bg-[var(--color-kb-panel-hover)] border border-indigo-100 dark:border-[var(--color-kb-panel-border)] text-indigo-500 dark:text-[var(--color-kb-text-muted)] px-1.5 py-0.5 rounded shadow-sm">
                            <File size={10} className="mr-1 opacity-70" />
                            <span className="truncate max-w-[120px]">{ref.title}</span>
                         </div>
                      ))}
                   </div>
                )}
                <div 
                  className={`rounded-2xl p-3.5 text-sm break-words flex flex-col shadow-sm ${msg.role === 'user' ? 'bg-[var(--color-kb-accent)] text-white font-medium shadow-indigo-500/20' : 'w-full bg-white dark:bg-[var(--color-kb-panel-hover)] border border-zinc-200/80 dark:border-[var(--color-kb-panel-border)]/55 text-zinc-800 dark:text-[var(--color-kb-text)] rounded-tl-sm markdown-body'} ${msg.role === 'assistant' ? 'whitespace-normal' : 'whitespace-pre-wrap'}`}
                  dangerouslySetInnerHTML={msg.role === 'assistant' ? formatMessageContent(msg.content) : undefined}
                >
                   {msg.role === 'user' ? msg.content : undefined}
                </div>

                {/* MCP Tool Calls Terminal display */}
                {msg.role === 'assistant' && msg.toolCalls && msg.toolCalls.length > 0 && (
                  <div className="w-full mt-2 space-y-1.5">
                    {msg.toolCalls.map((tc, tcIdx) => {
                      const toolNameMap: Record<string, string> = {
                        search_knowledge_base: t('tool_search_kb', { defaultValue: '检索相关文档与知识' }),
                        write_to_note: t('tool_write_note', { defaultValue: '提炼结构化信息并写入文档' }),
                        analyze_intent: t('tool_analyze_intent', { defaultValue: '分析上下文与意图' }),
                        insert_content: t('tool_insert_content', { defaultValue: '执行正文内容的追加与修改' }),
                      };
                      return (
                        <div key={tcIdx} className="overflow-hidden rounded-xl border border-indigo-100 dark:border-indigo-900/30 bg-gradient-to-r from-indigo-50/50 to-white dark:from-indigo-950/20 dark:to-transparent shadow-sm">
                          <div className="flex items-center px-3 py-2 text-[12px] text-slate-700 dark:text-indigo-200 font-medium">
                            {tc.status === 'running' ? (
                                <Loader2 size={13} className="text-indigo-500 dark:text-indigo-400 animate-spin mr-2 shrink-0" />
                            ) : tc.status === 'success' ? (
                                <CheckCircle2 size={13} className="text-emerald-500 dark:text-emerald-400 mr-2 shrink-0" />
                            ) : (
                                <CircleAlert size={13} className="text-red-500 dark:text-red-400 mr-2 shrink-0" />
                            )}
                            <span className="truncate">{toolNameMap[tc.name] || tc.name}</span>
                          </div>
                          {tc.status !== 'running' && tc.result && (
                            <div className="px-3 pb-2 pt-0.5">
                              <div className="text-[11px] text-slate-500 dark:text-indigo-300/70 border-l-[1.5px] border-indigo-200 dark:border-indigo-800 pl-2 py-0.5 whitespace-pre-wrap font-sans">
                                  {tc.result}
                              </div>
                            </div>
                          )}
                        </div>
                      )
                    })}
                  </div>
                )}
             </div>
          </div>
        ))}

        {isTyping && (
           <div className="flex items-center space-x-2 text-zinc-400 dark:text-[var(--color-kb-text-muted)] mt-2 font-medium">
              <Loader2 size={14} className="animate-spin text-indigo-500" />
              <span className="text-xs">{t('aiProcessing')}</span>
           </div>
        )}
      </div>
      
      {/* Selected References Display */}
      {selectedReferences.length > 0 && (
        <div className="px-4 py-2.5 border-t border-zinc-200 dark:border-[var(--color-kb-editor-border)] bg-[#fafafa] dark:bg-[var(--color-kb-panel)] flex flex-wrap gap-2 overflow-x-auto no-scrollbar shrink-0 shadow-[0_-4px_10px_-4px_rgba(0,0,0,0.02)] z-10">
          {selectedReferences.map(ref => (
            <div key={ref.id} className="flex items-center text-xs bg-indigo-50 dark:bg-[var(--color-kb-editor)] border border-indigo-200/50 dark:border-[var(--color-kb-accent)]/30 text-indigo-600 dark:text-[var(--color-kb-accent)] px-2 py-1 rounded-md shadow-sm">
              <File size={12} className="mr-1.5 opacity-80" />
              <span className="truncate max-w-[100px] font-bold">{ref.title}</span>
              <button 
                onClick={() => toggleReference(ref)} 
                className="ml-1.5 p-0.5 hover:bg-indigo-200/50 dark:hover:bg-[var(--color-kb-accent)]/10 rounded-full transition-colors opacity-70 hover:opacity-100"
              >
                <X size={12} />
              </button>
            </div>
          ))}
        </div>
      )}

      {/* Document Selector Popover */}
      {isDocSelectorOpen && (
        <div className="absolute bottom-[75px] left-4 right-4 bg-white dark:bg-[var(--color-kb-editor)] border border-zinc-200/80 dark:border-[var(--color-kb-panel-border)] shadow-2xl rounded-xl z-50 flex flex-col max-h-[240px] animate-in fade-in slide-in-from-bottom-2 overflow-hidden backdrop-blur-md">
          <div className="px-3.5 py-2.5 border-b border-zinc-100 dark:border-[var(--color-kb-panel-border)] text-xs font-bold text-zinc-500 dark:text-[var(--color-kb-text-muted)] flex justify-between items-center bg-[#fafafa]/50 dark:bg-transparent">
            <span>{t('referToFiles', { ns: 'mcp' }) || '引入文档上下文'}</span>
            <button onClick={() => setIsDocSelectorOpen(false)} className="hover:bg-black/5 dark:hover:bg-[var(--color-kb-panel-hover)] rounded p-1 transition-colors">
              <X size={14} />
            </button>
          </div>
          <div className="overflow-y-auto p-1.5 py-1.5 user-select-none">
            {flatDocs.length === 0 ? (
              <div className="text-center text-xs text-zinc-400 dark:text-[var(--color-kb-text-muted)] py-6">{t('noDocAvailable', { ns: 'mcp' }) || '无可用文档'}</div>
            ) : (
              flatDocs.map(doc => {
                const isSelected = !!selectedReferences.find(d => d.id === doc.id);
                return (
                  <button
                    key={doc.id}
                    onClick={() => toggleReference(doc)}
                    className="w-full flex items-center px-2.5 py-2 hover:bg-black/5 dark:hover:bg-[var(--color-kb-panel-hover)] rounded-lg text-sm text-zinc-700 dark:text-[var(--color-kb-text)] transition-colors text-left"
                  >
                    <div className={`mr-2.5 flex items-center justify-center w-4 h-4 rounded border transition-colors ${isSelected ? 'border-indigo-500 bg-indigo-500 dark:border-[var(--color-kb-accent)] dark:bg-[var(--color-kb-accent)] text-white shadow-sm' : 'border-zinc-300 dark:border-[var(--color-kb-text-muted)]'}`}>
                      {isSelected && <Check size={10} strokeWidth={3} />}
                    </div>
                    <File size={14} className="mr-2 text-zinc-400 dark:text-[var(--color-kb-text-muted)]" />
                    <span className="truncate flex-1 font-medium">{doc.title}</span>
                  </button>
                );
              })
            )}
          </div>
        </div>
      )}

      <AiAssistantInput
        inputValue={inputValue}
        setInputValue={setInputValue}
        isTyping={isTyping}
        selectedReferences={selectedReferences}
        isDocSelectorOpen={isDocSelectorOpen}
        setIsDocSelectorOpen={setIsDocSelectorOpen}
        handleSend={handleSend}
        handleAbort={handleAbort}
        handleKeyDown={handleKeyDown}
        t={t}
      />

    </div>
  );
}

