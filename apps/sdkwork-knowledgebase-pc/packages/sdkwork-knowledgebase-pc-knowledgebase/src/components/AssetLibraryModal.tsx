import { useState, useEffect, useMemo } from 'react';
import { X, Search, Image as ImageIcon, Music, Video, Plus, RefreshCw } from 'lucide-react';
import { useTranslation } from 'react-i18next';
import { isKnowledgebaseApiAvailable } from 'sdkwork-knowledgebase-pc-core';

import { DocumentService } from '../services/document';
import { toastKnowledgebaseError } from './ui/toastKnowledgebaseError';

export type AssetType = 'image' | 'audio' | 'video';

export interface AssetLibraryModalProps {
  isOpen: boolean;
  onClose: () => void;
  onSelect: (item: { url: string; title: string; type: AssetType; duration?: string }) => void;
  initialTab?: AssetType;
  title?: string;
  kbId?: string | null;
}

interface AssetListItem {
  title: string;
  url: string;
  type: AssetType;
  duration?: string;
  cover?: string;
  author?: string;
  size?: string;
}

export function AssetLibraryModal({
  isOpen,
  onClose,
  onSelect,
  initialTab = 'image',
  title,
  kbId,
}: AssetLibraryModalProps) {
  const { t } = useTranslation('common');
  const displayTitle = title || t('assetLibrary');

  const [activeTab, setActiveTab] = useState<AssetType>(initialTab);
  const [searchQuery, setSearchQuery] = useState('');
  const [apiItems, setApiItems] = useState<AssetListItem[]>([]);
  const [assetsTruncated, setAssetsTruncated] = useState(false);
  const [assetNextCursor, setAssetNextCursor] = useState<string | null>(null);
  const [assetHasMore, setAssetHasMore] = useState(false);
  const [loadingMore, setLoadingMore] = useState(false);
  const [loading, setLoading] = useState(false);
  const [loadError, setLoadError] = useState<string | null>(null);

  const apiMode = isKnowledgebaseApiAvailable();
  const useApiAssets = apiMode && !!kbId;

  useEffect(() => {
    if (isOpen) {
      setActiveTab(initialTab);
      setSearchQuery('');
    }
  }, [isOpen, initialTab]);

  useEffect(() => {
    if (!isOpen || !useApiAssets || !kbId) {
      setApiItems([]);
      setAssetNextCursor(null);
      setAssetHasMore(false);
      setLoadError(null);
      return;
    }

    let cancelled = false;
    setLoading(true);
    setLoadError(null);

    DocumentService.listAssetLibraryItemsPage(kbId, activeTab, null)
      .then((page) => {
        if (!cancelled) {
          setApiItems(page.items);
          setAssetsTruncated(page.truncated);
          setAssetNextCursor(page.nextCursor);
          setAssetHasMore(!!page.nextCursor);
        }
      })
      .catch((error) => {
        if (!cancelled) {
          toastKnowledgebaseError(error, t);
          setApiItems([]);
          setLoadError(error instanceof Error ? error.message : String(error));
        }
      })
      .finally(() => {
        if (!cancelled) {
          setLoading(false);
        }
      });

    return () => {
      cancelled = true;
    };
  }, [isOpen, useApiAssets, kbId, activeTab]);

  const displayedItems = useMemo(() => {
    if (apiMode && !kbId) {
      return [];
    }
    const source = useApiAssets ? apiItems : [];
    const query = searchQuery.trim().toLowerCase();
    if (!query) {
      return source;
    }
    return source.filter((item) => item.title.toLowerCase().includes(query));
  }, [apiMode, kbId, useApiAssets, apiItems, searchQuery]);

  const handleLoadMoreAssets = async () => {
    if (!kbId || !assetHasMore || loadingMore || !assetNextCursor) {
      return;
    }
    setLoadingMore(true);
    try {
      const page = await DocumentService.listAssetLibraryItemsPage(kbId, activeTab, assetNextCursor);
      setApiItems((prev) => [...prev, ...page.items]);
      setAssetsTruncated(page.truncated);
      setAssetNextCursor(page.nextCursor);
      setAssetHasMore(!!page.nextCursor);
    } catch (error) {
      toastKnowledgebaseError(error, t);
    } finally {
      setLoadingMore(false);
    }
  };

  if (!isOpen) return null;

  return (
    <div className="fixed inset-0 bg-zinc-950/40 backdrop-blur-md z-[300] flex items-center justify-center p-4 animate-in fade-in duration-200">
      <div className="bg-[var(--color-kb-panel)] border border-[var(--color-kb-panel-border)] rounded-2xl max-w-4xl w-full flex flex-col h-[85vh] shadow-2xl overflow-hidden animate-in zoom-in-95 duration-200">
        <div className="px-6 py-4 border-b border-[var(--color-kb-panel-border)] flex items-center justify-between shrink-0 bg-[var(--color-kb-editor)]">
          <div className="flex items-center gap-6">
            <span className="text-lg font-bold text-[var(--color-kb-text-heading)]">{displayTitle}</span>
            <div className="flex bg-[var(--color-kb-panel-hover)] p-1 rounded-lg">
              <button
                onClick={() => setActiveTab('image')}
                className={`px-4 py-1.5 rounded-md text-sm font-medium flex items-center gap-2 transition-colors ${activeTab === 'image' ? 'bg-[var(--color-kb-editor)] text-[var(--color-kb-accent)] shadow-sm' : 'text-[var(--color-kb-text-muted)] hover:text-[var(--color-kb-text)]'}`}
              >
                <ImageIcon size={16} />{t('imageStr')}
              </button>
              <button
                onClick={() => setActiveTab('audio')}
                className={`px-4 py-1.5 rounded-md text-sm font-medium flex items-center gap-2 transition-colors ${activeTab === 'audio' ? 'bg-[var(--color-kb-editor)] text-[var(--color-kb-accent)] shadow-sm' : 'text-[var(--color-kb-text-muted)] hover:text-[var(--color-kb-text)]'}`}
              >
                <Music size={16} />{t('audioStr')}
              </button>
              <button
                onClick={() => setActiveTab('video')}
                className={`px-4 py-1.5 rounded-md text-sm font-medium flex items-center gap-2 transition-colors ${activeTab === 'video' ? 'bg-[var(--color-kb-editor)] text-[var(--color-kb-accent)] shadow-sm' : 'text-[var(--color-kb-text-muted)] hover:text-[var(--color-kb-text)]'}`}
              >
                <Video size={16} />{t('videoStr')}
              </button>
            </div>
          </div>
          <button onClick={onClose} className="p-2 text-[var(--color-kb-text-muted)] hover:text-[var(--color-kb-text-heading)] hover:bg-[var(--color-kb-panel-hover)] rounded-full transition-colors">
            <X size={20} />
          </button>
        </div>

        <div className="px-6 py-3 border-b border-[var(--color-kb-panel-border)] flex items-center justify-between shrink-0">
          <div className="relative w-64">
            <Search size={14} className="absolute left-3 top-1/2 -translate-y-1/2 text-[var(--color-kb-text-muted)]" />
            <input
              type="text"
              placeholder={t('searchAsset')}
              value={searchQuery}
              onChange={(e) => setSearchQuery(e.target.value)}
              className="w-full pl-9 pr-3 py-1.5 text-sm bg-[var(--color-kb-editor)] border border-[var(--color-kb-panel-border)] rounded-full text-[var(--color-kb-text)] focus:outline-none focus:border-[var(--color-kb-accent)] transition-colors"
            />
          </div>
          {useApiAssets ? (
            <span className="text-xs text-[var(--color-kb-text-muted)]">
              {t('knowledgebaseAssets', { defaultValue: '来自知识库 Drive 素材' })}
            </span>
          ) : apiMode && !kbId ? (
            <span className="text-xs text-[var(--color-kb-text-muted)]">
              {t('kbRequiredForAssets', { defaultValue: '请先选择知识库' })}
            </span>
          ) : null}
        </div>

        {assetsTruncated ? (
          <div className="px-6 py-2 text-xs text-amber-600 bg-amber-50 border-b border-amber-100">
            {t('assetLibraryTruncated', { defaultValue: '素材过多，仅显示前 200 项。请缩小目录范围或使用搜索。' })}
          </div>
        ) : null}

        <div className="flex-1 overflow-y-auto p-6 bg-[var(--color-kb-panel)]">
          {loading ? (
            <div className="flex flex-col items-center justify-center py-20 text-[var(--color-kb-text-muted)]">
              <RefreshCw size={32} className="mb-3 animate-spin text-[var(--color-kb-accent)]" />
              <p>{t('loading', { defaultValue: 'Loading assets...' })}</p>
            </div>
          ) : loadError ? (
            <div className="flex flex-col items-center justify-center py-20 text-[var(--color-kb-text-muted)]">
              <p className="text-sm">{loadError}</p>
            </div>
          ) : activeTab === 'image' ? (
            <div className="grid grid-cols-2 xl:grid-cols-3 gap-6">
              {displayedItems.map((item) => (
                <div
                  key={`${item.type}-${item.url}`}
                  className="group relative aspect-[900/383] rounded-xl overflow-hidden cursor-pointer border border-transparent hover:border-[var(--color-kb-panel-border)] shadow-sm hover:shadow-md transition-all bg-[var(--color-kb-editor)]"
                  onClick={() => onSelect({ url: item.url, title: item.title, type: 'image' })}
                >
                  <img src={item.url} referrerPolicy="no-referrer" alt={item.title} className="w-full h-full object-cover group-hover:scale-105 transition-transform duration-300" />
                  <div className="absolute inset-x-0 bottom-0 bg-gradient-to-t from-black/80 to-transparent p-3 pt-6 flex items-end justify-between opacity-0 group-hover:opacity-100 transition-opacity">
                    <span className="text-white text-sm font-medium truncate pr-2">{item.title}</span>
                    <span className="w-6 h-6 rounded-full bg-[var(--color-kb-accent)] text-white flex items-center justify-center shrink-0 shadow-lg">
                      <Plus size={14} />
                    </span>
                  </div>
                </div>
              ))}
            </div>
          ) : activeTab === 'audio' ? (
            <div className="space-y-3 max-w-3xl mx-auto">
              {displayedItems.map((item) => (
                <div
                  key={`${item.type}-${item.url}`}
                  className="group flex items-center justify-between p-4 rounded-xl bg-[var(--color-kb-editor)] border border-[var(--color-kb-panel-border)] hover:border-[var(--color-kb-accent)]/50 hover:shadow-sm cursor-pointer transition-all"
                  onClick={() => onSelect({ url: item.url, title: item.title, type: 'audio', duration: item.duration })}
                >
                  <div className="flex items-center gap-4 min-w-0">
                    <div className="w-10 h-10 rounded-full bg-[var(--color-kb-accent)]/10 text-[var(--color-kb-accent)] flex items-center justify-center shrink-0 relative overflow-hidden group-hover:bg-[var(--color-kb-accent)] group-hover:text-white transition-colors">
                      <Music size={18} className="relative z-10" />
                    </div>
                    <div className="min-w-0 flex-1">
                      <h4 className="text-sm font-semibold text-[var(--color-kb-text-heading)] truncate mb-0.5">{item.title}</h4>
                      {item.author ? (
                        <div className="flex items-center text-xs text-[var(--color-kb-text-muted)] gap-3">
                          <span>{item.author}</span>
                          {item.size ? (
                            <>
                              <span className="w-1 h-1 rounded-full bg-gray-400/50" />
                              <span>{item.size}</span>
                            </>
                          ) : null}
                        </div>
                      ) : null}
                    </div>
                  </div>
                  <div className="flex items-center gap-4 shrink-0 mt-0">
                    {item.duration ? (
                      <span className="text-xs font-mono text-[var(--color-kb-text-muted)] bg-[var(--color-kb-panel)] px-2 py-1 rounded">{item.duration}</span>
                    ) : null}
                    <button className="w-8 h-8 rounded-full border border-[var(--color-kb-panel-border)] text-[var(--color-kb-text-muted)] flex items-center justify-center group-hover:bg-[var(--color-kb-accent)] group-hover:border-[var(--color-kb-accent)] group-hover:text-white transition-all shadow-sm">
                      <Plus size={14} />
                    </button>
                  </div>
                </div>
              ))}
            </div>
          ) : (
            <div className="grid grid-cols-1 md:grid-cols-2 xl:grid-cols-3 gap-6">
              {displayedItems.map((item) => (
                <div
                  key={`${item.type}-${item.url}`}
                  className="group flex flex-col rounded-xl overflow-hidden bg-[var(--color-kb-editor)] border border-[var(--color-kb-panel-border)] hover:border-[var(--color-kb-accent)]/50 hover:shadow-md cursor-pointer transition-all"
                  onClick={() => onSelect({ url: item.url, title: item.title, type: 'video', duration: item.duration })}
                >
                  <div className="aspect-video relative overflow-hidden bg-black/5 flex items-center justify-center">
                    {item.cover ? (
                      <img src={item.cover} alt={item.title} className="w-full h-full object-cover group-hover:scale-105 transition-transform duration-300 opacity-90" />
                    ) : (
                      <Video size={40} className="text-[var(--color-kb-text-muted)] opacity-40" />
                    )}
                    <div className="absolute inset-0 bg-black/20 group-hover:bg-black/10 transition-colors flex items-center justify-center">
                      <div className="w-10 h-10 rounded-full bg-white/20 backdrop-blur-sm border border-white/40 flex items-center justify-center shadow-lg group-hover:scale-110 transition-transform">
                        <Video size={18} className="text-white ml-0.5" />
                      </div>
                    </div>
                    {item.duration ? (
                      <div className="absolute bottom-2 right-2 px-1.5 py-0.5 bg-black/60 backdrop-blur-md rounded text-[10px] text-white font-mono shadow">
                        {item.duration}
                      </div>
                    ) : null}
                  </div>
                  <div className="p-3">
                    <h4 className="text-sm font-medium text-[var(--color-kb-text-heading)] line-clamp-2 leading-snug mb-2 group-hover:text-[var(--color-kb-accent)] transition-colors">{item.title}</h4>
                    {item.size ? (
                      <div className="flex items-center justify-between text-xs text-[var(--color-kb-text-muted)]">
                        <span>{item.size}</span>
                        <span className="flex items-center text-[var(--color-kb-accent)] font-medium opacity-0 group-hover:opacity-100 transition-opacity translate-y-1 group-hover:translate-y-0 duration-200">
                          {t('useAsset')} <Plus size={12} className="ml-0.5" />
                        </span>
                      </div>
                    ) : null}
                  </div>
                </div>
              ))}
            </div>
          )}

          {useApiAssets && assetHasMore && !loading && !loadError ? (
            <div className="flex justify-center pt-4">
              <button
                type="button"
                disabled={loadingMore}
                onClick={handleLoadMoreAssets}
                className="px-4 py-2 text-sm font-medium rounded-lg border border-[var(--color-kb-panel-border)] bg-[var(--color-kb-editor)] hover:bg-[var(--color-kb-panel-hover)] disabled:opacity-50"
              >
                {loadingMore
                  ? t('loading', { defaultValue: '加载中...' })
                  : t('loadMoreAssets', { defaultValue: '加载更多素材' })}
              </button>
            </div>
          ) : null}

          {!loading && !loadError && displayedItems.length === 0 && (
            <div className="flex flex-col items-center justify-center py-20 text-[var(--color-kb-text-muted)]">
              <Search size={48} className="mb-4 opacity-20" />
              <p>
                {apiMode && !kbId
                  ? t('kbRequiredForAssets', { defaultValue: '请先选择知识库后再浏览素材库' })
                  : searchQuery
                    ? t('noSearchResult', { query: searchQuery })
                    : t('noAssets', { defaultValue: '当前知识库暂无该类型素材' })}
              </p>
            </div>
          )}
        </div>
      </div>
    </div>
  );
}
