/**
 * Package-local default copy for the knowledge capability.
 *
 * Authored locale resources for this architecture live in
 * `entry/src/main/resources/base/element/string.json` and per-locale resource
 * directories; ArkTS `const` maps are code-level defaults, not authored
 * locale resources, so they live outside any locale resource directory.
 */
export const knowledgebaseMessagesEnUs: Record<string, string> = {
  'knowledgebase.list.title': 'Knowledgebase',
  'knowledgebase.list.loading': 'Loading...',
  'knowledgebase.list.empty': 'No knowledgebase yet',
  'knowledgebase.list.loadFailed': 'Failed to load knowledgebase',
  'knowledgebase.detail.title': 'Document',
  'knowledgebase.search.title': 'Search',
  'knowledgebase.search.empty': 'No results',
  'knowledgebase.settings.title': 'Settings',
  'knowledgebase.launch.title': 'Group knowledgebase',
  'knowledgebase.launch.invalidTicket': 'This link is invalid or has expired',
};

export const knowledgebaseMessagesZhCn: Record<string, string> = {
  'knowledgebase.list.title': '知识库',
  'knowledgebase.list.loading': '加载中...',
  'knowledgebase.list.empty': '暂无知识库',
  'knowledgebase.list.loadFailed': '知识库加载失败',
  'knowledgebase.detail.title': '文档',
  'knowledgebase.search.title': '搜索',
  'knowledgebase.search.empty': '暂无结果',
  'knowledgebase.settings.title': '设置',
  'knowledgebase.launch.title': '群知识库',
  'knowledgebase.launch.invalidTicket': '该链接无效或已过期',
};
