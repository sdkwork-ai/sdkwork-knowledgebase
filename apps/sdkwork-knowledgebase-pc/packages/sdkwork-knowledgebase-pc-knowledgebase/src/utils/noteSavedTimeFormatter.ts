/**
 * Locale-aware display time for the "已保存 <time>" autosave pill: notes saved
 * today show the clock time, older saves include the date.
 */
export function formatNoteSavedTime(savedAtMs: number, locale: string, now: Date = new Date()): string {
  const saved = new Date(savedAtMs);
  const timeOptions: Intl.DateTimeFormatOptions = { hour: '2-digit', minute: '2-digit' };
  if (saved.toDateString() === now.toDateString()) {
    return new Intl.DateTimeFormat(locale, timeOptions).format(saved);
  }
  return new Intl.DateTimeFormat(locale, {
    month: 'short',
    day: 'numeric',
    ...timeOptions,
  }).format(saved);
}
