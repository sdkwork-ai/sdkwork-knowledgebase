import path from 'node:path';
import { fileURLToPath } from 'node:url';
import { defineConfig } from 'vitest/config';
import { vitestSharedAliases } from '../../vitest.shared';

void (path.dirname(fileURLToPath(import.meta.url)));

export default defineConfig({
  resolve: {
    // Authored sources first: the package tree also carries compiled `.js`
    // siblings, and Vite's default extension order would import those instead
    // of the `.ts`/`.tsx` sources under test (the app's vite.config.ts pins the
    // same order for the same reason).
    extensions: ['.ts', '.tsx', '.mjs', '.js', '.jsx', '.json'],
    // One React instance per test graph: sibling workspace packages can resolve their own
    // copy, which breaks hooks with "Invalid hook call".
    dedupe: ['react', 'react-dom'],
    alias: {
      ...vitestSharedAliases}},
  test: {
    environment: 'node',
    include: ['src/**/*.test.ts']}});
