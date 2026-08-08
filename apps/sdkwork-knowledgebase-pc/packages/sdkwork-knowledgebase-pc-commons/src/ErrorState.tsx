import { AlertTriangle, RefreshCw } from 'lucide-react';
import { cn } from './utils';

export interface ErrorStateProps {
  title?: string;
  description?: string;
  error?: Error | null;
  onRetry?: () => void;
  retryLabel?: string;
  className?: string;
  showDetails?: boolean;
}

/**
 * Error state component for displaying error conditions with retry capability.
 * Used when API calls fail or unexpected errors occur in feature modules.
 */
export function ErrorState({
  title = 'Something went wrong',
  description = 'An unexpected error occurred. Please try again.',
  error,
  onRetry,
  retryLabel = 'Retry',
  className,
  showDetails = import.meta.env?.DEV,
}: ErrorStateProps) {
  return (
    <div
      className={cn(
        'flex h-full min-h-[240px] flex-col items-center justify-center',
        'rounded-xl border border-[var(--color-kb-panel-border)]',
        'bg-[var(--color-kb-panel)] p-6 text-center',
        className
      )}
      role="alert"
    >
      <AlertTriangle
        size={40}
        className="mb-4 text-red-500"
        aria-hidden="true"
      />
      
      <h2 className="mb-2 text-lg font-semibold text-[var(--color-kb-text-heading)]">
        {title}
      </h2>
      
      <p className="mb-4 max-w-md text-sm text-[var(--color-kb-text-muted)]">
        {description}
      </p>
      
      {showDetails && error?.message ? (
        <pre
          className="mb-4 max-h-28 w-full max-w-xl overflow-auto rounded"
          style={{
            border: '1px solid var(--color-kb-panel-border)',
            background: 'var(--color-kb-panel-hover)',
            padding: '12px',
            textAlign: 'left',
            fontSize: '12px',
            color: '#dc2626',
          }}
        >
          {error.message}
        </pre>
      ) : null}
      
      {onRetry ? (
        <button
          type="button"
          onClick={onRetry}
          className={cn(
            'inline-flex items-center rounded-lg',
            'bg-[var(--color-kb-accent)] px-4 py-2',
            'text-sm font-medium text-white',
            'hover:opacity-90 transition-opacity'
          )}
        >
          <RefreshCw size={16} className="mr-2" aria-hidden="true" />
          {retryLabel}
        </button>
      ) : null}
    </div>
  );
}

export interface InlineErrorProps {
  message: string;
  onRetry?: () => void;
  className?: string;
}

/**
 * Inline error component for smaller error states within lists or panels.
 */
export function InlineError({ message, onRetry, className }: InlineErrorProps) {
  return (
    <div
      className={cn(
        'flex items-center gap-2 px-3 py-2 rounded-lg',
        'bg-red-50 text-red-600 text-sm',
        className
      )}
      role="alert"
    >
      <AlertTriangle size={16} aria-hidden="true" />
      <span className="flex-1">{message}</span>
      {onRetry ? (
        <button
          type="button"
          onClick={onRetry}
          className="inline-flex items-center text-red-700 hover:text-red-800"
        >
          <RefreshCw size={14} className="mr-1" aria-hidden="true" />
          Retry
        </button>
      ) : null}
    </div>
  );
}