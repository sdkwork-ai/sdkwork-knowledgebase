import { useState, useMemo } from 'react';
import { isBlank } from '@sdkwork/utils';
import { createPortal } from 'react-dom';
import { 
  X, Search, MessageSquare, 
  MessageCircle, Sparkles, Check
} from 'lucide-react';

interface ChatDialogModalProps {
  isOpen: boolean;
  onClose: () => void;
  onConfirm: (selectedItems: Array<{ title: string; type: string; content?: string }>) => void;
}

const CHAT_DIALOGUE_ITEMS: Array<{
  id: string;
  title: string;
  type: string;
  source: string;
  updatedAt: string;
  messagesCount: number;
  avatar: string;
  messages: Array<{ sender: string; time: string; text: string }>;
  markdownContent: string;
}> = [];

export function ChatDialogModal({ isOpen, onClose, onConfirm }: ChatDialogModalProps) {
  const [searchQuery, setSearchQuery] = useState('');
  const [selectedIds, setSelectedIds] = useState<Set<string>>(new Set());
  const [activeDialogueId, setActiveDialogueId] = useState<string | null>(null);

  const filteredDialogues = useMemo(() => {
    if (isBlank(searchQuery)) return CHAT_DIALOGUE_ITEMS;
    const query = searchQuery.toLowerCase();
    return CHAT_DIALOGUE_ITEMS.filter(d => 
      d.title.toLowerCase().includes(query) || d.source.toLowerCase().includes(query)
    );
  }, [searchQuery]);

  // Currently viewing dialogue details in the right section
  const currentDialogueDetails = useMemo(() => {
    return CHAT_DIALOGUE_ITEMS.find(d => d.id === activeDialogueId) ?? null;
  }, [activeDialogueId]);

  if (!isOpen) return null;

  const handleToggleSelect = (id: string) => {
    const next = new Set(selectedIds);
    if (next.has(id)) {
      next.delete(id);
    } else {
      next.add(id);
    }
    setSelectedIds(next);
  };

  const handleSelectAll = () => {
    const allFilteredIds = filteredDialogues.map(d => d.id);
    const allSelectedInView = allFilteredIds.every(id => selectedIds.has(id));

    const next = new Set(selectedIds);
    if (allSelectedInView) {
      allFilteredIds.forEach(id => next.delete(id));
    } else {
      allFilteredIds.forEach(id => next.add(id));
    }
    setSelectedIds(next);
  };

  const handleImportSubmit = () => {
    const selectedObjs = CHAT_DIALOGUE_ITEMS.filter(d => selectedIds.has(d.id));
    if (selectedObjs.length === 0) return;

    const mapped = selectedObjs.map(d => ({
      title: `${d.title}.md`,
      type: 'markdown',
      content: d.markdownContent
    }));

    onConfirm(mapped);
  };

  return createPortal(
    <div className="fixed inset-0 z-[300] bg-zinc-950/40 flex items-center justify-center backdrop-blur-md p-4 select-none">
      <div className="w-[840px] h-[580px] bg-[var(--color-kb-editor)] rounded-2xl shadow-[0_24px_64px_-16px_rgba(0,0,0,0.25)] border border-[var(--color-kb-panel-border)] flex flex-col overflow-hidden animate-in zoom-in-95 duration-200">
        
        {/* Modal Header */}
        <div className="h-14 border-b border-zinc-200/80 dark:border-[var(--color-kb-panel-border)] flex items-center justify-between px-6 bg-[#fafafa] dark:bg-[var(--color-kb-panel)]/30 shrink-0">
          <div className="flex items-center gap-3">
            <div className="w-8 h-8 rounded-xl bg-emerald-50 dark:bg-[#07C160]/10 text-emerald-600 dark:text-[#07C160] flex items-center justify-center shadow-inner border border-emerald-100 dark:border-[#07C160]/20">
              <MessageCircle size={16} strokeWidth={2.5} />
            </div>
            <div>
              <h3 className="text-[14px] font-extrabold tracking-tight text-zinc-900 dark:text-[var(--color-kb-text-heading)]">从聊天对话导入</h3>
              <p className="text-[11px] font-medium text-zinc-500 dark:text-[var(--color-kb-text-muted)]">选择一处或多处聊天会话片段，一键自动梳理为Markdown知识文档</p>
            </div>
          </div>
          <button 
            onClick={onClose} 
            className="text-zinc-400 hover:text-red-500 hover:bg-red-50 dark:text-[var(--color-kb-text-muted)] dark:hover:bg-red-500/10 p-2 rounded-xl transition-all"
          >
            <X size={16} strokeWidth={2.5} />
          </button>
        </div>

        {/* Modal body */}
        <div className="flex-1 flex min-h-0 bg-white dark:bg-transparent">
          {/* Left conversations selection pane */}
          <div className="w-[360px] border-r border-zinc-200/80 dark:border-[var(--color-kb-panel-border)] bg-[#fafafa] dark:bg-[var(--color-kb-panel)]/10 flex flex-col shrink-0 min-h-0">
            {/* Search area */}
            <div className="p-4 border-b border-zinc-100 dark:border-[var(--color-kb-panel-border)] shrink-0 bg-white dark:bg-[var(--color-kb-panel)]/5">
              <div className="flex items-center bg-[#fafafa] dark:bg-[var(--color-kb-panel)] border-2 border-zinc-200/80 dark:border-[var(--color-kb-panel-border)] hover:border-emerald-300 focus-within:ring-4 focus-within:ring-emerald-500/10 focus-within:border-emerald-500 px-4 py-2 rounded-xl transition-all h-10 shadow-sm">
                <Search size={14} className="text-zinc-400 dark:text-[var(--color-kb-text-muted)] mr-2 shrink-0" strokeWidth={2.5} />
                <input
                  type="text"
                  placeholder="搜索聊天分组或发送者内容..."
                  value={searchQuery}
                  onChange={(e) => setSearchQuery(e.target.value)}
                  className="bg-transparent border-none outline-none text-[13px] font-bold text-zinc-900 dark:text-[var(--color-kb-text-heading)] placeholder-zinc-400 dark:placeholder-[var(--color-kb-text-muted)] w-full focus:ring-0 focus:outline-none"
                />
              </div>
            </div>

            {/* List header/Select All */}
            <div className="px-5 py-3 border-b border-zinc-200/80 dark:border-[var(--color-kb-panel-border)]/50 flex justify-between items-center bg-white dark:bg-[var(--color-kb-panel)]/5 shrink-0">
              <button 
                onClick={handleSelectAll} 
                className="text-[12px] text-zinc-500 dark:text-[var(--color-kb-text-muted)] hover:text-emerald-600 dark:hover:text-[#07C160] font-bold flex items-center gap-2.5 transition-colors group"
              >
                {filteredDialogues.length > 0 && filteredDialogues.every(d => selectedIds.has(d.id)) ? (
                  <div className="w-4 h-4 bg-emerald-500 text-white rounded border-2 border-emerald-500 flex items-center justify-center">
                    <Check size={12} strokeWidth={3} />
                  </div>
                ) : (
                  <div className="w-4 h-4 border-2 border-zinc-300 dark:border-zinc-500 rounded group-hover:border-emerald-400 transition-colors"></div>
                )}
                <span>全选历史会话 ({filteredDialogues.length})</span>
              </button>
            </div>

            {/* Scrolling Dialogue Threads list */}
            <div className="flex-1 overflow-y-auto p-3 space-y-2 min-h-0">
              {filteredDialogues.length === 0 ? (
                <div className="flex flex-col items-center justify-center py-16 text-zinc-400 dark:text-[var(--color-kb-text-muted)]">
                  <MessageSquare size={40} className="mb-3 opacity-20" />
                  <p className="text-[12px] font-medium text-center px-4">
                    暂无可导入的聊天对话。接入 IM 服务后，此处将展示可选择的会话列表。
                  </p>
                </div>
              ) : filteredDialogues.map(d => {
                const isSelected = selectedIds.has(d.id);
                const isActive = activeDialogueId === d.id;
                return (
                  <div
                    key={d.id}
                    onClick={() => setActiveDialogueId(d.id)}
                    className={`p-3.5 rounded-2xl cursor-pointer flex gap-3.5 transition-all group ${
                      isActive 
                        ? 'bg-emerald-50 dark:bg-[var(--color-kb-accent)]/[0.06] border-2 border-emerald-100 dark:border-[var(--color-kb-panel-border)] shadow-sm' 
                        : 'bg-white border-2 border-transparent hover:border-zinc-200/80 dark:hover:bg-[var(--color-kb-panel-hover)]/40 hover:shadow-sm'
                    }`}
                  >
                    {/* Selector checkbox */}
                    <div onClick={e => { e.stopPropagation(); handleToggleSelect(d.id); }} className="mt-1" >
                      <button className="transition-colors">
                        {isSelected ? (
                          <div className="w-4 h-4 bg-emerald-500 text-white rounded border-2 border-emerald-500 flex items-center justify-center shadow-sm">
                            <Check size={12} strokeWidth={4} />
                          </div>
                        ) : (
                          <div className="w-4 h-4 border-2 border-zinc-300 dark:border-zinc-500 rounded group-hover:border-emerald-400 transition-colors bg-white dark:bg-transparent shadow-sm"></div>
                        )}
                      </button>
                    </div>

                    {/* Logo */}
                    <div className="w-10 h-10 rounded-full bg-zinc-50 dark:bg-[var(--color-kb-panel)] border border-zinc-200/80 dark:border-[var(--color-kb-panel-border)] flex items-center justify-center shrink-0 shadow-sm">
                      <span className="text-lg">{d.avatar}</span>
                    </div>

                    {/* Meta information */}
                    <div className="flex-1 min-w-0">
                      <div className="text-[13px] font-extrabold text-zinc-900 dark:text-[var(--color-kb-text-heading)] truncate tracking-tight">
                        {d.title}
                      </div>
                      <div className="flex items-center justify-between text-[11px] font-bold text-zinc-400 dark:text-[var(--color-kb-text-muted)] mt-2 font-sans tracking-wide">
                        <span className="truncate max-w-[120px]">{d.source}</span>
                        <span>{d.updatedAt}</span>
                      </div>
                    </div>
                  </div>
                );
              })}
            </div>
          </div>

          {/* Right Message Details Panel */}
          <div className="flex-1 flex flex-col bg-white dark:bg-[var(--color-kb-editor)] min-w-0">
            {/* Header displaying currently viewing chat bubble */}
            <div className="h-14 border-b border-zinc-200/80 dark:border-[var(--color-kb-panel-border)] px-6 flex items-center justify-between bg-white dark:bg-[var(--color-kb-panel)]/5 shrink-0 shadow-[0_4px_20px_-10px_rgba(0,0,0,0.03)] z-10">
              <span className="text-[13px] font-extrabold text-zinc-900 dark:text-[var(--color-kb-text-heading)]">
                对话详情：{currentDialogueDetails?.title || '未选择'}
              </span>
              <span className="text-[10.5px] bg-[#fafafa] dark:bg-[var(--color-kb-panel-border)]/40 text-zinc-500 dark:text-[var(--color-kb-text-muted)] border-2 border-zinc-200/80 dark:border-[var(--color-kb-panel-border)] px-3 py-1 rounded-md font-bold font-sans shadow-sm">
                {currentDialogueDetails?.source}
              </span>
            </div>

            {/* Chat detail lists styled exactly as an elegant mockup messenger flow */}
            <div className="flex-1 overflow-y-auto p-6 space-y-6 min-h-0 bg-[#fafafa]/50 dark:bg-[var(--color-kb-panel)]/3">
              {currentDialogueDetails?.messages.map((msg, index) => {
                return (
                  <div key={index} className="flex flex-col">
                    <div className="flex items-center gap-2.5 mb-2 px-1">
                      <span className="text-[12px] font-extrabold text-zinc-700 dark:text-[var(--color-kb-text-heading)]">{msg.sender}</span>
                      <span className="text-[10px] text-zinc-400 dark:text-[var(--color-kb-text-muted)] font-mono font-bold">{msg.time}</span>
                    </div>
                    
                    <div className="flex">
                      <div className="p-4 bg-white dark:bg-[var(--color-kb-panel)] border-2 border-zinc-200/80 dark:border-[var(--color-kb-panel-border)] rounded-2xl rounded-tl-sm text-[13.5px] font-medium text-zinc-800 dark:text-[var(--color-kb-text-heading)] max-w-[85%] leading-relaxed shadow-sm">
                        {msg.text}
                      </div>
                    </div>
                  </div>
                );
              })}

              <div className="pt-6 border-t border-zinc-200/80 dark:border-[var(--color-kb-panel-border)]/40">
                <div className="bg-indigo-50/80 dark:bg-blue-950/20 border-2 border-indigo-100 dark:border-blue-900/30 rounded-2xl p-4 flex gap-3 shadow-sm relative overflow-hidden group">
                  <div className="absolute top-0 right-0 -mr-4 -mt-4 w-20 h-20 bg-indigo-500/10 rounded-full blur-[15px]"></div>
                  <div className="bg-white dark:bg-blue-500/10 text-indigo-600 dark:text-blue-500 p-2 rounded-xl h-fit border border-indigo-100 dark:border-transparent shrink-0 shadow-sm">
                    <Sparkles size={16} className="animate-pulse" strokeWidth={2.5} />
                  </div>
                  <div className="relative z-10">
                    <h5 className="text-[13px] font-extrabold text-indigo-900 dark:text-blue-300">智能转换机制预览</h5>
                    <p className="text-[11.5px] font-medium text-indigo-700/80 dark:text-blue-400/80 leading-relaxed mt-1.5">
                      导入后，此段多轮对话将自动整理成一篇格式优美带有逻辑结构的 Markdown 知识文档，归档讨论核心信息。
                    </p>
                  </div>
                </div>
              </div>
            </div>
          </div>
        </div>

        {/* Modal Footer */}
        <div className="h-16 border-t border-zinc-200/80 dark:border-[var(--color-kb-panel-border)] bg-[#fafafa] dark:bg-[var(--color-kb-panel)] flex items-center justify-between px-6 shrink-0 rounded-b-2xl z-20">
          <div className="text-[12.5px] font-medium text-zinc-500 dark:text-[var(--color-kb-text-muted)]">
            已选中 <strong className="text-zinc-900 dark:text-[var(--color-kb-text-heading)] font-extrabold font-mono mx-1">{selectedIds.size}</strong> 处对话片段
          </div>
          
          <div className="flex items-center gap-3">
            <button
              onClick={onClose}
              className="px-5 py-2 text-[13px] font-bold text-zinc-600 dark:text-[var(--color-kb-text-heading)] bg-white dark:bg-[var(--color-kb-editor)] hover:bg-zinc-100 hover:text-zinc-900 dark:hover:bg-[var(--color-kb-panel-hover)] border-2 border-zinc-200/80 dark:border-[var(--color-kb-panel-border)] rounded-xl transition-all shadow-sm active:scale-95"
            >
              放弃
            </button>
            <button
              onClick={handleImportSubmit}
              disabled={selectedIds.size === 0}
              className="px-6 py-2.5 text-[13px] font-semibold bg-[var(--color-kb-accent)] hover:bg-[var(--color-kb-accent-hover)] text-white disabled:opacity-40 disabled:grayscale rounded-xl shadow-md shadow-[var(--color-kb-accent)]/10 transition-all active:scale-95 flex items-center gap-2 focus:outline-none focus:ring-4 focus:ring-[var(--color-kb-accent)]/20"
            >
              <Check size={16} strokeWidth={3} />
              一键生成知识文档 ({selectedIds.size})
            </button>
          </div>
        </div>

      </div>
    </div>,
    document.body
  );
}
