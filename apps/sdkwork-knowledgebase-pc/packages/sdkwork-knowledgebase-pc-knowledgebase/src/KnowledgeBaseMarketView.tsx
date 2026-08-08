import { useState, useEffect } from 'react';
import { Search, Check, Sparkles, Compass, ArrowRight, ShieldCheck } from 'lucide-react';
import { DocumentService, MarketKnowledgeBase } from './services/document';
import { useLocalStorage } from '@sdkwork/sdkwork-knowledgebase-pc-commons';

interface KnowledgeBaseMarketViewProps {
  onSubscribedChange: () => void;
}

interface Category {
  id: string;
  name: string;
  icon: string;
  tags: string[];
}

const categories: Category[] = [
  { id: 'all', name: '全部共享库', icon: '🌍', tags: [] },
  { id: 'ai', name: '人工智能与提效', icon: '🤖', tags: ['人工智能', '提效工具', 'MCP协议', '生态工具'] },
  { id: 'dev', name: '编程与前端开发', icon: '💻', tags: ['编程开发', '前端', '技术框架', 'API指南'] },
  { id: 'finance', name: '金融与投资复盘', icon: '📈', tags: ['金融证券', '每日复盘'] },
  { id: 'community', name: '团队操作与规范', icon: '🏢', tags: ['社区规范', '新手帮助'] },
  { id: 'travel', name: '自驾旅行与探险', icon: '🧭', tags: ['旅行生活', '探险攻略'] },
];

