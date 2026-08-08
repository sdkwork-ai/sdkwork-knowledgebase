import { useState } from 'react';
import { isBlank } from '@sdkwork/utils';
import { BubbleMenu } from '@tiptap/react/menus';
import { Sparkles, PenTool, Type, FileType2, Languages, Bold, Italic, Strikethrough, CornerDownLeft, Loader2, Minimize2, Maximize2, CheckCheck, Lightbulb, HelpCircle } from 'lucide-react';
import { Editor } from '@tiptap/core';

export interface EditorBubbleMenuProps {
  editor: Editor;
  t: any;
  aiLoading: boolean;
  handleAiAction: (action: string, customPrompt?: string) => Promise<void>;
}

export function EditorBubbleMenu({
  editor,
  t,
  aiLoading,
  handleAiAction
}: EditorBubbleMenuProps) {
  const [prompt, setPrompt] = useState("");

  const submitCustomPrompt = () => {
    if (isBlank(prompt)) return;
    handleAiAction('custom', prompt.trim());
    setPrompt('');
  };

  if (!editor) return null;

  const BubbleMenuComponent = BubbleMenu as any;

  return (
    <BubbleMenuComponent 
      editor={editor} 
      tippyOptions={{
        duration: 100,
        placement: 'top',
        appendTo: () => document.body,
        zIndex: 50,
      }}
      className="flex flex-col bg-[var(--color-kb-editor)] border border-[var(--color-kb-panel-border)] shadow-[0_8px_30px_rgb(0,0,0,0.12)] rounded-xl overflow-hidden p-1 min-w-[320px]"
    >
      <div className="flex items-center px-3 py-2 border-b border-[var(--color-kb-panel-border)] group">
        <Sparkles size={16} className="text-[var(--color-kb-accent)] mr-2 flex-shrink-0" />
        <input 
           type="text" 
           value={prompt}
           onChange={(e) => setPrompt(e.target.value)}
           onKeyDown={(e) => {
             if (e.key === 'Enter') {
               e.preventDefault();
               submitCustomPrompt();
             }
           }}
           placeholder={t('aiEditPlaceholder', { ns: 'editor' })} 
           className="flex-1 bg-transparent text-sm outline-none text-[var(--color-kb-text)] placeholder:text-[var(--color-kb-text-muted)]"
           disabled={aiLoading}
        />
        {aiLoading ? (
           <Loader2 size={14} className="text-[var(--color-kb-text-muted)] animate-spin" />
        ) : (
           <button 
             onClick={submitCustomPrompt}
             disabled={isBlank(prompt)}
             className="text-[var(--color-kb-text-muted)] hover:text-[var(--color-kb-accent)] transition-colors opacity-0 group-hover:opacity-100 disabled:opacity-0 focus:opacity-100"
           >
             <CornerDownLeft size={14} />
           </button>
        )}
      </div>
      
      {!prompt && (
        <div className="flex flex-col p-1 max-h-[300px] overflow-y-auto no-scrollbar">
          <div className="text-[10px] font-semibold text-[var(--color-kb-text-muted)] px-2 py-1 uppercase tracking-wider">
            AI Operations
          </div>
          <div className="grid grid-cols-2 gap-1 px-1">
            <button 
              onClick={() => handleAiAction('polish')}
              disabled={aiLoading}
              className="flex items-center px-2 py-1.5 hover:bg-[var(--color-kb-panel-hover)] rounded-md text-xs text-[var(--color-kb-text)] transition-colors group disabled:opacity-50"
            >
              <PenTool size={13} className="mr-2 text-[var(--color-kb-text-muted)] group-hover:text-[var(--color-kb-text-heading)] transition-colors" /> 
              {t('polish') || '润色'}
            </button>
            <button 
              onClick={() => handleAiAction('continue')}
              disabled={aiLoading}
              className="flex items-center px-2 py-1.5 hover:bg-[var(--color-kb-panel-hover)] rounded-md text-xs text-[var(--color-kb-text)] transition-colors group disabled:opacity-50"
            >
              <Type size={13} className="mr-2 text-[var(--color-kb-text-muted)] group-hover:text-[var(--color-kb-text-heading)] transition-colors" /> 
              {t('continue') || '续写'}
            </button>
            <button 
              onClick={() => handleAiAction('summarize')}
              disabled={aiLoading}
              className="flex items-center px-2 py-1.5 hover:bg-[var(--color-kb-panel-hover)] rounded-md text-xs text-[var(--color-kb-text)] transition-colors group disabled:opacity-50"
            >
              <FileType2 size={13} className="mr-2 text-[var(--color-kb-text-muted)] group-hover:text-[var(--color-kb-text-heading)] transition-colors" /> 
              {t('summarize') || '总结'}
            </button>
            <button 
              onClick={() => handleAiAction('translate')}
              disabled={aiLoading}
              className="flex items-center px-2 py-1.5 hover:bg-[var(--color-kb-panel-hover)] rounded-md text-xs text-[var(--color-kb-text)] transition-colors group disabled:opacity-50"
            >
              <Languages size={13} className="mr-2 text-[var(--color-kb-text-muted)] group-hover:text-[var(--color-kb-text-heading)] transition-colors" /> 
              {t('translate') || '翻译'}
            </button>
            <button 
              onClick={() => handleAiAction('explain')}
              disabled={aiLoading}
              className="flex items-center px-2 py-1.5 hover:bg-[var(--color-kb-panel-hover)] rounded-md text-xs text-[var(--color-kb-text)] transition-colors group disabled:opacity-50"
            >
              <HelpCircle size={13} className="mr-2 text-[var(--color-kb-text-muted)] group-hover:text-[var(--color-kb-text-heading)] transition-colors" /> 
              {t('explainConcept', { ns: 'editor' })}
            </button>
            <button 
              onClick={() => handleAiAction('fix_grammar')}
              disabled={aiLoading}
              className="flex items-center px-2 py-1.5 hover:bg-[var(--color-kb-panel-hover)] rounded-md text-xs text-[var(--color-kb-text)] transition-colors group disabled:opacity-50"
            >
              <CheckCheck size={13} className="mr-2 text-[var(--color-kb-text-muted)] group-hover:text-[var(--color-kb-text-heading)] transition-colors" /> 
              {t('fixGrammar', { ns: 'editor' })}
            </button>
            <button 
              onClick={() => handleAiAction('shorten')}
              disabled={aiLoading}
              className="flex items-center px-2 py-1.5 hover:bg-[var(--color-kb-panel-hover)] rounded-md text-xs text-[var(--color-kb-text)] transition-colors group disabled:opacity-50"
            >
              <Minimize2 size={13} className="mr-2 text-[var(--color-kb-text-muted)] group-hover:text-[var(--color-kb-text-heading)] transition-colors" /> 
              {t('shorten', { ns: 'editor' })}
            </button>
            <button 
              onClick={() => handleAiAction('expand')}
              disabled={aiLoading}
              className="flex items-center px-2 py-1.5 hover:bg-[var(--color-kb-panel-hover)] rounded-md text-xs text-[var(--color-kb-text)] transition-colors group disabled:opacity-50"
            >
              <Maximize2 size={13} className="mr-2 text-[var(--color-kb-text-muted)] group-hover:text-[var(--color-kb-text-heading)] transition-colors" /> 
              {t('expand', { ns: 'editor' })}
            </button>
            <button 
              onClick={() => handleAiAction('brainstorm')}
              disabled={aiLoading}
              className="flex items-center px-2 py-1.5 hover:bg-[var(--color-kb-panel-hover)] rounded-md text-xs text-[var(--color-kb-text)] transition-colors group disabled:opacity-50"
            >
              <Lightbulb size={13} className="mr-2 text-[var(--color-kb-text-muted)] group-hover:text-[var(--color-kb-text-heading)] transition-colors" /> 
              {t('brainstorm', { ns: 'editor' })}
            </button>
          </div>

          <div className="h-px bg-[var(--color-kb-panel-border)] my-1 mx-2"></div>
          
          <div className="flex items-center px-1">
            <button onClick={() => editor.chain().focus().toggleBold().run()} className={`p-1.5 rounded-md transition-colors ${editor.isActive('bold') ? 'bg-[var(--color-kb-panel-active)] text-[var(--color-kb-text-heading)]' : 'hover:bg-[var(--color-kb-panel-hover)] text-[var(--color-kb-text)]'}`}><Bold size={15} /></button>
            <button onClick={() => editor.chain().focus().toggleItalic().run()} className={`p-1.5 rounded-md transition-colors ${editor.isActive('italic') ? 'bg-[var(--color-kb-panel-active)] text-[var(--color-kb-text-heading)]' : 'hover:bg-[var(--color-kb-panel-hover)] text-[var(--color-kb-text)]'}`}><Italic size={15} /></button>
            <button onClick={() => editor.chain().focus().toggleStrike().run()} className={`p-1.5 rounded-md transition-colors ${editor.isActive('strike') ? 'bg-[var(--color-kb-panel-active)] text-[var(--color-kb-text-heading)]' : 'hover:bg-[var(--color-kb-panel-hover)] text-[var(--color-kb-text)]'}`}><Strikethrough size={15} /></button>
          </div>
        </div>
      )}
    </BubbleMenuComponent>
  );
}
