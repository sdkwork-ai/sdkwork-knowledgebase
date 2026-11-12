import {
  getKnowledgebaseAppSdkClient,
  isKnowledgebaseApiAvailable,
} from './knowledgebaseApiRegistry';
import { parseSdkProblemDetails } from '../errors/sdkProblemError';
import type { KnowledgeDocumentContent } from '../sdk/sdkContractTypes.js';

export type KnowledgeDocumentContentPayload = KnowledgeDocumentContent;

/**
 * Read one document's authoritative body.
 *
 * A permission denial is NOT an unavailable body: returning `null` for it made
 * callers fall back to a locally cached copy and answer content the account may
 * no longer read. Only an unavailable API or a transient failure answers `null`.
 */
export async function fetchKnowledgeDocumentContent(
  documentId: string | number,
): Promise<KnowledgeDocumentContentPayload | null> {
  if (!isKnowledgebaseApiAvailable()) {
    return null;
  }

  try {
    const sdk = getKnowledgebaseAppSdkClient();
    return await sdk.client.knowledge.documents.content.list(String(documentId));
  } catch (error) {
    const status = parseSdkProblemDetails(error)?.status;
    if (status === 403 || status === 404) {
      throw error;
    }
    return null;
  }
}
