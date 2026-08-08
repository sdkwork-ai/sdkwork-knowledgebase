import React, { useState, useRef, useEffect } from 'react';
import { 
  Image as ImageIcon, RotateCw, RotateCcw, 
  FlipHorizontal, FlipVertical, ZoomIn, ZoomOut, RefreshCw, ArrowRight,
  Sparkles, Download, Eye,
  Sliders, Wand2, Crop, Hammer, Cpu, FolderOutput
} from 'lucide-react';
import { DocumentMeta, KnowledgeBase, DocumentService } from '../../services/document';
import { shouldUseKnowledgebaseDemoFallback } from 'sdkwork-knowledgebase-pc-core';
import { MoveCopyModal } from '../../MoveCopyModal';
export interface ImageViewerProps {
  activeDoc: DocumentMeta;
  activeKb?: KnowledgeBase | null;
  onUpdateDocs?: () => void;
  onToastMessage?: (msg: string | null) => void;
}

type ToolType = 'compress' | 'super_resolution' | 'repair' | 'bg_remove' | 'art_filter' | 'watermark_erase' | 'smart_crop' | 'format_convert' | 'privacy_blur' | null;

export function ImageViewer({ activeDoc, activeKb, onUpdateDocs, onToastMessage }: ImageViewerProps) {
  const [scale, setScale] = useState<number>(1);
  const [rotation, setRotation] = useState<number>(0);
  const [flipH, setFlipH] = useState<boolean>(false);
  const [flipV, setFlipV] = useState<boolean>(false);
  
  // Natural / original source size of image
  const [imgMeta, setImgMeta] = useState<{ width: number; height: number } | null>(null);
  // Container size
  const [containerSize, setContainerSize] = useState<{ width: number; height: number }>({ width: 0, height: 0 });
  
  // Bottom Toolkit states
  const [activeTool, setActiveTool] = useState<ToolType>(null);
  const [isProcessing, setIsProcessing] = useState<boolean>(false);
  const [, setProcessLogs] = useState<string[]>([]);
  const [processProgress, setProcessProgress] = useState<number>(0);
  const [processSuccess, setProcessSuccess] = useState<boolean>(false);
  const [showComparison, setShowComparison] = useState<boolean>(false);
  const [, setResultImg] = useState<string | null>(null);
  const [resultMeta, setResultMeta] = useState<{ sizeStr: string; width: number; height: number; savedPercent?: number } | null>(null);

  // Popup tool menu and save-as state
  const [showMoreTools, setShowMoreTools] = useState<boolean>(false);
  const [showSaveAsModal, setShowSaveAsModal] = useState<boolean>(false);

  const handleSaveAsSubmit = async (targetKbId: string, targetFolderId: string | null) => {
    try {
      const toolLabelMap: Record<string, string> = {
        compress: '压缩版',
        super_resolution: '超分辨率版',
        repair: '图像修复版',
        bg_remove: '抠图去背版',
        art_filter: '滤镜增强版',
        watermark_erase: '去水印版',
        smart_crop: '智能裁剪版',
        format_convert: '格式转换版',
        privacy_blur: '模糊隐藏版'
      };
      
      const suffix = toolLabelMap[activeTool || ''] || '优化版';
      const lastDotIndex = activeDoc.title.lastIndexOf('.');
      const titleWithoutExt = lastDotIndex !== -1 ? activeDoc.title.substring(0, lastDotIndex) : activeDoc.title;
      const ext = lastDotIndex !== -1 ? activeDoc.title.substring(lastDotIndex) : '.jpg';
      
      const targetTitle = `${titleWithoutExt}_${suffix}${ext}`;

      await DocumentService.createDocument({
        title: targetTitle,
        type: 'image',
        kbId: targetKbId,
        parentId: targetFolderId,
        url: activeDoc.url,
        content: activeDoc.content || '',
        size: resultMeta?.sizeStr || '950.2 KB',
        author: 'Me'
      });

      if (onUpdateDocs) {
        onUpdateDocs();
      }

      if (onToastMessage) {
        onToastMessage(`另存为成功！已将文件保存至目标知识库目录。`);
      }
      setShowSaveAsModal(false);
    } catch (e) {
      console.error(e);
      if (onToastMessage) {
        onToastMessage('另存为失败，请重试');
      }
    }
  };

  // Tool parameter states
  // 1. Compression quality slider
void (useState<number>(75));
void (useState<'png' | 'jpeg' | 'webp'>('webp'));
  // 2. Super_resolution scaling ratio
void (useState<'2x' | '4x'>('2x'));
void (useState<'Real-ESRGAN' | 'Topaz Gigapixel v6'>('Real-ESRGAN'));
void (useState<number>(30));
  // 3. Repair Face weight
void (useState<number>(0.75));
void (useState<boolean>(true));
void (useState<'GFP-GAN v1.4' | 'CodeFormer'>('GFP-GAN v1.4'));
  // 4. Background removal
void (useState<'Meta SAM (v2)' | 'Remove.bg Ultra'>('Meta SAM (v2)'));
void (useState<string>('transparent'));

  const containerRef = useRef<HTMLDivElement>(null);
  const imgRef = useRef<HTMLImageElement>(null);
  const pipelineIntervalRef = useRef<ReturnType<typeof setInterval> | null>(null);
  const aiToolsEnabled = shouldUseKnowledgebaseDemoFallback();

  useEffect(() => {
    return () => {
      if (pipelineIntervalRef.current) {
        clearInterval(pipelineIntervalRef.current);
        pipelineIntervalRef.current = null;
      }
    };
  }, []);

  // Resize listener
  useEffect(() => {
    if (!containerRef.current) return;
    const observer = new ResizeObserver((entries) => {
      for (let entry of entries) {
        setContainerSize({
          width: Math.round(entry.contentRect.width),
          height: Math.round(entry.contentRect.height - 40), // Exclude header height
        });
      }
    });
    observer.observe(containerRef.current);
    
    // Initial size
    setContainerSize({
      width: containerRef.current.clientWidth,
      height: containerRef.current.clientHeight - 40,
    });

    return () => observer.disconnect();
  }, [activeDoc.id]);

  // Reset transforms whenever active document changes
  useEffect(() => {
    setScale(1);
    setRotation(0);
    setFlipH(false);
    setFlipV(false);
    setImgMeta(null);
    setActiveTool(null);
    setIsProcessing(false);
    setProcessLogs([]);
    setProcessProgress(0);
    setProcessSuccess(false);
    setResultImg(null);
    setResultMeta(null);
    setShowComparison(false);
  }, [activeDoc.id]);

  const handleImageLoad = (e: React.SyntheticEvent<HTMLImageElement>) => {
    const img = e.currentTarget;
    setImgMeta({
      width: img.naturalWidth,
      height: img.naturalHeight
    });
  };

  const startToolPipeline = (toolId: ToolType) => {
    if (isProcessing) return;
    setActiveTool(toolId);
    setIsProcessing(true);
    setProcessProgress(0);
    setProcessSuccess(false);
    setShowComparison(false);
    setProcessLogs([]);

    const logMessages: Record<string, string[]> = {
      compress: ['初始化 WebAssembly 模块...', '量化通道折损计算...', '应用 TinyPNG 算法...'],
      super_resolution: ['加载 Real-ESRGAN 模型...', '重构高频纹理...', '像素高清插值...'],
      repair: ['初始化修复人脸校准...', '划痕与噪点修复...', '深度重建面部细节...'],
      bg_remove: ['启动 SAM 端到端抠图...', '图像色彩前景分离...', '高精度边缘羽化...'],
      art_filter: ['分析图层色彩色调结构...', '应用高级AI艺术渲染滤镜...', '强化对比度与饱和度...'],
      watermark_erase: ['扫描画面高置信度水印边界...', '执行智能内容填充填补...', '修复局部纹理质感...'],
      smart_crop: ['智能检测人像与主体构图中心...', '计算多尺度最佳黄金分割比例...', '裁剪边缘像素对齐...'],
      format_convert: ['转换编码模式为 WebP/HEIC...', '写入最佳色彩描述ICC profile...', '压缩通道并导出规范文件...'],
      privacy_blur: ['分析面部及隐私铭牌等关键特征...', '高斯模糊遮蔽处理...', '生成无损加密图像遮蔽...']
    };

    const currentLogs = logMessages[toolId || 'compress'] || [];
    let logIndex = 0;

    const interval = setInterval(() => {
      if (logIndex < currentLogs.length) {
        setProcessProgress(_prev => Math.round(Math.min(((logIndex + 1) / currentLogs.length) * 100, 100)));
        logIndex++;
      } else {
        clearInterval(interval);
        setTimeout(() => {
          setIsProcessing(false);
          setProcessSuccess(true);
          setShowComparison(true);
          
          if (toolId === 'compress') {
            setResultMeta({ sizeStr: '425.8 KB', width: imgMeta?.width || 1280, height: imgMeta?.height || 720, savedPercent: 68 });
          } else if (toolId === 'super_resolution') {
            setResultMeta({ sizeStr: '11.4 MB', width: (imgMeta?.width || 1280) * 2, height: (imgMeta?.height || 720) * 2 });
          } else if (toolId === 'repair') {
            setResultMeta({ sizeStr: '1.2 MB', width: imgMeta?.width || 1280, height: imgMeta?.height || 720 });
          } else if (toolId === 'bg_remove') {
            setResultMeta({ sizeStr: '880.4 KB', width: imgMeta?.width || 1280, height: imgMeta?.height || 720 });
          } else {
            setResultMeta({ sizeStr: '950.2 KB', width: imgMeta?.width || 1280, height: imgMeta?.height || 720 });
          }
        }, 100);
      }
    }, 400);
    pipelineIntervalRef.current = interval;
  };

  const rotateLeft = () => setRotation(prev => (prev - 90) % 360);
  const rotateRight = () => setRotation(prev => (prev + 90) % 360);
  const toggleFlipH = () => setFlipH(prev => !prev);
  const toggleFlipV = () => setFlipV(prev => !prev);
  
  const zoomIn = () => setScale(prev => Math.min(prev + 0.25, 4));
  const zoomOut = () => setScale(prev => Math.max(prev - 0.25, 0.25));
  
  const handleReset = () => {
    setScale(1);
    setRotation(0);
    setFlipH(false);
    setFlipV(false);
  };

  // Launch simulated pipeline

  const handleDownloadResult = () => {
    // Standard trigger for local download simulation
    const link = document.createElement('a');
    link.href = activeDoc.url || '';
    link.download = `optimized_${activeTool}_${activeDoc.title}`;
    document.body.appendChild(link);
    link.click();
    document.body.removeChild(link);
  };

  if (!activeDoc.url) {
    return (
      <div className="flex-1 flex items-center justify-center p-8 text-zinc-400 dark:text-zinc-650 font-medium">
        暂无图片链接资源。
      </div>
    );
  }

  // Combined CSS transform string
  const transformStyle = {
    transform: `
      scale(${scale}) 
      rotate(${rotation}deg) 
      scaleX(${flipH ? -1 : 1}) 
      scaleY(${flipV ? -1 : 1})
    `,
    transition: 'transform 0.25s cubic-bezier(0.2, 0.8, 0.2, 1)',
    cursor: scale > 1 ? 'grab' : 'default',
  };

  return (
    <div 
      ref={containerRef}
      className="w-full h-full flex flex-col justify-between bg-zinc-50 dark:bg-[#0a0a0c] text-zinc-900 dark:text-zinc-200 relative min-h-0 select-none overflow-hidden"
    >
      {/* 1. Header Toolbar */}
      <div className="bg-white dark:bg-[var(--color-kb-panel)] px-4 h-[40px] flex items-center justify-between z-20 border-b border-zinc-200/80 dark:border-[var(--color-kb-panel-border)]/80 shrink-0 shadow-sm backdrop-blur-md">
        <div className="min-w-0 flex items-center gap-2.5">
          <div className="w-7 h-7 rounded-lg bg-blue-50 dark:bg-zinc-900 border border-blue-100 dark:border-zinc-800 flex items-center justify-center text-blue-500 shrink-0 shadow-sm">
            <ImageIcon size={13} strokeWidth={2.5} />
          </div>
          <div className="min-w-0">
            <h3 className="text-[12px] font-bold text-zinc-900 dark:text-zinc-100 truncate tracking-tight flex items-center gap-2" title={activeDoc.title}>
              <span>{activeDoc.title}</span>
            </h3>
            <p className="text-[10px] font-mono font-medium text-zinc-400 flex items-center gap-1.5 mt-0.5 leading-none uppercase tracking-wide whitespace-nowrap">
              {imgMeta && (
                <>
                  <span>原图尺寸: {imgMeta.width}x{imgMeta.height}</span>
                  <span className="opacity-40">/</span>
                  <span className="text-blue-500 dark:text-blue-400 font-bold">{Math.round(scale * 100)}% 缩放</span>
                  <span className="opacity-40">/</span>
                </>
              )}
              <span>画面: {containerSize.width}x{containerSize.height}</span>
            </p>
          </div>
        </div>

        {/* Action Controls Panel */}
        <div className="flex items-center gap-1 bg-[#fafafa] dark:bg-zinc-950 p-0.5 rounded-lg border border-zinc-200/80 dark:border-[var(--color-kb-panel-border)]/50 scale-95 origin-right">
          
          {/* Zoom controls */}
          <button
            onClick={zoomOut}
            className="p-1 px-1.5 rounded-md transition-all text-zinc-500 dark:text-zinc-400 hover:text-zinc-800 dark:hover:text-zinc-200 hover:bg-zinc-200/50 dark:hover:bg-zinc-900 text-[11px] font-bold flex items-center gap-1"
            title="缩小"
          >
            <ZoomOut size={13} strokeWidth={2.5} />
          </button>
          <button
            onClick={zoomIn}
            className="p-1 px-1.5 rounded-md transition-all text-zinc-500 dark:text-zinc-400 hover:text-zinc-800 dark:hover:text-zinc-200 hover:bg-zinc-200/50 dark:hover:bg-zinc-900 text-[11px] font-bold flex items-center gap-1"
            title="放大"
          >
            <ZoomIn size={13} strokeWidth={2.5} />
          </button>

          <div className="w-px h-3.5 bg-zinc-200 dark:bg-zinc-800/80 mx-0.5"></div>

          {/* Rotate controls */}
          <button
            onClick={rotateLeft}
            className="p-1 px-1.5 rounded-md transition-all text-zinc-500 dark:text-zinc-400 hover:text-zinc-800 dark:hover:text-zinc-200 hover:bg-zinc-200/50 dark:hover:bg-zinc-900 text-[11px] font-bold flex items-center gap-1"
            title="向左旋转90°"
          >
            <RotateCcw size={13} strokeWidth={2.5} />
          </button>
          <button
            onClick={rotateRight}
            className="p-1 px-1.5 rounded-md transition-all text-zinc-500 dark:text-zinc-400 hover:text-zinc-800 dark:hover:text-zinc-200 hover:bg-zinc-200/50 dark:hover:bg-zinc-900 text-[11px] font-bold flex items-center gap-1"
            title="向右旋转90°"
          >
            <RotateCw size={13} strokeWidth={2.5} />
          </button>

          <div className="w-px h-3.5 bg-zinc-200 dark:bg-zinc-800/80 mx-0.5"></div>

          {/* Flip controls */}
          <button
            onClick={toggleFlipH}
            className={`p-1 px-1.5 rounded-md transition-all text-[11px] font-bold flex items-center gap-1 ${
              flipH 
                ? 'bg-blue-100 dark:bg-blue-950/40 text-blue-600 dark:text-blue-400 shadow-sm' 
                : 'text-zinc-500 dark:text-zinc-400 hover:text-zinc-800 dark:hover:text-zinc-200 hover:bg-zinc-200/50'
            }`}
            title="水平翻转"
          >
            <FlipHorizontal size={13} strokeWidth={2.5} />
          </button>
          <button
            onClick={toggleFlipV}
            className={`p-1 px-1.5 rounded-md transition-all text-[11px] font-bold flex items-center gap-1 ${
              flipV 
                ? 'bg-blue-100 dark:bg-blue-950/40 text-blue-600 dark:text-blue-400 shadow-sm' 
                : 'text-zinc-500 dark:text-zinc-400 hover:text-zinc-800 dark:hover:text-zinc-200 hover:bg-zinc-200/50'
            }`}
            title="垂直翻转"
          >
            <FlipVertical size={13} strokeWidth={2.5} />
          </button>

          <div className="w-px h-3.5 bg-zinc-200 dark:bg-zinc-800/80 mx-0.5"></div>

          {/* Reset controls */}
          <button
            onClick={handleReset}
            className="p-1 px-2 rounded-md transition-all text-zinc-500 dark:text-zinc-400 hover:text-zinc-800 dark:hover:text-zinc-200 hover:bg-zinc-200/50 dark:hover:bg-zinc-900 text-[11px] font-semibold flex items-center gap-1"
            title="重置视图"
          >
            <RefreshCw size={12} strokeWidth={2.5} />
            <span className="hidden sm:inline text-[10px]">重置</span>
          </button>

        </div>
      </div>

      {/* 2. Interactive Image Stage Canvas (Split side by side when showing comparison) */}
      <div className="flex-1 w-full flex items-center justify-center p-2 sm:p-3 min-h-0 relative bg-black/[0.02] dark:bg-black/[0.15]">
        <div className="absolute inset-0 bg-grid-pattern opacity-[0.03] dark:opacity-[0.1] pointer-events-none" />
        
        {showComparison ? (
          <div className="w-full h-full flex flex-col md:flex-row gap-6 items-center justify-center max-w-5xl">
            {/* Original Card */}
            <div className="flex-1 w-full h-full flex flex-col items-center bg-white dark:bg-zinc-950 p-3 rounded-xl border border-zinc-200/60 dark:border-zinc-800/80 shadow-md">
              <span className="text-[10px] uppercase font-bold tracking-widest text-zinc-400 mb-2">处理前 original</span>
              <div className="flex-1 w-full min-h-0 relative overflow-hidden flex items-center justify-center bg-zinc-50 dark:bg-zinc-900 rounded border border-zinc-200/20">
                <img 
                  src={activeDoc.url} 
                  alt="Original" 
                  referrerPolicy="no-referrer"
                  className="w-full h-full object-contain select-none transition-all duration-350"
                />
              </div>
              <div className="mt-2 text-[10.5px] font-mono text-zinc-400">
                {imgMeta?.width}x{imgMeta?.height} (原始像素)
              </div>
            </div>

            {/* Arrow */}
            <div className="flex shrink-0 w-8 h-8 rounded-full bg-blue-500 text-white items-center justify-center shadow-lg animate-pulse">
              <ArrowRight size={16} strokeWidth={2.5} />
            </div>

            {/* Enhanced Output Card */}
            <div className="flex-1 w-full h-full flex flex-col items-center bg-white dark:bg-zinc-950 p-3 rounded-xl border border-zinc-200 dark:border-blue-500/30 shadow-xl relative overflow-hidden">
              <div className="absolute top-0 right-0 bg-blue-500 text-white font-extrabold text-[8.5px] px-2.5 py-0.5 rounded-bl uppercase tracking-wider shadow z-10">
                智能算法优化
              </div>
              <span className="text-[10px] uppercase font-bold tracking-widest text-blue-500 mb-2">处理后 optimized</span>
              <div className="flex-1 w-full min-h-0 relative overflow-hidden flex items-center justify-center bg-zinc-50 dark:bg-zinc-900 rounded border border-zinc-200/20">
                {activeTool === 'bg_remove' ? (
                  <div className="relative w-full h-full p-2 bg-transparent bg-checkered-grid flex items-center justify-center">
                    <img 
                      src={activeDoc.url} 
                      alt="Segmented output" 
                      referrerPolicy="no-referrer"
                      className="w-full h-full object-contain select-none opacity-90 contrast-125 saturate-110 drop-shadow-[0_12px_24px_rgba(0,0,0,0.3)]"
                      style={{ clipPath: 'polygon(5% 5%, 95% 5%, 95% 95%, 5% 95%)' }}
                    />
                  </div>
                ) : (
                  <img 
                    src={activeDoc.url} 
                    alt="Optimized result" 
                    referrerPolicy="no-referrer"
                    className={`w-full h-full object-contain select-none transition-all duration-350 ${activeTool === 'repair' ? 'sepia-0 contrast-105 saturate-100 filter-grayscale-0 brightness-105 font-medium' : 'blur-none saturate-[1.05]'}`}
                  />
                )}
              </div>
              <div className="mt-2 flex items-center gap-2 text-[10.5px] font-mono text-zinc-500">
                <span className="font-bold text-blue-500">{resultMeta?.width}x{resultMeta?.height}</span>
              </div>
            </div>
          </div>
        ) : (
          <div className="relative w-full h-full overflow-hidden flex items-center justify-center">
            <img 
              ref={imgRef}
              src={activeDoc.url} 
              alt={activeDoc.title} 
              onLoad={handleImageLoad}
              style={{
                ...transformStyle,
                width: '100%',
                height: '100%',
              }}
              referrerPolicy="no-referrer"
              className="w-full h-full object-contain drop-shadow-md select-none rounded border border-zinc-200/20 shadow-lg"
            />
          </div>
        )}
      </div>

      {/* 3. Bottom AI tools — preview/demo only until media task APIs ship for image processing */}
      {aiToolsEnabled ? (
      <div className="h-[48px] border-t border-zinc-200/80 dark:border-zinc-900/80 bg-white dark:bg-[#09090b] flex items-center justify-between px-4 z-20 shrink-0 text-xs font-medium">
        {/* Left Side: Tool Buttons */}
        <div className="flex items-center gap-1.5 overflow-x-auto no-scrollbar py-1">
          <span className="text-[10px] text-zinc-400 dark:text-zinc-500 font-bold uppercase tracking-wider whitespace-nowrap mr-1 flex items-center gap-1">
            <Cpu size={12} className="text-blue-500" />
            AI 增强:
          </span>
          {[
            { id: 'compress', label: '压缩', icon: Crop },
            { id: 'super_resolution', label: '超分', icon: Sparkles },
            { id: 'repair', label: '修复', icon: RefreshCw },
            { id: 'bg_remove', label: '抠图', icon: FlipHorizontal }
          ].map((tool) => {
            const ToolIcon = tool.icon;
            const isSelected = activeTool === tool.id;
            return (
              <button
                key={tool.id}
                onClick={() => {
                  if (isProcessing) return;
                  if (activeTool === tool.id) {
                    setActiveTool(null);
                    setProcessSuccess(false);
                    setShowComparison(false);
                  } else {
                    startToolPipeline(tool.id as ToolType);
                  }
                }}
                disabled={isProcessing}
                className={`h-7 px-2.5 rounded-lg transition-all flex items-center gap-1 text-[11px] font-bold border whitespace-nowrap outline-none select-none ${
                  isSelected 
                    ? 'bg-blue-500/10 border-blue-500/30 text-blue-600 dark:text-blue-400' 
                    : 'border-zinc-200 dark:border-zinc-800/80 text-zinc-500 hover:bg-zinc-200/30 dark:hover:bg-zinc-900/60'
                }`}
              >
                <ToolIcon size={12} strokeWidth={2.5} className={isSelected ? 'text-blue-500' : ''} />
                <span>{tool.label}</span>
              </button>
            );
          })}

          {/* 更多工具 button with popup */}
          <div className="relative">
            <button
              type="button"
              onClick={() => {
                if (!isProcessing) {
                  setShowMoreTools(!showMoreTools);
                }
              }}
              disabled={isProcessing}
              className={`h-7 px-2.5 rounded-lg transition-all flex items-center gap-1 text-[11px] font-bold border whitespace-nowrap outline-none select-none ${
                showMoreTools 
                  ? 'bg-indigo-500/10 border-indigo-500/30 text-indigo-600 dark:text-indigo-400' 
                  : 'border-zinc-200 dark:border-zinc-800/80 text-zinc-500 hover:bg-zinc-200/30 dark:hover:bg-zinc-900/60'
              } disabled:opacity-50`}
            >
              <Wand2 size={12} strokeWidth={2.5} />
              <span>更多工具</span>
            </button>
            
            {showMoreTools && (
              <div className="absolute bottom-[36px] left-0 z-[100] bg-white dark:bg-zinc-950 border border-zinc-200 dark:border-zinc-850 rounded-xl shadow-2xl p-1.5 w-44 flex flex-col gap-1 text-xs animate-in slide-in-from-bottom-2 duration-150">
                {[
                  { id: 'art_filter', label: '滤镜增强', icon: Sliders },
                  { id: 'watermark_erase', label: '水印擦除', icon: Hammer },
                  { id: 'smart_crop', label: '智能裁切', icon: Crop },
                  { id: 'format_convert', label: '格式转换', icon: RefreshCw },
                  { id: 'privacy_blur', label: '隐私打码', icon: Eye }
                ].map((tool) => {
                  const ToolIcon = tool.icon;
                  const isSelected = activeTool === tool.id;
                  return (
                    <button
                      key={tool.id}
                      onClick={() => {
                        setShowMoreTools(false);
                        startToolPipeline(tool.id as ToolType);
                      }}
                      className={`w-full px-3 py-2 rounded-lg text-left font-bold flex items-center gap-2.5 transition-colors ${
                        isSelected
                          ? 'bg-blue-500/10 text-blue-600 dark:text-blue-400'
                          : 'text-zinc-600 dark:text-zinc-300 hover:bg-zinc-100 dark:hover:bg-zinc-900/65'
                      }`}
                    >
                      <ToolIcon size={13} strokeWidth={2.5} className={isSelected ? 'text-blue-500' : 'text-zinc-450'} />
                      <span>{tool.label}</span>
                    </button>
                  );
                })}
              </div>
            )}
          </div>
        </div>

        {/* Middle Side: Dynamic inline state */}
        <div className="flex-1 flex items-center justify-end md:justify-center px-4 font-mono text-[10.5px] truncate">
          {isProcessing ? (
            <div className="flex items-center gap-1.5 text-blue-500 animate-pulse">
              <span className="inline-block w-2.5 h-2.5 rounded-full border-2 border-blue-500 border-t-transparent animate-spin"></span>
              <span>处理中 {Math.round(processProgress)}%...</span>
            </div>
          ) : processSuccess && resultMeta ? (
            <div className="hidden md:flex items-center gap-1.5 text-[10.5px] text-zinc-400 dark:text-zinc-500 font-sans">
              <span>尺寸: <strong className="text-zinc-700 dark:text-zinc-300 font-mono font-bold">{resultMeta.width}x{resultMeta.height}</strong></span>
              <span>•</span>
              <span>体积: <strong className="text-zinc-700 dark:text-zinc-300 font-mono font-bold">{resultMeta.sizeStr}</strong></span>
              {resultMeta.savedPercent && (
                <>
                  <span>•</span>
                  <span className="text-emerald-500 font-extrabold bg-emerald-500/10 px-1 rounded-md">省 {resultMeta.savedPercent}%</span>
                </>
              )}
            </div>
          ) : null}
        </div>

        {/* Right Side: Quick Action Buttons */}
        <div className="flex items-center gap-1.5 shrink-0">
          {processSuccess ? (
            <>
              <button
                onClick={() => setShowComparison(!showComparison)}
                className="h-7 px-2.5 bg-zinc-100 dark:bg-zinc-900 text-zinc-700 dark:text-zinc-300 text-[11px] font-bold rounded-lg border border-zinc-200 dark:border-zinc-800 hover:bg-zinc-200/40 dark:hover:bg-zinc-800/80 flex items-center gap-1 transition-all"
                title={showComparison ? '退出双屏对比' : '双屏分屏对比'}
              >
                <Eye size={12} strokeWidth={2.5} />
                <span className="hidden sm:inline">分屏对比</span>
              </button>
              
              <button
                onClick={handleDownloadResult}
                className="h-7 px-2.5 bg-zinc-100 dark:bg-zinc-900 border border-zinc-200 dark:border-zinc-800 hover:bg-zinc-200/40 dark:hover:bg-zinc-800/80 text-zinc-700 dark:text-zinc-300 text-[11px] font-bold rounded-lg flex items-center gap-1 transition-all"
                title="保存优化后图像至本地"
              >
                <Download size={11} strokeWidth={2.5} />
                <span>下载</span>
              </button>

              <button
                onClick={() => setShowSaveAsModal(true)}
                className="h-7 px-2.5 bg-blue-500 hover:bg-blue-600 text-white text-[11px] font-bold rounded-lg flex items-center gap-1 transition-all focus:outline-none"
                title="另存为至知识库"
              >
                <FolderOutput size={12} strokeWidth={2.5} />
                <span>另存为</span>
              </button>
            </>
          ) : (
            <button
              onClick={() => {
                const link = document.createElement('a');
                link.href = activeDoc.url || '';
                link.download = activeDoc.title || 'download.jpg';
                document.body.appendChild(link);
                link.click();
                document.body.removeChild(link);
              }}
              className="h-7 px-2.5 bg-zinc-100 dark:bg-zinc-900 border border-zinc-200 dark:border-zinc-800 hover:bg-zinc-200/40 dark:hover:bg-zinc-800/80 text-zinc-700 dark:text-zinc-300 text-[11px] font-bold rounded-lg flex items-center gap-1 transition-all"
            >
              <Download size={11} strokeWidth={2.5} />
              <span>下载原图</span>
            </button>
          )}
        </div>
      </div>
      ) : (
      <div className="h-[40px] border-t border-zinc-200/80 dark:border-zinc-900/80 bg-white dark:bg-[#09090b] flex items-center px-4 z-20 shrink-0 text-[11px] text-zinc-500">
        AI 图像处理仅在离线预览模式可用；生产环境请使用 Drive 原图或后续媒体任务 API。
      </div>
      )}

      {/* Save As Modal Integration */}
      {showSaveAsModal && (
        <MoveCopyModal
          action="save_as"
          item={{
            id: 'save-as-img-doc',
            title: activeDoc.title,
            type: 'image'
          }}
          activeKb={activeKb || null}
          onClose={() => setShowSaveAsModal(false)}
          onSubmit={handleSaveAsSubmit}
        />
      )}
    </div>
  );
}
