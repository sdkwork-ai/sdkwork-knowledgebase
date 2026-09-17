/// Locale fragment projection entry for the knowledge capability.
///
/// Authority: `I18N_SPEC.md` section 6.1. Authored fragments live under
/// `lib/src/i18n/<locale>/<domain>/<capability>/<fragment>.arb`. The
/// `gen_l10n` projection that turns them into Dart accessors is tracked as
/// pending integration because it requires the Flutter toolchain.
library;

/// Authored fragment directories, relative to this package's `lib/`.
const List<String> knowledgebaseLocaleFragments = <String>[
  'src/i18n/en-US/intelligence/knowledge/list.arb',
  'src/i18n/en-US/intelligence/knowledge/detail.arb',
  'src/i18n/en-US/intelligence/knowledge/search.arb',
  'src/i18n/en-US/intelligence/knowledge/settings.arb',
  'src/i18n/en-US/intelligence/knowledge/launch.arb',
  'src/i18n/zh-CN/intelligence/knowledge/list.arb',
  'src/i18n/zh-CN/intelligence/knowledge/detail.arb',
  'src/i18n/zh-CN/intelligence/knowledge/search.arb',
  'src/i18n/zh-CN/intelligence/knowledge/settings.arb',
  'src/i18n/zh-CN/intelligence/knowledge/launch.arb',
];

/// Supported locale directories, aligned with the other client roots.
const List<String> knowledgebaseSupportedLocales = <String>['en-US', 'zh-CN'];
