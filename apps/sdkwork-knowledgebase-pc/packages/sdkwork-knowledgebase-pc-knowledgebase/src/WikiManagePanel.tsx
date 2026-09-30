import React, { useState, useEffect, useCallback } from 'react';
import { useTranslation } from 'react-i18next';
import {
  BookOpen,
  Play,
  Pause,
  Globe,
  Link2,
  EyeOff,
  RefreshCw,
  AlertCircle,
  ChevronDown,
  ShieldCheck,
} from 'lucide-react';
import {
  isKnowledgebaseApiAvailable,
  parseSdkProblemDetails,
} from 'sdkwork-knowledgebase-pc-core';
import type { KnowledgeBase } from './services/document';
import { toastKnowledgebaseError } from './components/ui/toastKnowledgebaseError';
import {
  activateWikiPublication,
  changeWikiSourceFileVisibility,
  isWikiPublishRowEligible,
  listWikiSourceFiles,
  pauseWikiPublication,
  publishWikiSourceFile,
  retrieveWikiPublication,
  unpublishWikiSourceFile,
  WIKI_ACTIVATE_ELIGIBLE_STATUSES,
  WIKI_PAUSE_ELIGIBLE_STATUSES,
  WIKI_PUBLISH_ELIGIBLE_STATUSES,
  WIKI_SOURCE_FILES_PAGE_SIZE,
  type KnowledgeWikiPublication,
  type KnowledgeWikiPublicationStatus,
  type KnowledgeWikiSourceFile,
  type WikiSourceFilePage,
} from './services/knowledgeWikiAdminService';

interface WikiManagePanelProps {
  kb: KnowledgeBase;
}

type WikiPanelError = 'unavailable' | 'not_initialized' | 'failed' | null;

const STATUS_BADGE_CLASS: Record<KnowledgeWikiPublicationStatus, string> = {
  draft: 'bg-zinc-100 text-zinc-600 dark:bg-zinc-800 dark:text-zinc-300',
  validating: 'bg-blue-50 text-blue-600 dark:bg-blue-950/60 dark:text-blue-300',
  ready: 'bg-teal-50 text-teal-600 dark:bg-teal-950/60 dark:text-teal-300',
  active: 'bg-emerald-100 text-emerald-700 dark:bg-emerald-950/60 dark:text-emerald-300',
  degraded: 'bg-amber-50 text-amber-600 dark:bg-amber-950/60 dark:text-amber-300',
  paused: 'bg-slate-100 text-slate-500 dark:bg-slate-800 dark:text-slate-300',
  archived: 'bg-zinc-100 text-zinc-400 dark:bg-zinc-800 dark:text-zinc-500',
  failed: 'bg-red-50 text-red-600 dark:bg-red-950/60 dark:text-red-300',
};

const SOURCE_STATE_CLASS: Record<string, string> = {
  discovered: 'bg-zinc-100 text-zinc-500 dark:bg-zinc-800 dark:text-zinc-400',
  queued: 'bg-blue-50 text-blue-500 dark:bg-blue-950/60 dark:text-blue-300',
  processing: 'bg-blue-50 text-blue-600 dark:bg-blue-950/60 dark:text-blue-300',
  ready: 'bg-emerald-50 text-emerald-600 dark:bg-emerald-950/60 dark:text-emerald-300',
  error: 'bg-red-50 text-red-600 dark:bg-red-950/60 dark:text-red-300',
  quarantined: 'bg-amber-50 text-amber-600 dark:bg-amber-950/60 dark:text-amber-300',
  deleted: 'bg-zinc-100 text-zinc-400 dark:bg-zinc-800 dark:text-zinc-500',
};

const PAGE_PUBLICATION_STATE_LABEL: Record<string, string> = {
  draft: '草稿',
  in_review: '审核中',
  scheduled: '已排期',
  published: '已发布',
  unpublished: '已下线',
  archived: '已归档',
};

