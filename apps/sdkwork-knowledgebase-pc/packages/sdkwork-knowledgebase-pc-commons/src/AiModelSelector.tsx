import { useState } from 'react';
import { Check, ChevronDown, Sparkles } from 'lucide-react';
import { AI_MODELS, AI_VENDORS, type AiModelInfo } from './aiModelCatalog';
import { useAiModelSelection } from './useAiModelSelection';

export type AiModelSelectorVariant = 'header' | 'compact';

export interface AiModelSelectorProps {
  variant?: AiModelSelectorVariant;
  /** Shown when no model name is available */
  fallbackLabel?: string;
  /** Popover opens above trigger (composer footer) or below (header) */
  popoverPlacement?: 'top' | 'bottom';
  className?: string;
}

export function AiModelSelector({
  variant = 'header',
  fallbackLabel = 'AI 助手',
  popoverPlacement = 'bottom',
  className = ''
}: AiModelSelectorProps) {
  const [isOpen, setIsOpen] = useState(false);
  const { activeVendor, setActiveVendor, activeModel, setActiveModel } = useAiModelSelection();

  const handleSelectModel = (model: AiModelInfo) => {
    setActiveModel(model);
    setIsOpen(false);
  };

  const triggerLabel = activeModel?.name ?? fallbackLabel;
  const isCompact = variant === 'compact';
  const opensUp = popoverPlacement === 'top';

  const popover = isOpen ? (
    <>
      <div className="fixed inset-0 z-[9998]" onClick={() => setIsOpen(false)} aria-hidden />
      <div
        className={`absolute z-[9999] flex w-[320px] max-h-[300px] overflow-hidden rounded-xl border border-[var(--color-kb-panel-border)] bg-[var(--color-kb-editor)] shadow-xl animate-in fade-in ${
          opensUp ? 'bottom-full mb-1.5 slide-in-from-bottom-2' : 'top-full mt-1.5 slide-in-from-top-2'
        } ${isCompact ? 'left-0' : 'left-0 md:left-0'}`}
      >
        <div className="w-[110px] shrink-0 overflow-y-auto search-theme-scrollbar border-r border-[var(--color-kb-panel-border)] bg-[var(--color-kb-panel)]/50 p-1.5 text-left flex flex-col gap-0.5">
          {AI_VENDORS.map((vendor) => (
            <button
              key={vendor.id}
              type="button"
              onClick={() => setActiveVendor(vendor.id)}
              className={`w-full rounded-lg px-2.5 py-1.5 text-left text-[12.5px] font-medium transition-colors ${
                activeVendor === vendor.id
                  ? 'bg-[var(--color-kb-editor)] text-[var(--color-kb-accent)] shadow-[0_1px_2px_rgba(0,0,0,0.05)]'
                  : 'text-[var(--color-kb-text-muted)] hover:bg-[var(--color-kb-panel-hover)]'
              }`}
            >
              {vendor.name}
            </button>
          ))}
        </div>
        <div className="flex flex-1 flex-col gap-0.5 overflow-y-auto search-theme-scrollbar p-1.5">
          {(AI_MODELS[activeVendor] || []).map((model) => (
            <button
              key={model.id}
              type="button"
              onClick={() => handleSelectModel(model)}
              className={`group flex w-full items-center justify-between rounded-lg px-2.5 py-1.5 text-left text-[12.5px] transition-colors ${
                activeModel?.id === model.id
                  ? 'bg-[var(--color-kb-panel-active)] text-[var(--color-kb-accent)]'
                  : 'text-[var(--color-kb-text)] hover:bg-[var(--color-kb-panel-hover)]'
              }`}
            >
              <span className="mr-2 truncate font-medium">{model.name}</span>
              {activeModel?.id === model.id && (
                <Check size={14} className="shrink-0 text-[var(--color-kb-accent)]" />
              )}
            </button>
          ))}
        </div>
      </div>
    </>
  ) : null;

  if (isCompact) {
    return (
      <div className={`relative ${className}`}>
        <button
          type="button"
          onClick={() => setIsOpen(!isOpen)}
          className="flex max-w-[168px] items-center gap-1 rounded-full px-2 py-1 text-[11px] font-semibold text-[var(--color-kb-text-muted)] transition-colors hover:bg-[var(--color-kb-panel-hover)] hover:text-[var(--color-kb-text)]"
          title="选择模型"
        >
          <Sparkles className="h-3 w-3 shrink-0 text-[var(--color-kb-accent)]" strokeWidth={2.5} />
          <span className="truncate">{triggerLabel}</span>
          <ChevronDown
            size={11}
            className={`shrink-0 transition-transform ${isOpen ? 'rotate-180' : ''}`}
            strokeWidth={2.5}
          />
        </button>
        {popover}
      </div>
    );
  }

  return (
    <div className={`relative ${className}`}>
      <button
        type="button"
        onClick={() => setIsOpen(!isOpen)}
        className="group -ml-1 flex cursor-pointer items-center rounded-lg border border-transparent px-2.5 py-1 text-[12.5px] font-bold text-[var(--color-kb-text-heading)] transition-all hover:border-[var(--color-kb-panel-border)] hover:bg-[var(--color-kb-panel-hover)] active:scale-95"
      >
        <div className="mr-1.5 flex h-[18px] w-[18px] items-center justify-center rounded border border-[color-mix(in_srgb,var(--color-kb-accent)_20%,var(--color-kb-panel-border))] bg-[var(--color-kb-panel-active)] shadow-[inset_0_1px_rgba(255,255,255,0.6)] dark:shadow-none">
          <Sparkles size={11} className="text-[var(--color-kb-accent)]" strokeWidth={2.5} />
        </div>
        {triggerLabel}
        <ChevronDown
          size={11}
          className="ml-1 text-[var(--color-kb-text-muted)] transition-colors group-hover:text-[var(--color-kb-text)]"
          strokeWidth={2.5}
        />
      </button>
      {popover}
    </div>
  );
}

export { useAiModelSelection } from './useAiModelSelection';
export * from './aiModelCatalog';
