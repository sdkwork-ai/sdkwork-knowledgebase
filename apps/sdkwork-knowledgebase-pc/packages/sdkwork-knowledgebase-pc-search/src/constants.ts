import { KNOWLEDGEBASE_ACCOUNT_SCOPED_STORAGE_KEYS } from 'sdkwork-knowledgebase-pc-core';

/**
 * Base key of the persisted AI search history.
 *
 * The value is written under an account-scoped key derived from this one; the
 * base key alone is only the legacy layout that older builds wrote and that
 * logout still removes.
 */
export const SEARCH_SESSIONS_STORAGE_KEY = KNOWLEDGEBASE_ACCOUNT_SCOPED_STORAGE_KEYS.searchSessions;

/** Center column width — shared by thread, composer, and landing prompts */
export const CHAT_LAYOUT_MAX = 'max-w-4xl';

export const PRESET_PROMPTS = [
  { text: '分析公司业务执行方案与营销预算', icon: '📊', type: 'doc' as const },
  { text: '汇总目前最新的 AI 智能桌面的设计灵感', icon: '💡', type: 'web' as const },
  { text: '排查我知识库中的 Q3 战略大纲与文件', icon: '🔍', type: 'doc' as const },
  { text: 'AI 与 AGI 时代最新的网络开发框架有哪些？', icon: '🌐', type: 'web' as const }
];

export const COMPOSER_MAX_HEIGHT = {
  chat: 100,
  hero: 280
} as const;
