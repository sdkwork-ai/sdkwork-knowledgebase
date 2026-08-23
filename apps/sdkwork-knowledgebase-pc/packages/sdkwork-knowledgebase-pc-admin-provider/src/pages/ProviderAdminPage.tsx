import React, { useCallback, useEffect, useState } from 'react';
import { Activity, ArrowLeft, KeyRound, Link2, Plus, RefreshCw, RotateCcw, ShieldAlert } from 'lucide-react';
import { useTranslation } from 'react-i18next';
import { useNavigate } from 'react-router-dom';
import { uuid } from '@sdkwork/utils/id';
import type {
  KnowledgeEngineProviderBinding,
  KnowledgeEngineProviderCredentialReference,
  KnowledgeEngineProviderMigrationOperation,
} from 'sdkwork-knowledgebase-pc-admin-core';
import {
  canAccessKnowledgebaseAdminConsole,
  isKnowledgebaseBackendApiAvailable,
} from 'sdkwork-knowledgebase-pc-admin-core';
import {
  resolveUserFacingErrorMessage,
  useKnowledgebaseRuntime,
  useKnowledgebaseSessionSnapshot,
} from 'sdkwork-knowledgebase-pc-core';
import { AdminDialog } from '../components/AdminDialog';
import { CursorPagination } from '../components/CursorPagination';
import { registerProviderAdminI18n } from '../providerAdminI18n';
import {
  providerAdminService,
  providerBindingActions,
  type ProviderAdminCursorPage,
  type ProviderAdminSpace,
} from '../services/providerAdminService';

type AdminTab = 'credentials' | 'bindings' | 'migrations';
type DialogState =
  | { kind: 'createCredential' }
  | { kind: 'rotateCredential'; credential: KnowledgeEngineProviderCredentialReference }
  | { kind: 'createBinding' }
  | { kind: 'editBinding'; binding: KnowledgeEngineProviderBinding }
  | { kind: 'createMigration' }
  | null;

const emptyPage = <T,>(): ProviderAdminCursorPage<T> => ({ items: [], hasMore: false });
const inputClass = 'mt-1 w-full border border-[var(--color-kb-panel-border)] bg-[var(--color-kb-bg)] px-3 py-2 text-sm outline-none focus:border-[var(--theme-accent)]';
const actionClass = 'inline-flex items-center gap-1 border border-[var(--color-kb-panel-border)] px-2.5 py-1.5 text-xs font-medium hover:bg-[var(--color-kb-panel-hover)] disabled:cursor-not-allowed disabled:opacity-40';

function displayDate(value: string | null | undefined, fallback: string): string {
  if (!value) return fallback;
  const date = new Date(value);
  return Number.isNaN(date.getTime()) ? value : date.toLocaleString();
}

function Status({ value }: { value: string }) {
  const color = value === 'active' || value === 'passed' || value === 'completed'
    ? 'border-emerald-500/30 bg-emerald-500/10 text-emerald-700 dark:text-emerald-300'
    : value === 'failed' || value === 'revoked'
      ? 'border-rose-500/30 bg-rose-500/10 text-rose-700 dark:text-rose-300'
      : 'border-amber-500/30 bg-amber-500/10 text-amber-700 dark:text-amber-300';
  return <span className={`inline-flex border px-2 py-1 text-xs font-medium ${color}`}>{value}</span>;
}

