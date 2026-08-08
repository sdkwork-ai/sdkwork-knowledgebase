import { useState, useRef, useEffect, useLayoutEffect, useMemo } from 'react';
import { createPortal } from 'react-dom';
import { useEditor, EditorContent } from '@tiptap/react';
import { Extension } from '@tiptap/core';
import Placeholder from '@tiptap/extension-placeholder';
import '@tiptap/react/menus';
import StarterKit from '@tiptap/starter-kit';
import Image from '@tiptap/extension-image';
import { Markdown } from 'tiptap-markdown';
import { Sparkles } from 'lucide-react';
import { useTranslation } from 'react-i18next';
import { WechatAppletModal } from './components/WechatAppletModal';
import { UniversalToolbar, UniversalToolbarGroup } from './components/UniversalToolbar';
import { DropdownItem, DropdownDivider } from './components/InsertToolsMenu';
import { WechatMiniprogram } from './extensions/WechatMiniprogram';
import { Audio } from './extensions/Audio';
import { Video as VideoExtension } from './extensions/Video';
import { EditorBubbleMenu } from './EditorBubbleMenu';
import { AIService } from './services/ai';
import { DocumentService } from './services/document';
import { toastKnowledgebaseError } from './components/ui/toastKnowledgebaseError';
import './components/ui/toast-manager';
import { isKnowledgebaseApiAvailable, KnowledgebaseErrorCodes, shouldUseKnowledgebaseDemoFallback, throwKnowledgebaseError } from 'sdkwork-knowledgebase-pc-core';
import { createTiptapExportContentProvider } from './components/DocumentExport';
import { sanitizeEditorHtml } from '@sdkwork/sdkwork-knowledgebase-pc-commons/htmlSanitizer';
import { resolveDriveNodeDownloadUrl } from './services/knowledgeDriveMediaService';
import type { ReactKeyedComponentProps } from '@sdkwork/sdkwork-knowledgebase-pc-commons/reactKeyedProps';
import {
  isKnowledgebaseWorkspaceAiEnabled,
  resolveKnowledgebaseWorkspaceAiScope,
  type KnowledgebaseWorkspaceMode,
} from './workspaceMode';

export interface TiptapEditorProps extends ReactKeyedComponentProps {
  initialContent: string;
  mode?: 'richtext' | 'markdown';
  onChange?: (content: string) => void;
  onEditorReady?: (editor: any) => void;
  docTitle?: string;
  onTitleChange?: (title: string) => void;
  hideTitle?: boolean;
  onOpenImageGallery?: () => void;
  onWechatScan?: () => void;
  onOpenAiImage?: () => void;
  onAudioInsert?: () => void;
  onAudioGallery?: () => void;
  onVideoGallery?: () => void;
  toolbarConfig?: UniversalToolbarGroup[];
  kbId?: string | null;
  parentFolderId?: string | null;
  workspaceMode?: KnowledgebaseWorkspaceMode;
}

interface EditorMediaRef {
  src: string;
  driveUri?: string;
  driveSpaceId?: string;
  driveNodeId?: string;
}

const DriveBackedImage = Image.extend({
  addAttributes() {
    return {
      ...this.parent?.(),
      driveSpaceId: {
        default: null,
        parseHTML: element => element.getAttribute('data-drive-space-id'),
        renderHTML: attributes => attributes.driveSpaceId
          ? { 'data-drive-space-id': attributes.driveSpaceId }
          : {},
      },
      driveNodeId: {
        default: null,
        parseHTML: element => element.getAttribute('data-drive-node-id'),
        renderHTML: attributes => attributes.driveNodeId
          ? { 'data-drive-node-id': attributes.driveNodeId }
          : {},
      },
      driveUri: {
        default: null,
        parseHTML: element => element.getAttribute('data-drive-uri'),
        renderHTML: attributes => attributes.driveUri
          ? { 'data-drive-uri': attributes.driveUri }
          : {},
      },
    };
  },
});

const StyleGlobalExtension = Extension.create({
  name: 'styleGlobal',
  addGlobalAttributes() {
    return [
      {
        types: [
          'heading', 'paragraph', 'blockquote', 'listItem', 'bulletList', 'orderedList', 'image',
          'bold', 'italic', 'strike'
        ],
        attributes: {
          style: {
            default: null,
            parseHTML: element => element.getAttribute('style'),
            renderHTML: attributes => {
              if (!attributes.style) {
                return {}
              }
              return { style: attributes.style }
            }
          }
        }
      }
    ];
  }
});

