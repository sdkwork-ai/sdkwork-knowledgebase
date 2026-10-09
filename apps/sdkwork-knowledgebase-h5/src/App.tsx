import { Component, Suspense, lazy, useMemo, type ReactNode } from "react";
import { HashRouter, Navigate, Route, Routes } from "react-router-dom";

import { AuthGate } from "./components/AuthGate";
import { createH5RouteContributions } from "./bootstrap/routes";

const KnowledgebaseAppShell = lazy(() =>
  import("@sdkwork/knowledgebase-h5-shell").then((module) => ({ default: module.KnowledgebaseAppShell })),
);

interface H5RootErrorBoundaryState {
  error: unknown;
}

/**
 * Root error boundary for the H5 tree: a render exception or a failed lazy
 * chunk download (classic stale-asset problem after a redeploy) must surface
 * a reload affordance instead of unmounting to a permanent white screen on
 * mobile web.
 */
class H5RootErrorBoundary extends Component<{ children: ReactNode }, H5RootErrorBoundaryState> {
  state: H5RootErrorBoundaryState = { error: null };

  static getDerivedStateFromError(error: unknown): H5RootErrorBoundaryState {
    return { error };
  }

  render() {
    if (this.state.error) {
      return (
        <div
          role="alert"
          className="flex min-h-screen flex-col items-center justify-center gap-3 px-6 text-center"
        >
          <p className="text-sm font-semibold">页面出现异常</p>
          <button
            type="button"
            onClick={() => window.location.reload()}
            className="rounded-lg border border-zinc-300 px-4 py-2 text-sm font-medium"
          >
            重新加载
          </button>
        </div>
      );
    }
    return this.props.children;
  }
}

export default function App() {
  // Stable identity: the shell keys memo/effects off the contributions array,
  // so it must not be rebuilt on every render.
  const contributions = useMemo(createH5RouteContributions, []);

  return (
    <H5RootErrorBoundary>
      <AuthGate>
        <HashRouter>
          <Suspense fallback={<div role="status">Loading SDKWork Knowledgebase</div>}>
            <Routes>
              <Route path="/" element={<KnowledgebaseAppShell contributions={contributions} />} />
              <Route path="*" element={<Navigate to="/" replace />} />
            </Routes>
          </Suspense>
        </HashRouter>
      </AuthGate>
    </H5RootErrorBoundary>
  );
}
