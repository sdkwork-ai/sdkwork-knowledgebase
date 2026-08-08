import { type ReactNode, useEffect } from 'react';
import { X } from 'lucide-react';

interface AdminDialogProps {
  children: ReactNode;
  onClose(): void;
  title: string;
}

export function AdminDialog({ children, onClose, title }: AdminDialogProps) {
  useEffect(() => {
    const closeOnEscape = (event: KeyboardEvent) => {
      if (event.key === 'Escape') onClose();
    };
    window.addEventListener('keydown', closeOnEscape);
    return () => window.removeEventListener('keydown', closeOnEscape);
  }, [onClose]);

  return (
    <div className="fixed inset-0 z-50 flex items-center justify-center bg-black/45 p-4" role="presentation">
      <section
        aria-modal="true"
        className="w-full max-w-lg border border-[var(--color-kb-panel-border)] bg-[var(--color-kb-panel)] shadow-2xl"
        role="dialog"
      >
        <header className="flex items-center justify-between border-b border-[var(--color-kb-panel-border)] px-5 py-4">
          <h2 className="text-base font-semibold text-[var(--color-kb-text-heading)]">{title}</h2>
          <button aria-label="Close" className="p-2 hover:bg-[var(--color-kb-panel-hover)]" onClick={onClose} type="button">
            <X size={18} />
          </button>
        </header>
        <div className="max-h-[75vh] overflow-y-auto p-5">{children}</div>
      </section>
    </div>
  );
}
