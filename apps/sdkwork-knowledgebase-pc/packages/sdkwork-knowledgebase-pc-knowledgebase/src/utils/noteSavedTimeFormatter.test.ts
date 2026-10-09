import { describe, expect, it } from 'vitest';
import { formatNoteSavedTime } from './noteSavedTimeFormatter';

describe('note saved time formatter', () => {
  it('shows the clock time for a save earlier the same day', () => {
    const now = new Date('2026-10-08T18:00:00');
    const savedAt = new Date('2026-10-08T09:05:00').getTime();
    const label = formatNoteSavedTime(savedAt, 'zh-CN', now);
    expect(label).toMatch(/09[:：]05/);
  });

  it('includes the date for a save from a previous day', () => {
    const now = new Date('2026-10-08T18:00:00');
    const savedAt = new Date('2026-10-01T09:05:00').getTime();
    const label = formatNoteSavedTime(savedAt, 'zh-CN', now);
    expect(label).toMatch(/10[/月-]/);
    expect(label).toMatch(/09[:：]05/);
  });

  it('honors the requested locale', () => {
    const now = new Date('2026-10-08T18:00:00');
    const savedAt = new Date('2026-10-08T09:05:00').getTime();
    const label = formatNoteSavedTime(savedAt, 'en-US', now);
    expect(label).toMatch(/09:05/);
    expect(label).toMatch(/AM/);
  });
});
