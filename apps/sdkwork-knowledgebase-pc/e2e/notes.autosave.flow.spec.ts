import { test, expect } from '@playwright/test';

import {
  createE2eMockTelemetry,
  setupKnowledgebaseE2ePage,
} from './knowledgeApiMocks';

/**
 * Notes workspace write → autosave flow, exercised in a real browser:
 * open the notes tab (first mocked document auto-selects), edit title and
 * body, assert the autosave status pill reaches its saved state and the mock
 * backend received both the rename and the ingest payload, then reload and
 * verify title + content persist through the mocked documents/content APIs.
 */
test.describe('Notes workspace write → autosave flow', () => {
  test('edits a note and persists title and content through autosave', async ({ page }) => {
    const telemetry = createE2eMockTelemetry();
    await setupKnowledgebaseE2ePage(page, telemetry);

    await page.goto('/');
    await expect(page.getByTestId('knowledgebase-pc-app-shell')).toBeVisible({ timeout: 30_000 });

    await page.getByTitle('Notes').click();
    const workspace = page.getByTestId('knowledgebase-pc-notes-workspace');
    await expect(workspace).toBeVisible({ timeout: 15_000 });

    // The first mocked document (E2E Source Document) auto-selects and loads
    // into the shared Tiptap editor.
    const editor = page.locator('.ProseMirror').first();
    await expect(editor).toBeVisible({ timeout: 15_000 });

    // Body edit → debounced autosave must deliver the new content via ingest
    // and the status pill must settle on the saved state.
    const bodyText = 'Notes workspace autosave flow verification content.';
    await editor.click();
    await editor.fill(bodyText);

    const status = page.getByTestId('knowledgebase-pc-notes-save-status');
    await expect(status).toBeVisible();
    await expect
      .poll(
        () => telemetry.ingestPayloads.some((payload) => payload.includes(bodyText)),
        { timeout: 15_000, message: 'expected the debounced save to ingest the new body' },
      )
      .toBe(true);
    await expect(status).toContainText('Saved', { timeout: 15_000 });

    // Title edit → in-place rename via the drive node update (browser-listed
    // documents rename through drive metadata, projected onto the document).
    const titleInput = page.locator('input.text-3xl').first();
    await expect(titleInput).toBeVisible();
    await titleInput.fill('Notes Autosave E2E');
    await expect
      .poll(
        () =>
          telemetry.requestedPaths.some(
            (path) =>
              (path.startsWith('PATCH /app/v3/api/drive/nodes/drive-node-') ||
                path.startsWith('PUT /app/v3/api/drive/nodes/drive-node-')) &&
              path.includes('1001'),
          ),
        { timeout: 15_000, message: 'expected the title rename to reach the drive node update' },
      )
      .toBe(true);

    // Reload: list and editor must reflect the persisted title and content.
    await page.reload();
    await expect(page.getByTestId('knowledgebase-pc-app-shell')).toBeVisible({ timeout: 30_000 });
    await page.getByTitle('Notes').click();
    const workspaceAfterReload = page.getByTestId('knowledgebase-pc-notes-workspace');
    await expect(workspaceAfterReload).toBeVisible({ timeout: 15_000 });

    await expect(
      page.getByTestId('knowledgebase-pc-notes-item').first(),
    ).toContainText('Notes Autosave E2E', { timeout: 15_000 });

    const editorAfterReload = page.locator('.ProseMirror').first();
    await expect(editorAfterReload).toBeVisible({ timeout: 15_000 });
    await expect(editorAfterReload).toContainText(bodyText, { timeout: 15_000 });
  });

  test('surfaces a recovery action when a save fails', async ({ page }) => {
    const telemetry = createE2eMockTelemetry();
    await setupKnowledgebaseE2ePage(page, telemetry);

    // Fail every drive node update (the title rename path for browser-listed
    // documents) so the autosave engine enters its error state after the
    // retry budget is exhausted.
    await page.route('**/app/v3/api/drive/nodes/**', async (route) => {
      if (route.request().method() === 'PATCH' || route.request().method() === 'PUT') {
        await route.fulfill({
          status: 500,
          contentType: 'application/problem+json',
          body: JSON.stringify({
            code: 50000,
            traceId: '00000000-0000-4000-8000-00000000000e',
            detail: 'forced e2e failure',
          }),
        });
        return;
      }
      await route.fallback();
    });

    await page.goto('/');
    await expect(page.getByTestId('knowledgebase-pc-app-shell')).toBeVisible({ timeout: 30_000 });
    await page.getByTitle('Notes').click();
    await expect(page.getByTestId('knowledgebase-pc-notes-workspace')).toBeVisible({ timeout: 15_000 });

    const titleInput = page.locator('input.text-3xl').first();
    await expect(titleInput).toBeVisible({ timeout: 15_000 });
    await titleInput.fill('Rename that must fail');

    // Retry budget exhausted → the pill offers the explicit retry action.
    const retry = page.getByTestId('knowledgebase-pc-notes-retry');
    await expect(retry).toBeVisible({ timeout: 15_000 });
  });

  test('recovers automatically when the network heals after failed saves', async ({ page }) => {
    const telemetry = createE2eMockTelemetry();
    await setupKnowledgebaseE2ePage(page, telemetry);

    // Simulate an outage: every content ingest fails while `ingestOnline` is
    // false. The engine must burn its retry budget, surface the failure pill,
    // keep the buffered body, and then save it via the explicit retry action
    // once the outage ends.
    let ingestOnline = false;
    await page.route('**/app/v3/api/knowledge/ingests', async (route) => {
      if (!ingestOnline) {
        await route.fulfill({
          status: 503,
          contentType: 'application/problem+json',
          body: JSON.stringify({
            code: 50301,
            traceId: '00000000-0000-4000-8000-00000000000f',
            detail: 'simulated outage',
          }),
        });
        return;
      }
      await route.fallback();
    });

    await page.goto('/');
    await expect(page.getByTestId('knowledgebase-pc-app-shell')).toBeVisible({ timeout: 30_000 });
    await page.getByTitle('Notes').click();
    await expect(page.getByTestId('knowledgebase-pc-notes-workspace')).toBeVisible({ timeout: 15_000 });

    const editor = page.locator('.ProseMirror').first();
    await expect(editor).toBeVisible({ timeout: 15_000 });
    const bodyText = 'Offline recovery flow content.';
    await editor.click();
    await editor.fill(bodyText);

    const retry = page.getByTestId('knowledgebase-pc-notes-retry');
    await expect(retry).toBeVisible({ timeout: 20_000 });

    // The outage ends: the explicit retry (the pill's action) must now save
    // the buffered body without any re-typing.
    ingestOnline = true;
    await retry.click();
    await expect
      .poll(
        () => telemetry.ingestPayloads.some((payload) => payload.includes(bodyText)),
        { timeout: 15_000, message: 'expected the retried save to ingest the buffered body' },
      )
      .toBe(true);
    await expect(page.getByTestId('knowledgebase-pc-notes-save-status')).toContainText(
      'Saved',
      { timeout: 15_000 },
    );
  });

  test('saves immediately on Ctrl+S instead of waiting for the debounce', async ({ page }) => {
    test.setTimeout(30_000);
    const telemetry = createE2eMockTelemetry();
    await setupKnowledgebaseE2ePage(page, telemetry);

    await page.goto('/');
    await expect(page.getByTestId('knowledgebase-pc-app-shell')).toBeVisible({ timeout: 30_000 });
    await page.getByTitle('Notes').click();
    const editor = page.locator('.ProseMirror').first();
    await expect(editor).toBeVisible({ timeout: 15_000 });

    const bodyText = 'Ctrl+S immediate flush content.';
    await editor.click();
    await editor.fill(bodyText);

    // Manual save: the ingest must arrive inside the debounce window (1.2 s),
    // proving Ctrl+S flushed immediately instead of waiting for the timer.
    const pressedAt = Date.now();
    await page.keyboard.press('Control+s');
    await expect
      .poll(
        () => telemetry.ingestPayloads.some((payload) => payload.includes(bodyText)),
        { timeout: 1_100, message: 'expected Ctrl+S to flush before the 1.2 s debounce' },
      )
      .toBe(true);
    expect(Date.now() - pressedAt).toBeLessThan(1_100);
    await expect(page.getByTestId('knowledgebase-pc-notes-save-status')).toContainText(
      'Saved',
      { timeout: 15_000 },
    );
  });
});
