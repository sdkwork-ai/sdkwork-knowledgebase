import { test, expect } from '@playwright/test';

import {
  createE2eMockTelemetry,
  setupKnowledgebaseE2ePage,
} from './knowledgeApiMocks';

/**
 * KB-view "New Note" lifecycle, exercised in a real browser: the note is a
 * knowledge document without a drive node, so it must be listed through the
 * notes projection (not the drive tree), survive a reload, save its body
 * through the title-keyed ingest, and expose the manual save action.
 */
test.describe('KB view note lifecycle', () => {
  test('creates a note, saves it, and reopens it after reload', async ({ page }) => {
    const telemetry = createE2eMockTelemetry();
    await setupKnowledgebaseE2ePage(page, telemetry);

    await page.goto('/');
    await expect(page.getByTestId('knowledgebase-pc-app-shell')).toBeVisible({ timeout: 30_000 });
    await page.getByTestId('knowledgebase-pc-add-menu-trigger').click();
    await page.getByRole('menuitem', { name: /New Note/ }).click();

    const titleInput = page.locator('input.text-3xl').first();
    await expect(titleInput).toBeVisible({ timeout: 15_000 });
    await expect(titleInput).toHaveValue('Untitled Note');

    const editor = page.locator('.ProseMirror').first();
    await expect(editor).toBeVisible({ timeout: 15_000 });
    await editor.fill('KB note verification body.');

    // Manual save: the ingest must land immediately, before the debounce.
    const pressedAt = Date.now();
    await page.getByTestId('knowledgebase-pc-editor-save').click();
    await expect
      .poll(
        () => telemetry.ingestPayloads.some((payload) => payload.includes('KB note verification body.')),
        { timeout: 1_100, message: 'expected the manual save to flush before the 1.2 s debounce' },
      )
      .toBe(true);
    expect(Date.now() - pressedAt).toBeLessThan(1_100);
    await expect(page.getByTestId('knowledgebase-pc-editor-save-status')).toContainText('Saved', {
      timeout: 15_000,
    });

    // The note must be listed immediately (created documents appear in the list).
    await expect(page.getByText('Untitled Note', { exact: true }).first()).toBeVisible();

    // Reload: the note has no drive node, so only the notes listing can expose
    // it in the browser tree.
    await page.reload();
    await expect(page.getByTestId('knowledgebase-pc-app-shell')).toBeVisible({ timeout: 30_000 });
    const listed = page.getByText('Untitled Note', { exact: true }).first();
    await expect(listed).toBeVisible({ timeout: 15_000 });

    await listed.click();
    const editorAfterReload = page.locator('.ProseMirror').first();
    await expect(editorAfterReload).toBeVisible({ timeout: 15_000 });
    await expect(editorAfterReload).toContainText('KB note verification body.', { timeout: 15_000 });
  });

  test('coalesces title typing into a single rename request', async ({ page }) => {
    const telemetry = createE2eMockTelemetry();
    await setupKnowledgebaseE2ePage(page, telemetry);

    await page.goto('/');
    await expect(page.getByTestId('knowledgebase-pc-app-shell')).toBeVisible({ timeout: 30_000 });
    await page.getByTestId('knowledgebase-pc-add-menu-trigger').click();
    await page.getByRole('menuitem', { name: /New Note/ }).click();

    const titleInput = page.locator('input.text-3xl').first();
    // The created note replaces the previously open editor; wait for the new
    // one (default title focused and selected) before typing.
    await expect(titleInput).toHaveValue('Untitled Note', { timeout: 15_000 });
    await expect(titleInput).toBeFocused();

    // Four keystrokes must coalesce into one debounced rename, not one PATCH
    // per character (the editor shares its autosave pipeline with the body).
    await titleInput.pressSequentially('项目周报', { delay: 60 });
    await expect(titleInput).toHaveValue('项目周报');

    await expect(page.getByTestId('knowledgebase-pc-editor-save-status')).toContainText('Saved', {
      timeout: 15_000,
    });
    const renames = telemetry.requestedPaths.filter((path) =>
      /^PATCH \/app\/v3\/api\/knowledge\/documents\/\d+$/.test(path),
    );
    expect(renames).toHaveLength(1);

    // Nothing left to persist: the manual save action is disabled and the
    // tree shows the persisted title.
    await expect(page.getByTestId('knowledgebase-pc-editor-save')).toBeDisabled();
    await expect(page.getByText('项目周报', { exact: true }).first()).toBeVisible();
  });
});
