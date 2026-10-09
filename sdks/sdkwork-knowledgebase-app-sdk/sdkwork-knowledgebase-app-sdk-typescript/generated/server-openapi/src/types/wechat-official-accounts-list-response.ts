import type { KnowledgeWechatOfficialAccount } from './knowledge-wechat-official-account';
import type { PageInfo } from './page-info';

export interface WechatOfficialAccountsListResponse {
  code: 0;
  data: unknown & { items: KnowledgeWechatOfficialAccount[]; pageInfo: PageInfo; };
  /** Server-owned request correlation id. */
  traceId: string;
}