export function TiptapEditor({ 
  initialContent, mode = 'richtext', onChange, onEditorReady, docTitle = '', onTitleChange, hideTitle = false,
  onOpenImageGallery, onWechatScan, onOpenAiImage, onAudioGallery, onVideoGallery, toolbarConfig,
  kbId, parentFolderId, workspaceMode = 'standard',
}: TiptapEditorProps) {
  const { t } = useTranslation('editor');
  const { t: tErrors } = useTranslation('errors');
  const aiEnabled = isKnowledgebaseWorkspaceAiEnabled(workspaceMode);
  const isEphemeralFixedWorkspace = workspaceMode === 'ephemeral-fixed';
  const [title, setTitle] = useState(docTitle);

  useEffect(() => {
    setTitle(docTitle);
  }, [docTitle]);

  const handleTitleChangeLocal = (newTitle: string) => {
    setTitle(newTitle);
    onTitleChange?.(newTitle);
  };
  const [aiLoading, setAiLoading] = useState(false);
  const [isWechatAppletModalOpen, setIsWechatAppletModalOpen] = useState(false);
  const [isSourceMode, setIsSourceMode] = useState(false);
  const [isSplitMode, setIsSplitMode] = useState(false);
  const [sourceCode, setSourceCode] = useState(initialContent);
  const fileInputRef = useRef<HTMLInputElement>(null);
  const videoInputRef = useRef<HTMLInputElement>(null);
  const audioInputRef = useRef<HTMLInputElement>(null);
  const editorRef = useRef<any>(null);

  const uploadEditorMedia = async (
    file: File,
    mediaType: 'image' | 'audio' | 'video',
  ): Promise<EditorMediaRef | null> => {
    if (isKnowledgebaseApiAvailable()) {
      if (!kbId) {
        try {
          throwKnowledgebaseError(KnowledgebaseErrorCodes.KB_ID_REQUIRED);
        } catch (error) {
          toastKnowledgebaseError(error, tErrors);
        }
        return null;
      }
      try {
        const uploaded = await DocumentService.uploadFiles(
          [file],
          kbId,
          parentFolderId ?? undefined,
          mediaType,
        );
        const media = uploaded[0];
        if (!media?.driveUri || !media.driveSpaceId || !media.driveNodeId) {
          throwKnowledgebaseError(KnowledgebaseErrorCodes.MEDIA_URL_UNRESOLVED);
        }
        return {
          src: media.driveUri,
          driveUri: media.driveUri,
          driveSpaceId: media.driveSpaceId,
          driveNodeId: media.driveNodeId,
        };
      } catch (error) {
        toastKnowledgebaseError(error, tErrors);
        return null;
      }
    }

    if (!shouldUseKnowledgebaseDemoFallback()) {
      try {
        throwKnowledgebaseError(KnowledgebaseErrorCodes.API_UNAVAILABLE);
      } catch (error) {
        toastKnowledgebaseError(error, tErrors);
      }
      return null;
    }

    return { src: URL.createObjectURL(file) };
  };

  const uploadImage = async (file: File): Promise<EditorMediaRef | null> =>
    uploadEditorMedia(file, 'image');

  const insertEditorMedia = async (
    file: File,
    mediaType: 'audio' | 'video',
    curEditor: any,
  ) => {
    const media = await uploadEditorMedia(file, mediaType);
    if (!media) {
      return;
    }
    curEditor?.commands.insertContent({
      type: mediaType,
      attrs: { ...media, controls: true },
    });
  };

  const handleImageFiles = async (files: FileList | null | undefined, curEditor: any) => {
    if (!files) return;
    for (let i = 0; i < files.length; i++) {
      const file = files[i];
      if (file.type.startsWith('image/')) {
        const media = await uploadImage(file);
        if (media) {
          curEditor.chain().focus().setImage(media).run();
        }
      }
    }
  };

  const extensions = [
    StarterKit,
    StyleGlobalExtension,
    WechatMiniprogram,
    Audio,
    VideoExtension,
    Placeholder.configure({
      placeholder: t('placeholderText', { defaultValue: '请输入正文或开始创作...' })
    }),
    DriveBackedImage.configure({
       HTMLAttributes: {
         class: 'rounded-lg max-w-full my-4 border border-[var(--color-kb-panel-border)] shadow-sm'
       }
    })
  ];
  
  if (mode === 'markdown') {
    extensions.push(Markdown);
  }

  const editor = useEditor({
    extensions,
    content: mode === 'markdown' ? initialContent : sanitizeEditorHtml(initialContent),
    editorProps: {
      attributes: {
        class: 'tiptap-editor outline-none focus:outline-none w-full min-h-[400px]',
      },
      transformPastedHTML: (html: string) => sanitizeEditorHtml(html),
      handleDrop: (_view, event,_slice, moved) => {
        if (!moved && event.dataTransfer && event.dataTransfer.files && event.dataTransfer.files.length > 0) {
          event.preventDefault();
          if (editorRef.current) {
            handleImageFiles(event.dataTransfer.files, editorRef.current);
          }
          return true;
        }
        return false;
      },
      handlePaste: (_view, event,_slice) => {
        if (event.clipboardData && event.clipboardData.files && event.clipboardData.files.length > 0) {
          event.preventDefault();
          if (editorRef.current) {
            handleImageFiles(event.clipboardData.files, editorRef.current);
          }
          return true;
        }
        return false;
      }
    },
    onUpdate: ({ editor }) => {
      const raw = mode === 'markdown' ? (editor.storage as any).markdown.getMarkdown() : editor.getHTML();
      const data = mode === 'markdown' ? raw : sanitizeEditorHtml(raw);
      onChange?.(data);
      if (document.activeElement?.tagName !== 'TEXTAREA') {
         setSourceCode(data);
      }
    }
  });

  editorRef.current = editor;

  useEffect(() => {
    if (!editor) {
      return undefined;
    }
    let disposed = false;
    const resolveMedia = async () => {
      const elements = editor.view.dom.querySelectorAll<HTMLElement>('[data-drive-node-id]');
      await Promise.all(Array.from(elements).map(async (element) => {
        const nodeId = element.dataset.driveNodeId?.trim();
        if (!nodeId || element.dataset.driveResolvedNodeId === nodeId) {
          return;
        }
        try {
          const url = await resolveDriveNodeDownloadUrl(nodeId);
          if (!disposed && url) {
            element.setAttribute('src', url);
            element.dataset.driveResolvedNodeId = nodeId;
          }
        } catch {
          // Stable Drive identity remains saved even when transient delivery is unavailable.
        }
      }));
    };
    const handleUpdate = () => { void resolveMedia(); };
    editor.on('update', handleUpdate);
    void resolveMedia();
    return () => {
      disposed = true;
      editor.off('update', handleUpdate);
    };
  }, [editor]);

  useEffect(() => {
    if (editor && onEditorReady) {
      onEditorReady(editor);
    }
    return () => {
      if (onEditorReady) {
        onEditorReady(null);
      }
    };
  }, [editor, onEditorReady]);

  const handleToggleSourceMode = () => {
    // If we transition out of source mode, update the editor
    if (isSourceMode && !isSplitMode) {
      if (editor) {
        const sanitized = mode === 'markdown' ? sourceCode : sanitizeEditorHtml(sourceCode);
        editor.commands.setContent(sanitized);
        onChange?.(sanitized);
      }
    } else {
      if (editor) {
        const data = mode === 'markdown' ? (editor.storage as any).markdown.getMarkdown() : editor.getHTML();
        setSourceCode(data);
      }
    }
    setIsSourceMode(!isSourceMode);
    if (isSplitMode) setIsSplitMode(false);
  };

  const getExportContent = useMemo(
    () =>
      createTiptapExportContentProvider(() => {
        if (!editor) {
          return null;
        }

        return {
          title: title || '无标题',
          mode,
          isSourceMode,
          isSplitMode,
          sourceCode,
          getHtml: () => editor.getHTML(),
          getMarkdown: () => {
            try {
              if ((editor.storage as any).markdown) {
                return (editor.storage as any).markdown.getMarkdown();
              }
            } catch {
              // fall through to plain text
            }
            return editor.getText();
          },
          getPlainText: () => editor.getText(),
        };
      }),
    [editor, title, mode, isSourceMode, isSplitMode, sourceCode],
  );

  const handleAiAction = async (action: string, customPrompt?: string) => {
    if (!editor) return;

    if (!aiEnabled) {
      try {
        throwKnowledgebaseError(KnowledgebaseErrorCodes.API_UNAVAILABLE_AI);
      } catch (error) {
        toastKnowledgebaseError(error, tErrors);
      }
      return;
    }
    
    setAiLoading(true);
    const selection = editor.state.selection;
    const text = editor.state.doc.textBetween(selection.from, selection.to) || editor.getText();
    const context = editor.getText();
    
    try {
      const scope = resolveKnowledgebaseWorkspaceAiScope(workspaceMode, kbId);
      const result = await AIService.handleAIAction(action, text, context, customPrompt, scope);
      const safeResult = mode === 'markdown' ? result : sanitizeEditorHtml(result);
      editor.commands.insertContent(safeResult);
    } catch (e: unknown) {
      toastKnowledgebaseError(e, tErrors);
    } finally {
      setAiLoading(false);
    }
  };

  const [portalNode, setPortalNode] = useState<Element | null>(null);

  useLayoutEffect(() => {
    setPortalNode(document.getElementById('editor-toolbar-portal'));
  }, []);

  if (!editor) {
    return null;
  }

  const defaultInsertTools = [
    {
      name: t('image'),
      hasDropdown: true,
      onClick: () => {},
      dropdownContent: (
        <>
          <DropdownItem onClick={() => fileInputRef.current?.click()} text={t('uploadLocalImage')} />
          <DropdownItem onClick={() => onOpenImageGallery?.()} text={t('selectFromImageGallery')} />
          <DropdownItem onClick={() => onWechatScan?.()} text={t('wechatScanUpload')} />
          <DropdownDivider />
          <DropdownItem 
             onClick={() => onOpenAiImage?.()} 
             icon={<Sparkles size={11} className="text-amber-500 animate-pulse" />} 
             text={t('aiImageGen')} 
             highlighted 
          />
        </>
      )
    },
    {
      name: t('audio'),
      hasDropdown: true,
      onClick: () => {},
      dropdownContent: (
        <>
          <DropdownItem onClick={() => audioInputRef.current?.click()} text={t('uploadLocalAudio')} />
          <DropdownItem onClick={() => audioInputRef.current?.click()} text={t('uploadLocalMusic')} />
          <DropdownItem onClick={() => onAudioGallery?.()} text={t('selectFromAssets')} />
          <DropdownItem onClick={() => onAudioGallery?.()} text={t('aiAudioGen')} />
          <DropdownItem onClick={() => onAudioGallery?.()} text={t('aiMusicGen')} />
        </>
      )
    },
    {
      name: t('video'),
      hasDropdown: true,
      onClick: () => {},
      dropdownContent: (
        <>
          <DropdownItem onClick={() => videoInputRef.current?.click()} text={t('uploadLocalVideo')} />
          <DropdownItem onClick={() => onVideoGallery?.()} text={t('selectFromVideos')} />
        </>
      )
    }
  ];

  const defaultConfig: UniversalToolbarGroup[] = [
    { type: 'typography' },
    { type: 'format' },
    { type: 'list' },
    { type: 'insert', tools: defaultInsertTools },
    { type: 'history' },
    { type: 'view' },
    { type: 'export' }
  ];
  const workspaceToolbarConfig = (toolbarConfig || defaultConfig).filter(
    (group) => !isEphemeralFixedWorkspace || group.type !== 'export',
  );

  const toolbarContent = (
    <>
      <input 
         type="file" 
         ref={fileInputRef} 
         className="hidden" 
         accept="image/*"
         multiple
         onChange={(e) => {
           handleImageFiles(e.target.files, editor);
           if (e.target) e.target.value = '';
         }}
      />
      <input 
         type="file" 
         ref={videoInputRef} 
         className="hidden" 
         accept="video/*"
         multiple
         onChange={(e) => {
           if (e.target.files && e.target.files.length > 0) {
             void insertEditorMedia(e.target.files[0], 'video', editor);
           }
           if (e.target) e.target.value = '';
         }}
      />
      <input 
         type="file" 
         ref={audioInputRef} 
         className="hidden" 
         accept="audio/*"
         multiple
         onChange={(e) => {
           if (e.target.files && e.target.files.length > 0) {
             void insertEditorMedia(e.target.files[0], 'audio', editor);
           }
           if (e.target) e.target.value = '';
         }}
      />
      <div className="flex-1 min-w-0 w-full">
        <UniversalToolbar
          editor={editor}
          t={t}
          config={workspaceToolbarConfig}
          handleToggleSourceMode={handleToggleSourceMode}
          isSourceMode={isSourceMode}
          isSplitMode={isSplitMode}
          setIsSplitMode={(val) => {
             setIsSplitMode(val);
             if (val) {
                setIsSourceMode(false);
                const data = mode === 'markdown' ? (editor.storage as any).markdown.getMarkdown() : editor.getHTML();
                setSourceCode(data);
             }
          }}
          getExportContent={isEphemeralFixedWorkspace ? undefined : getExportContent}
        />
      </div>
    </>
  );

  return (
    <div className="w-full flex-1 flex flex-col min-h-0 relative">
      {portalNode ? (
        createPortal(toolbarContent, portalNode)
      ) : (
        <div className="w-full border-b border-zinc-200/80 dark:border-[var(--color-kb-panel-border)]/80 bg-white dark:bg-[var(--color-kb-panel)] select-none h-[40px] flex items-center px-4 shrink-0 overflow-visible shadow-sm z-30">
          {toolbarContent}
        </div>
      )}
      {isWechatAppletModalOpen && (
        <WechatAppletModal 
          onClose={() => setIsWechatAppletModalOpen(false)}
          onConfirm={(data) => {
            editor.commands.insertContent({
              type: 'wechatMiniprogram',
              attrs: {
                'data-miniprogram-type': data.displayType,
                'data-miniprogram-title': data.displayType === 'card' ? data.cardTitle : data.textContent,
                'data-miniprogram-imageurl': data.imageUrl,
                'data-miniprogram-path': data.link,
                'data-miniprogram-nickname': '小程序'
              }
            });
            if (data.displayType === 'card' || data.displayType === 'image') {
              editor.commands.insertContent('<p></p>');
            }
          }}
        />
      )}
      {editor && aiEnabled && (
        <EditorBubbleMenu 
          editor={editor}
          t={t}
          aiLoading={aiLoading}
          handleAiAction={handleAiAction}
        />
      )}
      <div className="relative flex-1 flex flex-col min-h-0 overflow-y-auto w-full p-0 m-0 no-scrollbar">
        {!hideTitle && (
          <div className="w-full flex flex-col select-text shrink-0 px-6 pt-6 pb-2 max-w-4xl mx-auto">
            <input
              type="text"
              placeholder={t('untitledNote')}
              value={title}
              onChange={(e) => handleTitleChangeLocal(e.target.value)}
              className="w-full bg-transparent border-none text-3xl font-extrabold tracking-tight text-[var(--color-kb-text-heading)] outline-none focus:outline-none focus:ring-0 p-0 placeholder-zinc-300 dark:placeholder-zinc-700"
            />
            <div className="h-px bg-[var(--color-kb-panel-border)]/55 mt-4"></div>
          </div>
        )}

        {aiLoading && (
          <div className="absolute top-4 right-4 flex items-center bg-[var(--color-kb-panel)] text-[var(--color-kb-accent)] text-xs font-medium px-3 py-1.5 rounded-full shadow-sm animate-pulse z-10 border border-[var(--color-kb-panel-border)]">
            <Sparkles size={12} className="mr-1.5" /> {t('aiProcessing')}
          </div>
        )}
        {isSourceMode && !isSplitMode && (
          <textarea
            value={sourceCode}
            onChange={(e) => setSourceCode(e.target.value)}
            className="w-full max-w-4xl mx-auto px-6 py-6 flex-1 bg-transparent text-[var(--color-kb-text)] font-mono text-sm border-none focus:outline-none resize-none"
            spellCheck={false}
          />
        )}
        {isSplitMode && (
          <div className="flex w-full gap-0 border-t border-[var(--color-kb-panel-border)] flex-1 h-full min-h-0">
            <div className="w-1/2 h-full flex flex-col border-r border-[var(--color-kb-panel-border)] overflow-hidden bg-[var(--color-kb-panel-hover)]">
               <textarea
                 value={sourceCode}
                 onChange={(e) => {
                   setSourceCode(e.target.value);
                   editor.commands.setContent(e.target.value);
                   onChange?.(e.target.value);
                 }}
                 className="w-full h-full p-6 bg-transparent text-[var(--color-kb-text)] font-mono text-sm border-none focus:outline-none resize-none"
                 spellCheck={false}
               />
            </div>
            <div className="w-1/2 h-full overflow-y-auto bg-[var(--color-kb-editor)] p-6">
               <EditorContent editor={editor} className="h-full flex flex-col flex-1 min-h-0 w-full" />
            </div>
          </div>
        )}
        {!isSourceMode && !isSplitMode && (
          <EditorContent editor={editor} className="w-full max-w-4xl mx-auto px-6 pb-12" />
        )}
      </div>
    </div>
  );
}
