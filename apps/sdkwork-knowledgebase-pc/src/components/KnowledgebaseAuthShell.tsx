import { useEffect, useState, type ReactNode } from 'react';
import { BookOpen, Moon, Sun, X } from 'lucide-react';
import { isTauriDesktopRuntime } from 'sdkwork-knowledgebase-pc-core';

type AuthThemeMode = 'dark' | 'light';

export function KnowledgebaseAuthShell({ children }: { children: ReactNode }) {
  const [themeMode, setThemeMode] = useState<AuthThemeMode>(() => {
    if (typeof window === 'undefined') {
      return 'dark';
    }
    const root = document.documentElement;
    const hostColorMode = root.getAttribute('data-sdk-color-mode');
    if (hostColorMode === 'dark' || hostColorMode === 'light') {
      return hostColorMode;
    }
    if (root.classList.contains('dark')) {
      return 'dark';
    }
    if (root.classList.contains('light-mode')) {
      return 'light';
    }
    return window.matchMedia('(prefers-color-scheme: light)').matches ? 'light' : 'dark';
  });

  const isLightMode = themeMode === 'light';
  const shouldRenderDesktopHeader = isTauriDesktopRuntime();

  // Single-writer arbitration (THEME_DARKMODE_SPEC §7.2): while a host owns the
  // documentElement mode root, this standalone auth shell must not write it.
  useEffect(() => {
    if (document.documentElement.getAttribute('data-sdk-color-mode') !== null) {
      return;
    }
    document.documentElement.classList.toggle('light-mode', isLightMode);
    document.documentElement.style.colorScheme = themeMode;
  }, [themeMode, isLightMode]);

  const toggleTheme = () => {
    setThemeMode((current) => (current === 'light' ? 'dark' : 'light'));
  };

  const handleMinimize = () => {
    window.dispatchEvent(new CustomEvent('sdkwork-knowledgebase:window-control', {
      detail: { action: 'minimize' },
    }));
  };

  const handleToggleMaximize = () => {
    window.dispatchEvent(new CustomEvent('sdkwork-knowledgebase:window-control', {
      detail: { action: 'toggleMaximize' },
    }));
  };

  const handleClose = () => {
    window.dispatchEvent(new CustomEvent('sdkwork-knowledgebase:window-control', {
      detail: { action: 'close' },
    }));
  };

  return (
    <div className="sdkwork-knowledgebase-auth-shell" data-testid="knowledgebase-pc-auth-shell">
      {shouldRenderDesktopHeader && (
        <header className="sdkwork-knowledgebase-auth-header drag-region">
          <div className="sdkwork-knowledgebase-auth-header-brand">
            <span className="sdkwork-knowledgebase-auth-header-mark">
              <BookOpen size={12} />
            </span>
            <span>SDKWork Knowledgebase</span>
          </div>
          <div className="sdkwork-knowledgebase-auth-header-center" />
          <div className="sdkwork-knowledgebase-auth-header-actions no-drag">
            <button
              aria-label={isLightMode ? 'Switch to dark mode' : 'Switch to light mode'}
              className="sdkwork-knowledgebase-auth-theme-button"
              onClick={toggleTheme}
              title={isLightMode ? 'Switch to dark mode' : 'Switch to light mode'}
              type="button"
            >
              {isLightMode ? <Moon size={14} /> : <Sun size={14} />}
            </button>
            <div className="sdkwork-knowledgebase-auth-window-controls">
              <button
                aria-label="Minimize window"
                className="sdkwork-knowledgebase-auth-window-button"
                onClick={handleMinimize}
                title="Minimize"
                type="button"
              >
                <svg aria-hidden="true" className="h-3.5 w-3.5" fill="none" viewBox="0 0 10 10">
                  <path d="M2 7H8" stroke="currentColor" strokeLinecap="square" strokeWidth="1" />
                </svg>
              </button>
              <button
                aria-label="Maximize window"
                className="sdkwork-knowledgebase-auth-window-button"
                onClick={handleToggleMaximize}
                title="Maximize"
                type="button"
              >
                <svg aria-hidden="true" className="h-3.5 w-3.5" fill="none" viewBox="0 0 10 10">
                  <path d="M2 2.5H8V8H2V2.5Z" stroke="currentColor" strokeWidth="1" />
                </svg>
              </button>
              <button
                aria-label="Close window"
                className="sdkwork-knowledgebase-auth-window-button sdkwork-knowledgebase-auth-window-button-danger"
                onClick={handleClose}
                title="Close"
                type="button"
              >
                <X size={14} />
              </button>
            </div>
          </div>
        </header>
      )}
      <main className="sdkwork-knowledgebase-auth-main">{children}</main>
    </div>
  );
}
