import { X } from 'lucide-react';

export interface InsertToolConfigModalProps {
  activeInsertType: string | null;
  setActiveInsertType: (type: string | null) => void;
  widgetTitle: string;
  setWidgetTitle: (val: string) => void;
  widgetSubtitle: string;
  setWidgetSubtitle: (val: string) => void;
  widgetUrl: string;
  setWidgetUrl: (val: string) => void;
  widgetQuota: string;
  setWidgetQuota: (val: string) => void;
  widgetMerchant: string;
  setWidgetMerchant: (val: string) => void;
  widgetCondition: string;
  setWidgetCondition: (val: string) => void;
  widgetQuestion: string;
  setWidgetQuestion: (val: string) => void;
  widgetAnswer: string;
  setWidgetAnswer: (val: string) => void;
  widgetOptions: string[];
  setWidgetOptions: (val: string[]) => void;
  widgetHtml?: string;
  setWidgetHtml?: (val: string) => void;
  onConfirm: () => void;
}

export function InsertToolConfigModal({
  activeInsertType,
  setActiveInsertType,
  widgetTitle,
  setWidgetTitle,
  widgetSubtitle,
  setWidgetSubtitle,
  widgetUrl,
  setWidgetUrl,
  widgetQuota,
  setWidgetQuota,
  widgetMerchant,
  setWidgetMerchant,
  widgetCondition,
  setWidgetCondition,
  widgetQuestion,
  setWidgetQuestion,
  widgetAnswer,
  setWidgetAnswer,
  widgetOptions,
  setWidgetOptions,
  widgetHtml,
  setWidgetHtml,
  onConfirm
}: InsertToolConfigModalProps) {
  if (!activeInsertType) return null;

  return (
    <div className="fixed inset-0 bg-zinc-950/40 backdrop-blur-md z-[300] flex items-center justify-center p-4 animate-in fade-in duration-200">
      <div className="bg-[var(--color-kb-panel)] border border-[var(--color-kb-panel-border)] rounded-2xl max-w-md w-full overflow-hidden shadow-2xl flex flex-col animate-in zoom-in-95 duration-200">
        <div className="p-5 border-b border-[var(--color-kb-panel-border)] flex items-center justify-between">
          <span className="text-base font-bold text-[var(--color-kb-text-heading)] flex items-center gap-1.5">
             💎 新增组件: 从「{activeInsertType === 'coupons' ? '卡券' : activeInsertType === 'vote' ? '投票' : activeInsertType === 'link' ? '超链接' : activeInsertType === 'location' ? '地理位置' : activeInsertType === 'channel' ? '视频号' : activeInsertType === 'qa' ? '问答' : activeInsertType === 'card' ? '公众号卡片' : activeInsertType === 'gifts' ? '赞赏礼物' : '组件'}」导入
          </span>
          <button onClick={() => setActiveInsertType(null)} className="text-[var(--color-kb-text-muted)] hover:text-[var(--color-kb-text)] transition-colors">
            <X size={18} />
          </button>
        </div>
        
        <div className="p-6 space-y-4 max-h-[60vh] overflow-y-auto">
          {/* Conditional Input forms based on insertion type */}
          {(activeInsertType === 'link' || activeInsertType === 'search' || activeInsertType === 'location' || activeInsertType === 'channel' || activeInsertType === 'card') && (
            <div>
              <label className="block text-xs font-bold text-[var(--color-kb-text-heading)] mb-1.5">标题 / 主题名称</label>
              <input 
                type="text" 
                value={widgetTitle} 
                onChange={e => setWidgetTitle(e.target.value)}
                className="w-full bg-[var(--color-kb-editor)] border border-[var(--color-kb-panel-border)] rounded-lg px-3 py-2 text-xs text-[var(--color-kb-text)] focus:outline-none focus:border-[var(--color-kb-accent)]"
                placeholder="请输入名称" 
              />
            </div>
          )}

          {(activeInsertType === 'location' || activeInsertType === 'card') && (
            <div>
              <label className="block text-xs font-bold text-[var(--color-kb-text-heading)] mb-1.5">副标题 / 描述</label>
              <input 
                type="text" 
                value={widgetSubtitle} 
                onChange={e => setWidgetSubtitle(e.target.value)}
                className="w-full bg-[var(--color-kb-editor)] border border-[var(--color-kb-panel-border)] rounded-lg px-3 py-2 text-xs text-[var(--color-kb-text)] focus:outline-none focus:border-[var(--color-kb-accent)]"
                placeholder="请输入描述文字" 
              />
            </div>
          )}

          {activeInsertType === 'link' && (
            <div>
              <label className="block text-xs font-bold text-[var(--color-kb-text-heading)] mb-1.5">链接地址 / Web URL</label>
              <input 
                type="text" 
                value={widgetUrl} 
                onChange={e => setWidgetUrl(e.target.value)}
                className="w-full bg-[var(--color-kb-editor)] border border-[var(--color-kb-panel-border)] rounded-lg px-3 py-2 text-xs text-[var(--color-kb-text)] focus:outline-none focus:border-[var(--color-kb-accent)]"
                placeholder="例如: https://..." 
              />
            </div>
          )}

          {activeInsertType === 'coupons' && (
            <div className="space-y-3">
              <div>
                <label className="block text-xs font-bold text-[var(--color-kb-text-heading)] mb-1">卡券面额</label>
                <input type="text" value={widgetQuota} onChange={e => setWidgetQuota(e.target.value)} className="w-full bg-[var(--color-kb-editor)] border border-[var(--color-kb-panel-border)] rounded-lg px-3 py-2 text-xs text-[var(--color-kb-text)] focus:outline-none focus:border-[var(--color-kb-accent)]" />
              </div>
              <div>
                <label className="block text-xs font-bold text-[var(--color-kb-text-heading)] mb-1">商户名称</label>
                <input type="text" value={widgetMerchant} onChange={e => setWidgetMerchant(e.target.value)} className="w-full bg-[var(--color-kb-editor)] border border-[var(--color-kb-panel-border)] rounded-lg px-3 py-2 text-xs text-[var(--color-kb-text)] focus:outline-none focus:border-[var(--color-kb-accent)]" />
              </div>
              <div>
                <label className="block text-xs font-bold text-[var(--color-kb-text-heading)] mb-1">使用门槛</label>
                <input type="text" value={widgetCondition} onChange={e => setWidgetCondition(e.target.value)} className="w-full bg-[var(--color-kb-editor)] border border-[var(--color-kb-panel-border)] rounded-lg px-3 py-2 text-xs text-[var(--color-kb-text)] focus:outline-none focus:border-[var(--color-kb-accent)]" />
              </div>
            </div>
          )}

          {activeInsertType === 'vote' && (
            <div className="space-y-3">
              <p className="text-[11px] text-[var(--color-kb-text-muted)]">此项将生成支持单选与自动统计效果的专业微信文章投票面板。</p>
              <div>
                <label className="block text-xs font-bold text-[var(--color-kb-text-heading)] mb-1">候选选项 (用英文逗号分割)</label>
                <textarea 
                  value={widgetOptions.join(', ')} 
                  onChange={e => setWidgetOptions(e.target.value.split(',').map(s => s.trim()))} 
                  className="w-full h-16 bg-[var(--color-kb-editor)] border border-[var(--color-kb-panel-border)] rounded-lg px-3 py-2 text-xs text-[var(--color-kb-text)] focus:outline-none focus:border-[var(--color-kb-accent)]"
                />
              </div>
            </div>
          )}

          {activeInsertType === 'qa' && (
            <div className="space-y-3">
              <div>
                <label className="block text-xs font-bold text-[var(--color-kb-text-heading)] mb-1">提问内容 (Q)</label>
                <input type="text" value={widgetQuestion} onChange={e => setWidgetQuestion(e.target.value)} className="w-full bg-[var(--color-kb-editor)] border border-[var(--color-kb-panel-border)] rounded-lg px-3 py-2 text-xs text-[var(--color-kb-text)] focus:outline-none focus:border-[var(--color-kb-accent)]" />
              </div>
              <div>
                <label className="block text-xs font-bold text-[var(--color-kb-text-heading)] mb-1">解答内容 (A)</label>
                <textarea value={widgetAnswer} onChange={e => setWidgetAnswer(e.target.value)} className="w-full h-20 bg-[var(--color-kb-editor)] border border-[var(--color-kb-panel-border)] rounded-lg px-3 py-2 text-xs text-[var(--color-kb-text)] focus:outline-none focus:border-[var(--color-kb-accent)] resize-none" />
              </div>
            </div>
          )}

          {activeInsertType === 'templates' && (
            <div>
              <p className="text-xs text-[var(--color-kb-text-muted)]">
                点击下方的确认后，系统会在文章中当前光标位置完美插入一套包含精致左边框、精选色调引用的专业公众号文章小节排版大纲：
              </p>
            </div>
          )}

          {activeInsertType === 'ad' && (
            <div>
              <p className="text-xs text-[var(--color-kb-text-muted)] flex items-center gap-2">
                📱 将为您在文章正文插入一段带有“广告”透明角标的高逼真微信官方广告流量组卡片。
              </p>
            </div>
          )}

          {activeInsertType === 'gifts' && (
            <div>
              <p className="text-xs text-[var(--color-kb-text-muted)]">
                将为您在当前光标位置注入具有橙黄色浪漫主调的“打赏创作者”咖啡赞助组件。
              </p>
            </div>
          )}

          {activeInsertType === 'html' && (
            <div className="space-y-3">
              <p className="text-xs text-[var(--color-kb-text-muted)]">
                将插入下方输入的 HTML 源码到编辑器中进行渲染，您可以填入从其他网页复制的 HTML 代码或是自己手写的样式节点。
              </p>
              <div>
                <label className="block text-xs font-bold text-[var(--color-kb-text-heading)] mb-1">HTML 代码</label>
                <textarea 
                  value={widgetHtml || ''} 
                  onChange={e => setWidgetHtml?.(e.target.value)} 
                  className="w-full h-32 bg-[var(--color-kb-editor)] border border-[var(--color-kb-panel-border)] rounded-lg px-3 py-2 text-xs text-[var(--color-kb-text)] focus:outline-none focus:border-[var(--color-kb-accent)] font-mono"
                  placeholder="<div style='color: red;'>Hello World</div>"
                />
              </div>
            </div>
          )}
        </div>
        
        <div className="p-4 bg-[var(--color-kb-panel-hover)] border-t border-[var(--color-kb-panel-border)] flex justify-end gap-3.5">
          <button 
            onClick={() => setActiveInsertType(null)} 
            className="px-5 py-2 text-xs font-bold bg-[var(--color-kb-panel)] border border-[var(--color-kb-panel-border)] rounded-lg hover:bg-[var(--color-kb-panel-hover)] text-[var(--color-kb-text-heading)]"
          >
            取消
          </button>
          <button 
            onClick={onConfirm} 
            className="px-5 py-2 text-xs font-bold bg-[var(--color-kb-accent)] hover:bg-[var(--color-kb-accent-hover)] text-white rounded-lg shadow-sm"
          >
            确认并插入
          </button>
        </div>
      </div>
    </div>
  );
}
