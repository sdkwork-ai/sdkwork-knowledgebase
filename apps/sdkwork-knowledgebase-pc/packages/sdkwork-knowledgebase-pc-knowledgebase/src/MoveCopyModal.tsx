import React, { useState, useEffect } from 'react';
import { useTranslation } from 'react-i18next';
import { X, Folder, ChevronRight, File, Book, Globe, ArrowLeft, ArrowRight, Copy, FolderOutput, Check } from 'lucide-react';
import { DocumentMeta, FolderNode, KnowledgeBase, DocumentService } from './services/document';

interface MoveCopyModalProps {
  action: 'move' | 'copy' | 'save_as';
  item: DocumentMeta | FolderNode | { id: string; title: string; type: string };
  activeKb: KnowledgeBase | null;
  onClose: () => void;
  onSubmit: (targetKbId: string, targetFolderId: string | null) => void;
}

export function MoveCopyModal({ action, item, activeKb, onClose, onSubmit }: MoveCopyModalProps) {
  const { t } = useTranslation(['kb', 'common']);
  const [kbs, setKbs] = useState<{ team: KnowledgeBase[], personal: KnowledgeBase[], public: KnowledgeBase[] } | null>(null);
  const [selectedKbId, setSelectedKbId] = useState<string | null>(activeKb?.id || ('kbId' in item ? item.kbId : null) || null);
  const [kbDocsTree, setKbDocsTree] = useState<any[]>([]);
  const [loadingDocs, setLoadingDocs] = useState(false);
  const [navigationStack, setNavigationStack] = useState<{ id: string, title: string, children: any[] }[]>([]);

  useEffect(() => {
    DocumentService.getKnowledgeBases().then(setKbs);
  }, []);

  useEffect(() => {
    if (selectedKbId) {
      setLoadingDocs(true);
      DocumentService.getDocuments(selectedKbId).then(docs => {
        // Filter out the item itself to prevent moving/copying into itself or its descendants
        const filterItem = (nodes: any[]): any[] => {
          return nodes
            .filter(node => node.id !== item.id) // remove the item itself
            .map(node => {
              if (node.children) {
                return { ...node, children: filterItem(node.children) };
              }
              return node;
            });
        };
        const filteredDocs = filterItem(docs);
        setKbDocsTree(filteredDocs);
        setNavigationStack([]);
        setLoadingDocs(false);
      });
    } else {
      setKbDocsTree([]);
      setNavigationStack([]);
    }
  }, [selectedKbId, item.id]);

  const currentFolderNodes = navigationStack.length > 0 
    ? navigationStack[navigationStack.length - 1].children 
    : kbDocsTree;

  // Flatten to find a selected folder from another component if we wished, but here it's click-to-navigate.

  const handleBack = () => {
    if (navigationStack.length > 0) {
      setNavigationStack(prev => prev.slice(0, prev.length - 1));
    }
  };

  const selectedKbTitle = [...(kbs?.team || []), ...(kbs?.personal || []), ...(kbs?.public || [])]
    .find(k => k.id === selectedKbId)?.title || '';

  const getBreadcrumbs = () => {
    const list = [{ id: 'root', title: selectedKbTitle }];
    navigationStack.forEach(n => list.push({ id: n.id, title: n.title }));
    return list;
  };
  const breadcrumbs = getBreadcrumbs();

  return (
    <div className="fixed inset-0 z-[500] flex items-center justify-center bg-zinc-950/40 backdrop-blur-md">
      <div className="bg-white dark:bg-[var(--color-kb-editor)] border border-zinc-200/80 dark:border-[var(--color-kb-panel-border)] shadow-2xl rounded-2xl w-full max-w-[1100px] h-[75vh] min-h-[600px] max-h-[850px] flex flex-col overflow-hidden animate-in zoom-in-95 duration-200">
        {/* Header */}
        <div className="px-6 py-5 border-b border-zinc-200/80 dark:border-[var(--color-kb-panel-border)] bg-[#fafafa] dark:bg-[var(--color-kb-panel)]/30 shrink-0 relative z-10 shadow-sm">
          <div className="flex items-center justify-between">
            <h2 className="text-[15px] font-extrabold text-zinc-900 dark:text-[var(--color-kb-text-heading)] flex items-center gap-3">
              <div className="w-8 h-8 rounded-xl bg-indigo-50 dark:bg-[var(--color-kb-panel)] border border-indigo-100 dark:border-[var(--color-kb-panel-border)] flex items-center justify-center shadow-inner">
                {action === 'move' ? <FolderOutput size={16} className="text-indigo-600 dark:text-[var(--color-kb-text-muted)]" strokeWidth={2.5} /> : <Copy size={16} className="text-indigo-600 dark:text-[var(--color-kb-text-muted)]" strokeWidth={2.5} />}
              </div>
              <div className="flex items-center">
                <span className="text-zinc-500 dark:text-zinc-400 font-medium">
                  {action === 'move' ? t('moveTitle') : action === 'save_as' ? '另存为：' : t('copyTitle')}
                </span>
                <span className="tracking-tight">{item.title}</span>
              </div>
            </h2>
            <button
              onClick={onClose}
              className="p-2 rounded-xl hover:bg-red-50 hover:text-red-500 text-zinc-400 dark:text-[var(--color-kb-text-muted)] dark:hover:bg-red-500/10 transition-all active:scale-95"
            >
              <X size={16} strokeWidth={2.5} />
            </button>
          </div>
        </div>

        {/* Top Navigation Bar / Breadcrumbs */}
        <div className="px-4 py-3 flex items-center gap-1 border-b border-zinc-100 dark:border-[var(--color-kb-panel-border)]/60 bg-white dark:bg-[var(--color-kb-panel)]/30 shrink-0 shadow-[0_4px_20px_rgba(0,0,0,0.02)] z-10 relative">
          <div className="flex items-center bg-[#fafafa] border border-zinc-200/80 dark:bg-black/20 dark:border-transparent p-1 rounded-xl shadow-inner mr-2">
            <button 
              onClick={handleBack}
              disabled={navigationStack.length === 0}
              className={`p-1.5 rounded-lg transition-all active:scale-95 ${navigationStack.length === 0 ? 'text-zinc-300 dark:text-[var(--color-kb-panel-border)] cursor-not-allowed' : 'text-zinc-600 hover:text-zinc-900 hover:bg-white shadow-sm border border-transparent hover:border-zinc-200/50 dark:text-[var(--color-kb-text-muted)]'}`}
            >
              <ArrowLeft size={16} strokeWidth={2.5} />
            </button>
            <div className="w-[1px] h-4 bg-zinc-200 dark:bg-zinc-800 mx-1" />
            <button 
              disabled={true}
              className={`p-1.5 rounded-lg transition-colors text-zinc-300 dark:text-[var(--color-kb-panel-border)] cursor-not-allowed`}
            >
              <ArrowRight size={16} strokeWidth={2.5} />
            </button>
          </div>
          
          <div className="flex items-center text-[13.5px] font-medium text-zinc-600 dark:text-[var(--color-kb-text)] overflow-hidden shrink min-w-0 pr-4">
             {breadcrumbs.map((bc, idx) => (
                <React.Fragment key={bc.id}>
                  {idx > 0 && <span className="text-zinc-300 dark:text-[var(--color-kb-text-muted)] mx-1.5 shrink-0"><ChevronRight size={14} strokeWidth={3} /></span>}
                  <span className={`truncate max-w-[200px] ${idx === breadcrumbs.length - 1 ? 'font-extrabold text-zinc-900 dark:text-[var(--color-kb-text-heading)]' : 'text-zinc-500 font-bold dark:text-[var(--color-kb-text)] cursor-pointer hover:bg-zinc-100 dark:hover:bg-[var(--color-kb-panel-hover)] px-2.5 py-1.5 rounded-md transition-colors'}`}
                        onClick={() => {
                          if (idx === 0) setNavigationStack([]);
                          else setNavigationStack(prev => prev.slice(0, idx));
                        }}
                  >
                    {bc.title}
                  </span>
                </React.Fragment>
             ))}
          </div>
        </div>

        {/* Content Area */}
        <div className="flex-1 flex overflow-hidden min-h-0 bg-white dark:bg-[var(--color-kb-editor)]">
          {/* Left Sidebar - Knowledge Bases */}
          <div className="w-[280px] bg-[#fafafa]/50 dark:bg-[var(--color-kb-panel)]/20 border-r border-zinc-200/80 dark:border-[var(--color-kb-panel-border)]/60 flex flex-col min-h-0 shrink-0 select-none">
            <div className="flex-1 overflow-y-auto w-full no-scrollbar px-3 py-5 space-y-6 text-[13px]">
              
              {/* Personal */}
              {kbs?.personal && kbs.personal.length > 0 && (
                <div className="space-y-1 w-full min-w-0">
                    <div className="px-3 pb-2 text-[11px] font-extrabold tracking-wider text-zinc-400 dark:text-[var(--color-kb-text-muted)] mt-2 mb-2 uppercase">个人知识库</div>
                   {kbs.personal.map(kb => (
                      <div 
                        key={kb.id}
                        onClick={() => setSelectedKbId(kb.id)}
                        className={`flex items-center px-3.5 py-2.5 h-[42px] mx-1 mb-[4px] min-w-0 overflow-hidden rounded-[12px] cursor-pointer transition-all active:scale-[0.98] ${selectedKbId === kb.id ? 'bg-indigo-50 border border-indigo-100 text-indigo-900 dark:bg-[var(--color-kb-panel-active)] dark:border-transparent dark:text-[var(--color-kb-text-heading)] font-extrabold shadow-sm' : 'text-zinc-600 border border-transparent dark:text-[var(--color-kb-text)] hover:bg-black/5 dark:hover:bg-[var(--color-kb-panel-hover)] font-medium'}`}
                      >
                         <Book size={16} strokeWidth={selectedKbId === kb.id ? 2.5 : 2} className={`mr-3 shrink-0 ${selectedKbId === kb.id ? 'text-indigo-600 dark:text-[var(--color-kb-accent)]' : 'text-zinc-400 dark:text-[var(--color-kb-text-muted)]'}`} />
                         <span className="truncate flex-1 text-[13.5px] tracking-tight">{kb.title}</span>
                      </div>
                   ))}
                </div>
              )}

              {/* Team */}
              {kbs?.team && kbs.team.length > 0 && (
                <div className="space-y-1 w-full min-w-0">
                   <div className="px-3 pb-2 text-[11px] font-extrabold tracking-wider text-zinc-400 dark:text-[var(--color-kb-text-muted)] mt-4 mb-2 uppercase flex items-center justify-between">
                     <span>共享知识库</span>
                     <div className="w-5 h-5 rounded-md bg-zinc-200/50 dark:bg-zinc-700/50 flex items-center justify-center text-[10px] text-zinc-500 dark:text-zinc-300">{kbs.team.length}</div>
                   </div>
                   {kbs.team.map(kb => (
                      <div 
                        key={kb.id}
                        onClick={() => setSelectedKbId(kb.id)}
                        className={`flex items-center px-3.5 py-2.5 h-[42px] mx-1 mb-[4px] min-w-0 overflow-hidden rounded-[12px] cursor-pointer transition-all active:scale-[0.98] ${selectedKbId === kb.id ? 'bg-indigo-50 border border-indigo-100 text-indigo-900 dark:bg-[var(--color-kb-panel-active)] dark:border-transparent dark:text-[var(--color-kb-text-heading)] font-extrabold shadow-sm' : 'text-zinc-600 border border-transparent dark:text-[var(--color-kb-text)] hover:bg-black/5 dark:hover:bg-[var(--color-kb-panel-hover)] font-medium'}`}
                      >
                         <Globe size={16} strokeWidth={selectedKbId === kb.id ? 2.5 : 2} className={`mr-3 shrink-0 ${selectedKbId === kb.id ? 'text-indigo-600 dark:text-[var(--color-kb-accent)]' : 'text-zinc-400 dark:text-[var(--color-kb-text-muted)]'}`} />
                         <span className="truncate flex-1 text-[13.5px] tracking-tight">{kb.title}</span>
                      </div>
                   ))}
                </div>
              )}
            </div>
          </div>

          {/* Right Pane - Documents list */}
          <div className="flex-1 overflow-y-auto no-scrollbar relative bg-white dark:bg-transparent">
             {loadingDocs ? (
               <div className="absolute inset-0 flex items-center justify-center pt-10 text-[14px] font-bold text-zinc-400 dark:text-[var(--color-kb-text-muted)] bg-white/50 backdrop-blur-sm z-10 transition-all">
                  <div className="animate-spin w-6 h-6 border-4 border-indigo-500/20 border-t-indigo-500 rounded-full mr-3"></div>
                  读取文档结构...
               </div>
             ) : currentFolderNodes.length === 0 ? (
               <div className="flex flex-col items-center justify-center h-full text-[14px] text-zinc-400 dark:text-[var(--color-kb-text-muted)]">
                  <div className="w-16 h-16 bg-zinc-50 dark:bg-zinc-900 border-2 border-zinc-100 dark:border-zinc-800 rounded-2xl flex items-center justify-center mb-4 shadow-sm">
                    <Folder className="w-8 h-8 text-zinc-300 dark:text-[var(--color-kb-panel-border)]" strokeWidth={2} />
                  </div>
                  <span className="font-bold">此文件夹为空</span>
                  <p className="text-[12px] font-medium mt-1">您可以直接将内容{action === 'move' ? '移动' : action === 'save_as' ? '保存' : '复制'}到此层级</p>
               </div>
             ) : (
               <div className="p-4 space-y-1">
                 {currentFolderNodes.map(node => {
                    const isFolder = node.type === 'folder';
                    const Icon = isFolder ? Folder : File;
                    return (
                      <div 
                        key={node.id}
                        onDoubleClick={() => {
                          if (isFolder) {
                            setNavigationStack(prev => [...prev, { id: node.id, title: node.title, children: node.children || [] }]);
                          }
                        }}
                        className={`flex items-center justify-between px-5 py-3.5 rounded-xl group transition-all Select-none border-2 border-transparent ${isFolder ? 'cursor-pointer hover:border-zinc-200/80 hover:bg-zinc-50 hover:shadow-sm dark:hover:bg-[var(--color-kb-panel-hover)]' : 'opacity-60 cursor-default grayscale'}`}
                      >
                        <div className="flex items-center min-w-0">
                           <Icon size={20} strokeWidth={isFolder ? 2.5 : 2} className={`mr-4 shrink-0 transition-colors ${isFolder ? 'text-blue-400 dark:text-[var(--color-kb-text-muted)] group-hover:text-blue-500 group-hover:drop-shadow-sm' : 'text-zinc-300 dark:text-[var(--color-kb-panel-border)]'}`} />
                           <span className={`text-[14px] truncate tracking-tight transition-colors ${isFolder ? 'font-extrabold text-zinc-800 dark:text-[var(--color-kb-text)] group-hover:text-indigo-900 group-hover:dark:text-[var(--color-kb-text-heading)]' : 'font-medium text-zinc-400 dark:text-[var(--color-kb-text-muted)]'}`}>
                             {node.title}
                           </span>
                        </div>
                        <div className="flex items-center gap-2">
                          <div className={`text-[12.5px] font-bold text-zinc-400 dark:text-[var(--color-kb-text-muted)]/60 px-2 py-1 bg-zinc-100 rounded-md pointer-events-none group-hover:text-zinc-600 dark:bg-transparent dark:group-hover:text-[var(--color-kb-text-muted)] transition-colors ${!isFolder && 'opacity-0'}`}>
                             双击进入
                          </div>
                        </div>
                      </div>
                    );
                 })}
               </div>
             )}
          </div>
        </div>

        {/* Footer */}
        <div className="px-6 py-4 h-[70px] border-t border-zinc-200/80 dark:border-[var(--color-kb-panel-border)]/60 flex justify-end items-center gap-3 bg-[#fafafa] dark:bg-[var(--color-kb-editor)] shrink-0 z-10 shadow-[0_-4px_20px_rgba(0,0,0,0.02)]">
          <button
            onClick={onClose}
            className="px-6 py-2.5 rounded-xl text-[13px] font-bold text-zinc-600 dark:text-[var(--color-kb-text)] hover:bg-zinc-200/60 hover:text-zinc-900 dark:hover:bg-[var(--color-kb-panel-hover)] dark:hover:text-[var(--color-kb-text-heading)] transition-all border-2 border-transparent hover:border-zinc-300/80 dark:hover:border-[var(--color-kb-panel-border)]/50 focus:outline-none active:scale-95"
          >
            取消
          </button>
          <button
            disabled={!selectedKbId}
            onClick={() => {
              if (selectedKbId) {
                const targetFolderId = navigationStack.length > 0 ? navigationStack[navigationStack.length - 1].id : null;
                onSubmit(selectedKbId, targetFolderId);
              }
            }}
            className="px-6 py-2.5 rounded-xl text-[13.5px] font-extrabold text-white bg-zinc-900 dark:bg-[var(--color-kb-text)] hover:bg-indigo-600 hover:shadow-lg hover:shadow-indigo-500/20 transition-all shadow-md focus:outline-none focus:ring-4 focus:ring-indigo-500/20 disabled:opacity-40 disabled:grayscale disabled:shadow-none active:scale-95 flex items-center gap-2"
          >
            {action === 'move' ? <FolderOutput size={16} strokeWidth={3} /> : <Check size={16} strokeWidth={3} />}
            {action === 'move' ? '移动至此' : action === 'save_as' ? '选择并另存为' : '确认并复制'}
          </button>
        </div>
      </div>
    </div>
  );
}

