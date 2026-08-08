import { useState, useMemo, useRef, useEffect } from 'react';
import { createPortal } from 'react-dom';
import { DocumentMeta } from './services/document';
import { CodeEditor } from './CodeEditor';
import { FileCode2, Eye, MonitorSmartphone, Smartphone, Tablet, RefreshCw, Maximize } from 'lucide-react';
import type { ReactKeyedComponentProps } from '@sdkwork/sdkwork-knowledgebase-pc-commons/reactKeyedProps';
import { buildSandboxedHtmlPreview } from './services/sandboxedHtmlPreview';

export interface CodeEditorPanelProps extends ReactKeyedComponentProps {
  activeDoc: DocumentMeta;
  docContent: string;
  onContentChange: (content: string) => void;
}

export function CodeEditorPanel({ activeDoc, docContent, onContentChange }: CodeEditorPanelProps) {
  const [viewMode, setViewMode] = useState<'code' | 'preview'>('preview');
  const [device, setDevice] = useState<'current' | 'mobile' | 'tablet'>('current');
  const [, setDeviceMenuOpen] = useState(false);
  const [refreshKey, setRefreshKey] = useState(0);
  const [isRefreshing, setIsRefreshing] = useState(false);
  const menuRef = useRef<HTMLDivElement>(null);
  const previewDocument = useMemo(
    () => buildSandboxedHtmlPreview(docContent),
    [docContent],
  );
  
  useEffect(() => {
    const handleClickOutside = (event: MouseEvent) => {
      if (menuRef.current && !menuRef.current.contains(event.target as Node)) {
        setDeviceMenuOpen(false);
      }
    };
    document.addEventListener('mousedown', handleClickOutside);
    return () => document.removeEventListener('mousedown', handleClickOutside);
  }, []);

  const language = useMemo(() => {
    const ext = activeDoc.title.split('.').pop()?.toLowerCase();
    switch (ext) {
      case 'ts':
      case 'tsx':
        return 'typescript';
      case 'js':
      case 'jsx':
        return 'javascript';
      case 'html':
      case 'htm':
        return 'html';
      case 'css':
        return 'css';
      case 'json':
        return 'json';
      case 'xml':
        return 'xml';
      case 'py':
        return 'python';
      case 'java':
        return 'java';
      case 'cpp':
      case 'c':
        return 'cpp';
      case 'go':
        return 'go';
      case 'rs':
        return 'rust';
      case 'php':
        return 'php';
      case 'rb':
        return 'ruby';
      case 'swift':
        return 'swift';
      case 'kt':
        return 'kotlin';
      case 'sql':
        return 'sql';
      case 'sh':
        return 'shell';
      case 'yaml':
      case 'yml':
        return 'yaml';
      default:
        return 'plaintext';
    }
  }, [activeDoc.title]);

  const isHtml = language === 'html';
  
  const getDeviceStyle = () => {
    if (device === 'current') return { width: '100%', height: '100%' };
    if (device === 'mobile') return { width: '375px', height: '812px', maxHeight: '100%' };
    if (device === 'tablet') return { width: '768px', height: '1024px', maxHeight: '100%' };
    return { width: '100%', height: '100%' };
  };

  const toolbarContent = isHtml ? (
    <div className="flex items-center w-full justify-between gap-3 overflow-x-auto no-scrollbar py-0.5">
      {/* 1. Left Side: Compact Segmented Mode Switcher (Aligned left to replace redundant logo/branding) */}
      <div className="flex items-center bg-zinc-100/80 dark:bg-zinc-900/55 p-0.5 rounded-lg border-none relative shrink-0">
        <button
          type="button"
          onClick={() => setViewMode('preview')}
          className={`relative flex items-center gap-1.5 h-6.5 px-3 text-[11px] font-semibold rounded-[6px] transition-all duration-150 outline-none select-none active:scale-95 ${
            viewMode === 'preview' 
              ? 'bg-white dark:bg-zinc-800 text-[var(--color-kb-accent)] shadow-sm' 
              : 'text-zinc-500 dark:text-zinc-400 hover:text-zinc-800 dark:hover:text-zinc-200'
          }`}
        >
          <Eye size={12.5} strokeWidth={2.5} />
          <span>预览效果</span>
        </button>
        
        <button
          type="button"
          onClick={() => setViewMode('code')}
          className={`relative flex items-center gap-1.5 h-6.5 px-3 text-[11px] font-semibold rounded-[6px] transition-all duration-150 outline-none select-none active:scale-95 ${
            viewMode === 'code' 
              ? 'bg-white dark:bg-zinc-800 text-[var(--color-kb-accent)] shadow-sm' 
              : 'text-zinc-500 dark:text-zinc-400 hover:text-zinc-800 dark:hover:text-zinc-200'
          }`}
        >
          <FileCode2 size={12.5} strokeWidth={2.5} />
          <span>编辑源码</span>
        </button>
      </div>

      {/* 2. Right Side: Responsive Device Switches & Utility tools */}
      <div className="flex items-center gap-2.5 shrink-0 ml-auto">
        {viewMode === 'preview' && (
          <div className="flex items-center bg-zinc-100/80 dark:bg-zinc-900/55 p-0.5 rounded-lg border-none">
            {[
              { id: 'current', label: '自适应', icon: MonitorSmartphone, spec: 'Full Width' },
              { id: 'tablet', label: '平板端', icon: Tablet, spec: '768 × 1024' },
              { id: 'mobile', label: '手机端', icon: Smartphone, spec: '375 × 812' }
            ].map((item) => {
              const ItemIcon = item.icon;
              const isSelected = device === item.id;
              return (
                <button
                  key={item.id}
                  type="button"
                  onClick={() => setDevice(item.id as any)}
                  className={`h-6.5 px-2.5 rounded-[6px] transition-all duration-150 flex items-center gap-1.2 text-[10px] font-semibold outline-none select-none active:scale-95 ${
                    isSelected 
                      ? 'bg-white dark:bg-zinc-800 shadow-sm text-[var(--color-kb-accent)]' 
                      : 'text-zinc-400 dark:text-zinc-550 hover:text-zinc-700 dark:hover:text-zinc-300'
                  }`}
                  title={`${item.label} (${item.spec})`}
                >
                  <ItemIcon size={12} strokeWidth={2.5} />
                  <span className="hidden lg:inline text-[9.5px] tracking-tight">{item.label}</span>
                </button>
              );
            })}
          </div>
        )}
        
        {viewMode === 'preview' && <div className="h-3.5 w-px bg-zinc-200/60 dark:bg-zinc-800/60 shrink-0"></div>}

        <div className="flex items-center gap-1 shrink-0">
          {viewMode === 'preview' && (
            <button 
              type="button"
              onClick={() => {
                setIsRefreshing(true);
                setRefreshKey(k => k + 1);
                setTimeout(() => setIsRefreshing(false), 550);
              }} 
              className="h-6.5 w-6.5 flex items-center justify-center rounded-lg border border-zinc-200/30 dark:border-zinc-800/40 bg-zinc-50/50 dark:bg-[#121214] hover:bg-zinc-100 dark:hover:bg-zinc-800 text-zinc-500 dark:text-zinc-400 hover:text-zinc-800 dark:hover:text-zinc-200 active:scale-95 transition-all outline-none"
              title="重载预览"
            >
              <RefreshCw size={11.5} strokeWidth={2.5} className={isRefreshing ? "animate-spin text-[var(--color-kb-accent)]" : ""} />
            </button>
          )}
          
          <button 
            type="button"
            onClick={() => {
              if (document.fullscreenElement) { 
                document.exitFullscreen(); 
              } else { 
                document.documentElement.requestFullscreen(); 
              }
            }} 
            className="h-6.5 w-6.5 flex items-center justify-center rounded-lg border border-zinc-200/30 dark:border-zinc-800/40 bg-zinc-50/50 dark:bg-[#121214] hover:bg-zinc-100 dark:hover:bg-zinc-800 text-zinc-500 dark:text-zinc-400 hover:text-zinc-800 dark:hover:text-zinc-200 active:scale-95 transition-all outline-none"
            title="全屏演示"
          >
            <Maximize size={11.5} strokeWidth={2.5} />
          </button>
        </div>
      </div>
    </div>
  ) : null;

  const [htmlToolbarElement, setHtmlToolbarElement] = useState<HTMLElement | null>(null);

  useEffect(() => {
    setHtmlToolbarElement(document.getElementById('html-toolbar-portal'));
  }, []);

  return (
    <div className="w-full h-full flex flex-col min-h-0 bg-[var(--color-kb-editor)]">
      {htmlToolbarElement && toolbarContent ? (
        createPortal(toolbarContent, htmlToolbarElement)
      ) : (
        toolbarContent && (
          <div className="w-full border-b border-transparent bg-[var(--color-kb-panel)]/95 select-none h-[40px] flex items-center px-3.5 shrink-0 overflow-hidden">
            {toolbarContent}
          </div>
        )
      )}

      <div className="flex-1 w-full relative min-h-0 min-w-0 flex bg-[var(--color-kb-panel-hover)]">
        {docContent === 'Loading...' ? (
          <div className="absolute inset-4 animate-pulse bg-[var(--color-kb-panel-border)] rounded-lg"></div>
        ) : (
          <>
            {(viewMode === 'code' || !isHtml) && (
              <div className="p-0 flex flex-col min-h-0 w-full bg-[var(--color-kb-editor)]">
                <div className="flex-1 w-full relative">
                  <CodeEditor key={activeDoc.id} initialContent={docContent} language={language} onChange={onContentChange} />
                </div>
              </div>
            )}
            
            {(isHtml && viewMode === 'preview') && (
              <div className={`flex-1 w-full h-full flex items-center justify-center overflow-auto ${device === 'current' ? 'p-0' : 'p-4'}`}>
                <div 
                   style={getDeviceStyle()} 
                   className={`bg-white transition-all duration-300 ${device !== 'current' ? 'shadow-[0_0_20px_rgba(0,0,0,0.3)] rounded-3xl overflow-hidden border-8 border-gray-900 border-t-[24px] border-b-[24px]' : ''}`}
                >
                  <iframe
                    key={`${activeDoc.id}-${refreshKey}`}
                    srcDoc={previewDocument}
                    title="HTML Preview"
                    className="w-full h-full border-none bg-white font-sans text-sm"
                    referrerPolicy="no-referrer"
                    sandbox=""
                  />
                </div>
              </div>
            )}
          </>
        )}
      </div>
    </div>
  );
}
