import DOMPurify from 'dompurify';
import { isBlank } from '@sdkwork/utils';
import { marked } from 'marked';

export function parseFollowUpSuggestions(content: string): string[] {
  if (isBlank(content)) return [];
  const lines = content.split('\n');
  const suggestions: string[] = [];
  let inFollowUp = false;

  for (const line of lines) {
    const trimmed = line.trim();
    if (/追问|继续探索|建议您接下来|您可以继续/i.test(trimmed)) {
      inFollowUp = true;
      continue;
    }
    if (inFollowUp && /^[-*•]\s+/.test(trimmed)) {
      const text = trimmed
        .replace(/^[-*•]\s+/, '')
        .replace(/^\*+|\*+$/g, '')
        .replace(/^[_`]+|[_`]+$/g, '')
        .trim();
      if (text.length > 4 && text.length < 120) suggestions.push(text);
    }
    if (inFollowUp && trimmed.startsWith('###') && suggestions.length > 0) break;
  }

  return suggestions.slice(0, 3);
}

export function formatMarkdownHtml(content: string, messageId: string): string {
  const parsed = content.replace(/\[([0-9]+)\]/g, (_match, num) => {
    return `<button type="button" data-citation="${num}" data-msg="${messageId}" class="search-citation-badge" title="查看来源 [${num}]">${num}</button>`;
  });
  const rawHtml = marked.parse(parsed, { async: false }) as string;
  return DOMPurify.sanitize(rawHtml, { ADD_ATTR: ['data-citation', 'data-msg'], ADD_TAGS: ['button'] });
}

export const SEARCH_EXPAND_SOURCES_EVENT = 'sdkwork-search:expand-sources';

export function scrollToCitation(messageId: string, citationNum: string) {
  const targetId = `citation-card-${messageId}-${citationNum}`;

  const highlightTarget = () => {
    const el = document.getElementById(targetId);
    if (!el) return false;
    el.scrollIntoView({ behavior: 'smooth', block: 'nearest' });
    el.classList.add('search-source-row--highlight');
    window.setTimeout(() => el.classList.remove('search-source-row--highlight'), 1600);
    return true;
  };

  if (highlightTarget()) return;

  window.dispatchEvent(
    new CustomEvent(SEARCH_EXPAND_SOURCES_EVENT, { detail: { messageId, citationNum } })
  );
  window.setTimeout(() => highlightTarget(), 60);
}
