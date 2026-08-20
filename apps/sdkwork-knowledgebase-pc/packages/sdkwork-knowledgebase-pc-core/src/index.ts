export * from './auth/authGate';
export * from './types';
export * from './api/knowledgebaseApiRegistry';
export * from './api/knowledgebaseConnectivity';
export * from './api/knowledgebaseDriveApiRegistry';
export * from './api/knowledgebaseSpaceRegistry';
export * from './api/knowledgebaseDocumentContentCache';
export * from './api/knowledgebaseDocumentContentApi';
export * from './api/knowledgebaseRecentDocuments';
export * from './account/accountViewModel';
export {
  createRuntimeConfig,
  detectRuntimeTargetFromEnv,
  isDevSameOriginApiEnabled,
  isKnowledgebaseAppApiConfigured,
} from './config/runtimeConfig';
export {
  normalizeKnowledgebaseBrowserBasePath,
  toKnowledgebaseViteBasePath,
} from './config/browserBasePath';
export { resolveKnowledgebaseFeatureFlags } from './config/knowledgebaseFeatureFlags';
export type { KnowledgebaseFeatureFlags } from './config/knowledgebaseFeatureFlags';
export * from './sdk/knowledgebaseAppSdkClient';
export * from './sdk/driveAppSdkClient';
export * from './sdk/groupKnowledgebaseLaunch';
export * from './sdk/sdkContractTypes';
export * from './session/sessionStore';
export * from './session/sessionTokenManager';
export {
  signOutKnowledgebaseAccount,
  signOutKnowledgebaseSession,
  useHydrateKnowledgebaseAccount,
} from './session/accountSession';
export type {
  KnowledgebaseRuntimeConfig,
  RuntimeEnv,
  SdkworkAuthRuntimeConfig,
  SdkworkBuildMode,
  SdkworkConfigProfile,
  SdkworkDeploymentProfile,
  SdkworkEnvironment,
  SdkworkRuntimeTarget,
} from './config/runtimeConfig';
export { createHostAdapter, decodeBinaryResourcePayload } from './host/hostAdapter';
export {
  invokeDesktopCommand,
  isTauriDesktopRuntime,
  listenDesktopEvent,
} from './host/tauriBridge';
export type {
  BinaryResourcePayload,
  HostAdapter,
  NativeSaveExportFileResult,
  WindowControlAction,
} from './host/hostAdapter';
export {
  KnowledgebaseRuntimeProvider,
  useKnowledgebaseHostAdapter,
  useKnowledgebaseRuntime,
  useKnowledgebaseRuntimeConfig,
} from './runtime/KnowledgebaseRuntimeProvider';
export {
  captureGroupKnowledgebaseLaunchTicket,
  sanitizeGroupKnowledgebaseLaunchAuthLocation,
  takePendingGroupKnowledgebaseLaunchTicket,
} from './runtime/groupKnowledgebaseLaunchHandoff';
export {
  GROUP_KNOWLEDGEBASE_LAUNCH_PATH,
} from './runtime/groupKnowledgebaseLaunchHandoff';
export type { GroupKnowledgebaseLaunchLocation } from './runtime/groupKnowledgebaseLaunchHandoff';
export { isValidGroupKnowledgebaseLaunchTicket } from './runtime/groupKnowledgebaseLaunchTicket';
export type {
  KnowledgebasePcRuntime,
  KnowledgebaseRuntimeProviderProps,
} from './runtime/KnowledgebaseRuntimeProvider';
export { useKnowledgebaseSessionSnapshot } from './hooks/useKnowledgebaseSessionSnapshot';
export * from './errors/knowledgebaseErrorCodes';
export * from './errors/knowledgebaseAppError';
export * from './errors/sdkProblemError';
export * from './errors/resolveUserFacingError';
export * from './errors/serviceGuards';
