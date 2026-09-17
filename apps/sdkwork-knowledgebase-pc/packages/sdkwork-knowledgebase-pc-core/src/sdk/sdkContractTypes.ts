/**
 * Composed app SDK contract types re-exported for capability packages.
 * Feature modules must import SDK DTO types from pc-core, not from generated SDK packages.
 */
export type {
  IngestionJob,
  AuthTokenManager,
  KnowledgeAccessLevel,
  KnowledgeAgentKnowledgeMode,
  KnowledgeBrowserListData,
  KnowledgeBrowserNode,
  KnowledgeBrowserView,
  KnowledgeDocumentContent,
  KnowledgeSpaceContextBinding,
  KnowledgeSpaceMember,
  KnowledgeSpaceMemberRole,
  KnowledgeWechatApplet,
  KnowledgeWechatArticle,
  KnowledgeWechatOfficialAccount,
  SdkworkKnowledgebaseAppClient,
} from '@sdkwork/knowledgebase-app-sdk';

export type {
  DriveNode,
  DriveNodeProperty,
  DriveUploaderProfile,
  SdkworkDriveAppClient,
} from '@sdkwork/drive-app-sdk';