export function ProviderAdminPage() {
  const translation = useTranslation();
  registerProviderAdminI18n(translation.i18n);
  const t = translation.i18n.getFixedT(
    translation.i18n.resolvedLanguage ?? translation.i18n.language,
    'providerAdmin',
  );
  const navigate = useNavigate();
  const runtime = useKnowledgebaseRuntime();
  const session = useKnowledgebaseSessionSnapshot(runtime.session);
  const canAccess = canAccessKnowledgebaseAdminConsole(session.context?.permissionScope);
  const apiAvailable = isKnowledgebaseBackendApiAvailable();
  const [tab, setTab] = useState<AdminTab>('bindings');
  const [spaces, setSpaces] = useState<ProviderAdminSpace[]>([]);
  const [spaceCursor, setSpaceCursor] = useState<string>();
  const [selectedSpaceId, setSelectedSpaceId] = useState('');
  const [credentials, setCredentials] = useState(emptyPage<KnowledgeEngineProviderCredentialReference>);
  const [bindings, setBindings] = useState(emptyPage<KnowledgeEngineProviderBinding>);
  const [migrations, setMigrations] = useState(emptyPage<KnowledgeEngineProviderMigrationOperation>);
  const [credentialCursors, setCredentialCursors] = useState<(string | undefined)[]>([undefined]);
  const [bindingCursors, setBindingCursors] = useState<(string | undefined)[]>([undefined]);
  const [migrationCursors, setMigrationCursors] = useState<(string | undefined)[]>([undefined]);
  const [loading, setLoading] = useState(false);
  const [working, setWorking] = useState(false);
  const [errorMessage, setErrorMessage] = useState<string>();
  const [notice, setNotice] = useState<string>();
  const [dialog, setDialog] = useState<DialogState>(null);
  const [refreshToken, setRefreshToken] = useState(0);

  const fail = useCallback((error: unknown) => {
    setErrorMessage(resolveUserFacingErrorMessage(error, (key, options) => translation.t(key, options)));
  }, [translation]);

  const loadSpaces = useCallback(async (append = false) => {
    if (!canAccess || !apiAvailable) return;
    try {
      const page = await providerAdminService.listSpaces(append ? spaceCursor : undefined);
      setSpaces((current) => append ? [...current, ...page.items] : page.items);
      setSpaceCursor(page.nextCursor);
      setSelectedSpaceId((current) => current || page.items[0]?.id || '');
    } catch (error) {
      fail(error);
    }
  }, [apiAvailable, canAccess, fail, spaceCursor]);

  useEffect(() => {
    void loadSpaces(false);
  }, [apiAvailable, canAccess, refreshToken]);

  useEffect(() => {
    if (!canAccess || !apiAvailable) return;
    let cancelled = false;
    const load = async () => {
      setLoading(true);
      setErrorMessage(undefined);
      try {
        if (tab === 'credentials') {
          const page = await providerAdminService.listCredentials(credentialCursors.at(-1));
          if (!cancelled) setCredentials(page);
        } else if (selectedSpaceId && tab === 'bindings') {
          const page = await providerAdminService.listBindings(selectedSpaceId, bindingCursors.at(-1));
          if (!cancelled) setBindings(page);
        } else if (selectedSpaceId && tab === 'migrations') {
          const page = await providerAdminService.listMigrations(selectedSpaceId, migrationCursors.at(-1));
          if (!cancelled) setMigrations(page);
        }
      } catch (error) {
        if (!cancelled) fail(error);
      } finally {
        if (!cancelled) setLoading(false);
      }
    };
    void load();
    return () => { cancelled = true; };
  }, [apiAvailable, bindingCursors, canAccess, credentialCursors, fail, migrationCursors, refreshToken, selectedSpaceId, tab]);

  useEffect(() => {
    setBindingCursors([undefined]);
    setMigrationCursors([undefined]);
  }, [selectedSpaceId]);

  const activePage = tab === 'credentials' ? credentials : tab === 'bindings' ? bindings : migrations;
  const activeCursors = tab === 'credentials' ? credentialCursors : tab === 'bindings' ? bindingCursors : migrationCursors;
  const setActiveCursors = tab === 'credentials' ? setCredentialCursors : tab === 'bindings' ? setBindingCursors : setMigrationCursors;
  const selectedSpace = spaces.find((space) => space.id === selectedSpaceId);

  const refresh = useCallback(() => setRefreshToken((value) => value + 1), []);
  const mutate = useCallback(async (operation: () => Promise<void>) => {
    setWorking(true);
    setErrorMessage(undefined);
    setNotice(undefined);
    try {
      await operation();
      setDialog(null);
      setNotice(t('mutationComplete'));
      refresh();
    } catch (error) {
      fail(error);
    } finally {
      setWorking(false);
    }
  }, [fail, refresh, t]);

  if (!canAccess || !apiAvailable) {
    return (
      <AdminPageFrame backLabel={t('back')} onBack={() => navigate('/admin')} subtitle={t('subtitle')} title={t('title')}>
        <div className="flex gap-3 border border-rose-500/30 bg-rose-500/10 p-5" role="alert">
          <ShieldAlert className="shrink-0 text-rose-600" size={20} />
          <div>
            <h2 className="font-semibold">{canAccess ? t('apiUnavailable') : t('forbiddenTitle')}</h2>
            {!canAccess ? <p className="mt-1 text-sm">{t('forbiddenBody')}</p> : null}
          </div>
        </div>
      </AdminPageFrame>
    );
  }

  return (
    <AdminPageFrame backLabel={t('back')} onBack={() => navigate('/admin')} subtitle={t('subtitle')} title={t('title')}>
      <div className="flex flex-wrap items-end justify-between gap-4 border-b border-[var(--color-kb-panel-border)] pb-4">
        <div className="flex gap-1" role="tablist">
          {(['bindings', 'credentials', 'migrations'] as AdminTab[]).map((item) => (
            <button
              aria-selected={tab === item}
              className={`px-3 py-2 text-sm font-medium ${tab === item ? 'bg-[var(--theme-accent)] text-white' : 'hover:bg-[var(--color-kb-panel-hover)]'}`}
              key={item}
              onClick={() => setTab(item)}
              role="tab"
              type="button"
            >
              {t(item)}
            </button>
          ))}
        </div>
        <div className="flex flex-wrap items-end gap-2">
          {tab !== 'credentials' ? (
            <label className="text-xs font-medium text-[var(--color-kb-text-muted)]">
              {t('space')}
              <select className={`${inputClass} min-w-56`} onChange={(event) => setSelectedSpaceId(event.target.value)} value={selectedSpaceId}>
                <option value="">{t('selectSpace')}</option>
                {spaces.map((space) => <option key={space.id} value={space.id}>{space.name || space.id}</option>)}
              </select>
            </label>
          ) : null}
          {spaceCursor && tab !== 'credentials' ? (
            <button className={actionClass} onClick={() => void loadSpaces(true)} type="button">{t('loadMoreSpaces')}</button>
          ) : null}
          <button aria-label={t('refresh')} className={actionClass} onClick={refresh} title={t('refresh')} type="button">
            <RefreshCw size={15} />
          </button>
          <button
            className="inline-flex items-center gap-1 bg-[var(--theme-accent)] px-3 py-2 text-sm font-medium text-white disabled:opacity-40"
            disabled={tab !== 'credentials' && !selectedSpaceId}
            onClick={() => setDialog({ kind: tab === 'credentials' ? 'createCredential' : tab === 'bindings' ? 'createBinding' : 'createMigration' })}
            type="button"
          >
            <Plus size={15} /> {t('create')}
          </button>
        </div>
      </div>

      {selectedSpace && tab !== 'credentials' ? (
        <p className="text-xs text-[var(--color-kb-text-muted)]">{selectedSpace.name} · {selectedSpace.id} · {selectedSpace.knowledgeMode}</p>
      ) : null}
      {errorMessage ? <div className="border border-rose-500/30 bg-rose-500/10 p-3 text-sm" role="alert">{errorMessage}</div> : null}
      {notice ? <div className="border border-emerald-500/30 bg-emerald-500/10 p-3 text-sm" role="status">{notice}</div> : null}
      {loading ? <div className="py-10 text-center text-sm text-[var(--color-kb-text-muted)]" role="status">{t('loading')}</div> : null}
      {!loading && tab === 'credentials' ? <CredentialTable items={credentials.items} onMutate={mutate} onOpen={setDialog} t={t} /> : null}
      {!loading && tab === 'bindings' ? <BindingTable items={bindings.items} onMutate={mutate} onOpen={setDialog} spaceId={selectedSpaceId} t={t} /> : null}
      {!loading && tab === 'migrations' ? <MigrationTable items={migrations.items} onMutate={mutate} spaceId={selectedSpaceId} t={t} /> : null}

      {!loading ? (
        <CursorPagination
          canNext={activePage.hasMore && Boolean(activePage.nextCursor)}
          canPrevious={activeCursors.length > 1}
          nextLabel={t('next')}
          onNext={() => activePage.nextCursor && setActiveCursors((current) => [...current, activePage.nextCursor])}
          onPrevious={() => setActiveCursors((current) => current.length > 1 ? current.slice(0, -1) : current)}
          previousLabel={t('previous')}
        />
      ) : null}

      {dialog ? (
        <ProviderAdminFormDialog
          bindings={bindings.items}
          credentials={credentials.items}
          dialog={dialog}
          onClose={() => setDialog(null)}
          onSubmit={mutate}
          selectedSpaceId={selectedSpaceId}
          t={t}
          working={working}
        />
      ) : null}
    </AdminPageFrame>
  );
}

