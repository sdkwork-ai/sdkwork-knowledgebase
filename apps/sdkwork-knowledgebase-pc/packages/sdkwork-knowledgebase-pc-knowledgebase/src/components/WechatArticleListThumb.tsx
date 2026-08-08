import { Image as ImageIcon } from 'lucide-react';
import { WechatArticle } from '../services/wechat';
import { useTranslation } from 'react-i18next';
import type { ReactKeyedComponentProps } from '@sdkwork/sdkwork-knowledgebase-pc-commons/reactKeyedProps';

export interface WechatArticleListThumbProps extends ReactKeyedComponentProps {
  article: WechatArticle;
  idx: number;
  isSelected: boolean;
  onClick: () => void;
  isLast?: boolean;
}

export function WechatArticleListThumb({
  article,
  idx,
  isSelected,
  onClick,
  isLast = false
}: WechatArticleListThumbProps) {
  const { t } = useTranslation('common');
  const isFirst = idx === 0;
  const roundedClasses = `${isFirst ? 'rounded-t-[11px]' : ''} ${isLast ? 'rounded-b-[11px]' : ''}`;

  return (
    <div 
      data-selected={isSelected ? "true" : "false"}
      onClick={onClick}
      className={`relative group cursor-pointer overflow-hidden transition-all duration-200 ${
        isSelected 
          ? 'bg-[var(--color-kb-panel)] z-10' 
          : 'bg-[var(--color-kb-panel-hover)] hover:bg-[var(--color-kb-panel)] hover:shadow-sm'
      } ${isFirst ? 'h-[140px]' : 'h-[80px] flex justify-between items-center p-3.5'} ${roundedClasses}`}
    >
      {/* Selection Outline Overlay */}
      {isSelected && (
        <div className={`absolute inset-0 border-2 border-[#07c160] pointer-events-none z-20 shadow-[0_4px_16px_rgba(7,193,96,0.12)] ${roundedClasses}`} />
      )}

      {isFirst ? (
        <>
          {/* Main article cover */}
          <div className="absolute inset-0 bg-[var(--color-kb-panel-hover)] flex items-center justify-center overflow-hidden">
            {article.cover ? (
              <img 
                src={article.cover} 
                alt="" 
                className="w-full h-full object-contain animate-fade-in transition-transform duration-200" 
                style={{
                  transform: `scale(${article.coverZoom || 1}) translate(${(article.coverOffsetX || 0) * 0.65}px, ${(article.coverOffsetY || 0) * 0.65}px)`,
                }}
              />
            ) : (
              <div className="text-[var(--color-kb-text-muted)] flex flex-col items-center">
                <ImageIcon size={20} />
              </div>
            )}
          </div>
          <div className="absolute inset-x-0 bottom-0 pt-10 pb-3 px-4 bg-gradient-to-t from-black/85 to-transparent">
            <div className="text-white text-[14px] font-semibold leading-snug line-clamp-2">{article.title || t('titleStr')}</div>
          </div>
        </>
      ) : (
        <>
          {/* Sub article */}
          <div className="text-[13px] text-[var(--color-kb-text-heading)] line-clamp-2 flex-1 pr-3 leading-snug font-medium min-w-0">
            {article.title || t('titleStr')}
          </div>
          <div className="w-[48px] h-[48px] bg-[var(--color-kb-panel-hover)] flex-shrink-0 flex justify-center items-center rounded-lg overflow-hidden border border-[var(--color-kb-panel-border)]">
            {article.cover ? (
              <img 
                src={article.cover} 
                alt="" 
                className="w-full h-full object-contain animate-fade-in transition-transform duration-200" 
                style={{
                  transform: `scale(${article.coverZoom || 1}) translate(${(article.coverOffsetX || 0) * 0.17}px, ${(article.coverOffsetY || 0) * 0.17}px)`,
                }}
              />
            ) : (
              <ImageIcon size={16} className="text-[var(--color-kb-text-muted)] opacity-50" />
            )}
          </div>
        </>
      )}
    </div>
  );
}
