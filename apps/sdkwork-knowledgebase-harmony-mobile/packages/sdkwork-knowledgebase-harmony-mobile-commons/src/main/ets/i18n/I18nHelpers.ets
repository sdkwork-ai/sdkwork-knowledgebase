/**
 * Thin locale helpers for the Knowledgebase HarmonyOS capability family.
 *
 * Placement note (`I18N_SPEC.md` section 6.1): authored locale resources for
 * this architecture live in `entry/src/main/resources/base/element/string.json`
 * and per-locale resource directories. ArkTS code that normalizes a locale or
 * looks a key up in an already-loaded resource is a boundary helper, not a
 * locale resource, so it lives outside any locale resource directory.
 */
export type SdkworkKnowledgebaseLocale = 'en-US' | 'zh-CN';

export function normalizeSdkworkKnowledgebaseLocale(value: string): SdkworkKnowledgebaseLocale {
  const normalized: string = value.trim().toLowerCase();
  return normalized.startsWith('zh') ? 'zh-CN' : 'en-US';
}

export function pickSdkworkKnowledgebaseMessage(
  messages: Record<string, string>,
  key: string,
  fallback: string,
): string {
  const value: string | undefined = messages[key];
  return value !== undefined && value.length > 0 ? value : fallback;
}
