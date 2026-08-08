import { CHAT_LAYOUT_MAX } from '../constants';
import type {
  SearchMessage,
  SearchNavigateToFilePayload,
  SearchNavigateToKbPayload
} from '../types';
import { SearchMessageItem } from './SearchMessageItem';

export interface SearchChatThreadProps {
  messages: SearchMessage[];
  isTyping: boolean;
  expandedStepIds: Record<string, boolean>;
  onToggleSteps: (messageId: string) => void;
  onRegenerate: (query: string) => void;
  onExport: (text: string) => void;
  onCopy: (text: string) => void;
  onFollowUp: (text: string) => void;
  onGoToKb: (payload: SearchNavigateToKbPayload) => void;
  onGoToFile: (payload: SearchNavigateToFilePayload) => void;
  onOpenWebLink?: (url: string, title?: string) => void;
}

export function SearchChatThread({
  messages,
  isTyping,
  expandedStepIds,
  onToggleSteps,
  onRegenerate,
  onExport,
  onCopy,
  onFollowUp,
  onGoToKb,
  onGoToFile,
  onOpenWebLink
}: SearchChatThreadProps) {
  return (
    <div className={`${CHAT_LAYOUT_MAX} mx-auto w-full min-w-0 px-4 md:px-6 py-6`}>
      <div className="flex flex-col gap-6 animate-in fade-in duration-300">
        {messages.map((msg, msgIndex) => {
          const prevUserMsg = [...messages.slice(0, msgIndex)].reverse().find((m) => m.role === 'user');
          const isStreaming =
            isTyping &&
            msg.role === 'assistant' &&
            msgIndex === messages.length - 1 &&
            msg.content.length > 0;
          const stepsExpanded = expandedStepIds[msg.id] ?? msg.isSearching === true;

          return (
            <div key={msg.id}>
              <SearchMessageItem
                message={msg}
                prevUserQuery={prevUserMsg?.content}
                isStreaming={isStreaming}
                isTyping={isTyping}
                stepsExpanded={stepsExpanded}
                onToggleSteps={() => onToggleSteps(msg.id)}
                onRegenerate={onRegenerate}
                onExport={onExport}
                onCopy={onCopy}
                onFollowUp={onFollowUp}
                onGoToKb={onGoToKb}
                onGoToFile={onGoToFile}
                onOpenWebLink={onOpenWebLink}
              />
            </div>
          );
        })}
      </div>
    </div>
  );
}
