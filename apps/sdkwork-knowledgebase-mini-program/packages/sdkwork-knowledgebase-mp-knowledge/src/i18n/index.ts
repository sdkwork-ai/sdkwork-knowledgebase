/**
 * Thin i18n aggregation for the knowledgebase mini program capability package.
 *
 * Authority: `I18N_SPEC.md` section 6.1. This file may import and re-export
 * authored fragments; it MUST NOT author feature copy itself.
 */
import { knowledgebaseMpDetailEnUs } from "./en-US/intelligence/knowledge/detail";
import { knowledgebaseMpLaunchEnUs } from "./en-US/intelligence/knowledge/launch";
import { knowledgebaseMpListEnUs } from "./en-US/intelligence/knowledge/list";
import { knowledgebaseMpSearchEnUs } from "./en-US/intelligence/knowledge/search";
import { knowledgebaseMpSettingsEnUs } from "./en-US/intelligence/knowledge/settings";
import { knowledgebaseMpDetailZhCn } from "./zh-CN/intelligence/knowledge/detail";
import { knowledgebaseMpLaunchZhCn } from "./zh-CN/intelligence/knowledge/launch";
import { knowledgebaseMpListZhCn } from "./zh-CN/intelligence/knowledge/list";
import { knowledgebaseMpSearchZhCn } from "./zh-CN/intelligence/knowledge/search";
import { knowledgebaseMpSettingsZhCn } from "./zh-CN/intelligence/knowledge/settings";

/** Locale-keyed view of the knowledgebase fragments. */
export const knowledgebaseMpI18nFragments: Record<string, Record<string, string>> = {
  "en-US": {
    ...knowledgebaseMpDetailEnUs,
    ...knowledgebaseMpLaunchEnUs,
    ...knowledgebaseMpListEnUs,
    ...knowledgebaseMpSearchEnUs,
    ...knowledgebaseMpSettingsEnUs,
  },
  "zh-CN": {
    ...knowledgebaseMpDetailZhCn,
    ...knowledgebaseMpLaunchZhCn,
    ...knowledgebaseMpListZhCn,
    ...knowledgebaseMpSearchZhCn,
    ...knowledgebaseMpSettingsZhCn,
  },
};
