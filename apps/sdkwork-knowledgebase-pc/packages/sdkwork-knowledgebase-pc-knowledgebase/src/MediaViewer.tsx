import { useState, useEffect } from 'react';
import { Sparkles } from 'lucide-react';
import { DocumentMeta, KnowledgeBase } from './services/document';
import { ImageViewer } from './components/players/ImageViewer';
import { VideoPlayer } from './components/players/VideoPlayer';
import { FileDownloader } from './components/players/FileDownloader';
import { MusicPlayer } from './components/players/MusicPlayer';
import { useHydratedViewerDocument } from './hooks/useHydratedViewerDocument';
import {
  isKnowledgebaseWorkspaceAiEnabled,
  type KnowledgebaseWorkspaceMode,
} from './workspaceMode';

export interface MediaViewerProps {
  activeDoc: DocumentMeta;
  docContent?: string;
  onContentChange?: (content: string) => void;
  isTranscribing?: boolean;
  onTranscribeStart?: () => void;
  onTranscribeComplete?: (content: string) => void;
  onTitleChange?: (title: string) => void;
  activeKb?: KnowledgeBase | null;
  onUpdateDocs?: () => void;
  workspaceMode?: KnowledgebaseWorkspaceMode;
}

export function MediaViewer({ 
  activeDoc, 
  isTranscribing,
  onTranscribeStart,
  onTranscribeComplete,
  activeKb,
  onUpdateDocs,
  workspaceMode = 'standard',
}: MediaViewerProps) {
  const [toastMsg, setToastMsg] = useState<string | null>(null);
  const viewDoc = useHydratedViewerDocument(activeDoc);
  const aiEnabled = isKnowledgebaseWorkspaceAiEnabled(workspaceMode);

  // Clear toast after timeout
  useEffect(() => {
    if (toastMsg) {
      const timer = setTimeout(() => setToastMsg(null), 3000);
      return () => clearTimeout(timer);
    }
  }, [toastMsg]);

  if (!viewDoc) {
    return null;
  }

  return (
    <div id="media-transcription-workspace" className="w-full flex-1 p-0 flex flex-col bg-[var(--color-kb-editor)] min-h-0 select-none">
      
      {/* Toast Alert Widget */}
      {toastMsg && (
        <div id="audio-toast" className="fixed top-24 left-1/2 transform -translate-x-1/2 z-[9999] bg-white dark:bg-zinc-900 border border-zinc-200 dark:border-zinc-700/80 px-4 py-2.5 rounded-xl shadow-2xl flex items-center gap-2 text-xs font-semibold text-rose-500 dark:text-rose-300 animate-bounce">
          <Sparkles size={14} className="text-rose-500 dark:text-rose-400 shrink-0" />
          <span className="text-zinc-800 dark:text-rose-300">{toastMsg}</span>
        </div>
      )}

      {/* Media Types Router */}
      <div className="flex-1 w-full bg-[var(--color-kb-editor)] overflow-hidden flex flex-col min-h-0">
        
        {/* Render Image Source */}
        {viewDoc.type === 'image' && (
          <ImageViewer activeDoc={viewDoc} activeKb={activeKb} onUpdateDocs={onUpdateDocs} onToastMessage={setToastMsg} />
        )}

        {/* Render Video Player */}
        {viewDoc.type === 'video' && (
          <VideoPlayer activeDoc={viewDoc} activeKb={activeKb} onUpdateDocs={onUpdateDocs} onToastMessage={setToastMsg} />
        )}

        {/* Render Standard Generic File */}
        {viewDoc.type === 'file' && (
          <FileDownloader activeDoc={viewDoc} />
        )}

        {/* Generic Audio/Music Player Console */}
        {(viewDoc.type === 'audio' || viewDoc.type === 'music') && (
          <MusicPlayer 
            activeDoc={viewDoc} 
            onToastMessage={setToastMsg}
            isTranscribing={isTranscribing}
            onTranscribeStart={onTranscribeStart}
            onTranscribeComplete={onTranscribeComplete}
            aiEnabled={aiEnabled}
          />
        )}

      </div>
    </div>
  );
}
