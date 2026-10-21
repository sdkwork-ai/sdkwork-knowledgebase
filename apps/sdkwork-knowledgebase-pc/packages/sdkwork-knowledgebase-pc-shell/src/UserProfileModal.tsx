import { useEffect, useRef, type ReactNode } from 'react';
import { Building2, Globe2, Info, Mail, Network, ShieldCheck, UserRound, X } from 'lucide-react';
import { useTranslation } from 'react-i18next';
import type { KnowledgebaseAccountViewModel } from 'sdkwork-knowledgebase-pc-core';

export interface UserProfileModalProps {
  /**
   * Authenticated account projection derived from the IAM-backed session
   * (`createKnowledgebaseAccountViewModel`). This component never falls back
   * to a demo identity: every field is either the real account value or an
   * explicit empty state.
   */
  account?: KnowledgebaseAccountViewModel;
  isOpen: boolean;
  onClose: () => void;
}

const EMPTY_VALUE = '—';

function AccountDetailRow({
  icon,
  label,
  value,
  emptyLabel,
}: {
  icon: ReactNode;
  label: string;
  value?: string;
  emptyLabel: string;
}) {
  const normalized = value?.trim();

  return (
    <div className="flex items-start gap-3">
      <span
        aria-hidden="true"
        className="mt-0.5 shrink-0 rounded-xl bg-[var(--color-kb-accent)]/10 p-2 text-[var(--color-kb-accent)]"
      >
        {icon}
      </span>
      <div className="min-w-0">
        <dt className="text-[10px] text-zinc-400 dark:text-zinc-500">{label}</dt>
        <dd className="mt-0.5 break-words text-xs font-semibold text-zinc-800 dark:text-zinc-200">
          {normalized ? (
            normalized
          ) : (
            <>
              <span aria-hidden="true" className="text-zinc-400 dark:text-zinc-500">
                {EMPTY_VALUE}
              </span>
              <span className="sr-only">{emptyLabel}</span>
            </>
          )}
        </dd>
      </div>
    </div>
  );
}

/**
 * Read-only account profile.
 *
 * The IAM session is the only authority for identity and session metadata.
 * There is no profile-write service in this codebase (the generated IAM app
 * SDK exposes no profile update), so this view deliberately has no edit form
 * and no local persistence: a localStorage-only "save" would report success
 * for data nothing reads back.
 */