const INDEX_STATE_CLASS: Record<string, string> = {
  not_required: 'bg-zinc-100 text-zinc-400 dark:bg-zinc-800 dark:text-zinc-500',
  pending: 'bg-blue-50 text-blue-500 dark:bg-blue-950/60 dark:text-blue-300',
  indexing: 'bg-blue-50 text-blue-600 dark:bg-blue-950/60 dark:text-blue-300',
  ready: 'bg-emerald-50 text-emerald-600 dark:bg-emerald-950/60 dark:text-emerald-300',
  error: 'bg-red-50 text-red-600 dark:bg-red-950/60 dark:text-red-300',
};

function Badge({ className, children }: { className: string; children: React.ReactNode }) {
  return (
    <span className={`inline-flex items-center px-2 py-0.5 rounded-md text-[10.5px] font-bold uppercase tracking-wide ${className}`}>
      {children}
    </span>
  );
}

export function WikiManagePanel({ kb }: WikiManagePanelProps) {
  const { t } = useTranslation(['kb', 'common']);
  const [publication, setPublication] = useState<KnowledgeWikiPublication | null>(null);
  const [panelError, setPanelError] = useState<WikiPanelError>(null);
  const [loading, setLoading] = useState(true);
  const [refreshing, setRefreshing] = useState(false);
  const [lifecycleBusy, setLifecycleBusy] = useState<'activate' | 'pause' | null>(null);

  const [sourceFiles, setSourceFiles] = useState<KnowledgeWikiSourceFile[]>([]);
  const [filesNextCursor, setFilesNextCursor] = useState<string | null>(null);
  const [filesHasMore, setFilesHasMore] = useState(false);
  const [filesLoadingMore, setFilesLoadingMore] = useState(false);
  const [busyRowUuid, setBusyRowUuid] = useState<string | null>(null);

  const spaceId = String(kb.id);
  const numericSpaceId = Number(spaceId);
  const apiReady = isKnowledgebaseApiAvailable() && Number.isFinite(numericSpaceId) && numericSpaceId > 0;

  const applySourceFilePage = useCallback((page: WikiSourceFilePage, append: boolean) => {
    setSourceFiles((prev) => (append ? [...prev, ...page.items] : page.items));
    setFilesNextCursor(page.nextCursor);
    setFilesHasMore(page.hasMore);
  }, []);

  const reloadAll = useCallback(async () => {
    const pub = await retrieveWikiPublication(spaceId);
    setPublication(pub);
    const page = await listWikiSourceFiles(spaceId, null, WIKI_SOURCE_FILES_PAGE_SIZE);
    applySourceFilePage(page, false);
  }, [spaceId, applySourceFilePage]);

  useEffect(() => {
    if (!apiReady) {
      setPanelError('unavailable');
      setLoading(false);
      return;
    }
    let cancelled = false;
    setLoading(true);
    setPanelError(null);
    (async () => {
      try {
        await reloadAll();
        if (cancelled) return;
        setPanelError(null);
      } catch (error) {
        if (cancelled) return;
        const status = parseSdkProblemDetails(error)?.status;
        if (status === 404) {
          setPanelError('not_initialized');
        } else {
          setPanelError('failed');
        }
      } finally {
        if (!cancelled) {
          setLoading(false);
        }
      }
    })();
    return () => {
      cancelled = true;
    };
  }, [apiReady, reloadAll]);

  const handleRefresh = async () => {
    if (refreshing || !apiReady) return;
    setRefreshing(true);
    try {
      await reloadAll();
      setPanelError(null);
    } catch (error) {
      const status = parseSdkProblemDetails(error)?.status;
      setPanelError(status === 404 ? 'not_initialized' : 'failed');
    } finally {
      setRefreshing(false);
    }
  };

  const handleLifecycle = async (action: 'activate' | 'pause') => {
    if (!publication || lifecycleBusy) return;
    setLifecycleBusy(action);
    try {
      const updated =
        action === 'activate'
          ? await activateWikiPublication(spaceId, publication.version)
          : await pauseWikiPublication(spaceId, publication.version);
      setPublication(updated);
    } catch (error) {
      toastKnowledgebaseError(error, t);
      if (parseSdkProblemDetails(error)?.status === 409) {
        await handleRefresh();
      }
    } finally {
      setLifecycleBusy(null);
    }
  };

  const runRowCommand = async (
    file: KnowledgeWikiSourceFile,
    command: 'publish_public' | 'publish_unlisted' | 'unpublish' | 'toggle_visibility',
  ) => {
    if (busyRowUuid || !publication) return;
    setBusyRowUuid(file.uuid);
    try {
      let result;
      if (command === 'publish_public' || command === 'publish_unlisted') {
        result = await publishWikiSourceFile(
          spaceId,
          file.uuid,
          command === 'publish_public' ? 'public' : 'unlisted',
          publication.version,
          file.version,
        );
      } else if (command === 'unpublish') {
        result = await unpublishWikiSourceFile(spaceId, file.uuid, publication.version, file.version);
      } else {
        const nextVisibility = file.visibility === 'public' ? 'unlisted' : 'public';
        result = await changeWikiSourceFileVisibility(
          spaceId,
          file.uuid,
          nextVisibility,
          publication.version,
          file.version,
        );
      }
      setPublication(result.publication);
      setSourceFiles((prev) =>
        prev.map((item) => (item.uuid === result.sourceFile.uuid ? result.sourceFile : item)),
      );
    } catch (error) {
      toastKnowledgebaseError(error, t);
      if (parseSdkProblemDetails(error)?.status === 409) {
        await handleRefresh();
      }
    } finally {
      setBusyRowUuid(null);
    }
  };

  const handleLoadMore = async () => {
    if (!filesHasMore || filesLoadingMore) return;
    setFilesLoadingMore(true);
    try {
      const page = await listWikiSourceFiles(spaceId, filesNextCursor, WIKI_SOURCE_FILES_PAGE_SIZE);
      applySourceFilePage(page, true);
    } catch (error) {
      toastKnowledgebaseError(error, t);
    } finally {
      setFilesLoadingMore(false);
    }
  };

  if (loading) {
    return (
      <div className="flex items-center justify-center py-16 text-[13px] font-semibold text-zinc-500 dark:text-[var(--color-kb-text-muted)]">
        <RefreshCw size={16} className="mr-2 animate-spin" />
        {t('wikiPanelLoading', { defaultValue: '正在加载 Wiki 发布状态...' })}
      </div>
    );
  }

  if (panelError === 'unavailable') {
    return (
      <div className="flex flex-col items-center justify-center py-16 space-y-2 text-center">
        <AlertCircle size={28} className="text-zinc-400" />
        <p className="text-[13px] font-semibold text-zinc-500 dark:text-[var(--color-kb-text-muted)]">
          {t('wikiPanelUnavailable', { defaultValue: '知识库 API 不可用，无法管理 Wiki 发布。' })}
        </p>
      </div>
    );
  }

  if (panelError === 'not_initialized') {
    return (
      <div className="flex flex-col items-center justify-center py-16 space-y-2 text-center">
        <BookOpen size={28} className="text-zinc-400" />
        <p className="text-[13px] font-semibold text-zinc-500 dark:text-[var(--color-kb-text-muted)]">
          {t('wikiPanelNotInitialized', { defaultValue: '此知识库尚未绑定 Wiki 站点发布。' })}
        </p>
      </div>
    );
  }

  if (panelError === 'failed' || publication === null) {
    return (
      <div className="flex flex-col items-center justify-center py-16 space-y-3 text-center">
        <AlertCircle size={28} className="text-red-400" />
        <p className="text-[13px] font-semibold text-zinc-500 dark:text-[var(--color-kb-text-muted)]">
          {t('wikiPanelFailed', { defaultValue: 'Wiki 发布状态加载失败，请稍后重试。' })}
        </p>
        <button
          type="button"
          onClick={handleRefresh}
          disabled={refreshing}
          className="px-4 py-2 rounded-xl border-2 border-zinc-200/80 dark:border-[var(--color-kb-panel-border)] text-[12.5px] font-bold text-zinc-600 dark:text-[var(--color-kb-text)] hover:bg-zinc-50 dark:hover:bg-[var(--color-kb-panel-hover)] transition-colors disabled:opacity-50"
        >
          {t('wikiPanelRetry', { defaultValue: '重新加载' })}
        </button>
      </div>
    );
  }

  const canActivate = WIKI_ACTIVATE_ELIGIBLE_STATUSES.has(publication.status);
  const canPause = WIKI_PAUSE_ELIGIBLE_STATUSES.has(publication.status);
  const canPublishRows = WIKI_PUBLISH_ELIGIBLE_STATUSES.has(publication.status);

  return (
    <div className="space-y-6 animate-in fade-in duration-300">
      {/* Publication status card */}
      <div className="p-5 rounded-2xl border-2 border-zinc-200/80 dark:border-[var(--color-kb-panel-border)] bg-white dark:bg-[var(--color-kb-panel)] shadow-sm space-y-4">
        <div className="flex items-start justify-between gap-4">
          <div className="flex items-center gap-3 min-w-0">
            <div className="w-10 h-10 rounded-xl bg-zinc-100 dark:bg-zinc-800 text-zinc-600 dark:text-zinc-300 flex items-center justify-center shrink-0">
              <BookOpen size={18} strokeWidth={2.5} />
            </div>
            <div className="min-w-0">
              <div className="flex items-center gap-2">
                <span className="text-[14px] font-extrabold text-zinc-900 dark:text-[var(--color-kb-text-heading)] truncate tracking-tight">
                  {publication.title}
                </span>
                <Badge className={STATUS_BADGE_CLASS[publication.status]}>
                  {t(`wikiPublicationStatus.${publication.status}`, { defaultValue: publication.status })}
                </Badge>
              </div>
              <div className="text-[11px] text-zinc-500 dark:text-[var(--color-kb-text-muted)] font-medium mt-0.5 truncate">
                {publication.homepageSourcePath}
              </div>
            </div>
          </div>
          <button
            type="button"
            onClick={handleRefresh}
            disabled={refreshing}
            title={t('wikiPanelRefresh', { defaultValue: '刷新状态' })}
            className="w-8 h-8 flex items-center justify-center rounded-lg text-zinc-400 hover:text-zinc-700 dark:hover:text-[var(--color-kb-text)] hover:bg-zinc-100 dark:hover:bg-[var(--color-kb-panel-hover)] transition-all disabled:opacity-50"
          >
            <RefreshCw size={15} className={refreshing ? 'animate-spin' : ''} />
          </button>
        </div>

        <div className="grid grid-cols-2 gap-x-6 gap-y-2 text-[11.5px] font-medium">
          <div className="flex justify-between gap-2">
            <span className="text-zinc-400 dark:text-[var(--color-kb-text-muted)]">
              {t('wikiPublicationVersion', { defaultValue: '发布版本' })}
            </span>
            <span className="font-mono text-zinc-700 dark:text-[var(--color-kb-text)] truncate">{publication.version}</span>
          </div>
          <div className="flex justify-between gap-2">
            <span className="text-zinc-400 dark:text-[var(--color-kb-text-muted)]">
              {t('wikiPublicationMode', { defaultValue: '发布模式' })}
            </span>
            <span className="text-zinc-700 dark:text-[var(--color-kb-text)]">
              {publication.publicationMode === 'review_required'
                ? t('wikiPublicationModeReview', { defaultValue: '审核后发布' })
                : t('wikiPublicationModeAuto', { defaultValue: '检查通过自动发布' })}
            </span>
          </div>
          <div className="flex justify-between gap-2">
            <span className="text-zinc-400 dark:text-[var(--color-kb-text-muted)]">
              {t('wikiDefaultVisibility', { defaultValue: '默认可见性' })}
            </span>
            <span className="text-zinc-700 dark:text-[var(--color-kb-text)]">{publication.defaultVisibility}</span>
          </div>
          <div className="flex justify-between gap-2">
            <span className="text-zinc-400 dark:text-[var(--color-kb-text-muted)]">
              {t('wikiUpdatePolicy', { defaultValue: '更新策略' })}
            </span>
            <span className="text-zinc-700 dark:text-[var(--color-kb-text)]">
              {publication.updatePolicy === 'keep_last_public_until_ready'
                ? t('wikiUpdatePolicyKeep', { defaultValue: '保持旧版直至就绪' })
                : t('wikiUpdatePolicyUnpublish', { defaultValue: '处理期间下线' })}
            </span>
          </div>
        </div>

        <div className="flex items-center justify-between pt-3 border-t border-zinc-100 dark:border-[var(--color-kb-panel-border)]">
          <div className="flex items-center gap-1.5 text-[10.5px] text-zinc-400 dark:text-[var(--color-kb-text-muted)] font-medium">
            <ShieldCheck size={13} />
            {t('wikiOptimisticLockHint', { defaultValue: '命令基于版本号提交，冲突时将自动刷新' })}
          </div>
          <div className="flex items-center gap-2">
            <button
              type="button"
              disabled={!canActivate || lifecycleBusy !== null}
              title={
                canActivate
                  ? t('wikiActivateTitle', { defaultValue: '激活 Wiki 站点' })
                  : t('wikiActivateIneligible', { defaultValue: '仅 ready 或 paused 状态可激活' })
              }
              onClick={() => handleLifecycle('activate')}
              className="px-4 py-2 rounded-xl bg-[var(--color-kb-accent)] text-white hover:bg-[var(--color-kb-accent-hover)] text-[12.5px] font-extrabold transition-all shadow-sm active:scale-95 flex items-center gap-1.5 disabled:opacity-40 disabled:pointer-events-none"
            >
              <Play size={13} strokeWidth={3} />
              {lifecycleBusy === 'activate'
                ? t('wikiActivating', { defaultValue: '激活中...' })
                : t('wikiActivate', { defaultValue: '激活' })}
            </button>
            <button
              type="button"
              disabled={!canPause || lifecycleBusy !== null}
              title={
                canPause
                  ? t('wikiPauseTitle', { defaultValue: '暂停对外发布' })
                  : t('wikiPauseIneligible', { defaultValue: '仅 active 或 degraded 状态可暂停' })
              }
              onClick={() => handleLifecycle('pause')}
              className="px-4 py-2 rounded-xl border-2 border-zinc-200/80 dark:border-[var(--color-kb-panel-border)] text-[12.5px] font-bold text-zinc-600 dark:text-[var(--color-kb-text)] hover:bg-zinc-50 dark:hover:bg-[var(--color-kb-panel-hover)] transition-colors active:scale-95 flex items-center gap-1.5 disabled:opacity-40 disabled:pointer-events-none"
            >
              <Pause size={13} strokeWidth={3} />
              {lifecycleBusy === 'pause'
                ? t('wikiPausing', { defaultValue: '暂停中...' })
                : t('wikiPause', { defaultValue: '暂停' })}
            </button>
          </div>
        </div>
      </div>

      {/* Source files table */}
      <div className="space-y-3">
        <div className="flex items-center justify-between">
          <div className="text-[14px] font-extrabold text-zinc-900 dark:text-[var(--color-kb-text-heading)] flex items-center gap-2">
            {t('wikiSourceFilesTitle', { defaultValue: '源文件发布管理' })}
            <span className="text-[11px] font-bold bg-zinc-900 text-white dark:bg-white dark:text-zinc-900 px-2 py-0.5 rounded-full">
              {sourceFiles.length}
            </span>
          </div>
          {!canPublishRows && (
            <span className="text-[10.5px] text-amber-600 dark:text-amber-400 font-bold flex items-center gap-1">
              <AlertCircle size={12} />
              {t('wikiPublishBlockedByStatus', { defaultValue: '站点当前状态不允许发布页面' })}
            </span>
          )}
        </div>

        <div className="border-2 border-zinc-200/80 dark:border-[var(--color-kb-panel-border)] rounded-2xl overflow-hidden bg-white dark:bg-[var(--color-kb-editor)] shadow-sm">
          <table className="w-full text-left">
            <thead>
              <tr className="bg-[#fafafa] dark:bg-[var(--color-kb-panel)] border-b border-zinc-200/80 dark:border-[var(--color-kb-panel-border)]">
                {[
                  t('wikiColumnSourcePath', { defaultValue: '源路径' }),
                  t('wikiColumnRoute', { defaultValue: '路由' }),
                  t('wikiColumnSourceState', { defaultValue: '源状态' }),
                  t('wikiColumnPublication', { defaultValue: '发布' }),
                  t('wikiColumnVisibility', { defaultValue: '可见性' }),
                  t('wikiColumnIndex', { defaultValue: '索引' }),
                  t('wikiColumnActions', { defaultValue: '操作' }),
                ].map((heading) => (
                  <th
                    key={heading}
                    className="px-3 py-2.5 text-[10.5px] font-extrabold uppercase tracking-wider text-zinc-400 dark:text-[var(--color-kb-text-muted)]"
                  >
                    {heading}
                  </th>
                ))}
              </tr>
            </thead>
            <tbody className="divide-y divide-zinc-100 dark:divide-[var(--color-kb-panel-border)]">
              {sourceFiles.length === 0 ? (
                <tr>
                  <td colSpan={7} className="px-4 py-10 text-center text-[12.5px] font-semibold text-zinc-400 dark:text-[var(--color-kb-text-muted)]">
                    {t('wikiSourceFilesEmpty', { defaultValue: '暂无投影的源文件' })}
                  </td>
                </tr>
              ) : (
                sourceFiles.map((file) => {
                  const rowBusy = busyRowUuid === file.uuid;
                  const publishable = isWikiPublishRowEligible(file);
                  const published = file.publicationState === 'published';
                  return (
                    <tr key={file.uuid} className="hover:bg-[#fafafa] dark:hover:bg-zinc-900/40 transition-colors">
                      <td className="px-3 py-2.5 max-w-[180px]">
                        <div className="text-[12px] font-bold text-zinc-800 dark:text-[var(--color-kb-text)] truncate" title={file.sourcePath}>
                          {file.sourcePath}
                        </div>
                      </td>
                      <td className="px-3 py-2.5 max-w-[140px]">
                        <div className="text-[11px] font-mono text-zinc-500 dark:text-[var(--color-kb-text-muted)] truncate" title={file.canonicalRoute ?? ''}>
                          {file.canonicalRoute ?? '—'}
                        </div>
                      </td>
                      <td className="px-3 py-2.5">
                        <Badge className={SOURCE_STATE_CLASS[file.sourceState] ?? SOURCE_STATE_CLASS.discovered}>
                          {file.sourceState}
                        </Badge>
                      </td>
                      <td className="px-3 py-2.5">
                        <span className="text-[11.5px] font-semibold text-zinc-600 dark:text-[var(--color-kb-text)]">
                          {PAGE_PUBLICATION_STATE_LABEL[file.publicationState] ?? file.publicationState}
                        </span>
                      </td>
                      <td className="px-3 py-2.5">
                        <span className="text-[11.5px] font-semibold text-zinc-600 dark:text-[var(--color-kb-text)]">
                          {file.visibility}
                        </span>
                      </td>
                      <td className="px-3 py-2.5">
                        <Badge className={INDEX_STATE_CLASS[file.indexState] ?? INDEX_STATE_CLASS.pending}>
                          {file.indexState}
                        </Badge>
                      </td>
                      <td className="px-3 py-2.5">
                        <div className="flex items-center gap-1.5">
                          {!published && (
                            <>
                              <button
                                type="button"
                                disabled={!canPublishRows || !publishable || busyRowUuid !== null}
                                title={
                                  publishable
                                    ? t('wikiPublishPublicTitle', { defaultValue: '发布为公开页面' })
                                    : t('wikiRowNotPublishable', { defaultValue: '源状态、路由或索引未就绪，暂不可发布' })
                                }
                                onClick={() => runRowCommand(file, 'publish_public')}
                                className="px-2.5 py-1.5 rounded-lg text-[11px] font-bold bg-emerald-600 text-white hover:bg-emerald-700 transition-all active:scale-95 disabled:opacity-40 disabled:pointer-events-none flex items-center gap-1"
                              >
                                <Globe size={11} strokeWidth={2.5} />
                                {t('wikiPublishPublic', { defaultValue: '公开' })}
                              </button>
                              <button
                                type="button"
                                disabled={!canPublishRows || !publishable || busyRowUuid !== null}
                                title={
                                  publishable
                                    ? t('wikiPublishUnlistedTitle', { defaultValue: '以链接可见方式发布' })
                                    : t('wikiRowNotPublishable', { defaultValue: '源状态、路由或索引未就绪，暂不可发布' })
                                }
                                onClick={() => runRowCommand(file, 'publish_unlisted')}
                                className="px-2.5 py-1.5 rounded-lg text-[11px] font-bold border border-zinc-200 dark:border-[var(--color-kb-panel-border)] text-zinc-600 dark:text-[var(--color-kb-text)] hover:bg-zinc-50 dark:hover:bg-[var(--color-kb-panel-hover)] transition-all active:scale-95 disabled:opacity-40 disabled:pointer-events-none flex items-center gap-1"
                              >
                                <Link2 size={11} strokeWidth={2.5} />
                                {t('wikiPublishUnlisted', { defaultValue: '链接' })}
                              </button>
                            </>
                          )}
                          {published && (
                            <>
                              <button
                                type="button"
                                disabled={busyRowUuid !== null}
                                onClick={() => runRowCommand(file, 'toggle_visibility')}
                                title={t('wikiToggleVisibilityTitle', { defaultValue: '在公开与链接可见之间切换' })}
                                className="px-2.5 py-1.5 rounded-lg text-[11px] font-bold border border-zinc-200 dark:border-[var(--color-kb-panel-border)] text-zinc-600 dark:text-[var(--color-kb-text)] hover:bg-zinc-50 dark:hover:bg-[var(--color-kb-panel-hover)] transition-all active:scale-95 disabled:opacity-40 disabled:pointer-events-none flex items-center gap-1"
                              >
                                {file.visibility === 'public' ? (
                                  <>
                                    <Link2 size={11} strokeWidth={2.5} />
                                    {t('wikiMakeUnlisted', { defaultValue: '转链接' })}
                                  </>
                                ) : (
                                  <>
                                    <Globe size={11} strokeWidth={2.5} />
                                    {t('wikiMakePublic', { defaultValue: '转公开' })}
                                  </>
                                )}
                              </button>
                              <button
                                type="button"
                                disabled={busyRowUuid !== null}
                                onClick={() => runRowCommand(file, 'unpublish')}
                                title={t('wikiUnpublishTitle', { defaultValue: '下线该页面' })}
                                className="px-2.5 py-1.5 rounded-lg text-[11px] font-bold text-red-500 hover:bg-red-50 dark:hover:bg-red-950/40 transition-all active:scale-95 disabled:opacity-40 disabled:pointer-events-none flex items-center gap-1"
                              >
                                <EyeOff size={11} strokeWidth={2.5} />
                                {t('wikiUnpublish', { defaultValue: '下线' })}
                              </button>
                            </>
                          )}
                        </div>
                      </td>
                    </tr>
                  );
                })
              )}
            </tbody>
          </table>
        </div>

        {filesHasMore ? (
          <div className="flex justify-center pt-1">
            <button
              type="button"
              disabled={filesLoadingMore}
              onClick={handleLoadMore}
              className="px-4 py-2 text-[12.5px] font-medium rounded-lg border border-zinc-200 dark:border-[var(--color-kb-panel-border)] text-zinc-600 dark:text-[var(--color-kb-text)] hover:bg-zinc-50 dark:hover:bg-[var(--color-kb-panel-hover)] disabled:opacity-50 flex items-center gap-1.5 transition-colors"
            >
              <ChevronDown size={14} className={filesLoadingMore ? 'animate-bounce' : ''} />
              {filesLoadingMore
                ? t('loading', { ns: 'common', defaultValue: '加载中...' })
                : t('wikiLoadMoreFiles', { defaultValue: '加载更多源文件' })}
            </button>
          </div>
        ) : null}
      </div>
    </div>
  );
}
