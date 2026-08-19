import { getKnowledgebasePcSdkPorts } from './sdkPorts';

function readDocumentThemeSnapshot(): { hadDark: boolean; hadLightMode: boolean; sdkColorMode: string | null } {
  const root = document.documentElement
  return {
    hadDark: root.classList.contains('dark'),
    hadLightMode: root.classList.contains('light-mode'),
    sdkColorMode: root.getAttribute('data-sdk-color-mode'),
  }
}

function restoreDocumentThemeSnapshot(snapshot: { hadDark: boolean; hadLightMode: boolean; sdkColorMode: string | null }): void {
  const root = document.documentElement
  root.classList.toggle('dark', snapshot.hadDark)
  root.classList.toggle('light-mode', snapshot.hadLightMode)
  if (snapshot.sdkColorMode === null) root.removeAttribute('data-sdk-color-mode')
  else root.setAttribute('data-sdk-color-mode', snapshot.sdkColorMode)
}

function applyHostColorScheme(scheme: 'light' | 'dark'): void {
  if (typeof document === 'undefined') {
    return;
  }

  document.documentElement.classList.toggle('dark', scheme === 'dark');
  document.documentElement.classList.toggle('light-mode', scheme === 'light');
  document.documentElement.setAttribute('data-sdk-color-mode', scheme);
}

export function syncKnowledgebaseHostColorScheme(): void {
  try {
    const ports = getKnowledgebasePcSdkPorts();
    const scheme = ports.resolveHostColorScheme?.();
    if (scheme === undefined) {
      return;
    }
    applyHostColorScheme(scheme);
  } catch {
    // Host SDK ports may not be configured during standalone bootstrap.
  }
}

export function subscribeKnowledgebaseHostColorScheme(onChange?: () => void): (() => void) | undefined {
  try {
    const ports = getKnowledgebasePcSdkPorts();
    const subscribe = ports.subscribeHostColorScheme;
    if (!subscribe) {
      return undefined;
    }

    const previous = typeof document === 'undefined' ? undefined : readDocumentThemeSnapshot();
    const unsubscribe = subscribe((scheme) => {
      applyHostColorScheme(scheme);
      onChange?.();
    });
    return () => {
      unsubscribe();
      if (previous !== undefined) restoreDocumentThemeSnapshot(previous);
    };
  } catch {
    return undefined;
  }
}

export function resolveKnowledgebaseHostColorScheme(): 'light' | 'dark' | undefined {
  try {
    return getKnowledgebasePcSdkPorts().resolveHostColorScheme?.();
  } catch {
    return undefined;
  }
}
