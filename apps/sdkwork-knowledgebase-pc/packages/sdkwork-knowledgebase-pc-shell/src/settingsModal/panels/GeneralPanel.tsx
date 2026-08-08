import { useMemo } from 'react';
import { Trash2 } from 'lucide-react';
import { useTranslation } from 'react-i18next';
import { toast } from '@sdkwork/sdkwork-knowledgebase-pc-knowledgebase';

import {
  SETTINGS_STORAGE_KEYS,
  type StartupModule,
} from '../../settingsModalConstants';
import type { DesktopPlatform } from '../../settingsDesktopBridge';
import {
  resolveAutoStartDescriptionKey,
  resolveHideToTrayDescriptionKey,
} from '../../settingsDesktopCopy';
import {
  estimateBrowserStorageUsage,
  fieldMatchesSettingsQuery,
  formatStorageSize,
} from '../../settingsPreferences';
import {
  SegmentedControl,
  SettingsCard,
  SettingsEmptyFilterState,
  SettingsRow,
  ToggleSwitch,
} from '../settingsModalUi';

export function GeneralPanel({
  startupModule,
  aiPanelOpen,
  autoStart,
  autostartSupported,
  desktopPlatform,
  hideToTray,
  hideToTraySupported,
  isDesktopRuntime,
  filterQuery,
  onStartupModuleChange,
  onAiPanelOpenChange,
  onAutoStartChange,
  onHideToTrayChange,
  onResetAllSettings,
}: {
  startupModule: StartupModule;
  aiPanelOpen: boolean;
  autoStart: boolean;
  autostartSupported: boolean;
  desktopPlatform: DesktopPlatform | null;
  hideToTray: boolean;
  hideToTraySupported: boolean;
  isDesktopRuntime: boolean;
  filterQuery: string;
  onStartupModuleChange: (value: StartupModule) => void;
  onAiPanelOpenChange: (value: boolean) => void;
  onAutoStartChange: (value: boolean) => void;
  onHideToTrayChange: (value: boolean) => void;
  onResetAllSettings: () => void;
}) {
  const { t, i18n } = useTranslation('shell');
  const storageUsage = useMemo(() => estimateBrowserStorageUsage(), [filterQuery, aiPanelOpen, autoStart, hideToTray, startupModule]);

  const showLanguage = fieldMatchesSettingsQuery(filterQuery, 'language', ['language', 'locale', '语言'], t);
  const showStartupRow = fieldMatchesSettingsQuery(
    filterQuery,
    'startupModule',
    ['startup', 'workspace', '启动', '工作区'],
    t,
  );
  const showAiRow = fieldMatchesSettingsQuery(filterQuery, 'aiPanelDefault', ['ai', 'assistant', '助手'], t);
  const showWorkspace = showStartupRow || showAiRow;
  const showAutoStartRow = fieldMatchesSettingsQuery(filterQuery, 'autoStart', ['autostart', 'boot', '开机'], t);
  const showTrayRow = fieldMatchesSettingsQuery(filterQuery, 'hideToTray', ['tray', '托盘', '后台'], t);
  const showDesktop = showAutoStartRow || showTrayRow;
  const showData =
    fieldMatchesSettingsQuery(filterQuery, 'clearSearchData', ['clear', 'delete', 'privacy', 'data', '清除', '数据', '隐私'], t)
    || fieldMatchesSettingsQuery(filterQuery, 'settingsDataPrivacy', ['storage', 'local', '存储'], t);
  const showReset =
    fieldMatchesSettingsQuery(filterQuery, 'resetAllSettings', ['reset', 'default', '恢复', '重置'], t);

  const handleClearSearchData = () => {
    if (!window.confirm(t('clearSearchDataConfirm'))) {
      return;
    }
    try {
      localStorage.removeItem(SETTINGS_STORAGE_KEYS.searchSessions);
      toast.success(t('clearSearchDataSuccess'));
    } catch {
      toast.error(t('clearSearchDataFailed'));
    }
  };

  const handleLanguageChange = (value: string) => {
    void i18n.changeLanguage(value);
  };

  return (
    <div className="mx-auto max-w-2xl space-y-6">
      {showLanguage ? (
      <SettingsCard title={t('settingsLanguageRegion')}>
        <SettingsRow
          description={t('languageDescription')}
          label={t('language')}
          control={
            <SegmentedControl
              options={[
                { value: 'zh-CN', label: t('zh') },
                { value: 'en-US', label: t('en') },
              ]}
              value={i18n.language.startsWith('zh') ? 'zh-CN' : 'en-US'}
              onChange={handleLanguageChange}
            />
          }
        />
      </SettingsCard>
      ) : null}

      {showWorkspace ? (
      <SettingsCard title={t('settingsWorkspace')}>
        <div className="divide-y divide-zinc-100 dark:divide-[var(--color-kb-panel-border)]">
          {showStartupRow ? (
          <SettingsRow
            description={t('startupModuleDescription')}
            label={t('startupModule')}
            control={
              <SegmentedControl
                options={[
                  { value: 'kb', label: t('myKnowledgeBase') },
                  { value: 'market', label: t('knowledgeBaseMarket') },
                ]}
                value={startupModule}
                onChange={(value) => onStartupModuleChange(value as StartupModule)}
              />
            }
            inline
          />
          ) : null}
          {showAiRow ? (
          <SettingsRow
            description={t('aiPanelDefaultDescription')}
            label={t('aiPanelDefault')}
            control={<ToggleSwitch active={aiPanelOpen} onChange={onAiPanelOpenChange} />}
            inline
          />
          ) : null}
        </div>
      </SettingsCard>
      ) : null}

      {showDesktop ? (
      <SettingsCard title={t('settingsDesktopBehavior')}>
        {!isDesktopRuntime ? (
          <div className="mb-4 rounded-xl border border-amber-200/80 bg-amber-50 px-3 py-2 text-xs font-medium text-amber-800 dark:border-amber-900/40 dark:bg-amber-950/20 dark:text-amber-300">
            {t('desktopOnlyNote')}
          </div>
        ) : null}
        {isDesktopRuntime && !autostartSupported ? (
          <div className="mb-4 rounded-xl border border-zinc-200/80 bg-zinc-50 px-3 py-2 text-xs font-medium text-zinc-600 dark:border-[var(--color-kb-panel-border)] dark:bg-[var(--color-kb-panel-hover)] dark:text-[var(--color-kb-text-muted)]">
            {t('autostartUnsupportedNote')}
          </div>
        ) : null}
        <div className="divide-y divide-zinc-100 dark:divide-[var(--color-kb-panel-border)]">
          {showAutoStartRow ? (
          <SettingsRow
            description={t(resolveAutoStartDescriptionKey(desktopPlatform))}
            label={t('autoStart')}
            control={
              <ToggleSwitch
                active={autoStart}
                disabled={!isDesktopRuntime || !autostartSupported}
                onChange={onAutoStartChange}
              />
            }
            inline
          />
          ) : null}
          {showTrayRow ? (
          <SettingsRow
            description={t(resolveHideToTrayDescriptionKey(desktopPlatform))}
            label={t('hideToTray')}
            control={
              <ToggleSwitch
                active={hideToTray}
                disabled={!isDesktopRuntime || !hideToTraySupported}
                onChange={onHideToTrayChange}
              />
            }
            inline
          />
          ) : null}
        </div>
      </SettingsCard>
      ) : null}

      {showData ? (
      <SettingsCard title={t('settingsDataPrivacy')}>
        <p className="mb-3 text-xs leading-relaxed text-zinc-500 dark:text-[var(--color-kb-text-muted)]">
          {t('settingsDataPrivacyDescription')}
        </p>
        <div className="mb-4 rounded-xl border border-zinc-200 bg-zinc-50 px-3 py-2 text-xs text-zinc-600 dark:border-[var(--color-kb-panel-border)] dark:bg-[var(--color-kb-panel-hover)] dark:text-[var(--color-kb-text-muted)]">
          {t('localStorageUsage', {
            count: storageUsage.itemCount,
            size: formatStorageSize(storageUsage.approxBytes),
          })}
        </div>
        <button
          type="button"
          onClick={handleClearSearchData}
          className="inline-flex items-center gap-2 rounded-xl border border-zinc-200 px-4 py-2.5 text-sm font-semibold text-zinc-700 transition-colors hover:bg-zinc-50 dark:border-[var(--color-kb-panel-border)] dark:text-[var(--color-kb-text)] dark:hover:bg-[var(--color-kb-panel-hover)]"
        >
          <Trash2 size={15} />
          {t('clearSearchData')}
        </button>
      </SettingsCard>
      ) : null}

      {showReset ? (
      <SettingsCard title={t('resetAllSettings')} variant="danger">
        <p className="mb-4 text-xs leading-relaxed text-zinc-500 dark:text-[var(--color-kb-text-muted)]">
          {t('resetAllSettingsDescription')}
        </p>
        <button
          type="button"
          onClick={onResetAllSettings}
          className="inline-flex items-center gap-2 rounded-xl border border-rose-200 px-4 py-2.5 text-sm font-semibold text-rose-600 transition-colors hover:bg-rose-50 dark:border-rose-900/40 dark:text-rose-400 dark:hover:bg-rose-950/30"
        >
          <Trash2 size={15} />
          {t('resetAllSettings')}
        </button>
      </SettingsCard>
      ) : null}

      {!showLanguage && !showWorkspace && !showDesktop && !showData && !showReset && filterQuery ? (
        <SettingsEmptyFilterState />
      ) : null}
    </div>
  );
}
