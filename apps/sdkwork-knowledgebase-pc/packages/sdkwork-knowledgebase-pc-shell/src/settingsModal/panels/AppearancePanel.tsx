import { Sparkles } from 'lucide-react';
import { useTranslation } from 'react-i18next';

import { ACCENT_COLOR_PRESETS } from '../../settingsModalConstants';
import { fieldMatchesSettingsQuery } from '../../settingsPreferences';
import {
  AppearancePreview,
  SegmentedControl,
  SettingsCard,
  SettingsEmptyFilterState,
  SettingsRow,
  ThemeCard,
} from '../settingsModalUi';

export function AppearancePanel({
  theme,
  setTheme,
  activeColor,
  onAccentChange,
  fontSize,
  onFontSizeChange,
  onRestoreDefaults,
  filterQuery,
}: {
  theme: 'light' | 'dark' | 'system';
  setTheme: (theme: 'light' | 'dark' | 'system') => void;
  activeColor: string;
  onAccentChange: (color: string) => void;
  fontSize: 'small' | 'normal' | 'large';
  onFontSizeChange: (size: 'small' | 'normal' | 'large') => void;
  onRestoreDefaults: () => void;
  filterQuery: string;
}) {
  const { t } = useTranslation('shell');
  const activePreset = ACCENT_COLOR_PRESETS.find((preset) => preset.color === activeColor);

  const showPreview = fieldMatchesSettingsQuery(filterQuery, 'appearancePreview', ['preview', '预览'], t)
    || fieldMatchesSettingsQuery(filterQuery, 'theme', ['theme', '主题'], t)
    || fieldMatchesSettingsQuery(filterQuery, 'accentColor', ['accent', 'color', '强调色'], t)
    || fieldMatchesSettingsQuery(filterQuery, 'fontSize', ['font', 'size', '字号'], t);
  const showTheme = fieldMatchesSettingsQuery(filterQuery, 'theme', ['theme', 'dark', 'light', '主题'], t);
  const showAccent = fieldMatchesSettingsQuery(filterQuery, 'accentColor', ['accent', 'color', '强调色'], t);
  const showFont = fieldMatchesSettingsQuery(filterQuery, 'fontSize', ['font', 'size', '字号'], t);
  const showRestore = fieldMatchesSettingsQuery(filterQuery, 'restoreAppearanceDefaults', ['reset', 'default', '恢复'], t);

  return (
    <div className="mx-auto max-w-2xl space-y-6">
      {showPreview ? (
      <SettingsCard title={t('appearancePreview')}>
        <AppearancePreview accentColor={activeColor} fontSize={fontSize} theme={theme} />
        <div className="mt-3 flex items-center gap-2 text-xs text-zinc-500 dark:text-[var(--color-kb-text-muted)]">
          <Sparkles size={14} className="text-[var(--color-kb-accent)]" />
          <span>
            {t('appearancePreviewHint')}
            {activePreset ? ` · ${t(activePreset.labelKey)}` : ''}
          </span>
        </div>
      </SettingsCard>
      ) : null}

      {showTheme ? (
      <SettingsCard title={t('theme')}>
        <div className="grid grid-cols-3 gap-5">
          <ThemeCard active={theme === 'light'} label={t('light')} onClick={() => setTheme('light')} type="light" />
          <ThemeCard active={theme === 'dark'} label={t('dark')} onClick={() => setTheme('dark')} type="dark" />
          <ThemeCard active={theme === 'system'} label={t('system')} onClick={() => setTheme('system')} type="system" />
        </div>
      </SettingsCard>
      ) : null}

      {showAccent ? (
      <SettingsCard title={t('accentColor')}>
        <p className="mb-4 text-xs text-zinc-500 dark:text-[var(--color-kb-text-muted)]">
          {t('accentColorDescription')}
        </p>
        <div className="flex flex-wrap items-center gap-3">
          {ACCENT_COLOR_PRESETS.map((preset) => (
            <button
              key={preset.id}
              type="button"
              title={t(preset.labelKey)}
              onClick={() => onAccentChange(preset.color)}
              className={`h-8 w-8 rounded-full transition-all ${activeColor === preset.color ? 'ring-2 ring-offset-2 ring-offset-white dark:ring-offset-[var(--color-kb-editor)] ring-[var(--color-kb-accent)] scale-110' : 'hover:scale-105'}`}
              style={{ backgroundColor: preset.color }}
            />
          ))}
        </div>
      </SettingsCard>
      ) : null}

      {showFont ? (
      <SettingsCard title={t('fontSize')}>
        <SettingsRow
          description={t('fontSizeDescription')}
          label={t('fontSize')}
          control={
            <SegmentedControl
              options={[
                { value: 'small', label: t('small') },
                { value: 'normal', label: t('normal') },
                { value: 'large', label: t('large') },
              ]}
              value={fontSize}
              onChange={(value) => onFontSizeChange(value as 'small' | 'normal' | 'large')}
            />
          }
        />
      </SettingsCard>
      ) : null}

      {showRestore ? (
      <div className="flex justify-end">
        <button
          type="button"
          onClick={onRestoreDefaults}
          className="rounded-xl border border-zinc-200 px-4 py-2 text-xs font-semibold text-zinc-600 transition-colors hover:bg-zinc-50 dark:border-[var(--color-kb-panel-border)] dark:text-[var(--color-kb-text)] dark:hover:bg-[var(--color-kb-panel-hover)]"
        >
          {t('restoreAppearanceDefaults')}
        </button>
      </div>
      ) : null}

      {!showPreview && !showTheme && !showAccent && !showFont && !showRestore && filterQuery ? (
        <SettingsEmptyFilterState />
      ) : null}
    </div>
  );
}
