export const SETTINGS_APP_VERSION = '0.1.0';
export const SETTINGS_APP_DISPLAY_NAME = 'SDKWork Knowledgebase';
export const SETTINGS_VENDOR_NAME = 'SDKWork';

/** Default brand accent — the birdcoder2 DSW DeepSeek blue (deepseek-500).
 * The CSS light/dark rules own this value; only a non-default user pick is
 * applied inline (AppShell), so dark mode can keep its brighter variant. */
export const DEFAULT_ACCENT_COLOR = '#4176e6';

export const SETTINGS_STORAGE_KEYS = {
  settingsTab: 'app-settings-tab',
  themePreference: 'app-theme-preference',
  accentColor: 'app-accent-color',
  fontSize: 'app-font-size',
  autoStart: 'app-auto-start',
  hideToTray: 'app-hide-to-tray',
  startupModule: 'app-startup-module',
  aiPanelOpen: 'app-is-ai-open',
  searchSessions: 'app-search-sessions',
} as const;

export type StartupModule = 'kb' | 'market';

export const SETTINGS_LINKS = {
  website: 'https://sdkwork.com/apps/sdkwork-knowledgebase',
  support: 'https://sdkwork.com/support',
  privacy: 'https://sdkwork.com/privacy',
  terms: 'https://sdkwork.com/terms',
} as const;

export const ACCENT_COLOR_PRESETS = [
  { id: 'wechat', color: '#07c160', labelKey: 'accentPresetWechat' },
  { id: 'blue', color: DEFAULT_ACCENT_COLOR, labelKey: 'accentPresetBlue' },
  { id: 'violet', color: '#8b5cf6', labelKey: 'accentPresetViolet' },
  { id: 'rose', color: '#ef4444', labelKey: 'accentPresetRose' },
] as const;

export type SettingsTabId = 'account' | 'general' | 'appearance' | 'shortcuts' | 'about';

export interface SettingsNavItem {
  id: SettingsTabId;
  labelKey: string;
  section: 'personal' | 'preferences' | 'support';
  keywords: string[];
}

export const SETTINGS_NAV_ITEMS: SettingsNavItem[] = [
  { id: 'account', labelKey: 'account', section: 'personal', keywords: ['account', 'profile', 'sign', 'tenant', 'email', '账号', '登录', '租户'] },
  { id: 'general', labelKey: 'general', section: 'preferences', keywords: ['general', 'language', 'startup', 'desktop', 'workspace', 'data', '常规', '语言', '工作区', '数据'] },
  { id: 'appearance', labelKey: 'appearance', section: 'preferences', keywords: ['appearance', 'theme', 'color', 'font', 'dark', 'light', '外观', '主题', '字号'] },
  { id: 'shortcuts', labelKey: 'shortcut', section: 'preferences', keywords: ['shortcut', 'keyboard', 'hotkey', '快捷键', '键盘'] },
  { id: 'about', labelKey: 'about', section: 'support', keywords: ['about', 'version', 'legal', 'support', '关于', '版本'] },
];
