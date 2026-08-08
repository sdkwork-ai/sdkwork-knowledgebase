import React, { useState, useEffect } from 'react';
import { isBlank, trim } from '@sdkwork/utils';
import { createPortal } from 'react-dom';
import { X, Trash2, Send, History, Check, UserPlus, Loader2 } from 'lucide-react';
import { toast } from './components/ui/toast-manager';
import { useTranslation } from 'react-i18next';
import {
  resolveUserFacingErrorMessage,
  type ErrorTranslateFn,
} from 'sdkwork-knowledgebase-pc-core';

export interface WechatSendPreviewModalProps {
  isOpen: boolean;
  onClose: () => void;
  previewWechatId: string;
  setPreviewWechatId: (id: string) => void;
  onConfirmSend: (recipients: string[]) => Promise<void>;
  isSending?: boolean;
}

export function WechatSendPreviewModal({ isOpen, onClose, previewWechatId, setPreviewWechatId, onConfirmSend, isSending }: WechatSendPreviewModalProps) {
  const { t } = useTranslation(['common', 'officialAccount', 'editor']);
  const { t: tErrors } = useTranslation('errors');
  const [recipients, setRecipients] = useState<string[]>([]);
  const [newIdInput, setNewIdInput] = useState('');
  const [history, setHistory] = useState<string[]>([]);
  const [sendError, setSendError] = useState<string | null>(null);

  useEffect(() => {
    if (isOpen) {
      try {
        const stored = localStorage.getItem('wechat_preview_history');
        if (stored) {
          const parsed = JSON.parse(stored);
          if (Array.isArray(parsed)) {
            setHistory(parsed.filter((item) => typeof item === 'string' && !isBlank(item)));
          }
        }
      } catch (e) {
        console.error('Failed to load wechat history', e);
      }

      if (previewWechatId) {
        const initial = previewWechatId
          .split(/[,，\s]+/)
          .map((segment) => trim(segment))
          .filter((segment) => !isBlank(segment));
        setRecipients(initial);
      } else {
        setRecipients([]);
      }
      setNewIdInput('');
      setSendError(null);
    }
  }, [isOpen, previewWechatId]);

  if (!isOpen) return null;

  const handleAddRecipient = (idStr: string) => {
    const trimmed = trim(idStr);
    if (isBlank(trimmed)) return;
    
    const parts = trimmed.split(/[,，\s]+/).map((segment) => trim(segment)).filter((segment) => !isBlank(segment));
    
    setRecipients(prev => {
      const combined = [...prev];
      let added = false;
      parts.forEach(part => {
        if (!combined.includes(part)) {
          combined.push(part);
          added = true;
        }
      });
      if (added) setNewIdInput('');
      return combined;
    });
  };

  const handleRemoveRecipient = (indexToRemove: number) => {
    setRecipients(prev => prev.filter((_, i) => i !== indexToRemove));
  };

  const handleToggleHistory = (historyId: string) => {
    setRecipients(prev => {
      if (prev.includes(historyId)) {
        return prev.filter(id => id !== historyId);
      } else {
        return [...prev, historyId];
      }
    });
  };

  const handleDeleteHistory = (e: React.MouseEvent, historyId: string) => {
    e.stopPropagation();
    const updated = history.filter(id => id !== historyId);
    setHistory(updated);
    if (recipients.includes(historyId)) {
      setRecipients(prev => prev.filter(id => id !== historyId));
    }
    
    try {
      localStorage.setItem('wechat_preview_history', JSON.stringify(updated));
    } catch (err) {
      console.error(err);
    }
  };

  const handleSend = async () => {
    let currentRecipients = [...recipients];
    if (currentRecipients.length === 0 && !isBlank(newIdInput)) {
      handleAddRecipient(newIdInput); // This updates state asynchronously
      // Best effort parse to use immediately
      currentRecipients = trim(newIdInput).split(/[,，\s]+/).map((segment) => trim(segment)).filter((segment) => !isBlank(segment));
    }

    if (currentRecipients.length === 0) {
      toast.error(t('wechatPreviewRecipientRequired', { ns: 'editor' }));
      return;
    }

    setSendError(null);
    try {
      await onConfirmSend(currentRecipients);
      
      const updatedHistory = [...history];
      currentRecipients.forEach(id => {
        if (!updatedHistory.includes(id)) {
          updatedHistory.unshift(id);
        }
      });
      const finalHistory = updatedHistory.slice(0, 15);
      
      try {
        localStorage.setItem('wechat_preview_history', JSON.stringify(finalHistory));
      } catch (e) {
        console.error(e);
      }

      setPreviewWechatId(currentRecipients.join(', '));
      onClose();
      toast.success(
        t('wechatPreviewSentSuccess', {
          ns: 'editor',
          recipients: currentRecipients.join(', '),
        }),
      );
    } catch (e: unknown) {
      const message = resolveUserFacingErrorMessage(
        e,
        tErrors as unknown as ErrorTranslateFn,
      );
      setSendError(message);
      toast.error(message);
    }
  };

  return createPortal(
    <div className="fixed inset-0 z-[300] bg-zinc-950/60 flex items-center justify-center backdrop-blur-sm" onClick={onClose}>
       <div className="bg-white dark:bg-[var(--color-kb-editor)] w-[480px] flex flex-col rounded-2xl shadow-2xl overflow-hidden border border-zinc-200/50 dark:border-zinc-800/50 animate-in fade-in zoom-in-95 duration-200" onClick={e => e.stopPropagation()}>
          
          <div className="flex justify-between items-center px-6 py-5 bg-[#fafafa] dark:bg-[var(--color-kb-panel)]/50 border-b border-zinc-200/50 dark:border-[var(--color-kb-panel-border)]/80">
            <h3 className="text-[16px] font-bold text-zinc-900 dark:text-[var(--color-kb-text-heading)] flex items-center gap-2">
              <div className="w-8 h-8 rounded-xl bg-[#07c160]/10 flex items-center justify-center">
                <Send size={16} className="text-[#07c160]" />
              </div>
              发送至手机预览
            </h3>
            <button onClick={onClose} className="p-2 text-zinc-400 hover:text-zinc-600 dark:text-[var(--color-kb-text-muted)] dark:hover:text-[var(--color-kb-text-heading)] hover:bg-zinc-100 dark:hover:bg-zinc-800/50 rounded-xl transition-colors">
                <X size={18} />
            </button>
          </div>

          <div className="p-6">
            <div className="mb-6">
              <div className="flex items-center gap-2 mb-2">
                <p className="text-[13px] font-bold text-zinc-700 dark:text-[var(--color-kb-text-heading)]">
                  输入微信号/手机号
                </p>
              </div>
              <div className="flex items-center border border-zinc-200/80 dark:border-[var(--color-kb-panel-border)]/80 rounded-xl bg-white dark:bg-[var(--color-kb-input-bg)] focus-within:ring-2 focus-within:ring-[#07c160]/20 focus-within:border-[#07c160] transition-all p-1">
                <input 
                  type="text" 
                  value={newIdInput} 
                  onChange={e => setNewIdInput(e.target.value)} 
                  onKeyDown={e => {
                    if (e.key === 'Enter') {
                      e.preventDefault();
                      handleAddRecipient(newIdInput);
                    }
                  }}
                  placeholder="如: wx_123456" 
                  className="flex-1 bg-transparent px-3 py-2 text-[14px] text-zinc-900 dark:text-[var(--color-kb-text-heading)] focus:outline-none placeholder:text-zinc-400"
                />
                <button
                  onClick={() => handleAddRecipient(newIdInput)}
                  className={`px-4 py-2 rounded-lg font-bold text-[13px] transition-colors ${
                    !isBlank(newIdInput)
                      ? 'bg-[#07c160] hover:bg-[#06ad56] text-white shadow-sm' 
                      : 'bg-zinc-100 dark:bg-zinc-800 text-zinc-400 dark:text-zinc-500 cursor-not-allowed'
                  }`}
                  disabled={isBlank(newIdInput)}
                >
                  添加
                </button>
              </div>
            </div>

            <div className="mb-6">
              <p className="text-[13px] font-bold text-zinc-700 dark:text-[var(--color-kb-text-heading)] mb-2 flex items-center gap-2">
                已选接收人 
                <span className="text-[11px] font-medium text-zinc-400 bg-zinc-100 dark:bg-zinc-800 px-2 py-0.5 rounded-full">{recipients.length}</span>
              </p>
              
              <div className="min-h-[60px] p-2 border border-zinc-200/50 dark:border-[var(--color-kb-panel-border)]/50 rounded-xl bg-[#fafafa] dark:bg-[var(--color-kb-panel)]/30 flex flex-wrap gap-2 items-start">
                {recipients.length === 0 ? (
                  <div className="w-full h-[40px] flex items-center justify-center text-[12.5px] text-zinc-400">
                    输入微信号并点击添加
                  </div>
                ) : (
                  recipients.map((recipient, index) => (
                    <div 
                      key={recipient + '-' + index}
                      className="group flex items-center gap-1.5 pl-3 pr-1.5 py-1.5 bg-white dark:bg-zinc-800 border border-zinc-200/80 dark:border-zinc-700 rounded-lg text-[13px] text-zinc-800 dark:text-zinc-200 shadow-sm animate-in zoom-in-95 duration-150"
                    >
                      <UserPlus size={14} className="text-[#07c160]" />
                      <span className="font-medium">{recipient}</span>
                      <button 
                        onClick={() => handleRemoveRecipient(index)} 
                        className="text-zinc-400 hover:text-red-500 hover:bg-red-50 dark:hover:bg-red-500/10 rounded-md p-1 transition-colors"
                      >
                        <X size={12} strokeWidth={2.5} />
                      </button>
                    </div>
                  ))
                )}
              </div>
            </div>

            {history.length > 0 && (
              <div>
                <p className="text-[11px] font-bold text-zinc-500 dark:text-[var(--color-kb-text-muted)] mb-2 uppercase tracking-wider flex items-center gap-1.5">
                  <History size={12} />
                  历史记录
                </p>
                <div className="flex flex-wrap gap-2">
                  {history.map((id) => {
                    const isSelected = recipients.includes(id);
                    return (
                      <div 
                        key={'history-' + id}
                        onClick={() => handleToggleHistory(id)}
                        className={`group flex items-center gap-2 pl-3 pr-2 py-1.5 rounded-lg text-[13px] border cursor-pointer transition-all ${
                          isSelected 
                            ? 'bg-[#07c160]/10 border-[#07c160]/30 text-[#07c160]' 
                            : 'bg-white dark:bg-[var(--color-kb-editor)] border-zinc-200/80 dark:border-zinc-700 text-zinc-600 dark:text-zinc-400 hover:border-[#07c160]/50 hover:text-[#07c160]'
                        }`}
                      >
                        {isSelected && <Check size={14} strokeWidth={2.5} />}
                        <span className="font-medium">{id}</span>
                        <div className="w-[1px] h-3 bg-zinc-200 dark:bg-zinc-700 mx-0.5"></div>
                        <button 
                          onClick={(e) => handleDeleteHistory(e, id)}
                          className="text-zinc-400 hover:text-red-500 rounded p-1 opacity-60 group-hover:opacity-100 transition-all hover:bg-red-50 dark:hover:bg-red-500/10"
                        >
                          <Trash2 size={12} />
                        </button>
                      </div>
                    );
                  })}
                </div>
              </div>
            )}

            {sendError && (
              <p role="alert" className="mt-4 text-[12px] text-red-500">
                {sendError}
              </p>
            )}
          </div>

          <div className="flex items-center justify-between px-6 py-4 bg-[#fafafa] dark:bg-[var(--color-kb-panel)]/50 border-t border-zinc-200/50 dark:border-[var(--color-kb-panel-border)]/80">
            <span className="text-[12px] text-zinc-400 font-medium">预览文章无水印限制</span>
             <div className="flex gap-3">
               <button className="px-5 py-2.5 text-[13.5px] font-bold rounded-xl bg-white dark:bg-transparent border border-zinc-200/80 dark:border-zinc-700 text-zinc-600 dark:text-zinc-300 hover:bg-zinc-50 dark:hover:bg-zinc-800 transition-colors" onClick={onClose}>
                 取消
               </button>
               <button 
                 className={`px-6 py-2.5 text-[13.5px] font-extrabold rounded-xl transition-all shadow-sm flex items-center gap-2 ${
                    recipients.length > 0 || !isBlank(newIdInput)
                      ? 'bg-[#07c160] hover:bg-[#06ad56] text-white hover:shadow-md'
                      : 'bg-zinc-100 dark:bg-zinc-800 text-zinc-400 cursor-not-allowed'
                 }`}
                 disabled={(recipients.length === 0 && isBlank(newIdInput)) || isSending} 
                 onClick={handleSend}
               >
                 {isSending ? (
                   <>
                     <Loader2 size={15} className="animate-spin" strokeWidth={2.5} />
                     正在发送...
                   </>
                 ) : (
                   <>
                     <Send size={15} strokeWidth={2.5} />
                     开始发送
                   </>
                 )}
               </button>
             </div>
          </div>

       </div>
    </div>,
    document.body
  );
}