export function KnowledgeBaseMarketView({ onSubscribedChange }: KnowledgeBaseMarketViewProps) {
  const [marketKbs, setMarketKbs] = useState<MarketKnowledgeBase[]>([]);
  const [marketCursor, setMarketCursor] = useState<string | null>(null);
  const [marketHasMore, setMarketHasMore] = useState(false);
  const [loadingMore, setLoadingMore] = useState(false);
  const [loading, setLoading] = useState(true);
  const [searchQuery, setSearchQuery] = useLocalStorage('market-search', '');
  const [selectedTag, setSelectedTag] = useLocalStorage<string | null>('market-tag', null);
  const [selectedCategory, setSelectedCategory] = useLocalStorage<Category>('market-category', categories[0]);
  const [toastMessage, setToastMessage] = useState<string | null>(null);

  useEffect(() => {
    fetchMarketKbs();
  }, []);

  const fetchMarketKbs = () => {
    setLoading(true);
    setMarketCursor(null);
    setMarketHasMore(false);
    DocumentService.listMarketKnowledgeBasesPage(null).then(res => {
      setMarketKbs(res.items);
      setMarketCursor(res.nextCursor);
      setMarketHasMore(res.hasMore);
      setLoading(false);
    });
  };

  const loadMoreMarketKbs = () => {
    if (!marketCursor || loadingMore) return;
    setLoadingMore(true);
    DocumentService.listMarketKnowledgeBasesPage(marketCursor).then(res => {
      setMarketKbs(prev => {
        const seen = new Set(prev.map(item => item.id));
        return [...prev, ...res.items.filter(item => !seen.has(item.id))];
      });
      setMarketCursor(res.nextCursor);
      setMarketHasMore(res.hasMore);
      setLoadingMore(false);
    });
  };

  const showToast = (msg: string) => {
    setToastMessage(msg);
    setTimeout(() => {
      setToastMessage(null);
    }, 2500);
  };

  const handleSubscribe = async (id: string, title: string) => {
    const success = await DocumentService.subscribeMarketKb(id);
    if (success) {
      showToast(`已成功订阅共享知识库："${title}"`);
      fetchMarketKbs();
      onSubscribedChange();
    }
  };

  const handleUnsubscribe = async (id: string, title: string) => {
    const success = await DocumentService.unsubscribeMarketKb(id);
    if (success) {
      showToast(`已成功取消订阅："${title}"`);
      fetchMarketKbs();
      onSubscribedChange();
    }
  };

  // Get dynamic count of items belonging to a category
  const getCategoryCount = (cat: Category) => {
    if (cat.id === 'all') return marketKbs.length;
    return marketKbs.filter(item => item.tags.some(t => cat.tags.includes(t))).length;
  };

  // Filter items
  const filteredKbs = marketKbs.filter(item => {
    const matchesSearch = item.title.toLowerCase().includes(searchQuery.toLowerCase()) || 
                          item.description.toLowerCase().includes(searchQuery.toLowerCase()) ||
                          item.author.toLowerCase().includes(searchQuery.toLowerCase());
    
    const matchesCategory = selectedCategory.id === 'all' 
      ? true 
      : item.tags.some(t => selectedCategory.tags.includes(t));

    const matchesTag = selectedTag ? item.tags.includes(selectedTag) : true;
    
    return matchesSearch && matchesCategory && matchesTag;
  });

  return (
    <div className="flex-1 flex h-full bg-[var(--color-kb-editor)] relative overflow-hidden">
      
      {/* 1. Left Sidebar: Enhanced Categories list */}
      <div className="w-[230px] flex-shrink-0 flex flex-col bg-[var(--color-kb-panel)] border-r border-[var(--color-kb-panel-border)] relative overflow-hidden select-none">
        
        {/* Marketplace header */}
        <div className="px-5 py-4 min-h-[60px] flex items-center justify-between min-w-0 border-b border-[var(--color-kb-panel-border)]/60 bg-[var(--color-kb-panel)]">
          <div className="flex items-center min-w-0 pr-2">
            <div className="w-6 h-6 rounded bg-emerald-500/10 text-emerald-600 flex items-center justify-center mr-2.5 shadow-sm shrink-0">
              <Compass size={13} className="text-emerald-500" />
            </div>
            <h2 className="font-bold text-[14px] tracking-wide text-[var(--color-kb-text-heading)] truncate">市场分类</h2>
          </div>
        </div>

        {/* Categories iteration list */}
        <div className="flex-1 overflow-y-auto py-3 space-y-1">
          {categories.map((cat) => {
            const isActive = selectedCategory.id === cat.id;
            return (
              <div 
                key={cat.id}
                onClick={() => {
                  setSelectedCategory(cat);
                  setSelectedTag(null); // Reset tag filters when category changes
                }}
                className={`flex items-center px-4 py-2.5 mx-2.5 rounded-lg cursor-pointer transition-all border border-transparent ${isActive ? 'bg-emerald-500/10 text-emerald-600 dark:text-emerald-400 font-bold border-emerald-500/10' : 'text-[var(--color-kb-text)] hover:bg-[var(--color-kb-panel-hover)]'}`}
              >
                <span className="mr-2.5 text-[14px] leading-none shrink-0">{cat.icon}</span>
                <span className="text-[12.5px] flex-1 truncate">{cat.name}</span>
                <span className="text-[10px] font-mono font-bold bg-[var(--color-kb-panel-active)]/50 border border-[var(--color-kb-panel-border)] px-1.5 py-0.5 rounded-lg text-zinc-500 shrink-0 select-none">
                  {getCategoryCount(cat)}
                </span>
              </div>
            );
          })}
        </div>
      </div>

      {/* 2. Right view: Market content and lists */}
      <div className="flex-1 flex flex-col h-full overflow-hidden">
        
        {/* Header banner decoration */}
        <div className="relative bg-gradient-to-r from-emerald-600/10 via-teal-600/5 to-transparent border-b border-[var(--color-kb-panel-border)] px-8 py-5 flex items-center justify-between shrink-0">
          <div className="flex items-center gap-3.5">
            <div className="w-10 h-10 rounded-2xl bg-gradient-to-br from-emerald-500 to-teal-500 text-white flex items-center justify-center shadow-lg shadow-emerald-500/20">
              <Compass size={20} className="animate-spin-slow" />
            </div>
            <div>
              <div className="flex items-center gap-2">
                <h3 className="font-bold text-[17px] text-[var(--color-kb-text-heading)] tracking-wide">云端知识库市场</h3>
                <span className="text-[9px] bg-emerald-500/10 text-emerald-600 dark:text-emerald-400 font-bold px-2 py-0.5 rounded-full uppercase tracking-wider font-mono">Marketplace</span>
              </div>
              <p className="text-[11.5px] text-[var(--color-kb-text-muted)] mt-0.5">一键订阅全网精选的行业共享知识库，直接绑定大模型助理对海量文档开展高灵敏问答与解读。</p>
            </div>
          </div>
        </div>

        {/* Search controls row */}
        <div className="px-8 py-4 bg-[var(--color-kb-panel)]/40 border-b border-[var(--color-kb-panel-border)] flex items-center gap-4 shrink-0 shadow-sm">
          <div className="relative flex-1">
            <Search className="absolute left-3.5 top-1/2 -translate-y-1/2 text-[var(--color-kb-text-muted)]" size={15} />
            <input 
              type="text"
              placeholder="通过知识库名称、描述核心词、作者等搜索共享库..."
              value={searchQuery}
              onChange={(e) => setSearchQuery(e.target.value)}
              className="w-full bg-[var(--color-kb-input-bg)] hover:bg-[var(--color-kb-input-bg)]/80 border border-[var(--color-kb-panel-border)] focus:border-emerald-500 focus:ring-4 focus:ring-emerald-500/10 rounded-xl pl-10 pr-4 py-2 text-[13px] transition-all outline-none text-[var(--color-kb-text)] shadow-inner"
            />
          </div>
        </div>

        {/* Sub-tag filter pills inside the chosen category */}
        {selectedCategory.id !== 'all' && selectedCategory.tags.length > 0 && (
          <div className="flex items-center gap-2 px-8 py-2.5 bg-[var(--color-kb-panel)]/20 border-b border-[var(--color-kb-panel-border)] overflow-x-auto shrink-0 select-none no-scrollbar">
            <span className="text-[10px] text-[var(--color-kb-text-muted)] font-bold mr-1 whitespace-nowrap uppercase tracking-wider">子标签筛选:</span>
            <button 
              onClick={() => setSelectedTag(null)}
              className={`px-2.5 py-1 text-[11px] rounded-lg border font-bold transition-all ${!selectedTag ? 'bg-emerald-500 border-emerald-500 text-white shadow-sm shadow-emerald-500/10' : 'bg-transparent border-[var(--color-kb-panel-border)] text-zinc-500 hover:text-[var(--color-kb-text)] hover:bg-[var(--color-kb-panel-hover)]'}`}
            >
              全部
            </button>
            {selectedCategory.tags.map(tag => (
              <button 
                key={tag}
                onClick={() => setSelectedTag(tag)}
                className={`px-2.5 py-1 text-[11px] rounded-lg border font-bold transition-all whitespace-nowrap ${selectedTag === tag ? 'bg-emerald-500 border-emerald-500 text-white shadow-sm' : 'bg-[var(--color-kb-panel-hover)] border-[var(--color-kb-panel-border)] text-[var(--color-kb-text)] hover:text-emerald-500'}`}
              >
                #{tag}
              </button>
            ))}
          </div>
        )}

        {/* Content body layout */}
        <div className="flex-1 overflow-y-auto px-8 py-6">
          {loading ? (
            <div className="h-full flex flex-col items-center justify-center space-y-3">
              <div className="w-9 h-9 border-4 border-emerald-500/30 border-t-emerald-500 rounded-full animate-spin" />
              <p className="text-[12px] text-[var(--color-kb-text-muted)] font-medium">正在拉取最新共享知识库资产...</p>
            </div>
          ) : filteredKbs.length === 0 ? (
            <div className="h-full flex flex-col items-center justify-center p-12 text-center">
              <span className="text-[40px] mb-3">🔍</span>
              <h4 className="font-bold text-[14.5px] text-[var(--color-kb-text-heading)]">没有找到匹配的共享库</h4>
              <p className="text-[11.5px] text-[var(--color-kb-text-muted)] mt-1 max-w-sm">请尝试切换其他更宽泛的关键字或者清除当前的分类/子标签检索筛选。</p>
              {(selectedTag || selectedCategory.id !== 'all') && (
                <button 
                  onClick={() => {
                    setSelectedCategory(categories[0]);
                    setSelectedTag(null);
                    setSearchQuery('');
                  }} 
                  className="mt-4 px-4 py-1.5 text-[11.5px] font-bold text-white bg-emerald-500 hover:bg-emerald-600 rounded-xl transition-all shadow-md"
                >
                  重置所有筛选
                </button>
              )}
            </div>
          ) : (
            <div className="grid grid-cols-1 xl:grid-cols-2 gap-5">
              {filteredKbs.map(item => (
                <div 
                  key={item.id}
                  className={`group relative rounded-2xl border transition-all duration-300 p-5 bg-[var(--color-kb-panel)] hover:shadow-xl ${item.isSubscribed ? 'border-emerald-500/30 bg-emerald-500/2 shadow-inner shadow-emerald-500/2' : 'border-[var(--color-kb-panel-border)] hover:border-emerald-500/20'}`}
                >
                  <div className="flex items-start justify-between gap-4">
                    <div className="flex items-start gap-4">
                      {/* Better/More Vivid visual icon design */}
                      <div className={`w-11 h-11 rounded-xl flex items-center justify-center text-[20px] shadow-md shrink-0 transition-transform group-hover:scale-110 group-hover:rotate-3 ${item.isSubscribed ? 'bg-gradient-to-br from-emerald-500 to-teal-500 text-white' : 'bg-[var(--color-kb-panel-hover)] border border-[var(--color-kb-panel-border)] dark:bg-zinc-800'}`}>
                        {item.icon}
                      </div>
                      
                      <div className="min-w-0">
                        <div className="flex items-center gap-1.5 flex-wrap">
                          <h4 className="font-bold text-[14px] text-[var(--color-kb-text-heading)] group-hover:text-emerald-600 dark:group-hover:text-emerald-400 transition-colors leading-snug truncate">{item.title}</h4>
                          {item.isSubscribed && (
                            <span className="text-[9px] font-bold font-mono bg-emerald-500 text-white px-1.5 py-0.5 rounded-full flex items-center gap-0.5 shadow-sm scale-95 shrink-0 select-none">
                              <Check size={8} strokeWidth={3} /> 已订阅
                            </span>
                          )}
                        </div>
                        <div className="text-[11px] text-[var(--color-kb-text-muted)] mt-0.5 flex items-center gap-1">
                          <span>By {item.author}</span>
                          <span>•</span>
                          <span className="font-mono text-emerald-600 dark:text-emerald-400 font-bold">{item.modelName}</span>
                        </div>
                      </div>
                    </div>
                  </div>

                  <p className="text-[11.5px] text-[var(--color-kb-text-muted)] leading-relaxed mt-3 h-[34px] line-clamp-2">
                    {item.description}
                  </p>

                  <div className="flex flex-wrap gap-1.5 mt-3">
                    {item.tags.map(t => (
                      <span key={t} className="text-[10px] px-2 py-0.5 rounded-md bg-[var(--color-kb-panel-hover)] border border-[var(--color-kb-panel-border)] text-zinc-600 dark:text-zinc-400 font-medium">#{t}</span>
                    ))}
                  </div>

                  {/* Divider */}
                  <div className="border-t border-[var(--color-kb-panel-border)]/60 my-3.5" />

                  <div className="flex items-center justify-between mt-1">
                    <div className="text-[10.5px] text-[var(--color-kb-text-muted)] font-medium flex items-center gap-3">
                      <span>👀 {item.subscribersCount} 人订阅</span>
                      <span>📖 {item.documentsCount} 篇资源</span>
                    </div>

                    {item.isSubscribed ? (
                      <button 
                        onClick={() => handleUnsubscribe(item.id, item.title)}
                        className="px-3.5 py-1.5 text-[11px] font-bold border border-red-500/25 text-red-500 hover:bg-red-500 hover:text-white rounded-xl transition-all shadow-sm"
                      >
                        取消订阅
                      </button>
                    ) : (
                      <button 
                        onClick={() => handleSubscribe(item.id, item.title)}
                        className="px-4 py-1.5 text-[10.5px] font-bold bg-gradient-to-r from-emerald-600 to-teal-600 hover:from-emerald-700 hover:to-teal-700 text-white rounded-xl transition-all shadow-md flex items-center gap-1 active:scale-95"
                      >
                        免费订阅 <ArrowRight size={11} />
                      </button>
                    )}
                  </div>
                </div>
              ))}
            </div>
          )}
          {!loading && marketHasMore && (
            <div className="flex justify-center py-6">
              <button
                type="button"
                onClick={loadMoreMarketKbs}
                disabled={loadingMore}
                className="px-5 py-2 text-[12px] font-bold text-white bg-emerald-500 hover:bg-emerald-600 disabled:opacity-60 rounded-xl transition-all shadow-md flex items-center gap-2"
              >
                {loadingMore ? (
                  <>
                    <span className="w-3.5 h-3.5 border-2 border-white/40 border-t-white rounded-full animate-spin" />
                    加载中...
                  </>
                ) : (
                  '加载更多共享库'
                )}
              </button>
            </div>
          )}
        </div>

        {/* Footer info alert */}
        <div className="px-8 py-3.5 border-t border-[var(--color-kb-panel-border)] bg-[var(--color-kb-panel)]/40 flex items-center shrink-0">
          <span className="text-[11px] text-[var(--color-kb-text-muted)] flex items-center gap-1.5">
            <ShieldCheck size={14} className="text-emerald-500 animate-pulse" />
            所有在市场中陈列的共享知识库均经过大语言模型内容安全审查以及高可用质量测评。
          </span>
        </div>
      </div>

      {/* Floating custom message toast */}
      {toastMessage && (
        <div className="absolute top-6 left-1/2 -translate-y-0 -translate-x-1/2 px-5 py-3 rounded-2xl bg-zinc-900 border border-zinc-800 text-white shadow-2xl flex items-center gap-2.5 z-[500] animate-in fade-in slide-in-from-top-4 duration-300">
          <Sparkles size={16} className="text-yellow-400 animate-pulse" />
          <span className="text-[12.5px] font-semibold">{toastMessage}</span>
        </div>
      )}
    </div>
  );
}
