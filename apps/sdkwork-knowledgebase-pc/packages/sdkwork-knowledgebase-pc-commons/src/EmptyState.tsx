import { FileX, FolderOpen, Search, Plus } from 'lucide-react';
import { cn } from './utils';

export interface EmptyStateProps {
  title?: string;
  description?: string;
  icon?: 'folder' | 'file' | 'search' | 'default';
  actionLabel?: string;
  onAction?: () => void;
  className?: string;
}

const iconMap = {
  folder: FolderOpen,
  file: FileX,
  search: Search,
  default: FileX,
};

/**
 * Empty state component for displaying when no content is available.
 * Used for empty lists, no search results, or empty folders.
 */
export function EmptyState({
  title = 'No items found',
  description = 'There are no items to display.',
  icon = 'default',
  actionLabel,
  onAction,
  className,
}: EmptyStateProps) {
  const IconComponent = iconMap[icon];
  
  return (
    <div
      className={cn(
        'flex h-full min-h-[240px] flex-col items-center justify-center',
        'rounded-xl border border-[var(--color-kb-panel-border)]',
        'bg-[var(--color-kb-panel)] p-6 text-center',
        className
      )}
      role="status"
    >
      <IconComponent
        size={48}
        className="mb-4 text-[var(--color-kb-text-muted)] opacity-50"
        aria-hidden="true"
      />
      
      <h2 className="mb-2 text-lg font-semibold text-[var(--color-kb-text-heading)]">
        {title}
      </h2>
      
      <p className="mb-4 max-w-md text-sm text-[var(--color-kb-text-muted)]">
        {description}
      </p>
      
      {onAction && actionLabel ? (
        <button
          type="button"
          onClick={onAction}
          className={cn(
            'inline-flex items-center rounded-lg',
            'bg-[var(--color-kb-accent)] px-4 py-2',
            'text-sm font-medium text-white',
            'hover:opacity-90 transition-opacity'
          )}
        >
          <Plus size={16} className="mr-2" aria-hidden="true" />
          {actionLabel}
        </button>
      ) : null}
    </div>
  );
}

export interface NoSearchResultsProps {
  query?: string;
  onClear?: () => void;
  className?: string;
}

/**
 * Specialized empty state for search results.
 */
export function NoSearchResults({ query, onClear, className }: NoSearchResultsProps) {
  return (
    <EmptyState
      icon="search"
      title="No results found"
      description={
        query
          ? `No items match "${query}". Try a different search term.`
          : 'No items match your search. Try a different search term.'
      }
      actionLabel={onClear ? 'Clear search' : undefined}
      onAction={onClear}
      className={className}
    />
  );
}