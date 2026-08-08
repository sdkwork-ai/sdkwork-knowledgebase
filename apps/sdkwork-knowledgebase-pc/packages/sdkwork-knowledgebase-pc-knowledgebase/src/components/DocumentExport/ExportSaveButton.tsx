import { useEffect, useState } from 'react';
import { ChevronDown, FileDown } from 'lucide-react';
import { DropdownItem } from '../InsertToolsMenu';
import { toast } from '../ui/toast-manager';
import { getDocumentExportCapabilities } from './documentExportCapabilities';
import {
  buildExportFileName,
  EXPORT_MIME_TYPES,
  persistExportFile,
} from './documentExportSave';
import { dismissExportProgress, showExportProgress } from './exportProgress';
import { notifyExportCancelled, notifyExportSaveResult } from './exportSaveNotify';
import { isDesktopExportHost } from './exportRuntime';
import type { ExportSaveMode } from './types';

export interface ExportSaveButtonProps {
  title: string;
  extension: string;
  getBytes: () => Uint8Array | null;
  mimeType?: string;
  disabled?: boolean;
  primaryLabel?: string;
  className?: string;
  dropdownClassName?: string;
}

export function ExportSaveButton({
  title,
  extension,
  getBytes,
  mimeType = EXPORT_MIME_TYPES.pdf,
  disabled = false,
  primaryLabel = '下载',
  className = '',
  dropdownClassName = 'relative export-save-dropdown',
}: ExportSaveButtonProps) {
  const [isOpen, setIsOpen] = useState(false);
  const [isSaving, setIsSaving] = useState(false);
  const saveAsAvailable = getDocumentExportCapabilities().saveAsAvailable;

  useEffect(() => {
    const handleOutsideClick = (event: MouseEvent) => {
      const target = event.target as HTMLElement;
      if (!target.closest('.export-save-dropdown')) {
        setIsOpen(false);
      }
    };
    document.addEventListener('click', handleOutsideClick);
    return () => document.removeEventListener('click', handleOutsideClick);
  }, []);

  useEffect(() => {
    if (!isOpen) {
      return;
    }
    const handleKeyDown = (event: KeyboardEvent) => {
      if (event.key === 'Escape') {
        setIsOpen(false);
      }
    };
    document.addEventListener('keydown', handleKeyDown);
    return () => document.removeEventListener('keydown', handleKeyDown);
  }, [isOpen]);

  const handleSave = async (mode: ExportSaveMode) => {
    const bytes = getBytes();
    if (!bytes || bytes.length === 0) {
      toast.error('没有可保存的内容');
      return;
    }

    setIsOpen(false);
    setIsSaving(true);
    if (mode === 'saveAs') {
      showExportProgress('请选择保存位置...');
    } else {
      showExportProgress('正在保存...');
    }

    try {
      const result = await persistExportFile({
        bytes,
        suggestedName: buildExportFileName(title, extension),
        mode,
        mimeType,
      });

      if (result.cancelled) {
        notifyExportCancelled();
        return;
      }
      if (!result.saved) {
        dismissExportProgress();
        toast.error(isDesktopExportHost() ? '桌面端保存失败，请检查权限或重试' : '保存失败');
        return;
      }
      void notifyExportSaveResult(result);
    } catch (error: unknown) {
      dismissExportProgress();
      const message = error instanceof Error ? error.message : String(error);
      toast.error(`保存失败: ${message}`);
    } finally {
      setIsSaving(false);
    }
  };

  return (
    <div className={`${dropdownClassName} flex items-center`}>
      <button
        type="button"
        disabled={disabled || isSaving}
        onClick={() => void handleSave('downloads')}
        className={`text-[11px] px-2.5 py-1 bg-indigo-50 dark:bg-[var(--color-kb-accent)]/10 text-indigo-600 dark:text-[var(--color-kb-accent)] rounded-l-lg hover:bg-indigo-100 dark:hover:bg-[var(--color-kb-accent)]/20 transition-all font-bold border border-indigo-200 dark:border-[var(--color-kb-accent)]/20 border-r-0 active:scale-95 disabled:opacity-50 disabled:cursor-not-allowed ${className}`}
      >
        {isSaving ? '保存中...' : primaryLabel}
      </button>
      <button
        type="button"
        disabled={disabled || isSaving || !saveAsAvailable}
        onClick={(event) => {
          event.stopPropagation();
          if (!saveAsAvailable) {
            return;
          }
          setIsOpen((open) => !open);
        }}
        className="text-[11px] px-1.5 py-1 bg-indigo-50 dark:bg-[var(--color-kb-accent)]/10 text-indigo-600 dark:text-[var(--color-kb-accent)] rounded-r-lg hover:bg-indigo-100 dark:hover:bg-[var(--color-kb-accent)]/20 transition-all font-bold border border-indigo-200 dark:border-[var(--color-kb-accent)]/20 active:scale-95 disabled:opacity-50 disabled:cursor-not-allowed"
        title={saveAsAvailable ? '更多保存选项' : '当前环境不支持另存为'}
      >
        <ChevronDown size={12} className={isOpen ? 'rotate-180 transition-transform' : 'transition-transform'} />
      </button>

      {isOpen && saveAsAvailable && (
        <div className="absolute right-0 top-[calc(100%+4px)] bg-white dark:bg-[var(--color-kb-editor)] border border-zinc-200/80 dark:border-[var(--color-kb-panel-border)] shadow-[0_4px_16px_rgba(0,0,0,0.08)] dark:shadow-[0_4px_24px_rgba(0,0,0,0.4)] rounded-xl py-1 min-w-[148px] z-[310] animate-in fade-in slide-in-from-top-2 duration-150">
          <DropdownItem
            text="另存为..."
            disabled={isSaving}
            icon={<FileDown size={14} className="opacity-70" />}
            onClick={() => void handleSave('saveAs')}
          />
        </div>
      )}
    </div>
  );
}
