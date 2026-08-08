import { useState, useEffect, useMemo } from 'react';
import { isBlank } from '@sdkwork/utils';
import { createPortal } from 'react-dom';
import { 
  X, Search, Library, FileText, Hash, Image as ImageIcon, 
  Video, Music, CheckSquare, Square,
  Inbox, RefreshCw
} from 'lucide-react';
import { useTranslation } from 'react-i18next';
import { DocumentService, KnowledgeBase } from './services/document';

interface PersonalKbModalProps {
  isOpen: boolean;
  onClose: () => void;
  onConfirm: (selectedItems: Array<{ id: string; kbId: string; title: string; type: string; content?: string; url?: string }>) => void;
}

export function PersonalKbModal({ isOpen, onClose, onConfirm }: PersonalKbModalProps) {
  const { t } = useTranslation(['kb', 'common']);
  
  const [personalKbs, setPersonalKbs] = useState<KnowledgeBase[]>([]);
  const [activeKbId, setActiveKbId] = useState<string | null>(null);
  const [rawDocs, setRawDocs] = useState<any[]>([]);
  const [loading, setLoading] = useState(false);
  const [searchQuery, setSearchQuery] = useState('');
  const [selectedIds, setSelectedIds] = useState<Set<string>>(new Set());

  // Restore defaults on close/open
  useEffect(() => {
    if (isOpen) {
      setLoading(true);
      DocumentService.getKnowledgeBases()
        .then(data => {
          const personalList = data.personal || [];
          setPersonalKbs(personalList);
          if (personalList.length > 0) {
            setActiveKbId(personalList[0].id);
          } else {
            setActiveKbId(null);
          }
        })
        .finally(() => {
          setLoading(false);
        });
    } else {
      setSelectedIds(new Set());
      setSearchQuery('');
    }
  }, [isOpen]);

  // Load documents for active KB
  useEffect(() => {
    if (isOpen && activeKbId) {
      setLoading(true);
      DocumentService.getDocuments(activeKbId)
        .then(docs => {
          setRawDocs(docs);
        })
        .catch(err => {
          console.error('Error fetching direct personal docs', err);
        })
        .finally(() => {
          setLoading(false);
        });
      setSelectedIds(new Set());
    }
  }, [activeKbId, isOpen]);

  // Helper to flatten documents tree for picking non-folder files
  const flatFiles = useMemo(() => {
    const result: any[] = [];
    const traverse = (items: any[]) => {
      items.forEach(item => {
        if (item.type !== 'folder') {
          result.push(item);
        }
        if (item.children && item.children.length > 0) {
          traverse(item.children);
        }
      });
    };
    traverse(rawDocs);
    return result;
  }, [rawDocs]);

  // Filtered by Search Query
  const displayedFiles = useMemo(() => {
    if (isBlank(searchQuery)) return flatFiles;
    const query = searchQuery.toLowerCase().trim();
    return flatFiles.filter(f => f.title.toLowerCase().includes(query));
  }, [flatFiles, searchQuery]);

  // Selection state helpers
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
    if (selectedIds.size === displayedFiles.length) {
      setSelectedIds(new Set());
    } else {
      setSelectedIds(new Set(displayedFiles.map(f => f.id)));
    }
  };

  const handleImportSubmit = () => {
    const selectedObjs = flatFiles.filter(f => selectedIds.has(f.id));
    if (selectedObjs.length === 0 || !activeKbId) return;

    const mapped = selectedObjs.map(doc => ({
      id: doc.id,
      kbId: activeKbId,
      title: doc.title,
      type: doc.type,
      content: doc.content || '',
      url: doc.url
    }));

    onConfirm(mapped);
  };

  const activeKbDetails = useMemo(() => {
    return personalKbs.find(kb => kb.id === activeKbId) || null;
  }, [personalKbs, activeKbId]);

  if (!isOpen) return null;

  const renderIcon = (type: string) => {
    if (type === 'code') return <Hash size={15} className="text-orange-500" />;
    if (type === 'image') return <ImageIcon size={15} className="text-blue-500" />;
    if (type === 'video') return <Video size={15} className="text-purple-500" />;
    if (type === 'audio') return <Music size={15} className="text-yellow-500" />;
    if (type === 'pdf') return <FileText size={15} className="text-red-500" />;
    return <FileText size={15} className="text-emerald-500" />;
  };

  return createPortal(
    <div className="fixed inset-0 z-[300] bg-zinc-950/40 flex items-center justify-center backdrop-blur-sm p-4 select-none">
      <div className="w-[900px] h-[580px] bg-white dark:bg-[var(--color-kb-editor)] rounded-2xl shadow-2xl border border-zinc-200/80 dark:border-[var(--color-kb-panel-border)] flex flex-col overflow-hidden animate-in zoom-in-95 duration-200">
        
        {/* Modal Header */}
        <div className="h-16 border-b border-zinc-200/80 dark:border-[var(--color-kb-panel-border)] flex items-center justify-between px-6 bg-[#fafafa] dark:bg-[var(--color-kb-panel)]/30 shrink-0 shadow-sm z-10">
          <div className="flex items-center gap-3">
            <div className="p-2 bg-gradient-to-tr from-amber-50 to-orange-50 dark:from-amber-500/20 dark:to-amber-500/10 border border-amber-100 dark:border-transparent text-amber-600 dark:text-amber-500 rounded-xl shadow-inner">
              <Library size={18} strokeWidth={2.5} />
            </div>
            <div>
              <h3 className="text-[15px] font-extrabold tracking-tight text-zinc-900 dark:text-[var(--color-kb-text-heading)] leading-tight">{t('importFromPersonal')}</h3>
              <p className="text-[11.5px] font-medium text-zinc-500 dark:text-[var(--color-kb-text-muted)] tracking-wide">{t('importFromPersonalDesc')}</p>
            </div>
          </div>
          <button 
            onClick={onClose} 
            className="text-zinc-400 hover:text-red-500 hover:bg-red-50 dark:text-[var(--color-kb-text-muted)] dark:hover:bg-red-500/10 p-2 rounded-xl transition-all active:scale-95"
          >
            <X size={16} strokeWidth={2.5} />
          </button>
        </div>

        {/* Modal Main Body */}
        <div className="flex-1 flex min-h-0">
          
          {/* Left Sidebar - Personal KBs Selection */}
          <div className="w-[220px] border-r border-zinc-200/80 dark:border-[var(--color-kb-panel-border)] bg-zinc-50 dark:bg-[var(--color-kb-panel)]/15 flex flex-col shrink-0 min-h-0">
            <div className="px-4 py-3 shrink-0 mt-1">
              <div className="text-[11px] font-extrabold text-zinc-400 dark:text-[var(--color-kb-text-muted)] tracking-wider uppercase mb-1">{t('personalKbList')}</div>
            </div>
            
            <div className="flex-1 overflow-y-auto px-3 space-y-1">
              {personalKbs.map((kb) => {
                const isActive = kb.id === activeKbId;
                return (
                  <button
                    key={kb.id}
                    onClick={() => setActiveKbId(kb.id)}
                    className={`w-full flex items-center px-3 py-2.5 rounded-xl text-[13px] font-bold transition-all text-left ${
                      isActive 
                        ? 'bg-amber-100/50 dark:bg-[var(--color-kb-accent)]/10 text-amber-700 dark:text-[var(--color-kb-accent)] shadow-sm border border-amber-200/50 dark:border-transparent' 
                        : 'text-zinc-600 dark:text-[var(--color-kb-text)] hover:bg-white dark:hover:bg-[var(--color-kb-panel-hover)] border border-transparent'
                    }`}
                  >
                    <span className="text-[16px] mr-2.5 leading-none shrink-0">{kb.icon || '📓'}</span>
                    <span className="truncate flex-1 tracking-tight">{kb.title}</span>
                  </button>
                );
              })}
              
              {personalKbs.length === 0 && !loading && (
                <div className="p-4 text-center">
                  <p className="text-[12px] font-medium text-zinc-400 dark:text-[var(--color-kb-text-muted)] mt-4">{t('noPersonalKb')}</p>
                </div>
              )}
            </div>
          </div>

          {/* Right Panel - Files List of Selected KB */}
          <div className="flex-1 flex flex-col bg-white dark:bg-[var(--color-kb-editor)] overflow-hidden min-w-0">
            
            {/* Search Header */}
            <div className="p-4 border-b border-zinc-200/80 dark:border-[var(--color-kb-panel-border)] bg-[#fafafa] dark:bg-[var(--color-kb-panel)]/5 flex gap-3 shrink-0">
              <div className="flex-1 flex items-center bg-white dark:bg-[var(--color-kb-panel)] border-2 border-zinc-200/80 dark:border-[var(--color-kb-panel-border)] hover:border-amber-300 dark:hover:border-[var(--color-kb-accent)]/50 focus-within:border-amber-500 focus-within:ring-4 focus-within:ring-amber-500/10 px-3 py-1.5 rounded-xl transition-all h-10 shadow-sm">
                <Search size={16} strokeWidth={2.5} className="text-zinc-400 dark:text-[var(--color-kb-text-muted)] mr-2 shrink-0" />
                <input
                  type="text"
                  placeholder={`${t('searchInKb', { kbName: activeKbDetails?.title || '未选择' })}`}
                  value={searchQuery}
                  onChange={(e) => setSearchQuery(e.target.value)}
                  className="bg-transparent border-none outline-none text-[13px] font-bold text-zinc-900 dark:text-[var(--color-kb-text-heading)] placeholder-zinc-400 dark:placeholder-[var(--color-kb-text-muted)] w-full focus:ring-0 focus:outline-none"
                />
                {searchQuery && (
                  <button 
                    onClick={() => setSearchQuery('')}
                    className="text-zinc-400 hover:text-zinc-600 dark:hover:text-current p-1 shrink-0 bg-[#fafafa] dark:bg-transparent rounded-md"
                  >
                    <X size={14} strokeWidth={2.5} />
                  </button>
                )}
              </div>
            </div>

            {/* Document List */}
            <div className="flex-1 overflow-y-auto relative min-h-0">
              {loading ? (
                <div className="absolute inset-0 flex flex-col items-center justify-center bg-white/70 dark:bg-[var(--color-kb-editor)]/70 z-10 backdrop-blur-sm">
                  <RefreshCw size={24} className="text-amber-500 dark:text-[var(--color-kb-accent)] animate-spin mb-3" />
                  <span className="text-[13px] font-bold text-zinc-500 dark:text-[var(--color-kb-text-muted)]">{t('loadingDocs')}</span>
                </div>
              ) : null}

              {displayedFiles.length === 0 && !loading ? (
                <div className="flex-1 h-full flex flex-col items-center justify-center p-8 text-center bg-transparent">
                  <div className="w-16 h-16 bg-[#fafafa] dark:bg-transparent rounded-full flex items-center justify-center mb-4">
                     <Inbox size={32} className="text-zinc-300 dark:text-[var(--color-kb-text-muted)]" strokeWidth={1.5} />
                  </div>
                  <h4 className="text-[14px] font-extrabold text-zinc-900 dark:text-[var(--color-kb-text-heading)]">{t('noImportableDocs')}</h4>
                  <p className="text-[12px] font-medium text-zinc-500 dark:text-[var(--color-kb-text-muted)] max-w-xs mt-1.5 leading-relaxed">{t('noImportableDocsDesc')}</p>
                </div>
              ) : (
                <div className="w-full">
                  {/* Select All Row */}
                  <div className="h-12 border-b border-zinc-200/80 dark:border-[var(--color-kb-panel-border)] bg-[#fafafa] dark:bg-[var(--color-kb-panel)]/5 flex items-center px-6 sticky top-0 z-10 shadow-sm">
                    <button 
                      onClick={handleSelectAll}
                      className="flex items-center gap-3 text-[12.5px] font-bold text-zinc-500 dark:text-[var(--color-kb-text-muted)] hover:text-amber-600 dark:hover:text-[var(--color-kb-accent)] transition-colors group"
                    >
                      {selectedIds.size === displayedFiles.length ? (
                        <CheckSquare size={18} strokeWidth={2.5} className="text-amber-500 dark:text-[var(--color-kb-accent)]" />
                      ) : (
                        <Square size={18} strokeWidth={2.5} className="opacity-80 group-hover:opacity-100" />
                      )}
                      <span>{t('selectAllPage', { count: displayedFiles.length })}</span>
                    </button>
                  </div>

                  {/* List Container */}
                  <div className="divide-y divide-zinc-100 dark:divide-[var(--color-kb-panel-border)]/50">
                    {displayedFiles.map((doc) => {
                      const isSelected = selectedIds.has(doc.id);
                      return (
                        <div 
                          key={doc.id}
                          onClick={() => handleToggleSelect(doc.id)}
                          className={`flex items-center px-6 py-3.5 cursor-pointer transition-colors hover:bg-[#fafafa] dark:hover:bg-[var(--color-kb-panel-hover)]/60 group ${isSelected ? 'bg-amber-50/50 dark:bg-[var(--color-kb-accent)]/[0.03]' : ''}`}
                        >
                          <div className="mr-5 shrink-0" onClick={(e) => e.stopPropagation()}>
                            <button onClick={() => handleToggleSelect(doc.id)} className="text-zinc-300 dark:text-[var(--color-kb-text-muted)] hover:text-amber-500 dark:hover:text-[var(--color-kb-accent)] transition-all">
                              {isSelected ? (
                                <CheckSquare size={18} strokeWidth={2.5} className="text-amber-500 dark:text-[var(--color-kb-accent)]" />
                              ) : (
                                <Square size={18} strokeWidth={2.5} className="opacity-90 group-hover:opacity-100" />
                              )}
                            </button>
                          </div>
                          
                          <div className="shrink-0 mr-4 group-hover:scale-110 transition-transform">
                            {renderIcon(doc.type)}
                          </div>

                          <div className="flex-1 min-w-0 pr-4">
                            <div className={`text-[13.5px] font-bold truncate tracking-tight transition-colors ${isSelected ? 'text-zinc-900 dark:text-[var(--color-kb-text-heading)]' : 'text-zinc-700 dark:text-[var(--color-kb-text-heading)]'}`}>{doc.title}</div>
                            {doc.author && (
                              <div className="text-[11px] font-medium text-zinc-400 dark:text-[var(--color-kb-text-muted)] mt-1 font-sans">
                                {t('editorPrefix')}{doc.author}
                              </div>
                            )}
                          </div>

                          <div className="text-[12px] font-bold text-zinc-400 dark:text-[var(--color-kb-text-muted)] font-mono text-right shrink-0">
                            {doc.updatedAt ? doc.updatedAt.slice(0, 10) : '--'}
                          </div>
                        </div>
                      );
                    })}
                  </div>
                </div>
              )}

            </div>
          </div>
        </div>

        {/* Modal Footer */}
        <div className="h-16 border-t border-zinc-200/80 dark:border-[var(--color-kb-panel-border)] bg-[#fafafa] dark:bg-[var(--color-kb-panel)] flex items-center justify-between px-6 shrink-0 rounded-b-2xl shadow-[0_-4px_20px_rgba(0,0,0,0.02)] z-20">
          <div className="text-[12.5px] font-medium text-zinc-500 dark:text-[var(--color-kb-text-muted)]">
            {t('selectedPersonalDocs', { count: selectedIds.size })}
          </div>
          
          <div className="flex items-center gap-3">
            <button
              onClick={onClose}
              className="px-5 py-2.5 text-[13px] font-bold text-zinc-600 dark:text-[var(--color-kb-text-heading)] bg-white dark:bg-[var(--color-kb-editor)] hover:bg-zinc-100 dark:hover:bg-[var(--color-kb-panel-hover)] border-2 border-transparent hover:border-zinc-200/80 dark:border-[var(--color-kb-panel-border)] rounded-xl transition-all shadow-sm active:scale-95"
            >
              取消
            </button>
            <button
              onClick={handleImportSubmit}
              disabled={selectedIds.size === 0 || loading}
              className="px-6 py-2.5 text-[13px] font-semibold bg-[var(--color-kb-accent)] hover:bg-[var(--color-kb-accent-hover)] disabled:opacity-40 disabled:grayscale text-white rounded-xl shadow-md shadow-[var(--color-kb-accent)]/10 hover:shadow-lg transition-all disabled:shadow-none active:scale-95 focus:outline-none focus:ring-4 focus:ring-[var(--color-kb-accent)]/20"
            >
              {t('importSelected', { count: selectedIds.size })}
            </button>
          </div>
        </div>

      </div>
    </div>,
    document.body
  );
}
