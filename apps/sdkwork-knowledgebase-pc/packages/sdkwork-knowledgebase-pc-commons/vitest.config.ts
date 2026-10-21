import { defineConfig } from 'vitest/config';
import { vitestSharedAliases } from '../../vitest.shared';

export default defineConfig({
  resolve: {
    // One React instance per test graph: sibling workspace packages can resolve their own
    // copy, which breaks hooks with "Invalid hook call".
    dedupe: ['react', 'react-dom'],
    alias: {
      ...vitestSharedAliases,
    },
  },
  test: {
    environment: 'jsdom',
    include: ['src/**/*.test.ts'],
  },
});
