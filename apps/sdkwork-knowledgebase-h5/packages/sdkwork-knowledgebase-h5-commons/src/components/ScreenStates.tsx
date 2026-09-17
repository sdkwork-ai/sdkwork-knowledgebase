import type { ReactNode } from "react";

import { cn } from "../utils";

export interface ScreenStateProps {
  action?: ReactNode;
  className?: string;
  description?: string;
  title: string;
}

/** Domain-neutral empty/error state primitive. */
export function ScreenState({ action, className, description, title }: ScreenStateProps) {
  return (
    <div className={cn("flex flex-col items-center gap-2 px-6 py-12 text-center", className)} role="status">
      <p className="text-base font-medium">{title}</p>
      {description ? <p className="text-sm opacity-70">{description}</p> : null}
      {action}
    </div>
  );
}

export function LoadingState({ label, className }: { label: string; className?: string }) {
  return (
    <div className={cn("flex items-center justify-center gap-2 px-6 py-10", className)} role="status">
      <span className="size-4 animate-spin rounded-full border-2 border-current border-t-transparent" />
      <span className="text-sm">{label}</span>
    </div>
  );
}
