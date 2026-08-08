import { ChevronDown, ChevronUp, CircleAlert, Cpu } from 'lucide-react';
import { useTranslation } from 'react-i18next';

export interface McpConsolePanelProps {
  isOpen: boolean;
  onToggle: () => void;
  isTyping: boolean;
  onTriggerQuickTool: (command: string) => void;
}

export function McpConsolePanel({
  isOpen,
  onToggle,
}: McpConsolePanelProps) {
  const { t } = useTranslation(['mcp', 'errors']);

  return (
    <div className="overflow-hidden rounded-xl border border-[var(--color-kb-panel-border)] bg-[var(--color-kb-panel)] shadow-xs">
      <button
        type="button"
        onClick={onToggle}
        aria-expanded={isOpen}
        className="flex w-full items-center justify-between border-b border-[var(--color-kb-panel-border)] bg-[var(--color-kb-panel-hover)] px-3.5 py-3 text-left transition-colors hover:bg-[var(--color-kb-panel)]"
      >
        <span className="flex items-center gap-2 text-[12px] font-bold text-[var(--color-kb-text-heading)]">
          <Cpu size={13} className="text-[var(--color-kb-text-muted)]" />
          {t('mcpConsoleTitle')}
        </span>
        {isOpen ? (
          <ChevronUp size={14} className="text-[var(--color-kb-text-muted)]" />
        ) : (
          <ChevronDown size={14} className="text-[var(--color-kb-text-muted)]" />
        )}
      </button>

      {isOpen && (
        <div
          role="alert"
          className="flex items-start gap-2.5 px-4 py-3.5 text-xs leading-5 text-[var(--color-kb-text-muted)]"
        >
          <CircleAlert size={15} className="mt-0.5 shrink-0 text-amber-500" />
          <span>{t('errors:api.unavailable.sdk')}</span>
        </div>
      )}
    </div>
  );
}
