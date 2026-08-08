import { useCallback, useEffect, useState } from 'react';
import { useTranslation } from 'react-i18next';
import { Activity, ArrowLeft, Database, FileStack, Layers, ServerCog, ShieldAlert, Users } from 'lucide-react';
import { useNavigate } from 'react-router-dom';
import { formatBytes } from '@sdkwork/utils';
import {
  extractNextCursor,
  extractSdkWorkListItems,
  getKnowledgebaseBackendSdkClient,
  isKnowledgebaseBackendApiAvailable,
  loadAdminSpaceMembers,
  readNumberField,
  readOptionalStringField,
  readStringField,
  canAccessKnowledgebaseAdminConsole,
  type AdminSpaceMemberRow,
  type AdminSpaceRow,
} from 'sdkwork-knowledgebase-pc-admin-core';
import {
  resolveUserFacingErrorMessage,
  useKnowledgebaseRuntime,
  useKnowledgebaseSessionSnapshot,
} from 'sdkwork-knowledgebase-pc-core';

interface TenantStatusView {
  status: string;
  spaceCount: number;
  documentCount: number;
  createdAt?: string;
  quota?: TenantQuotaView;
}

interface TenantQuotaView {
  maxDocuments: number;
  documentCount: number;
  maxConcurrentIngestJobs: number;
  inflightIngestJobs: number;
  maxRetrievalsPerMinute: number;
  maxStorageBytes: number;
  storageBytesUsed: number;
}

interface SourceRow {
  id: string;
  spaceId: string;
  sourceType: string;
  provider?: string;
}

interface IndexRow {
  id: string;
  spaceId: string;
  indexKind: string;
  status: string;
}

interface RetrievalTraceRow {
  id: string;
  status: string;
  latencyMs?: number;
  resultCount: number;
}

interface ProviderHealthView {
  status: string;
  providerId: string;
  checkedAt?: string;
}

