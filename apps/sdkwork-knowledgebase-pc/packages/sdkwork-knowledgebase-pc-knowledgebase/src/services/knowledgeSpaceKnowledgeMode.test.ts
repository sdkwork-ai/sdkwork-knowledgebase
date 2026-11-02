import { afterEach, describe, expect, it, vi } from 'vitest';
import { configureKnowledgebaseAppSdk } from 'sdkwork-knowledgebase-pc-core';

import {
  forgetKnowledgeSpaceKnowledgeMode,
  partitionKnowledgeSpaceIdsByMode,
  resolveKnowledgeSpaceKnowledgeMode,
} from './knowledgeSpaceKnowledgeMode';

interface SpaceFixture {
  knowledgeMode?: string;
  okfBundleInitialized?: boolean;
}

function configureSpaces(spaces: Record<string, SpaceFixture | Error>): {
  retrieve: ReturnType<typeof vi.fn>;
} {
  const retrieve = vi.fn(async (spaceId: string) => {
    const fixture = spaces[spaceId];
    if (fixture === undefined) {
      throw new Error(`unknown space ${spaceId}`);
    }
    if (fixture instanceof Error) {
      throw fixture;
    }
    return { id: spaceId, ...fixture };
  });
  configureKnowledgebaseAppSdk({
    client: { knowledge: { spaces: { retrieve } } } as never,
    setTokenManager() {
      // Test fake; token propagation is covered by pc-core SDK bootstrap tests.
    },
  });
  return { retrieve };
}

afterEach(() => {
  forgetKnowledgeSpaceKnowledgeMode();
  vi.restoreAllMocks();
});

describe('resolveKnowledgeSpaceKnowledgeMode', () => {
  it('reports the space wire mode', async () => {
    configureSpaces({
      '1': { knowledgeMode: 'rag' },
      '2': { knowledgeMode: 'okf_bundle', okfBundleInitialized: true },
      '3': { knowledgeMode: 'external' },
    });

    await expect(resolveKnowledgeSpaceKnowledgeMode('1')).resolves.toBe('rag');
    await expect(resolveKnowledgeSpaceKnowledgeMode('2')).resolves.toBe('okf_bundle');
    await expect(resolveKnowledgeSpaceKnowledgeMode('3')).resolves.toBe('external');
  });

  it('reports okf_bundle for a missing wire value, which is the contract default', async () => {
    configureSpaces({ '7': {} });

    await expect(resolveKnowledgeSpaceKnowledgeMode('7')).resolves.toBe('okf_bundle');
  });

  it('caches a successful read for the session', async () => {
    const { retrieve } = configureSpaces({ '9': { knowledgeMode: 'okf_bundle' } });

    await resolveKnowledgeSpaceKnowledgeMode('9');
    await resolveKnowledgeSpaceKnowledgeMode(' 9 ');
    await resolveKnowledgeSpaceKnowledgeMode('9');

    expect(retrieve).toHaveBeenCalledTimes(1);
  });

  it('never reports rag when the space read fails, and retries afterwards', async () => {
    const warn = vi.spyOn(console, 'warn').mockImplementation(() => undefined);
    const { retrieve } = configureSpaces({ '4': new Error('gateway unavailable') });

    await expect(resolveKnowledgeSpaceKnowledgeMode('4')).resolves.toBe('okf_bundle');
    // An unresolved read is not cached: the next call retries and can classify.
    await expect(resolveKnowledgeSpaceKnowledgeMode('4')).resolves.toBe('okf_bundle');
    expect(retrieve).toHaveBeenCalledTimes(2);
    expect(warn).toHaveBeenCalled();
  });

  it('answers the contract default for a blank space id without a request', async () => {
    const { retrieve } = configureSpaces({});

    await expect(resolveKnowledgeSpaceKnowledgeMode('  ')).resolves.toBe('okf_bundle');
    expect(retrieve).not.toHaveBeenCalled();
  });
});

describe('partitionKnowledgeSpaceIdsByMode', () => {
  it('groups spaces by mode and collapses blanks and duplicates', async () => {
    configureSpaces({
      '1': { knowledgeMode: 'rag' },
      '2': { knowledgeMode: 'okf_bundle', okfBundleInitialized: true },
      '3': { knowledgeMode: 'external' },
      '4': new Error('not reachable'),
    });

    const partition = await partitionKnowledgeSpaceIdsByMode(['1', '2', '1', ' ', '3', '4']);

    expect(partition.rag).toEqual(['1']);
    expect(partition.okfBundle).toEqual(['2', '4']);
    expect(partition.external).toEqual(['3']);
  });
});