function AdminPageFrame(props: { backLabel: string; children: React.ReactNode; onBack(): void; subtitle: string; title: string }) {
  return (
    <div className="min-h-screen bg-[var(--color-kb-bg)] text-[var(--color-kb-text)]">
      <header className="flex items-center gap-4 border-b border-[var(--color-kb-panel-border)] bg-[var(--color-kb-panel)] px-5 py-3">
        <button className={actionClass} onClick={props.onBack} type="button"><ArrowLeft size={15} /> {props.backLabel}</button>
        <div><h1 className="text-base font-semibold">{props.title}</h1><p className="text-xs text-[var(--color-kb-text-muted)]">{props.subtitle}</p></div>
      </header>
      <main className="mx-auto max-w-screen-2xl space-y-4 px-5 py-5">{props.children}</main>
    </div>
  );
}

type Translator = (key: string, options?: Record<string, unknown>) => string;

function CredentialTable(props: { items: KnowledgeEngineProviderCredentialReference[]; onMutate(operation: () => Promise<void>): void; onOpen(dialog: DialogState): void; t: Translator }) {
  if (props.items.length === 0) return <EmptyState icon={<KeyRound size={20} />} text={props.t('emptyCredentials')} />;
  return <DataTable headers={['name', 'implementation', 'rotationState', 'lastRotated', 'version', 'actions'].map((key) => props.t(key))} rows={props.items.map((item) => [
    <div key="name"><div className="font-medium">{item.displayName}</div><div className="text-xs text-[var(--color-kb-text-muted)]">{item.id}</div></div>,
    item.implementationId,
    <span key="state"><Status value={item.rotationState} /></span>,
    displayDate(item.lastRotatedAt, props.t('notAvailable')),
    item.version,
    <div className="flex gap-2" key="actions">
      <button className={actionClass} disabled={item.rotationState === 'revoked'} onClick={() => props.onOpen({ kind: 'rotateCredential', credential: item })} type="button">{props.t('rotate')}</button>
      <button className={actionClass} disabled={item.rotationState === 'revoked'} onClick={() => window.confirm(props.t('confirmRevoke')) && props.onMutate(() => providerAdminService.revokeCredential(item.id, item.version))} type="button">{props.t('revoke')}</button>
    </div>,
  ])} />;
}

