import enDetail from "./en-US/intelligence/knowledge/detail.json";
import enLaunch from "./en-US/intelligence/knowledge/launch.json";
import enList from "./en-US/intelligence/knowledge/list.json";
import enSearch from "./en-US/intelligence/knowledge/search.json";
import enSettings from "./en-US/intelligence/knowledge/settings.json";
import zhDetail from "./zh-CN/intelligence/knowledge/detail.json";
import zhLaunch from "./zh-CN/intelligence/knowledge/launch.json";
import zhList from "./zh-CN/intelligence/knowledge/list.json";
import zhSearch from "./zh-CN/intelligence/knowledge/search.json";
import zhSettings from "./zh-CN/intelligence/knowledge/settings.json";

/** Thin i18n aggregator; authored copy stays in the locale fragments. */
export const knowledgebaseH5I18nResources = {
  "en-US": {
    detail: enDetail,
    launch: enLaunch,
    list: enList,
    search: enSearch,
    settings: enSettings,
  },
  "zh-CN": {
    detail: zhDetail,
    launch: zhLaunch,
    list: zhList,
    search: zhSearch,
    settings: zhSettings,
  },
} as const;
