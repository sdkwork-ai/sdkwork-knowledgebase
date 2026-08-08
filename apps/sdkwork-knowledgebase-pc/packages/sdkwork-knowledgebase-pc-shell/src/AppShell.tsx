import { useCallback, useEffect, useMemo, useState } from 'react';
import { useTranslation } from 'react-i18next';
import { useNavigate } from 'react-router-dom';
import { KnowledgeBaseApp, ToastContainer, TabCacheService } from '@sdkwork/sdkwork-knowledgebase-pc-knowledgebase';
import { DocumentService } from '@sdkwork/sdkwork-knowledgebase-pc-knowledgebase/services/document';
import { findDocInTree } from '@sdkwork/sdkwork-knowledgebase-pc-knowledgebase/utils/docTreeUtils';
import {
  SearchModule,
  type SearchNavigateToFilePayload,
  type SearchNavigateToKbPayload
} from '@sdkwork/knowledgebase-pc-search';
import {
  InAppBrowserHost,
  dispatchOpenInAppBrowser,
  setKbNavIntent,
  useLocalStorage
} from '@sdkwork/sdkwork-knowledgebase-pc-commons';
import {
  createKnowledgebaseAccountViewModel,
  KNOWLEDGEBASE_POST_AUTH_LANDING_FLAG,
  setKnowledgebaseNetworkOnline,
  signOutKnowledgebaseSession,
  useHydrateKnowledgebaseAccount,
  useKnowledgebaseRuntime,
  useKnowledgebaseSessionSnapshot,
} from 'sdkwork-knowledgebase-pc-core';
import { canAccessKnowledgebaseAdminConsole } from 'sdkwork-knowledgebase-pc-admin-core';
import { SettingsModal } from './SettingsModal';
import { GlobalNav } from './GlobalNav';
import { UserProfileModal, DEFAULT_USER_PROFILE, type UserProfile } from './UserProfileModal';
import { SETTINGS_STORAGE_KEYS, type StartupModule } from './settingsModalConstants';
import { readStoredDesktopPreferences, syncDesktopPreferences } from './settingsDesktopBridge';
import { useDesktopHostIntegration } from './useDesktopHostIntegration';
import { useNetworkAvailability } from './useNetworkAvailability';

const APP_ACTIVE_TAB_STORAGE_KEY = 'app-active-tab';
const APP_SEARCH_VIEW_SESSION_KEY = 'app-search-view-active';

type PersistedAppTab = 'kb' | 'market';
type AppShellTab = PersistedAppTab | 'search';

function readStartupModulePreference(): PersistedAppTab {
  if (typeof window === 'undefined') {
    return 'kb';
  }

  try {
    const item = window.localStorage.getItem(SETTINGS_STORAGE_KEYS.startupModule);
    if (!item) {
      return 'kb';
    }
    const parsed = JSON.parse(item) as StartupModule;
    return parsed === 'market' ? 'market' : 'kb';
  } catch {
    return 'kb';
  }
}

function readPersistedAppTabFromStorage(): PersistedAppTab {
  if (typeof window === 'undefined') {
    return 'kb';
  }

  try {
    const item = window.localStorage.getItem(APP_ACTIVE_TAB_STORAGE_KEY);
    if (!item) {
      return readStartupModulePreference();
    }

    const parsed = JSON.parse(item);
    if (parsed === 'market') {
      return 'market';
    }

    if (parsed === 'search') {
      window.localStorage.setItem(APP_ACTIVE_TAB_STORAGE_KEY, JSON.stringify('kb'));
    }

    return 'kb';
  } catch {
    return 'kb';
  }
}

function readSearchViewActiveFromSession(): boolean {
  if (typeof window === 'undefined') {
    return false;
  }

  try {
    return window.sessionStorage.getItem(APP_SEARCH_VIEW_SESSION_KEY) === '1';
  } catch {
    return false;
  }
}