function BindingTable(props: { items: KnowledgeEngineProviderBinding[]; onMutate(operation: () => Promise<void>): void; onOpen(dialog: DialogState): void; spaceId: string; t: Translator }) {
  if (!props.spaceId || props.items.length === 0) return <EmptyState icon={<Link2 size={20} />} text={props.t('emptyBindings')} />;
  return <DataTable headers={['implementation', 'remoteResource', 'credential', 'lifecycle', 'capabilities', 'lastTested', 'actions'].map((key) => props.t(key))} rows={props.items.map((item) => {
    const actions = providerBindingActions(item);
    return [
      <div key="implementation"><div className="font-medium">{item.implementationId}</div><div className="text-xs text-[var(--color-kb-text-muted)]">{item.id}</div></div>,
      <span className="break-all" key="remote">{item.remoteResourceType}:{item.remoteResourceId}</span>,
      item.credentialReferenceId ?? props.t('credentialOptional'),
      <span key="state"><Status value={item.lifecycleState} /></span>,
      item.capabilitySnapshot.length ? item.capabilitySnapshot.join(', ') : props.t('noCapabilities'),
      displayDate(item.lastTestedAt, props.t('notAvailable')),
      <div className="flex flex-wrap gap-2" key="actions">
        <button className={actionClass} disabled={!actions.canUpdate} onClick={() => props.onOpen({ kind: 'editBinding', binding: item })} type="button">{props.t('edit')}</button>
        <button className={actionClass} disabled={!actions.canTest} onClick={() => props.onMutate(() => providerAdminService.testBinding(props.spaceId, item))} type="button">{props.t('test')}</button>
        <button className={actionClass} disabled={!actions.canActivate} onClick={() => props.onMutate(() => providerAdminService.activateBinding(props.spaceId, item))} type="button">{props.t('activate')}</button>
        <button className={actionClass} disabled={!actions.canDisable} onClick={() => window.confirm(props.t('confirmDisable')) && props.onMutate(() => providerAdminService.disableBinding(props.spaceId, item))} type="button">{props.t('disable')}</button>
      </div>,
    ];
  })} />;
}

