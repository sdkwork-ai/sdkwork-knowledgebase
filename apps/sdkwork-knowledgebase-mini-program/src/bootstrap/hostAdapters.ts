import { createKnowledgebaseMpWxHostAdapter } from "@sdkwork/knowledgebase-mp-host";

/** Register the WeChat mini program host adapter. */
export function registerHostAdapters() {
  return createKnowledgebaseMpWxHostAdapter();
}