function useAppShellNavigation() {
  const [persistedTab, setPersistedTabState] = useState<PersistedAppTab>(readPersistedAppTabFromStorage);
  const [searchViewActive, setSearchViewActive] = useState(readSearchViewActiveFromSession);

  const setPersistedTab = useCallback((tab: PersistedAppTab) => {
    setPersistedTabState(tab);
    if (typeof window === 'undefined') {
      return;
    }

    try {
      window.localStorage.setItem(APP_ACTIVE_TAB_STORAGE_KEY, JSON.stringify(tab));
      queueMicrotask(() => {
        window.dispatchEvent(
          new CustomEvent('local-storage', { detail: { key: APP_ACTIVE_TAB_STORAGE_KEY, value: tab } }),
        );
      });
    } catch {
      // Ignore storage errors.
    }
  }, []);

  const resetToKnowledgeBase = useCallback(() => {
    setSearchViewActive(false);
    if (typeof window !== 'undefined') {
      try {
        window.sessionStorage.removeItem(APP_SEARCH_VIEW_SESSION_KEY);
      } catch {
        // Ignore storage errors.
      }
    }
    setPersistedTab('kb');
  }, [setPersistedTab]);

  const setActiveTab = useCallback((tab: AppShellTab) => {
    if (tab === 'search') {
      setSearchViewActive(true);
      if (typeof window !== 'undefined') {
        try {
          window.sessionStorage.setItem(APP_SEARCH_VIEW_SESSION_KEY, '1');
        } catch {
          // Ignore storage errors.
        }
      }
      return;
    }

    setSearchViewActive(false);
    if (typeof window !== 'undefined') {
      try {
        window.sessionStorage.removeItem(APP_SEARCH_VIEW_SESSION_KEY);
      } catch {
        // Ignore storage errors.
      }
    }
    setPersistedTab(tab);
  }, [setPersistedTab]);

  useEffect(() => {
    if (typeof window === 'undefined') {
      return;
    }

    if (window.sessionStorage.getItem(KNOWLEDGEBASE_POST_AUTH_LANDING_FLAG) === '1') {
      window.sessionStorage.removeItem(KNOWLEDGEBASE_POST_AUTH_LANDING_FLAG);
      resetToKnowledgeBase();
    }
  }, [resetToKnowledgeBase]);

  const activeTab: AppShellTab = searchViewActive ? 'search' : persistedTab;

  return { activeTab, setActiveTab, resetToKnowledgeBase };
}

