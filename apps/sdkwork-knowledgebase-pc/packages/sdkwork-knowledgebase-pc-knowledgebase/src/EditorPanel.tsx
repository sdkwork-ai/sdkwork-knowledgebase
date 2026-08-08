import React, { useState, useEffect } from 'react';
import { Share2, Pin, Sparkles, Music, Video, FileText, Code, Image as ImageIcon } from 'lucide-react';
import { Tabs } from './components/Tabs';
import { DocumentMeta, KnowledgeBase, FolderNode } from './services/document';
import { useTranslation } from 'react-i18next';
import { TiptapEditor } from './TiptapEditor';
import { CodeEditorPanel } from './CodeEditorPanel';
import { useHydratedViewerDocument } from './hooks/useHydratedViewerDocument';
import { AiAssistantPanel } from './AiAssistantPanel';
import { AssetLibraryModal } from './components/AssetLibraryModal';
import { sanitizeEditorHtml } from '@sdkwork/sdkwork-knowledgebase-pc-commons/htmlSanitizer';
import {
  isKnowledgebaseWorkspaceAiEnabled,
  type KnowledgebaseWorkspaceMode} from './workspaceMode';

const MediaViewer = React.lazy(async () => {
  const module = await import('./MediaViewer');
  return { default: module.MediaViewer };
});

const PdfViewer = React.lazy(async () => {
  const module = await import('./PdfViewer');
  return { default: module.PdfViewer };
});

export interface EditorPanelProps {
  activeKb: KnowledgeBase | null;
  activeDoc: DocumentMeta | null;
  openDocs?: DocumentMeta[];
  onSelectDoc?: (doc: DocumentMeta) => void;
  onCloseDoc?: (docId: string) => void;
  onCloseOthers?: (docId: string) => void;
  onCloseToRight?: (docId: string) => void;
  onCloseAll?: () => void;
  onTitleChange?: (docId: string, title: string) => void;
  docContent: string;
  loadingDocs: boolean;
  isAIOpen: boolean;
  onToggleAI: () => void;
  onContentChange: (content: string) => void;
  onPublishDoc: (doc: DocumentMeta) => void;
  onUpdateDocs?: () => void;
  docs?: (FolderNode | DocumentMeta)[];
  aiWidth?: number;
  isDraggingAi?: boolean;
  onMouseDownDragAi?: () => void;
  publishEnabled?: boolean;
  workspaceMode?: KnowledgebaseWorkspaceMode;
}

const getTabIcon = (type: string) => {
  switch (type) {
    case 'pdf':
      return <FileText size={13} className="text-rose-500 shrink-0" />;
    case 'code':
      return <Code size={13} className="text-orange-500 shrink-0" />;
    case 'image':
      return <ImageIcon size={13} className="text-blue-500 shrink-0" />;
    case 'video':
      return <Video size={13} className="text-sky-500 shrink-0" />;
    case 'audio':
      return <Music size={13} className="text-fuchsia-500 shrink-0" />;
    case 'richtext':
    case 'markdown':
      return <FileText size={13} className="text-emerald-500 shrink-0" />;
    default:
      return <FileText size={13} className="text-zinc-400 shrink-0" />;
  }
};

