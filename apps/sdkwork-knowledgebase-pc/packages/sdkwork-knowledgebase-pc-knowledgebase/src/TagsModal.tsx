import { useState } from 'react';

export interface TagsModalProps {
  isOpen: boolean;
  item: { title: string; author?: string } | null;
  onClose: () => void;
  onSave: (tags: string) => void;
}

export function TagsModal({ isOpen, item, onClose, onSave }: TagsModalProps) {
  const [tagInput, setTagInput] = useState('');

  if (!isOpen || !item) return null;

  return (
    <div className="fixed inset-0 bg-zinc-950/40 z-[1000] flex items-center justify-center backdrop-blur-md">
      <div className="bg-white dark:bg-[var(--color-kb-editor)] w-[400px] rounded-2xl shadow-2xl border border-zinc-200/80 dark:border-[var(--color-kb-panel-border)] p-6 animate-in fade-in zoom-in-95 duration-200">
        <h3 className="text-[15px] font-extrabold text-zinc-900 dark:text-[var(--color-kb-text-heading)] mb-4">编辑标签</h3>
        <div className="mb-5">
          <label className="text-[13px] font-medium text-zinc-500 dark:text-[var(--color-kb-text-muted)] mb-2 block">
            为 {item.title} 添加标签（逗号分隔）
          </label>
          <input 
            type="text" 
            value={tagInput}
            onChange={(e) => setTagInput(e.target.value)}
            className="w-full bg-[#fafafa] dark:bg-[var(--color-kb-input-bg)] border-2 border-zinc-200/80 dark:border-[var(--color-kb-panel-border)] rounded-xl px-4 py-2.5 text-[14px] font-bold outline-none text-zinc-800 dark:text-[var(--color-kb-text)] focus:border-indigo-400 focus:bg-white transition-all shadow-sm" 
            placeholder="例如: 重要, 项目A" 
          />
        </div>
        <div className="flex justify-end gap-3">
          <button 
            type="button" 
            onClick={onClose} 
            className="px-4 py-2 text-[13px] font-bold text-zinc-600 dark:text-[var(--color-kb-text)] hover:bg-zinc-100 hover:text-zinc-900 dark:hover:bg-[var(--color-kb-panel-hover)] rounded-xl transition-all active:scale-95 border-2 border-transparent hover:border-zinc-200/80 dark:hover:border-transparent"
          >
            取消
          </button>
          <button 
            type="button" 
            onClick={() => onSave(tagInput)} 
            className="px-6 py-2 text-[13px] font-extrabold bg-[#07C160] text-white rounded-xl shadow-md hover:bg-[#06ad56] transition-all active:scale-95 shadow-[#07C160]/20"
          >
            保存
          </button>
        </div>
      </div>
    </div>
  );
}
