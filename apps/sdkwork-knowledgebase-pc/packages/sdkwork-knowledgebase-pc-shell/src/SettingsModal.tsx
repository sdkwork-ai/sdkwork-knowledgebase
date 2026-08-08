import React, { useCallback, useEffect, useMemo, useState } from 'react';
import {
  X,
  Monitor,
  Palette,
  MousePointer2,
  Info,
  UserRound,
  Search,
} from 'lucide-react';
import { useTranslation } from 'react-i18next';
import { useLocalStorage } from '@sdkwork/sdkwork-knowledgebase-pc-commons';
import { toast } from '@sdkwork/sdkwork-knowledgebase-pc-knowledgebase';
import type { KnowledgebaseAccountViewModel, KnowledgebaseRuntimeConfig } from 'sdkwork-knowledgebase-pc-core';

import {
  SETTINGS_APP_VERSION,
  SETTINGS_NAV_ITEMS,
  SETTINGS_STORAGE_KEYS,
  type SettingsTabId,
  type StartupModule,
} from './settingsModalConstants';
import {
  findSettingsTabByQuery,
  isSettingsTabId,
  resetAllAppearancePreferences,
  resetAllGeneralPreferences,
  sanitizeSettingsTabId,
  SETTINGS_DEFAULTS,
  tabMatchesSettingsQuery,
} from './settingsPreferences';
import { readDesktopHostStatus, syncDesktopPreferences, type DesktopPlatform } from './settingsDesktopBridge';
import { useDebouncedValue } from './useDebouncedValue';
import { SidebarSection, TabButton } from './settingsModal/settingsModalUi';
import { AccountPanel } from './settingsModal/panels/AccountPanel';
import { GeneralPanel } from './settingsModal/panels/GeneralPanel';
import { AppearancePanel } from './settingsModal/panels/AppearancePanel';
import { ShortcutsPanel } from './settingsModal/panels/ShortcutsPanel';
import { AboutPanel } from './settingsModal/panels/AboutPanel';

export interface SettingsModalProps {
  account?: KnowledgebaseAccountViewModel;
  deploymentProfile?: KnowledgebaseRuntimeConfig['deploymentProfile'];
  initialTab?: string;
  isOpen: boolean;
  onClose: () => void;
  onOpenProfile?: () => void;
  onSignOut?: () => void | Promise<void>;
  runtimeConfig?: KnowledgebaseRuntimeConfig;
  theme: 'light' | 'dark' | 'system';
  setTheme: (theme: 'light' | 'dark' | 'system') => void;
  onStartupModuleApply?: (module: StartupModule) => void;
  onOpenExternalLink?: (url: string, title?: string) => void;
}

