import React, { useState, useEffect } from 'react';
import { X, ZoomIn, ZoomOut } from 'lucide-react';

export interface WechatImageCropperModalProps {
  isOpen: boolean;
  imageSrc: string;
  onClose: () => void;
  onConfirm: (cropData: {
    cover: string;
    coverZoom: number;
    coverOffsetX: number;
    coverOffsetY: number;
    coverAspect: '2.35' | '1:1';
  }) => void;
  onBack?: () => void;
}

export function WechatImageCropperModal({ isOpen, imageSrc, onClose, onConfirm, onBack }: WechatImageCropperModalProps) {
  const [aspectMode, setAspectMode] = useState<'2.35' | '1:1'>('2.35');
  const [zoom, setZoom] = useState(1);
  const [offsetX, setOffsetX] = useState(0);
  const [offsetY, setOffsetY] = useState(0);
  
  const [isDragging, setIsDragging] = useState(false);
  const [dragStart, setDragStart] = useState({ x: 0, y: 0 });
  const [naturalSize, setNaturalSize] = useState({ w: 0, h: 0 });

  useEffect(() => {
    if (!imageSrc) return;
    const img = new Image();
    img.src = imageSrc;
    img.onload = () => {
      setNaturalSize({ w: img.naturalWidth, h: img.naturalHeight });
    };
  }, [imageSrc]);
  
  // Base dimensions for the cropper viewport
  const baseDim = {
    '2.35': { w: 560, h: 238 },
    '1:1': { w: 360, h: 360 }
  };

  const currentDim = baseDim[aspectMode];

  const enforceBounds = (x: number, y: number, z: number) => {
    if (!naturalSize.w || !naturalSize.h) return { x, y };
    
    const containerRatio = currentDim.w / currentDim.h;
    const imgRatio = naturalSize.w / naturalSize.h;
    
    let actualImgW = currentDim.w;
    let actualImgH = currentDim.h;
    
    if (imgRatio > containerRatio) {
      actualImgW = currentDim.h * imgRatio;
    } else {
      actualImgH = currentDim.w / imgRatio;
    }
    
    const zoomedW = actualImgW * z;
    const zoomedH = actualImgH * z;
    
    const maxOffsetX = Math.max(0, (zoomedW - currentDim.w) / 2);
    const maxOffsetY = Math.max(0, (zoomedH - currentDim.h) / 2);
    
    return {
      x: Math.min(Math.max(x, -maxOffsetX), maxOffsetX),
      y: Math.min(Math.max(y, -maxOffsetY), maxOffsetY)
    };
  };

  if (!isOpen) return null;

  const handleApply = () => {
    onConfirm({
      cover: imageSrc,
      coverZoom: zoom,
      coverOffsetX: offsetX,
      coverOffsetY: offsetY,
      coverAspect: aspectMode
    });
    onClose();
  };

  const handleWheel = (e: React.WheelEvent) => {
    const newZoom = Math.min(Math.max(1, zoom - e.deltaY * 0.002), 3);
    const bounded = enforceBounds(offsetX, offsetY, newZoom);
    setZoom(newZoom);
    setOffsetX(bounded.x);
    setOffsetY(bounded.y);
  };

  const handleMouseDown = (e: React.MouseEvent) => {
    setIsDragging(true);
    setDragStart({ x: e.clientX - offsetX, y: e.clientY - offsetY });
  };

  const handleMouseMove = (e: React.MouseEvent) => {
    if (!isDragging) return;
    const bounded = enforceBounds(e.clientX - dragStart.x, e.clientY - dragStart.y, zoom);
    setOffsetX(bounded.x);
    setOffsetY(bounded.y);
  };

  const handleMouseUp = () => {
    setIsDragging(false);
  };

  return (
    <div className="fixed inset-0 z-[600] bg-black/40 flex items-center justify-center p-4">
      <div className="bg-white dark:bg-[var(--color-kb-editor)] w-[1000px] h-[750px] rounded shadow-2xl flex flex-col overflow-hidden">

        {/* Header */}
        <div className="flex items-center justify-between px-8 py-5">
          <h3 className="text-xl font-normal text-gray-800 dark:text-[var(--color-kb-text-heading)]">编辑封面</h3>
          <button
            onClick={onClose}
            className="text-gray-400 hover:text-gray-600 dark:text-zinc-500 dark:hover:text-zinc-300 transition-colors"
          >
            <X size={24} strokeWidth={1} />
          </button>
        </div>

        {/* Content */}
        <div className="flex-1 flex px-8 overflow-hidden min-h-0">
          
          {/* Left Canvas */}
          <div className="flex-1 bg-[#f7f7f7] relative flex flex-col items-center justify-center overflow-hidden border border-gray-100 rounded">
            
            {/* Cropping Viewport */}
            <div 
              className={`relative overflow-hidden bg-white shadow-sm flex items-center justify-center transition-[width,height] ${isDragging ? 'cursor-grabbing' : 'cursor-grab'}`}
              style={{ width: currentDim.w, height: currentDim.h }}
              onWheel={handleWheel}
              onMouseDown={handleMouseDown}
              onMouseMove={handleMouseMove}
              onMouseUp={handleMouseUp}
              onMouseLeave={handleMouseUp}
            >
              {/* Grid guide */}
              <div className="absolute inset-0 z-10 pointer-events-none">
                <div className="w-full h-full border border-white/40 grid grid-cols-3 grid-rows-3 shadow-[0_0_0_9999px_rgba(0,0,0,0.05)]">
                  {Array.from({ length: 9 }).map((_, i) => (
                    <div key={i} className="border border-white/20" />
                  ))}
                </div>
              </div>

              <img 
                src={imageSrc} 
                className="max-w-none pointer-events-none select-none transition-transform duration-75"
                style={{
                  transform: `translate(${offsetX}px, ${offsetY}px) scale(${zoom})`,
                  width: (() => {
                    if (!naturalSize.w) return '100%';
                    const containerRatio = currentDim.w / currentDim.h;
                    const imgRatio = naturalSize.w / naturalSize.h;
                    return imgRatio > containerRatio ? `${(imgRatio / containerRatio) * 100}%` : '100%';
                  })(),
                  height: (() => {
                    if (!naturalSize.w) return '100%';
                    const containerRatio = currentDim.w / currentDim.h;
                    const imgRatio = naturalSize.w / naturalSize.h;
                    return imgRatio > containerRatio ? '100%' : `${(containerRatio / imgRatio) * 100}%`;
                  })(),
                }}
              />
            </div>

            {/* Slider floating at bottom */}
            <div className="absolute bottom-6 bg-white dark:bg-zinc-800 shadow-md dark:shadow-none rounded-full px-4 py-2 flex items-center gap-3 border border-gray-100 dark:border-[var(--color-kb-panel-border)]">
              <ZoomOut size={16} className="text-gray-400 dark:text-zinc-500" />
              <input 
                type="range" 
                min="1" 
                max="3" 
                step="0.05"
                value={zoom} 
                onChange={(e) => {
                  const newZoom = parseFloat(e.target.value);
                  const bounded = enforceBounds(offsetX, offsetY, newZoom);
                  setZoom(newZoom);
                  setOffsetX(bounded.x);
                  setOffsetY(bounded.y);
                }}
                className="w-32 accent-[#07c160] h-1 bg-gray-200 dark:bg-zinc-700 rounded-lg appearance-none cursor-pointer"
              />
              <ZoomIn size={16} className="text-gray-400 dark:text-zinc-500" />
            </div>

          </div>

          {/* Right Panel: Previews */}
          <div className="w-[260px] ml-8 flex flex-col gap-6 pt-2">
            
            {/* 1:1 Preview */}
            <div className="flex flex-col gap-2">
              <span className="text-sm text-gray-700">1:1 (消息列表、转发卡片)</span>
              <div 
                onClick={() => { setAspectMode('1:1'); setZoom(1); setOffsetX(0); setOffsetY(0); }}
                className={`cursor-pointer rounded-lg p-3 border transition-colors ${aspectMode === '1:1' ? 'border-[#07c160] bg-[#07c160]/5' : 'border-transparent hover:border-gray-200'}`}
              >
                <div className="bg-[#f4f5f5] w-full aspect-[2.5/1] rounded overflow-hidden flex items-center p-3 relative space-x-3 border border-gray-100">
                   <div className="flex-1 flex flex-col gap-2 justify-center">
                      <div className="h-2.5 w-full bg-gray-200 rounded-full"></div>
                      <div className="h-2.5 w-2/3 bg-gray-200 rounded-full"></div>
                   </div>
                   <div className="w-14 h-14 flex-shrink-0 bg-white rounded overflow-hidden relative items-center justify-center flex">
                     {aspectMode === '1:1' ? (
                        <img 
                          src={imageSrc} 
                          className="max-w-none pointer-events-none select-none transition-transform duration-75"
                          style={{
                            transform: `translate(${offsetX * (56 / baseDim['1:1'].w)}px, ${offsetY * (56 / baseDim['1:1'].w)}px) scale(${zoom})`,
                            width: (() => {
                              if (!naturalSize.w) return '100%';
                              const cRatio = 1;
                              const iRatio = naturalSize.w / naturalSize.h;
                              return iRatio > cRatio ? `${(iRatio / cRatio) * 100}%` : '100%';
                            })(),
                            height: (() => {
                              if (!naturalSize.w) return '100%';
                              const cRatio = 1;
                              const iRatio = naturalSize.w / naturalSize.h;
                              return iRatio > cRatio ? '100%' : `${(cRatio / iRatio) * 100}%`;
                            })(),
                          }}
                        />
                     ) : (
                        <img src={imageSrc} className="w-full h-full object-cover" />
                     )}
                   </div>
                </div>
              </div>
            </div>

            {/* 2.35:1 Preview */}
            <div className="flex flex-col gap-2">
              <span className="text-sm text-gray-700 dark:text-zinc-200">2.35:1 (内容推荐)</span>
              <div 
                onClick={() => { setAspectMode('2.35'); setZoom(1); setOffsetX(0); setOffsetY(0); }}
                className={`cursor-pointer rounded-lg p-3 border transition-colors ${aspectMode === '2.35' ? 'border-[#07c160] bg-[#07c160]/5' : 'border-transparent hover:border-gray-200 dark:hover:border-[var(--color-kb-panel-border)]'}`}
              >
                <div className="bg-white dark:bg-zinc-900 border border-gray-200 dark:border-[var(--color-kb-panel-border)] shadow-sm w-full rounded overflow-hidden flex flex-col">
                  <div className="h-6 bg-white dark:bg-zinc-900 flex items-center px-3">
                    <div className="h-2 w-1/4 bg-gray-200 dark:bg-zinc-700 rounded-full"></div>
                  </div>
                  <div className="w-full aspect-[2.35/1] bg-gray-100 dark:bg-zinc-800 overflow-hidden relative items-center justify-center flex">
                    {aspectMode === '2.35' ? (
                       <img 
                         src={imageSrc} 
                         className="max-w-none pointer-events-none select-none transition-transform duration-75"
                         style={{
                           transform: `translate(${offsetX * (232 / baseDim['2.35'].w)}px, ${offsetY * (232 / baseDim['2.35'].w)}px) scale(${zoom})`,
                           width: (() => {
                             if (!naturalSize.w) return '100%';
                             const cRatio = 2.35;
                             const iRatio = naturalSize.w / naturalSize.h;
                             return iRatio > cRatio ? `${(iRatio / cRatio) * 100}%` : '100%';
                           })(),
                           height: (() => {
                             if (!naturalSize.w) return '100%';
                             const cRatio = 2.35;
                             const iRatio = naturalSize.w / naturalSize.h;
                             return iRatio > cRatio ? '100%' : `${(cRatio / iRatio) * 100}%`;
                           })(),
                         }}
                       />
                    ) : (
                       <img src={imageSrc} className="w-full h-full object-cover" />
                    )}
                  </div>
                </div>
              </div>
            </div>

          </div>

        </div>

        {/* Footer */}
        <div className="py-4 border-t border-gray-100 dark:border-[var(--color-kb-panel-border)] flex justify-center gap-4 mt-6">
          <button
            onClick={onBack ? onBack : onClose}
            className="px-8 py-2 text-sm text-gray-700 dark:text-zinc-200 bg-white dark:bg-zinc-800 border border-gray-300 dark:border-[var(--color-kb-panel-border)] rounded hover:bg-gray-50 dark:hover:bg-zinc-700 transition-colors"
          >
            {onBack ? '上一步' : '取消'}
          </button>
          <button 
            onClick={handleApply} 
            className="px-8 py-2 text-sm text-white bg-[#07c160] hover:bg-[#06ad56] rounded transition-colors"
          >
            确认
          </button>
        </div>

      </div>
    </div>
  );
}