function MigrationTable(props: { items: KnowledgeEngineProviderMigrationOperation[]; onMutate(operation: () => Promise<void>): void; spaceId: string; t: Translator }) {
  if (!props.spaceId || props.items.length === 0) return <EmptyState icon={<Activity size={20} />} text={props.t('emptyMigrations')} />;
  const rollbackStates = new Set(['dry_run', 'preparing', 'validating', 'cutover', 'observing', 'failed']);
  return <DataTable headers={['sourceBinding', 'targetBinding', 'state', 'attempts', 'updated', 'version', 'actions'].map((key) => props.t(key))} rows={props.items.map((item) => [
    item.sourceBindingId,
    item.targetBindingId,
    <span key="state"><Status value={item.operationState} /></span>,
    String(item.attemptCount),
    displayDate(item.updatedAt, props.t('notAvailable')),
    item.version,
    <button className={actionClass} disabled={!rollbackStates.has(item.operationState)} key="rollback" onClick={() => window.confirm(props.t('confirmRollback')) && props.onMutate(() => providerAdminService.rollbackMigration(props.spaceId, item))} type="button"><RotateCcw size={13} /> {props.t('rollback')}</button>,
  ])} />;
}

function DataTable({ headers, rows }: { headers: string[]; rows: React.ReactNode[][] }) {
  return <div className="overflow-x-auto border border-[var(--color-kb-panel-border)]"><table className="min-w-full text-sm"><thead className="bg-[var(--color-kb-panel-hover)] text-left text-xs text-[var(--color-kb-text-muted)]"><tr>{headers.map((header) => <th className="whitespace-nowrap px-4 py-3 font-medium" key={header}>{header}</th>)}</tr></thead><tbody>{rows.map((row, rowIndex) => <tr className="border-t border-[var(--color-kb-panel-border)] align-top" key={rowIndex}>{row.map((cell, cellIndex) => <td className="max-w-72 px-4 py-3" key={cellIndex}>{cell}</td>)}</tr>)}</tbody></table></div>;
}

function EmptyState({ icon, text }: { icon: React.ReactNode; text: string }) {
  return <div className="flex items-center justify-center gap-2 border border-dashed border-[var(--color-kb-panel-border)] py-12 text-sm text-[var(--color-kb-text-muted)]">{icon}{text}</div>;
}

