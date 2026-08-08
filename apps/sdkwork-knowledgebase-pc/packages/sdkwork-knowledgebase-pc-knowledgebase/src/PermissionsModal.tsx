import { useEffect, useState } from 'react';
import { useTranslation } from 'react-i18next';
import {
  DocumentService,
  type KnowledgeDocumentVisibility,
} from './services/document';

export interface PermissionsModalProps {
  isOpen: boolean;
  item: { id: string; title: string; kbId?: string; author?: string } | null;
  onClose: () => void;
  onOpenSpaceSettings?: () => void;
}

const VISIBILITY_OPTIONS: Array<{
  value: KnowledgeDocumentVisibility;
  labelKey: string;
  descriptionKey: string;
}> = [
  { value: 'private', labelKey: 'closedPrivate', descriptionKey: 'visibilityPrivateDesc' },
  { value: 'space', labelKey: 'teamVisibility', descriptionKey: 'visibilitySpaceDesc' },
  { value: 'organization', labelKey: 'organizationVisibility', descriptionKey: 'visibilityOrganizationDesc' },
  { value: 'public', labelKey: 'publicVisible', descriptionKey: 'visibilityPublicDesc' },
];

export function PermissionsModal({
  isOpen,
  item,
  onClose,
  onOpenSpaceSettings,
}: PermissionsModalProps) {
  const { t } = useTranslation(['kb', 'common']);
  const [visibility, setVisibility] = useState<KnowledgeDocumentVisibility>('space');
  const [memberCountLabel, setMemberCountLabel] = useState('0');
  const [loading, setLoading] = useState(false);
  const [saving, setSaving] = useState(false);
  const [error, setError] = useState<string | null>(null);

  useEffect(() => {
    if (!isOpen || !item?.id) {
      return;
    }

    let cancelled = false;
    setLoading(true);
    setError(null);

    Promise.all([
      DocumentService.getDocumentAccess(item.id),
      item.kbId
        ? DocumentService.loadKnowledgeSpaceMembersPage(Number(item.kbId), null, 20)
        : Promise.resolve({ items: [], nextCursor: null, hasMore: false }),
    ])
      .then(([access, membersPage]) => {
        if (cancelled) {
          return;
        }
        setVisibility(access.visibility);
        const count = membersPage.items.length;
        setMemberCountLabel(membersPage.hasMore ? `${count}+` : String(count));
        setLoading(false);
      })
      .catch((loadError) => {
        if (cancelled) {
          return;
        }
        setLoading(false);
        setError(loadError instanceof Error ? loadError.message : 'Failed to load document access.');
      });

    return () => {
      cancelled = true;
    };
  }, [isOpen, item?.id, item?.kbId]);

  if (!isOpen || !item) {
    return null;
  }

  const handleSave = async () => {
    setSaving(true);
    setError(null);
    try {
      await DocumentService.updateDocumentVisibility(item.id, visibility);
      onClose();
    } catch (saveError) {
      setError(saveError instanceof Error ? saveError.message : 'Failed to save document access.');
    } finally {
      setSaving(false);
    }
  };

  return (
    <div className="fixed inset-0 bg-zinc-950/40 z-[1000] flex items-center justify-center backdrop-blur-sm p-4 select-none">
      <div className="bg-white dark:bg-[var(--color-kb-editor)] w-[500px] rounded-2xl shadow-2xl border border-zinc-200/80 dark:border-[var(--color-kb-panel-border)] overflow-hidden animate-in zoom-in-95 duration-200">
        <div className="p-6 border-b border-zinc-200/80 dark:border-[var(--color-kb-panel-border)] bg-[#fafafa] dark:bg-[var(--color-kb-panel)]/30">
          <h3 className="text-[16px] font-extrabold text-zinc-900 dark:text-[var(--color-kb-text-heading)] tracking-tight leading-tight mb-1">
            {t('memberPermissions')}
          </h3>
          <p className="text-[13px] font-medium text-zinc-500 dark:text-[var(--color-kb-text-muted)]">
            {t('setPermissionsFor')} {item.title}
          </p>
        </div>

        <div className="p-6 pb-2">
          {loading ? (
            <div className="flex justify-center py-10">
              <div className="w-7 h-7 border-3 border-indigo-500/30 border-t-indigo-500 rounded-full animate-spin" />
            </div>
          ) : (
            <>
              <div className="border border-zinc-200/80 dark:border-[var(--color-kb-panel-border)] rounded-2xl divide-y divide-zinc-200/80 dark:divide-[var(--color-kb-panel-border)] mb-4 overflow-hidden shadow-sm">
                {VISIBILITY_OPTIONS.map((option) => (
                  <label
                    key={option.value}
                    className="flex justify-between items-center gap-4 p-4 bg-white dark:bg-[var(--color-kb-editor)] hover:bg-zinc-50 dark:hover:bg-[var(--color-kb-panel)] transition-colors cursor-pointer"
                  >
                    <div className="flex flex-col">
                      <span className="text-[14px] text-zinc-900 dark:text-[var(--color-kb-text-heading)] font-bold">
                        {t(option.labelKey, {
                          defaultValue:
                            option.value === 'organization' ? 'Organization' : undefined,
                        })}
                      </span>
                      <span className="text-[11px] text-zinc-500 dark:text-[var(--color-kb-text-muted)] font-medium mt-0.5">
                        {t(option.descriptionKey, {
                          defaultValue:
                            option.value === 'private'
                              ? 'Only space members with access can view this document.'
                              : option.value === 'space'
                                ? 'Visible to members of this knowledge space.'
                                : option.value === 'organization'
                                  ? 'Visible to organization members.'
                                  : 'Anyone with the link can view this document.',
                        })}
                      </span>
                    </div>
                    <input
                      type="radio"
                      name="document-visibility"
                      checked={visibility === option.value}
                      onChange={() => setVisibility(option.value)}
                      className="accent-indigo-500"
                    />
                  </label>
                ))}
              </div>

              <p className="text-[12px] text-zinc-500 dark:text-[var(--color-kb-text-muted)] mb-2">
                {t('spaceMemberCount', {
                  count: memberCountLabel,
                  defaultValue: 'This knowledge space has {{count}} members.',
                })}
              </p>
              {onOpenSpaceSettings ? (
                <button
                  type="button"
                  onClick={onOpenSpaceSettings}
                  className="text-[12px] font-semibold text-indigo-600 dark:text-indigo-400 hover:underline"
                >
                  {t('manageSpaceMembers', { defaultValue: 'Manage members in knowledge base settings' })}
                </button>
              ) : null}
              {error ? <p className="text-sm text-rose-500 mt-3">{error}</p> : null}
            </>
          )}
        </div>

        <div className="flex justify-end gap-3 p-5 pt-0">
          <button
            type="button"
            onClick={onClose}
            className="px-5 py-2.5 text-[13px] font-bold bg-white dark:bg-[var(--color-kb-panel-hover)] border-2 border-zinc-200/80 dark:border-[var(--color-kb-panel-border)] text-zinc-700 dark:text-[var(--color-kb-text-heading)] rounded-xl"
          >
            {t('cancel', { ns: 'common', defaultValue: 'Cancel' })}
          </button>
          <button
            type="button"
            onClick={handleSave}
            disabled={loading || saving}
            className="px-6 py-2.5 text-[13px] font-extrabold bg-[#07C160] hover:bg-[#06ad56] disabled:opacity-60 text-white rounded-xl shadow-md transition-all active:scale-95"
          >
            {saving ? t('saving', { ns: 'common', defaultValue: 'Saving...' }) : t('save', { ns: 'common' })}
          </button>
        </div>
      </div>
    </div>
  );
}