export function SettingsModal({
  account,
  deploymentProfile,
  initialTab,
  isOpen,
  onClose,
  onOpenProfile,
  onSignOut,
  runtimeConfig,
  theme,
  setTheme,
  onStartupModuleApply,
  onOpenExternalLink,
}: SettingsModalProps) {
  const { t } = useTranslation('shell');
  const [activeTab, setActiveTab] = useLocalStorage<SettingsTabId>(
    SETTINGS_STORAGE_KEYS.settingsTab,
    'appearance',
  );
  const [activeColor, setActiveColor] = useLocalStorage(SETTINGS_STORAGE_KEYS.accentColor, '#2563eb');
  const [fontSize, setFontSize] = useLocalStorage<'small' | 'normal' | 'large'>(
    SETTINGS_STORAGE_KEYS.fontSize,
    'normal',
  );
  const [autoStart, setAutoStart] = useLocalStorage(SETTINGS_STORAGE_KEYS.autoStart, false);
  const [hideToTray, setHideToTray] = useLocalStorage(SETTINGS_STORAGE_KEYS.hideToTray, true);
  const [startupModule, setStartupModule] = useLocalStorage<StartupModule>(
    SETTINGS_STORAGE_KEYS.startupModule,
    'kb',
  );
  const [aiPanelOpen, setAiPanelOpen] = useLocalStorage(SETTINGS_STORAGE_KEYS.aiPanelOpen, true);
  const [searchQuery, setSearchQuery] = useState('');
  const [autostartSupported, setAutostartSupported] = useState(true);
  const [desktopPlatform, setDesktopPlatform] = useState<DesktopPlatform | null>(null);
  const [hideToTraySupported, setHideToTraySupported] = useState(true);
  const dialogRef = React.useRef<HTMLDivElement>(null);
  const searchInputRef = React.useRef<HTMLInputElement>(null);

  const isDesktopRuntime = runtimeConfig?.runtimeTarget === 'desktop';

  const resolvedDeploymentProfile =
    deploymentProfile ?? runtimeConfig?.deploymentProfile ?? 'standard';

  const tabMeta = useMemo(
    () => ({
      account: { title: t('account'), description: t('accountDescription') },
      general: { title: t('general'), description: t('generalDescription') },
      appearance: { title: t('appearance'), description: t('appearanceDescription') },
      shortcuts: { title: t('shortcut'), description: t('shortcutsDescription') },
      about: { title: t('about'), description: t('aboutDescription') },
    }),
    [t],
  );

  const handleClose = useCallback(() => {
    setSearchQuery('');
    onClose();
  }, [onClose]);

  const handleSelectTab = useCallback((tab: SettingsTabId) => {
    setSearchQuery('');
    setActiveTab(tab);
  }, [setActiveTab]);

  const normalizedSearch = searchQuery.trim().toLowerCase();
  const debouncedSearch = useDebouncedValue(normalizedSearch, 280);

  const visibleNavItems = useMemo(() => {
    return SETTINGS_NAV_ITEMS.filter((item) => {
      if (item.id === 'account' && !account) {
        return false;
      }
      return tabMatchesSettingsQuery(item.id, normalizedSearch, t);
    });
  }, [account, normalizedSearch, t]);

  const navBySection = useMemo(() => ({
    personal: visibleNavItems.filter((item) => item.section === 'personal'),
    preferences: visibleNavItems.filter((item) => item.section === 'preferences'),
    support: visibleNavItems.filter((item) => item.section === 'support'),
  }), [visibleNavItems]);

  const navIconMap: Record<SettingsTabId, React.ReactNode> = {
    account: <UserRound size={15} />,
    general: <Monitor size={15} />,
    appearance: <Palette size={15} />,
    shortcuts: <MousePointer2 size={15} />,
    about: <Info size={15} />,
  };

  useEffect(() => {
    if (!isOpen) {
      setSearchQuery('');
    }
  }, [isOpen]);

  useEffect(() => {
    if (!isSettingsTabId(activeTab)) {
      setActiveTab(SETTINGS_DEFAULTS.settingsTab);
    }
  }, [activeTab, setActiveTab]);

  useEffect(() => {
    if (isOpen && initialTab && isSettingsTabId(initialTab)) {
      setActiveTab(initialTab);
    }
  }, [initialTab, isOpen, setActiveTab]);

  useEffect(() => {
    if (isOpen && activeTab === 'account' && !account) {
      setActiveTab('appearance');
    }
  }, [account, activeTab, isOpen, setActiveTab]);

  useEffect(() => {
    if (!isOpen) {
      return undefined;
    }

    const previousOverflow = document.body.style.overflow;
    document.body.style.overflow = 'hidden';
    return () => {
      document.body.style.overflow = previousOverflow;
    };
  }, [isOpen]);

  useEffect(() => {
    if (!isOpen) {
      return;
    }

    const frame = window.requestAnimationFrame(() => {
      const activeNav = document.querySelector('[data-settings-nav-active="true"]');
      activeNav?.scrollIntoView({ block: 'nearest' });
    });

    return () => window.cancelAnimationFrame(frame);
  }, [activeTab, isOpen]);

  useEffect(() => {
    if (!isOpen || !debouncedSearch) {
      return;
    }

    const matchedTab = findSettingsTabByQuery(debouncedSearch, t, { hasAccount: Boolean(account) });
    if (matchedTab) {
      setActiveTab(matchedTab);
    }
  }, [account, debouncedSearch, isOpen, setActiveTab, t]);

  useEffect(() => {
    if (!isDesktopRuntime || !isOpen) {
      return undefined;
    }

    let cancelled = false;
    void readDesktopHostStatus().then((status) => {
      if (cancelled || !status) {
        return;
      }

      setDesktopPlatform(status.platform);
      setAutostartSupported(status.supported);
      setHideToTraySupported(status.hideToTraySupported);
      setAutoStart((current) => (status.enabled !== current ? status.enabled : current));
    });

    return () => {
      cancelled = true;
    };
  }, [isDesktopRuntime, isOpen, setAutoStart]);

  const applyDesktopPreference = useCallback(async (
    next: { autoStart: boolean; hideToTray: boolean },
  ) => {
    const result = await syncDesktopPreferences(next);
    if (!result.ok) {
      toast.error(t('desktopPreferenceApplyFailed'));
    }
    return result.ok;
  }, [t]);

  const handleAutoStartChange = useCallback((value: boolean) => {
    const previous = autoStart;
    setAutoStart(value);
    void applyDesktopPreference({ autoStart: value, hideToTray }).then((ok) => {
      if (!ok) {
        setAutoStart(previous);
      }
    });
  }, [applyDesktopPreference, autoStart, hideToTray, setAutoStart]);

  const handleHideToTrayChange = useCallback((value: boolean) => {
    const previous = hideToTray;
    setHideToTray(value);
    void applyDesktopPreference({ autoStart, hideToTray: value }).then((ok) => {
      if (!ok) {
        setHideToTray(previous);
      }
    });
  }, [applyDesktopPreference, autoStart, hideToTray, setHideToTray]);

  useEffect(() => {
    if (!isOpen) {
      return;
    }

    const frame = window.requestAnimationFrame(() => {
      searchInputRef.current?.focus();
    });

    return () => window.cancelAnimationFrame(frame);
  }, [isOpen]);

  useEffect(() => {
    if (!isOpen) {
      return undefined;
    }

    const onKeyDown = (event: KeyboardEvent) => {
      if ((event.ctrlKey || event.metaKey) && event.key === ',') {
        event.preventDefault();
        handleClose();
        return;
      }

      if (event.key === 'Escape') {
        event.preventDefault();
        handleClose();
        return;
      }

      if (event.key === '/') {
        const target = event.target as HTMLElement | null;
        const tagName = target?.tagName?.toLowerCase();
        if (tagName !== 'input' && tagName !== 'textarea' && !target?.isContentEditable) {
          event.preventDefault();
          searchInputRef.current?.focus();
          searchInputRef.current?.select();
        }
        return;
      }

      if (event.key !== 'ArrowDown' && event.key !== 'ArrowUp') {
        return;
      }

      const target = event.target as HTMLElement | null;
      const tagName = target?.tagName?.toLowerCase();
      if (tagName === 'input' || tagName === 'textarea' || target?.isContentEditable) {
        return;
      }

      event.preventDefault();
      const tabIds = visibleNavItems.map((item) => item.id);
      const currentIndex = tabIds.indexOf(sanitizeSettingsTabId(activeTab));
      if (currentIndex === -1) {
        return;
      }

      const nextIndex =
        event.key === 'ArrowDown'
          ? Math.min(currentIndex + 1, tabIds.length - 1)
          : Math.max(currentIndex - 1, 0);

      if (nextIndex !== currentIndex) {
        setSearchQuery('');
        setActiveTab(tabIds[nextIndex]);
      }
    };

    window.addEventListener('keydown', onKeyDown);
    return () => window.removeEventListener('keydown', onKeyDown);
  }, [activeTab, handleClose, isOpen, setActiveTab, visibleNavItems]);

  useEffect(() => {
    if (!isOpen || !dialogRef.current) {
      return undefined;
    }

    const dialog = dialogRef.current;
    const selector = 'button:not([disabled]), input:not([disabled]), a[href], [tabindex]:not([tabindex="-1"])';

    const handleTabKey = (event: KeyboardEvent) => {
      if (event.key !== 'Tab') {
        return;
      }

      const focusable = Array.from(dialog.querySelectorAll(selector)).filter(
        (node): node is HTMLElement =>
          node instanceof HTMLElement && node.offsetParent !== null,
      );
      if (focusable.length === 0) {
        return;
      }

      const first = focusable[0];
      const last = focusable[focusable.length - 1];
      const activeElement = document.activeElement as HTMLElement | null;

      if (event.shiftKey && activeElement === first) {
        event.preventDefault();
        last.focus();
      } else if (!event.shiftKey && activeElement === last) {
        event.preventDefault();
        first.focus();
      }
    };

    dialog.addEventListener('keydown', handleTabKey);
    return () => dialog.removeEventListener('keydown', handleTabKey);
  }, [isOpen]);

  const handleResetAllSettings = useCallback(() => {
    if (!window.confirm(t('resetAllSettingsConfirm'))) {
      return;
    }

    const generalDefaults = resetAllGeneralPreferences();
    const appearanceDefaults = resetAllAppearancePreferences();

    setAutoStart(generalDefaults.autoStart);
    setHideToTray(generalDefaults.hideToTray);
    setStartupModule(generalDefaults.startupModule);
    setAiPanelOpen(generalDefaults.aiPanelOpen);
    setActiveColor(appearanceDefaults.accentColor);
    setFontSize(appearanceDefaults.fontSize);
    setTheme(appearanceDefaults.theme);
    setActiveTab(SETTINGS_DEFAULTS.settingsTab);
    setSearchQuery('');
    onStartupModuleApply?.(generalDefaults.startupModule);
    void syncDesktopPreferences({
      autoStart: generalDefaults.autoStart,
      hideToTray: generalDefaults.hideToTray,
    }).then((result) => {
      if (result.ok) {
        toast.success(t('settingsResetSuccess'));
      } else {
        toast.error(t('desktopPreferenceApplyFailed'));
      }
    });
  }, [
    onStartupModuleApply,
    setActiveColor,
    setActiveTab,
    setAiPanelOpen,
    setAutoStart,
    setFontSize,
    setHideToTray,
    setStartupModule,
    setTheme,
    t,
  ]);

  if (!isOpen) return null;

  const header = tabMeta[activeTab as SettingsTabId] ?? tabMeta.appearance;

  return (
    <div
      className="fixed inset-0 z-[100] flex items-center justify-center bg-zinc-950/45 p-4 backdrop-blur-md"
      onClick={handleClose}
      role="presentation"
    >
      <div
        ref={dialogRef}
        className="flex h-[min(780px,calc(100vh-2rem))] w-[860px] max-w-full overflow-hidden rounded-2xl border border-zinc-200/80 bg-white shadow-[0_24px_70px_-15px_rgba(0,0,0,0.22)] dark:border-[var(--color-kb-panel-border)] dark:bg-[var(--color-kb-editor)] dark:shadow-[0_24px_70px_-15px_rgba(0,0,0,0.45)]"
        onClick={(event) => event.stopPropagation()}
        role="dialog"
        aria-modal="true"
        aria-label={t('systemSettings')}
      >
        <aside className="flex w-[220px] shrink-0 flex-col border-r border-zinc-200/80 bg-[#fafafa] py-5 dark:border-[var(--color-kb-panel-border)] dark:bg-[var(--color-kb-panel)]">
          <div className="px-5 pb-5">
            <div className="text-[10px] font-bold uppercase tracking-widest text-zinc-400 dark:text-[var(--color-kb-text-muted)]">
              {t('settingsCenterLabel')}
            </div>
            <div className="mt-1 text-[15px] font-extrabold tracking-tight text-zinc-900 dark:text-[var(--color-kb-text-heading)]">
              {t('systemSettings')}
            </div>
          </div>

          <div className="px-3 pb-4">
            <div className="relative">
              <Search
                size={14}
                className="pointer-events-none absolute left-3 top-1/2 -translate-y-1/2 text-zinc-400 dark:text-[var(--color-kb-text-muted)]"
              />
              <input
                ref={searchInputRef}
                type="search"
                value={searchQuery}
                onChange={(event) => setSearchQuery(event.target.value)}
                placeholder={t('settingsSearchPlaceholder')}
                aria-label={t('settingsSearchPlaceholder')}
                className="w-full rounded-xl border border-zinc-200 bg-white py-2 pl-9 pr-8 text-xs font-medium text-zinc-700 outline-none transition-colors placeholder:text-zinc-400 focus:border-zinc-400 dark:border-[var(--color-kb-panel-border)] dark:bg-[var(--color-kb-editor)] dark:text-[var(--color-kb-text)] dark:placeholder:text-[var(--color-kb-text-muted)] dark:focus:border-[var(--color-kb-accent)]"
              />
              {searchQuery ? (
                <button
                  type="button"
                  onClick={() => setSearchQuery('')}
                  title={t('settingsClearSearch')}
                  className="absolute right-2 top-1/2 -translate-y-1/2 rounded-md p-1 text-zinc-400 transition-colors hover:bg-black/5 hover:text-zinc-600 dark:hover:bg-[var(--color-kb-panel-hover)] dark:hover:text-[var(--color-kb-text)]"
                >
                  <X size={12} />
                </button>
              ) : null}
            </div>
            {normalizedSearch ? (
              <div className="mt-2 px-1 text-[10px] font-medium text-zinc-400 dark:text-[var(--color-kb-text-muted)]">
                {t('settingsSearchMatchCount', { count: visibleNavItems.length })}
              </div>
            ) : null}
          </div>

          {account ? (
            <button
              type="button"
              onClick={() => handleSelectTab('account')}
              className={
                activeTab === 'account'
                  ? 'mx-3 mb-4 flex items-center gap-2.5 rounded-xl border border-[var(--color-kb-accent)] bg-zinc-100 px-3 py-2.5 text-left shadow-sm transition-colors dark:bg-[var(--color-kb-panel-active)]'
                  : 'mx-3 mb-4 flex items-center gap-2.5 rounded-xl border border-zinc-200/80 bg-white px-3 py-2.5 text-left transition-colors hover:border-zinc-300 hover:bg-zinc-50 dark:border-[var(--color-kb-panel-border)] dark:bg-[var(--color-kb-editor)] dark:hover:bg-[var(--color-kb-panel-hover)]'
              }
            >
              <div className="h-8 w-8 shrink-0 overflow-hidden rounded-lg border border-zinc-200 dark:border-[var(--color-kb-panel-border)]">
                <img
                  src={account.avatarUrl || `https://api.dicebear.com/7.x/notionists/svg?seed=${encodeURIComponent(account.id)}`}
                  alt=""
                  className="h-full w-full object-cover"
                />
              </div>
              <div className="min-w-0 flex-1">
                <div className="truncate text-xs font-bold text-zinc-900 dark:text-[var(--color-kb-text-heading)]">
                  {account.displayName}
                </div>
                <div className="truncate text-[10px] text-zinc-500 dark:text-[var(--color-kb-text-muted)]">
                  {account.email ?? t('noEmailBound')}
                </div>
              </div>
            </button>
          ) : null}

          <nav className="flex-1 space-y-5 overflow-y-auto px-3 search-theme-scrollbar">
            {navBySection.personal.length > 0 ? (
              <SidebarSection title={t('settingsGroupPersonal')}>
                {navBySection.personal.map((item) => (
                  <div key={item.id}>
                    <TabButton
                      active={activeTab === item.id}
                      onClick={() => handleSelectTab(item.id)}
                      icon={navIconMap[item.id]}
                      label={t(item.labelKey)}
                    />
                  </div>
                ))}
              </SidebarSection>
            ) : null}

            {navBySection.preferences.length > 0 ? (
              <SidebarSection title={t('settingsGroupPreferences')}>
                {navBySection.preferences.map((item) => (
                  <div key={item.id}>
                    <TabButton
                      active={activeTab === item.id}
                      onClick={() => handleSelectTab(item.id)}
                      icon={navIconMap[item.id]}
                      label={t(item.labelKey)}
                    />
                  </div>
                ))}
              </SidebarSection>
            ) : null}

            {navBySection.support.length > 0 ? (
              <SidebarSection title={t('settingsGroupSupport')}>
                {navBySection.support.map((item) => (
                  <div key={item.id}>
                    <TabButton
                      active={activeTab === item.id}
                      onClick={() => handleSelectTab(item.id)}
                      icon={navIconMap[item.id]}
                      label={t(item.labelKey)}
                    />
                  </div>
                ))}
              </SidebarSection>
            ) : null}

            {visibleNavItems.length === 0 ? (
              <div className="px-3 py-6 text-center text-xs text-zinc-500 dark:text-[var(--color-kb-text-muted)]">
                {t('settingsSearchEmpty')}
              </div>
            ) : null}
          </nav>

          <div className="space-y-1 px-5 pt-4">
            <span className="text-[10px] font-mono font-bold uppercase tracking-widest text-zinc-400 dark:text-[var(--color-kb-text-muted)]">
              v{SETTINGS_APP_VERSION}
            </span>
            <p className="text-[10px] leading-relaxed text-zinc-400 dark:text-[var(--color-kb-text-muted)]">
              {t('settingsKeyboardHints')}
            </p>
          </div>
        </aside>

        <div className="flex min-w-0 flex-1 flex-col overflow-hidden bg-white dark:bg-[var(--color-kb-editor)]">
          <header className="flex shrink-0 items-start justify-between border-b border-zinc-200/80 px-8 py-5 dark:border-[var(--color-kb-panel-border)] dark:bg-[var(--color-kb-panel)]/30">
            <div className="min-w-0 pr-4">
              <h2 className="text-[18px] font-extrabold tracking-tight text-zinc-900 dark:text-[var(--color-kb-text-heading)]">
                {header.title}
              </h2>
              <p className="mt-1 text-[12px] font-medium text-zinc-500 dark:text-[var(--color-kb-text-muted)]">
                {header.description}
              </p>
            </div>
            <button
              type="button"
              onClick={handleClose}
              title={t('close')}
              className="rounded-xl p-2 text-zinc-400 transition-all hover:bg-black/5 hover:text-zinc-700 dark:text-[var(--color-kb-text-muted)] dark:hover:bg-[var(--color-kb-panel-hover)] dark:hover:text-[var(--color-kb-text-heading)]"
            >
              <X size={18} />
            </button>
          </header>

          <div className="min-h-0 flex-1 overflow-y-auto px-8 py-7 search-theme-scrollbar">
            <div key={activeTab} className="animate-in fade-in duration-200">
            {activeTab === 'account' && account ? (
              <AccountPanel
                account={account}
                deploymentProfile={String(resolvedDeploymentProfile)}
                filterQuery={normalizedSearch}
                onOpenProfile={onOpenProfile}
                onSignOut={onSignOut}
                onClose={handleClose}
              />
            ) : null}

            {activeTab === 'general' ? (
              <GeneralPanel
                aiPanelOpen={aiPanelOpen}
                autoStart={autoStart}
                autostartSupported={autostartSupported}
                desktopPlatform={desktopPlatform}
                filterQuery={normalizedSearch}
                hideToTray={hideToTray}
                hideToTraySupported={hideToTraySupported}
                isDesktopRuntime={isDesktopRuntime}
                onAiPanelOpenChange={setAiPanelOpen}
                onAutoStartChange={handleAutoStartChange}
                onHideToTrayChange={handleHideToTrayChange}
                onResetAllSettings={handleResetAllSettings}
                onStartupModuleChange={(value) => {
                  setStartupModule(value);
                  onStartupModuleApply?.(value);
                }}
                startupModule={startupModule}
              />
            ) : null}

            {activeTab === 'appearance' ? (
              <AppearancePanel
                activeColor={activeColor}
                filterQuery={normalizedSearch}
                fontSize={fontSize}
                onAccentChange={setActiveColor}
                onFontSizeChange={setFontSize}
                onRestoreDefaults={() => {
                  const defaults = resetAllAppearancePreferences();
                  setActiveColor(defaults.accentColor);
                  setFontSize(defaults.fontSize);
                  setTheme(defaults.theme);
                  toast.success(t('restoreAppearanceDefaultsSuccess'));
                }}
                setTheme={setTheme}
                theme={theme}
              />
            ) : null}

            {activeTab === 'shortcuts' ? <ShortcutsPanel filterQuery={normalizedSearch} /> : null}

            {activeTab === 'about' ? (
              <AboutPanel
                deploymentProfile={String(resolvedDeploymentProfile)}
                desktopPlatform={desktopPlatform}
                filterQuery={normalizedSearch}
                onOpenExternalLink={onOpenExternalLink}
                runtimeConfig={runtimeConfig}
              />
            ) : null}
            </div>
          </div>
        </div>
      </div>
    </div>
  );
}
