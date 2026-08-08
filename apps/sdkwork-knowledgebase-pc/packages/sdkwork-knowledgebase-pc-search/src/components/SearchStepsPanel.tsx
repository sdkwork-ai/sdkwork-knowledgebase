import { ChevronDown, CheckCircle2, Loader2 } from 'lucide-react';
import type { SearchMessage } from '../types';

interface SearchStepsPanelProps {
  message: SearchMessage;
  expanded: boolean;
  onToggle: () => void;
}

export function SearchStepsPanel({ message, expanded, onToggle }: SearchStepsPanelProps) {
  const steps = message.searchSteps;
  if (!steps?.length) return null;

  const stepsDone = steps.every((s) => s.status === 'success' || s.status === 'failed');
  const successCount = steps.filter((s) => s.status === 'success').length;

  return (
    <div className="rounded-lg border border-[var(--color-kb-panel-border)] overflow-hidden">
      <button
        type="button"
        onClick={onToggle}
        className="w-full flex items-center justify-between gap-2 px-3 py-2 text-left bg-[var(--color-kb-panel)]/40 hover:bg-[var(--color-kb-panel-hover)] transition-colors"
      >
        <span className="text-xs font-medium text-[var(--color-kb-text-muted)]">
          {message.isSearching ? '正在检索...' : stepsDone ? `检索完成 · ${successCount} 步` : '检索过程'}
        </span>
        <ChevronDown
          className={`w-3.5 h-3.5 text-[var(--color-kb-text-muted)] transition-transform ${expanded ? 'rotate-180' : ''}`}
        />
      </button>
      {expanded && (
        <div className="px-3 pb-3 pt-1 grid grid-cols-1 md:grid-cols-2 gap-1.5">
          {steps.map((step) => {
            const isIdle = step.status === 'idle';
            const isRun = step.status === 'running';
            const isDone = step.status === 'success';
            return (
              <div
                key={step.id}
                className={`flex items-center gap-2 px-2 py-1.5 rounded-md text-[11px] font-medium transition-all ${
                  isDone
                    ? 'text-emerald-600 dark:text-emerald-400 bg-emerald-500/5'
                    : isRun
                      ? 'text-[var(--color-kb-accent)] bg-[var(--color-kb-panel-active)]'
                      : 'text-[var(--color-kb-text-muted)] opacity-60'
                }`}
              >
                {isDone && <CheckCircle2 className="w-3.5 h-3.5 shrink-0 text-emerald-500" />}
                {isRun && <Loader2 className="w-3.5 h-3.5 shrink-0 text-[var(--color-kb-accent)] animate-spin" />}
                {isIdle && (
                  <span className="w-3.5 h-3.5 shrink-0 rounded-full border border-[var(--color-kb-panel-border)]" />
                )}
                <span className="truncate">{step.label}</span>
              </div>
            );
          })}
        </div>
      )}
    </div>
  );
}
