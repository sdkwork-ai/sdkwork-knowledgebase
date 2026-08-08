import { LogOut, Pencil } from 'lucide-react';
import { useTranslation } from 'react-i18next';
import type { KnowledgebaseAccountViewModel } from 'sdkwork-knowledgebase-pc-core';

import { fieldMatchesSettingsQuery } from '../../settingsPreferences';
import {
  InfoRow,
  SettingsCard,
  SettingsEmptyFilterState,
} from '../settingsModalUi';

export function AccountPanel({
  account,
  deploymentProfile,
  filterQuery,
  onClose,
  onOpenProfile,
  onSignOut,
}: {
  account: KnowledgebaseAccountViewModel;
  deploymentProfile: string;
  filterQuery: string;
  onClose: () => void;
  onOpenProfile?: () => void;
  onSignOut?: () => void | Promise<void>;
}) {
  const { t } = useTranslation('shell');
  const avatar = account.avatarUrl || `https://api.dicebear.com/7.x/notionists/svg?seed=${encodeURIComponent(account.id)}`;

  const showProfile =
    fieldMatchesSettingsQuery(filterQuery, 'account', ['account', 'email', 'avatar', '账号', '邮箱', '头像'], t)
    || fieldMatchesSettingsQuery(filterQuery, 'editProfile', ['profile', 'edit', '资料', '编辑'], t);
  const showSession = fieldMatchesSettingsQuery(
    filterQuery,
    'accountSessionInfo',
    ['session', 'environment', 'deployment', 'tenant', '会话', '环境', '租户'],
    t,
  );
  const showDanger =
    onSignOut
    && fieldMatchesSettingsQuery(
      filterQuery,
      'accountDangerZone',
      ['sign out', 'logout', 'danger', '退出', '危险'],
      t,
    );

  return (
    <div className="mx-auto max-w-2xl space-y-6">
      {showProfile ? (
      <SettingsCard>
        <div className="flex items-center gap-4">
          <div className="h-16 w-16 overflow-hidden rounded-2xl border border-zinc-200 bg-white shadow-sm dark:border-[var(--color-kb-panel-border)] dark:bg-[var(--color-kb-panel)]">
            <img src={avatar} alt="" className="h-full w-full object-cover" />
          </div>
          <div className="min-w-0 flex-1">
            <div className="truncate text-lg font-bold text-zinc-900 dark:text-[var(--color-kb-text-heading)]">
              {account.displayName}
            </div>
            <div className="truncate text-sm text-zinc-500 dark:text-[var(--color-kb-text-muted)]">
              {account.email ?? t('noEmailBound')}
            </div>
            <div className="mt-2 inline-flex items-center rounded-full border border-zinc-200 bg-zinc-50 px-2.5 py-0.5 text-[11px] font-semibold text-zinc-600 dark:border-[var(--color-kb-panel-border)] dark:bg-[var(--color-kb-panel-hover)] dark:text-[var(--color-kb-text)]">
              {account.authLevel}
            </div>
          </div>
          {onOpenProfile ? (
            <button
              type="button"
              onClick={() => {
                onClose();
                onOpenProfile();
              }}
              className="inline-flex shrink-0 items-center gap-1.5 rounded-xl border border-zinc-200 px-3 py-2 text-xs font-semibold text-zinc-700 transition-colors hover:bg-zinc-50 dark:border-[var(--color-kb-panel-border)] dark:text-[var(--color-kb-text)] dark:hover:bg-[var(--color-kb-panel-hover)]"
            >
              <Pencil size={14} />
              {t('editProfile')}
            </button>
          ) : null}
        </div>
      </SettingsCard>
      ) : null}

      {showSession ? (
      <SettingsCard title={t('accountSessionInfo')}>
        <div className="divide-y divide-zinc-100 dark:divide-[var(--color-kb-panel-border)]">
          <InfoRow label={t('displayName')} value={account.displayName} />
          <InfoRow label={t('email')} value={account.email ?? '—'} />
          <InfoRow copyable label={t('tenant')} value={account.tenantId ?? '—'} mono />
          <InfoRow label={t('environment')} value={account.environmentLabel} />
          <InfoRow label={t('deploymentProfile')} value={deploymentProfile} />
        </div>
      </SettingsCard>
      ) : null}

      {showDanger ? (
        <SettingsCard title={t('accountDangerZone')} variant="danger">
          <p className="mb-4 text-xs leading-relaxed text-zinc-500 dark:text-[var(--color-kb-text-muted)]">
            {t('signOutDescription')}
          </p>
          <button
            type="button"
            onClick={() => {
              onClose();
              void onSignOut();
            }}
            className="inline-flex items-center gap-2 rounded-xl border border-rose-200 px-4 py-2.5 text-sm font-semibold text-rose-600 transition-colors hover:bg-rose-50 dark:border-rose-900/40 dark:text-rose-400 dark:hover:bg-rose-950/30"
          >
            <LogOut size={16} />
            {t('signOut')}
          </button>
        </SettingsCard>
      ) : null}

      {!showProfile && !showSession && !showDanger && filterQuery ? (
        <SettingsEmptyFilterState />
      ) : null}
    </div>
  );
}
