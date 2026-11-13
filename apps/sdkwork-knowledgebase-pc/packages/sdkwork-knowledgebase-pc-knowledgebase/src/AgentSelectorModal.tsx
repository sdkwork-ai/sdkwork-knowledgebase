import { useCallback, useEffect, useMemo, useState } from 'react';
import {
  BookOpen,
  Bot,
  CircleAlert,
  FileText,
  Library,
  Loader2,
  Megaphone,
  Newspaper,
  PenTool,
  Search,
  SpellCheck,
  X,
} from 'lucide-react';
import { useTranslation } from 'react-i18next';

import {
  listWritingAgents,
  type WritingAgent,
} from './services/writingAgentService';
import { resolveUserFacingErrorMessage, type ErrorTranslateFn } from 'sdkwork-knowledgebase-pc-core';

export interface AgentSelectorModalProps {
  onClose: () => void;
  onSelect: (agent: WritingAgent) => void;
}

const AGENT_ICONS = {
  BookOpen,
  Library,
  Newspaper,
  FileText,
  Megaphone,
  SpellCheck,
} as const;

function AgentIcon({ agent }: { agent: WritingAgent }) {
  const Icon = agent.iconName && agent.iconName in AGENT_ICONS
    ? AGENT_ICONS[agent.iconName as keyof typeof AGENT_ICONS]
    : PenTool;
  return (
    <span
      className="flex h-11 w-11 shrink-0 items-center justify-center rounded-xl border border-[var(--color-kb-panel-border)] bg-[var(--color-kb-panel)]"
      style={agent.color ? { color: agent.color } : undefined}
    >
      <Icon size={20} strokeWidth={1.8} />
    </span>
  );
}

