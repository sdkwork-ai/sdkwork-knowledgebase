import { ChevronRight } from 'lucide-react';
import { useAiModelSelection } from '@sdkwork/sdkwork-knowledgebase-pc-commons';

interface SearchChatHeaderProps {
  title: string;
  sidebarOpen: boolean;
  onToggleSidebar: () => void;
}

export function SearchChatHeader({ title, sidebarOpen, onToggleSidebar }: SearchChatHeaderProps) {
  const { activeModel } = useAiModelSelection();

  return (
    <div className="h-14 border-b border-[var(--color-kb-panel-border)] px-5 flex items-center bg-[var(--color-kb-editor)]/90 backdrop-blur-md shrink-0 z-10 select-none">
      <div className="flex items-center gap-3 min-w-0">
        <button
          type="button"
          onClick={onToggleSidebar}
          className="p-2 hover:bg-[var(--color-kb-panel-hover)] rounded-lg text-[var(--color-kb-text-muted)] hover:text-[var(--color-kb-text-heading)] transition-all active:scale-95 border border-[var(--color-kb-panel-border)]/60"
          title={sidebarOpen ? '隐藏边栏' : '显示边栏'}
        >
          <ChevronRight
            className={`w-4 h-4 transform transition-transform duration-200 ${sidebarOpen ? 'rotate-180' : ''}`}
          />
        </button>
        <div className="flex flex-col min-w-0">
          <h1 className="text-[15px] font-bold text-[var(--color-kb-text-heading)] truncate max-w-[280px] md:max-w-[520px]">
            {title}
          </h1>
          <div className="flex items-center gap-1.5 mt-0.5">
            <span className="w-1.5 h-1.5 bg-emerald-500 rounded-full animate-pulse" />
            <span className="text-[10px] text-[var(--color-kb-text-muted)] font-bold font-mono tracking-wider truncate">
              对话检索 · {activeModel?.name ?? 'AI 模型'}
            </span>
          </div>
        </div>
      </div>
    </div>
  );
}
