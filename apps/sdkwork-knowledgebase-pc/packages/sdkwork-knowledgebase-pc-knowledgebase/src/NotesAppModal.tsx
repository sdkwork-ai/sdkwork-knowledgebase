import { useState } from 'react';
import { useTranslation } from 'react-i18next';
import { createPortal } from 'react-dom';
import { X, Notebook, CheckCircle2, CheckSquare, Square } from 'lucide-react';

interface NotesAppModalProps {
  isOpen: boolean;
  onClose: () => void;
  onConfirm: (selectedItems: Array<{ title: string; type: string; content?: string }>) => void;
}

const NOTES_ITEMS: Array<{
  id: string;
  title: string;
  type: string;
  updatedAt: string;
  sizeInfo: string;
  content: string;
}> = [];

export function NotesAppModal({ isOpen, onClose, onConfirm }: NotesAppModalProps) {
  const { t } = useTranslation(['kb', 'common']);
  const [selectedIds, setSelectedIds] = useState<Set<string>>(new Set());

  if (!isOpen) return null;

  const toggleSelect = (id: string) => {
    const next = new Set(selectedIds);
    if (next.has(id)) {
      next.delete(id);
    } else {
      next.add(id);
    }
    setSelectedIds(next);
  };

  const handleImportClick = () => {
    const selectedObjs = NOTES_ITEMS.filter(n => selectedIds.has(n.id));
    if (selectedObjs.length === 0) return;

    const mapped = selectedObjs.map(n => ({
      title: n.title,
      type: n.type,
      content: n.content
    }));

    onConfirm(mapped);
  };

  return createPortal(
    <div className="fixed inset-0 z-[300] bg-zinc-950/40 flex items-center justify-center backdrop-blur-sm p-4 select-none">
      <div className="w-[480px] bg-white dark:bg-[var(--color-kb-editor)] rounded-2xl shadow-2xl border border-zinc-200/80 dark:border-[var(--color-kb-panel-border)] animate-in zoom-in-95 duration-200 overflow-hidden">
         
         <div className="px-6 py-4 border-b border-zinc-200/80 dark:border-[var(--color-kb-panel-border)] flex items-center justify-between bg-[#fafafa] dark:bg-[var(--color-kb-panel)]/30 shadow-sm z-10 shrink-0">
           <div className="flex items-center gap-3">
             <div className="w-9 h-9 rounded-xl bg-orange-50 dark:bg-orange-500/10 border border-orange-100 dark:border-transparent text-orange-500 flex items-center justify-center shadow-inner">
               <Notebook size={18} strokeWidth={2.5} />
             </div>
             <div>
               <h3 className="text-[15px] font-extrabold tracking-tight leading-tight text-zinc-900 dark:text-[var(--color-kb-text-heading)]">{t('importNotes')}</h3>
               <p className="text-[11.5px] font-medium text-zinc-500 dark:text-[var(--color-kb-text-muted)] tracking-wide mt-0.5">{t('importNotesDesc')}</p>
             </div>
           </div>
           <button onClick={onClose} className="text-zinc-400 hover:text-red-500 hover:bg-red-50 dark:text-[var(--color-kb-text-muted)] dark:hover:bg-red-500/10 transition-all p-2 rounded-xl active:scale-95"><X size={16} strokeWidth={2.5} /></button>
         </div>

         <div className="p-6">
            <h4 className="text-[13px] font-extrabold tracking-tight text-zinc-900 dark:text-[var(--color-kb-text-heading)] mb-3">{t('checkNotes')}</h4>
            <div className="border border-zinc-200/80 dark:border-[var(--color-kb-panel-border)] rounded-2xl overflow-hidden divide-y divide-zinc-200/80 dark:divide-[var(--color-kb-panel-border)] bg-zinc-50/50 dark:bg-[var(--color-kb-panel)]/5 mb-6 shadow-sm">
              
              {NOTES_ITEMS.map(note => {
                const isSelected = selectedIds.has(note.id);
                return (
                  <div 
                    key={note.id}
                    onClick={() => toggleSelect(note.id)}
                    className="p-3.5 flex items-start gap-3.5 cursor-pointer hover:bg-white dark:hover:bg-[var(--color-kb-panel-hover)] transition-colors group"
                  >
                    <div className="mt-0.5" onClick={(e) => e.stopPropagation()}>
                      <button onClick={() => toggleSelect(note.id)} className="text-zinc-300 dark:text-[var(--color-kb-text-muted)] hover:text-orange-500 transition-all active:scale-95">
                        {isSelected ? (
                          <CheckSquare size={18} strokeWidth={2.5} className="text-orange-500" />
                        ) : (
                          <Square size={18} strokeWidth={2.5} className="opacity-90 hover:opacity-100" />
                        )}
                      </button>
                    </div>
                    <div className="flex-1 min-w-0">
                      <div className={`text-[13.5px] tracking-tight transition-colors ${isSelected ? 'text-zinc-900 dark:text-[var(--color-kb-text-heading)] font-bold' : 'text-zinc-600 dark:text-[var(--color-kb-text-muted)] font-medium'}`}>{note.title}</div>
                      <div className="text-[11.5px] text-zinc-400 dark:text-[var(--color-kb-text-muted)] mt-1 font-mono font-medium">{note.updatedAt} · {note.sizeInfo}</div>
                    </div>
                  </div>
                );
              })}

            </div>

            <div className="flex bg-blue-50 dark:bg-blue-900/10 border border-blue-100 dark:border-blue-900/30 rounded-xl p-4 items-start gap-3 shadow-inner">
               <CheckCircle2 size={16} strokeWidth={2.5} className="text-blue-500 mt-0.5 shrink-0" />
               <div className="text-[12px] font-medium text-blue-800 dark:text-blue-300 leading-relaxed">
                 {t('importingNotesTip')}
               </div>
            </div>
         </div>

         <div className="px-6 py-4 flex justify-end gap-3 bg-[#fafafa] dark:bg-[var(--color-kb-panel)]/50 border-t border-zinc-200/80 dark:border-[var(--color-kb-panel-border)] shrink-0 z-10 shadow-[0_-4px_20px_rgba(0,0,0,0.02)]">
           <button onClick={onClose} className="px-5 py-2.5 text-[13px] font-bold text-zinc-600 dark:text-[var(--color-kb-text-heading)] bg-white dark:bg-[var(--color-kb-editor)] hover:bg-zinc-100 dark:hover:bg-[var(--color-kb-panel-hover)] border-2 border-transparent hover:border-zinc-200/80 dark:border-[var(--color-kb-panel-border)] rounded-xl transition-all shadow-sm active:scale-95">{t('cancel', { ns: 'common' })}</button>
           <button 
             onClick={handleImportClick} 
             disabled={selectedIds.size === 0}
             className="px-6 py-2.5 text-[13px] font-extrabold bg-orange-500 hover:bg-orange-600 disabled:opacity-40 disabled:grayscale text-white rounded-xl shadow-[0_4px_12px_rgba(249,115,22,0.2)] hover:shadow-lg transition-all disabled:shadow-none active:scale-95 focus:outline-none focus:ring-4 focus:ring-orange-500/20"
           >
             {t('importCount', { count: selectedIds.size })}
           </button>
         </div>

      </div>
    </div>,
    document.body
  );
}