export function KnowledgebaseAdminConsole() {
  const { t } = useTranslation('shell');
  const navigate = useNavigate();
  const runtime = useKnowledgebaseRuntime();
  const session = useKnowledgebaseSessionSnapshot(runtime.session);
  const permissionScope = session.context?.permissionScope;
  const canAccess = canAccessKnowledgebaseAdminConsole(permissionScope);
  const [tenantStatus, setTenantStatus] = useState<TenantStatusView | null>(null);
  const [sources, setSources] = useState<SourceRow[]>([]);
  const [spaces, setSpaces] = useState<AdminSpaceRow[]>([]);
  const [members, setMembers] = useState<AdminSpaceMemberRow[]>([]);
  const [indexes, setIndexes] = useState<IndexRow[]>([]);
  const [traces, setTraces] = useState<RetrievalTraceRow[]>([]);
  const [providerHealth, setProviderHealth] = useState<ProviderHealthView | null>(null);
  const [loading, setLoading] = useState(true);
  const [loadingMore, setLoadingMore] = useState(false);
  const [errorMessage, setErrorMessage] = useState<string | null>(null);
  // Cursor pagination state: each list loads one bounded page at a time and appends the
  // next page on demand (PAGINATION_SPEC §8 — interactive lists never aggregate
  // full collections client-side).
  const [sourcesCursor, setSourcesCursor] = useState<string | undefined>(undefined);
  const [spacesCursor, setSpacesCursor] = useState<string | undefined>(undefined);
  const [indexesCursor, setIndexesCursor] = useState<string | undefined>(undefined);
  const [tracesCursor, setTracesCursor] = useState<string | undefined>(undefined);

  const ADMIN_PAGE_SIZE = 50;

  useEffect(() => {
    if (!canAccess) {
      setLoading(false);
      return;
    }
    if (!isKnowledgebaseBackendApiAvailable()) {
      setErrorMessage(t('adminConsoleApiUnavailable'));
      setLoading(false);
      return;
    }

    let cancelled = false;
    async function loadAdminData() {
      setLoading(true);
      setErrorMessage(null);
      try {
        const client = getKnowledgebaseBackendSdkClient().client;
        const [status, sourceList, spaceList, indexList, traceList, health] = await Promise.all([
          client.knowledge.tenants.current.list(),
          client.knowledge.sources.list({ pageSize: ADMIN_PAGE_SIZE }),
          client.knowledge.spaces.list({ pageSize: ADMIN_PAGE_SIZE }),
          client.knowledge.indexes.list({ pageSize: ADMIN_PAGE_SIZE }),
          client.knowledge.retrievalTraces.list({ pageSize: ADMIN_PAGE_SIZE }),
          client.knowledge.providerHealth.list(),
        ]);
        if (cancelled) {
          return;
        }
        setSourcesCursor(extractNextCursor(sourceList));
        setSpacesCursor(extractNextCursor(spaceList));
        setIndexesCursor(extractNextCursor(indexList));
        setTracesCursor(extractNextCursor(traceList));
        const spaceRows: AdminSpaceRow[] = extractSdkWorkListItems(spaceList).map((item) => ({
          id: readStringField(item, 'id'),
          name: readStringField(item, 'name'),
          knowledgeMode: readStringField(item, 'knowledgeMode'),
          driveBound: readOptionalStringField(item, 'driveSpaceId') != null,
        }));
        const memberRows = await loadAdminSpaceMembers(
          (spaceId) => client.knowledge.spaces.members.list(spaceId),
          spaceRows,
        );
        if (cancelled) {
          return;
        }
        setTenantStatus({
          status: status.status,
          spaceCount: Number(status.spaceCount) || 0,
          documentCount: Number(status.documentCount) || 0,
          createdAt: status.createdAt ?? undefined,
          quota: status.quota
            ? {
                maxDocuments: Number(status.quota.maxDocuments) || 0,
                documentCount: Number(status.quota.documentCount) || 0,
                maxConcurrentIngestJobs: status.quota.maxConcurrentIngestJobs,
                inflightIngestJobs: status.quota.inflightIngestJobs,
                maxRetrievalsPerMinute: status.quota.maxRetrievalsPerMinute,
                maxStorageBytes: Number(status.quota.maxStorageBytes) || 0,
                storageBytesUsed: Number(status.quota.storageBytesUsed) || 0,
              }
            : undefined,
        });
        setSources(
          extractSdkWorkListItems(sourceList).map((item) => ({
            id: readStringField(item, 'id'),
            spaceId: readStringField(item, 'spaceId'),
            sourceType: readStringField(item, 'sourceType'),
            provider: readOptionalStringField(item, 'provider'),
          })),
        );
        setSpaces(spaceRows);
        setMembers(memberRows);
        setIndexes(
          extractSdkWorkListItems(indexList).map((item) => ({
            id: readStringField(item, 'indexId'),
            spaceId: readStringField(item, 'spaceId'),
            indexKind: readStringField(item, 'indexKind'),
            status: readStringField(item, 'status'),
          })),
        );
        setTraces(
          extractSdkWorkListItems(traceList).map((item) => ({
            id: readStringField(item, 'retrievalTraceId'),
            status: readStringField(item, 'status'),
            latencyMs: readOptionalStringField(item, 'latencyMs')
              ? readNumberField(item, 'latencyMs')
              : undefined,
            resultCount: readNumberField(item, 'resultCount'),
          })),
        );
        setProviderHealth({
          status: health.status,
          providerId: health.providerId,
          checkedAt: health.checkedAt ?? undefined,
        });
      } catch (error) {
        if (cancelled) {
          return;
        }
        const message = resolveUserFacingErrorMessage(error, (key, options) => t(key, options));
        setErrorMessage(message);
      } finally {
        if (!cancelled) {
          setLoading(false);
        }
      }
    }

    void loadAdminData();
    return () => {
      cancelled = true;
    };
  }, [canAccess, t]);

  const hasMoreAdminData =
    sourcesCursor !== undefined ||
    spacesCursor !== undefined ||
    indexesCursor !== undefined ||
    tracesCursor !== undefined;

  const loadMoreAdminData = useCallback(async () => {
    if (loadingMore || !hasMoreAdminData) {
      return;
    }
    setLoadingMore(true);
    setErrorMessage(null);
    try {
      const client = getKnowledgebaseBackendSdkClient().client;
      if (sourcesCursor !== undefined) {
        const next = await client.knowledge.sources.list({
          cursor: sourcesCursor,
          pageSize: ADMIN_PAGE_SIZE,
        });
        setSources((prev) => [
          ...prev,
          ...extractSdkWorkListItems(next).map((item) => ({
            id: readStringField(item, 'id'),
            spaceId: readStringField(item, 'spaceId'),
            sourceType: readStringField(item, 'sourceType'),
            provider: readOptionalStringField(item, 'provider'),
          })),
        ]);
        setSourcesCursor(extractNextCursor(next));
      }
      if (spacesCursor !== undefined) {
        const next = await client.knowledge.spaces.list({
          cursor: spacesCursor,
          pageSize: ADMIN_PAGE_SIZE,
        });
        const nextRows: AdminSpaceRow[] = extractSdkWorkListItems(next).map((item) => ({
          id: readStringField(item, 'id'),
          name: readStringField(item, 'name'),
          knowledgeMode: readStringField(item, 'knowledgeMode'),
          driveBound: readOptionalStringField(item, 'driveSpaceId') != null,
        }));
        const nextMembers = await loadAdminSpaceMembers(
          (spaceId) => client.knowledge.spaces.members.list(spaceId),
          nextRows,
        );
        setSpaces((prev) => [...prev, ...nextRows]);
        setMembers((prev) => [...prev, ...nextMembers]);
        setSpacesCursor(extractNextCursor(next));
      }
      if (indexesCursor !== undefined) {
        const next = await client.knowledge.indexes.list({
          cursor: indexesCursor,
          pageSize: ADMIN_PAGE_SIZE,
        });
        setIndexes((prev) => [
          ...prev,
          ...extractSdkWorkListItems(next).map((item) => ({
            id: readStringField(item, 'indexId'),
            spaceId: readStringField(item, 'spaceId'),
            indexKind: readStringField(item, 'indexKind'),
            status: readStringField(item, 'status'),
          })),
        ]);
        setIndexesCursor(extractNextCursor(next));
      }
      if (tracesCursor !== undefined) {
        const next = await client.knowledge.retrievalTraces.list({
          cursor: tracesCursor,
          pageSize: ADMIN_PAGE_SIZE,
        });
        setTraces((prev) => [
          ...prev,
          ...extractSdkWorkListItems(next).map((item) => ({
            id: readStringField(item, 'retrievalTraceId'),
            status: readStringField(item, 'status'),
            latencyMs: readOptionalStringField(item, 'latencyMs')
              ? readNumberField(item, 'latencyMs')
              : undefined,
            resultCount: readNumberField(item, 'resultCount'),
          })),
        ]);
        setTracesCursor(extractNextCursor(next));
      }
    } catch (error) {
      setErrorMessage(resolveUserFacingErrorMessage(error, (key, options) => t(key, options)));
    } finally {
      setLoadingMore(false);
    }
  }, [
    loadingMore,
    hasMoreAdminData,
    sourcesCursor,
    spacesCursor,
    indexesCursor,
    tracesCursor,
    t,
  ]);

  return (
    <div className="min-h-screen bg-[var(--color-kb-bg)] text-[var(--color-kb-text)]">
      <header className="border-b border-[var(--color-kb-panel-border)] bg-[var(--color-kb-panel)] px-6 py-4 flex items-center gap-4">
        <button
          type="button"
          onClick={() => navigate('/')}
          className="inline-flex items-center gap-2 rounded-lg px-3 py-2 text-sm font-medium hover:bg-[var(--color-kb-panel-hover)]"
        >
          <ArrowLeft size={16} />
          {t('adminConsoleBack')}
        </button>
        <div className="min-w-0 flex-1">
          <h1 className="text-lg font-semibold text-[var(--color-kb-text-heading)]">{t('adminConsoleTitle')}</h1>
          <p className="text-sm text-[var(--color-kb-text-muted)]">{t('adminConsoleSubtitle')}</p>
        </div>
        {canAccess ? (
          <button
            type="button"
            onClick={() => navigate('/admin/providers')}
            className="inline-flex items-center gap-2 border border-[var(--color-kb-panel-border)] px-3 py-2 text-sm font-medium hover:bg-[var(--color-kb-panel-hover)]"
          >
            <ServerCog size={16} />
            {t('adminConsoleProviderOperations')}
          </button>
        ) : null}
      </header>

      <main className="mx-auto max-w-5xl px-6 py-8 space-y-6">
        {!canAccess ? (
          <section className="rounded-2xl border border-rose-200 bg-rose-50/70 dark:border-rose-900/40 dark:bg-rose-950/20 p-6 flex gap-3">
            <ShieldAlert className="text-rose-600 shrink-0" size={20} />
            <div>
              <h2 className="font-semibold text-rose-700 dark:text-rose-300">{t('adminConsoleForbiddenTitle')}</h2>
              <p className="text-sm text-rose-700/80 dark:text-rose-200/80 mt-1">{t('adminConsoleForbiddenBody')}</p>
            </div>
          </section>
        ) : null}

        {canAccess && loading ? (
          <p className="text-sm text-[var(--color-kb-text-muted)]">{t('adminConsoleLoading')}</p>
        ) : null}

        {canAccess && errorMessage ? (
          <section className="rounded-2xl border border-amber-200 bg-amber-50/70 dark:border-amber-900/40 dark:bg-amber-950/20 p-4 text-sm">
            {errorMessage}
          </section>
        ) : null}

        {canAccess && tenantStatus ? (
          <section className="grid gap-4 md:grid-cols-3">
            <article className="rounded-2xl border border-[var(--color-kb-panel-border)] bg-[var(--color-kb-panel)] p-5">
              <p className="text-xs uppercase tracking-wide text-[var(--color-kb-text-muted)]">{t('adminConsoleTenantStatus')}</p>
              <p className="mt-2 text-2xl font-semibold">{tenantStatus.status}</p>
            </article>
            <article className="rounded-2xl border border-[var(--color-kb-panel-border)] bg-[var(--color-kb-panel)] p-5">
              <div className="flex items-center gap-2 text-[var(--color-kb-text-muted)]">
                <Database size={16} />
                <p className="text-xs uppercase tracking-wide">{t('adminConsoleSpaceCount')}</p>
              </div>
              <p className="mt-2 text-2xl font-semibold">{tenantStatus.spaceCount}</p>
            </article>
            <article className="rounded-2xl border border-[var(--color-kb-panel-border)] bg-[var(--color-kb-panel)] p-5">
              <div className="flex items-center gap-2 text-[var(--color-kb-text-muted)]">
                <FileStack size={16} />
                <p className="text-xs uppercase tracking-wide">{t('adminConsoleDocumentCount')}</p>
              </div>
              <p className="mt-2 text-2xl font-semibold">{tenantStatus.documentCount}</p>
            </article>
          </section>
        ) : null}

        {canAccess && tenantStatus?.quota ? (
          <section className="rounded-2xl border border-[var(--color-kb-panel-border)] bg-[var(--color-kb-panel)] p-5">
            <h2 className="font-semibold text-[var(--color-kb-text-heading)]">{t('adminConsoleQuotaTitle')}</h2>
            <p className="text-sm text-[var(--color-kb-text-muted)] mt-1">{t('adminConsoleQuotaSubtitle')}</p>
            <div className="mt-4 grid gap-4 md:grid-cols-4 text-sm">
              <div>
                <p className="text-[var(--color-kb-text-muted)]">{t('adminConsoleQuotaDocuments')}</p>
                <p className="font-medium mt-1">
                  {tenantStatus.quota.documentCount} / {tenantStatus.quota.maxDocuments}
                </p>
              </div>
              <div>
                <p className="text-[var(--color-kb-text-muted)]">{t('adminConsoleQuotaStorage')}</p>
                <p className="font-medium mt-1">
                  {formatBytes(tenantStatus.quota.storageBytesUsed)} / {formatBytes(tenantStatus.quota.maxStorageBytes)}
                </p>
              </div>
              <div>
                <p className="text-[var(--color-kb-text-muted)]">{t('adminConsoleQuotaIngestJobs')}</p>
                <p className="font-medium mt-1">
                  {tenantStatus.quota.inflightIngestJobs} / {tenantStatus.quota.maxConcurrentIngestJobs}
                </p>
              </div>
              <div>
                <p className="text-[var(--color-kb-text-muted)]">{t('adminConsoleQuotaRetrievals')}</p>
                <p className="font-medium mt-1">{tenantStatus.quota.maxRetrievalsPerMinute}</p>
              </div>
            </div>
          </section>
        ) : null}

        {canAccess && providerHealth ? (
          <section className="rounded-2xl border border-[var(--color-kb-panel-border)] bg-[var(--color-kb-panel)] p-5">
            <div className="flex items-center gap-2 text-[var(--color-kb-text-muted)]">
              <Activity size={16} />
              <h2 className="font-semibold text-[var(--color-kb-text-heading)]">{t('adminConsoleProviderHealthTitle')}</h2>
            </div>
            <div className="mt-3 grid gap-3 md:grid-cols-3 text-sm">
              <div>
                <p className="text-[var(--color-kb-text-muted)]">{t('adminConsoleProviderStatus')}</p>
                <p className="font-medium mt-1">{providerHealth.status}</p>
              </div>
              <div>
                <p className="text-[var(--color-kb-text-muted)]">{t('adminConsoleProviderId')}</p>
                <p className="font-medium mt-1 break-all">{providerHealth.providerId || '—'}</p>
              </div>
              <div>
                <p className="text-[var(--color-kb-text-muted)]">{t('adminConsoleProviderCheckedAt')}</p>
                <p className="font-medium mt-1">{providerHealth.checkedAt ?? '—'}</p>
              </div>
            </div>
          </section>
        ) : null}

        {canAccess && !loading ? (
          <section className="rounded-2xl border border-[var(--color-kb-panel-border)] bg-[var(--color-kb-panel)] overflow-hidden">
            <div className="border-b border-[var(--color-kb-panel-border)] px-5 py-4">
              <h2 className="font-semibold text-[var(--color-kb-text-heading)]">{t('adminConsoleSourcesTitle')}</h2>
              <p className="text-sm text-[var(--color-kb-text-muted)] mt-1">{t('adminConsoleSourcesSubtitle')}</p>
            </div>
            {sources.length === 0 ? (
              <p className="px-5 py-6 text-sm text-[var(--color-kb-text-muted)]">{t('adminConsoleSourcesEmpty')}</p>
            ) : (
              <div className="overflow-x-auto">
                <table className="min-w-full text-sm">
                  <thead className="bg-[var(--color-kb-panel-hover)] text-left text-[var(--color-kb-text-muted)]">
                    <tr>
                      <th className="px-5 py-3 font-medium">{t('adminConsoleSourceId')}</th>
                      <th className="px-5 py-3 font-medium">{t('adminConsoleSpaceId')}</th>
                      <th className="px-5 py-3 font-medium">{t('adminConsoleSourceType')}</th>
                      <th className="px-5 py-3 font-medium">{t('adminConsoleProvider')}</th>
                    </tr>
                  </thead>
                  <tbody>
                    {sources.map((source) => (
                      <tr key={source.id} className="border-t border-[var(--color-kb-panel-border)]">
                        <td className="px-5 py-3">{source.id}</td>
                        <td className="px-5 py-3">{source.spaceId}</td>
                        <td className="px-5 py-3">{source.sourceType}</td>
                        <td className="px-5 py-3">{source.provider ?? '—'}</td>
                      </tr>
                    ))}
                  </tbody>
                </table>
              </div>
            )}
          </section>
        ) : null}

        {canAccess && !loading ? (
          <section className="rounded-2xl border border-[var(--color-kb-panel-border)] bg-[var(--color-kb-panel)] overflow-hidden">
            <div className="border-b border-[var(--color-kb-panel-border)] px-5 py-4">
              <h2 className="font-semibold text-[var(--color-kb-text-heading)]">{t('adminConsoleSpacesTitle')}</h2>
              <p className="text-sm text-[var(--color-kb-text-muted)] mt-1">{t('adminConsoleSpacesSubtitle')}</p>
            </div>
            {spaces.length === 0 ? (
              <p className="px-5 py-6 text-sm text-[var(--color-kb-text-muted)]">{t('adminConsoleSpacesEmpty')}</p>
            ) : (
              <div className="overflow-x-auto">
                <table className="min-w-full text-sm">
                  <thead className="bg-[var(--color-kb-panel-hover)] text-left text-[var(--color-kb-text-muted)]">
                    <tr>
                      <th className="px-5 py-3 font-medium">{t('adminConsoleSpaceId')}</th>
                      <th className="px-5 py-3 font-medium">{t('adminConsoleSpaceName')}</th>
                      <th className="px-5 py-3 font-medium">{t('adminConsoleSpaceMode')}</th>
                      <th className="px-5 py-3 font-medium">{t('adminConsoleSpaceDriveBound')}</th>
                    </tr>
                  </thead>
                  <tbody>
                    {spaces.map((space) => (
                      <tr key={space.id} className="border-t border-[var(--color-kb-panel-border)]">
                        <td className="px-5 py-3">{space.id}</td>
                        <td className="px-5 py-3">{space.name}</td>
                        <td className="px-5 py-3">{space.knowledgeMode}</td>
                        <td className="px-5 py-3">{space.driveBound ? t('adminConsoleYes') : t('adminConsoleNo')}</td>
                      </tr>
                    ))}
                  </tbody>
                </table>
              </div>
            )}
          </section>
        ) : null}

        {canAccess && !loading ? (
          <section className="rounded-2xl border border-[var(--color-kb-panel-border)] bg-[var(--color-kb-panel)] overflow-hidden">
            <div className="border-b border-[var(--color-kb-panel-border)] px-5 py-4 flex items-center gap-2">
              <Users size={16} className="text-[var(--color-kb-text-muted)]" />
              <div>
                <h2 className="font-semibold text-[var(--color-kb-text-heading)]">{t('adminConsoleMembersTitle')}</h2>
                <p className="text-sm text-[var(--color-kb-text-muted)] mt-1">{t('adminConsoleMembersSubtitle')}</p>
              </div>
            </div>
            {members.length === 0 ? (
              <p className="px-5 py-6 text-sm text-[var(--color-kb-text-muted)]">{t('adminConsoleMembersEmpty')}</p>
            ) : (
              <div className="overflow-x-auto">
                <table className="min-w-full text-sm">
                  <thead className="bg-[var(--color-kb-panel-hover)] text-left text-[var(--color-kb-text-muted)]">
                    <tr>
                      <th className="px-5 py-3 font-medium">{t('adminConsoleSpaceName')}</th>
                      <th className="px-5 py-3 font-medium">{t('adminConsoleMemberSubjectType')}</th>
                      <th className="px-5 py-3 font-medium">{t('adminConsoleMemberSubjectId')}</th>
                      <th className="px-5 py-3 font-medium">{t('adminConsoleMemberRole')}</th>
                      <th className="px-5 py-3 font-medium">{t('adminConsoleMemberInherited')}</th>
                    </tr>
                  </thead>
                  <tbody>
                    {members.map((member) => (
                      <tr
                        key={`${member.spaceId}:${member.subjectType}:${member.subjectId}`}
                        className="border-t border-[var(--color-kb-panel-border)]"
                      >
                        <td className="px-5 py-3">{member.spaceName}</td>
                        <td className="px-5 py-3">{member.subjectType}</td>
                        <td className="px-5 py-3">{member.subjectId}</td>
                        <td className="px-5 py-3">{member.role}</td>
                        <td className="px-5 py-3">{member.inherited ? t('adminConsoleYes') : t('adminConsoleNo')}</td>
                      </tr>
                    ))}
                  </tbody>
                </table>
              </div>
            )}
          </section>
        ) : null}

        {canAccess && !loading ? (
          <section className="rounded-2xl border border-[var(--color-kb-panel-border)] bg-[var(--color-kb-panel)] overflow-hidden">
            <div className="border-b border-[var(--color-kb-panel-border)] px-5 py-4 flex items-center gap-2">
              <Layers size={16} className="text-[var(--color-kb-text-muted)]" />
              <div>
                <h2 className="font-semibold text-[var(--color-kb-text-heading)]">{t('adminConsoleIndexesTitle')}</h2>
                <p className="text-sm text-[var(--color-kb-text-muted)] mt-1">{t('adminConsoleIndexesSubtitle')}</p>
              </div>
            </div>
            {indexes.length === 0 ? (
              <p className="px-5 py-6 text-sm text-[var(--color-kb-text-muted)]">{t('adminConsoleIndexesEmpty')}</p>
            ) : (
              <div className="overflow-x-auto">
                <table className="min-w-full text-sm">
                  <thead className="bg-[var(--color-kb-panel-hover)] text-left text-[var(--color-kb-text-muted)]">
                    <tr>
                      <th className="px-5 py-3 font-medium">{t('adminConsoleIndexId')}</th>
                      <th className="px-5 py-3 font-medium">{t('adminConsoleSpaceId')}</th>
                      <th className="px-5 py-3 font-medium">{t('adminConsoleIndexKind')}</th>
                      <th className="px-5 py-3 font-medium">{t('adminConsoleIndexStatus')}</th>
                    </tr>
                  </thead>
                  <tbody>
                    {indexes.map((index) => (
                      <tr key={index.id} className="border-t border-[var(--color-kb-panel-border)]">
                        <td className="px-5 py-3">{index.id}</td>
                        <td className="px-5 py-3">{index.spaceId}</td>
                        <td className="px-5 py-3">{index.indexKind}</td>
                        <td className="px-5 py-3">{index.status}</td>
                      </tr>
                    ))}
                  </tbody>
                </table>
              </div>
            )}
          </section>
        ) : null}

        {canAccess && !loading ? (
          <section className="rounded-2xl border border-[var(--color-kb-panel-border)] bg-[var(--color-kb-panel)] overflow-hidden">
            <div className="border-b border-[var(--color-kb-panel-border)] px-5 py-4">
              <h2 className="font-semibold text-[var(--color-kb-text-heading)]">{t('adminConsoleTracesTitle')}</h2>
              <p className="text-sm text-[var(--color-kb-text-muted)] mt-1">{t('adminConsoleTracesSubtitle')}</p>
            </div>
            {traces.length === 0 ? (
              <p className="px-5 py-6 text-sm text-[var(--color-kb-text-muted)]">{t('adminConsoleTracesEmpty')}</p>
            ) : (
              <div className="overflow-x-auto">
                <table className="min-w-full text-sm">
                  <thead className="bg-[var(--color-kb-panel-hover)] text-left text-[var(--color-kb-text-muted)]">
                    <tr>
                      <th className="px-5 py-3 font-medium">{t('adminConsoleTraceId')}</th>
                      <th className="px-5 py-3 font-medium">{t('adminConsoleTraceStatus')}</th>
                      <th className="px-5 py-3 font-medium">{t('adminConsoleTraceLatency')}</th>
                      <th className="px-5 py-3 font-medium">{t('adminConsoleTraceResults')}</th>
                    </tr>
                  </thead>
                  <tbody>
                    {traces.map((trace) => (
                      <tr key={trace.id} className="border-t border-[var(--color-kb-panel-border)]">
                        <td className="px-5 py-3">{trace.id}</td>
                        <td className="px-5 py-3">{trace.status}</td>
                        <td className="px-5 py-3">
                          {trace.latencyMs != null ? `${trace.latencyMs} ms` : '—'}
                        </td>
                        <td className="px-5 py-3">{trace.resultCount}</td>
                      </tr>
                    ))}
                  </tbody>
                </table>
              </div>
            )}
          </section>
        ) : null}

        {canAccess && !loading && hasMoreAdminData ? (
          <section className="flex justify-center pt-2">
            <button
              type="button"
              onClick={() => void loadMoreAdminData()}
              disabled={loadingMore}
              className="inline-flex items-center gap-2 rounded-lg border border-[var(--color-kb-panel-border)] bg-[var(--color-kb-panel)] px-4 py-2 text-sm font-medium hover:bg-[var(--color-kb-panel-hover)] disabled:opacity-60"
            >
              <Layers size={16} />
              {loadingMore ? t('adminConsoleLoadingMore') : t('adminConsoleLoadMore')}
            </button>
          </section>
        ) : null}
      </main>
    </div>
  );
}
