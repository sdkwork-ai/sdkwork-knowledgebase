import path from 'node:path';
import { fileURLToPath } from 'node:url';
import { defineConfig } from 'vitest/config';
import { vitestSharedAliases } from '../../vitest.shared';

void (path.dirname(fileURLToPath(import.meta.url)));

export default defineConfig({
  resolve: {
    alias: {
      ...vitestSharedAliases}},
  test: {
    environment: 'node',
    include: ['src/**/*.test.ts']}});
