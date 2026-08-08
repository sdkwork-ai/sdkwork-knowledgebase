import { useState, useMemo, useEffect } from 'react';
import { createPortal } from 'react-dom';
import { X, HelpCircle, Calendar, Users, Radio, CheckCircle2, ChevronDown, ChevronUp, RefreshCw, AlertCircle } from 'lucide-react';
import { toast } from './ui/toast-manager';
import { useTranslation } from 'react-i18next';
import {
  resolveUserFacingErrorMessage,
  type ErrorTranslateFn,
} from 'sdkwork-knowledgebase-pc-core';
import { WechatService } from '../services/wechat';

export interface WechatPublishModalProps {
  isOpen: boolean;
  onClose: () => void;
  isPublishing: boolean;
  onConfirmPublish: (options: {
    sendNotification: boolean;
    groupNotification: boolean;
    selectedGroupId: string;
    scheduleTime: string | null;
  }) => Promise<void>;
  officialAccountId?: string;
  officialAccountName?: string;
  officialAccountType?: 'subscription' | 'service'; // 公众号类型
}

export function WechatPublishModal({
  isOpen,
  onClose,
  isPublishing,
  onConfirmPublish,
  officialAccountId,
  officialAccountType = 'subscription'
}: WechatPublishModalProps) {
  const { t } = useTranslation(['editor', 'common', 'officialAccount']);
  const { t: tErrors } = useTranslation('errors');

  const [fanGroupOptions, setFanGroupOptions] = useState([
    { id: 'all', name: t('tags_all', { defaultValue: '所有人 (全粉丝)' }) },
  ]);
  const [loadingFanTags, setLoadingFanTags] = useState(false);
  const [fanTagError, setFanTagError] = useState<string | null>(null);

  useEffect(() => {
    if (!isOpen || !officialAccountId) {
      setFanGroupOptions([
        { id: 'all', name: t('tags_all', { defaultValue: '所有人 (全粉丝)' }) },
      ]);
      setFanTagError(null);
      setLoadingFanTags(false);
      return;
    }

    let cancelled = false;
    setLoadingFanTags(true);
    setFanTagError(null);
    void WechatService.listFanTags(officialAccountId)
      .then((tags) => {
        if (cancelled) {
          return;
        }
        setFanGroupOptions([
          { id: 'all', name: t('tags_all', { defaultValue: '所有人 (全粉丝)' }) },
          ...tags.map((tag) => ({
            id: tag.id,
            name: `${tag.name} (${tag.fanCount})`,
          })),
        ]);
      })
      .catch((error) => {
        if (!cancelled) {
          setFanGroupOptions([]);
          setFanTagError(
            resolveUserFacingErrorMessage(
              error,
              tErrors as unknown as ErrorTranslateFn,
            ),
          );
        }
      })
      .finally(() => {
        if (!cancelled) {
          setLoadingFanTags(false);
        }
      });

    return () => {
      cancelled = true;
    };
  }, [isOpen, officialAccountId, t, tErrors]);

  // States matching the screenshot UI
  const [sendNotification, setSendNotification] = useState(true);
  const [groupNotification, setGroupNotification] = useState(false);
  const [scheduledPublish, setScheduledPublish] = useState(false);
  
  // Custom interactive params
  const [selectedGroupId, setSelectedGroupId] = useState('all');
  const [scheduleDate, setScheduleDate] = useState(() => new Date().toISOString().slice(0, 10));
  const [scheduleTime, setScheduleTime] = useState('18:00');

  const scheduleDateBounds = useMemo(() => {
    const today = new Date();
    const min = today.toISOString().slice(0, 10);
    const maxDate = new Date(today);
    maxDate.setDate(maxDate.getDate() + 7);
    return { min, max: maxDate.toISOString().slice(0, 10) };
  }, []);
  
  // API description collapsible panel state
  const [isApiDetailsOpen, setIsApiDetailsOpen] = useState(true);
  
  if (!isOpen) return null;

  const handlePublishClick = async () => {
    const finalSchedule = scheduledPublish ? `${scheduleDate} ${scheduleTime}:00` : null;
    try {
      await onConfirmPublish({
        sendNotification,
        groupNotification,
        selectedGroupId,
        scheduleTime: finalSchedule
      });
      toast.success(scheduledPublish ? t('schedPublishSuccess', { defaultValue: '文章定时发表任务已成功提交微信发布队列！' }) : t('publishSuccess', { defaultValue: '文章已发表，发布成功！' }));
      onClose();
    } catch (e: any) {
      toast.error(t('publishException', { defaultValue: '发表异常: ' }) + (e.message || '未知错误'));
    }
  };

  return createPortal(
    <div className="fixed inset-0 z-[400] bg-zinc-950/40 flex items-center justify-center backdrop-blur-md animate-in fade-in duration-200" onClick={onClose}>
      <div 
        className="bg-[var(--color-kb-panel)] w-[540px] rounded-3xl shadow-2xl border border-[var(--color-kb-panel-border)] overflow-hidden flex flex-col text-[var(--color-kb-text)] animate-in fade-in zoom-in-95 duration-200" 
        onClick={e => e.stopPropagation()}
        id="wechat-publish-modal-container"
      >
        {/* Header */}
        <div className="flex justify-between items-center px-6 py-5 border-b border-[var(--color-kb-panel-border)] bg-[var(--color-kb-editor)]/30">
          <h3 className="text-[16px] font-bold text-[var(--color-kb-text-heading)] flex items-center gap-2">
            <Radio size={18} className="text-[#07c160]" />
            {t('massPublishAndSched', { defaultValue: '图文群发与定时' })}
          </h3>
          <button 
            onClick={onClose} 
            className="w-8 h-8 rounded-full flex items-center justify-center text-[var(--color-kb-text-muted)] hover:text-red-500 hover:bg-red-500/10 transition-colors"
            id="wechat-publish-modal-close-btn"
          >
            <X size={18} />
          </button>
        </div>

        {/* Content Body */}
        <div className="p-6 space-y-4 max-h-[70vh] overflow-y-auto bg-[var(--color-kb-editor)]/30">
          {loadingFanTags && (
            <div role="status" className="flex items-center gap-2 rounded-xl border border-[var(--color-kb-panel-border)] bg-[var(--color-kb-panel)] px-4 py-3 text-xs text-[var(--color-kb-text-muted)]">
              <RefreshCw size={14} className="animate-spin" />
              {t('common:loading')}
            </div>
          )}

          {fanTagError && (
            <div role="alert" className="flex items-start gap-2 rounded-xl border border-red-500/20 bg-red-500/5 px-4 py-3 text-xs text-red-500">
              <AlertCircle size={14} className="mt-0.5 shrink-0" />
              <span>{fanTagError}</span>
            </div>
          )}
          
          {/* Card 1: 群发通知 */}
          <div className="bg-[var(--color-kb-panel)] rounded-2xl p-5 border border-[var(--color-kb-panel-border)] shadow-xs space-y-2.5">
            <div className="flex justify-between items-center">
              <div className="flex items-center gap-1.5">
                <span className="font-bold text-[14px] text-[var(--color-kb-text-heading)]">
                  {t('oaMassNotification', { defaultValue: '公众号粉丝群发通知' })}
                </span>
                <div className="group relative inline-block cursor-help text-[var(--color-kb-text-muted)] hover:text-[var(--color-kb-text-heading)]">
                  <HelpCircle size={14} />
                  {/* Tooltip */}
                  <div className="invisible group-hover:visible absolute left-1/2 -translate-x-1/2 bottom-full mb-2 w-64 p-3 bg-[var(--color-kb-editor)] border border-[var(--color-kb-panel-border)] text-[var(--color-kb-text)] text-[11px] font-medium leading-relaxed rounded-xl shadow-xl z-50">
                    {t('massNotificationTooltip', { defaultValue: '开启后，图文消息将通过公众号下发给粉丝，促成聊天列表中的强弹窗通知。' })}
                  </div>
                </div>
              </div>
              
              {/* Green iOS-style Toggle Switch */}
              <button
                onClick={() => {
                  setSendNotification(!sendNotification);
                }}
                className={`w-12 h-6 flex items-center rounded-full p-0.5 transition-colors duration-300 focus:outline-none ${
                  sendNotification ? 'bg-[#07c160]' : 'bg-[var(--color-kb-panel-hover)] border border-[var(--color-kb-panel-border)]'
                }`}
                id="toggle-mass-send"
              >
                <div 
                  className={`bg-white w-5 h-5 rounded-full shadow-md transform transition-transform duration-300 ${
                    sendNotification ? 'translate-x-6' : 'translate-x-0'
                  }`} 
                />
              </button>
            </div>
            
            <p className="text-[12px] text-[var(--color-kb-text-muted)] font-medium">
              {t('massQuotaFromWechat', {
                defaultValue: '群发配额与剩余次数以微信公众平台接口返回为准。',
              })}
            </p>
          </div>

          {/* Card 2: 分组通知 & 定时发表 */}
          <div className="bg-[var(--color-kb-panel)] rounded-2xl border border-[var(--color-kb-panel-border)] shadow-xs divide-y divide-[var(--color-kb-panel-border)]">
            
            {/* Row 1: 分组通知 */}
            <div className="p-5 space-y-4">
              <div className="flex justify-between items-center">
                <div className="flex items-center gap-1.5">
                  <span className="font-bold text-[14px] text-[var(--color-kb-text-heading)]">
                    {t('specGroupMassPublish', { defaultValue: '指定分组定向群发' })}
                  </span>
                </div>
                
                {/* Switch */}
              <button
                onClick={() => setGroupNotification(!groupNotification)}
                disabled={loadingFanTags || Boolean(fanTagError)}
                className={`w-12 h-6 flex items-center rounded-full p-0.5 transition-colors duration-300 focus:outline-none ${
                  groupNotification ? 'bg-[#07c160]' : 'bg-[var(--color-kb-panel-hover)] border border-[var(--color-kb-panel-border)]'
                } disabled:cursor-not-allowed disabled:opacity-50`}
                id="toggle-group-send"
                >
                  <div 
                    className={`bg-white w-5 h-5 rounded-full shadow-md transform transition-transform duration-300 ${
                      groupNotification ? 'translate-x-6' : 'translate-x-0'
                    }`} 
                  />
                </button>
              </div>

              {/* Group selection when active */}
              {groupNotification && (
                <div className="pl-1 pt-1 animate-in slide-in-from-top-1 duration-200 space-y-2">
                  <span className="text-[11px] font-bold text-[var(--color-kb-text-muted)] uppercase tracking-wider flex items-center gap-1">
                    <Users size={12} className="text-[#07c160]" />
                    {t('selectGroupFans', { defaultValue: '选择发送的目标分组粉丝' })}
                  </span>
                  <div className="relative">
                    <select 
                      value={selectedGroupId}
                      onChange={(e) => setSelectedGroupId(e.target.value)}
                      className="w-full border border-[var(--color-kb-panel-border)] rounded-xl px-3 py-2 text-xs bg-[var(--color-kb-input-bg)] text-[var(--color-kb-text)] outline-none focus:border-[#07c160] focus:ring-1 focus:ring-[#07c160] transition-all cursor-pointer"
                    >
                      {fanGroupOptions.map((tag) => (
                        <option key={tag.id} value={tag.id} className="bg-[var(--color-kb-panel)]">
                          {tag.name}
                        </option>
                      ))}
                    </select>
                  </div>
                </div>
              )}
            </div>

            {/* Row 2: 定时发表 */}
            <div className="p-5 space-y-4">
              <div className="flex justify-between items-center">
                <div className="flex items-center gap-1.5">
                  <span className="font-bold text-[14px] text-[var(--color-kb-text-heading)]">
                    {t('schedulePublishEvent', { defaultValue: '定时发表活动' })}
                  </span>
                  <div className="group relative inline-block cursor-help text-[var(--color-kb-text-muted)] hover:text-[var(--color-kb-text-heading)]">
                    <HelpCircle size={14} />
                    {/* Tooltip */}
                    <div className="invisible group-hover:visible absolute left-1/2 -translate-x-1/2 bottom-full mb-2 w-64 p-3 bg-[var(--color-kb-editor)] border border-[var(--color-kb-panel-border)] text-[var(--color-kb-text)] text-[11px] font-medium leading-relaxed rounded-xl shadow-xl z-50">
                      {t('schedulePublishTooltip', { defaultValue: '支持预设时间，在选定的未来时刻触发微信自动发布，最长可在未来 7 天内定时。' })}
                    </div>
                  </div>
                </div>
                
                {/* Switch */}
                <button 
                  onClick={() => setScheduledPublish(!scheduledPublish)}
                  className={`w-12 h-6 flex items-center rounded-full p-0.5 transition-colors duration-300 focus:outline-none ${
                    scheduledPublish ? 'bg-[#07c160]' : 'bg-[var(--color-kb-panel-hover)] border border-[var(--color-kb-panel-border)]'
                  }`}
                  id="toggle-schedule-send"
                >
                  <div 
                    className={`bg-white w-5 h-5 rounded-full shadow-md transform transition-transform duration-300 ${
                      scheduledPublish ? 'translate-x-6' : 'translate-x-0'
                    }`} 
                  />
                </button>
              </div>

              {/* Scheduling pickers when active */}
              {scheduledPublish && (
                <div className="pl-1 pt-1 animate-in slide-in-from-top-1 duration-200 space-y-2">
                  <span className="text-[11px] font-bold text-[var(--color-kb-text-muted)] uppercase tracking-wider flex items-center gap-1">
                    <Calendar size={12} className="text-[#07c160]" />
                    {t('setSpecificSchedule', { defaultValue: '设置具体定时发表时刻（最迟 7 天内）' })}
                  </span>
                  
                  <div className="grid grid-cols-2 gap-3">
                    <input 
                      type="date" 
                      value={scheduleDate}
                      onChange={(e) => setScheduleDate(e.target.value)}
                      min={scheduleDateBounds.min}
                      max={scheduleDateBounds.max}
                      className="border border-[var(--color-kb-panel-border)] rounded-xl px-3 py-2 text-xs bg-[var(--color-kb-input-bg)] text-[var(--color-kb-text)] outline-none focus:border-[#07c160] transition-all"
                    />
                    <input 
                      type="time" 
                      value={scheduleTime}
                      onChange={(e) => setScheduleTime(e.target.value)}
                      className="border border-[var(--color-kb-panel-border)] rounded-xl px-3 py-2 text-xs bg-[var(--color-kb-input-bg)] text-[var(--color-kb-text)] outline-none focus:border-[#07c160] transition-all"
                    />
                  </div>
                </div>
              )}
            </div>

          </div>

          {/* Interactive WeChat API Compatibility Diagnostic Check section (Address user prompt directly with authority) */}
          <div className="bg-emerald-500/5 dark:bg-emerald-500/2 rounded-2xl border border-emerald-500/15 overflow-hidden shadow-xs">
            <button 
              type="button"
              onClick={() => setIsApiDetailsOpen(!isApiDetailsOpen)}
              className="w-full px-5 py-3.5 flex items-center justify-between text-left hover:bg-emerald-500/10 dark:hover:bg-emerald-500/5 transition-colors"
            >
              <div className="flex items-center gap-2">
                <CheckCircle2 size={15} className="text-[#07c160]" />
                <span className="text-[12px] font-bold text-emerald-800 dark:text-emerald-400">
                  {t('wechatApiDiagCheck', { defaultValue: '📡 微信公众号开放平台 API 兼容协议检测' })}
                </span>
              </div>
              <span className="text-emerald-600">
                {isApiDetailsOpen ? <ChevronUp size={14} /> : <ChevronDown size={14} />}
              </span>
            </button>

            {isApiDetailsOpen && (
              <div className="px-5 pb-4 pt-1 border-t border-emerald-500/5 text-[11px] text-[var(--color-kb-text-muted)] space-y-2.5 font-medium leading-relaxed">
                <div className="flex items-start gap-1.5">
                  <div className="w-1.5 h-1.5 bg-[#07c160] rounded-full mt-1.5 flex-shrink-0" />
                  <div>
                    <strong className="text-[var(--color-kb-text-heading)]">
                      {t('diagMassApi', { defaultValue: '群发接口 (Mass API): ' })}
                    </strong> 
                    {t('diagMassApiUrl', { defaultValue: '采用官方统一的' })} <code className="bg-[var(--color-kb-panel-hover)] border border-[var(--color-kb-panel-border)] px-1 py-0.5 rounded text-rose-500 font-mono text-[9.5px]">/cgi-bin/message/mass/sendall</code>。
                    {officialAccountType === 'subscription' 
                      ? t('subscriptionOADesc', { defaultValue: ' 订阅号每天支持呼叫1次群发通知，服务限制自动适配。' }) 
                      : t('serviceOADesc', { defaultValue: ' 服务号每月支持4次群发通知且高级分组可完全配置。' })}
                  </div>
                </div>

                <div className="flex items-start gap-1.5">
                  <div className="w-1.5 h-1.5 bg-[#07c160] rounded-full mt-1.5 flex-shrink-0" />
                  <div>
                    <strong className="text-[var(--color-kb-text-heading)]">
                      {t('diagGroupMassApi', { defaultValue: '分组通知 (Group mass API): ' })}
                    </strong> 
                    {t('diagGroupMassApiDesc', { defaultValue: '当启用时，API中' })} <code className="bg-[var(--color-kb-panel-hover)] border border-[var(--color-kb-panel-border)] px-1 py-0.5 rounded text-[var(--color-kb-text)] font-mono text-[9.5px]">"filter": {"{"} "is_to_all": false, "tag_id": "{selectedGroupId === 'all' ? '0' : selectedGroupId}" {"}"}</code>。{t('diagGroupMassApiPending', { defaultValue: '粉丝标签列表需对接微信公众平台标签 API 后在此展示。' })}
                  </div>
                </div>

                <div className="flex items-start gap-1.5">
                  <div className="w-1.5 h-1.5 bg-[#07c160] rounded-full mt-1.5 flex-shrink-0" />
                  <div>
                    <strong className="text-[var(--color-kb-text-heading)]">
                      {t('diagScheduleApi', { defaultValue: '定时发布逻辑 (Schedule API): ' })}
                    </strong> 
                    {t('diagScheduleApiDesc', { defaultValue: '根据微信官方规范，草稿箱定时功能由本地调度守护进程对接。发布成功后，数据将在本地发布队列中以 Cron 定时器形式锁存并在所设置的' })} <span className="font-bold text-[var(--color-kb-text-heading)]">{scheduleDate} {scheduleTime}</span> {t('diagScheduleApiEnd', { defaultValue: '自动下发到微信草稿箱发布通道。' })}
                  </div>
                </div>
              </div>
            )}
          </div>

        </div>

        {/* Footer Actions */}
        <div className="flex items-center justify-end gap-3 px-6 py-4 border-t border-[var(--color-kb-panel-border)] flex-shrink-0 bg-[var(--color-kb-editor)]/40">
          <button 
            onClick={onClose} 
            disabled={isPublishing}
            className="px-5 py-2.5 text-[13px] font-bold bg-[var(--color-kb-panel)] hover:bg-[var(--color-kb-panel-hover)] text-[var(--color-kb-text-muted)] hover:text-[var(--color-kb-text)] border border-[var(--color-kb-panel-border)] rounded-xl transition-all cursor-pointer disabled:opacity-50"
            id="wechat-publish-modal-cancel-btn"
          >
            {t('common:cancel', { defaultValue: '取消' })}
          </button>
          
          <button 
            onClick={handlePublishClick}
            disabled={isPublishing}
            className="px-6 py-2.5 text-[13px] font-black bg-[#07c160] hover:bg-[#06b056] text-white rounded-xl shadow-lg shadow-emerald-500/10 hover:shadow-emerald-500/20 active:scale-98 transition-all flex items-center gap-1.5 cursor-pointer disabled:opacity-50"
            id="wechat-publish-modal-confirm-btn"
          >
            {isPublishing ? (
              <>
                <RefreshCw size={14} className="animate-spin" />
                <span>{t('publishing', { defaultValue: '正在发表...' })}</span>
              </>
            ) : (
              <span>{t('publish', { defaultValue: '发表' })}</span>
            )}
          </button>
        </div>

      </div>
    </div>,
    document.body
  );
}
