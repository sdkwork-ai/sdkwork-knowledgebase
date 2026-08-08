import { useState, useMemo } from 'react';
import { useTranslation } from 'react-i18next';
import { createPortal } from 'react-dom';
import { 
  X, Search, MessageSquare, FileText, 
  FolderArchive, Image as ImageIcon, Video, Music, Link as LinkIcon, 
  Globe, FileJson, Check
} from 'lucide-react';

interface ChatFileModalProps {
  isOpen: boolean;
  onClose: () => void;
  onConfirm: (selectedItems: Array<{ title: string; type: string; content?: string }>) => void;
}

// Chat file import requires IM connector integration; no synthetic items are shown.
const CHAT_FILE_ITEMS: Array<{
  id: string;
  title: string;
  type: string;
  fileType: string;
  author: string;
  date: string;
  size: string;
  content?: string;
}> = [];

type CategoryType = 'all' | 'document' | 'media' | 'link' | 'music' | 'other';

export function ChatFileModal({ isOpen, onClose, onConfirm }: ChatFileModalProps) {
  const { t } = useTranslation(['kb', 'common']);
  const [activeTab, setActiveTab] = useState<CategoryType>('document');
  const [searchQuery, setSearchQuery] = useState('');
  const [selectedIds, setSelectedIds] = useState<Set<string>>(new Set());

  // Filter based on selected tabs and search queries
  const filteredFiles = useMemo(() => {
    return CHAT_FILE_ITEMS.filter(file => {
      // Tab filter
      if (activeTab === 'document' && file.fileType !== 'document' && file.fileType !== 'archive') {
        return false;
      }
      if (activeTab === 'media' && file.fileType !== 'media') {
        return false;
      }
      if (activeTab === 'link' && file.fileType !== 'link') {
        return false;
      }
      if (activeTab === 'music' && file.type !== 'audio') {
        return false;
      }

      // Query filter
      if (searchQuery.trim()) {
        const query = searchQuery.toLowerCase();
        return file.title.toLowerCase().includes(query) || file.author.toLowerCase().includes(query);
      }
      return true;
    });
  }, [activeTab, searchQuery]);

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
    const allFilteredIds = filteredFiles.map(f => f.id);
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
    const selectedObjs = CHAT_FILE_ITEMS.filter(f => selectedIds.has(f.id));
    if (selectedObjs.length === 0) return;

    // Map to acceptable batch creation format
    const mapped = selectedObjs.map(f => {
      let docType = 'markdown';
      if (f.type === 'image') docType = 'image';
      else if (f.type === 'video') docType = 'video';
      else if (f.type === 'audio') docType = 'audio';

      return {
        title: f.title,
        type: docType,
        content: f.content
      };
    });

    onConfirm(mapped);
  };

  const renderFileIcon = (file: (typeof CHAT_FILE_ITEMS)[number]) => {
    const baseClass = "w-10 h-10 rounded-lg flex items-center justify-center shrink-0 shadow-sm font-sans ";
    
    if (file.fileType === 'archive') {
      return (
        <div className={baseClass + "bg-amber-100 dark:bg-amber-950/40 text-amber-600 dark:text-amber-400"}>
          <FolderArchive size={18} />
        </div>
      );
    }
    if (file.type === 'image') {
      return (
        <div className={baseClass + "bg-blue-100 dark:bg-blue-950/40 text-blue-600 dark:text-blue-400"}>
          <ImageIcon size={18} />
        </div>
      );
    }
    if (file.type === 'video') {
      return (
        <div className={baseClass + "bg-purple-100 dark:bg-purple-950/40 text-purple-600 dark:text-purple-400"}>
          <Video size={18} />
        </div>
      );
    }
    if (file.type === 'link') {
      return (
        <div className={baseClass + "bg-teal-100 dark:bg-teal-950/40 text-teal-600 dark:text-teal-400"}>
          <LinkIcon size={18} />
        </div>
      );
    }
    if (file.type === 'json' || file.type === 'toml') {
      return (
        <div className={baseClass + "bg-indigo-100 dark:bg-indigo-950/40 text-indigo-600 dark:text-indigo-400"}>
          <FileJson size={18} />
        </div>
      );
    }
    
    // Default file icon
    return (
      <div className={baseClass + "bg-gray-100 dark:bg-gray-800 text-gray-500 dark:text-gray-400"}>
        <FileText size={18} />
      </div>
    );
  };

  return createPortal(
    <div className="fixed inset-0 z-[300] bg-zinc-950/40 flex items-center justify-center backdrop-blur-md p-4 select-none">
      <div className="w-[620px] h-[580px] bg-[var(--color-kb-editor)] rounded-2xl shadow-2xl border border-[var(--color-kb-panel-border)] flex flex-col overflow-hidden animate-in zoom-in-95 duration-200">
        
        {/* Modal Header */}
        <div className="h-14 border-b border-zinc-200/80 dark:border-[var(--color-kb-panel-border)] flex items-center justify-between px-6 bg-[#fafafa] dark:bg-[var(--color-kb-panel)]/30 shrink-0">
          <div className="flex items-center gap-3">
            <div className="w-8 h-8 rounded-xl bg-emerald-50 dark:bg-[#07C160]/10 text-emerald-600 dark:text-[#07C160] flex items-center justify-center shadow-inner border border-emerald-100 dark:border-[#07C160]/20">
              <MessageSquare size={16} strokeWidth={2.5} />
            </div>
            <div>
              <h3 className="text-[14px] font-extrabold text-zinc-900 dark:text-[var(--color-kb-text-heading)]">
                {t('chatFileImportTitle', { defaultValue: '从聊天文件导入' })}
              </h3>
              <p className="text-[11px] font-medium text-zinc-500 dark:text-[var(--color-kb-text-muted)]">
                {t('chatFileImportSubtitle', { defaultValue: '接入 IM 服务后，此处将展示可选择的聊天文件' })}
              </p>
            </div>
          </div>
          <button 
            onClick={onClose} 
            className="text-zinc-400 hover:text-red-500 hover:bg-red-50 dark:text-[var(--color-kb-text-muted)] dark:hover:bg-red-500/10 p-2 rounded-xl transition-all"
          >
            <X size={16} strokeWidth={2.5} />
          </button>
        </div>

        {/* Search Bar Block */}
        <div className="p-4 bg-white dark:bg-[var(--color-kb-panel)]/5 border-b border-zinc-100 dark:border-[var(--color-kb-panel-border)]/50 shrink-0">
          <div className="flex items-center bg-[#fafafa] dark:bg-[var(--color-kb-panel)] border-2 border-zinc-200/80 dark:border-[var(--color-kb-panel-border)] hover:border-emerald-300 focus-within:border-emerald-500 focus-within:ring-4 focus-within:ring-emerald-500/10 px-4 py-2 rounded-xl transition-all shadow-sm">
            <Search size={16} className="text-zinc-400 dark:text-[var(--color-kb-text-muted)] mr-3 shrink-0" strokeWidth={2.5} />
            <div className="flex items-center gap-1.5 bg-emerald-50 dark:bg-[var(--color-kb-panel-hover)] border border-emerald-100 dark:border-[var(--color-kb-panel-border)] px-2.5 py-0.5 rounded-md text-[11.5px] text-emerald-700 dark:text-[#07C160] font-bold mr-2 shrink-0 tracking-wide">
              文件 <X size={12} className="cursor-pointer hover:text-emerald-900 transition-colors" strokeWidth={2.5} />
            </div>
            <input
              type="text"
              placeholder="搜索聊天记录文件或发送人..."
              value={searchQuery}
              onChange={(e) => setSearchQuery(e.target.value)}
              className="bg-transparent border-none outline-none text-[13.5px] font-bold text-zinc-900 dark:text-[var(--color-kb-text-heading)] placeholder-zinc-400 dark:placeholder-[var(--color-kb-text-muted)] w-full focus:ring-0 focus:outline-none"
            />
            {searchQuery && (
              <button 
                onClick={() => setSearchQuery('')}
                className="text-zinc-400 hover:text-zinc-600 dark:text-[var(--color-kb-text-muted)] dark:hover:text-current p-1 shrink-0 bg-white dark:bg-transparent rounded-md shadow-sm border border-zinc-200 dark:border-transparent"
              >
                <X size={14} strokeWidth={2.5} />
              </button>
            )}
          </div>
        </div>

        {/* Tab Selection */}
        <div className="px-5 border-b border-zinc-200/80 dark:border-[var(--color-kb-panel-border)] bg-[#fafafa] dark:bg-[var(--color-kb-panel)]/10 flex gap-4 shrink-0 overflow-x-auto overflow-hidden">
          {[
            { id: 'document', label: '文件', icon: <FileText size={14} strokeWidth={2.5} /> },
            { id: 'media', label: '图片与视频', icon: <ImageIcon size={14} strokeWidth={2.5} /> },
            { id: 'link', label: '链接', icon: <LinkIcon size={14} strokeWidth={2.5} /> },
            { id: 'music', label: '音乐', icon: <Music size={14} strokeWidth={2.5} /> },
            { id: 'other', label: '小程序', icon: <Globe size={14} strokeWidth={2.5} /> }
          ].map(tab => {
            const isActive = activeTab === tab.id;
            return (
              <button
                key={tab.id}
                onClick={() => setActiveTab(tab.id as CategoryType)}
                className={`flex items-center gap-2 px-1 py-3.5 text-[13px] font-extrabold border-b-2 transition-all shrink-0 ${
                  isActive 
                    ? 'border-emerald-500 text-emerald-600 dark:border-[#07C160] dark:text-[#07C160]' 
                    : 'border-transparent text-zinc-500 dark:text-[var(--color-kb-text-muted)] hover:text-zinc-900 dark:hover:text-[var(--color-kb-text-heading)]'
                }`}
              >
                {tab.icon}
                <span>{tab.label}</span>
              </button>
            );
          })}
        </div>

        {/* Select All Bar */}
        <div className="h-10 px-6 bg-white dark:bg-[var(--color-kb-panel)]/5 border-b border-zinc-100 dark:border-[var(--color-kb-panel-border)]/50 flex items-center justify-between shrink-0">
          <button 
            onClick={handleSelectAll}
            className="flex items-center gap-2.5 text-[12px] font-bold text-zinc-500 dark:text-[var(--color-kb-text-muted)] hover:text-emerald-600 dark:hover:text-[#07C160] transition-colors group"
          >
            {filteredFiles.length > 0 && filteredFiles.every(f => selectedIds.has(f.id)) ? (
              <div className="w-4 h-4 bg-emerald-500 text-white rounded border-2 border-emerald-500 flex items-center justify-center">
                <Check size={12} strokeWidth={3} />
              </div>
            ) : (
              <div className="w-4 h-4 border-2 border-zinc-300 dark:border-zinc-500 rounded group-hover:border-emerald-400 transition-colors"></div>
            )}
            <span>全选当前分类下的聊天文件 ({filteredFiles.length})</span>
          </button>
          
          <span className="text-[11px] text-zinc-400 dark:text-[var(--color-kb-text-muted)] font-mono font-bold tracking-wide">
            已选中: {selectedIds.size}
          </span>
        </div>

        {/* Chat Files list */}
        <div className="flex-1 overflow-y-auto divide-y divide-zinc-100 dark:divide-[var(--color-kb-panel-border)]/40 min-h-0 bg-white dark:bg-[var(--color-kb-panel)]/2">
          {filteredFiles.length === 0 ? (
            <div className="h-full flex flex-col items-center justify-center p-8 text-center">
              <div className="w-16 h-16 bg-[#fafafa] dark:bg-transparent rounded-full flex items-center justify-center mb-3">
                <FileText size={32} className="text-zinc-300 dark:text-[var(--color-kb-text-muted)]" strokeWidth={1.5} />
              </div>
              <p className="text-[13px] font-bold text-zinc-500 dark:text-[var(--color-kb-text-muted)]">
                {t('chatFileImportEmpty', { defaultValue: '暂无可导入的聊天文件。接入 IM 服务后，此处将展示可选择的文件列表。' })}
              </p>
            </div>
          ) : (
            filteredFiles.map(file => {
              const isSelected = selectedIds.has(file.id);
              return (
                <div 
                  key={file.id}
                  onClick={() => handleToggleSelect(file.id)}
                  className={`flex items-center px-6 py-3.5 cursor-pointer hover:bg-[#fafafa] dark:hover:bg-[var(--color-kb-panel-hover)]/40 transition-colors group ${
                    isSelected ? 'bg-emerald-50/50 dark:bg-[#07C160]/[0.02]' : ''
                  }`}
                >
                  {/* Selector check box */}
                  <div className="mr-5 mt-0.5 shrink-0" onClick={e => e.stopPropagation()}>
                    <button 
                      onClick={() => handleToggleSelect(file.id)}
                      className="transition-all"
                    >
                      {isSelected ? (
                        <div className="w-4 h-4 bg-emerald-500 text-white rounded border-2 border-emerald-500 flex items-center justify-center shadow-sm">
                          <Check size={12} strokeWidth={4} />
                        </div>
                      ) : (
                        <div className="w-4 h-4 bg-white dark:bg-transparent border-2 border-zinc-300 dark:border-zinc-500 rounded group-hover:border-emerald-400 transition-colors shadow-sm"></div>
                      )}
                    </button>
                  </div>

                  {/* Icon */}
                  <div className="mr-4 shrink-0 opacity-90 group-hover:opacity-100 group-hover:scale-110 transition-transform">
                    {renderFileIcon(file)}
                  </div>

                  {/* Main specs */}
                  <div className="flex-1 min-w-0 pr-4">
                    <div className="text-[13.5px] font-extrabold text-zinc-900 dark:text-[var(--color-kb-text-heading)] truncate tracking-tight">
                      {file.title}
                    </div>
                    
                    <div className="flex items-center gap-2 mt-1.5">
                      <span className="text-[10px] bg-[#fafafa] dark:bg-[var(--color-kb-panel)] border border-zinc-200/80 dark:border-[var(--color-kb-panel-border)] text-zinc-500 dark:text-[var(--color-kb-text-muted)] px-2 py-0.5 rounded font-bold shadow-sm">
                        {file.author}
                      </span>
                      <span className="text-[10.5px] font-medium text-zinc-400 dark:text-[var(--color-kb-text-muted)]">
                        来自聊天记录
                      </span>
                    </div>
                  </div>

                  {/* Sizes and Date */}
                  <div className="text-right shrink-0">
                    <div className="text-[11.5px] font-extrabold text-zinc-700 dark:text-[var(--color-kb-text-heading)] font-mono tracking-wide">
                      {file.size}
                    </div>
                    <div className="text-[10.5px] font-bold text-zinc-400 dark:text-[var(--color-kb-text-muted)] mt-1.5 font-sans">
                      {file.date}
                    </div>
                  </div>
                </div>
              );
            })
          )}
        </div>

        {/* Footer */}
        <div className="h-16 border-t border-zinc-200/80 dark:border-[var(--color-kb-panel-border)] bg-[#fafafa] dark:bg-[var(--color-kb-panel)] flex items-center justify-between px-6 shrink-0 rounded-b-2xl z-20">
          <div className="text-[12.5px] font-medium text-zinc-500 dark:text-[var(--color-kb-text-muted)]">
            已勾选 <strong className="text-zinc-900 dark:text-[var(--color-kb-text-heading)] font-extrabold font-mono mx-1">{selectedIds.size}</strong> 篇聊天文件
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
              确定导入选中的文件
            </button>
          </div>
        </div>

      </div>
    </div>,
    document.body
  );
}
