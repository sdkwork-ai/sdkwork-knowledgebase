import {
  SETTINGS_NAV_ITEMS,
  SETTINGS_STORAGE_KEYS,
  type SettingsTabId,
  type StartupModule,
} from './settingsModalConstants';

export const SETTINGS_TAB_IDS: SettingsTabId[] = [
  'account',
  'general',
  'appearance',
  'shortcuts',
  'about',
];

export const SETTINGS_DEFAULTS = {
  theme: 'system' as const,
  accentColor: '#2563eb',
  fontSize: 'normal' as const,
  autoStart: false,
  hideToTray: true,
  startupModule: 'kb' as const,
  aiPanelOpen: true,
  settingsTab: 'appearance' as SettingsTabId,
};

export interface SettingsFieldSearchEntry {
  tab: SettingsTabId;
  labelKey: string;
  keywords: string[];
}

export const SETTINGS_FIELD_SEARCH: SettingsFieldSearchEntry[] = [
  { tab: 'account', labelKey: 'account', keywords: ['account', 'email', 'tenant', 'sign out', '账号', '邮箱', '租户', '退出'] },
  { tab: 'account', labelKey: 'editProfile', keywords: ['profile', 'edit', 'avatar', '资料', '编辑', '头像'] },
  { tab: 'account', labelKey: 'accountSessionInfo', keywords: ['session', 'environment', 'deployment', '会话', '环境'] },
  { tab: 'account', labelKey: 'accountDangerZone', keywords: ['sign out', 'logout', 'danger', '退出', '危险'] },
  { tab: 'general', labelKey: 'language', keywords: ['language', 'locale', 'zh', 'en', '语言'] },
  { tab: 'general', labelKey: 'startupModule', keywords: ['startup', 'workspace', 'launch', '启动', '工作区'] },
  { tab: 'general', labelKey: 'aiPanelDefault', keywords: ['ai', 'assistant', 'panel', '助手'] },
  { tab: 'general', labelKey: 'autoStart', keywords: ['autostart', 'boot', '开机', '启动'] },
  { tab: 'general', labelKey: 'hideToTray', keywords: ['tray', 'background', '托盘', '后台'] },
  { tab: 'general', labelKey: 'clearSearchData', keywords: ['clear', 'delete', 'privacy', 'data', '清除', '数据', '隐私'] },
  { tab: 'appearance', labelKey: 'theme', keywords: ['theme', 'dark', 'light', 'system', '主题', '深色', '浅色'] },
  { tab: 'appearance', labelKey: 'accentColor', keywords: ['accent', 'color', '强调色', '颜色'] },
  { tab: 'appearance', labelKey: 'fontSize', keywords: ['font', 'size', '字号', '字体'] },
  { tab: 'shortcuts', labelKey: 'shortcut', keywords: ['shortcut', 'keyboard', 'hotkey', '快捷键'] },
  { tab: 'shortcuts', labelKey: 'shortcutsGlobal', keywords: ['global', 'settings', 'esc', '全局', '设置'] },
  { tab: 'shortcuts', labelKey: 'shortcutsSearch', keywords: ['search', 'message', 'enter', '搜索', '发送'] },
  { tab: 'about', labelKey: 'about', keywords: ['about', 'version', 'legal', '关于', '版本'] },
  { tab: 'about', labelKey: 'runtimeDiagnostics', keywords: ['diagnostics', 'runtime', 'environment', '诊断', '运行'] },
  { tab: 'about', labelKey: 'resourcesAndLegal', keywords: ['privacy', 'terms', 'support', 'website', '隐私', '条款', '支持', '官网'] },
];

export function matchesSettingsToken(query: string, keyword: string): boolean {
  const normalizedQuery = query.trim().toLowerCase();
  const normalizedKeyword = keyword.trim().toLowerCase();
  if (!normalizedQuery || !normalizedKeyword) {
    return false;
  }

  return (
    normalizedKeyword.includes(normalizedQuery)
    || normalizedQuery.includes(normalizedKeyword)
  );
}

export function isSettingsTabId(value: string): value is SettingsTabId {
  return SETTINGS_TAB_IDS.includes(value as SettingsTabId);
}

export function sanitizeSettingsTabId(
  value: string | null | undefined,
  fallback: SettingsTabId = SETTINGS_DEFAULTS.settingsTab,
): SettingsTabId {
  if (!value || !isSettingsTabId(value)) {
    return fallback;
  }
  return value;
}

export function sanitizeStartupModule(
  value: string | null | undefined,
  fallback: StartupModule = SETTINGS_DEFAULTS.startupModule,
): StartupModule {
  if (value === 'market') {
    return 'market';
  }
  if (value === 'kb') {
    return 'kb';
  }
  return fallback;
}

