import React, { useState, useEffect } from 'react';
import { isBlank, trim } from '@sdkwork/utils';
import { X, GitBranch, Share, Loader2, ArrowRight, ShieldCheck, Check } from 'lucide-react';
import { KnowledgeBase, DocumentService } from '../services/document';
import { isKnowledgebaseApiAvailable, KnowledgebaseErrorCodes, resolveUserFacingErrorMessage, shouldUseKnowledgebaseDemoFallback, throwKnowledgebaseError } from 'sdkwork-knowledgebase-pc-core';
import { useTranslation } from 'react-i18next';

export interface GitIntegrationModalProps {
  mode: 'import' | 'sync';
  kb: KnowledgeBase;
  onClose: () => void;
  onSuccess: () => void;
}

export function GitIntegrationModal({ mode, kb, onClose, onSuccess }: GitIntegrationModalProps) {
  const { t } = useTranslation('kb');
  const { t: tErrors } = useTranslation('errors');
  const [repoUrl, setRepoUrl] = useState('');
  const [branch, setBranch] = useState('main');
  const [accessToken, setAccessToken] = useState('');
  const [commitMsg, setCommitMsg] = useState('sync: update knowledge base assets');
  const [loading, setLoading] = useState(false);
  const [success, setSuccess] = useState(false);
  const [errorMessage, setErrorMessage] = useState<string | null>(null);
  const [step, setStep] = useState<number>(0);
  const [progressMsg, setProgressMsg] = useState('');
  const [isUrlValid, setIsUrlValid] = useState(true);

  // Validate URL format roughly
  useEffect(() => {
    if (isBlank(repoUrl)) {
      setIsUrlValid(true);
      return;
    }
    const gitUrlPattern = /^(https:\/\/|git@|http:\/\/)/i;
    setIsUrlValid(gitUrlPattern.test(repoUrl));
  }, [repoUrl]);

  const stepsImport = [
    t('git.stepsImport.0', { defaultValue: '正在解析 Git URL 协议...' }),
    t('git.stepsImport.1', { defaultValue: '建立安全 SSH/HTTPS 网络会话...' }),
    t('git.stepsImport.2', { defaultValue: '拉取远程分支元数据并克隆文件树...' }),
    t('git.stepsImport.3', { defaultValue: '读取仓库并过滤 Markdown/TS/JS/JSON/PDF 开发资源...' }),
    t('git.stepsImport.4', { defaultValue: '组装知识库层级文件夹，并写入后端存储...' }),
    t('git.stepsImport.5', { defaultValue: '全量导入成功，正在重新索引文档树...' })
  ];

  const stepsSync = [
    t('git.stepsSync.0', { defaultValue: '打包当前知识库全部文档为 Git 数据帧...' }),
    t('git.stepsSync.1', { defaultValue: '安全套接字连接远程 Git 服务端并鉴权保护...' }),
    t('git.stepsSync.2', { defaultValue: '本地暂存区创建差异 Commit 信息树...' }),
    t('git.stepsSync.3', { defaultValue: '冲突检测与分支一致性安全检查完毕...' }),
    t('git.stepsSync.4', { defaultValue: '推送内容数据流至 Git 服务器并在远程提交...' }),
    t('git.stepsSync.5', { defaultValue: '代码树同步完成且保持实时跟踪备份状态！' })
  ];

  const activeSteps = mode === 'import' ? stepsImport : stepsSync;

  const handleAction = async (e: React.FormEvent) => {
    e.preventDefault();
    if (isBlank(repoUrl) || !isUrlValid) return;

    setLoading(true);
    setErrorMessage(null);
    setStep(0);
    setProgressMsg(activeSteps[0]);

    const useApiImport = isKnowledgebaseApiAvailable() && mode === 'import';
    const useApiSync = isKnowledgebaseApiAvailable() && mode === 'sync';
    if (!useApiImport && !useApiSync) {
      if (!shouldUseKnowledgebaseDemoFallback()) {
        setErrorMessage(t('git.apiUnavailable', { defaultValue: '知识库 API 不可用，无法执行 Git 操作。' }));
        setLoading(false);
        return;
      }
      for (let i = 0; i < activeSteps.length; i++) {
        setStep(i);
        setProgressMsg(activeSteps[i]);
        await new Promise((resolve) => setTimeout(resolve, 600));
      }
    }

    try {
      if (mode === 'import') {
        const imported = await DocumentService.importGitRepository(kb.id, repoUrl, branch, {
          accessToken: isBlank(accessToken) ? undefined : trim(accessToken),
          onProgress: useApiImport
            ? (progress) => {
                setProgressMsg(progress.message);
                if (progress.totalCount) {
                  setStep(Math.min(activeSteps.length - 1, progress.importedCount ?? 0));
                }
              }
            : undefined,
        });
        if (!imported) {
          throwKnowledgebaseError(KnowledgebaseErrorCodes.INGEST_FAILED);
        }
      } else {
        const synced = await DocumentService.syncGitRepository(kb.id, commitMsg, {
          repoUrl,
          branch,
          accessToken: isBlank(accessToken) ? undefined : trim(accessToken),
          onProgress: useApiSync
            ? (progress) => {
                setProgressMsg(progress.message);
                if (progress.syncedCount) {
                  setStep(Math.min(activeSteps.length - 1, progress.syncedCount));
                }
              }
            : undefined,
        });
        if (!synced.accepted) {
          throwKnowledgebaseError(KnowledgebaseErrorCodes.OPERATION_FAILED);
        }
      }
      setSuccess(true);
      setTimeout(() => {
        onSuccess();
        onClose();
      }, 1500);
    } catch (err) {
      console.error(err);
      setErrorMessage(resolveUserFacingErrorMessage(err, tErrors));
    } finally {
      setLoading(false);
    }
  };

  return (
    <div className="fixed inset-0 z-[300] bg-zinc-950/40 flex items-center justify-center backdrop-blur-md p-4">
      <div className="bg-[var(--color-kb-editor)] w-full max-w-[480px] rounded-2xl shadow-2xl border border-[var(--color-kb-panel-border)] overflow-hidden animate-in fade-in zoom-in-95 duration-200">
        
        {/* Header bar */}
        <div className="flex items-center justify-between px-6 py-4 border-b border-[var(--color-kb-panel-border)] bg-[var(--color-kb-panel-active)]/10">
          <div className="flex items-center gap-2">
            {mode === 'import' ? (
              <div className="p-2 rounded-lg bg-teal-500/10 text-teal-600 dark:text-teal-400">
                <GitBranch size={16} />
              </div>
            ) : (
              <div className="p-2 rounded-lg bg-blue-500/10 text-blue-600 dark:text-blue-400">
                <Share size={16} />
              </div>
            )}
            <div>
              <h3 className="font-bold text-[14.5px] text-[var(--color-kb-text-heading)]">
                {mode === 'import' ? t('git.batchImport', { defaultValue: '从 Git 仓库批量导入' }) : t('git.syncToRemote', { defaultValue: '同步至远程 Git 存储库' })}
              </h3>
              <p className="text-[11px] text-[var(--color-kb-text-muted)]">
                {t('git.bindingTo', { defaultValue: '正在对：' })}<strong className="text-[var(--color-kb-text-heading)] font-semibold">{kb.title}</strong> {t('git.channelBinding', { defaultValue: ' 进行通道绑定' })}
              </p>
            </div>
          </div>
          <button onClick={onClose} className="p-1.5 rounded-lg text-[var(--color-kb-text-muted)] hover:text-[var(--color-kb-text)] hover:bg-[var(--color-kb-panel-hover)] transition-all">
            <X size={16} />
          </button>
        </div>

        {/* Dynamic content form */}
        <form onSubmit={handleAction}>
          <div className="p-6 space-y-4 text-xs">
            
            {loading ? (
              /* Simulation dynamic stage list view */
              <div className="py-8 flex flex-col items-center justify-center space-y-4">
                <div className="relative flex items-center justify-center">
                  <Loader2 size={36} className="text-emerald-500 animate-spin" />
                  <span className="absolute text-[10px] font-bold text-emerald-600 dark:text-emerald-400">{Math.round(((step + 1) / activeSteps.length) * 100)}%</span>
                </div>
                <div className="text-center">
                  <p className="font-bold text-[13px] text-[var(--color-kb-text-heading)] mb-1">{t('git.processingChannel', { defaultValue: '正在处理代码库同步通道...' })}</p>
                  <p className="text-[11px] text-[var(--color-kb-text-muted)] animate-pulse">{progressMsg}</p>
                </div>
                
                {/* Stepper indicator dot-line */}
                <div className="flex items-center justify-center gap-1.5 pt-2 max-w-[280px]">
                  {activeSteps.map((_, idx) => (
                    <div 
                      key={idx} 
                      className={`h-1.5 rounded-full transition-all duration-300 ${idx === step ? 'w-6 bg-emerald-500' : idx < step ? 'w-1.5 bg-emerald-500/50' : 'w-1.5 bg-gray-300 dark:bg-zinc-700'}`} 
                    />
                  ))}
                </div>
              </div>

            ) : success ? (
              /* Success View */
              <div className="py-8 flex flex-col items-center justify-center text-center space-y-3">
                <div className="w-12 h-12 rounded-full bg-emerald-500/15 text-emerald-500 flex items-center justify-center shadow-lg animate-bounce">
                  <Check size={24} strokeWidth={2.5} />
                </div>
                <div>
                  <h4 className="font-bold text-[14px] text-[var(--color-kb-text-heading)]">{t('git.successTitle', { defaultValue: '集成操作成功！' })}</h4>
                  <p className="text-[11px] text-[var(--color-kb-text-muted)] mt-1">
                    {t('git.successDesc', { defaultValue: '当前指令已被成功执行，文件数据同步完毕且刷新状态。' })}
                  </p>
                </div>
              </div>

            ) : (
              /* Normal setup view */
              <>
                <div className="space-y-1.5">
                  <label className="block font-bold text-[11px] text-[var(--color-kb-text-heading)] uppercase tracking-wider">
                    {t('git.repoUrlLabel', { defaultValue: 'Git 仓库远程 URL 地址 *' })}
                  </label>
                  <div className="relative">
                    <input
                      type="text"
                      required
                      placeholder={mode === 'import' ? "https://github.com/developer/my-project.git" : t('git.pushUrlPlaceholder', { defaultValue: '输入远程推送对端 URL' })}
                      value={repoUrl}
                      onChange={(e) => setRepoUrl(e.target.value)}
                      className={`w-full bg-[var(--color-kb-input-bg)] border rounded-xl px-3.5 py-2.5 text-[12.5px] text-[var(--color-kb-text)] focus:outline-none focus:ring-4 ${!isUrlValid ? 'border-red-500/50 focus:ring-red-500/10' : 'border-[var(--color-kb-panel-border)] focus:border-emerald-500 focus:ring-emerald-500/10'} transition-all`}
                    />
                  </div>
                  {!isUrlValid && (
                    <p className="text-[10px] text-red-500 font-medium">{t('git.invalidUrlError', { defaultValue: '请输入合法的 Git URL，如 https://github.com/... 或 ssh/git@ 协议地址。' })}</p>
                  )}
                </div>

                <div className="grid grid-cols-2 gap-4">
                  <div className="space-y-1.5">
                    <label className="block font-bold text-[11px] text-[var(--color-kb-text-heading)] uppercase tracking-wider">
                      {t('git.branchLabel', { defaultValue: '远程分支 (Branch)' })}
                    </label>
                    <input
                      type="text"
                      placeholder="main"
                      value={branch}
                      onChange={(e) => setBranch(e.target.value)}
                      className="w-full bg-[var(--color-kb-input-bg)] border border-[var(--color-kb-panel-border)] rounded-xl px-3.5 py-2.5 text-[12.5px] text-[var(--color-kb-text)] focus:outline-none focus:border-emerald-500 focus:ring-4 focus:ring-emerald-500/10 transition-all"
                    />
                  </div>

                  <div className="space-y-1.5">
                    <label className="block font-bold text-[11px] text-[var(--color-kb-text-heading)] uppercase tracking-wider flex items-center gap-1">
                      <span>{t('git.privateTokenLabel', { defaultValue: '私有库 Token' })}</span>
                      <span className="text-[9px] lowercase bg-zinc-100 dark:bg-zinc-800 px-1 py-0.2 rounded font-normal text-zinc-500 select-none">{t('git.optional', { defaultValue: '选填' })}</span>
                    </label>
                    <input
                      type="password"
                      placeholder="Access Token / Password"
                      value={accessToken}
                      onChange={(e) => setAccessToken(e.target.value)}
                      className="w-full bg-[var(--color-kb-input-bg)] border border-[var(--color-kb-panel-border)] rounded-xl px-3.5 py-2.5 text-[12.5px] text-[var(--color-kb-text)] focus:outline-none focus:border-emerald-500 focus:ring-4 focus:ring-emerald-500/10 transition-all"
                    />
                  </div>
                </div>

                {mode === 'sync' && (
                  <div className="space-y-1.5">
                    <label className="block font-bold text-[11px] text-[var(--color-kb-text-heading)] uppercase tracking-wider">
                      {t('git.commitMsgLabel', { defaultValue: '修改提交内容说明 (Commit Message)' })}
                    </label>
                    <input
                      type="text"
                      required
                      placeholder="Commit Message..."
                      value={commitMsg}
                      onChange={(e) => setCommitMsg(e.target.value)}
                      className="w-full bg-[var(--color-kb-input-bg)] border border-[var(--color-kb-panel-border)] rounded-xl px-3.5 py-2.5 text-[12.5px] text-[var(--color-kb-text)] focus:outline-none focus:border-emerald-500 focus:ring-4 focus:ring-emerald-500/10 transition-all"
                    />
                  </div>
                )}

                {/* Useful notice alert card */}
                <div className="p-3.5 bg-zinc-500/5 rounded-xl border border-[var(--color-kb-panel-border)]/50 space-y-1.5">
                  <div className="flex items-center gap-1.5 text-zinc-700 dark:text-zinc-300 font-semibold text-[11px]">
                    <ShieldCheck size={14} className="text-emerald-500 shrink-0" />
                    <span>{t('git.encryptionGuarantee', { defaultValue: 'Git 端对端传输加密保证' })}</span>
                  </div>
                  <p className="text-[10px] text-[var(--color-kb-text-muted)] leading-relaxed">
                    {t('git.encryptionNotice', { defaultValue: '数据集成由系统通过底层 SSH 沙箱会话拉取，平台采用阅后即焚原则提取 markdown 文本，全程不保留您的 Git 私钥或多端敏感密钥。' })}
                  </p>
                </div>
              </>
            )}
          </div>

          {errorMessage && !loading && !success ? (
            <div className="px-6 pb-2 text-[12px] text-red-600 dark:text-red-400">{errorMessage}</div>
          ) : null}

          {/* Action buttons footer */}
          {!loading && !success && (
            <div className="px-6 py-4 border-t border-[var(--color-kb-panel-border)] bg-[var(--color-kb-panel-active)]/5 flex justify-end gap-2.5">
              <button 
                type="button" 
                onClick={onClose} 
                className="px-4 py-2 text-[12px] font-bold text-[var(--color-kb-text)] hover:bg-[var(--color-kb-panel-hover)] rounded-xl transition-all border border-[var(--color-kb-panel-border)]"
              >
                {t('common:cancel', { defaultValue: '取消' })}
              </button>
              <button 
                type="submit" 
                disabled={isBlank(repoUrl) || !isUrlValid} 
                className="px-4 py-2 text-[12px] font-bold bg-gradient-to-r from-emerald-600 to-teal-600 hover:from-emerald-700 hover:to-teal-700 text-white rounded-xl disabled:opacity-50 disabled:pointer-events-none transition-all shadow-md shadow-emerald-500/10 flex items-center gap-1"
              >
                {mode === 'import' ? t('git.startPullImport', { defaultValue: '开始拉取并导入' }) : t('git.startPushSync', { defaultValue: '建立推送并同步' })} 
                <ArrowRight size={13} />
              </button>
            </div>
          )}
        </form>
      </div>
    </div>
  );
}
