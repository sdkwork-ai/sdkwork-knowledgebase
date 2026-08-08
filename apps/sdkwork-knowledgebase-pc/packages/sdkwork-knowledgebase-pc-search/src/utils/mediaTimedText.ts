import type { MediaTimedLine } from '../types';

export function findActiveTimedLineIndex(lines: MediaTimedLine[], currentTime: number): number {
  if (!lines.length) return -1;
  for (let index = lines.length - 1; index >= 0; index -= 1) {
    const line = lines[index];
    if (currentTime >= line.startTime) {
      if (line.endTime != null && currentTime >= line.endTime) continue;
      return index;
    }
  }
  return -1;
}

export function hasSyncedTimedText(lines?: MediaTimedLine[]): boolean {
  return Boolean(lines?.some((line) => line.text.trim()));
}

/** Progress through the active line (0–1) for karaoke-style highlight. */
export function computeLineProgress(
  line: MediaTimedLine,
  nextLine: MediaTimedLine | undefined,
  currentTime: number
): number {
  const end = line.endTime ?? nextLine?.startTime;
  if (end == null || end <= line.startTime) return 0;
  return Math.max(0, Math.min(1, (currentTime - line.startTime) / (end - line.startTime)));
}