export function tabMatchesSettingsQuery(
  tab: SettingsTabId,
  query: string,
  translate: (key: string) => string,
): boolean {
  const normalized = query.trim().toLowerCase();
  if (!normalized) {
    return true;
  }

  const navItem = SETTINGS_NAV_ITEMS.find((item) => item.id === tab);
  if (navItem) {
    const label = translate(navItem.labelKey).toLowerCase();
    if (label.includes(normalized)) {
      return true;
    }
    if (navItem.keywords.some((keyword) => matchesSettingsToken(normalized, keyword))) {
      return true;
    }
  }

  return SETTINGS_FIELD_SEARCH
    .filter((field) => field.tab === tab)
    .some((field) => fieldMatchesSettingsQuery(query, field.labelKey, field.keywords, translate));
}

export function findSettingsTabByQuery(
  query: string,
  translate: (key: string) => string,
  options?: { hasAccount?: boolean },
): SettingsTabId | null {
  const normalized = query.trim().toLowerCase();
  if (!normalized) {
    return null;
  }

  for (const tab of SETTINGS_TAB_IDS) {
    if (tab === 'account' && options?.hasAccount === false) {
      continue;
    }
    if (tabMatchesSettingsQuery(tab, normalized, translate)) {
      return tab;
    }
  }

  return null;
}

export function findAllSettingsTabsByQuery(
  query: string,
  translate: (key: string) => string,
  options?: { hasAccount?: boolean },
): SettingsTabId[] {
  const normalized = query.trim().toLowerCase();
  if (!normalized) {
    return [];
  }

  const matches = new Set<SettingsTabId>();
  for (const tab of SETTINGS_TAB_IDS) {
    if (tab === 'account' && options?.hasAccount === false) {
      continue;
    }
    if (tabMatchesSettingsQuery(tab, normalized, translate)) {
      matches.add(tab);
    }
  }

  return [...matches];
}

export function fieldMatchesSettingsQuery(
  query: string,
  labelKey: string,
  keywords: string[],
  translate: (key: string) => string,
): boolean {
  const normalized = query.trim().toLowerCase();
  if (!normalized) {
    return true;
  }

  const label = translate(labelKey).toLowerCase();
  return (
    label.includes(normalized)
    || keywords.some((keyword) => matchesSettingsToken(normalized, keyword))
  );
}

export function estimateBrowserStorageUsage(): { itemCount: number; approxBytes: number } {
  if (typeof window === 'undefined') {
    return { itemCount: 0, approxBytes: 0 };
  }

  let approxBytes = 0;
  let itemCount = 0;

  for (let index = 0; index < window.localStorage.length; index += 1) {
    const key = window.localStorage.key(index);
    if (!key) {
      continue;
    }
    const value = window.localStorage.getItem(key) ?? '';
    itemCount += 1;
    approxBytes += key.length + value.length;
  }

  return { itemCount, approxBytes };
}

export function formatStorageSize(bytes: number): string {
  if (bytes < 1024) {
    return `${bytes} B`;
  }
  if (bytes < 1024 * 1024) {
    return `${(bytes / 1024).toFixed(1)} KB`;
  }
  return `${(bytes / (1024 * 1024)).toFixed(2)} MB`;
}

export function clearLocalPreferenceData(): void {
  if (typeof window === 'undefined') {
    return;
  }

  const keysToClear = [
    SETTINGS_STORAGE_KEYS.searchSessions,
    SETTINGS_STORAGE_KEYS.settingsTab,
  ];

  for (const key of keysToClear) {
    try {
      window.localStorage.removeItem(key);
    } catch {
      // Ignore storage cleanup errors.
    }
  }
}

export function resetAllAppearancePreferences(): {
  theme: typeof SETTINGS_DEFAULTS.theme;
  accentColor: string;
  fontSize: typeof SETTINGS_DEFAULTS.fontSize;
} {
  return {
    theme: SETTINGS_DEFAULTS.theme,
    accentColor: SETTINGS_DEFAULTS.accentColor,
    fontSize: SETTINGS_DEFAULTS.fontSize,
  };
}

export function resetAllGeneralPreferences(): {
  autoStart: boolean;
  hideToTray: boolean;
  startupModule: typeof SETTINGS_DEFAULTS.startupModule;
  aiPanelOpen: boolean;
} {
  return {
    autoStart: SETTINGS_DEFAULTS.autoStart,
    hideToTray: SETTINGS_DEFAULTS.hideToTray,
    startupModule: SETTINGS_DEFAULTS.startupModule,
    aiPanelOpen: SETTINGS_DEFAULTS.aiPanelOpen,
  };
}
