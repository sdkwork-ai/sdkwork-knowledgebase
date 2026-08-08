import { useState } from 'react';
import { X, Send, Check, AlertCircle } from 'lucide-react';
import { isKnowledgebaseApiAvailable } from 'sdkwork-knowledgebase-pc-core';
import { DocumentMeta } from './services/document';
export interface PublishModalProps {
  documents: DocumentMeta[];
  onClose: () => void;
  onWechatFlow?: () => void;
}

const PUBLISH_PLATFORMS = [
  { id: 'wechat', name: '微信公众号', icon: '💬', desc: '支持多图文发布 (Official API)', disabled: false },
  { id: 'wordpress', name: 'WordPress', icon: '📝', desc: 'REST API 同步', disabled: true },
  { id: 'ghost', name: 'Ghost', icon: '👻', desc: 'Ghost Admin API', disabled: true },
  { id: 'notion', name: 'Notion', icon: '📓', desc: 'Notion API (Block Sync)', disabled: true },
  { id: 'medium', name: 'Medium', icon: 'M', desc: '基于 User API', disabled: true },
  { id: 'hashnode', name: 'Hashnode', icon: 'H', desc: 'GraphQL API', disabled: true },
  { id: 'blogger', name: 'Blogger', icon: 'B', desc: 'Google Blogger API', disabled: true },
  { id: 'shopify', name: 'Shopify', icon: '🛒', desc: 'Shopify Blog API', disabled: true },
  { id: 'devto', name: 'Dev.to', icon: '👨‍💻', desc: '基于 Forem API', disabled: true },
  { id: 'yuque', name: '语雀', icon: '🕊️', desc: '语雀 Open API', disabled: true },
  { id: 'zhihu', name: '知乎', icon: 'Z', desc: '基于专栏草稿发布', disabled: true },
];

