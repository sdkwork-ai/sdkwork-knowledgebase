import { Component, type ErrorInfo, type ReactNode } from 'react';
import { AlertTriangle, RefreshCw } from 'lucide-react';

export interface FeatureErrorBoundaryProps {
  children: ReactNode;
  title?: string;
  description?: string;
  onReset?: () => void;
}

interface FeatureErrorBoundaryState {
  hasError: boolean;
  error: Error | null;
}

export class FeatureErrorBoundary extends Component<
  FeatureErrorBoundaryProps,
  FeatureErrorBoundaryState
> {
  constructor(props: FeatureErrorBoundaryProps) {
    super(props);
    this.state = { hasError: false, error: null };
  }

  public static getDerivedStateFromError(error: Error): FeatureErrorBoundaryState {
    return { hasError: true, error };
  }

  public componentDidCatch(error: Error, errorInfo: ErrorInfo) {
    console.error('Feature module error:', error, errorInfo);
  }

  private handleReset = () => {
    this.setState({ hasError: false, error: null });
    this.props.onReset?.();
  };

  public render() {
    if (this.state.hasError) {
      return (
        <div className="flex h-full min-h-[240px] flex-col items-center justify-center rounded-xl border border-[var(--color-kb-panel-border)] bg-[var(--color-kb-panel)] p-6 text-center">
          <AlertTriangle size={40} className="mb-4 text-red-500" />
          <h2 className="mb-2 text-lg font-semibold text-[var(--color-kb-text-heading)]">
            {this.props.title ?? 'This section failed to load'}
          </h2>
          <p className="mb-4 max-w-md text-sm text-[var(--color-kb-text-muted)]">
            {this.props.description ??
              'An unexpected error occurred in this module. You can retry without reloading the whole application.'}
          </p>
          {import.meta.env?.DEV && this.state.error?.message ? (
            <pre className="mb-4 max-h-28 w-full max-w-xl overflow-auto rounded border border-[var(--color-kb-panel-border)] bg-[var(--color-kb-panel-hover)] p-3 text-left text-xs text-red-600">
              {this.state.error.message}
            </pre>
          ) : null}
          <button
            type="button"
            onClick={this.handleReset}
            className="inline-flex items-center rounded-lg bg-[var(--color-kb-accent)] px-4 py-2 text-sm font-medium text-white hover:opacity-90"
          >
            <RefreshCw size={16} className="mr-2" />
            Retry
          </button>
        </div>
      );
    }

    return this.props.children;
  }
}