export function AppShell() {
  const { t } = useTranslation('shell');
  const navigate = useNavigate();
  const runtime = useKnowledgebaseRuntime();
  const networkOnline = useNetworkAvailability();
  const sessionSnapshot = useKnowledgebaseSessionSnapshot(runtime.session);
  const canAccessAdminConsole = canAccessKnowledgebaseAdminConsole(
    sessionSnapshot.context?.permissionScope,
  );
  const account = useMemo(
    () => createKnowledgebaseAccountViewModel(sessionSnapshot),
    [sessionSnapshot],
  );

  const [isSettingsOpen, setIsSettingsOpen] = useState(false);
  const [settingsInitialTab, setSettingsInitialTab] = useState('appearance');
  const [isProfileOpen, setIsProfileOpen] = useState(false);
  const [themePreference, setThemePreference] = useLocalStorage<'light' | 'dark' | 'system'>(
    SETTINGS_STORAGE_KEYS.themePreference,
    'system',
  );
  const { activeTab, setActiveTab } = useAppShellNavigation();
  const [activeColor] = useLocalStorage(SETTINGS_STORAGE_KEYS.accentColor, '#2563eb');
  const [fontSize] = useLocalStorage<'small' | 'normal' | 'large'>(
    SETTINGS_STORAGE_KEYS.fontSize,
    'normal',
  );
  const [profile, setProfile] = useLocalStorage<UserProfile>('app-user-profile', DEFAULT_USER_PROFILE);

  useHydrateKnowledgebaseAccount(runtime);

  useEffect(() => {
    setKnowledgebaseNetworkOnline(networkOnline);
  }, [networkOnline]);

  useEffect(() => {
    document.documentElement.style.setProperty('--theme-accent', activeColor);

    if (fontSize === 'small') {
      document.documentElement.style.fontSize = '14px';
    } else if (fontSize === 'large') {
      document.documentElement.style.fontSize = '18px';
    } else {
      document.documentElement.style.fontSize = '16px';
    }
  }, [activeColor, fontSize]);

  useEffect(() => {
    const applyTheme = () => {
      const isDark = themePreference === 'dark'
        || (themePreference === 'system' && window.matchMedia('(prefers-color-scheme: dark)').matches);

      if (isDark) {
        document.documentElement.classList.add('dark');
      } else {
        document.documentElement.classList.remove('dark');
      }
    };

    applyTheme();

    if (themePreference === 'system') {
      const mediaQuery = window.matchMedia('(prefers-color-scheme: dark)');
      mediaQuery.addEventListener('change', applyTheme);
      return () => mediaQuery.removeEventListener('change', applyTheme);
    }
  }, [themePreference]);

  useEffect(() => {
    if (runtime.config.runtimeTarget !== 'desktop') {
      return;
    }

    void syncDesktopPreferences(readStoredDesktopPreferences());
  }, [runtime.config.runtimeTarget]);

  useEffect(() => {
    if (!account.displayName && !account.email) {
      return;
    }

    const shouldSyncName = profile.name === DEFAULT_USER_PROFILE.name;
    const shouldSyncEmail = profile.email === DEFAULT_USER_PROFILE.email;
    if (!shouldSyncName && !shouldSyncEmail) {
      return;
    }

    setProfile({
      ...profile,
      name: shouldSyncName ? account.displayName : profile.name,
      email: shouldSyncEmail ? (account.email ?? profile.email) : profile.email,
    });
  }, [account.displayName, account.email, profile, setProfile]);

  const handleSignOut = useCallback(() => {
    void signOutKnowledgebaseSession(runtime);
  }, [runtime]);

  const openSettings = useCallback((tab = 'appearance') => {
    setSettingsInitialTab(tab);
    setIsSettingsOpen(true);
  }, []);

  useDesktopHostIntegration(runtime.config.runtimeTarget === 'desktop', openSettings);

  useEffect(() => {
    const onKeyDown = (event: KeyboardEvent) => {
      if ((event.ctrlKey || event.metaKey) && event.key === ',') {
        event.preventDefault();
        setIsSettingsOpen((open) => {
          if (open) {
            return false;
          }
          setSettingsInitialTab('appearance');
          return true;
        });
        return;
      }

      const target = event.target as HTMLElement | null;
      const tagName = target?.tagName?.toLowerCase();
      if (tagName === 'input' || tagName === 'textarea' || target?.isContentEditable) {
        return;
      }
    };

    window.addEventListener('keydown', onKeyDown);
    return () => window.removeEventListener('keydown', onKeyDown);
  }, []);

  const activateKnowledgeBase = useCallback((kbId: string, kbTitle?: string) => {
    const prev = localStorage.getItem('app-active-kb');
    let newKb: { id: string; title: string };
    if (prev && prev !== 'null') {
      newKb = { ...JSON.parse(prev), id: kbId, title: kbTitle ?? JSON.parse(prev).title ?? '' };
    } else {
      newKb = { id: kbId, title: kbTitle ?? '' };
    }
    localStorage.setItem('app-active-kb', JSON.stringify(newKb));
    window.dispatchEvent(new CustomEvent('local-storage', { detail: { key: 'app-active-kb', value: newKb } }));
    setActiveTab('kb');
  }, [setActiveTab]);

  const handleGoToKb = useCallback((payload: SearchNavigateToKbPayload) => {
    activateKnowledgeBase(payload.kbId, payload.kbTitle);
  }, [activateKnowledgeBase]);

  const handleGoToFile = useCallback(async (payload: SearchNavigateToFilePayload) => {
    setKbNavIntent({
      kbId: payload.kbId,
      kbTitle: payload.kbTitle,
      docId: payload.docId,
      docTitle: payload.title,
      docType: payload.type,
      parentId: payload.parentId,
      author: payload.author,
      updatedAt: payload.updatedAt,
      highlight: true
    });

    let docMeta = {
      id: payload.docId,
      title: payload.title,
      type: payload.type,
      kbId: payload.kbId,
      author: payload.author ?? account.displayName,
      updatedAt: payload.updatedAt ?? new Date().toISOString(),
      parentId: payload.parentId
    };

    try {
      const tree = await DocumentService.getDocuments(payload.kbId);
      const found = findDocInTree(tree, payload.docId);
      if (found) {
        docMeta = {
          ...docMeta,
          ...found,
          id: found.id,
          title: found.title,
          type: found.type
        };
        setKbNavIntent({
          kbId: payload.kbId,
          kbTitle: payload.kbTitle,
          docId: payload.docId,
          docTitle: found.title,
          docType: found.type,
          parentId: found.parentId ?? payload.parentId,
          author: found.author ?? payload.author,
          updatedAt: found.updatedAt ?? payload.updatedAt,
          highlight: true
        });
      }
    } catch {
      /* use payload snapshot */
    }

    TabCacheService.openDoc(payload.kbId, docMeta);
    activateKnowledgeBase(payload.kbId, payload.kbTitle);
  }, [account.displayName, activateKnowledgeBase]);

  const handleOpenWebLink = useCallback((url: string, title?: string) => {
    dispatchOpenInAppBrowser({ url, title });
  }, []);

  const handleOpenAdminConsole = useCallback(() => {
    navigate('/admin');
  }, [navigate]);

  return (
    <div
      className="flex h-screen w-screen overflow-hidden bg-[var(--color-kb-nav)] text-[var(--color-kb-text)] font-sans"
      data-testid="knowledgebase-pc-app-shell"
    >
      <GlobalNav
        account={account}
        profile={profile}
        activeTab={activeTab}
        onTabChange={(tab) => setActiveTab(tab as 'kb' | 'market' | 'search')}
        onOpenSettings={() => openSettings('appearance')}
        onOpenProfile={() => setIsProfileOpen(true)}
        onOpenAccountSettings={() => openSettings('account')}
        showAdminConsole={canAccessAdminConsole}
        onOpenAdminConsole={handleOpenAdminConsole}
      />
      <div className="flex-1 flex flex-col overflow-hidden relative">
        {!networkOnline ? (
          <div
            className="shrink-0 border-b border-amber-500/30 bg-amber-500/10 px-4 py-2 text-sm text-amber-900 dark:text-amber-100"
            data-testid="knowledgebase-offline-banner"
            role="status"
          >
            {t('offlineBanner')}
          </div>
        ) : null}
        <div className="flex-1 flex overflow-hidden">
        {activeTab === 'search' ? (
          <SearchModule
            onGoToKb={handleGoToKb}
            onGoToFile={handleGoToFile}
            onOpenWebLink={handleOpenWebLink}
          />
        ) : (
          <KnowledgeBaseApp
            activeTab={activeTab}
            onActiveTabChange={setActiveTab as (tab: string) => void}
          />
        )}
        </div>
      </div>

      <InAppBrowserHost />

      <SettingsModal
        account={account}
        deploymentProfile={runtime.config.deploymentProfile}
        initialTab={settingsInitialTab}
        isOpen={isSettingsOpen}
        onClose={() => setIsSettingsOpen(false)}
        onOpenProfile={() => setIsProfileOpen(true)}
        onStartupModuleApply={(module) => setActiveTab(module)}
        onOpenExternalLink={handleOpenWebLink}
        onSignOut={handleSignOut}
        runtimeConfig={runtime.config}
        setTheme={setThemePreference}
        theme={themePreference}
      />

      <UserProfileModal
        account={account}
        isOpen={isProfileOpen}
        onClose={() => setIsProfileOpen(false)}
      />

      <ToastContainer />
    </div>
  );
}