export function UserProfileModal({ account, isOpen, onClose }: UserProfileModalProps) {
  const { t } = useTranslation('shell');
  const dialogRef = useRef<HTMLDivElement>(null);

  useEffect(() => {
    if (isOpen) {
      dialogRef.current?.focus();
    }
  }, [isOpen]);

  useEffect(() => {
    if (!isOpen) {
      return undefined;
    }

    const handleKeyDown = (event: KeyboardEvent) => {
      if (event.key === 'Escape') {
        event.preventDefault();
        onClose();
      }
    };

    window.addEventListener('keydown', handleKeyDown);
    return () => window.removeEventListener('keydown', handleKeyDown);
  }, [isOpen, onClose]);

  if (!isOpen) return null;

  const displayName = account?.displayName?.trim();
  const initials = account?.initials?.trim();
  const avatarUrl = account?.avatarUrl?.trim();
  const notProvided = t('notProvided');

  return (
    <div className="fixed inset-0 z-[100] flex items-center justify-center bg-zinc-950/40 p-4 backdrop-blur-[4px] transition-opacity">
      <div
        ref={dialogRef}
        role="dialog"
        aria-modal="true"
        aria-labelledby="kb-user-profile-title"
        tabIndex={-1}
        data-testid="knowledgebase-user-profile-modal"
        className="flex max-h-[90vh] w-[580px] max-w-full flex-col overflow-hidden rounded-2xl border border-zinc-200 bg-white shadow-2xl transition-all animate-in fade-in zoom-in-95 duration-200 focus:outline-none dark:border-zinc-800 dark:bg-zinc-900"
      >
        {/* Banner with the real account avatar (or its derived initials). */}
        <div className="relative h-[110px] flex-shrink-0 bg-gradient-to-r from-[var(--color-kb-accent)]/85 via-[var(--color-kb-accent)]/65 to-[var(--color-kb-accent)]/45">
          <button
            type="button"
            onClick={onClose}
            aria-label={t('close')}
            title={t('close')}
            className="absolute right-4 top-4 rounded-full bg-black/10 p-1.5 text-white transition-colors hover:bg-black/20 focus-visible:outline focus-visible:outline-2 focus-visible:outline-offset-2 focus-visible:outline-white"
          >
            <X size={16} aria-hidden="true" />
          </button>

          <div className="absolute -bottom-10 left-6 flex items-end">
            <div className="flex h-[84px] w-[84px] shrink-0 items-center justify-center overflow-hidden rounded-2xl border-[3.5px] border-white bg-zinc-100 shadow-lg dark:border-zinc-900 dark:bg-zinc-800">
              {avatarUrl ? (
                <img src={avatarUrl} alt="" className="h-full w-full object-cover" />
              ) : (
                <span aria-hidden="true" className="text-2xl font-bold text-[var(--color-kb-accent)]">
                  {initials || EMPTY_VALUE}
                </span>
              )}
            </div>

            <div className="mb-1 ml-4 min-w-0 pb-1">
              <div className="flex items-center space-x-2">
                <h2
                  id="kb-user-profile-title"
                  className="truncate text-lg font-bold tracking-tight text-zinc-900 dark:text-zinc-50"
                >
                  {displayName || t('notProvided')}
                </h2>
                {account ? (
                  <span className="flex shrink-0 items-center space-x-1 rounded-md border border-[var(--color-kb-accent)]/20 bg-[var(--color-kb-accent)]/10 px-1.5 py-0.5 text-[10px] font-semibold text-[var(--color-kb-accent)]">
                    <ShieldCheck size={11} aria-hidden="true" />
                    <span>{t('signedInAccountBadge')}</span>
                  </span>
                ) : null}
              </div>
              {account?.environmentLabel ? (
                <p className="text-[12px] font-medium text-zinc-500 dark:text-zinc-400">
                  {account.environmentLabel}
                </p>
              ) : null}
            </div>
          </div>
        </div>

        {/* Scrollable account details */}
        <div className="mt-11 flex-1 space-y-4 overflow-y-auto px-6 py-4 scrollbar-thin scrollbar-thumb-zinc-200 dark:scrollbar-thumb-zinc-800">
          {account ? (
            <>
              <div className="flex items-start gap-2.5 rounded-xl border border-[var(--color-kb-accent)]/10 bg-[var(--color-kb-accent)]/5 p-3.5">
                <Info
                  size={15}
                  className="mt-0.5 shrink-0 text-[var(--color-kb-accent)]"
                  aria-hidden="true"
                />
                <p className="text-[11.5px] leading-relaxed text-zinc-600 dark:text-zinc-300">
                  {t('profileManagedByAccountSettings')}
                </p>
              </div>

              <section
                aria-labelledby="kb-user-profile-identity-heading"
                className="space-y-3.5 rounded-2xl border border-zinc-100 bg-white p-4 shadow-sm dark:border-zinc-800/60 dark:bg-zinc-900"
              >
                <h3
                  id="kb-user-profile-identity-heading"
                  className="border-b border-zinc-100 pb-1 text-[11px] font-bold uppercase tracking-widest text-zinc-400 dark:border-zinc-800 dark:text-zinc-500"
                >
                  {t('accountIdentitySection')}
                </h3>
                <dl className="space-y-3.5">
                  <AccountDetailRow
                    icon={<UserRound size={15} />}
                    label={t('displayName')}
                    value={account.displayName}
                    emptyLabel={notProvided}
                  />
                  <AccountDetailRow
                    icon={<Mail size={15} />}
                    label={t('email')}
                    value={account.email}
                    emptyLabel={notProvided}
                  />
                </dl>
              </section>

              <section
                aria-labelledby="kb-user-profile-session-heading"
                className="space-y-3.5 rounded-2xl border border-zinc-100 bg-white p-4 shadow-sm dark:border-zinc-800/60 dark:bg-zinc-900"
              >
                <h3
                  id="kb-user-profile-session-heading"
                  className="border-b border-zinc-100 pb-1 text-[11px] font-bold uppercase tracking-widest text-zinc-400 dark:border-zinc-800 dark:text-zinc-500"
                >
                  {t('accountSessionInfo')}
                </h3>
                <dl className="space-y-3.5">
                  <AccountDetailRow
                    icon={<Building2 size={15} />}
                    label={t('tenant')}
                    value={account.tenantId}
                    emptyLabel={notProvided}
                  />
                  <AccountDetailRow
                    icon={<Network size={15} />}
                    label={t('organization')}
                    value={account.organizationId}
                    emptyLabel={notProvided}
                  />
                  <AccountDetailRow
                    icon={<Globe2 size={15} />}
                    label={t('environment')}
                    value={account.environmentLabel}
                    emptyLabel={notProvided}
                  />
                  <AccountDetailRow
                    icon={<ShieldCheck size={15} />}
                    label={t('authLevel')}
                    value={account.authLevel}
                    emptyLabel={notProvided}
                  />
                </dl>
              </section>
            </>
          ) : (
            <p
              role="status"
              className="rounded-xl border border-zinc-100 bg-zinc-50 p-4 text-xs text-zinc-600 dark:border-zinc-800 dark:bg-zinc-950/40 dark:text-zinc-300"
            >
              {t('accountUnavailable')}
            </p>
          )}
        </div>

        <div className="flex flex-shrink-0 items-center justify-end border-t border-zinc-100 bg-zinc-50/50 px-6 py-4 dark:border-zinc-800 dark:bg-zinc-950/20">
          <button
            type="button"
            onClick={onClose}
            className="rounded-xl border border-zinc-200 px-4 py-2 text-xs font-semibold text-zinc-700 transition-colors hover:bg-zinc-100 dark:border-zinc-700 dark:text-zinc-300 dark:hover:bg-zinc-800/80"
          >
            {t('close')}
          </button>
        </div>
      </div>
    </div>
  );
}
