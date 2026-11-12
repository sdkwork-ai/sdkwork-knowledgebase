import { afterEach, describe, expect, it } from 'vitest';

import {
  configureKnowledgebaseAppSdk,
  setKnowledgebaseApiEnabled,
} from './knowledgebaseApiRegistry';
import { fetchKnowledgeDocumentContent } from './knowledgebaseDocumentContentApi';

function configureContentRead(list: (documentId: string) => Promise<unknown>): void {
  configureKnowledgebaseAppSdk({
    client: {
      knowledge: {
        documents: {
          content: { list },
        },
      },
    },
    setTokenManager() {
      // Test fake; token propagation is covered by SDK bootstrap tests.
    },
  } as never);
  setKnowledgebaseApiEnabled(true);
}

afterEach(() => {
  setKnowledgebaseApiEnabled(false);
});

describe('fetchKnowledgeDocumentContent', () => {
  it('rethrows an access denial so callers do not answer from a stale cache', async () => {
    configureContentRead(async () => {
      throw { status: 403, code: 'knowledge_space_access_denied' };
    });

    await expect(fetchKnowledgeDocumentContent('99')).rejects.toMatchObject({ status: 403 });
  });

  it('rethrows a missing document', async () => {
    configureContentRead(async () => {
      throw { status: 404 };
    });

    await expect(fetchKnowledgeDocumentContent('99')).rejects.toMatchObject({ status: 404 });
  });

  it('answers null for a transient failure', async () => {
    configureContentRead(async () => {
      throw { status: 503 };
    });

    await expect(fetchKnowledgeDocumentContent('99')).resolves.toBeNull();
  });

  it('answers null when the API is unavailable', async () => {
    setKnowledgebaseApiEnabled(false);

    await expect(fetchKnowledgeDocumentContent('99')).resolves.toBeNull();
  });
});
