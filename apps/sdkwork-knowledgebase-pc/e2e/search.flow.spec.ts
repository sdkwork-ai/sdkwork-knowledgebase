import { test, expect } from '@playwright/test';

import {
  E2E_SOURCE_DOCUMENT,
  setupKnowledgebaseE2ePage,
} from './knowledgeApiMocks';

test.describe('Knowledgebase PC search flow', () => {
  test('answers from the OKF bundle without attempting RAG retrieval', async ({ page }) => {
    // The registered E2E space runs in OKF-bundle mode, the product default. Its
    // content is answered by `okf.queries`; `POST /knowledge/retrievals` is
    // RAG-only and the gateway rejects it for this space (40003), so the search
    // must never send that request.
    const telemetry = await setupKnowledgebaseE2ePage(page);

    await page.goto('/');
    await expect(page.getByTestId('knowledgebase-pc-app-shell')).toBeVisible({ timeout: 30_000 });

    await page.getByTestId('knowledgebase-pc-nav-search').click();
    await expect(page.getByText('What would you like to search today?')).toBeVisible();

    const query = 'launch readiness verification';
    await page
      .getByPlaceholder('Ask anything. Press Enter to send, Shift+Enter for a new line...')
      .fill(query);
    await page.getByTitle('Send').click();

    await expect(page.getByText(E2E_SOURCE_DOCUMENT.title)).toBeVisible({ timeout: 30_000 });
    await expect(page.getByText(`OKF ? ${query}`)).toBeVisible({ timeout: 30_000 });

    await page.getByTestId('search-sources-toggle').click();
    await expect(page.getByTestId('search-source-row-doc')).toBeVisible();

    expect(telemetry.requestedPaths).not.toContain('POST /app/v3/api/knowledge/retrievals');
    expect(telemetry.requestedPaths).toContain('POST /app/v3/api/knowledge/okf/queries');
  });
});
