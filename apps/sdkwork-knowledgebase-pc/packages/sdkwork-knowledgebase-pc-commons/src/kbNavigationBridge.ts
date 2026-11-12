/**
 * Base key of the one-shot search → knowledge-base navigation intent.
 *
 * Callers persist it under an account-scoped key so a pending intent cannot
 * point the next account at a document it may not open.
 */
export const KB_NAV_INTENT_KEY = 'app-kb-nav-intent';

export interface KbNavIntent {
  kbId: string;
  kbTitle?: string;
  docId?: string;
  docTitle?: string;
  docType?: string;
  parentId?: string | null;
  author?: string;
  updatedAt?: string;
  highlight?: boolean;
}

export function setKbNavIntent(intent: KbNavIntent, storageKey: string = KB_NAV_INTENT_KEY) {
  sessionStorage.setItem(storageKey, JSON.stringify(intent));
}

export function readKbNavIntent(storageKey: string = KB_NAV_INTENT_KEY): KbNavIntent | null {
  try {
    const raw = sessionStorage.getItem(storageKey);
    if (!raw) return null;
    return JSON.parse(raw) as KbNavIntent;
  } catch {
    return null;
  }
}

export function clearKbNavIntent(storageKey: string = KB_NAV_INTENT_KEY) {
  sessionStorage.removeItem(storageKey);
}

export interface KbLocateFileDetail {
  docId: string;
  parentId?: string | null;
}

export function dispatchLocateKbFile(detail: KbLocateFileDetail) {
  window.dispatchEvent(new CustomEvent('kb-locate-file', { detail }));
}

export const APP_OPEN_BROWSER_EVENT = 'app-open-in-app-browser';

export interface InAppBrowserOpenDetail {
  url: string;
  title?: string;
}

export function dispatchOpenInAppBrowser(detail: InAppBrowserOpenDetail) {
  window.dispatchEvent(new CustomEvent(APP_OPEN_BROWSER_EVENT, { detail }));
}
