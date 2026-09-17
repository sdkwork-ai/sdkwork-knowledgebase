/** Thin locale helpers; authored message copy lives in capability packages. */

export const SDKWORK_MP_SUPPORTED_LOCALES = ["zh-CN", "en-US"] as const;
export type SdkworkMpSupportedLocale = (typeof SDKWORK_MP_SUPPORTED_LOCALES)[number];

export const SDKWORK_MP_DEFAULT_LOCALE: SdkworkMpSupportedLocale = "zh-CN";

export function normalizeSdkworkMpLocale(value: string | undefined): SdkworkMpSupportedLocale {
  if (!value) {
    return SDKWORK_MP_DEFAULT_LOCALE;
  }
  const normalized = value.trim().toLowerCase();
  if (normalized.startsWith("zh")) {
    return "zh-CN";
  }
  if (normalized.startsWith("en")) {
    return "en-US";
  }
  return SDKWORK_MP_DEFAULT_LOCALE;
}
