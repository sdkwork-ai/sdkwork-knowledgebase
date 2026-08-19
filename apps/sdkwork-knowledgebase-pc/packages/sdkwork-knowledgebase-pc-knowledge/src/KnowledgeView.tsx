import React, { useEffect, useMemo, useSyncExternalStore, type ComponentProps } from 'react';
import { I18nextProvider } from 'react-i18next';
import { KnowledgebaseRuntimeProvider } from 'sdkwork-knowledgebase-pc-core';

import { KnowledgeBaseApp, ToastContainer } from '@sdkwork/sdkwork-knowledgebase-pc-knowledgebase';
import '@sdkwork/knowledgebase-pc-knowledge/i18n';
import i18n from './i18n';

import { bindHostSessionToKnowledgebaseStore } from './sessionBridge';
import { createHostManagedKnowledgebaseRuntime } from './createHostManagedKnowledgebaseRuntime';
import {
  subscribeKnowledgebaseHostLanguage,
  syncKnowledgebaseHostLanguage,
} from './hostLanguageBridge';
import {
  resolveKnowledgebaseHostColorScheme,
  subscribeKnowledgebaseHostColorScheme,
  syncKnowledgebaseHostColorScheme,
} from './hostColorSchemeBridge';

type HostManagedKnowledgebaseRuntime = ReturnType<typeof createHostManagedKnowledgebaseRuntime>;

const KnowledgeViewContent: React.FC<{ runtime: HostManagedKnowledgebaseRuntime }> = ({ runtime }) => {
  const readHostColorScheme = (): 'light' | 'dark' | undefined => resolveKnowledgebaseHostColorScheme()
  const hostColorScheme = useSyncExternalStore(
    (listener) => subscribeKnowledgebaseHostColorScheme(listener) ?? (() => {}),
    readHostColorScheme,
    readHostColorScheme,
  )

  useEffect(() => {
    return bindHostSessionToKnowledgebaseStore(runtime.session);
  }, [runtime.session]);

  useEffect(() => {
    syncKnowledgebaseHostColorScheme();
    return subscribeKnowledgebaseHostColorScheme();
  }, []);

  return (
    <KnowledgebaseRuntimeProvider runtime={runtime}>
      <div
        className={`flex flex-1 min-h-0 min-w-0 overflow-hidden bg-[var(--color-kb-bg-app)]${hostColorScheme === 'dark' ? ' dark' : hostColorScheme === 'light' ? ' light-mode' : ''}`}
        data-sdk-color-mode={hostColorScheme}
      >
        <React.Suspense
          fallback={(
            <div className="flex flex-1 items-center justify-center text-kb-text-muted">
              Loading knowledge base...
            </div>
          )}
        >
          <KnowledgeBaseApp />
        </React.Suspense>
        <ToastContainer />
      </div>
    </KnowledgebaseRuntimeProvider>
  );
};

type KnowledgebaseI18nProviderProps = ComponentProps<typeof I18nextProvider>;

export const KnowledgeView: React.FC = () => {
  syncKnowledgebaseHostLanguage();
  syncKnowledgebaseHostColorScheme();

  useEffect(() => subscribeKnowledgebaseHostLanguage(), []);
  useEffect(() => subscribeKnowledgebaseHostColorScheme(), []);

  const runtime = useMemo(() => createHostManagedKnowledgebaseRuntime(), []);

  return (
    <I18nextProvider i18n={i18n as KnowledgebaseI18nProviderProps['i18n']}>
      <KnowledgeViewContent runtime={runtime} />
    </I18nextProvider>
  );
};
