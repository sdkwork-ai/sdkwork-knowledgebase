import type { KnowledgeWechatApplet } from './knowledge-wechat-applet';
import type { PageInfo } from './page-info';

export interface WechatAppletsListResponse {
  code: 0;
  data: unknown & { items: KnowledgeWechatApplet[]; pageInfo: PageInfo; };
  /** Server-owned request correlation id. */
  traceId: string;
}