export function EditorPanel({
  activeKb,
  activeDoc,
  openDocs = [],
  onSelectDoc,
  onCloseDoc,
  onCloseOthers,
  onCloseToRight,
  onCloseAll,
  onTitleChange,
  docContent,
  loadingDocs,
  isAIOpen,
  onToggleAI,
  onContentChange,
  onPublishDoc,
  onUpdateDocs,
  docs = [],
  aiWidth = 420,
  isDraggingAi = false,
  onMouseDownDragAi,
  publishEnabled = true,
  workspaceMode = 'standard'}: EditorPanelProps) {
  const { t } = useTranslation('editor');
  const aiEnabled = isKnowledgebaseWorkspaceAiEnabled(workspaceMode);

  const [contextMenu, setContextMenu] = useState<{ x: number, y: number, docId: string, docTitle: string } | null>(null);

  // Asset Library State
  const [assetLibraryOpen, setAssetLibraryOpen] = useState(false);
  const [assetLibraryTab, setAssetLibraryTab] = useState<'image' | 'audio' | 'video'>('image');
  const [activeEditor, setActiveEditor] = useState<any>(null);
  const [transcribingDocs, setTranscribingDocs] = useState<Record<string, boolean>>({});
  const pdfViewDoc = useHydratedViewerDocument(activeDoc?.type === 'pdf' ? activeDoc : null);

  const insertHtmlToEditor = (html: string) => {
    if (activeEditor && !activeEditor.isDestroyed && typeof activeEditor.chain === 'function') {
      try {
        activeEditor.chain().focus().insertContent(sanitizeEditorHtml(html)).run();
      } catch (e) {
        console.error('Failed to insert HTML to editor:', e);
      }
    }
  };

  useEffect(() => {
    const handleCloseMenu = () => setContextMenu(null);
    window.addEventListener('click', handleCloseMenu);
    window.addEventListener('contextmenu', handleCloseMenu);
    return () => {
      window.removeEventListener('click', handleCloseMenu);
      window.removeEventListener('contextmenu', handleCloseMenu);
    };
  }, []);

  const handleTabContextMenu = (e: React.MouseEvent, doc: DocumentMeta) => {
    e.preventDefault();
    e.stopPropagation();
    setContextMenu({
      x: e.clientX,
      y: e.clientY,
      docId: doc.id,
      docTitle: doc.title
    });
  };

  const handleBarContextMenu = (e: React.MouseEvent) => {
    e.preventDefault();
    setContextMenu({
      x: e.clientX,
      y: e.clientY,
      docId: '',
      docTitle: t('tabsBar', { ns: 'editor', defaultValue: '标签栏' })
    });
  };

  return (
    <div className="flex-1 flex flex-col bg-white dark:bg-[var(--color-kb-editor)] overflow-hidden relative">
      {/* 1. VS Code-Style Tabs Bar with Scroll Buttons & Right Click Context Menu */}
      <Tabs 
        items={openDocs}
        activeId={activeDoc?.id}
        onSelect={onSelectDoc}
        onClose={(id) => onCloseDoc?.(id)}
        getTabIcon={getTabIcon}
        onContextMenu={handleTabContextMenu}
        onBarContextMenu={handleBarContextMenu}
        rightActions={
          <div className={`flex items-center h-[39px] px-3 gap-1.5 shrink-0 border-l border-zinc-200/80 dark:border-[var(--color-kb-panel-border)]/55 bg-zinc-50/50 dark:bg-black/10 select-none z-30 w-[190px] justify-end ${!publishEnabled && !aiEnabled ? 'hidden' : ''}`}>
            {publishEnabled && (
              <button
              type="button"
              disabled={!activeDoc}
              onClick={() => activeDoc && onPublishDoc(activeDoc)} 
              className="px-3 py-1 bg-indigo-600 dark:bg-[var(--color-kb-accent)] text-white text-[11.5px] font-bold rounded-lg hover:bg-indigo-700 dark:hover:bg-[var(--color-kb-accent-hover)] transition-all active:scale-95 shadow-sm shrink-0 disabled:opacity-40 disabled:cursor-not-allowed disabled:pointer-events-none"
            >
              {t('publish')}
            </button>
            )}
            {aiEnabled && (
            <div className="flex items-center space-x-1 border border-zinc-200/80 dark:border-transparent bg-white dark:bg-black/20 p-0.5 rounded-lg shadow-sm shrink-0">
              <button 
                disabled
                className="p-1 text-zinc-400 dark:text-zinc-600 rounded-md cursor-not-allowed opacity-60"
                title={t('shareComingSoon', { defaultValue: '分享功能即将上线' })}
              >
                <Share2 size={13} strokeWidth={2.5} />
              </button>
              <button 
                disabled
                className="p-1 text-zinc-400 dark:text-zinc-600 rounded-md cursor-not-allowed opacity-60"
                title={t('pinComingSoon', { defaultValue: '置顶功能即将上线' })}
              >
                <Pin size={13} strokeWidth={2.5} />
              </button>
              <div className="w-px h-3 bg-zinc-200 dark:bg-zinc-800 mx-0.5"></div>
              <button 
                disabled={!activeDoc}
                onClick={onToggleAI} 
                className={`p-1 rounded-md transition-all ${isAIOpen && activeDoc ? 'bg-indigo-100 text-indigo-600 dark:bg-[var(--color-kb-accent)]/20 dark:text-[var(--color-kb-accent)] font-semibold' : 'hover:bg-zinc-100 hover:text-zinc-900 dark:hover:bg-white/10 dark:text-zinc-500 dark:hover:text-zinc-200'} disabled:opacity-30 disabled:pointer-events-none`}
                title={t('aiWritingAssistant', { defaultValue: 'AI 智能写作助手' })}
              >
                <Sparkles size={13} strokeWidth={2.5} />
              </button>
            </div>
            )}
          </div>
        }
      />

      {/* Context Menu Overlay Portal */}
      {contextMenu && (
        <div 
          className="fixed z-[9999] bg-[var(--color-kb-editor)] border border-[var(--color-kb-panel-border)] rounded-lg shadow-xl py-1.5 w-52 text-xs text-[var(--color-kb-text)] select-none animate-in fade-in duration-100"
          style={{ left: `${contextMenu.x}px`, top: `${contextMenu.y}px` }}
        >
          {contextMenu.docId ? (
            <>
              <div className="px-3 py-1.5 font-semibold text-[10px] text-[var(--color-kb-text-muted)] uppercase tracking-wider border-b border-[var(--color-kb-panel-border)]/40 mb-1 max-w-[200px] truncate">
                {contextMenu.docTitle}
              </div>
              <button 
                type="button"
                onClick={() => { onCloseDoc?.(contextMenu.docId); setContextMenu(null); }}
                className="flex items-center w-full px-3 py-2 text-left hover:bg-[var(--color-kb-panel-hover)] hover:text-[var(--color-kb-text-heading)] transition-colors"
              >
                <span className="flex-1">{t('closeTab', { ns: 'editor' })}</span>
                <span className="text-[10px] text-[var(--color-kb-text-muted)] ml-auto">Ctrl+W</span>
              </button>
              <button 
                type="button"
                onClick={() => { onCloseOthers?.(contextMenu.docId); setContextMenu(null); }}
                className="flex items-center w-full px-3 py-2 text-left hover:bg-[var(--color-kb-panel-hover)] hover:text-[var(--color-kb-text-heading)] transition-colors"
              >
                <span className="flex-1">{t('closeOtherTabs', { ns: 'editor' })}</span>
              </button>
              <button 
                type="button"
                onClick={() => { onCloseToRight?.(contextMenu.docId); setContextMenu(null); }}
                className="flex items-center w-full px-3 py-2 text-left hover:bg-[var(--color-kb-panel-hover)] hover:text-[var(--color-kb-text-heading)] transition-colors"
              >
                <span className="flex-1">{t('closeTabsToRight', { ns: 'editor' })}</span>
              </button>
              <div className="h-px bg-[var(--color-kb-panel-border)]/40 my-1"></div>
              <button 
                type="button"
                onClick={() => { onSelectDoc?.(openDocs.find(d => d.id === contextMenu.docId)!); onCloseAll?.(); setContextMenu(null); }}
                className="flex items-center w-full px-3 py-2 text-left text-red-500 hover:bg-red-500/10 transition-colors"
              >
                <span className="flex-1 font-medium">{t('closeAllTabs', { ns: 'editor' })}</span>
              </button>
              <button 
                type="button"
                onClick={() => { navigator.clipboard.writeText(contextMenu.docTitle); setContextMenu(null); }}
                className="flex items-center w-full px-3 py-2 text-left hover:bg-[var(--color-kb-panel-hover)] hover:text-[var(--color-kb-text-heading)] transition-colors"
              >
                <span className="flex-1">{t('copyTabTitle', { ns: 'editor' })}</span>
              </button>
            </>
          ) : (
            <>
              <div className="px-3 py-1.5 font-semibold text-[10px] text-[var(--color-kb-text-muted)] uppercase tracking-wider border-b border-[var(--color-kb-panel-border)]/40 mb-1 max-w-[200px] truncate">
                {contextMenu.docTitle}
              </div>
              <button 
                type="button"
                disabled={openDocs.length === 0}
                onClick={() => { onCloseAll?.(); setContextMenu(null); }}
                className={`flex items-center w-full px-3 py-2 text-left text-red-500 transition-colors ${openDocs.length > 0 ? 'hover:bg-red-500/10 cursor-pointer' : 'opacity-40 cursor-not-allowed'}`}
              >
                <span className="flex-1 font-medium">{t('closeAllTabs', { ns: 'editor' })}</span>
              </button>
            </>
          )}
        </div>
      )}

      {/* Main split workspace beneath the tabs bar */}
      <div className="flex-1 flex flex-row min-h-0 w-full overflow-hidden relative">
        {/* Left: Interactive Editor Area */}
        <div className="flex-1 flex flex-col min-h-0 overflow-hidden relative">
          {/* 3. Editor Content Container */}
          <div className="flex-1 flex flex-col min-h-0 overflow-hidden w-full">
            {activeDoc?.type === 'richtext' && (
              <div className="w-full h-full flex-col flex-1 min-h-0">
                <div className="w-full flex-1 flex flex-col min-h-0">
                  {docContent === 'Loading...' ? (
                    <div className="animate-pulse flex space-x-4 p-8">
                      <div className="flex-1 space-y-6 py-1">
                        <div className="h-4 bg-[var(--color-kb-panel-border)] rounded w-3/4"></div>
                        <div className="space-y-3">
                          <div className="h-4 bg-[var(--color-kb-panel-border)] rounded"></div>
                          <div className="h-4 bg-[var(--color-kb-panel-border)] rounded w-5/6"></div>
                        </div>
                      </div>
                    </div>
                  ) : (
                    <TiptapEditor 
                      key={activeDoc.id} 
                      initialContent={docContent} 
                      mode="richtext" 
                      onChange={onContentChange}
                      docTitle={activeDoc.title}
                      onTitleChange={(newTitle) => onTitleChange?.(activeDoc.id, newTitle)}
                      onEditorReady={setActiveEditor}
                      kbId={activeKb?.id}
                      workspaceMode={workspaceMode}
                      parentFolderId={activeDoc.parentId}
                      onOpenImageGallery={() => {
                        setAssetLibraryTab('image');
                        setAssetLibraryOpen(true);
                      }}
                      onAudioGallery={() => {
                        setAssetLibraryTab('audio');
                        setAssetLibraryOpen(true);
                      }}
                      onVideoGallery={() => {
                        setAssetLibraryTab('video');
                        setAssetLibraryOpen(true);
                      }}
                    />
                  )}
                </div>
              </div>
            )}

            {activeDoc?.type === 'markdown' && (
              <div className="w-full h-full flex flex-col p-0 flex-1 min-h-0">
                <div className="w-full flex-1 flex flex-col min-h-0">
                  {docContent === 'Loading...' ? (
                    <div className="animate-pulse flex space-x-4 p-6">
                      <div className="flex-1 space-y-6 py-1">
                        <div className="h-4 bg-[var(--color-kb-panel-border)] rounded w-3/4"></div>
                        <div className="space-y-3">
                          <div className="h-4 bg-[var(--color-kb-panel-border)] rounded"></div>
                          <div className="h-4 bg-[var(--color-kb-panel-border)] rounded w-5/6"></div>
                        </div>
                      </div>
                    </div>
                  ) : (
                    <div className="w-full h-full flex-1 flex flex-col min-h-0">
                      <TiptapEditor 
                        key={activeDoc.id} 
                        initialContent={docContent} 
                        mode="markdown" 
                        onChange={onContentChange}
                        docTitle={activeDoc.title}
                        onTitleChange={(newTitle) => onTitleChange?.(activeDoc.id, newTitle)}
                      onEditorReady={setActiveEditor}
                      kbId={activeKb?.id}
                      workspaceMode={workspaceMode}
                      parentFolderId={activeDoc.parentId}
                        onOpenImageGallery={() => {
                          setAssetLibraryTab('image');
                          setAssetLibraryOpen(true);
                        }}
                        onAudioGallery={() => {
                          setAssetLibraryTab('audio');
                          setAssetLibraryOpen(true);
                        }}
                        onVideoGallery={() => {
                          setAssetLibraryTab('video');
                          setAssetLibraryOpen(true);
                        }}
                      />
                    </div>
                  )}
                </div>
              </div>
            )}

            {activeDoc?.type === 'code' && (
              <CodeEditorPanel 
                key={activeDoc.id}
                activeDoc={activeDoc} 
                docContent={docContent} 
                onContentChange={onContentChange}
              />
            )}

            {activeDoc?.type === 'pdf' && pdfViewDoc && (
              <React.Suspense fallback={<div className="h-full w-full bg-[var(--color-kb-editor)]" />}>
                <PdfViewer activeDoc={pdfViewDoc} />
              </React.Suspense>
            )}

            {activeDoc && ['image', 'video', 'audio', 'music', 'file'].includes(activeDoc.type) && (
              <React.Suspense fallback={<div className="h-full w-full bg-[var(--color-kb-editor)]" />}>
                <MediaViewer
                  activeDoc={activeDoc}
                  docContent={docContent}
                  activeKb={activeKb}
                  onUpdateDocs={onUpdateDocs}
                  onContentChange={(content) => {
                    onContentChange(content);
                  }}
                  isTranscribing={!!transcribingDocs[activeDoc.id]}
                  onTranscribeStart={() => {
                    setTranscribingDocs(prev => ({ ...prev, [activeDoc.id]: true }));
                  }}
                  onTranscribeComplete={(text) => {
                    onContentChange(text);
                    setTranscribingDocs(prev => ({ ...prev, [activeDoc.id]: false }));
                  }}
                  onTitleChange={(newTitle) => {
                    if (onTitleChange) {
                      onTitleChange(activeDoc.id, newTitle);
                    }
                  }}
                  workspaceMode={workspaceMode}
                />
              </React.Suspense>
            )}

            {!activeDoc && !loadingDocs && (
              <div className="w-full flex-1 flex flex-col items-center justify-center bg-[var(--color-kb-panel)]/[0.03] min-h-0 select-none">
                <div className="w-24 h-24 rounded-3xl bg-[var(--color-kb-editor)] flex items-center justify-center mb-6 border border-[var(--color-kb-panel-border)]/40 shadow-[0_8px_30px_rgba(0,0,0,0.04)] relative">
                  <div className="absolute inset-0 bg-gradient-to-br from-[var(--color-kb-panel-hover)] to-transparent rounded-3xl opacity-50"></div>
                  <div className="relative z-10 flex items-center justify-center">
                    <FileText size={40} className="text-[var(--color-kb-text-muted)] opacity-60" strokeWidth={1} />
                    <Sparkles size={16} className="text-[var(--color-kb-accent)] absolute -top-1 -right-2 opacity-80" />
                  </div>
                </div>
                <h3 className="text-[18px] font-medium text-[var(--color-kb-text-heading)] mb-2 tracking-wide">
                  {t('noFileSelected', { ns: 'editor' })}
                </h3>
                <p className="text-[14px] text-[var(--color-kb-text-muted)] max-w-xs text-center leading-relaxed">
                  {t('selectDocForPreviewEdit', { ns: 'editor' })}
                </p>
              </div>
            )}
          </div>
        </div>

        {/* Right: AI Assistant Workspace Panel */}
        {aiEnabled && isAIOpen && (
          <AiAssistantPanel 
            aiWidth={aiWidth}
            isDraggingAi={isDraggingAi}
            onMouseDownDrag={onMouseDownDragAi!}
            onClose={onToggleAI}
            docContent={docContent}
            docs={docs}
            activeDoc={activeDoc}
            activeKbId={activeKb?.id}
            headerHeightClass="h-[40px]"
            workspaceMode={workspaceMode}
          />
        )}
      </div>

      <AssetLibraryModal
        isOpen={assetLibraryOpen}
        onClose={() => setAssetLibraryOpen(false)}
        initialTab={assetLibraryTab}
        title={t('selectMaterialToInsert', { ns: 'editor' })}
        kbId={activeKb?.id}
        onSelect={(item) => {
          if (item.type === 'image') {
            insertHtmlToEditor(`<p><img src="${item.url}" alt="${item.title}" style="border-radius: 12px; max-width: 100%; margin: 16px 0; border: 1px solid var(--color-kb-panel-border);" /></p>`);
          } else if (item.type === 'video') {
            insertHtmlToEditor(`<video src="${item.url}" controls></video>`);
          } else if (item.type === 'audio') {
            insertHtmlToEditor(`<audio src="${item.url}" controls></audio>`);
          }
          setAssetLibraryOpen(false);
        }}
      />
    </div>
  );
}
