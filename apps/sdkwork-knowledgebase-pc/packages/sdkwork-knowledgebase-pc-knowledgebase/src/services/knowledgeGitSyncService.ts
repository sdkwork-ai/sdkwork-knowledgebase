import {
  KnowledgebaseErrorCodes,
  requireKnowledgebaseAppSdkHttpClient,
  requireNonEmptyString,
  requireRegisteredSpaceId,
} from 'sdkwork-knowledgebase-pc-core';

import { invalidateKnowledgeBrowserNodeCacheForSpaceIds } from './knowledgeBrowserListService';

export interface GitSyncProgress {
  phase: 'resolve' | 'push' | 'done';
  message: string;
  syncedCount?: number;
}

export interface GitSyncResult {
  accepted: boolean;
  status: string;
  hash: string;
  syncedCount: number;
}

function requireSdkClient() {
  return requireKnowledgebaseAppSdkHttpClient();
}

function normalizeBranch(branch: string): string {
  const trimmed = branch.trim();
  return trimmed || 'main';
}

function buildSyncIdempotencyKey(
  spaceId: string,
  repoUrl: string,
  branch: string,
  commitMessage: string,
): string {
  // Every sync attempt is a distinct operation: a key derived only from
  // space/repo/branch/message made the ingest create-or-get deduplicate a
  // second sync with the same commit message into the first job and silently
  // drop its changes.
  const randomSuffix = Math.random().toString(36).slice(2, 10);
  const raw = `git-sync-${spaceId}-${repoUrl.trim()}-${branch}-${commitMessage.trim()}-${Date.now()}-${randomSuffix}`;
  return raw.replace(/[^a-zA-Z0-9._-]/g, '-').slice(0, 128);
}

export async function syncGitRepository(
  kbId: string,
  repoUrl: string,
  branch: string,
  commitMessage: string,
  accessToken?: string,
  onProgress?: (progress: GitSyncProgress) => void,
): Promise<GitSyncResult> {
  const trimmedRepoUrl = requireNonEmptyString(repoUrl, KnowledgebaseErrorCodes.REPO_URL_REQUIRED);
  const trimmedCommitMessage = requireNonEmptyString(
    commitMessage,
    KnowledgebaseErrorCodes.COMMIT_MESSAGE_REQUIRED,
  );

  const normalizedBranch = normalizeBranch(branch);
  const spaceId = requireRegisteredSpaceId(kbId);
  const client = requireSdkClient();
  const idempotencyKey = buildSyncIdempotencyKey(
    spaceId,
    trimmedRepoUrl,
    normalizedBranch,
    trimmedCommitMessage,
  );
  const trimmedAccessToken = accessToken?.trim();

  onProgress?.({
    phase: 'resolve',
    message: `Resolving repository on branch "${normalizedBranch}"\u2026`,
  });

  onProgress?.({
    phase: 'push',
    message: 'Pushing knowledge base documents to the remote repository\u2026',
  });

  const result = await client.knowledge.gitSyncs.create({
    spaceId,
    repoUrl: trimmedRepoUrl,
    branch: normalizedBranch,
    commitMessage: trimmedCommitMessage,
    idempotencyKey,
    gitAccessToken: trimmedAccessToken || undefined,
  });

  onProgress?.({
    phase: 'done',
    message: `Synced ${result.syncedCount} file(s) to Git.`,
    syncedCount: result.syncedCount,
  });

  invalidateKnowledgeBrowserNodeCacheForSpaceIds(spaceId);
  return {
    accepted: result.accepted === true,
    status: result.status,
    hash: result.hash,
    syncedCount: result.syncedCount,
  };
}
