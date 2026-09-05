import React, { Suspense, useMemo, useRef, type ErrorInfo, type ReactNode } from 'react';
import { BrowserRouter, Route, Routes, useLocation, useNavigate } from 'react-router-dom';
import { AlertTriangle, RefreshCw } from 'lucide-react';
import { SdkworkSessionAuthBrowserRoot } from '@sdkwork/auth-pc-react';
import {
  KnowledgebaseAuthGate,
  KnowledgebaseErrorCodes,
  KnowledgebaseRuntimeProvider,
  captureGroupKnowledgebaseLaunchTicket,
  GROUP_KNOWLEDGEBASE_LAUNCH_PATH,
  sanitizeGroupKnowledgebaseLaunchAuthLocation,
  throwKnowledgebaseError,
} from 'sdkwork-knowledgebase-pc-core';

import { createKnowledgebasePcRuntime } from './bootstrap/createKnowledgebasePcRuntime';
import i18n from '@sdkwork/knowledgebase-pc-knowledge/i18n';
import type { KnowledgebaseIamRuntime } from './bootstrap/knowledgebaseIamRuntime';
import {
  resolveKnowledgebaseAuthAppearance,
  resolveKnowledgebaseAuthLocale,
  resolveKnowledgebaseAuthRuntimeConfig,
} from './bootstrap/knowledgebaseAuthConfig';
import { KnowledgebaseAuthShell } from './components/KnowledgebaseAuthShell';
import { GroupKnowledgebaseDesktopLaunchBridge } from './components/GroupKnowledgebaseDesktopLaunchBridge';

const AppShell = React.lazy(() =>
  import('@sdkwork/sdkwork-knowledgebase-pc-shell').then((module) => ({
    default: module.AppShell,
  })),
);

const KnowledgebaseAdminConsole = React.lazy(() =>
  import('@sdkwork/sdkwork-knowledgebase-pc-shell').then((module) => ({
    default: module.KnowledgebaseAdminConsole,
  })),
);

const WechatPublishPage = React.lazy(() =>
  import('@sdkwork/sdkwork-knowledgebase-pc-knowledgebase/WechatPublishPage').then((module) => ({
    default: module.WechatPublishPage,
  })),
);

const ProviderAdminPage = React.lazy(() =>
  import('sdkwork-knowledgebase-pc-admin-provider').then((module) => ({
    default: module.ProviderAdminPage,
  })),
);

const GroupKnowledgebaseLaunchPage = React.lazy(() =>
  import('@sdkwork/knowledgebase-pc-knowledge').then((module) => ({
    default: module.GroupKnowledgebaseLaunchPage,
  })),
);

const SdkworkIamAuthRoutes = React.lazy(() =>
  import('@sdkwork/auth-pc-react').then((module) => ({ default: module.SdkworkIamAuthRoutes })),
);

interface ErrorBoundaryProps {
  children: ReactNode;
}

interface ErrorBoundaryState {
  hasError: boolean;
  error: Error | null;
}

class ErrorBoundary extends React.Component<ErrorBoundaryProps, ErrorBoundaryState> {
  declare readonly props: Readonly<ErrorBoundaryProps>;

  public state: ErrorBoundaryState = {
    hasError: false,
    error: null,
  };

  public static getDerivedStateFromError(error: Error): ErrorBoundaryState {
    return { hasError: true, error };
  }

  public componentDidCatch(error: Error, errorInfo: ErrorInfo) {
    console.error('Uncaught error:', error, errorInfo);
  }

  public render() {
    if (this.state.hasError) {
      const showDiagnosticDetails =
        import.meta.env.DEV || import.meta.env.MODE === 'playwright';
      return (
        <div className="w-screen h-screen flex flex-col items-center justify-center bg-gray-50 dark:bg-[var(--color-kb-editor)] text-gray-800 dark:text-[var(--color-kb-text)] p-6">
          <AlertTriangle size={64} className="text-red-500 mb-6" />
          <h1 className="text-2xl font-bold mb-2">{i18n.t('errors:root.title')}</h1>
          <p className="text-gray-600 dark:text-[var(--color-kb-text-muted)] mb-6 text-center max-w-md">
            {i18n.t('errors:root.description')}
          </p>
          {showDiagnosticDetails ? (
            <div className="bg-white dark:bg-zinc-900 border dark:border-[var(--color-kb-panel-border)] rounded shadow-sm p-4 w-full max-w-2xl overflow-auto mb-8">
              <pre className="text-xs text-red-600 font-mono">
                {this.state.error?.message}
              </pre>
            </div>
          ) : null}
          <button
            onClick={() => window.location.reload()}
            className="flex items-center px-6 py-3 bg-blue-600 text-white rounded-lg hover:bg-blue-700 transition-colors shadow"
          >
            <RefreshCw size={18} className="mr-2" />
            {i18n.t('errors:root.reload')}
          </button>
        </div>
      );
    }

    return this.props.children;
  }
}

