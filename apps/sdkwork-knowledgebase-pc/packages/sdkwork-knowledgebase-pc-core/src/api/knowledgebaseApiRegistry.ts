import { isBlank, trim } from '@sdkwork/utils';

import type { KnowledgebaseAppSdkClient } from '../sdk/knowledgebaseAppSdkClient';
import { KnowledgebaseErrorCodes } from '../errors/knowledgebaseErrorCodes';
import { throwKnowledgebaseError } from '../errors/knowledgebaseAppError';

let appSdkClient: KnowledgebaseAppSdkClient | null = null;
let apiEnabled = false;

export function configureKnowledgebaseAppSdk(client: KnowledgebaseAppSdkClient): void {
  appSdkClient = client;
}

export function getKnowledgebaseAppSdkClient(): KnowledgebaseAppSdkClient {
  if (!appSdkClient) {
    throwKnowledgebaseError(KnowledgebaseErrorCodes.API_UNAVAILABLE_SDK);
  }
  return appSdkClient;
}

export function setKnowledgebaseApiEnabled(enabled: boolean): void {
  apiEnabled = enabled;
}

export function isKnowledgebaseApiAvailable(): boolean {
  return apiEnabled && appSdkClient !== null;
}

function readBooleanEnv(value: string | undefined): boolean | undefined {
  if (value === undefined || isBlank(value)) {
    return undefined;
  }
  const normalized = trim(value).toLowerCase();
  if (normalized === '1' || normalized === 'true' || normalized === 'yes') {
    return true;
  }
  if (normalized === '0' || normalized === 'false' || normalized === 'no') {
    return false;
  }
  return undefined;
}

/** Demo/offline UX requires the explicit `VITE_SDKWORK_KNOWLEDGEBASE_ENABLE_DEMO_MODE`
 * flag in every environment. Demo mode fabricates content (search answers, imported
 * notes/chat records, media previews) that must never reach real users, so it is never
 * enabled by default — not even in development — and a production bundle can never
 * enable it: `PROD` wins over any deployment-supplied flag. */
export function isKnowledgebaseDemoModeEnabled(
  env: Record<string, string | boolean | undefined> = import.meta.env as Record<
    string,
    string | boolean | undefined
  >,
): boolean {
  if (env.PROD === true) {
    return false;
  }
  const explicit = readBooleanEnv(typeof env.VITE_SDKWORK_KNOWLEDGEBASE_ENABLE_DEMO_MODE === 'string'
    ? env.VITE_SDKWORK_KNOWLEDGEBASE_ENABLE_DEMO_MODE
    : undefined);
  return explicit === true;
}

export function shouldUseKnowledgebaseDemoFallback(
  env: Record<string, string | boolean | undefined> = import.meta.env as Record<
    string,
    string | boolean | undefined
  >,
): boolean {
  return !isKnowledgebaseApiAvailable() && isKnowledgebaseDemoModeEnabled(env);
}

export function requireKnowledgebaseApi(operation: string): void {
  if (!isKnowledgebaseApiAvailable()) {
    throwKnowledgebaseError(KnowledgebaseErrorCodes.API_UNAVAILABLE, {
      cause: operation,
    });
  }
}
