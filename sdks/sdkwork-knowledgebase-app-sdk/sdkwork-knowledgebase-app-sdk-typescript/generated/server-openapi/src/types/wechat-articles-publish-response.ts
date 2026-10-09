import type { KnowledgeWechatOperationResult } from './knowledge-wechat-operation-result';

export interface WechatArticlesPublishResponse {
  code: 0;
  data: unknown & { item: KnowledgeWechatOperationResult; };
  /** Server-owned request correlation id. */
  traceId: string;
}
