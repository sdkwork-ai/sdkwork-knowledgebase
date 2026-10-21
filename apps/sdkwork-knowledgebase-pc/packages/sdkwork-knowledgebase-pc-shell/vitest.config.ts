import path from 'node:path';
import { fileURLToPath } from 'node:url';
import { defineConfig } from 'vitest/config';
import { vitestSharedAliases } from '../../vitest.shared';

void (path.dirname(fileURLToPath(import.meta.url)));

export default defineConfig({
  resolve: {
    // One React instance per test graph: sibling workspace packages can resolve their own
    // copy, which breaks hooks with "Invalid hook call".
    dedupe: ['react', 'react-dom'],
    alias: {
      ...vitestSharedAliases}},
  test: {
    environment: 'node',
    include: ['src/**/*.test.ts']}});
