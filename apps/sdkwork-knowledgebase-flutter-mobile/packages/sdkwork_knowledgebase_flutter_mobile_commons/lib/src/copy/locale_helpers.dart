/// Thin locale helpers for the Knowledgebase Flutter mobile capability family.
///
/// Placement note (`I18N_SPEC.md` section 6.1): the authored Flutter fragment
/// layout is
/// `lib/src/i18n/<locale>/<domain>/<capability>/<screen-or-widget>.arb` or
/// `.json` — `.dart` is not an authored fragment extension. Dart code that
/// normalizes a locale or looks a key up in an already-loaded fragment is a
/// boundary helper, not a locale resource, so it lives outside
/// `lib/src/i18n/`.
enum SdkworkKnowledgebaseLocale { enUs, zhCn }

SdkworkKnowledgebaseLocale normalizeSdkworkKnowledgebaseLocale(String value) {
  final normalized = value.trim().toLowerCase();
  return normalized.startsWith('zh')
      ? SdkworkKnowledgebaseLocale.zhCn
      : SdkworkKnowledgebaseLocale.enUs;
}

String localeDirectoryName(SdkworkKnowledgebaseLocale locale) {
  switch (locale) {
    case SdkworkKnowledgebaseLocale.enUs:
      return 'en-US';
    case SdkworkKnowledgebaseLocale.zhCn:
      return 'zh-CN';
  }
}

String pickSdkworkKnowledgebaseMessage(
  Map<String, String> messages,
  String key,
  String fallback,
) {
  final value = messages[key];
  return value != null && value.isNotEmpty ? value : fallback;
}