function ProviderAdminFormDialog(props: { bindings: KnowledgeEngineProviderBinding[]; credentials: KnowledgeEngineProviderCredentialReference[]; dialog: Exclude<DialogState, null>; onClose(): void; onSubmit(operation: () => Promise<void>): void; selectedSpaceId: string; t: Translator; working: boolean }) {
  const binding = props.dialog.kind === 'editBinding' ? props.dialog.binding : undefined;
  const rotatingCredential = props.dialog.kind === 'rotateCredential' ? props.dialog.credential : undefined;
  const [implementationId, setImplementationId] = useState(binding?.implementationId ?? '');
  const [displayName, setDisplayName] = useState('');
  const [referenceLocator, setReferenceLocator] = useState('');
  const [remoteResourceType, setRemoteResourceType] = useState(binding?.remoteResourceType ?? 'dataset');
  const [remoteResourceId, setRemoteResourceId] = useState(binding?.remoteResourceId ?? '');
  const [credentialReferenceId, setCredentialReferenceId] = useState(binding?.credentialReferenceId ?? '');
  const [clearCredential, setClearCredential] = useState(false);
  const activeBinding = props.bindings.find((item) => item.lifecycleState === 'active');
  const testedBindings = props.bindings.filter((item) => item.lifecycleState === 'testing');
  const [sourceBindingId, setSourceBindingId] = useState<string>(activeBinding?.id ?? '');
  const [targetBindingId, setTargetBindingId] = useState<string>(testedBindings[0]?.id ?? '');
  const [observationSeconds, setObservationSeconds] = useState(300);
  const [idempotencyKey, setIdempotencyKey] = useState<string>(() => uuid());
  const [validationError, setValidationError] = useState<string>();
  const title = props.t(props.dialog.kind);

  const submit = () => {
    setValidationError(undefined);
    if (props.dialog.kind === 'createCredential') {
      if (!implementationId || !displayName || !referenceLocator) return setValidationError(props.t('requiredFields'));
      return props.onSubmit(() => providerAdminService.createCredential({ implementationId, displayName, referenceLocator }).then(() => undefined));
    }
    if (props.dialog.kind === 'rotateCredential') {
      if (!referenceLocator) return setValidationError(props.t('requiredFields'));
      return props.onSubmit(() => providerAdminService.rotateCredential(rotatingCredential!.id, { referenceLocator, expectedVersion: rotatingCredential!.version }));
    }
    if (props.dialog.kind === 'createBinding') {
      if (!implementationId || !remoteResourceType || !remoteResourceId) return setValidationError(props.t('requiredFields'));
      return props.onSubmit(() => providerAdminService.createBinding(props.selectedSpaceId, { implementationId, remoteResourceType, remoteResourceId, credentialReferenceId: credentialReferenceId || null }).then(() => undefined));
    }
    if (props.dialog.kind === 'editBinding' && binding) {
      if (!remoteResourceType || !remoteResourceId) return setValidationError(props.t('requiredFields'));
      return props.onSubmit(() => providerAdminService.updateBinding(props.selectedSpaceId, binding.id, { remoteResourceType, remoteResourceId, credentialReferenceId: clearCredential ? null : credentialReferenceId || null, clearCredentialReference: clearCredential, expectedVersion: binding.version }).then(() => undefined));
    }
    if (!sourceBindingId || !targetBindingId || sourceBindingId === targetBindingId || observationSeconds < 30 || !idempotencyKey) return setValidationError(props.t('requiredFields'));
    const source = props.bindings.find((item) => item.id === sourceBindingId);
    const target = props.bindings.find((item) => item.id === targetBindingId);
    if (!source || !target) return setValidationError(props.t('requiredFields'));
    props.onSubmit(() => providerAdminService.createMigration(props.selectedSpaceId, { sourceBindingId, targetBindingId, idempotencyKey, expectedSourceVersion: source.version, expectedTargetVersion: target.version, observationSeconds }).then(() => undefined));
  };

  return <AdminDialog onClose={props.onClose} title={title}><div className="space-y-4">
    {(props.dialog.kind === 'createCredential' || props.dialog.kind === 'createBinding') ? <Field label={props.t('implementation')} onChange={setImplementationId} required value={implementationId} /> : null}
    {props.dialog.kind === 'createCredential' ? <Field label={props.t('displayName')} onChange={setDisplayName} required value={displayName} /> : null}
    {(props.dialog.kind === 'createCredential' || props.dialog.kind === 'rotateCredential') ? <Field autoComplete="off" hint={props.t('referenceLocatorHint')} label={props.t('referenceLocator')} onChange={setReferenceLocator} required type="password" value={referenceLocator} /> : null}
    {(props.dialog.kind === 'createBinding' || props.dialog.kind === 'editBinding') ? <><Field label={props.t('remoteResourceType')} onChange={setRemoteResourceType} required value={remoteResourceType} /><Field label={props.t('remoteResourceId')} onChange={setRemoteResourceId} required value={remoteResourceId} /><label className="block text-sm font-medium">{props.t('credentialReferenceId')}<select className={inputClass} onChange={(event) => setCredentialReferenceId(event.target.value)} value={credentialReferenceId}><option value="">{props.t('credentialOptional')}</option>{props.credentials.filter((item) => item.rotationState !== 'revoked' && (!implementationId || item.implementationId === implementationId)).map((item) => <option key={item.id} value={item.id}>{item.displayName} · {item.id}</option>)}</select></label>{props.dialog.kind === 'editBinding' ? <label className="flex items-center gap-2 text-sm"><input checked={clearCredential} onChange={(event) => setClearCredential(event.target.checked)} type="checkbox" />{props.t('clearCredential')}</label> : null}</> : null}
    {props.dialog.kind === 'createMigration' ? <><SelectBinding label={props.t('sourceBinding')} onChange={setSourceBindingId} options={props.bindings.filter((item) => item.lifecycleState === 'active')} value={sourceBindingId} /><SelectBinding label={props.t('targetBinding')} onChange={setTargetBindingId} options={testedBindings} value={targetBindingId} /><Field label={props.t('observationSeconds')} min={30} onChange={(value) => setObservationSeconds(Number(value))} required type="number" value={String(observationSeconds)} /><Field label={props.t('idempotencyKey')} onChange={setIdempotencyKey} required value={idempotencyKey} /></> : null}
    {validationError ? <p className="text-sm text-rose-600" role="alert">{validationError}</p> : null}
    <div className="flex justify-end gap-2 pt-2"><button className={actionClass} disabled={props.working} onClick={props.onClose} type="button">{props.t('cancel')}</button><button className="bg-[var(--theme-accent)] px-4 py-2 text-sm font-medium text-white disabled:opacity-40" disabled={props.working} onClick={submit} type="button">{props.working ? props.t('working') : props.t('submit')}</button></div>
  </div></AdminDialog>;
}

function Field(props: { autoComplete?: string; hint?: string; label: string; min?: number; onChange(value: string): void; required?: boolean; type?: string; value: string }) {
  return <label className="block text-sm font-medium">{props.label}<input autoComplete={props.autoComplete} className={inputClass} min={props.min} onChange={(event) => props.onChange(event.target.value)} required={props.required} type={props.type ?? 'text'} value={props.value} />{props.hint ? <span className="mt-1 block text-xs font-normal text-[var(--color-kb-text-muted)]">{props.hint}</span> : null}</label>;
}

function SelectBinding(props: { label: string; onChange(value: string): void; options: KnowledgeEngineProviderBinding[]; value: string }) {
  return <label className="block text-sm font-medium">{props.label}<select className={inputClass} onChange={(event) => props.onChange(event.target.value)} value={props.value}><option value="">--</option>{props.options.map((item) => <option key={item.id} value={item.id}>{item.implementationId} · {item.id}</option>)}</select></label>;
}
