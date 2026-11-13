import { accountScopedStorageKey } from 'sdkwork-knowledgebase-pc-core';

import { SEARCH_SESSIONS_STORAGE_KEY } from './constants';
import type { SearchSession } from './types';

export const MAX_SEARCH_SESSIONS = 50;
export const MAX_MESSAGES_PER_SESSION = 100;

/**
 * Storage key of the signed-in account's search history.
 *
 * Sessions quote the account's own questions, answers and cited knowledge-base
 * titles, so the key is account-scoped: a shared profile that switches accounts
 * must not render (or re-persist) the previous account's history.
 *
 * @returns `<base>.<tenantId>.<userId>`, or the legacy base key before an
 *   identity is bound.
 */
export function currentSearchSessionsStorageKey(): string {
  return accountScopedStorageKey(SEARCH_SESSIONS_STORAGE_KEY);
}

export function trimSearchSessions(sessions: SearchSession[]): SearchSession[] {
  const trimmedSessions = sessions
    .slice(0, MAX_SEARCH_SESSIONS)
    .map((session) => ({
      ...session,
      messages: session.messages.slice(-MAX_MESSAGES_PER_SESSION),
    }));

  return trimmedSessions;
}

export function persistSearchSessions(
  storageKey: string,
  sessions: SearchSession[],
): SearchSession[] {
  const trimmed = trimSearchSessions(sessions);
  try {
    localStorage.setItem(storageKey, JSON.stringify(trimmed));
  } catch (error) {
    console.error('Failed to persist search sessions to localStorage', error);
  }
  return trimmed;
}

export function loadSearchSessions(storageKey: string): SearchSession[] | null {
  try {
    const stored = localStorage.getItem(storageKey);
    if (!stored) {
      return null;
    }
    const parsed = JSON.parse(stored) as SearchSession[];
    if (!Array.isArray(parsed) || parsed.length === 0) {
      return null;
    }
    return trimSearchSessions(parsed);
  } catch (error) {
    console.error('Failed to parse search sessions from localStorage', error);
    return null;
  }
}