export default function App() {
  const runtime = useMemo(() => createKnowledgebasePcRuntime(), []);

  return (
    <ErrorBoundary>
      <KnowledgebaseRuntimeProvider runtime={runtime}>
        <BrowserRouter basename={runtime.config.browserBasePath}>
          <SdkworkSessionAuthBrowserRoot>
            <KnowledgebaseAppRoutes runtime={runtime} />
          </SdkworkSessionAuthBrowserRoot>
        </BrowserRouter>
      </KnowledgebaseRuntimeProvider>
    </ErrorBoundary>
  );
}

function KnowledgebaseAppRoutes({
  runtime,
}: {
  runtime: ReturnType<typeof createKnowledgebasePcRuntime>;
}) {
  const location = useLocation();
  const navigate = useNavigate();

  const getIamRuntime = useMemo(() => {
    return () => getKnowledgebaseIamRuntime(runtime);
  }, [runtime]);

  const authRoutes = useMemo(() => (
    <KnowledgebaseAuthShell>
      <KnowledgebaseAppbaseAuthRouteHost getRuntime={getIamRuntime} />
    </KnowledgebaseAuthShell>
  ), [getIamRuntime]);

  const capturedGroupLaunchHashRef = useRef<string | null>(null);
  const groupLaunchLocationKey = `${location.pathname}${location.search}${location.hash ?? ''}`;
  const hasCapturedGroupLaunchTicket = capturedGroupLaunchHashRef.current === groupLaunchLocationKey;
  const isGroupKnowledgebaseLaunch = location.pathname === GROUP_KNOWLEDGEBASE_LAUNCH_PATH;
  if (isGroupKnowledgebaseLaunch && !hasCapturedGroupLaunchTicket) {
    captureGroupKnowledgebaseLaunchTicket(location);
    capturedGroupLaunchHashRef.current = groupLaunchLocationKey;
  }
  const authGateLocation = sanitizeGroupKnowledgebaseLaunchAuthLocation(location);

  return (
    <>
      <GroupKnowledgebaseDesktopLaunchBridge />
      <KnowledgebaseAuthGate
        authRoutes={authRoutes}
        location={authGateLocation}
        navigate={(to, options) => navigate(to, options)}
        session={runtime.session}
      >
        <Suspense fallback={<KnowledgebaseRouteFallback label="Loading Knowledgebase workspace" />}>
          <Routes>
            <Route path="/" element={<AppShell />} />
            <Route path="/admin" element={<KnowledgebaseAdminConsole />} />
            <Route path="/admin/providers" element={<ProviderAdminPage />} />
            <Route path="/wechat-publish" element={<WechatPublishPage />} />
            <Route path={GROUP_KNOWLEDGEBASE_LAUNCH_PATH} element={<GroupKnowledgebaseLaunchPage />} />
          </Routes>
        </Suspense>
      </KnowledgebaseAuthGate>
    </>
  );
}

function KnowledgebaseAppbaseAuthRouteHost({
  getRuntime,
}: {
  getRuntime: () => KnowledgebaseIamRuntime;
}) {
  const props = {
    appearance: resolveKnowledgebaseAuthAppearance(),
    basePath: '/auth',
    getRuntime,
    homePath: '/',
    locale: resolveKnowledgebaseAuthLocale(),
    runtimeConfig: resolveKnowledgebaseAuthRuntimeConfig(),
    viewportMode: 'fixed' as const,
  };

  return (
    <React.Suspense fallback={<KnowledgebaseAuthRoutesFallback />}>
      <SdkworkIamAuthRoutes {...props as any} />
    </React.Suspense>
  );
}

function getKnowledgebaseIamRuntime(
  runtime: ReturnType<typeof createKnowledgebasePcRuntime>,
): KnowledgebaseIamRuntime {
  const iamRuntime = runtime.auth?.iamRuntime;
  if (!iamRuntime) {
    throwKnowledgebaseError(KnowledgebaseErrorCodes.IAM_RUNTIME_MISSING);
  }
  return iamRuntime as KnowledgebaseIamRuntime;
}

function KnowledgebaseAuthRoutesFallback() {
  return (
    <div
      aria-label="Loading Knowledgebase auth routes"
      className="sdkwork-knowledgebase-auth-loading"
    >
      <div className="h-7 w-7 rounded-full border-2 border-blue-500 border-t-transparent animate-spin" />
    </div>
  );
}

function KnowledgebaseRouteFallback({ label }: { label: string }) {
  return (
    <div
      aria-label={label}
      className="flex h-screen w-screen items-center justify-center bg-[var(--color-kb-bg-app)]"
    >
      <div className="h-8 w-8 rounded-full border-2 border-blue-500 border-t-transparent animate-spin" />
    </div>
  );
}
