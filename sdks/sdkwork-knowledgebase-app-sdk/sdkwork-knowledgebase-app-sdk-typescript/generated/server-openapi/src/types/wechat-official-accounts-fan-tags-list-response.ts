import type { KnowledgeWechatFanTag } from './knowledge-wechat-fan-tag';
import type { PageInfo } from './page-info';

export interface WechatOfficialAccountsFanTagsListResponse {
  code: 0;
  data: unknown & { items: KnowledgeWechatFanTag[]; pageInfo: PageInfo; };
  /** Server-owned request correlation id. */
  traceId: string;
}
