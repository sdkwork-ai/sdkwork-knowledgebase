import { AlertTriangle, Check, CloudOff, Loader2 } from 'lucide-react';

import type { NotesSaveStatus } from '../hooks/notesAutosaveEngine';
import { formatNoteSavedTime } from '../utils/noteSavedTimeFormatter';

/** Localized copy for every save state shown by {@link SaveStatusPill}. */
export interface SaveStatusPillLabels {
  saving: string;
  saved: string;
  /** Saved-state copy including the confirmation time, e.g. "已保存 21:04". */
  savedAt: (time: string) => string;
  unsaved: string;
  failed: string;
  retry: string;
  conflict: string;
  overwrite: string;
}

export interface SaveStatusPillProps {
  status: NotesSaveStatus;
  /** Epoch milliseconds of the last confirmed save; null before the first. */
  lastSavedAt: number | null;
  locale: string;
  labels: SaveStatusPillLabels;
  /**
   * Test id prefix: the pill renders `${testIdPrefix}-save-status`, its retry
   * action `${testIdPrefix}-retry`, and the conflict action
   * `${testIdPrefix}-overwrite`.
   */
  testIdPrefix: string;
  onRetry: () => void;
  onOverwrite: () => void;
}

/**
 * Shared save-state indicator for every document editor (knowledge editor and
 * notes workspace). Quiet "已保存 <time>" when healthy; explicit progress and
 * recovery actions otherwise, so persistence is always observable without a
 * toast per keystroke. Recovery actions live in the pill because they only
 * apply to the state that produced them (retry a failed save, overwrite a
 * conflicted one).
 */
export function SaveStatusPill(props: SaveStatusPillProps) {
  const { status, lastSavedAt, locale, labels, testIdPrefix, onRetry, onOverwrite } = props;
  if (status === 'idle') {
    return null;
  }

  const statusId = `${testIdPrefix}-save-status`;
  const baseClass =
    'flex items-center gap-1.5 text-[11.5px] leading-none text-[var(--color-kb-text-muted)] whitespace-nowrap';
  const actionClass =
    'ml-0.5 px-2 py-1 rounded-md border text-[11px] font-medium transition-colors focus:outline-none focus-visible:ring-2 focus-visible:ring-[var(--color-kb-accent)]';

  if (status === 'saving') {
    return (
      <span className={baseClass} role="status" aria-live="polite" data-testid={statusId}>
        <Loader2 size={12} className="animate-spin" />
        {labels.saving}
      </span>
    );
  }

  if (status === 'saved') {
    const timeLabel = lastSavedAt === null
      ? null
      : formatNoteSavedTime(lastSavedAt, locale || 'zh-CN');
    return (
      <span className={baseClass} role="status" aria-live="polite" data-testid={statusId}>
        <Check size={12} className="text-emerald-500" />
        {timeLabel ? labels.savedAt(timeLabel) : labels.saved}
      </span>
    );
  }

  if (status === 'conflict') {
    return (
      <span
        className="flex items-center gap-1.5 text-[11.5px] leading-none whitespace-nowrap text-amber-600"
        role="status"
        aria-live="polite"
        data-testid={statusId}
      >
        <AlertTriangle size={12} />
        {labels.conflict}
        <button
          type="button"
          onClick={onOverwrite}
          className={`${actionClass} border-amber-500/50 text-amber-600 hover:bg-amber-500/10`}
          data-testid={`${testIdPrefix}-overwrite`}
        >
          {labels.overwrite}
        </button>
      </span>
    );
  }

  if (status === 'error') {
    return (
      <span
        className="flex items-center gap-1.5 text-[11.5px] leading-none whitespace-nowrap text-rose-500"
        role="status"
        aria-live="polite"
        data-testid={statusId}
      >
        <CloudOff size={12} />
        {labels.failed}
        <button
          type="button"
          onClick={onRetry}
          className={`${actionClass} border-rose-400/50 text-rose-500 hover:bg-rose-500/10`}
          data-testid={`${testIdPrefix}-retry`}
        >
          {labels.retry}
        </button>
      </span>
    );
  }

  // dirty: local edits are waiting for the debounce window or a manual save.
  return (
    <span className={baseClass} role="status" aria-live="polite" data-testid={statusId}>
      <span className="inline-block w-[5px] h-[5px] rounded-full bg-[var(--color-kb-text-muted)]/60" />
      {labels.unsaved}
    </span>
  );
}