export function AgentSelectorModal({ onClose, onSelect }: AgentSelectorModalProps) {
  const { t } = useTranslation(['editor', 'mcp', 'errors']);
  const [agents, setAgents] = useState<WritingAgent[]>([]);
  const [loading, setLoading] = useState(true);
  const [errorMessage, setErrorMessage] = useState<string | null>(null);
  const [searchQuery, setSearchQuery] = useState('');

  const loadAgents = useCallback(async () => {
    setLoading(true);
    setErrorMessage(null);
    try {
      setAgents(await listWritingAgents());
    } catch (error) {
      setErrorMessage(resolveUserFacingErrorMessage(error, t as unknown as ErrorTranslateFn));
    } finally {
      setLoading(false);
    }
  }, [t]);

  useEffect(() => {
    void loadAgents();
  }, [loadAgents]);

  const filteredAgents = useMemo(() => {
    const query = searchQuery.trim().toLowerCase();
    if (!query) {
      return agents;
    }
    return agents.filter((agent) =>
      agent.name.toLowerCase().includes(query)
      || agent.description.toLowerCase().includes(query)
      || agent.tags.some((tag) => tag.toLowerCase().includes(query)),
    );
  }, [agents, searchQuery]);

  return (
    <div
      className="fixed inset-0 z-[400] flex items-center justify-center bg-zinc-950/40 backdrop-blur-md"
      onClick={onClose}
    >
      <div
        className="flex h-[560px] w-[760px] max-w-[calc(100vw-48px)] flex-col overflow-hidden rounded-2xl border border-[var(--color-kb-panel-border)] bg-[var(--color-kb-panel)] shadow-2xl animate-in zoom-in-95 duration-200"
        onClick={(event) => event.stopPropagation()}
      >
        <div className="flex items-center justify-between border-b border-[var(--color-kb-panel-border)] px-5 py-3.5">
          <div>
            <h3 className="text-[15px] font-semibold text-[var(--color-kb-text-heading)]">
              {t('writingAgentPickerTitle', { defaultValue: '写作智能体' })}
            </h3>
            <p className="mt-0.5 text-[11px] text-[var(--color-kb-text-muted)]">
              {t('writingAgentPickerSubtitle', {
                defaultValue: '来自 SDKWork 智能体平台，按写作场景选择专属智能体',
              })}
            </p>
          </div>
          <button
            type="button"
            onClick={onClose}
            aria-label={t('close', { ns: 'common' })}
            className="rounded-full p-1.5 text-[var(--color-kb-text-muted)] transition-colors hover:bg-[var(--color-kb-panel-hover)]"
          >
            <X size={18} />
          </button>
        </div>

        <div className="flex items-center gap-2 border-b border-[var(--color-kb-panel-border)] px-5 py-3">
          <div className="relative w-72 max-w-full">
            <Search
              size={15}
              className="absolute left-3 top-1/2 -translate-y-1/2 text-[var(--color-kb-text-muted)]"
            />
            <input
              type="text"
              value={searchQuery}
              onChange={(event) => setSearchQuery(event.target.value)}
              placeholder={t('writingAgentSearchPlaceholder', { defaultValue: '搜索智能体...' })}
              className="w-full rounded-full bg-[var(--color-kb-input-bg)] py-1.5 pl-9 pr-4 text-sm text-[var(--color-kb-text)] focus:outline-none focus:ring-1 focus:ring-[var(--color-kb-accent)]"
            />
          </div>
          {!loading && !errorMessage && (
            <span className="text-[11px] text-[var(--color-kb-text-muted)]">
              {t('writingAgentCount', {
                defaultValue: '共 {{total}} 个写作智能体',
                total: agents.length,
              })}
            </span>
          )}
        </div>

        <div className="flex-1 overflow-y-auto p-5">
          {loading && (
            <div className="flex h-full flex-col items-center justify-center gap-3 text-[var(--color-kb-text-muted)]">
              <Loader2 size={22} className="animate-spin text-[var(--color-kb-accent)]" />
              <span className="text-sm">{t('writingAgentLoading', { defaultValue: '正在加载写作智能体...' })}</span>
            </div>
          )}

          {!loading && errorMessage && (
            <div className="flex h-full flex-col items-center justify-center gap-3 text-center">
              <CircleAlert size={26} className="text-amber-500" />
              <p className="max-w-sm text-sm leading-6 text-[var(--color-kb-text-muted)]">{errorMessage}</p>
              <button
                type="button"
                onClick={() => void loadAgents()}
                className="rounded-lg border border-[var(--color-kb-panel-border)] px-3.5 py-1.5 text-sm text-[var(--color-kb-text)] transition-colors hover:bg-[var(--color-kb-panel-hover)]"
              >
                {t('retry', { ns: 'common', defaultValue: '重试' })}
              </button>
            </div>
          )}

          {!loading && !errorMessage && (
            <div className="grid grid-cols-2 gap-3.5">
              {filteredAgents.map((agent) => (
                <button
                  key={agent.agentId}
                  type="button"
                  onClick={() => onSelect(agent)}
                  className="group flex items-start gap-3 rounded-xl border border-[var(--color-kb-panel-border)] bg-[var(--color-kb-editor)] p-4 text-left transition-all hover:border-[var(--color-kb-accent)] hover:shadow-sm"
                >
                  <AgentIcon agent={agent} />
                  <span className="min-w-0">
                    <span className="mb-1 block truncate font-medium text-[var(--color-kb-text-heading)] group-hover:text-[var(--color-kb-accent)]">
                      {agent.name}
                    </span>
                    <span className="line-clamp-2 block text-[12.5px] leading-relaxed text-[var(--color-kb-text-muted)]">
                      {agent.description}
                    </span>
                  </span>
                </button>
              ))}
              {filteredAgents.length === 0 && (
                <div className="col-span-2 flex flex-col items-center justify-center py-14 text-[var(--color-kb-text-muted)]">
                  <Bot size={30} className="mb-3 opacity-50" />
                  <p className="text-sm">{t('writingAgentEmpty', { defaultValue: '没有找到相关写作智能体' })}</p>
                </div>
              )}
            </div>
          )}
        </div>
      </div>
    </div>
  );
}
