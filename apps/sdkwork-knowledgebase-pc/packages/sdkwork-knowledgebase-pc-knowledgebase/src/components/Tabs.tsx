import React, { useRef, useState, useEffect } from 'react';
import { ChevronLeft, ChevronRight, X, ChevronDown, Search, FolderX } from 'lucide-react';
import { useTranslation } from 'react-i18next';

export interface TabItem {
  id: string;
  title: string;
  type: string;
}

export interface TabsProps<T extends TabItem = TabItem> {
  items: T[];
  activeId?: string;
  onSelect?: (item: T) => void;
  onClose?: (id: string, e: React.MouseEvent) => void;
  getTabIcon?: (type: string) => React.ReactNode;
  onContextMenu?: (e: React.MouseEvent, item: T) => void;
  onBarContextMenu?: (e: React.MouseEvent) => void;
  rightActions?: React.ReactNode;
}

export function Tabs<T extends TabItem = TabItem>({
  items = [],
  activeId,
  onSelect,
  onClose,
  getTabIcon,
  onContextMenu,
  onBarContextMenu,
  rightActions
}: TabsProps<T>) {
  const { t } = useTranslation('editor');
  const tabsContainerRef = useRef<HTMLDivElement>(null);
  const dropdownRef = useRef<HTMLDivElement>(null);
  
  const [showScrollLeft, setShowScrollLeft] = useState(false);
  const [showScrollRight, setShowScrollRight] = useState(false);
  const [isDropdownOpen, setIsDropdownOpen] = useState(false);
  const [searchQuery, setSearchQuery] = useState('');

  // 1. Intelligent width / padding calculation helper based on current active count
  const getTabWidthClass = () => {
    const count = items.length;
    if (count <= 4) {
      return 'px-4 gap-2.5 text-[12.5px] min-w-[130px] max-w-[195px] font-extrabold tracking-tight shrink-0';
    } else if (count <= 8) {
      return 'px-3.5 gap-2 text-[12px] min-w-[110px] max-w-[155px] font-semibold tracking-tight shrink-0';
    } else {
      return 'px-2.5 gap-1.5 text-[11px] min-w-[100px] max-w-[135px] font-medium tracking-tight shrink-0';
    }
  };

  const getTabIconClass = () => {
    const count = items.length;
    if (count <= 4) return 'scale-100 shrink-0';
    if (count <= 8) return 'scale-90 shrink-0';
    return 'scale-[0.82] shrink-0';
  };

  const getTabTextClass = () => {
    const count = items.length;
    if (count <= 4) return 'max-w-[125px]';
    if (count <= 8) return 'max-w-[95px]';
    return 'max-w-[85px]';
  };

  // 2. Intelligent scroll centering helper for clicked/active tabs
  const smartScrollToActiveTab = (tabId: string) => {
    const container = tabsContainerRef.current;
    if (!container) return;
    const activeTabEl = container.querySelector(`[data-tab-id="${tabId}"]`) as HTMLElement;
    if (activeTabEl) {
      const containerWidth = container.clientWidth;
      const tabWidth = activeTabEl.clientWidth;
      const tabOffsetLeft = activeTabEl.offsetLeft;
      
      // Keep selected tab intelligently scrolled to the exact horizontal center of the viewport
      const targetScrollLeft = tabOffsetLeft - (containerWidth / 2) + (tabWidth / 2);
      
      container.scrollTo({
        left: Math.max(0, targetScrollLeft),
        behavior: 'smooth'
      });
    }
  };

  const updateScrollButtons = () => {
    if (tabsContainerRef.current) {
      const { scrollLeft, scrollWidth, clientWidth } = tabsContainerRef.current;
      setShowScrollLeft(scrollLeft > 1);
      setShowScrollRight(scrollLeft + clientWidth < scrollWidth - 1);
    }
  };

  useEffect(() => {
    const container = tabsContainerRef.current;
    if (container) {
      container.addEventListener('scroll', updateScrollButtons);
      updateScrollButtons();
      
      const observer = new ResizeObserver(updateScrollButtons);
      observer.observe(container);

      // Smoothly center the active tab into view when it mounts/updates
      let timer: NodeJS.Timeout;
      if (activeId) {
        timer = setTimeout(() => {
          smartScrollToActiveTab(activeId);
        }, 120);
      }

      return () => {
        container.removeEventListener('scroll', updateScrollButtons);
        observer.disconnect();
        if (timer) clearTimeout(timer);
      };
    }
    return undefined;
  }, [items, activeId]);

  // Click outside to close the dropdown
  useEffect(() => {
    function handleClickOutside(event: MouseEvent) {
      if (dropdownRef.current && !dropdownRef.current.contains(event.target as Node)) {
        setIsDropdownOpen(false);
      }
    }
    if (isDropdownOpen) {
      document.addEventListener('mousedown', handleClickOutside);
    }
    return () => {
      document.removeEventListener('mousedown', handleClickOutside);
    };
  }, [isDropdownOpen]);

  const scrollTabs = (direction: 'left' | 'right') => {
    if (tabsContainerRef.current) {
      const scrollAmount = 240;
      tabsContainerRef.current.scrollBy({
        left: direction === 'left' ? -scrollAmount : scrollAmount,
        behavior: 'smooth'
      });
    }
  };

  const handleWheel = (e: React.WheelEvent) => {
    if (tabsContainerRef.current) {
      if (Math.abs(e.deltaX) > Math.abs(e.deltaY)) return;
      tabsContainerRef.current.scrollLeft += e.deltaY;
    }
  };

  const filteredItems = items.filter(item => 
    item.title?.toLowerCase().includes(searchQuery.toLowerCase())
  );

  return (
    <div 
      onContextMenu={onBarContextMenu}
      className={`relative flex items-end bg-[var(--color-kb-panel)]/90 border-b border-zinc-200/80 dark:border-[var(--color-kb-panel-border)] h-[40px] shrink-0 select-none w-full group/tabbar backdrop-blur-md justify-between ${
        isDropdownOpen ? 'z-50' : 'z-10'
      }`}
    >
      {/* Scrollable Tabs Wrapper */}
      <div className="flex-1 flex items-end h-[40px] min-w-0 overflow-hidden bg-[var(--color-kb-panel)]/50">
        
        {/* Left Arrow */}
        {showScrollLeft && items.length > 0 && (
          <button 
            type="button"
            onClick={() => scrollTabs('left')}
            className="h-[39px] w-7 flex items-center justify-center shrink-0 border-r border-zinc-200/50 dark:border-zinc-800/25 text-zinc-400 dark:text-[var(--color-kb-text-muted)] hover:text-zinc-700 dark:hover:text-[var(--color-kb-text-heading)] hover:bg-zinc-100/50 dark:hover:bg-zinc-900/50 active:scale-95 transition-all outline-none bg-[var(--color-kb-panel)]/95"
            title={t('scrollLeft', { defaultValue: '向左滚动' })}
          >
            <ChevronLeft size={13} strokeWidth={2.5} />
          </button>
        )}

        {/* Inner dynamic scroll-container */}
        <div 
          ref={tabsContainerRef}
          onWheel={handleWheel}
          className="flex-1 flex items-end h-full overflow-x-auto no-scrollbar scroll-smooth whitespace-nowrap px-1.5"
        >
          {items.map((item) => {
            const isActive = activeId === item.id;
            return (
              <div
                key={item.id}
                data-tab-id={item.id}
                onClick={() => {
                  onSelect?.(item);
                  smartScrollToActiveTab(item.id);
                }}
                onContextMenu={(e) => onContextMenu?.(e, item)}
                className={`group/tab relative flex items-center h-[40px] rounded-t-lg cursor-pointer transition-all select-none border-x border-t ${getTabWidthClass()} ${
                  isActive
                    ? 'bg-white dark:bg-[var(--color-kb-editor)] text-slate-900 dark:text-white border-zinc-200/80 dark:border-[var(--color-kb-panel-border)] shadow-sm z-10'
                    : 'bg-transparent text-zinc-500 dark:text-zinc-500 hover:text-zinc-800 dark:hover:text-zinc-300 hover:bg-zinc-200/40 dark:hover:bg-zinc-800/40 border-transparent'
                }`}
                title={item.title}
              >
                {getTabIcon && (
                  <div className={`flex items-center justify-center transition-all ${getTabIconClass()} ${isActive ? 'opacity-100 drop-shadow-sm' : 'opacity-60 group-hover/tab:opacity-100'}`}>
                    {getTabIcon(item.type)}
                  </div>
                )}
                
                <span className={`truncate pr-1 tracking-tight ${getTabTextClass()}`}>
                  {item.title}
                </span>

                {isActive && (
                  <div className="absolute -bottom-px left-0 right-0 h-[2px] bg-white dark:bg-[var(--color-kb-editor)] z-20"></div>
                )}
                
                {!isActive && (
                  <div className="absolute right-0 top-3 bottom-3 w-[1px] bg-zinc-200 dark:bg-zinc-800/60 pointer-events-none transition-opacity group-hover/tab:opacity-0" />
                )}
                
                {onClose && (
                  <button
                    onClick={(e) => {
                      e.stopPropagation();
                      onClose(item.id, e);
                    }}
                    className={`p-0.5 rounded-md text-zinc-400 dark:text-zinc-500 hover:text-red-500 hover:bg-red-50 dark:hover:bg-red-500/15 shrink-0 ml-auto transition-all outline-none ${
                      isActive ? 'opacity-100' : 'opacity-0 group-hover/tab:opacity-85'
                    }`}
                  >
                    <X size={11} strokeWidth={2.5} />
                  </button>
                )}
              </div>
            );
          })}
        </div>

        {/* Right Arrow */}
        {showScrollRight && items.length > 0 && (
          <button 
            type="button"
            onClick={() => scrollTabs('right')}
            className="h-[39px] w-7 flex items-center justify-center shrink-0 border-l border-zinc-200/50 dark:border-zinc-800/25 text-zinc-400 dark:text-[var(--color-kb-text-muted)] hover:text-zinc-700 dark:hover:text-[var(--color-kb-text-heading)] hover:bg-zinc-100/50 dark:hover:bg-zinc-900/50 active:scale-95 transition-all outline-none bg-[var(--color-kb-panel)]/95"
            title={t('scrollRight', { defaultValue: '向右滚动' })}
          >
            <ChevronRight size={13} strokeWidth={2.5} />
          </button>
        )}
      </div>

      {/* Smart Hub Controller & Actions */}
      <div className={`flex items-center gap-1.5 px-2 h-[39px] shrink-0 border-l border-zinc-200/50 dark:border-zinc-800/30 bg-[var(--color-kb-panel)]/95 relative ${
        isDropdownOpen ? 'z-50' : 'z-30'
      }`}>
        
        {/* Smart tabs count preview switcher - Text removed, now a micro button containing only highly optimized dropdown trigger */}
        {items.length > 0 && (
          <div className="relative z-50" ref={dropdownRef}>
            <button
              type="button"
              onClick={() => {
                setIsDropdownOpen(!isDropdownOpen);
                setSearchQuery('');
              }}
              className={`relative h-7 w-7 flex items-center justify-center rounded-lg transition-all border outline-none select-none active:scale-95 ${
                isDropdownOpen
                  ? 'bg-zinc-100 dark:bg-zinc-800 border-zinc-300 dark:border-zinc-700 text-zinc-800 dark:text-zinc-100'
                  : 'bg-white dark:bg-zinc-900 border-zinc-200/80 dark:border-zinc-800 text-zinc-500 dark:text-zinc-400 hover:bg-zinc-50 dark:hover:bg-zinc-800/50 hover:text-zinc-800 dark:hover:text-zinc-200 shadow-[0_1px_2px_rgba(0,0,0,0.02)]'
              }`}
              title="搜索/显示已打开的所有标签"
            >
              <ChevronDown size={14} className={`transition-transform duration-250 shrink-0 ${isDropdownOpen ? 'rotate-180 text-zinc-700 dark:text-zinc-250' : 'text-zinc-500 dark:text-zinc-400'}`} />
              <span className={`absolute -top-1.5 -right-1.5 flex h-4 min-w-[16px] px-1 items-center justify-center rounded-full text-[8px] font-black text-white ring-1 ring-white dark:ring-black leading-none transform scale-90 transition-all ${
                isDropdownOpen
                  ? 'bg-zinc-700 dark:bg-zinc-300 text-white dark:text-zinc-900'
                  : 'bg-zinc-500 dark:bg-zinc-500 text-white'
              }`}>
                {items.length}
              </span>
            </button>

            {/* Smart panel dropdown - Absolute placement */}
            {isDropdownOpen && (
              <div className="absolute right-0 top-full mt-1.5 w-72 bg-white dark:bg-[#0c0c0e] border border-zinc-200 dark:border-zinc-800 rounded-2xl shadow-xl overflow-hidden z-[9999] flex flex-col max-h-[350px] animate-in fade-in slide-in-from-top-1.5 duration-150">
                
                {/* Search Header */}
                <div className="p-2.5 border-b border-zinc-100 dark:border-zinc-900 bg-zinc-50/50 dark:bg-black/10">
                  <div className="relative">
                    <Search className="absolute left-2.5 top-1/2 -translate-y-1/2 text-zinc-400 dark:text-zinc-500" size={13} />
                    <input
                      type="text"
                      value={searchQuery}
                      onChange={(e) => setSearchQuery(e.target.value)}
                      placeholder="搜索已打开的标签/文件..."
                      className="w-full pl-8 pr-6 py-1.5 bg-white dark:bg-zinc-900 text-xs border border-zinc-200 dark:border-zinc-800 rounded-xl focus:outline-none focus:ring-1 focus:ring-indigo-500/40 dark:focus:ring-indigo-400/45 font-semibold placeholder-zinc-400 dark:placeholder-zinc-600 text-zinc-800 dark:text-zinc-200"
                    />
                    {searchQuery && (
                      <button
                        onClick={() => setSearchQuery('')}
                        className="absolute right-2 top-1/2 -translate-y-1/2 text-zinc-400 hover:text-zinc-650 dark:text-zinc-500 dark:hover:text-zinc-300"
                      >
                        <X size={10} />
                      </button>
                    )}
                  </div>
                </div>

                {/* list scroll container */}
                <div className="flex-1 overflow-y-auto py-1 max-h-[190px] min-h-[60px] no-scrollbar">
                  {filteredItems.length === 0 ? (
                    <div className="p-5 text-center text-[11px] font-bold text-zinc-400 dark:text-zinc-550 italic">
                      未检索到匹配的标签
                    </div>
                  ) : (
                    filteredItems.map((item) => {
                      const isActive = activeId === item.id;
                      return (
                        <div
                          key={item.id}
                          className={`flex items-center justify-between px-3 py-2 text-xs font-bold cursor-pointer transition-colors group/item ${
                            isActive
                              ? 'bg-indigo-500/5 dark:bg-indigo-500/10 text-indigo-600 dark:text-indigo-400'
                              : 'text-zinc-600 dark:text-zinc-400 hover:bg-zinc-50 dark:hover:bg-zinc-900/60 hover:text-zinc-900 dark:hover:text-zinc-200'
                          }`}
                          onClick={() => {
                            onSelect?.(item);
                            setIsDropdownOpen(false);
                          }}
                        >
                          <div className="flex items-center gap-2 truncate flex-1 pr-2">
                            {getTabIcon && (
                              <span className={`shrink-0 opacity-80 ${isActive ? 'opacity-100 text-indigo-500' : ''}`}>
                                {getTabIcon(item.type)}
                              </span>
                            )}
                            <span className="truncate text-[11px]">
                              {item.title}
                            </span>
                          </div>
                          
                          <div className="flex items-center gap-1.5 shrink-0" onClick={(e) => e.stopPropagation()}>
                            {isActive && (
                              <div className="w-1.5 h-1.5 rounded-full bg-indigo-500 dark:bg-indigo-400 shrink-0" />
                            )}
                            {onClose && (
                              <button
                                onClick={(e) => {
                                  e.stopPropagation();
                                  onClose(item.id, e);
                                }}
                                className="opacity-0 group-hover/item:opacity-100 p-1 rounded-md hover:bg-red-50 dark:hover:bg-red-500/10 text-zinc-400 hover:text-red-500 transition-all shrink-0 outline-none"
                                title="关闭标签"
                              >
                                <X size={11} strokeWidth={2.5} />
                              </button>
                            )}
                          </div>
                        </div>
                      );
                    })
                  )}
                </div>

                {/* quick action footer bar */}
                {onClose && items.length > 1 && (
                  <div className="flex items-center justify-between gap-1.5 px-3 py-2.5 border-t border-zinc-100 dark:border-zinc-900 bg-zinc-50/50 dark:bg-black/20 shrink-0 text-[10.5px]">
                    <button
                      type="button"
                      onClick={() => {
                        const dummyEvent = {
                          stopPropagation: () => {},
                          preventDefault: () => {}
                        } as unknown as React.MouseEvent;
                        
                        // Close non-active items
                        items.forEach(item => {
                          if (item.id !== activeId) {
                            onClose(item.id, dummyEvent);
                          }
                        });
                        setIsDropdownOpen(false);
                      }}
                      className="text-zinc-500 hover:text-zinc-900 dark:text-zinc-400 dark:hover:text-zinc-200 font-extrabold hover:underline py-0.5 transition-colors outline-none"
                    >
                      关闭其他标签
                    </button>
                    <button
                      type="button"
                      onClick={() => {
                        const dummyEvent = {
                          stopPropagation: () => {},
                          preventDefault: () => {}
                        } as unknown as React.MouseEvent;
                        
                        // Clone items list to prevent index offset mutation race condition
                        [...items].forEach(item => {
                          onClose(item.id, dummyEvent);
                        });
                        setIsDropdownOpen(false);
                      }}
                      className="text-red-500 hover:text-red-600 dark:text-red-400 dark:hover:text-red-300 font-extrabold hover:underline py-0.5 transition-colors flex items-center gap-0.5 outline-none"
                    >
                      <FolderX size={11} strokeWidth={2.5} />
                      清空全部 ({items.length})
                    </button>
                  </div>
                )}
              </div>
            )}
          </div>
        )}

        {/* Existing right custom actions */}
        {rightActions && (
          <div className="flex items-center h-full">
            {rightActions}
          </div>
        )}
      </div>
    </div>
  );
}
