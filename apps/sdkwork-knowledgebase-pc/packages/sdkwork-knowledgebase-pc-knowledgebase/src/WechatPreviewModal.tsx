import { WechatArticle } from './services/wechat';
import { Share, MoreHorizontal, ChevronLeft, X } from 'lucide-react';
import { sanitizePreviewHtml } from './utils/htmlSanitizer';

export interface WechatPreviewModalProps {
  isOpen: boolean;
  onClose: () => void;
  selectedArticle: WechatArticle;
}

export function WechatPreviewModal({ isOpen, onClose, selectedArticle }: WechatPreviewModalProps) {
  if (!isOpen || !selectedArticle) return null;

  return (
    <div className="fixed inset-0 z-[300] bg-black/70 flex items-center justify-center backdrop-blur-sm" onClick={onClose}>
      <div 
        className="w-[375px] h-[812px] bg-white rounded-[44px] shadow-[0_30px_60px_-15px_rgba(0,0,0,0.6)] border-[12px] border-zinc-900 overflow-hidden flex flex-col transform transition-all animate-in zoom-in-95 duration-300 relative"
        onClick={e => e.stopPropagation()}
      >
        {/* Hardware details - Island / Notch */}
        <div className="absolute top-0 inset-x-0 h-6 flex justify-center z-50">
           <div className="w-[120px] h-[30px] bg-zinc-900 rounded-b-[18px]"></div>
        </div>

        {/* Status Bar Mock */}
        <div className="h-11 w-full bg-[#EDEDED]/90 backdrop-blur-md flex items-center justify-between px-6 z-40 text-black font-semibold text-[14px]">
           <span>9:41</span>
           <div className="flex gap-1.5 items-center">
             <div className="w-4 h-3 rounded-sm border-[1.5px] border-black pb-0.5"><div className="w-2.5 h-full bg-black ml-0.5"></div></div>
             <div className="w-3 h-3 rounded-full bg-black"></div>
           </div>
        </div>
        
        {/* Wechat Header Mock */}
        <div className="h-14 bg-[#EDEDED]/90 backdrop-blur-md flex items-center justify-between px-3 z-30 flex-shrink-0 relative">
           <button onClick={onClose} className="p-2 -ml-2 text-zinc-800 flex items-center justify-center active:bg-black/5 rounded-full transition-colors">
              <ChevronLeft size={28} strokeWidth={2.5} />
           </button>
           <div className="absolute left-1/2 -translate-x-1/2 flex flex-col items-center justify-center w-[180px]">
             <span className="text-zinc-900 font-bold text-[16px] truncate w-full text-center">
                {selectedArticle.author || '公众号作者'}
             </span>
           </div>
           <div className="flex items-center gap-4 text-zinc-800 pr-1">
             <Share size={20} className="opacity-80" strokeWidth={2} />
             <MoreHorizontal size={24} className="opacity-80" strokeWidth={2} />
           </div>
        </div>
        
        {/* Scrollable Content Area */}
        <div className="flex-1 overflow-y-auto bg-white no-scrollbar pb-10">
          <div className="p-4 pt-5 pb-8 min-h-full">
            {/* Native Wechat article title styling */}
            <h1 className="text-[22px] font-bold text-[#333333] mb-3 leading-[1.4] tracking-normal break-words" style={{ fontFamily: '-apple-system, BlinkMacSystemFont, "Helvetica Neue", "PingFang SC", "Hiragino Sans GB", "Microsoft YaHei UI", "Microsoft YaHei", Arial, sans-serif' }}>
              {selectedArticle.title || '无标题文章'}
            </h1>
            
            {/* Native Wechat author/meta styling */}
            <div className="flex items-center gap-2 text-[15px] mb-8 leading-none">
              <span className="text-[#576b95] cursor-pointer inline-block" style={{ fontFamily: '-apple-system, BlinkMacSystemFont, "Helvetica Neue", "PingFang SC", "Hiragino Sans GB", "Microsoft YaHei UI", "Microsoft YaHei", Arial, sans-serif' }}>
                {selectedArticle.author || '公众号作者'}
              </span>
            </div>
            
            {/* Render HTML content exactly as Wechat does */}
            {selectedArticle.content ? (
              <div 
                className="wechat-preview-richtext" 
                style={{ 
                  wordBreak: 'break-word',
                  fontFamily: '-apple-system, BlinkMacSystemFont, "Helvetica Neue", "PingFang SC", "Hiragino Sans GB", "Microsoft YaHei UI", "Microsoft YaHei", Arial, sans-serif',
                  color: '#333',
                  fontSize: '17px',
                  lineHeight: '1.6',
                  letterSpacing: '0.034em'
                }}
                dangerouslySetInnerHTML={{ __html: sanitizePreviewHtml(selectedArticle.content) }}
              />
            ) : (
              <div className="text-gray-400 mt-20 text-center text-[15px]">文章暂无正文内容</div>
            )}
          </div>
        </div>

      </div>
      
      {/* Close button outside */}
      <button onClick={onClose} className="absolute top-8 right-8 w-12 h-12 bg-white/10 hover:bg-white/20 text-white rounded-full flex items-center justify-center backdrop-blur-md transition-all active:scale-95">
        <X size={24} strokeWidth={2} />
      </button>
    </div>
  );
}
