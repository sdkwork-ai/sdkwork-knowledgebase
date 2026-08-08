import { useEffect, useLayoutEffect, useRef, useState } from 'react';
import { AlertTriangle, LoaderCircle } from 'lucide-react';
import { useLocation } from 'react-router-dom';
import { useTranslation } from 'react-i18next';
import {
  consumeGroupKnowledgebaseLaunchTicket,
  isValidGroupKnowledgebaseLaunchTicket,
  takePendingGroupKnowledgebaseLaunchTicket,
  useKnowledgebaseRuntime,
  type GroupKnowledgebaseLaunchConsumeResult,
  type GroupKnowledgebaseLaunchTarget,
} from 'sdkwork-knowledgebase-pc-core';

import {
  KnowledgeBaseApp,
  ToastContainer,
} from '@sdkwork/sdkwork-knowledgebase-pc-knowledgebase';

type LaunchPageState =
  | { kind: 'reading' }
  | { kind: 'consuming'; ticket: string }
  | { kind: 'ready'; target: GroupKnowledgebaseLaunchTarget }
  | { kind: 'unavailable' }
  | { kind: 'failed' };

function takeLaunchTicketFromFragment(): string | null {
  if (typeof window === 'undefined') {
    return null;
  }

  const hash = window.location.hash;
  if (!hash.startsWith('#')) {
    return null;
  }

  const params = new URLSearchParams(hash.slice(1));
  const ticket = params.get('ticket');
  // Remove the one-time ticket before any network activity or rendering that
  // could otherwise persist it in browser history or diagnostic tooling.
  window.history.replaceState(
    window.history.state,
    '',
    `${window.location.pathname}${window.location.search}`,
  );

  return params.size === 1 && isValidGroupKnowledgebaseLaunchTicket(ticket)
    ? ticket
    : null;
}

function renderLaunchState(
  state: Exclude<LaunchPageState, { kind: 'ready' }>,
  t: (key: string) => string,
) {
  if (state.kind === 'reading' || state.kind === 'consuming') {
    return (
      <div className="flex h-screen w-screen items-center justify-center bg-[var(--color-kb-bg-app)] text-kb-text-muted">
        <LoaderCircle aria-label={t('groupLaunch.opening')} className="h-8 w-8 animate-spin" />
      </div>
    );
  }

  const message = state.kind === 'unavailable'
    ? t('groupLaunch.unavailable')
    : t('groupLaunch.failed');
  return (
    <div className="flex h-screen w-screen flex-col items-center justify-center gap-3 bg-[var(--color-kb-bg-app)] px-6 text-center text-kb-text-muted">
      <AlertTriangle aria-hidden="true" className="h-8 w-8 text-amber-500" />
      <p className="max-w-md text-sm leading-6">{message}</p>
    </div>
  );
}

/**
 * Full standalone group workspace. It consumes the opaque ticket once and
 * scopes the shared editor to the exact server-authorized knowledge space.
 */
export function GroupKnowledgebaseLaunchPage() {
  const runtime = useKnowledgebaseRuntime();
  const location = useLocation();
  const { t } = useTranslation('common');
  const [state, setState] = useState<LaunchPageState>({ kind: 'reading' });
  const handledHashRef = useRef<string | null>(null);
  const ticketForLocationRef = useRef<{ locationKey: string; ticket: string } | null>(null);
  const pendingConsumeRef = useRef<{
    promise: Promise<GroupKnowledgebaseLaunchConsumeResult>;
    ticket: string;
  } | null>(null);

  useLayoutEffect(() => {
    const locationKey = `${location.pathname}${location.search}${location.hash ?? ''}`;
    const currentHash = typeof window === 'undefined' ? '' : window.location.hash;
    const pendingTicket = takePendingGroupKnowledgebaseLaunchTicket();
    if (pendingTicket) {
      ticketForLocationRef.current = { locationKey, ticket: pendingTicket };
      setState({ kind: 'consuming', ticket: pendingTicket });
      return;
    }
    const rememberedTicket = ticketForLocationRef.current;
    if (rememberedTicket?.locationKey === locationKey) {
      setState({ kind: 'consuming', ticket: rememberedTicket.ticket });
      return;
    }
    if (!currentHash || handledHashRef.current === currentHash) {
      setState({ kind: 'failed' });
      return;
    }
    handledHashRef.current = currentHash;
    const ticket = takeLaunchTicketFromFragment();
    if (ticket) {
      ticketForLocationRef.current = { locationKey, ticket };
    }
    setState(ticket ? { kind: 'consuming', ticket } : { kind: 'failed' });
  }, [location.hash, location.pathname, location.search]);

  useEffect(() => {
    if (state.kind !== 'consuming') {
      return;
    }

    if (pendingConsumeRef.current?.ticket !== state.ticket) {
      pendingConsumeRef.current = {
        ticket: state.ticket,
        promise: consumeGroupKnowledgebaseLaunchTicket(runtime.sdk.app, state.ticket),
      };
    }

    const pendingConsume = pendingConsumeRef.current;
    if (!pendingConsume) {
      return;
    }

    let active = true;
    void pendingConsume.promise.then((result) => {
      if (!active) {
        return;
      }
      if (result.kind === 'ready') {
        setState({ kind: 'ready', target: result.target });
      } else {
        setState({ kind: result.kind });
      }
    });
    return () => {
      active = false;
    };
  }, [runtime.sdk.app, state]);

  if (state.kind !== 'ready') {
    return renderLaunchState(state, t);
  }

  return (
    <div
      key={state.target.spaceId}
      className="flex h-screen w-screen min-h-0 min-w-0 overflow-hidden bg-[var(--color-kb-bg-app)]"
    >
      <KnowledgeBaseApp
        activeTab="kb"
        fixedKnowledgeBase={{
          id: state.target.spaceId,
          title: state.target.groupName,
          type: 'team',
        }}
        workspaceMode="ephemeral-fixed"
      />
      <ToastContainer />
    </div>
  );
}
