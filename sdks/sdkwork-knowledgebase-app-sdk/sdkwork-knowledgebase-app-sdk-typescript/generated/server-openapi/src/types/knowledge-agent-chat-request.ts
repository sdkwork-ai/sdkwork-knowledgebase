import type { KnowledgeAgentChatMessage } from './knowledge-agent-chat-message';
import type { KnowledgeAgentKnowledgeMode } from './knowledge-agent-knowledge-mode';

export interface KnowledgeAgentChatRequest {
  message: string;
  history?: KnowledgeAgentChatMessage[];
  mode?: KnowledgeAgentKnowledgeMode;
  sessionId?: string | null;
  modelProviderId?: string | null;
  modelId?: string | null;
  agentImplementationId?: string | null;
}
