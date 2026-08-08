import React, { useState, useEffect, useMemo, useCallback } from 'react';
import { isBlank } from '@sdkwork/utils';
import { createPortal } from 'react-dom';
import { useTranslation } from 'react-i18next';
import {
  X, Cloud, Folder, FileText,
  FileSpreadsheet, ChevronRight, Search, LayoutGrid,
  List, CheckSquare, Square, Home, Users,
  Clock, Star, RefreshCw, Layers, ServerIcon, Download, AlertCircle, Video
} from 'lucide-react';

import {
  CloudDriveService,
  type CloudDriveBrowserItemsPage,
  type CloudDriveBrowserItem,
  type CloudDriveImportResultItem,
} from './services/cloudDriveService';

interface BreadcrumbItem {
  id: string;
  name: string;
}

interface CloudDriveModalProps {
  isOpen: boolean;
  onClose: () => void;
  spaceId?: string | null;
  targetParentFolderId?: string | null;
  onConfirm: (selectedItems: CloudDriveImportResultItem[]) => void;
}

const renderFileIcon = (item: CloudDriveBrowserItem) => {
  if (item.type === 'folder') {
    return (
      <div className="p-1.5 bg-amber-500/10 text-amber-600 dark:text-amber-500 rounded-lg shrink-0">
        <Folder size={16} />
      </div>
    );
  }

  const nameLower = item.name.toLowerCase();
  const mime = item.mimeType ?? '';
  if (nameLower.endsWith('.md') || mime.includes('markdown')) {
    return (
      <div className="p-1.5 bg-emerald-500/10 text-emerald-600 dark:text-emerald-500 rounded-lg shrink-0">
        <FileText size={16} />
      </div>
    );
  }
  if (mime.includes('html') || nameLower.endsWith('.html')) {
    return (
      <div className="p-1.5 bg-blue-500/10 text-blue-600 dark:text-blue-500 rounded-lg shrink-0">
        <FileSpreadsheet size={16} />
      </div>
    );
  }
  if (nameLower.endsWith('.pdf') || mime.includes('pdf')) {
    return (
      <div className="p-1.5 bg-red-500/10 text-red-600 dark:text-red-500 rounded-lg shrink-0">
        <FileText size={16} />
      </div>
    );
  }
  if (nameLower.endsWith('.zip') || nameLower.endsWith('.rar')) {
    return (
      <div className="p-1.5 bg-purple-500/10 text-purple-600 dark:text-purple-500 rounded-lg shrink-0">
        <Layers size={16} />
      </div>
    );
  }
  if (nameLower.endsWith('.mp4') || nameLower.endsWith('.mov') || mime.startsWith('video/')) {
    return (
      <div className="p-1.5 bg-orange-500/10 text-orange-600 dark:text-orange-500 rounded-lg shrink-0">
        <Video size={16} />
      </div>
    );
  }

  return (
    <div className="p-1.5 bg-zinc-500/10 text-zinc-600 dark:text-zinc-500 rounded-lg shrink-0">
      <FileSpreadsheet size={16} />
    </div>
  );
};

const renderGridIcon = (item: CloudDriveBrowserItem) => {
  if (item.type === 'folder') {
    return <Folder size={32} className="text-amber-500 fill-amber-500/10 mb-2 shrink-0" />;
  }
  const nameLower = item.name.toLowerCase();
  if (nameLower.endsWith('.md')) {
    return <FileText size={32} className="text-emerald-500 mb-2 shrink-0" />;
  }
  if (nameLower.endsWith('.pdf')) {
    return <FileText size={32} className="text-red-500 mb-2 shrink-0" />;
  }
  if (nameLower.endsWith('.zip')) {
    return <Layers size={32} className="text-purple-500 mb-2 shrink-0" />;
  }
  if (nameLower.endsWith('.mp4') || nameLower.endsWith('.mov')) {
    return <Video size={32} className="text-orange-500 mb-2 shrink-0" />;
  }
  return <FileSpreadsheet size={32} className="text-zinc-500 mb-2 shrink-0" />;
};

function formatDriveUpdatedAt(value: string | null | undefined): string {
  return value?.trim() || '--';
}

