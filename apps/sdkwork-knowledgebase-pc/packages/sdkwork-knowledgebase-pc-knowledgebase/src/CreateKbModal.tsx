import { useState } from 'react';
import { isBlank } from '@sdkwork/utils';
import { X, Plus, GitBranch, FolderGit2, BookPlus, Loader2 } from 'lucide-react';
import { useTranslation } from 'react-i18next';

export interface CreateKbModalProps {
  newKbTitle: string;
  setNewKbTitle: (val: string) => void;
  newKbType: 'team' | 'personal' | 'public';
  setNewKbType: (val: 'team' | 'personal' | 'public') => void;
  newKbIcon: string;
  setNewKbIcon: (val: string) => void;
  onCancel: () => void;
  onCreate: (gitUrl?: string, gitBranch?: string) => void;
}

const predefinedIcons = ['📘', '📗', '📕', '📙', '📓', '📁', '🌟', '🚀', '💡', '🔥', '⚙️', '📊'];

export function CreateKbModal({
  newKbTitle, setNewKbTitle,
  newKbType, setNewKbType,
  newKbIcon, setNewKbIcon,
  onCancel, onCreate
}: CreateKbModalProps) {
  const { t } = useTranslation(['kb', 'common']);
  
  const [creationMode, setCreationMode] = useState<'blank' | 'git'>('blank');
  const [gitUrl, setGitUrl] = useState('');
  const [gitBranch, setGitBranch] = useState('main');
  const [isImporting, setIsImporting] = useState(false);

  const handleCreate = async () => {
    if (creationMode === 'git') {
      setIsImporting(true);
      // Let the parent component handle the git logic
      onCreate(gitUrl, gitBranch);
    } else {
      onCreate();
    }
  };

  return (
    <div className="fixed inset-0 bg-zinc-950/40 z-[100] flex items-center justify-center backdrop-blur-md">
      <div className="bg-white dark:bg-[var(--color-kb-editor)] w-[520px] rounded-2xl shadow-2xl border border-zinc-200/80 dark:border-[var(--color-kb-panel-border)] flex flex-col overflow-hidden animate-in fade-in zoom-in-95 duration-200">
        <div className="flex items-center justify-between px-6 py-5 border-b border-zinc-200/80 dark:border-[var(--color-kb-panel-border)] bg-[#fafafa] dark:bg-[var(--color-kb-panel)]/30 backdrop-blur-sm shadow-sm">
          <h3 className="font-display font-extrabold text-[15px] text-zinc-900 dark:text-[var(--color-kb-text-heading)] tracking-tight">{t('newKb')}</h3>
          <button onClick={onCancel} className="text-zinc-400 hover:text-red-500 hover:bg-red-50 dark:text-[var(--color-kb-text-muted)] dark:hover:text-red-400 dark:hover:bg-red-500/10 transition-all p-1.5 rounded-xl">
            <X size={18} strokeWidth={2.5} />
          </button>
        </div>
        
        <div className="px-6 pt-5 pb-2">
          <div className="flex p-1.5 bg-zinc-100/80 dark:bg-[var(--color-kb-panel-hover)] rounded-xl shadow-inner border border-zinc-200/50 dark:border-transparent">
            <button
              onClick={() => setCreationMode('blank')}
              className={`flex-1 flex justify-center items-center py-2 text-[13px] font-bold rounded-lg transition-all active:scale-[0.98] ${creationMode === 'blank' ? 'bg-white dark:bg-[var(--color-kb-editor)] text-indigo-600 dark:text-[var(--color-kb-accent)] shadow-md shadow-indigo-500/5' : 'text-zinc-500 dark:text-[var(--color-kb-text-muted)] hover:text-zinc-800 dark:hover:text-[var(--color-kb-text)] hover:bg-black/5 dark:hover:bg-white/5'}`}
            >
              <BookPlus size={16} strokeWidth={2.5} className="mr-2" />
              空白知识库
            </button>
            <button
              onClick={() => setCreationMode('git')}
              className={`flex-1 flex justify-center items-center py-2 text-[13px] font-bold rounded-lg transition-all active:scale-[0.98] ${creationMode === 'git' ? 'bg-white dark:bg-[var(--color-kb-editor)] text-indigo-600 dark:text-[var(--color-kb-accent)] shadow-md shadow-indigo-500/5' : 'text-zinc-500 dark:text-[var(--color-kb-text-muted)] hover:text-zinc-800 dark:hover:text-[var(--color-kb-text)] hover:bg-black/5 dark:hover:bg-white/5'}`}
            >
              <FolderGit2 size={16} strokeWidth={2.5} className="mr-2" />
              从 Git 导入
            </button>
          </div>
        </div>

        <div className="px-6 py-4 flex flex-col space-y-5">
          {creationMode === 'git' && (
            <div className="flex flex-col space-y-4">
              <div className="flex flex-col space-y-2">
                <label className="text-[13px] font-semibold text-[var(--color-kb-text-heading)] flex items-center">
                  <GitBranch size={16} className="mr-2" />
                  {t('gitRepoUrl')}
                </label>
                <input 
                  type="text" 
                  value={gitUrl}
                  onChange={e => setGitUrl(e.target.value)}
                  placeholder="https://github.com/owner/repo.git" 
                  className="w-full bg-[var(--color-kb-input-bg)] border border-[var(--color-kb-panel-border)] rounded-xl px-4 py-2.5 text-[14px] text-[var(--color-kb-text)] focus:outline-none focus:ring-2 focus:ring-[var(--color-kb-accent)]/20 focus:border-[var(--color-kb-accent)] transition-all shadow-sm"
                  autoFocus
                />
              </div>
              <div className="flex flex-col space-y-2">
                <label className="text-[13px] font-semibold text-[var(--color-kb-text-heading)]">{t('branch')}</label>
                <input 
                  type="text" 
                  value={gitBranch}
                  onChange={e => setGitBranch(e.target.value)}
                  placeholder="main" 
                  className="w-full bg-[var(--color-kb-input-bg)] border border-[var(--color-kb-panel-border)] rounded-xl px-4 py-2.5 text-[14px] text-[var(--color-kb-text)] focus:outline-none focus:ring-2 focus:ring-[var(--color-kb-accent)]/20 focus:border-[var(--color-kb-accent)] transition-all shadow-sm"
                />
              </div>
            </div>
          )}

          <div className="flex flex-col space-y-2">
            <label className="text-[13px] font-semibold text-[var(--color-kb-text-heading)]">{creationMode === 'git' ? '知识库名称' : t('kbName')}</label>
            <input 
              type="text" 
              value={newKbTitle}
              onChange={e => setNewKbTitle(e.target.value)}
              placeholder={t('kbNamePlaceholder')} 
              className="w-full bg-[var(--color-kb-input-bg)] border border-[var(--color-kb-panel-border)] rounded-xl px-4 py-2.5 text-[14px] text-[var(--color-kb-text)] focus:outline-none focus:ring-2 focus:ring-[var(--color-kb-accent)]/20 focus:border-[var(--color-kb-accent)] transition-all shadow-sm"
              autoFocus={creationMode === 'blank'}
            />
          </div>
          
          <div className="flex flex-col space-y-2">
            <label className="text-[13px] font-semibold text-[var(--color-kb-text-heading)]">{t('kbIconAvatar')}</label>
            <div className="flex items-start gap-4">
              <div className="w-[72px] h-[72px] rounded-xl border border-[var(--color-kb-panel-border)] bg-[var(--color-kb-panel-hover)] flex items-center justify-center shadow-sm flex-shrink-0">
                <span className="text-[32px]">{newKbIcon || '📘'}</span>
              </div>
              
              <div className="flex-1 flex flex-col justify-center">
                <div className="flex flex-wrap gap-2 mb-2">
                   {predefinedIcons.map(icon => (
                       <button
                           key={icon}
                           type="button"
                           onClick={() => setNewKbIcon(icon)}
                           className={`w-9 h-9 flex items-center justify-center text-[18px] rounded-lg border transition-all ${newKbIcon === icon ? 'border-[var(--color-kb-accent)] bg-[var(--color-kb-accent)]/10 shadow-sm' : 'border-[var(--color-kb-panel-border)] bg-[var(--color-kb-panel)] hover:bg-[var(--color-kb-panel-hover)]'}`}
                       >
                           {icon}
                       </button>
                   ))}
                </div>
              </div>
            </div>
          </div>

          <div className="flex flex-col space-y-2">
            <label className="text-[13px] font-semibold text-[var(--color-kb-text-heading)]">{t('kbType')}</label>
            <div className="grid grid-cols-3 gap-2.5">
              <button 
                type="button"
                onClick={() => setNewKbType('team')}
                className={`flex flex-col items-start px-3 py-2.5 rounded-xl border transition-all shadow-sm ${newKbType === 'team' ? 'border-[var(--color-kb-accent)] bg-[var(--color-kb-accent)]/5 text-[var(--color-kb-accent)]' : 'border-[var(--color-kb-panel-border)] bg-[var(--color-kb-panel)] text-[var(--color-kb-text)] hover:bg-[var(--color-kb-panel-hover)] hover:border-[var(--color-kb-text-muted)]/30'}`}
              >
                <span className="font-semibold text-[13px] mb-0.5 tracking-wide">{t('teamKb')}</span>
                <span className={`text-[11px] leading-tight ${newKbType === 'team' ? 'text-[var(--color-kb-accent)]/80' : 'text-[var(--color-kb-text-muted)]'}`}>{t('teamKbDesc')}</span>
              </button>
              <button 
                type="button"
                onClick={() => setNewKbType('personal')}
                className={`flex flex-col items-start px-3 py-2.5 rounded-xl border transition-all shadow-sm ${newKbType === 'personal' ? 'border-[var(--color-kb-accent)] bg-[var(--color-kb-accent)]/5 text-[var(--color-kb-accent)]' : 'border-[var(--color-kb-panel-border)] bg-[var(--color-kb-panel)] text-[var(--color-kb-text)] hover:bg-[var(--color-kb-panel-hover)] hover:border-[var(--color-kb-text-muted)]/30'}`}
              >
                <span className="font-semibold text-[13px] mb-0.5 tracking-wide">{t('personalKb')}</span>
                <span className={`text-[11px] leading-tight ${newKbType === 'personal' ? 'text-[var(--color-kb-accent)]/80' : 'text-[var(--color-kb-text-muted)]'}`}>{t('personalKbDesc')}</span>
              </button>
              <button 
                type="button"
                onClick={() => setNewKbType('public')}
                className={`flex flex-col items-start px-3 py-2.5 rounded-xl border transition-all shadow-sm ${newKbType === 'public' ? 'border-[var(--color-kb-accent)] bg-[var(--color-kb-accent)]/5 text-[var(--color-kb-accent)]' : 'border-[var(--color-kb-panel-border)] bg-[var(--color-kb-panel)] text-[var(--color-kb-text)] hover:bg-[var(--color-kb-panel-hover)] hover:border-[var(--color-kb-text-muted)]/30'}`}
              >
                <span className="font-semibold text-[13px] mb-0.5 tracking-wide">{t('sharedKb')}</span>
                <span className={`text-[11px] leading-tight ${newKbType === 'public' ? 'text-[var(--color-kb-accent)]/80' : 'text-[var(--color-kb-text-muted)]'}`}>{t('sharedKbDesc')}</span>
              </button>
            </div>
          </div>
        </div>
        <div className="flex items-center justify-end px-6 py-4 border-t border-[var(--color-kb-panel-border)] bg-[var(--color-kb-panel)]/50 space-x-3">
          <button 
            onClick={onCancel}
            className="px-5 py-2.5 text-[14px] font-medium text-[var(--color-kb-text-heading)] hover:bg-[var(--color-kb-panel-border)] rounded-xl transition-all"
          >
            {t('cancel', { ns: 'common' })}
          </button>
          <button 
            onClick={handleCreate}
            disabled={isBlank(newKbTitle) || (creationMode === 'git' && isBlank(gitUrl)) || isImporting}
            className="px-6 py-2.5 text-[14px] font-semibold bg-[var(--color-kb-accent)] text-white rounded-xl hover:bg-[var(--color-kb-accent-hover)] transition-all disabled:opacity-50 disabled:cursor-not-allowed shadow-[0_4px_12px_rgba(37,99,235,0.2)] dark:shadow-[0_4px_12px_rgba(59,130,246,0.2)] flex items-center hover:shadow-[0_6px_16px_rgba(37,99,235,0.3)] tracking-wide"
          >
            {isImporting ? (
              <><Loader2 size={15} className="mr-1.5 animate-spin" /> {t('importing')}</>
            ) : creationMode === 'git' ? (
              <><FolderGit2 size={15} className="mr-1.5" /> {t('importAndCreate')}</>
            ) : (
              <><Plus size={15} className="mr-1.5" /> {t('create', { ns: 'common' })}</>
            )}
          </button>
        </div>
      </div>
    </div>
  );
}
