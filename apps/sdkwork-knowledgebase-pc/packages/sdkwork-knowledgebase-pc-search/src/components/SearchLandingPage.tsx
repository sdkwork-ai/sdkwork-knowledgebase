import { Search, Globe, FileText, BookOpen, ChevronRight } from 'lucide-react';
import { useTranslation } from 'react-i18next';
import { CHAT_LAYOUT_MAX, PRESET_PROMPTS } from '../constants';
import { SearchComposer } from './SearchComposer';
import type { SearchComposerProps } from './SearchComposer';

export interface SearchLandingPageProps extends SearchComposerProps {
  onPresetClick: (text: string) => void;
}

export function SearchLandingPage({ onPresetClick, ...composerProps }: SearchLandingPageProps) {
  const { t } = useTranslation('search');

  return (
    <div className="min-h-full flex flex-col justify-center items-center px-6 md:px-12 py-8 text-center animate-in fade-in zoom-in-95 duration-300 w-full">
      <div className="w-[72px] h-[72px] rounded-[22px] bg-gradient-to-tr from-[var(--color-kb-accent)] to-[color-mix(in_srgb,var(--color-kb-accent)_70%,#7c3aed)] flex items-center justify-center shadow-lg shadow-[color-mix(in_srgb,var(--color-kb-accent)_25%,transparent)] mb-6 relative group hover:scale-105 transition-transform duration-300">
        <Search className="w-8 h-8 text-white" strokeWidth={2.5} />
      </div>
      <h2 className="text-[28px] md:text-[32px] font-extrabold tracking-tight text-[var(--color-kb-text-heading)] font-display">
        {t('landingTitle')}
      </h2>
      <p className="text-[15px] text-[var(--color-kb-text-muted)] max-w-xl mt-3 leading-relaxed">
        {t('landingSubtitle')}
      </p>
      <div className="flex flex-wrap items-center justify-center gap-2 mt-5">
        {[
          { icon: BookOpen, label: t('badgeLocalRag') },
          { icon: Globe, label: t('badgeWebSearch') },
          { icon: FileText, label: t('badgeCitations') }
        ].map(({ icon: Icon, label }) => (
          <span
            key={label}
            className="inline-flex items-center gap-1.5 px-3 py-1.5 rounded-full text-[11px] font-semibold bg-[var(--color-kb-panel)]/80 text-[var(--color-kb-text-muted)] border border-[var(--color-kb-panel-border)]"
          >
            <Icon className="w-3 h-3" />
            {label}
          </span>
        ))}
      </div>

      <div className="w-full mt-10 px-2">
        <SearchComposer
          {...composerProps}
          variant="hero"
          placeholder={t('composerPlaceholderHero')}
        />
      </div>

      <div className={`w-full ${CHAT_LAYOUT_MAX} mt-10 grid grid-cols-1 md:grid-cols-2 gap-3 pb-6 px-2`}>
        {PRESET_PROMPTS.map((pill, i) => (
          <div
            key={i}
            role="button"
            tabIndex={0}
            onClick={() => onPresetClick(pill.text)}
            onKeyDown={(e) => {
              if (e.key === 'Enter' || e.key === ' ') {
                e.preventDefault();
                onPresetClick(pill.text);
              }
            }}
            className="flex items-center gap-3.5 p-4 rounded-2xl bg-[var(--color-kb-editor)] border border-[var(--color-kb-panel-border)] text-left hover:border-[color-mix(in_srgb,var(--color-kb-accent)_40%,var(--color-kb-panel-border))] hover:bg-[var(--color-kb-panel-active)]/30 cursor-pointer shadow-sm active:scale-98 transition-all group"
          >
            <div className="w-9 h-9 rounded-xl bg-[var(--color-kb-panel)] flex items-center justify-center font-bold text-base shadow-inner group-hover:bg-[var(--color-kb-panel-active)] transition-colors">
              {pill.icon}
            </div>
            <div className="flex-1 min-w-0">
              <p className="text-xs font-semibold text-[var(--color-kb-text-heading)] group-hover:text-[var(--color-kb-accent)] transition-colors truncate">
                {pill.text}
              </p>
              <span className="text-[10px] text-[var(--color-kb-text-muted)]">
                {pill.type === 'doc' ? '📄 优先检索本地多级目录文件' : '🌐 深度爬取并聚合互联网文章'}
              </span>
            </div>
            <ChevronRight className="w-3.5 h-3.5 text-[var(--color-kb-text-muted)]/50 group-hover:text-[var(--color-kb-accent)] transition-colors shrink-0" />
          </div>
        ))}
      </div>
    </div>
  );
}
