import React, { useState, useEffect } from 'react';
import { isBlank } from '@sdkwork/utils';
import { X, Type } from 'lucide-react';
import { useTranslation } from 'react-i18next';

export interface RenameModalProps {
  initialTitle: string;
  onClose: () => void;
  onConfirm: (newTitle: string) => void;
}

export function RenameModal({ initialTitle, onClose, onConfirm }: RenameModalProps) {
  const { t } = useTranslation(['common']);
  const [title, setTitle] = useState(initialTitle);

  useEffect(() => {
    setTitle(initialTitle);
  }, [initialTitle]);

  const handleSubmit = (e: React.FormEvent) => {
    e.preventDefault();
    if (title.trim()) {
      onConfirm(title.trim());
    }
  };

  return (
    <div className="fixed inset-0 z-[200] bg-zinc-950/40 backdrop-blur-md flex items-center justify-center p-4">
      <div className="bg-white dark:bg-zinc-900 w-full max-w-sm rounded-2xl shadow-2xl border border-zinc-200 dark:border-zinc-800/80 overflow-hidden animate-in fade-in zoom-in-95 duration-200">
        {/* Header */}
        <div className="flex items-center justify-between px-5 py-4 border-b border-zinc-100 dark:border-zinc-800/80">
          <div className="flex items-center space-x-2">
            <span className="p-1.5 bg-indigo-50 dark:bg-indigo-950/40 text-indigo-600 dark:text-indigo-400 rounded-lg shrink-0">
              <Type size={15} />
            </span>
            <h3 className="font-semibold text-sm text-zinc-900 dark:text-zinc-100 tracking-tight">{t('rename')}</h3>
          </div>
          <button onClick={onClose} className="text-zinc-400 hover:text-zinc-600 dark:text-zinc-500 dark:hover:text-zinc-300 p-1 rounded-lg hover:bg-zinc-100 dark:hover:bg-zinc-800 transition-all active:scale-95">
            <X size={15} strokeWidth={2.5} />
          </button>
        </div>

        {/* Form Body */}
        <form onSubmit={handleSubmit}>
          <div className="px-5 py-5 space-y-3">
            <div className="relative">
              <input
                type="text"
                autoFocus
                value={title}
                onChange={(e) => setTitle(e.target.value)}
                className="w-full bg-zinc-50/50 dark:bg-zinc-950/40 border border-zinc-200 dark:border-zinc-800/80 rounded-xl px-3.5 py-2.5 text-[13px] font-medium text-zinc-900 dark:text-zinc-100 focus:outline-none focus:ring-2 focus:ring-indigo-500/10 focus:border-indigo-500 dark:focus:ring-indigo-500/20 dark:focus:border-indigo-400 transition-all placeholder-zinc-400 dark:placeholder-zinc-600"
                placeholder={t('enterName')}
              />
            </div>
            <p className="text-[11px] text-zinc-400 dark:text-zinc-500 leading-normal pl-0.5">
              支持中英文、数字组合，长度建议在 30 个字符以内。
            </p>
          </div>

          {/* Footer Actions */}
          <div className="px-5 py-3.5 border-t border-zinc-100 dark:border-zinc-800/80 bg-zinc-50/50 dark:bg-zinc-950/10 flex justify-end items-center space-x-2">
            <button 
              type="button" 
              onClick={onClose} 
              className="px-4 py-2 text-[12px] font-semibold text-zinc-600 hover:text-zinc-800 dark:text-zinc-400 dark:hover:text-zinc-200 hover:bg-zinc-100 dark:hover:bg-zinc-800/60 rounded-xl transition-all active:scale-95"
            >
              {t('cancel')}
            </button>
            <button 
              type="submit" 
              disabled={isBlank(title) || title === initialTitle} 
              className="px-4.5 py-2 text-[12px] font-semibold bg-indigo-600 hover:bg-indigo-700 text-white dark:bg-indigo-500 dark:hover:bg-indigo-600 rounded-xl disabled:opacity-40 disabled:pointer-events-none transition-all shadow-md active:scale-95"
            >
              {t('confirm')}
            </button>
          </div>
        </form>
      </div>
    </div>
  );
}