export function PublishModal({ documents, onClose, onWechatFlow }: PublishModalProps) {
  const [selectedPlatform, setSelectedPlatform] = useState<string>('wechat');
  const isPublishing = false;
  const apiMode = isKnowledgebaseApiAvailable();

  const togglePlatform = (id: string) => {
    setSelectedPlatform(id);
  };

  const handlePublish = () => {
    if (selectedPlatform === 'wechat') {
      onWechatFlow?.();
    }
  };

  return (
    <div className="fixed inset-0 bg-zinc-950/40 z-[100] flex items-center justify-center backdrop-blur-md">
      <div className="bg-white dark:bg-[var(--color-kb-editor)] w-[880px] h-[640px] rounded-2xl shadow-2xl border border-zinc-200/80 dark:border-[var(--color-kb-panel-border)] flex flex-col overflow-hidden animate-in fade-in zoom-in duration-200">
        <div className="flex items-center justify-between px-6 py-5 border-b border-zinc-200/80 dark:border-[var(--color-kb-panel-border)] bg-[#fafafa] dark:bg-[var(--color-kb-panel)]/30 shadow-sm z-10">
          <div>
            <h3 className="font-display font-extrabold text-[16px] tracking-tight text-zinc-900 dark:text-[var(--color-kb-text-heading)]">多渠道发布</h3>
            <p className="text-[12px] font-medium text-zinc-500 dark:text-[var(--color-kb-text-muted)] mt-1">将 {documents.length} 篇文章分发至多平台</p>
          </div>
          <button onClick={onClose} className="text-zinc-400 hover:text-red-500 hover:bg-red-50 dark:text-[var(--color-kb-text-muted)] transition-all p-2 rounded-xl active:scale-95">
            <X size={18} strokeWidth={2.5} />
          </button>
        </div>
        
        <div className="flex flex-1 overflow-hidden relative">
          {apiMode && (
            <div className="absolute top-[72px] left-6 right-6 z-20 flex items-start gap-2 rounded-xl border border-emerald-200/80 bg-emerald-50 px-4 py-3 text-[12px] text-emerald-900 dark:border-emerald-500/30 dark:bg-emerald-500/10 dark:text-emerald-100">
              <AlertCircle size={16} className="shrink-0 mt-0.5" />
              <span>API 模式下微信公众号预览与发布已接入 Knowledgebase SDK；其他第三方平台请使用网站托管部署。</span>
            </div>
          )}
          {/* Left panel: selected docs */}
          <div className="w-[32%] border-r border-[var(--color-kb-panel-border)]/80 bg-[var(--color-kb-panel)]/30 flex flex-col">
            <div className="px-5 py-4 border-b border-[var(--color-kb-panel-border)]/50 text-[13px] font-bold text-[var(--color-kb-text-heading)] uppercase tracking-wider">
              待发布内容 ({documents.length})
            </div>
            <div className="flex-1 overflow-y-auto p-4 space-y-2">
              {documents.map(doc => (
                <div key={doc.id} className="text-[13px] font-medium px-4 py-3 rounded-xl bg-[var(--color-kb-editor)] border border-[var(--color-kb-panel-border)] text-[var(--color-kb-text)] line-clamp-2 shadow-sm relative overflow-hidden" title={doc.title}>
                  <div className="absolute left-0 top-0 bottom-0 w-1 bg-[var(--color-kb-accent)]"></div>
                  {doc.title}
                </div>
              ))}
            </div>
          </div>
          
          {/* Right panel: platforms */}
          <div className="flex-1 flex flex-col bg-[var(--color-kb-editor)]">
            <div className="px-6 py-4 border-b border-[var(--color-kb-panel-border)]/50 text-[13px] font-bold text-[var(--color-kb-text-heading)] uppercase tracking-wider flex items-center justify-between">
              <span>选择发布平台</span>
              <span className="text-[11px] font-medium text-[var(--color-kb-accent)] bg-[var(--color-kb-accent)]/10 px-2.5 py-1 rounded-md">多选发布尚未开启</span>
            </div>
            <div className="flex-1 overflow-y-auto p-5 grid grid-cols-2 gap-4 auto-rows-max">
              {PUBLISH_PLATFORMS.map(platform => {
                const isSelected = selectedPlatform === platform.id;
                return (
                  <div 
                    key={platform.id}
                    onClick={() => {
                      if (platform.disabled || isPublishing) return;
                      togglePlatform(platform.id);
                    }}
                    className={`flex items-center p-3.5 rounded-2xl border-2 transition-all ${
                      platform.disabled 
                        ? 'opacity-40 cursor-not-allowed bg-[var(--color-kb-panel)]/50 border-transparent grayscale select-none' 
                        : 'cursor-pointer active:scale-[0.98]'
                    } ${
                      isSelected && !platform.disabled
                        ? 'border-[var(--color-kb-accent)] bg-[var(--color-kb-accent)]/5 shadow-md' 
                        : !platform.disabled
                          ? 'border-transparent hover:border-[var(--color-kb-panel-border)]/80 hover:bg-[var(--color-kb-panel-hover)] bg-[var(--color-kb-panel)]/20' 
                          : ''
                    }`}
                  >
                    <div className="w-10 h-10 rounded-xl bg-white dark:bg-white/5 border border-zinc-200/50 dark:border-white/10 flex items-center justify-center text-xl shadow-sm mr-4 shrink-0 relative overflow-hidden">
                      {platform.icon}
                    </div>
                    <div className="flex-1 min-w-0">
                      <div className={`text-[13.5px] font-bold truncate ${isSelected ? 'text-[var(--color-kb-accent)]' : 'text-[var(--color-kb-text-heading)]'}`}>
                        {platform.name}
                      </div>
                      <div className={`text-[11px] font-medium truncate mt-0.5 ${platform.disabled ? 'text-zinc-400' : isSelected ? 'text-[var(--color-kb-accent)]/80' : 'text-[var(--color-kb-text-muted)]'}`}>
                        {platform.disabled ? '即将推出...' : platform.desc}
                      </div>
                    </div>
                    <div className="ml-3 flex-shrink-0">
                      {isSelected && !platform.disabled ? (
                        <div className="w-5 h-5 bg-[var(--color-kb-accent)] text-white rounded-full flex items-center justify-center shadow-sm"><Check size={12} strokeWidth={3} /></div>
                      ) : (
                        <div className={`w-5 h-5 border-2 border-[var(--color-kb-panel-border)] rounded-full bg-[var(--color-kb-editor)] shadow-sm ${platform.disabled ? 'opacity-50' : ''}`} />
                      )}
                    </div>
                  </div>
                );
              })}
            </div>
          </div>
        </div>
        
        <div className="px-6 py-4 border-t border-[var(--color-kb-panel-border)] bg-[var(--color-kb-panel)]/50 flex items-center justify-between z-10 shadow-[0_-4px_20px_rgba(0,0,0,0.02)]">
          <div className="text-[12.5px] font-bold text-[var(--color-kb-text-muted)] bg-[var(--color-kb-editor)] px-3 py-1.5 rounded-lg border border-[var(--color-kb-panel-border)]/80 shadow-sm">
            已选中 <span className="text-[var(--color-kb-text-heading)] mx-1">{selectedPlatform && !PUBLISH_PLATFORMS.find(p => p.id === selectedPlatform)?.disabled ? 1 : 0}</span> 个聚合渠道
          </div>
          <div className="flex space-x-3">
            <button 
              onClick={onClose}
              disabled={isPublishing}
              className="px-5 py-2.5 text-[13px] font-bold text-[var(--color-kb-text)] border border-[var(--color-kb-panel-border)] bg-[var(--color-kb-editor)] hover:bg-[var(--color-kb-panel)] rounded-xl transition-all disabled:opacity-50 active:scale-95 shadow-sm"
            >
              放弃
            </button>
            <button 
              onClick={handlePublish}
              disabled={!selectedPlatform || isPublishing || PUBLISH_PLATFORMS.find(p => p.id === selectedPlatform)?.disabled}
              className="px-6 py-2.5 text-[13px] font-extrabold bg-[var(--color-kb-accent)] hover:bg-[var(--color-kb-accent-hover)] text-white rounded-xl transition-all disabled:opacity-40 disabled:grayscale disabled:cursor-not-allowed shadow-md hover:shadow-lg active:scale-95 flex items-center space-x-2"
            >
              {isPublishing ? (
                <>
                  <div className="w-4 h-4 border-2 border-white border-t-transparent rounded-full animate-spin" />
                  <span>分发执行中...</span>
                </>
              ) : (
                <>
                  <Send size={15} strokeWidth={2.5} />
                  <span>{selectedPlatform === 'wechat' ? '进入公众号发布' : `一键聚合发布 (${documents.length} 篇)`}</span>
                </>
              )}
            </button>
          </div>
        </div>
      </div>
    </div>
  );
}
