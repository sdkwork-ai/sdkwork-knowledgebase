import type { AuthTokenManager } from '@sdkwork/knowledgebase-app-sdk';

import type { KnowledgebaseSessionTokenManager } from '../session/sessionTokenManager';

/**
 * Token managers the generated-SDK wrappers may bind: the app-owned
 * session-backed instance, or the embedding host's shared instance.
 * APP_SDK_INTEGRATION_SPEC closure rule: within one authenticated session
 * context every SDK client shares exactly one TokenManager instance, so a
 * host-managed runtime binds the host's manager instead of minting another.
 */
export type KnowledgebaseSdkTokenManager = KnowledgebaseSessionTokenManager | AuthTokenManager;
