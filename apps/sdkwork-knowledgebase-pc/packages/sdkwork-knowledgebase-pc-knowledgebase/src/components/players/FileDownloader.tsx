import { FileText, FileUp } from 'lucide-react';
import { DocumentMeta } from '../../services/document';
import { useTranslation } from 'react-i18next';

export interface FileDownloaderProps {
  activeDoc: DocumentMeta;
}

export function FileDownloader({ activeDoc }: FileDownloaderProps) {
  const { t } = useTranslation('editor');
  const title = (activeDoc.title || '').toLowerCase();
  
  const isWord = title.endsWith('.doc') || title.endsWith('.docx') || title.endsWith('.wps');
  const isExcel = title.endsWith('.xls') || title.endsWith('.xlsx') || title.endsWith('.csv');
  const isPPT = title.endsWith('.ppt') || title.endsWith('.pptx');
  const isZip = title.endsWith('.zip') || title.endsWith('.rar') || title.endsWith('.7z') || title.endsWith('.tar');

  const getFormatProps = () => {
    if (isWord) {
      return {
        label: 'Microsoft Word 文档',
        iconBg: 'bg-blue-600/10 dark:bg-blue-500/15 text-blue-600 dark:text-blue-400',
        dotBg: 'bg-blue-500'
      };
    }
    if (isExcel) {
      return {
        label: 'Microsoft Excel 工作表',
        iconBg: 'bg-emerald-600/10 dark:bg-emerald-500/15 text-emerald-600 dark:text-emerald-400',
        dotBg: 'bg-emerald-500'
      };
    }
    if (isPPT) {
      return {
        label: 'PowerPoint 演示文稿',
        iconBg: 'bg-orange-600/10 dark:bg-orange-500/15 text-orange-600 dark:text-orange-400',
        dotBg: 'bg-orange-500'
      };
    }
    if (isZip) {
      return {
        label: '压缩存档分包',
        iconBg: 'bg-purple-600/10 dark:bg-purple-500/15 text-purple-600 dark:text-purple-400',
        dotBg: 'bg-purple-500'
      };
    }
    return {
      label: '通用工作文件',
      iconBg: 'bg-zinc-500/10 dark:bg-zinc-500/15 text-zinc-600 dark:text-zinc-400',
      dotBg: 'bg-[var(--color-kb-accent)]'
    };
  };

  const format = getFormatProps();

  return (
    <div id={`file-downloader-${activeDoc.id}`} className="flex flex-col items-center justify-center text-center p-10 h-full">
      <div className={`w-24 h-24 rounded-3xl ${format.iconBg} flex items-center justify-center mb-6 border border-zinc-200/20 shadow-lg relative transition-all duration-300 hover:scale-105`}>
         <FileText size={42} strokeWidth={1.5} />
         <div className={`absolute top-2.5 right-2.5 w-3.5 h-3.5 ${format.dotBg} rounded-full border-2 border-white dark:border-black shadow-sm`} />
      </div>
      <h2 className="text-xl font-bold text-[var(--color-kb-text-heading)] mb-1 tracking-wide">{activeDoc.title}</h2>
      <p className="text-xs font-semibold text-[var(--color-kb-text-muted)] opacity-85 mb-4">{format.label}</p>
      
      <p className="text-xs text-[var(--color-kb-text-muted)] mb-8 bg-[var(--color-kb-panel)] px-4 py-1.5 rounded-full border border-[var(--color-kb-panel-border)]/50 font-mono">
        文件属性大小: {activeDoc.size || t('unknownSize')}
      </p>
      {activeDoc.url && (
        <a 
          href={activeDoc.url} 
          download={activeDoc.title} 
          className="px-6 py-2.5 bg-[var(--color-kb-accent)] text-white rounded-lg hover:bg-[var(--color-kb-accent-hover)] transition-all font-semibold text-xs flex items-center shadow-lg hover:shadow-[var(--color-kb-accent)]/15 active:scale-95"
        >
          <FileUp size={15} className="mr-2" /> 立即安全下载文件
        </a>
      )}
    </div>
  );
}
