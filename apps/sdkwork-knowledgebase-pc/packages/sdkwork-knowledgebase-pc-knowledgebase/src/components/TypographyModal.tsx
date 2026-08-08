import { useState, useEffect, useMemo } from 'react';
import { X, LayoutTemplate, Wand2, Check, Sparkles, Loader2 } from 'lucide-react';
import { useTranslation } from 'react-i18next';
import { isKnowledgebaseApiAvailable } from 'sdkwork-knowledgebase-pc-core';
import { AIService } from '../services/ai';
import { sanitizePreviewHtml } from '../utils/htmlSanitizer';

export interface TypographyModalProps {
  isOpen: boolean;
  onClose: () => void;
  originalContent: string;
  onConfirm: (formattedHtml: string) => void;
  articleTitle?: string;
  previewAuthorName?: string;
}

const THEME_COLORS = [
  { id: 'blue-1', name: '智黑蓝', value: '#2563eb' },
  { id: 'blue-2', name: '浅天蓝', value: '#38bdf8' },
  { id: 'purple-1', name: '淡雅紫', value: '#8b5cf6' },
  { id: 'green-1', name: '经典绿', value: '#07c160' }, // Classic WeChat Green
  { id: 'green-2', name: '竹青绿', value: '#10b981' },
  { id: 'orange-1', name: '晚霞黄', value: '#f59e0b' },
  { id: 'orange-2', name: '珊瑚橘', value: '#f97316' },
  { id: 'rose-1', name: '海棠粉', value: '#f43f5e' },
  { id: 'gray-1', name: '深色空间', value: '#64748b' },
  { id: 'gray-2', name: '极简炭黑', value: '#1e293b' },
  { id: 'gradient-1', name: '摩登渐变', value: 'linear-gradient(135deg, #07c160, #2563eb)' },
];

