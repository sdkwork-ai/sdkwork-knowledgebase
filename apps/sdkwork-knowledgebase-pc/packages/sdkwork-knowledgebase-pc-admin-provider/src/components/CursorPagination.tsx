import { ChevronLeft, ChevronRight } from 'lucide-react';

interface CursorPaginationProps {
  canNext: boolean;
  canPrevious: boolean;
  nextLabel: string;
  onNext(): void;
  onPrevious(): void;
  previousLabel: string;
}

export function CursorPagination(props: CursorPaginationProps) {
  return (
    <div className="flex justify-end gap-2 border-t border-[var(--color-kb-panel-border)] px-4 py-3">
      <button className="inline-flex items-center gap-1 px-3 py-2 text-sm disabled:opacity-40" disabled={!props.canPrevious} onClick={props.onPrevious} type="button">
        <ChevronLeft size={15} /> {props.previousLabel}
      </button>
      <button className="inline-flex items-center gap-1 px-3 py-2 text-sm disabled:opacity-40" disabled={!props.canNext} onClick={props.onNext} type="button">
        {props.nextLabel} <ChevronRight size={15} />
      </button>
    </div>
  );
}
