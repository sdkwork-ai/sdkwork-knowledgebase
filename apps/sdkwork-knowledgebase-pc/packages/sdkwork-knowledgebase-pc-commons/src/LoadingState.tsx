import { cn } from './utils';

export interface LoadingStateProps {
  label?: string;
  size?: 'small' | 'medium' | 'large';
  className?: string;
  fullPage?: boolean;
}

const sizeClasses = {
  small: 'h-4 w-4',
  medium: 'h-6 w-6',
  large: 'h-8 w-8',
};

const containerSizeClasses = {
  small: 'min-h-[60px]',
  medium: 'min-h-[120px]',
  large: 'min-h-[240px]',
};

/**
 * Loading indicator component for displaying loading states.
 * Uses a spinner animation to indicate content is being loaded.
 */
export function LoadingState({
  label = 'Loading...',
  size = 'medium',
  className,
  fullPage = false,
}: LoadingStateProps) {
  const containerClasses = cn(
    'flex flex-col items-center justify-center',
    containerSizeClasses[size],
    fullPage && 'fixed inset-0 bg-[var(--color-kb-panel)] z-50',
    className
  );
  
  return (
    <div className={containerClasses} role="status" aria-live="polite">
      <div
        className={cn(
          'animate-spin rounded-full',
          'border-2 border-[var(--color-kb-border)]',
          'border-t-[var(--color-kb-accent)]',
          sizeClasses[size]
        )}
        aria-hidden="true"
      />
      <span className="mt-3 text-sm text-[var(--color-kb-text-muted)]">
        {label}
      </span>
    </div>
  );
}

export interface InlineLoadingProps {
  className?: string;
}

/**
 * Inline loading spinner for buttons or small areas.
 */
export function InlineLoading({ className }: InlineLoadingProps) {
  return (
    <div
      className={cn(
        'animate-spin rounded-full',
        'h-4 w-4',
        'border-2 border-current',
        'border-t-transparent',
        className
      )}
      role="status"
      aria-label="Loading"
    >
      <span className="sr-only">Loading...</span>
    </div>
  );
}