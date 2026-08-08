import React, { useState, useRef, useEffect, useMemo } from 'react';
import { 
  Video, Play, Pause, Sliders, Sparkles, Tv, Crop, Expand, Shrink,
  Cpu, Music, Download, Wand2, Scissors, Film, FolderOutput
} from 'lucide-react';
import { DocumentMeta, KnowledgeBase, DocumentService } from '../../services/document';
import { shouldUseKnowledgebaseDemoFallback } from 'sdkwork-knowledgebase-pc-core';
import { MoveCopyModal } from '../../MoveCopyModal';
import { useTranslation } from 'react-i18next';
export interface VideoPlayerProps {
  activeDoc: DocumentMeta;
  activeKb?: KnowledgeBase | null;
  onUpdateDocs?: () => void;
  onToastMessage?: (msg: string | null) => void;
}

type VideoToolType = 'compress' | 'interpolate' | 'extract_audio' | 'convert_format' | 'smart_subtitle' | 'video_denoise' | 'video_stabilize' | 'video_color_grade' | 'intro_trim' | null;

export function VideoPlayer({ activeDoc, activeKb, onUpdateDocs, onToastMessage }: VideoPlayerProps) {
  const { t } = useTranslation(['editor', 'common']);

  const containerRef = useRef<HTMLDivElement>(null);
  const videoRef = useRef<HTMLVideoElement>(null);
  const [containerSize, setContainerSize] = useState({ width: 0, height: 0 });
  const [videoMeta, setVideoMeta] = useState<{ width: number; height: number } | null>(null);
  
  const [isPlaying, setIsPlaying] = useState(false);
  const [playbackRate, setPlaybackRate] = useState(1);
  const [, setDuration] = useState(0);
  const [, setCurrentTime] = useState(0);

  // Default mode is 'adaptive' which maintains ratio perfectly and adaptively maximizes play area
  const [viewMode, setViewMode] = useState<'adaptive' | '16-9' | '4-3' | 'stretch' | 'zoom-cover'>('adaptive');

  // Bottom Toolkit states
  const [activeTool, setActiveTool] = useState<VideoToolType>(null);
  const [isProcessing, setIsProcessing] = useState<boolean>(false);
  const [, setProcessLogs] = useState<string[]>([]);
  const [processProgress, setProcessProgress] = useState<number>(0);
  const [processSuccess, setProcessSuccess] = useState<boolean>(false);
  const [resultMeta, setResultMeta] = useState<{ sizeStr: string; bitrate: string; fps: number; savedPercent?: number } | null>(null);

  // Popup tool menu and save-as state
  const [showMoreTools, setShowMoreTools] = useState<boolean>(false);
  const [showSaveAsModal, setShowSaveAsModal] = useState<boolean>(false);

  const handleSaveAsSubmit = async (targetKbId: string, targetFolderId: string | null) => {
    try {
      const toolLabelMap: Record<string, string> = {
        compress: '压缩版',
        interpolate: '超帧插值版',
        extract_audio: '音频提取伴奏',
        convert_format: '格式转换版',
        smart_subtitle: '智能字幕版',
        video_denoise: '高清降噪版',
        video_stabilize: '超强防抖版',
        video_color_grade: '极智调色版',
        intro_trim: '片头裁剪版'
      };
      
      const suffix = toolLabelMap[activeTool || ''] || '优化版';
      const lastDotIndex = activeDoc.title.lastIndexOf('.');
      const titleWithoutExt = lastDotIndex !== -1 ? activeDoc.title.substring(0, lastDotIndex) : activeDoc.title;
      const ext = lastDotIndex !== -1 ? activeDoc.title.substring(lastDotIndex) : '.mp4';
      
      const targetTitle = `${titleWithoutExt}_${suffix}${ext}`;

      await DocumentService.createDocument({
        title: targetTitle,
        type: 'video',
        kbId: targetKbId,
        parentId: targetFolderId,
        url: activeDoc.url,
        content: activeDoc.content || '',
        author: 'Me'
      });

      if (onUpdateDocs) {
        onUpdateDocs();
      }

      if (onToastMessage) {
        onToastMessage(`另存为成功！已将视频文件保存至目标知识库目录。`);
      }
      setShowSaveAsModal(false);
    } catch (e) {
      console.error(e);
      if (onToastMessage) {
        onToastMessage('另存为失败，请重试');
      }
    }
  };

  // Tool settings
  // 1. Compression quality (Mbps)
  const [compressBitrate] = useState<number>(3.5); // target Mbps
void (useState<'H.264 (AVC)' | 'H.265 (HEVC)' | 'AV1 Ultra'>('H.264 (AVC)'));
  // 2. Interpolation / resolution upscaling
void (useState<'60 FPS' | '120 FPS'>('60 FPS'));
void (useState<'Topaz Apollo (插帧)' | 'DAIN Superflow'>('Topaz Apollo (插帧)'));
  // 3. Audio extractor settings
  const [audioFormat] = useState<'mp3' | 'aac' | 'flac'>('mp3');
void (useState<string>('320 kbps'));
  // 4. Format converter / Trimmer
  const [convertTarget] = useState<'mp4' | 'webm' | 'gif'>('gif');
void (useState<number>(15));

  const processIntervalRef = useRef<ReturnType<typeof setInterval> | null>(null);
  const processTimeoutRef = useRef<ReturnType<typeof setTimeout> | null>(null);

  // AI video toolkit is a preview/demo simulation only, gated the same way as
  // ImageViewer: it never renders in production unless demo mode is explicitly enabled.
  const aiToolsEnabled = shouldUseKnowledgebaseDemoFallback();

  useEffect(() => {
    return () => {
      if (processIntervalRef.current) {
        clearInterval(processIntervalRef.current);
        processIntervalRef.current = null;
      }
      if (processTimeoutRef.current) {
        clearTimeout(processTimeoutRef.current);
        processTimeoutRef.current = null;
      }
    };
  }, []);

  // Watch container dimensions using ResizeObserver
  useEffect(() => {
    if (!containerRef.current) return;

    const observer = new ResizeObserver((entries) => {
      if (!entries || entries.length === 0) return;
      const { width, height } = entries[0].contentRect;
      setContainerSize({ width, height });
    });

    observer.observe(containerRef.current);
    return () => {
      observer.disconnect();
    };
  }, []);

  // Reset tool active state whenever source changes
  useEffect(() => {
    setActiveTool(null);
    setIsProcessing(false);
    setProcessProgress(0);
    setProcessLogs([]);
    setProcessSuccess(false);
    setResultMeta(null);
  }, [activeDoc.id]);

  const startVideoToolPipeline = (toolId: VideoToolType) => {
    // Demo simulation only — never runs in production.
    if (isProcessing || !aiToolsEnabled) return;
    setActiveTool(toolId);
    setIsProcessing(true);
    setProcessProgress(0);
    setProcessSuccess(false);
    setProcessLogs([]);

    const videoLogs: Record<string, string[]> = {
      compress: ['加载 WASM 编码内核...', '估计GOP配置...', '进行比特率二次压制...'],
      interpolate: ['启动 AI 矢量位移场...', '光流特征对齐计算...', '生成流畅插值插帧...'],
      extract_audio: ['音视频分流(Demuxing)...', '提取高保真波形声码...', '降噪压缩输出转码...'],
      convert_format: ['加载高品质 GIF 帧映射器...', '计算抖动噪色调节...', '导出运动GIF动图...'],
      smart_subtitle: ['收集高精度语调声波纹路...', '调用ASR端到端神经网络分词...', '渲染软字幕轴并写入对齐...'],
      video_denoise: ['对齐多帧时域噪点纹理...', '启动空域AI双边降噪过滤器...', '微调暗部增益与清晰度...'],
      video_stabilize: ['平滑摄像机空间平移旋转震抖...', '补偿全局三维形变失真比例...', '自适应四周裁切边界对齐...'],
      video_color_grade: ['应用影视级达芬奇LUT色彩映射表...', '自动分析场景白平衡与亮度空间...', '提升视频对比度与暗部纯净度...'],
      intro_trim: ['检测视频声画标志性音律高潮与灰度...', '确定正片切入起码帧率点...', '精准自动裁剪前奏片头...']
    };

    const currentLogs = videoLogs[toolId || 'compress'] || [];
    let logIndex = 0;

    if (processIntervalRef.current) {
      clearInterval(processIntervalRef.current);
    }
    if (processTimeoutRef.current) {
      clearTimeout(processTimeoutRef.current);
    }

    const interval = setInterval(() => {
      if (logIndex < currentLogs.length) {
        setProcessProgress(_prev => Math.round(Math.min(((logIndex + 1) / currentLogs.length) * 100, 100)));
        logIndex++;
      } else {
        clearInterval(interval);
        processIntervalRef.current = null;
        processTimeoutRef.current = setTimeout(() => {
          setIsProcessing(false);
          setProcessSuccess(true);
          setResultMeta({
            sizeStr: toolId === 'compress' ? '4.8 MB' : toolId === 'extract_audio' ? '980 KB' : '1.4 MB',
            bitrate: toolId === 'compress' ? `${compressBitrate} Mbps` : '320 kbps',
            fps: toolId === 'interpolate' ? 60 : 30,
            savedPercent: toolId === 'compress' ? 71 : undefined
          });
          processTimeoutRef.current = null;
        }, 100);
      }
    }, 400);
    processIntervalRef.current = interval;
  };

  // Recalculate dimensions to fit perfectly inside the parent container while keeping ratio bounds
  const calculatedStyle = useMemo(() => {
    const { width: cWidth, height: cHeight } = containerSize;
    if (cWidth === 0 || cHeight === 0) {
      return { width: '100%', height: '100%' };
    }

    if (viewMode === 'stretch' || viewMode === 'zoom-cover') {
      return { width: '100%', height: '100%' };
    }

    let vRatio = 16 / 9;
    if (viewMode === 'adaptive' && videoMeta) {
      vRatio = videoMeta.width / videoMeta.height;
    } else if (viewMode === '16-9') {
      vRatio = 16 / 9;
    } else if (viewMode === '4-3') {
      vRatio = 4 / 3;
    }

    const cRatio = cWidth / cHeight;
    let targetWidth = 0;
    let targetHeight = 0;

    if (vRatio > cRatio) {
      // Width limited
      targetWidth = cWidth;
      targetHeight = cWidth / vRatio;
    } else {
      // Height limited
      targetWidth = cHeight * vRatio;
      targetHeight = cHeight;
    }

    return {
      width: `${Math.floor(targetWidth)}px`,
      height: `${Math.floor(targetHeight)}px`,
      transition: 'width 0.2s ease-out, height 0.2s ease-out'
    };
  }, [containerSize, videoMeta, viewMode]);

  const objectFitClass = useMemo(() => {
    switch (viewMode) {
      case 'stretch':
        return 'object-fill';
      case 'zoom-cover':
        return 'object-cover';
      default:
        // For adaptive, 16-9, 4-3, the container wrapper is already perfectly sized to the exact aspect ratio,
        // so filling it with object-cover fits perfectly with zero letterboxes or extra black bar paddings inside.
        return 'object-cover';
    }
  }, [viewMode]);

  const handleLoadedMetadata = (e: React.SyntheticEvent<HTMLVideoElement, Event>) => {
    const video = e.currentTarget;
    if (video) {
      setVideoMeta({
        width: video.videoWidth || 1920,
        height: video.videoHeight || 1080,
      });
      setDuration(video.duration || 0);
    }
  };

  const handleTimeUpdate = (e: React.SyntheticEvent<HTMLVideoElement, Event>) => {
    const video = e.currentTarget;
    if (video) {
      setCurrentTime(video.currentTime || 0);
    }
  };

  const togglePlay = () => {
    if (videoRef.current) {
      if (isPlaying) {
        videoRef.current.pause();
        setIsPlaying(false);
      } else {
        videoRef.current.play()
          .then(() => setIsPlaying(true))
          .catch(() => {});
      }
    }
  };

  const handleSpeedChange = (rate: number) => {
    setPlaybackRate(rate);
    if (videoRef.current) {
      videoRef.current.playbackRate = rate;
    }
  };

  const currentRatioLabel = useMemo(() => {
    if (viewMode === 'adaptive') {
      if (videoMeta) {
        const gcd = (a: number, b: number): number => b === 0 ? a : gcd(b, a % b);
        const divisor = gcd(videoMeta.width, videoMeta.height);
        const wRatio = Math.round(videoMeta.width / divisor);
        const hRatio = Math.round(videoMeta.height / divisor);
        return `源比例 (${wRatio}:${hRatio})`;
      }
      return '自适应原始';
    }
    if (viewMode === '16-9') return '宽屏 16:9';
    if (viewMode === '4-3') return '经典 4:3';
    if (viewMode === 'stretch') return '拉伸撑满';
    if (viewMode === 'zoom-cover') return '沉浸裁剪';
    return '';
  }, [viewMode, videoMeta]);

  return (
    <div 
      id={`video-player-${activeDoc.id}`} 
      ref={containerRef}
      className="w-full h-full bg-white dark:bg-[#0a0a0c] flex flex-col justify-between text-zinc-900 dark:text-zinc-200 relative min-h-0 select-none overflow-hidden"
    >
      {/* 1. Dyn Headbar Overlay */}
      {activeDoc.url && (
        <div className="bg-white dark:bg-[var(--color-kb-panel)] px-4 h-[40px] flex items-center justify-between z-20 border-b border-zinc-200/80 dark:border-[var(--color-kb-panel-border)]/80 shrink-0 shadow-sm backdrop-blur-md">
          <div className="min-w-0 flex items-center gap-2.5">
            <div className="w-7 h-7 rounded-lg bg-orange-50 dark:bg-zinc-900 border border-orange-100 dark:border-zinc-800 flex items-center justify-center text-orange-500 shrink-0 shadow-sm">
              <Video size={13} className={isPlaying ? 'animate-pulse' : ''} strokeWidth={2.5} />
            </div>
            <div className="min-w-0">
              <h3 className="text-[12px] font-bold text-zinc-900 dark:text-zinc-100 truncate tracking-tight flex items-center gap-2" title={activeDoc.title}>
                <span>{activeDoc.title}</span>
              </h3>
              <p className="text-[10px] font-mono font-medium text-zinc-400 flex items-center gap-1.5 mt-0.5 leading-none uppercase tracking-wide whitespace-nowrap">
                {videoMeta && (
                  <>
                    <span>源尺寸: {videoMeta.width}x{videoMeta.height}</span>
                    <span className="opacity-40">/</span>
                    <span className="text-orange-500 dark:text-orange-400 font-bold">{currentRatioLabel}</span>
                    <span className="opacity-40">/</span>
                  </>
                )}
                <span>画面: {containerSize.width}x{containerSize.height}</span>
              </p>
            </div>
          </div>

          {/* Quick ratio action bars */}
          <div className="flex items-center gap-1 bg-[#fafafa] dark:bg-zinc-950 p-0.5 rounded-lg border border-zinc-200/80 dark:border-zinc-800 scale-95 origin-right">
            {[
              { id: 'adaptive', label: t('adaptive', { ns: 'common' }), icon: Shrink },
              { id: '16-9', label: '16:9', icon: Tv },
              { id: '4-3', label: '4:3', icon: Tv },
              { id: 'stretch', label: t('stretch', { ns: 'common' }), icon: Expand },
              { id: 'zoom-cover', label: t('zoomCover', { ns: 'common' }), icon: Crop }
            ].map((option) => {
              const Icon = option.icon;
              const isSelected = viewMode === option.id;
              return (
                <button
                  key={option.id}
                  onClick={() => setViewMode(option.id as any)}
                  className={`px-2 py-0.5 rounded-md transition-all text-[11px] font-bold flex items-center gap-1 outline-none ${
                    isSelected 
                      ? 'bg-white dark:bg-[var(--color-kb-editor)] text-orange-600 dark:text-orange-400 shadow-sm ring-1 ring-black/5 font-semibold' 
                      : 'text-zinc-400 dark:text-zinc-500 hover:text-zinc-800 dark:hover:text-zinc-300'
                  }`}
                  title={option.label}
                >
                  <Icon size={12} strokeWidth={isSelected ? 2.5 : 2} />
                  <span>{option.label}</span>
                </button>
              );
            })}
          </div>
        </div>
      )}

      {/* 2. Resizing Video Canvas viewport */}
      <div className="flex-1 w-full flex items-center justify-center relative bg-zinc-50/50 dark:bg-[#060608] min-h-0 p-2 sm:p-3">
        {activeDoc.url ? (
          <div 
            style={calculatedStyle}
            className="relative bg-black rounded-2xl overflow-hidden shadow-2xl flex items-center justify-center group ring-1 ring-zinc-200/50 dark:ring-transparent"
          >
            <video 
              ref={videoRef}
              src={activeDoc.url}
              controls
              crossOrigin="anonymous"
              onLoadedMetadata={handleLoadedMetadata}
              onTimeUpdate={handleTimeUpdate}
              onPlay={() => setIsPlaying(true)}
              onPause={() => setIsPlaying(false)}
              className={`w-full h-full ${objectFitClass}`}
            />

            {/* Micro Interaction Playback HUD Toggle overlay */}
            <div 
              onClick={togglePlay}
              className="absolute inset-0 z-10 flex items-center justify-center bg-black/10 dark:bg-black/35 opacity-0 group-hover:opacity-100 transition-opacity duration-300 cursor-pointer"
            >
              <div className="w-14 h-14 rounded-full bg-white/95 dark:bg-zinc-950/85 border border-zinc-200/50 dark:border-zinc-800 flex items-center justify-center text-orange-500 shadow-2xl transform scale-90 group-hover:scale-100 transition-transform duration-300 pointer-events-none backdrop-blur-sm">
                {isPlaying ? <Pause size={20} fill="currentColor" /> : <Play size={20} className="ml-1" fill="currentColor" />}
              </div>
            </div>
          </div>
        ) : (
          <div className="flex flex-col items-center p-8 bg-white dark:bg-[#0b0b0d] rounded-2xl border-2 border-zinc-100 dark:border-zinc-900 max-w-sm text-center shadow-sm">
            <div className="w-16 h-16 rounded-2xl bg-zinc-50 dark:bg-zinc-900 border border-zinc-200/80 dark:border-zinc-800 flex items-center justify-center text-zinc-300 dark:text-zinc-500 mb-5 relative overflow-hidden shadow-inner">
               <Video size={28} className="animate-pulse" />
            </div>
            <h4 className="text-[11px] font-extrabold text-zinc-500 dark:text-zinc-300 uppercase tracking-widest font-mono mb-2">
              {t('videoPlayerOffline', { ns: 'common' })}
            </h4>
            <p className="text-[12.5px] font-medium text-zinc-400 leading-relaxed max-w-xs">
              {t('noVideoSource', { ns: 'common' })}
            </p>
          </div>
        )}
      </div>

       {/* 3. Footer Console for Speed adjustments & Premium AI Media Toolkit - Compact Single Line */}
      {activeDoc.url && (
        <div className="h-[48px] bg-white dark:bg-[#09090b] border-t border-zinc-200/80 dark:border-zinc-900/70 px-4 flex items-center justify-between gap-4 text-xs font-semibold z-20 shrink-0 select-none">
          {/* Left Side: Playback Speed */}
          <div className="flex items-center gap-2 shrink-0">
            <span className="text-[10px] text-zinc-400 dark:text-zinc-500 font-mono flex items-center gap-1 uppercase tracking-wider font-extrabold whitespace-nowrap">
              <Sliders size={12} className="text-orange-500" strokeWidth={2.5} />
              倍速:
            </span>
            <div className="flex items-center bg-[#fafafa] dark:bg-zinc-950 rounded-lg p-0.5 border border-zinc-200/80 dark:border-zinc-850 shadow-sm">
              {[0.75, 1.0, 1.25, 1.5, 2.0].map((rate) => (
                <button 
                  key={rate}
                  onClick={() => handleSpeedChange(rate)}
                  className={`px-2 py-0.5 text-[10px] font-bold rounded transition-all outline-none whitespace-nowrap ${
                    playbackRate === rate 
                      ? 'bg-white text-orange-600 dark:bg-orange-600/15 dark:text-orange-400 border border-zinc-200/80 dark:border-orange-500/20 shadow-xs' 
                      : 'text-zinc-400 dark:text-zinc-500 hover:text-zinc-700 dark:hover:text-zinc-300 hover:bg-zinc-200/20'
                  }`}
                >
                  {rate}x
                </button>
              ))}
            </div>
          </div>

          {/* Middle Side: AI Toolkit — preview/demo only until real video processing APIs ship.
              Never rendered in production unless demo mode is explicitly enabled. */}
          {aiToolsEnabled && (
          <div className="flex-1 flex items-center justify-end md:justify-center gap-2 overflow-x-auto no-scrollbar scroll-smooth">
            <span className="text-[10px] text-zinc-400 dark:text-zinc-500 font-bold uppercase tracking-wider whitespace-nowrap mr-1 flex items-center gap-1">
              <Cpu size={12} className="text-orange-500" />
              AI视频:
            </span>
            {[
              { id: 'compress', label: '压缩', icon: Film },
              { id: 'interpolate', label: '60帧插帧', icon: Sparkles },
              { id: 'extract_audio', label: '提音轨', icon: Music },
              { id: 'convert_format', label: '转GIF', icon: Scissors }
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
                    } else {
                      startVideoToolPipeline(tool.id as VideoToolType);
                    }
                  }}
                  disabled={isProcessing}
                  className={`h-7 px-2.5 rounded-lg transition-all flex items-center gap-1 text-[11px] font-bold border whitespace-nowrap outline-none select-none ${
                    isSelected 
                      ? 'bg-orange-500/10 border-orange-500/30 text-orange-600 dark:text-orange-400' 
                      : 'border-zinc-200 dark:border-zinc-800/80 text-zinc-500 hover:bg-zinc-200/30 dark:hover:bg-zinc-900/60'
                  }`}
                >
                  <ToolIcon size={12} strokeWidth={2.5} className={isSelected ? 'text-orange-500' : ''} />
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
                <div className="absolute bottom-[36px] right-0 z-[100] bg-white dark:bg-zinc-950 border border-zinc-200 dark:border-zinc-850 rounded-xl shadow-2xl p-1.5 w-44 flex flex-col gap-1 text-xs animate-in slide-in-from-bottom-2 duration-150">
                  {[
                    { id: 'smart_subtitle', label: 'AI智能字幕', icon: Tv },
                    { id: 'video_denoise', label: '视频降噪', icon: Sliders },
                    { id: 'video_stabilize', label: '视频防抖', icon: Film },
                    { id: 'video_color_grade', label: 'AI智能调色', icon: Sparkles },
                    { id: 'intro_trim', label: 'AI片头切除', icon: Scissors }
                  ].map((tool) => {
                    const ToolIcon = tool.icon;
                    const isSelected = activeTool === tool.id;
                    return (
                      <button
                        key={tool.id}
                        onClick={() => {
                          setShowMoreTools(false);
                          startVideoToolPipeline(tool.id as VideoToolType);
                        }}
                        className={`w-full px-3 py-2 rounded-lg text-left font-bold flex items-center gap-2.5 transition-colors ${
                          isSelected
                            ? 'bg-orange-500/10 text-orange-600 dark:text-orange-400'
                            : 'text-zinc-600 dark:text-zinc-300 hover:bg-zinc-100 dark:hover:bg-zinc-900/65'
                        }`}
                      >
                        <ToolIcon size={13} strokeWidth={2.5} className={isSelected ? 'text-orange-500' : 'text-zinc-450'} />
                        <span>{tool.label}</span>
                      </button>
                    );
                  })}
                </div>
              )}
            </div>
          </div>
          )}

          {/* Right Side: Execution feedback or download buttons */}
          <div className="flex items-center gap-1.5 shrink-0 ml-2">
            {isProcessing ? (
              <div className="flex items-center gap-1 text-orange-500 animate-pulse text-[10.5px]">
                <span className="inline-block w-2.5 h-2.5 rounded-full border-2 border-orange-500 border-t-transparent animate-spin"></span>
                <span>{processProgress}%</span>
              </div>
            ) : processSuccess && resultMeta ? (
              <div className="flex items-center gap-1.5 font-sans">
                <span className="hidden lg:inline text-[9.5px] text-zinc-400 whitespace-nowrap">
                  已生成 ({resultMeta.sizeStr})
                </span>
                
                <button
                  onClick={() => {
                    const link = document.createElement('a');
                    link.href = activeDoc.url || '';
                    link.download = `optimized_${activeTool}_${activeDoc.title || 'video'}.${activeTool === 'extract_audio' ? audioFormat : activeTool === 'convert_format' ? convertTarget : 'mp4'}`;
                    document.body.appendChild(link);
                    link.click();
                    document.body.removeChild(link);
                  }}
                  className="h-7 px-2.5 bg-zinc-105 border border-zinc-205 dark:bg-zinc-900 dark:border-zinc-800 hover:bg-zinc-200/40 dark:hover:bg-zinc-800/85 text-zinc-700 dark:text-zinc-300 text-[11px] font-bold rounded-lg flex items-center gap-1 transition-all"
                  title="下载视频到本地"
                >
                  <Download size={11} strokeWidth={2.5} />
                  <span>下载</span>
                </button>

                <button
                  onClick={() => setShowSaveAsModal(true)}
                  className="h-7 px-2.5 bg-blue-500 hover:bg-blue-600 text-white text-[11px] font-bold rounded-lg flex items-center gap-1 transition-all focus:outline-none"
                  title="另存为到选定知识库"
                >
                  <FolderOutput size={12} strokeWidth={2.5} />
                  <span>另存为</span>
                </button>
              </div>
            ) : (
              <button
                onClick={() => {
                  const link = document.createElement('a');
                  link.href = activeDoc.url || '';
                  link.download = activeDoc.title || 'download.mp4';
                  document.body.appendChild(link);
                  link.click();
                  document.body.removeChild(link);
                }}
                className="h-7 px-2.5 bg-zinc-100 dark:bg-zinc-900 border border-zinc-200 dark:border-zinc-800 hover:bg-zinc-200/40 dark:hover:bg-zinc-800/80 text-zinc-700 dark:text-zinc-300 text-[11px] font-bold rounded-lg flex items-center gap-1 transition-all"
              >
                <Download size={11} strokeWidth={2.5} />
                <span>下载原片</span>
              </button>
            )}
          </div>
        </div>
      )}

      {/* Save As Modal Integration */}
      {showSaveAsModal && (
        <MoveCopyModal
          action="save_as"
          item={{
            id: 'save-as-video-doc',
            title: activeDoc.title,
            type: 'video'
          }}
          activeKb={activeKb || null}
          onClose={() => setShowSaveAsModal(false)}
          onSubmit={handleSaveAsSubmit}
        />
      )}
    </div>
  );
}


