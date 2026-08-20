import React, { Suspense, lazy } from 'react';

import type { KnowledgebaseHostContext, KnowledgebaseHostPresentationMode } from './knowledgebaseHostPresentation';
import { resolveKnowledgebaseHostEmbedUrl } from './resolveKnowledgebaseHostEmbedUrl';

const KnowledgeView = lazy(async () => {
  const module = await import('./KnowledgeView');
  return { default: module.KnowledgeView };
});

export interface KnowledgebaseHostSurfaceProps {
  presentationMode: KnowledgebaseHostPresentationMode;
  title?: string;
  context?: KnowledgebaseHostContext;
}

export const KnowledgebaseHostSurface: React.FC<KnowledgebaseHostSurfaceProps> = ({
  presentationMode,
  title,
  context,
}) => {
  if (presentationMode === 'detached-iframe') {
    const embedUrl = resolveKnowledgebaseHostEmbedUrl(context);
    return (
      <iframe
        title={title?.trim() || 'Knowledge Base'}
        src={embedUrl}
        className="h-full w-full flex-1 border-0 bg-[var(--color-kb-bg-app,#1e1e1e)]"
        allow="clipboard-read; clipboard-write"
      />
    );
  }

  return (
    <Suspense
      fallback={(
        <div className="flex flex-1 items-center justify-center text-sm text-gray-400">
          Loading knowledge base...
        </div>
      )}
    >
      <KnowledgeView />
    </Suspense>
  );
};
