/// Package-local default copy for the knowledge capability.
///
/// Placement note (`I18N_SPEC.md` section 6.1): Flutter authored fragments use
/// `.arb`/`.json` under `lib/src/i18n/<locale>/<domain>/<capability>/`. Dart
/// `const` maps are not an authored fragment format, so they are classified as
/// code-level defaults and live outside `lib/src/i18n/`. The authored
/// fragments under `lib/src/i18n/` are the projection source once the
/// `gen_l10n` step is wired into the Flutter build.
const Map<String, String> knowledgebaseMessagesEnUs = <String, String>{
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

const Map<String, String> knowledgebaseMessagesZhCn = <String, String>{
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
