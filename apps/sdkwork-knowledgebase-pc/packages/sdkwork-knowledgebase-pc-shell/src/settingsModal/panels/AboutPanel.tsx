import { useMemo, useState } from 'react';
import { BookOpen, Check, Copy, ExternalLink } from 'lucide-react';
import { useTranslation } from 'react-i18next';
import { toast } from '@sdkwork/sdkwork-knowledgebase-pc-knowledgebase';
import type { KnowledgebaseRuntimeConfig } from 'sdkwork-knowledgebase-pc-core';

import {
  SETTINGS_APP_DISPLAY_NAME,
  SETTINGS_APP_VERSION,
  SETTINGS_LINKS,
  SETTINGS_VENDOR_NAME,
} from '../../settingsModalConstants';
import { fieldMatchesSettingsQuery } from '../../settingsPreferences';
import type { DesktopPlatform } from '../../settingsDesktopBridge';
import { resolveNativePlatformLabel } from '../../settingsDesktopCopy';
import { InfoRow, SettingsCard, SettingsEmptyFilterState } from '../settingsModalUi';

export function AboutPanel({
  runtimeConfig,
  deploymentProfile,
  desktopPlatform,
  filterQuery,
  onOpenExternalLink,
}: {
  runtimeConfig?: KnowledgebaseRuntimeConfig;
  deploymentProfile: string;
  desktopPlatform?: DesktopPlatform | null;
  filterQuery: string;
  onOpenExternalLink?: (url: string, title?: string) => void;
}) {
  const { t } = useTranslation('shell');
  const [copied, setCopied] = useState(false);

  const showHero = fieldMatchesSettingsQuery(
    filterQuery,
    'about',
    ['about', 'version', 'legal', '关于', '版本'],
    t,
  );
  const showDiagnostics = fieldMatchesSettingsQuery(
    filterQuery,
    'runtimeDiagnostics',
    ['diagnostics', 'runtime', 'environment', '诊断', '运行'],
    t,
  );
  const showResources = fieldMatchesSettingsQuery(
    filterQuery,
    'resourcesAndLegal',
    ['privacy', 'terms', 'support', 'website', '隐私', '条款', '支持', '官网'],
    t,
  );

  const diagnostics = useMemo(
    () => ({
      app: SETTINGS_APP_DISPLAY_NAME,
      version: SETTINGS_APP_VERSION,
      vendor: SETTINGS_VENDOR_NAME,
      environment: runtimeConfig?.environment ?? 'unknown',
      runtimeTarget: runtimeConfig?.runtimeTarget ?? 'browser',
      nativeHostOs: desktopPlatform ? resolveNativePlatformLabel(desktopPlatform, t) : '—',
      deploymentProfile,
      appKey: runtimeConfig?.appKey ?? 'sdkwork-knowledgebase-pc',
      buildMode: runtimeConfig?.buildMode ?? 'unknown',
      userAgent: typeof navigator !== 'undefined' ? navigator.userAgent : 'unknown',
    }),
    [deploymentProfile, desktopPlatform, runtimeConfig, t],
  );

  const handleCopyDiagnostics = async () => {
    try {
      await navigator.clipboard.writeText(JSON.stringify(diagnostics, null, 2));
      setCopied(true);
      toast.success(t('diagnosticsCopied'));
      window.setTimeout(() => setCopied(false), 2000);
    } catch {
      toast.error(t('diagnosticsCopyFailed'));
    }
  };

  const links = [
    { label: t('officialWebsite'), href: SETTINGS_LINKS.website },
    { label: t('supportCenter'), href: SETTINGS_LINKS.support },
    { label: t('privacyPolicy'), href: SETTINGS_LINKS.privacy },
    { label: t('termsOfService'), href: SETTINGS_LINKS.terms },
  ];

  const filteredLinks = filterQuery
    ? links.filter((link) => link.label.toLowerCase().includes(filterQuery.toLowerCase()))
    : links;

  return (
    <div className="mx-auto max-w-2xl space-y-6">
      {showHero ? (
      <SettingsCard>
        <div className="flex items-start gap-4">
          <div className="flex h-14 w-14 items-center justify-center rounded-2xl bg-zinc-900 text-white shadow-md dark:bg-[var(--color-kb-accent)]">
            <BookOpen size={24} />
          </div>
          <div>
            <div className="text-lg font-extrabold tracking-tight text-zinc-900 dark:text-[var(--color-kb-text-heading)]">
              {SETTINGS_APP_DISPLAY_NAME}
            </div>
            <div className="mt-1 text-sm text-zinc-500 dark:text-[var(--color-kb-text-muted)]">
              {t('aboutTagline')}
            </div>
            <div className="mt-2 inline-flex rounded-full border border-zinc-200 bg-zinc-50 px-2.5 py-0.5 text-[11px] font-bold text-zinc-600 dark:border-[var(--color-kb-panel-border)] dark:bg-[var(--color-kb-panel-hover)] dark:text-[var(--color-kb-text)]">
              {t('appVersion')} {SETTINGS_APP_VERSION}
            </div>
          </div>
        </div>
      </SettingsCard>
      ) : null}

      {showDiagnostics ? (
      <SettingsCard title={t('runtimeDiagnostics')}>
        <div className="mb-4 divide-y divide-zinc-100 dark:divide-[var(--color-kb-panel-border)]">
          <InfoRow label={t('buildEnvironment')} value={runtimeConfig?.environment ?? '—'} />
          <InfoRow label={t('runtimeTarget')} value={runtimeConfig?.runtimeTarget ?? 'browser'} />
          {desktopPlatform ? (
            <InfoRow
              label={t('nativeHostOs')}
              value={resolveNativePlatformLabel(desktopPlatform, t)}
            />
          ) : null}
          <InfoRow label={t('deploymentProfile')} value={deploymentProfile} />
          <InfoRow label={t('appKey')} value={runtimeConfig?.appKey ?? 'sdkwork-knowledgebase-pc'} mono />
        </div>
        <button
          type="button"
          onClick={() => void handleCopyDiagnostics()}
          className="inline-flex items-center gap-2 rounded-xl border border-zinc-200 px-4 py-2 text-xs font-semibold text-zinc-700 transition-colors hover:bg-zinc-50 dark:border-[var(--color-kb-panel-border)] dark:text-[var(--color-kb-text)] dark:hover:bg-[var(--color-kb-panel-hover)]"
        >
          {copied ? <Check size={14} className="text-emerald-500" /> : <Copy size={14} />}
          {copied ? t('diagnosticsCopied') : t('copyDiagnostics')}
        </button>
      </SettingsCard>
      ) : null}

      {showResources && filteredLinks.length > 0 ? (
      <SettingsCard title={t('resourcesAndLegal')}>
        <div className="grid gap-2 sm:grid-cols-2">
          {filteredLinks.map((link) =>
            onOpenExternalLink ? (
              <button
                key={link.href}
                type="button"
                onClick={() => onOpenExternalLink(link.href, link.label)}
                className="flex items-center justify-between rounded-xl border border-zinc-200 px-3 py-2.5 text-left text-sm font-medium text-zinc-700 transition-colors hover:border-zinc-300 hover:bg-zinc-50 dark:border-[var(--color-kb-panel-border)] dark:text-[var(--color-kb-text)] dark:hover:bg-[var(--color-kb-panel-hover)]"
              >
                <span>{link.label}</span>
                <ExternalLink size={14} className="text-zinc-400" />
              </button>
            ) : (
              <a
                key={link.href}
                href={link.href}
                target="_blank"
                rel="noopener noreferrer"
                className="flex items-center justify-between rounded-xl border border-zinc-200 px-3 py-2.5 text-sm font-medium text-zinc-700 transition-colors hover:border-zinc-300 hover:bg-zinc-50 dark:border-[var(--color-kb-panel-border)] dark:text-[var(--color-kb-text)] dark:hover:bg-[var(--color-kb-panel-hover)]"
              >
                <span>{link.label}</span>
                <ExternalLink size={14} className="text-zinc-400" />
              </a>
            ),
          )}
        </div>
      </SettingsCard>
      ) : null}

      {!showHero && !showDiagnostics && !(showResources && filteredLinks.length > 0) && filterQuery ? (
        <SettingsEmptyFilterState />
      ) : null}

      <p className="text-center text-[11px] text-zinc-400 dark:text-[var(--color-kb-text-muted)]">
        {t('copyrightNotice', { vendor: SETTINGS_VENDOR_NAME, year: new Date().getFullYear() })}
      </p>
    </div>
  );
}