const getSolidColor = (colorVal: string) => {
  if (colorVal.includes('gradient')) {
    const match = colorVal.match(/#[0-9a-fA-F]{6}|#[0-9a-fA-F]{3}/);
    return match ? match[0] : '#07c160';
  }
  return colorVal;
};

export function TypographyModal({ isOpen, onClose, originalContent, onConfirm, articleTitle, previewAuthorName }: TypographyModalProps) {
  const { t } = useTranslation(['editor', 'common', 'mcp']);
  const previewDateLabel = useMemo(
    () => new Date().toLocaleDateString(),
    [isOpen],
  );
  
  const getTranslatedColorName = (id: string, def: string) => {
    switch (id) {
      case 'blue-1': return t('colorBlue1', { ns: 'mcp' }) || def;
      case 'blue-2': return t('colorBlue2', { ns: 'mcp' }) || def;
      case 'purple-1': return t('colorPurple1', { ns: 'mcp' }) || def;
      case 'green-1': return t('colorGreen1', { ns: 'mcp' }) || def;
      case 'green-2': return t('colorGreen2', { ns: 'mcp' }) || def;
      case 'orange-1': return t('colorOrange1', { ns: 'mcp' }) || def;
      case 'orange-2': return t('colorOrange2', { ns: 'mcp' }) || def;
      case 'rose-1': return t('colorRose1', { ns: 'mcp' }) || def;
      case 'gray-1': return t('colorGray1', { ns: 'mcp' }) || def;
      case 'gray-2': return t('colorGray2', { ns: 'mcp' }) || def;
      case 'gradient-1': return t('colorGradient1', { ns: 'mcp' }) || def;
      default: return def;
    }
  };

  const getTranslatedTemplate = (id: string, defName: string, defDesc: string) => {
    switch (id) {
      case 'golden':
        return {
          name: t('wechatBrandName', { ns: 'mcp' }) || defName,
          desc: t('wechatBrandDesc', { ns: 'mcp' }) || defDesc
        };
      case 'minimalist':
        return {
          name: t('minimalistName', { ns: 'mcp' }) || defName,
          desc: t('minimalistDesc', { ns: 'mcp' }) || defDesc
        };
      case 'tech':
        return {
          name: t('techName', { ns: 'mcp' }) || defName,
          desc: t('techDesc', { ns: 'mcp' }) || defDesc
        };
      default:
        return { name: defName, desc: defDesc };
    }
  };

  const [selectedTemplateId, setSelectedTemplateId] = useState('golden');
  const [activeColor, setActiveColor] = useState(THEME_COLORS[3]); // Default classic green
  const [previewContent, setPreviewContent] = useState('');
  const [isFormatting, setIsFormatting] = useState(false);
  const [isRewriting, setIsRewriting] = useState(false);
  const [activeContent, setActiveContent] = useState(originalContent);

  const TEMPLATES = [
    {
      id: 'golden',
      name: '微信品牌排版',
      desc: '公众号官方大师级排版，配有严谨的边框和黄金呼吸留白。',
      apply: (html: string, colorVal: string) => {
        const solidColor = getSolidColor(colorVal);
        let formatted = html
          .replace(/style\s*=\s*(['"])([\s\S]*?)\1/gi, '')
          .replace(/class\s*=\s*(['"])([\s\S]*?)\1/gi, '');
        
        // Headings
        formatted = formatted.replace(
          /<h1[^>]*>([\s\S]*?)<\/h1>/gi, 
          `<h1 style="font-size: 20px; font-weight: bold; text-align: center; color: ${solidColor}; margin: 32px 0 16px; padding: 12px 10px; border-top: 2px solid ${solidColor}; border-bottom: 2px solid ${solidColor}; letter-spacing: 1.5px; display: block; line-height: 1.4;">$1</h1>`
        );
        formatted = formatted.replace(
          /<h2[^>]*>([\s\S]*?)<\/h2>/gi, 
          `<h2 style="font-size: 17px; font-weight: bold; padding: 4px 12px; border-left: 5px solid ${solidColor}; margin: 28px 0 14px; color: #1e293b; letter-spacing: 1px; line-height: 1.4; background-color: ${solidColor}0a;">$1</h2>`
        );
        formatted = formatted.replace(
          /<h3[^>]*>([\s\S]*?)<\/h3>/gi, 
          `<h3 style="font-size: 15px; font-weight: bold; color: ${solidColor}; margin: 22px 0 10px; display: flex; align-items: center; gap: 6px;"><span style="color: ${solidColor}; font-weight: 900; font-size: 14px;">✦</span>$1</h3>`
        );
        
        // Quotes
        formatted = formatted.replace(
          /<blockquote[^>]*>([\s\S]*?)<\/blockquote>/gi, 
          `<blockquote style="border-left: 4px solid ${solidColor}; background-color: #f8fafc; padding: 14px 18px; margin: 22px 0; border-radius: 4px; font-size: 14px; line-height: 1.8; color: #4b5563; font-style: normal;">$1</blockquote>`
        );
        
        // Paragraphs
        formatted = formatted.replace(
          /<p[^>]*>([\s\S]*?)<\/p>/gi,
          `<p style="font-size: 14.5px; line-height: 1.8; letter-spacing: 1.5px; color: #3f3f3f; margin-bottom: 20px; text-align: justify; word-break: break-all;">$1</p>`
        );
        
        // Lists
        formatted = formatted.replace(
          /<ul[^>]*>/gi,
          `<ul style="list-style-type: disc; padding-left: 20px; margin: 18px 0; font-size: 14.5px; line-height: 1.8; color: #3f3f3f;">`
        );
        formatted = formatted.replace(
          /<ol[^>]*>/gi,
          `<ol style="list-style-type: decimal; padding-left: 20px; margin: 18px 0; font-size: 14.5px; line-height: 1.8; color: #3f3f3f;">`
        );
        formatted = formatted.replace(
          /<li[^>]*>([\s\S]*?)<\/li>/gi,
          `<li style="margin-bottom: 8px; line-height: 1.8; color: #3f3f3f;">$1</li>`
        );

        // Images
        formatted = formatted.replace(
          /<img([^>]*)\/?>/gi,
          `<img$1 style="max-width: 100%; border-radius: 12px; height: auto; box-shadow: 0 4px 20px rgba(0,0,0,0.08); margin: 20px auto; display: block;" />`
        );

        // Bold text with color tint
        formatted = formatted.replace(
          /<strong[^>]*>([\s\S]*?)<\/strong>/gi,
          `<strong style="color: ${solidColor}; font-weight: bold;">$1</strong>`
        );
        formatted = formatted.replace(
          /<b[^>]*>([\s\S]*?)<\/b>/gi,
          `<strong style="color: ${solidColor}; font-weight: bold;">$1</strong>`
        );

        // Links with color tint
        formatted = formatted.replace(
          /<a[^>]*>([\s\S]*?)<\/a>/gi,
          `<a href="#" style="color: ${solidColor}; text-decoration: underline; font-weight: 500;">$1</a>`
        );

        return formatted;
      }
    },
    {
      id: 'minimalist',
      name: '极简商务现代',
      desc: '主打高质感极简大面积空白，低饱和度色调点缀，极富设计张力。',
      apply: (html: string, colorVal: string) => {
        const solidColor = getSolidColor(colorVal);
        let formatted = html
          .replace(/style\s*=\s*(['"])([\s\S]*?)\1/gi, '')
          .replace(/class\s*=\s*(['"])([\s\S]*?)\1/gi, '');
        
        formatted = formatted.replace(
          /<h1[^>]*>([\s\S]*?)<\/h1>/gi, 
          `<h1 style="font-size: 22px; font-weight: 300; text-align: center; color: #0f172a; margin: 40px 0 24px; letter-spacing: 2.5px; border-bottom: 1.5px solid ${solidColor}33; padding-bottom: 14px; line-height: 1.5;">$1</h1>`
        );
        formatted = formatted.replace(
          /<h2[^>]*>([\s\S]*?)<\/h2>/gi, 
          `<h2 style="font-size: 16px; font-weight: bold; margin: 32px 0 16px; color: #1e293b; letter-spacing: 1.5px; display: flex; align-items: center; gap: 8px; line-height: 1.4;"><span style="width: 6px; height: 6px; background-color: ${solidColor}; border-radius: 50%; display: inline-block;"></span>$1</h2>`
        );
        formatted = formatted.replace(
          /<h3[^>]*>([\s\S]*?)<\/h3>/gi, 
          `<h3 style="font-size: 14.5px; font-weight: bold; color: ${solidColor}; margin: 24px 0 12px; letter-spacing: 1px;">| $1</h3>`
        );
        formatted = formatted.replace(
          /<blockquote[^>]*>([\s\S]*?)<\/blockquote>/gi, 
          `<blockquote style="font-style: italic; text-align: center; color: #4b5563; font-size: 14px; margin: 32px 10px; border-top: 1.5px dashed ${solidColor}55; border-bottom: 1.5px dashed ${solidColor}55; padding: 18px 0; line-height: 1.8;">“ $1 ”</blockquote>`
        );
        formatted = formatted.replace(
          /<p[^>]*>([\s\S]*?)<\/p>/gi,
          `<p style="font-size: 14px; line-height: 1.9; letter-spacing: 1.2px; color: #4b5563; margin-bottom: 22px; text-align: justify; font-weight: 300;">$1</p>`
        );
        
        formatted = formatted.replace(
          /<ul[^>]*>/gi,
          `<ul style="list-style-type: circle; padding-left: 20px; margin: 18px 0; font-size: 14px; line-height: 1.9; color: #4b5563;">`
        );
        formatted = formatted.replace(
          /<ol[^>]*>/gi,
          `<ol style="list-style-type: decimal; padding-left: 20px; margin: 18px 0; font-size: 14px; line-height: 1.9; color: #4b5563;">`
        );
        formatted = formatted.replace(
          /<li[^>]*>([\s\S]*?)<\/li>/gi,
          `<li style="margin-bottom: 8px; line-height: 1.9; color: #4b5563; font-weight: 300;">$1</li>`
        );

        formatted = formatted.replace(
          /<img([^>]*)\/?>/gi,
          `<img$1 style="max-width: 100%; border-radius: 4px; height: auto; margin: 24px auto; display: block;" />`
        );

        // Bold text with color tint
        formatted = formatted.replace(
          /<strong[^>]*>([\s\S]*?)<\/strong>/gi,
          `<strong style="color: ${solidColor}; font-weight: bold;">$1</strong>`
        );
        formatted = formatted.replace(
          /<b[^>]*>([\s\S]*?)<\/b>/gi,
          `<strong style="color: ${solidColor}; font-weight: bold;">$1</strong>`
        );

        // Links with color tint
        formatted = formatted.replace(
          /<a[^>]*>([\s\S]*?)<\/a>/gi,
          `<a href="#" style="color: ${solidColor}; text-decoration: underline; font-weight: 500;">$1</a>`
        );

        return formatted;
      }
    },
    {
      id: 'tech',
      name: '探索极客科技',
      desc: '专门针对开发与科学前沿，拥有标志性伪终端框架，代码细节突出。',
      apply: (html: string, colorVal: string) => {
        const solidColor = getSolidColor(colorVal);
        let formatted = html
          .replace(/style\s*=\s*(['"])([\s\S]*?)\1/gi, '')
          .replace(/class\s*=\s*(['"])([\s\S]*?)\1/gi, '');
        
        formatted = formatted.replace(
          /<h1[^>]*>([\s\S]*?)<\/h1>/gi, 
          `<h1 style="font-size: 20px; font-weight: 600; font-family: 'Fira Code', 'Courier New', monospace; color: ${solidColor}; margin: 28px 0 16px; border-bottom: 2.5px dashed ${solidColor}; padding-bottom: 8px; line-height: 1.5;">&gt; $1 _</h1>`
        );
        formatted = formatted.replace(
          /<h2[^>]*>([\s\S]*?)<\/h2>/gi, 
          `<h2 style="font-size: 15px; font-weight: bold; background-color: ${solidColor}; color: #ffffff; display: inline-block; padding: 6px 14px; margin: 26px 0 12px; border-radius: 4px; box-shadow: 3px 3px 0px #090d16; font-family: 'Fira Code', 'Courier New', monospace;">$1</h2>`
        );
        formatted = formatted.replace(
          /<h3[^>]*>([\s\S]*?)<\/h3>/gi, 
          `<h3 style="font-size: 14px; font-weight: bold; color: ${solidColor}; margin: 20px 0 8px; font-family: 'Fira Code', 'Courier New', monospace;"># $1</h3>`
        );
        formatted = formatted.replace(
          /<blockquote[^>]*>([\s\S]*?)<\/blockquote>/gi, 
          `<blockquote style="border-left: 4px solid ${solidColor}; background-color: #f1f5f9; padding: 12px 18px; margin: 22px 0; font-size: 13px; line-height: 1.7; color: #1e293b; font-family: 'Fira Code', 'Courier New', monospace; border-radius: 2px;">$1</blockquote>`
        );
        formatted = formatted.replace(
          /<p[^>]*>([\s\S]*?)<\/p>/gi,
          `<p style="font-size: 14px; line-height: 1.75; color: #1e293b; margin-bottom: 16px; text-align: justify; font-family: -apple-system, BlinkMacSystemFont, inherit;">$1</p>`
        );
        
        formatted = formatted.replace(
          /<ul[^>]*>/gi,
          `<ul style="list-style-type: square; padding-left: 20px; margin: 18px 0; font-size: 14px; line-height: 1.75; color: #1e293b;">`
        );
        formatted = formatted.replace(
          /<ol[^>]*>/gi,
          `<ol style="list-style-type: decimal; padding-left: 20px; margin: 18px 0; font-size: 14px; line-height: 1.75; color: #1e293b;">`
        );
        formatted = formatted.replace(
          /<li[^>]*>([\s\S]*?)<\/li>/gi,
          `<li style="margin-bottom: 8px; line-height: 1.75; color: #1e293b;">$1</li>`
        );

        formatted = formatted.replace(
          /<img([^>]*)\/?>/gi,
          `<img$1 style="max-width: 100%; border-radius: 6px; height: auto; border: 2px solid #090d16; margin: 20px auto; display: block;" />`
        );

        // Bold text with color tint
        formatted = formatted.replace(
          /<strong[^>]*>([\s\S]*?)<\/strong>/gi,
          `<strong style="color: ${solidColor}; font-weight: bold;">$1</strong>`
        );
        formatted = formatted.replace(
          /<b[^>]*>([\s\S]*?)<\/b>/gi,
          `<strong style="color: ${solidColor}; font-weight: bold;">$1</strong>`
        );

        // Links with color tint
        formatted = formatted.replace(
          /<a[^>]*>([\s\S]*?)<\/a>/gi,
          `<a href="#" style="color: ${solidColor}; text-decoration: underline; font-weight: 500;">$1</a>`
        );

        return formatted;
      }
    }
  ];

  const triggerAutoStyle = (templateId: string, colorValue: string, contentSource: string) => {
    setIsFormatting(true);
    setTimeout(() => {
      const template = TEMPLATES.find(t => t.id === templateId) || TEMPLATES[0];
      const result = template.apply(contentSource, colorValue);
      setPreviewContent(result);
      setIsFormatting(false);
    }, 450);
  };

  useEffect(() => {
    if (isOpen) {
      setActiveContent(originalContent);
      triggerAutoStyle(selectedTemplateId, activeColor.value, originalContent);
    }
  }, [isOpen, originalContent]);

  const handleTemplateChange = (id: string) => {
    setSelectedTemplateId(id);
    triggerAutoStyle(id, activeColor.value, activeContent);
  };

  const handleColorChange = (colObj: typeof THEME_COLORS[0]) => {
    setActiveColor(colObj);
    triggerAutoStyle(selectedTemplateId, colObj.value, activeContent);
  };

  const handleRewrite = async () => {
    setIsRewriting(true);
    try {
      if (isKnowledgebaseApiAvailable()) {
        const plainText = activeContent.replace(/<[^>]*>/g, ' ').replace(/\s+/g, ' ').trim();
        const polished = await AIService.handleAIAction(
          'polish',
          plainText || articleTitle || '',
          articleTitle || '',
        );
        const rewritten = polished.trim().startsWith('<')
          ? polished
          : `<p>${polished.replace(/\n{2,}/g, '</p><p>').replace(/\n/g, '<br>')}</p>`;
        setActiveContent(rewritten);
        triggerAutoStyle(selectedTemplateId, activeColor.value, rewritten);
        return;
      }

      await new Promise((resolve) => setTimeout(resolve, 1200));
      const rewritten = activeContent.replace(/<p[^>]*>([\s\S]*?)<\/p>/g, (_match, p1) => {
        return `<p>${p1}${t('aiRewrittenFeedback', { defaultValue: '【此段内容已通过微信金句引擎优化润色】' })}</p>`;
      });
      setActiveContent(rewritten);
      triggerAutoStyle(selectedTemplateId, activeColor.value, rewritten);
    } catch (error) {
      console.error(error);
    } finally {
      setIsRewriting(false);
    }
  };

  if (!isOpen) return null;

  const resolvedSolidColor = getSolidColor(activeColor.value);
  const titleToShow = articleTitle || t('defaultArticleTitle', { ns: 'mcp' }) || "若水无声：解密微信排版的美学张力";

  return (
    <div className="fixed inset-0 z-[500] flex items-center justify-center bg-black/40 backdrop-blur-md p-4 lg:p-6 animate-in fade-in duration-200">
      <div className="bg-[var(--color-kb-editor)] border border-[var(--color-kb-panel-border)] w-full max-w-[1300px] h-[90vh] rounded-3xl shadow-2xl flex flex-col overflow-hidden text-[var(--color-kb-text)]">
        
        {/* Elegant Minimal Header */}
        <div className="flex items-center justify-between px-8 py-5 border-b border-[var(--color-kb-panel-border)] flex-shrink-0 bg-[var(--color-kb-panel)] backdrop-blur">
          <div className="flex flex-col gap-1">
            <div className="flex items-center gap-2.5">
              <span className="w-2 h-5 bg-[#07c160] rounded-full" style={{ backgroundColor: resolvedSolidColor }}></span>
              <h1 className="text-xl font-extrabold text-[var(--color-kb-text-heading)] tracking-tight">{t('typographyTitle', { ns: 'mcp' }) || '一键排版'}</h1>
            </div>
            <p className="text-[11px] text-[var(--color-kb-text-muted)]">{t('typographyDesc', { ns: 'mcp' }) || '选择排版样式整体美化布局，内置色彩色泽校调与微信排版最适字距间隙比例'}</p>
          </div>
          <button 
            onClick={onClose} 
            className="w-10 h-10 rounded-full border border-[var(--color-kb-panel-border)] hover:bg-[var(--color-kb-panel-hover)] flex items-center justify-center transition-all cursor-pointer text-[var(--color-kb-text-muted)] hover:text-[var(--color-kb-text-heading)]"
          >
            <X size={18} />
          </button>
        </div>

        {/* Content Body Layout */}
        <div className="flex-1 overflow-hidden flex flex-col lg:flex-row bg-[var(--color-kb-panel)]/50">
          
          {/* Main Visual Preview Area (Centered Mobile View Canvas) */}
          <div className="flex-1 flex flex-col min-w-0 border-r border-[var(--color-kb-panel-border)] bg-[var(--color-kb-panel)]/20 p-6 overflow-y-auto">
            
            {/* Action Bar Floating Inside Canvas */}
            <div className="flex items-center justify-between bg-[var(--color-kb-editor)]/80 border border-[var(--color-kb-panel-border)]/80 rounded-2xl px-5 py-3 mb-6 flex-shrink-0 backdrop-blur-md">
              <div className="flex items-center gap-2">
                <Sparkles size={14} className="text-[#07c160]" style={{ color: resolvedSolidColor }} />
                <span className="text-[12px] font-bold text-[var(--color-kb-text-heading)]">{t('previewArea', { ns: 'mcp' }) || '微信标准排版预览区'}</span>
              </div>

              <button 
                onClick={handleRewrite}
                disabled={isRewriting || isFormatting}
                className="flex items-center gap-1.5 px-3 py-1.5 rounded-lg text-[11px] bg-[var(--color-kb-panel-hover)] border border-[var(--color-kb-panel-border)] font-bold transition-all disabled:opacity-50 text-[var(--color-kb-text-heading)] hover:bg-[var(--color-kb-panel)]"
              >
                {isRewriting ? (
                  <Loader2 size={13} className="animate-spin text-[#07c160]" style={{ color: resolvedSolidColor }} />
                ) : (
                  <Wand2 size={13} className="text-[#07c160]" style={{ color: resolvedSolidColor }} />
                )}
                <span>{isRewriting ? (t('rewriting', { ns: 'mcp' }) || '润色重写中...') : (t('aiRewrite', { ns: 'mcp' }) || '一键AI重写/强化润色')}</span>
              </button>
            </div>

            {/* Simulated Smartphone Screen Preview Container */}
            <div className="flex-1 flex justify-center items-start py-4">
              <div className="relative bg-white text-zinc-900 w-full max-w-[480px] min-h-[640px] rounded-[32px] shadow-2xl border-4 border-zinc-800/80 overflow-hidden flex flex-col transform transition-all duration-300">
                {/* Status Bar */}
                <div className="bg-zinc-100/50 border-b border-zinc-100 flex justify-between items-center px-6 py-2 select-none text-[10px] font-bold text-zinc-500">
                  <span>9:41</span>
                  <div className="flex items-center gap-1.5">
                    <span className="w-2.5 h-2 my-auto bg-zinc-500 rounded-xs inline-block"></span>
                    <span className="text-[9px]">4G</span>
                  </div>
                </div>

                {/* Content Frame */}
                {isFormatting ? (
                  <div className="flex-1 flex flex-col items-center justify-center p-8 bg-zinc-50 animate-pulse">
                    <div className="w-12 h-12 rounded-full border-4 border-t-transparent animate-spin mb-4" style={{ borderColor: `${resolvedSolidColor}33`, borderTopColor: resolvedSolidColor }} />
                    <span className="text-xs font-bold text-zinc-500">{t('formatting', { ns: 'mcp' }) || '正在排版布局渲染中...'}</span>
                  </div>
                ) : (
                  <div className="flex-1 overflow-y-auto p-7 bg-white relative">
                    {/* Header Details */}
                    <div className="mb-6">
                      <h2 className="text-xl font-bold text-zinc-850 leading-snug">
                        {titleToShow}
                      </h2>
                      <div className="flex items-center gap-2 mt-2 text-[11px] text-zinc-400 font-medium">
                        <span style={{ color: resolvedSolidColor }}>
                          {previewAuthorName || t('previewAuthorPlaceholder', { ns: 'mcp', defaultValue: '公众号名称' })}
                        </span>
                        <span>•</span>
                        <span>{previewDateLabel}</span>
                      </div>
                    </div>

                    {/* Rich HTML body output without layout classes that mask style colors */}
                    <div className="text-[14.5px] leading-relaxed text-zinc-800" style={{ fontFamily: '-apple-system, BlinkMacSystemFont, "Helvetica Neue", "PingFang SC", "Hiragino Sans GB", "Microsoft YaHei", Arial, sans-serif' }}>
                      <div dangerouslySetInnerHTML={{ __html: sanitizePreviewHtml(previewContent || `<p style="color: #a1a1aa;">${t('noContent', { ns: 'mcp' }) || '目前暂无正文内容...'}</p>`) }} />
                    </div>
                  </div>
                )}

                {/* Home Indicator */}
                <div className="bg-white py-2 flex justify-center border-t border-zinc-50/50">
                  <div className="w-28 h-1 bg-zinc-300 rounded-full"></div>
                </div>
              </div>
            </div>

          </div>

          {/* Right Parameters Selection Control Panel */}
          <div className="w-full lg:w-[420px] bg-[var(--color-kb-panel)] border-t lg:border-t-0 p-6 lg:p-7 flex flex-col justify-between overflow-y-auto border-l border-[var(--color-kb-panel-border)] flex-shrink-0">
            
            <div className="space-y-6">
              {/* Part 1: Typography Style Options */}
              <div className="space-y-3">
                <span className="text-xs font-black uppercase text-[var(--color-kb-text-muted)] tracking-wider flex items-center gap-1.5 mb-1">
                  <LayoutTemplate size={13} className="text-[#07c160]" style={{ color: resolvedSolidColor }} />
                  {t('stylesTitle', { ns: 'mcp' }) || '排版样式（三套标杆）'}
                </span>

                <div className="flex flex-col gap-3">
                  {TEMPLATES.map(temp => {
                    const isActive = selectedTemplateId === temp.id;
                    const meta = getTranslatedTemplate(temp.id, temp.name, temp.desc);
                    return (
                      <button 
                        key={temp.id}
                        type="button"
                        onClick={() => handleTemplateChange(temp.id)}
                        className={`text-left p-4 rounded-2xl border transition-all relative ${
                          isActive 
                            ? 'bg-[var(--color-kb-panel-hover)] text-[var(--color-kb-text-heading)] ring-1' 
                            : 'border-[var(--color-kb-panel-border)] hover:border-[var(--color-kb-accent)]/50 bg-[var(--color-kb-editor)] text-[var(--color-kb-text)]'
                        }`}
                        style={{ borderColor: isActive ? resolvedSolidColor : undefined, boxShadow: isActive ? `0 0 12px ${resolvedSolidColor}22` : undefined }}
                      >
                        <div className="flex items-center justify-between mb-1">
                          <span className="font-extrabold text-[13px]" style={{ color: isActive ? resolvedSolidColor : undefined }}>{meta.name}</span>
                          {isActive && <Check size={14} style={{ color: resolvedSolidColor }} className="font-bold" />}
                        </div>
                        <p className="text-[11px] text-[var(--color-kb-text-muted)] leading-relaxed font-normal">{meta.desc}</p>
                      </button>
                    );
                  })}
                </div>
              </div>

              {/* Part 2: Color Palettes Selection */}
              <div className="space-y-3 border-t border-[var(--color-kb-panel-border)] pt-5">
                <div className="space-y-1">
                  <span className="text-xs font-black uppercase text-[var(--color-kb-text-muted)] tracking-wider flex items-center gap-1.5">
                    <span className="w-1.5 h-1.5 rounded-full" style={{ backgroundColor: resolvedSolidColor }}></span>
                    {t('themeColors', { ns: 'mcp' }) || '主题配色'}
                  </span>
                  <p className="text-[10px] text-[var(--color-kb-text-muted)]">{t('themeColorsDesc', { ns: 'mcp' }) || '选择全局点缀主调配色，推荐适合微信阅读器高解析度的标准色相'}</p>
                </div>

                {/* Color Blocks Layout */}
                <div className="grid grid-cols-6 gap-3 pt-1">
                  {THEME_COLORS.map((col) => {
                    const isSelected = activeColor.id === col.id;
                    const isGradient = col.value.includes('gradient');
                    return (
                      <button 
                        key={col.id}
                        type="button"
                        onClick={() => handleColorChange(col)}
                        title={getTranslatedColorName(col.id, col.name)}
                        className={`w-full aspect-square rounded-xl transition-all relative flex items-center justify-center border-2 ${
                          isSelected 
                            ? 'border-white scale-110 shadow-lg ring-2 ring-[var(--color-kb-editor)] z-10' 
                            : 'border-transparent hover:scale-105 hover:border-[var(--color-kb-panel-border)]'
                        }`}
                        style={{ background: isGradient ? undefined : col.value }}
                      >
                        {isGradient && (
                          <div 
                            className="absolute inset-0 rounded-[10px]" 
                            style={{ background: col.value }}
                          />
                        )}
                        {isSelected && (
                          <Check size={14} className="text-[var(--color-kb-text)] bg-white rounded-full p-0.5 font-bold z-10" />
                        )}
                      </button>
                    );
                  })}
                </div>

                <div className="text-[11px] text-[var(--color-kb-text-muted)] flex justify-between items-center bg-[var(--color-kb-editor)] px-3 py-2 rounded-xl mt-2 select-none border border-[var(--color-kb-panel-border)]">
                  <span>{t('activeColorLabel', { ns: 'mcp' }) || '当前激活色调'}:</span>
                  <span className="font-extrabold text-[var(--color-kb-text-heading)] flex items-center gap-1.5">
                    <span className="w-2 h-2 rounded-full inline-block" style={{ background: activeColor.value }} />
                    {getTranslatedColorName(activeColor.id, activeColor.name)}
                  </span>
                </div>
              </div>

            </div>

            {/* Bottom Actions Frame */}
            <div className="border-t border-[var(--color-kb-panel-border)] pt-6 mt-6 flex flex-col gap-2">
              <button 
                onClick={() => onConfirm(previewContent)} 
                disabled={isFormatting}
                className="w-full h-11 text-sm font-black text-white rounded-xl shadow-lg transition-all flex items-center justify-center gap-1.5 cursor-pointer active:scale-98 disabled:opacity-50"
                style={{ backgroundColor: resolvedSolidColor }}
              >
                <span>{t('useTypography', { ns: 'mcp' }) || '使用此排版'}</span>
              </button>
              <button 
                onClick={onClose} 
                className="w-full h-11 text-sm font-extrabold bg-[var(--color-kb-editor)] hover:bg-[var(--color-kb-panel-hover)] border border-[var(--color-kb-panel-border)] text-[var(--color-kb-text)] rounded-xl transition-all cursor-pointer active:scale-98"
              >
                <span>{t('cancel', { ns: 'mcp' }) || '取消'}</span>
              </button>
            </div>

          </div>

        </div>

      </div>
    </div>
  );
}
