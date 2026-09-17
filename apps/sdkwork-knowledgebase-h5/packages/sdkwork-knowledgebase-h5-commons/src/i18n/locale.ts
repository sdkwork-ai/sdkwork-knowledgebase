/** Thin locale helpers; authored message copy lives in capability packages. */

export const SDKWORK_SUPPORTED_LOCALES = ['zh-CN', 'en-US'] as const;
export type SdkworkSupportedLocale = (typeof SDKWORK_SUPPORTED_LOCALES)[number];

export const SDKWORK_DEFAULT_LOCALE: SdkworkSupportedLocale = 'zh-CN';

export function normalizeSdkworkLocale(value: string | undefined): SdkworkSupportedLocale {
  if (!value) {
    return SDKWORK_DEFAULT_LOCALE;
  }
  const normalized = value.trim().toLowerCase();
  if (normalized.startsWith('zh')) {
    return 'zh-CN';
  }
  if (normalized.startsWith('en')) {
    return 'en-US';
  }
  return SDKWORK_DEFAULT_LOCALE;
}

/** Resolve the initial locale from an explicit value, then the host language. */
export function resolveInitialSdkworkLocale(explicit?: string): SdkworkSupportedLocale {
  if (explicit) {
    return normalizeSdkworkLocale(explicit);
  }
  const hostLanguage = typeof navigator === 'undefined' ? undefined : navigator.language;
  return normalizeSdkworkLocale(hostLanguage);
}