export function CloudDriveModal({
  isOpen,
  onClose,
  spaceId,
  targetParentFolderId,
  onConfirm,
}: CloudDriveModalProps) {
  const { t } = useTranslation('cloudDrive');
  const [activeTab, setActiveTab] = useState<'my-drive' | 'shared' | 'recent' | 'starred'>('my-drive');
  const [searchQuery, setSearchQuery] = useState('');
  const [viewMode, setViewMode] = useState<'list' | 'grid'>('list');
  const [currentFolderId, setCurrentFolderId] = useState<string | null>(null);
  const [breadcrumbs, setBreadcrumbs] = useState<BreadcrumbItem[]>([]);
  const [items, setItems] = useState<CloudDriveBrowserItem[]>([]);
  const [itemIndex, setItemIndex] = useState<Map<string, CloudDriveBrowserItem>>(new Map());
  const [selectedIds, setSelectedIds] = useState<Set<string>>(new Set());
  const [isSyncing, setIsSyncing] = useState(false);
  const [isLoading, setIsLoading] = useState(false);
  const [loadingMore, setLoadingMore] = useState(false);
  const [driveNextCursor, setDriveNextCursor] = useState<string | null>(null);
  const [driveHasMore, setDriveHasMore] = useState(false);
  const [loadError, setLoadError] = useState<string | null>(null);
  const [feedbackMsg, setFeedbackMsg] = useState<string | null>(null);

  const loadDriveItems = useCallback(async () => {
    if (!spaceId || isBlank(spaceId)) {
      setItems([]);
      setLoadError(t('spaceRequired', { defaultValue: 'Knowledge space is required to browse enterprise drive.' }));
      return;
    }

    setIsLoading(true);
    setLoadError(null);
    try {
      let page: CloudDriveBrowserItemsPage;
      switch (activeTab) {
        case 'starred':
          page = await CloudDriveService.listStarredItemsPage(spaceId);
          break;
        case 'recent':
          page = await CloudDriveService.listRecentItemsPage(spaceId);
          break;
        case 'shared':
          page = await CloudDriveService.listSharedItemsPage(spaceId);
          break;
        default:
          page = await CloudDriveService.listBrowserItemsPage(spaceId, currentFolderId);
          break;
      }
      const nextItems = page.items;
      setDriveNextCursor(page.nextCursor);
      setDriveHasMore(page.hasMore);
      setItems(nextItems);
      setItemIndex(() => {
        const nextIndex = new Map<string, CloudDriveBrowserItem>();
        for (const item of nextItems) {
          nextIndex.set(item.id, item);
        }
        return nextIndex;
      });
    } catch (error) {
      console.error('[CloudDriveModal] failed to list drive browser items', error);
      setItems([]);
      setLoadError(t('loadFailed', { defaultValue: 'Failed to load enterprise drive files.' }));
    } finally {
      setIsLoading(false);
    }
  }, [spaceId, currentFolderId, activeTab, t]);

  useEffect(() => {
    setSelectedIds(new Set());
  }, [activeTab, currentFolderId]);

  useEffect(() => {
    if (!isOpen) {
      return;
    }
    void loadDriveItems();
  }, [isOpen, activeTab, loadDriveItems]);

  useEffect(() => {
    if (!isOpen) {
      const timer = window.setTimeout(() => {
        setActiveTab('my-drive');
        setCurrentFolderId(null);
        setBreadcrumbs([]);
        setSelectedIds(new Set());
        setSearchQuery('');
        setFeedbackMsg(null);
        setIsSyncing(false);
        setLoadError(null);
        setItems([]);
        setItemIndex(new Map());
        setDriveNextCursor(null);
        setDriveHasMore(false);
        setLoadingMore(false);
      }, 200);
      return () => window.clearTimeout(timer);
    }
    return undefined;
  }, [isOpen]);

  const enterFolder = (item: CloudDriveBrowserItem) => {
    setCurrentFolderId(item.id);
    setBreadcrumbs((previous) => [...previous, { id: item.id, name: item.name }]);
  };

  const navigateToBreadcrumb = (index: number) => {
    if (index < 0) {
      setCurrentFolderId(null);
      setBreadcrumbs([]);
      return;
    }
    const next = breadcrumbs.slice(0, index + 1);
    setBreadcrumbs(next);
    setCurrentFolderId(next[next.length - 1]?.id ?? null);
  };

  const displayedFiles = useMemo(() => {
    if (isBlank(searchQuery)) {
      return items;
    }
    const query = searchQuery.toLowerCase().trim();
    return items.filter((file) => file.name.toLowerCase().includes(query));
  }, [items, searchQuery]);

  const handleToggleSelect = (item: CloudDriveBrowserItem, e: React.MouseEvent) => {
    e.stopPropagation();
    const next = new Set(selectedIds);
    if (next.has(item.id)) {
      next.delete(item.id);
    } else {
      next.add(item.id);
    }
    setSelectedIds(next);
  };

  const handleSelectAll = () => {
    if (selectedIds.size === displayedFiles.length) {
      setSelectedIds(new Set());
    } else {
      setSelectedIds(new Set(displayedFiles.map((item) => item.id)));
    }
  };

  const getSelectedItemObjects = (): CloudDriveBrowserItem[] => {
    const selected: CloudDriveBrowserItem[] = [];
    for (const id of selectedIds) {
      const item = itemIndex.get(id);
      if (item) {
        selected.push(item);
      }
    }
    return selected;
  };

  const handleImportClick = async () => {
    const objects = getSelectedItemObjects().filter((item) => item.type !== 'folder');
    if (objects.length === 0 || !spaceId) {
      return;
    }

    setIsSyncing(true);
    setFeedbackMsg(t('syncingFeedback'));

    try {
      const mapped = await CloudDriveService.importItems(spaceId, objects, targetParentFolderId);
      onConfirm(mapped);
      onClose();
    } catch (error) {
      console.error('[CloudDriveModal] drive import failed', error);
      setFeedbackMsg(t('importFailed', { defaultValue: 'Drive import failed. Please retry.' }));
      setIsSyncing(false);
    }
  };

  const handleLoadMore = async () => {
    if (
      !spaceId
      || !driveHasMore
      || loadingMore
      || !driveNextCursor
      || isBlank(spaceId)
    ) {
      return;
    }

    setLoadingMore(true);
    try {
      let page: CloudDriveBrowserItemsPage;
      switch (activeTab) {
        case 'starred':
          page = await CloudDriveService.listStarredItemsPage(spaceId, driveNextCursor);
          break;
        case 'recent':
          page = await CloudDriveService.listRecentItemsPage(spaceId, driveNextCursor);
          break;
        case 'shared':
          page = await CloudDriveService.listSharedItemsPage(spaceId, driveNextCursor);
          break;
        default:
          page = await CloudDriveService.listBrowserItemsPage(spaceId, currentFolderId, driveNextCursor);
          break;
      }
      setItems((previous) => [...previous, ...page.items]);
      setDriveNextCursor(page.nextCursor);
      setDriveHasMore(page.hasMore);
      setItemIndex((previous) => {
        const merged = new Map(previous);
        for (const item of page.items) {
          merged.set(item.id, item);
        }
        return merged;
      });
    } catch (error) {
      console.error('[CloudDriveModal] failed to load more drive browser items', error);
      setLoadError(t('loadFailed', { defaultValue: 'Failed to load enterprise drive files.' }));
    } finally {
      setLoadingMore(false);
    }
  };

  if (!isOpen) {
    return null;
  }

  const showFolderNavigation = activeTab === 'my-drive';
  const currentSelectableFiles = displayedFiles.filter((item) => item.type !== 'folder');

  return createPortal(
    <div className="fixed inset-0 z-[300] bg-zinc-950/40 flex items-center justify-center backdrop-blur-md p-4">
      <div className="w-[1400px] h-[896px] max-w-[95vw] max-h-[95vh] bg-[var(--color-kb-editor)] rounded-2xl shadow-[0_24px_64px_-16px_rgba(0,0,0,0.25)] border border-[var(--color-kb-panel-border)] flex flex-col overflow-hidden animate-in fade-in zoom-in-95 duration-200">

        <div className="h-16 border-b border-zinc-200/80 dark:border-[var(--color-kb-panel-border)] flex items-center justify-between px-6 bg-[#fafafa] dark:bg-[var(--color-kb-panel)]/30 shrink-0 z-10 shadow-sm">
          <div className="flex items-center gap-3">
            <div className="p-2 bg-gradient-to-tr from-cyan-50 dark:from-cyan-500/20 to-blue-50 dark:to-blue-500/20 border border-cyan-100 dark:border-transparent text-cyan-600 dark:text-cyan-500 rounded-xl shadow-inner">
              <Cloud size={20} strokeWidth={2.5} className="animate-pulse" />
            </div>
            <div>
              <h3 className="text-[15px] font-extrabold tracking-tight text-zinc-900 dark:text-[var(--color-kb-text-heading)] leading-tight">{t('myEnterpriseDrive')}</h3>
              <p className="text-[11.5px] font-medium text-zinc-500 dark:text-[var(--color-kb-text-muted)] tracking-wide">{t('driveDescription')}</p>
            </div>
          </div>
          <button
            onClick={onClose}
            className="text-zinc-400 hover:text-red-500 hover:bg-red-50 dark:text-[var(--color-kb-text-muted)] dark:hover:bg-red-500/10 p-2 rounded-xl transition-all active:scale-95"
          >
            <X size={16} strokeWidth={2.5} />
          </button>
        </div>

        <div className="h-14 border-b border-[var(--color-kb-panel-border)] bg-[var(--color-kb-panel)]/10 px-6 flex items-center justify-between gap-4">
          <div className="flex items-center gap-1 text-[13px] text-[var(--color-kb-text-muted)] overflow-hidden">
            {showFolderNavigation ? (
              <>
                <button
                  onClick={() => navigateToBreadcrumb(-1)}
                  className={`hover:text-[var(--color-kb-accent)] flex items-center gap-1.5 transition-colors shrink-0 font-medium ${!currentFolderId ? 'text-[var(--color-kb-text-heading)] font-semibold' : ''}`}
                >
                  <Home size={15} />
                  <span>{t('myDrive')}</span>
                </button>

                {breadcrumbs.map((crumb, idx) => (
                  <React.Fragment key={crumb.id}>
                    <ChevronRight size={14} className="opacity-60 shrink-0" />
                    <button
                      onClick={() => navigateToBreadcrumb(idx)}
                      className={`hover:text-[var(--color-kb-accent)] transition-colors truncate max-w-[120px] font-medium ${idx === breadcrumbs.length - 1 ? 'text-[var(--color-kb-text-heading)] font-semibold' : ''}`}
                    >
                      {crumb.name}
                    </button>
                  </React.Fragment>
                ))}
              </>
            ) : (
              <span className="text-[var(--color-kb-text-heading)] font-semibold">
                {activeTab === 'shared' ? t('sharedWithMe') : activeTab === 'recent' ? t('recentAccess') : t('starredFiles')}
              </span>
            )}
          </div>

          <div className="flex items-center gap-3 w-[360px]">
            <div className="flex-1 flex items-center bg-[var(--color-kb-panel)] border border-[var(--color-kb-panel-border)] hover:border-[var(--color-kb-accent)]/50 focus-within:ring-1 focus-within:ring-[var(--color-kb-accent)] focus-within:border-[var(--color-kb-accent)] px-3 py-1.5 rounded-xl transition-all h-9">
              <Search size={14} className="text-[var(--color-kb-text-muted)] mr-2 shrink-0" />
              <input
                type="text"
                placeholder={t('searchPlaceholder')}
                value={searchQuery}
                onChange={(e) => setSearchQuery(e.target.value)}
                className="bg-transparent border-none outline-none text-[13px] font-medium text-[var(--color-kb-text-heading)] placeholder-[var(--color-kb-text-muted)] w-full focus:ring-0 focus:outline-none focus:border-none"
              />
              {searchQuery && (
                <button
                  onClick={() => setSearchQuery('')}
                  className="text-[var(--color-kb-text-muted)] hover:text-[var(--color-kb-accent)] p-0.5 shrink-0"
                >
                  <X size={12} />
                </button>
              )}
            </div>

            <div className="flex items-center bg-[var(--color-kb-panel)] border border-[var(--color-kb-panel-border)] rounded-xl p-0.5 h-9 shrink-0 shadow-inner">
              <button
                onClick={() => setViewMode('list')}
                className={`p-1.5 rounded-lg transition-all ${viewMode === 'list' ? 'bg-[var(--color-kb-editor)] text-[var(--color-kb-accent)] shadow-sm' : 'text-[var(--color-kb-text-muted)] hover:text-current'}`}
                title={t('listView')}
              >
                <List size={14} />
              </button>
              <button
                onClick={() => setViewMode('grid')}
                className={`p-1.5 rounded-lg transition-all ${viewMode === 'grid' ? 'bg-[var(--color-kb-editor)] text-[var(--color-kb-accent)] shadow-sm' : 'text-[var(--color-kb-text-muted)] hover:text-current'}`}
                title={t('gridView')}
              >
                <LayoutGrid size={14} />
              </button>
            </div>
          </div>
        </div>

        <div className="flex-1 flex min-h-0">
          <div className="w-[180px] border-r border-[var(--color-kb-panel-border)] bg-[var(--color-kb-panel)]/20 p-3 space-y-1 shrink-0 flex flex-col justify-between">
            <div className="space-y-1">
              <button
                onClick={() => { setActiveTab('my-drive'); setCurrentFolderId(null); setBreadcrumbs([]); }}
                className={`w-full flex items-center gap-3 px-3 py-2.5 rounded-xl text-[13px] font-semibold transition-all ${activeTab === 'my-drive' ? 'bg-[var(--color-kb-accent)]/10 text-[var(--color-kb-accent)] shadow-sm' : 'text-[var(--color-kb-text)] hover:bg-[var(--color-kb-panel-hover)]'}`}
              >
                <ServerIcon size={16} />
                <span>{t('myFiles')}</span>
              </button>
              <button
                onClick={() => { setActiveTab('shared'); setCurrentFolderId(null); setBreadcrumbs([]); }}
                className={`w-full flex items-center gap-3 px-3 py-2.5 rounded-xl text-[13px] font-semibold transition-all ${activeTab === 'shared' ? 'bg-[var(--color-kb-accent)]/10 text-[var(--color-kb-accent)] shadow-sm' : 'text-[var(--color-kb-text)] hover:bg-[var(--color-kb-panel-hover)]'}`}
              >
                <Users size={16} />
                <span>{t('sharedWithMe')}</span>
              </button>
              <button
                onClick={() => { setActiveTab('recent'); setCurrentFolderId(null); setBreadcrumbs([]); }}
                className={`w-full flex items-center gap-3 px-3 py-2.5 rounded-xl text-[13px] font-semibold transition-all ${activeTab === 'recent' ? 'bg-[var(--color-kb-accent)]/10 text-[var(--color-kb-accent)] shadow-sm' : 'text-[var(--color-kb-text)] hover:bg-[var(--color-kb-panel-hover)]'}`}
              >
                <Clock size={16} />
                <span>{t('recentAccess')}</span>
              </button>
              <button
                onClick={() => { setActiveTab('starred'); setCurrentFolderId(null); setBreadcrumbs([]); }}
                className={`w-full flex items-center gap-3 px-3 py-2.5 rounded-xl text-[13px] font-semibold transition-all ${activeTab === 'starred' ? 'bg-[var(--color-kb-accent)]/10 text-[var(--color-kb-accent)] shadow-sm' : 'text-[var(--color-kb-text)] hover:bg-[var(--color-kb-panel-hover)]'}`}
              >
                <Star size={16} />
                <span>{t('starredFiles')}</span>
              </button>
            </div>
          </div>

          <div className="flex-1 flex flex-col bg-[var(--color-kb-editor)] relative overflow-hidden min-w-0">
            {isSyncing ? (
              <div className="absolute inset-0 z-50 bg-[var(--color-kb-editor)]/90 backdrop-blur-sm flex flex-col items-center justify-center text-center p-6">
                <RefreshCw size={42} className="text-[var(--color-kb-accent)] animate-spin mb-4" />
                <h4 className="text-[15px] font-bold text-[var(--color-kb-text-heading)] mb-1">{t('syncingTitle')}</h4>
                <p className="text-[12px] text-[var(--color-kb-text-muted)] max-w-sm">{feedbackMsg}</p>
              </div>
            ) : null}

            {isLoading ? (
              <div className="flex-1 flex flex-col items-center justify-center p-8 text-center">
                <RefreshCw size={32} className="text-[var(--color-kb-accent)] animate-spin mb-3" />
                <p className="text-[12px] text-[var(--color-kb-text-muted)]">{t('loading', { defaultValue: 'Loading drive files...' })}</p>
              </div>
            ) : loadError ? (
              <div className="flex-1 flex flex-col items-center justify-center p-8 text-center">
                <AlertCircle size={32} className="text-amber-500 mb-3" />
                <p className="text-[12px] text-[var(--color-kb-text-muted)]">{loadError}</p>
              </div>
            ) : displayedFiles.length === 0 ? (
              <div className="flex-1 flex flex-col items-center justify-center p-8 text-center select-none">
                <FileText size={40} className="text-[var(--color-kb-text-muted)] opacity-60 mb-3" />
                <h4 className="text-[14px] font-bold text-[var(--color-kb-text-heading)]">{t('emptyFolderTitle')}</h4>
                <p className="text-[12px] text-[var(--color-kb-text-muted)] mt-1">{t('emptyFolderDesc')}</p>
              </div>
            ) : viewMode === 'list' ? (
              <div className="flex-1 overflow-y-auto w-full">
                <table className="w-full text-left text-[12px] border-collapse relative">
                  <thead className="sticky top-0 bg-[var(--color-kb-editor)] shadow-[0_1px_0_0_rgba(0,0,0,0.05)] border-b border-[var(--color-kb-panel-border)] z-10 select-none">
                    <tr className="text-[var(--color-kb-text-muted)] font-semibold">
                      <th className="w-12 pl-6 py-3">
                        {currentSelectableFiles.length > 0 && (
                          <button
                            onClick={handleSelectAll}
                            className="p-1 hover:bg-[var(--color-kb-panel-hover)] rounded-md text-[var(--color-kb-accent)] transition-all"
                            title={t('selectAll')}
                          >
                            {selectedIds.size === currentSelectableFiles.length ? (
                              <CheckSquare size={16} />
                            ) : (
                              <Square size={16} />
                            )}
                          </button>
                        )}
                      </th>
                      <th className="py-3 font-semibold text-[13px] text-[var(--color-kb-text-heading)]">{t('name')}</th>
                      <th className="py-3 w-32 font-semibold">{t('updatedAt')}</th>
                      <th className="py-3 w-24 font-semibold pr-6 text-right">{t('size')}</th>
                    </tr>
                  </thead>
                  <tbody className="divide-y divide-[var(--color-kb-panel-border)]/50">
                    {displayedFiles.map((item) => {
                      const isFolder = item.type === 'folder';
                      const isSelected = selectedIds.has(item.id);
                      return (
                        <tr
                          key={item.id}
                          onClick={() => {
                            if (isFolder && showFolderNavigation) {
                              enterFolder(item);
                            }
                          }}
                          className={`hover:bg-[var(--color-kb-panel-hover)]/60 transition-colors group cursor-pointer ${isSelected ? 'bg-[var(--color-kb-accent)]/5' : ''}`}
                        >
                          <td className="pl-6 py-3" onClick={(e) => e.stopPropagation()}>
                            <button
                              onClick={(e) => handleToggleSelect(item, e)}
                              className="p-1 hover:text-[var(--color-kb-accent)] transition-all"
                            >
                              {isSelected ? (
                                <CheckSquare size={15} className="text-[var(--color-kb-accent)]" />
                              ) : (
                                <Square size={15} className="text-[var(--color-kb-text-muted)]/70 group-hover:text-[var(--color-kb-text-muted)]" />
                              )}
                            </button>
                          </td>
                          <td className="py-3 pr-4 font-medium text-[13px] text-[var(--color-kb-text-heading)]">
                            <div className="flex items-center gap-3">
                              {renderFileIcon(item)}
                              <span className="truncate max-w-[340px] group-hover:text-[var(--color-kb-accent)] transition-colors">{item.name}</span>
                              {isFolder && (
                                <span className="ml-auto opacity-0 group-hover:opacity-100 flex items-center text-[10px] text-[var(--color-kb-accent)] font-semibold gap-0.5 shrink-0 transition-all">
                                  <span>{t('clickToEnter')}</span>
                                  <ChevronRight size={10} />
                                </span>
                              )}
                            </div>
                          </td>
                          <td className="py-3 text-[var(--color-kb-text-muted)] font-mono">{formatDriveUpdatedAt(item.updatedAt)}</td>
                          <td className="py-3 pr-6 text-right text-[var(--color-kb-text-muted)] font-mono">{item.size || t('folderSize')}</td>
                        </tr>
                      );
                    })}
                  </tbody>
                </table>
              </div>
            ) : (
              <div className="flex-1 overflow-y-auto p-6">
                <div className="grid grid-cols-4 gap-4">
                  {displayedFiles.map((item) => {
                    const isFolder = item.type === 'folder';
                    const isSelected = selectedIds.has(item.id);
                    return (
                      <div
                        key={item.id}
                        onClick={() => {
                          if (isFolder && showFolderNavigation) {
                            enterFolder(item);
                          }
                        }}
                        className={`border border-[var(--color-kb-panel-border)] rounded-2xl p-4 flex flex-col justify-between hover:border-[var(--color-kb-accent)] hover:shadow-md transition-all cursor-pointer group h-[120px] relative ${isSelected ? 'bg-[var(--color-kb-accent)]/[0.04] !border-[var(--color-kb-accent)]' : 'bg-[var(--color-kb-panel)]/10'}`}
                      >
                        <button
                          onClick={(e) => handleToggleSelect(item, e)}
                          className="absolute top-3 left-3 opacity-0 group-hover:opacity-100 focus:opacity-100 transition-opacity"
                        >
                          {isSelected ? (
                            <CheckSquare size={16} className="text-[var(--color-kb-accent)] opacity-100" />
                          ) : (
                            <Square size={16} className="text-[var(--color-kb-text-muted)]" />
                          )}
                        </button>

                        <div className="flex flex-col items-center justify-center text-center py-1">
                          {renderGridIcon(item)}
                          <p className="text-[12px] font-semibold text-[var(--color-kb-text-heading)] truncate max-w-[150px] w-full px-1">{item.name}</p>
                        </div>

                        <div className="flex items-center justify-between mt-2 pt-2 border-t border-[var(--color-kb-panel-border)]/50 text-[10px] text-[var(--color-kb-text-muted)] font-mono leading-none">
                          <span>{formatDriveUpdatedAt(item.updatedAt)}</span>
                          <span>{item.size || t('folderSize')}</span>
                        </div>
                      </div>
                    );
                  })}
                </div>
              </div>
            )}
            {driveHasMore && !isLoading && !loadError ? (
              <div className="shrink-0 border-t border-[var(--color-kb-panel-border)]/50 px-6 py-3 flex justify-center bg-[var(--color-kb-editor)]">
                <button
                  type="button"
                  onClick={() => { void handleLoadMore(); }}
                  disabled={loadingMore}
                  className="px-5 py-2 text-[12px] font-semibold text-[var(--color-kb-accent)] hover:bg-[var(--color-kb-accent)]/10 border border-[var(--color-kb-panel-border)] rounded-xl transition-all disabled:opacity-50 flex items-center gap-2"
                >
                  {loadingMore ? <RefreshCw size={14} className="animate-spin" /> : null}
                  <span>{t('loadMore', { defaultValue: '加载更多' })}</span>
                </button>
              </div>
            ) : null}
          </div>
        </div>

        <div className="h-16 border-t border-[var(--color-kb-panel-border)] bg-[var(--color-kb-panel)] flex items-center justify-between px-6 shrink-0 shadow-[0_-5px_15px_rgba(0,0,0,0.02)] select-none">
          <div className="flex items-center gap-2">
            {selectedIds.size > 0 ? (
              <div className="flex items-center gap-2.5 text-[var(--color-kb-accent)] font-semibold text-[13px]">
                <CheckSquare size={16} />
                <span>{t('selectedCount')}</span>
              </div>
            ) : (
              <div className="flex items-center gap-2 text-[var(--color-kb-text-muted)] text-[12px]">
                <AlertCircle size={15} />
                <span>{t('selectHint')}</span>
              </div>
            )}
          </div>

          <div className="flex items-center gap-3">
            <button
              onClick={onClose}
              disabled={isSyncing}
              className="px-5 py-2 text-[13px] font-medium text-[var(--color-kb-text-heading)] hover:bg-[var(--color-kb-panel-hover)] border border-[var(--color-kb-panel-border)] rounded-xl transition-all disabled:opacity-50"
            >
              {t('cancel', { defaultValue: '取消' })}
            </button>
            <button
              onClick={() => { void handleImportClick(); }}
              disabled={selectedIds.size === 0 || isSyncing || !spaceId}
              className="px-6 py-2 text-[13px] font-medium bg-[var(--color-kb-accent)] hover:bg-[var(--color-kb-accent-hover)] text-white font-semibold rounded-xl shadow-[0_4px_12px_rgba(37,99,235,0.2)] hover:shadow-[0_6px_16px_rgba(37,99,235,0.3)] transition-all disabled:opacity-40 disabled:cursor-not-allowed flex items-center gap-1.5"
            >
              <Download size={14} />
              <span>{t('importCount')}</span>
            </button>
          </div>
        </div>

      </div>
    </div>,
    document.body,
  );
}
