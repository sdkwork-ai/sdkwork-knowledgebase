import { describe, expect, it } from 'vitest';

import { buildUniqueNoteTitle } from './noteTitle';

describe('buildUniqueNoteTitle', () => {
  it('keeps the default title when no note uses it', () => {
    expect(buildUniqueNoteTitle('未命名笔记', [])).toBe('未命名笔记');
    expect(buildUniqueNoteTitle('未命名笔记', ['周报', '会议纪要'])).toBe('未命名笔记');
  });

  it('allocates the next free numeric suffix', () => {
    expect(buildUniqueNoteTitle('未命名笔记', ['未命名笔记'])).toBe('未命名笔记 2');
    expect(
      buildUniqueNoteTitle('未命名笔记', ['未命名笔记', '未命名笔记 2', '未命名笔记 3']),
    ).toBe('未命名笔记 4');
  });

  it('ignores blank and untrimmed entries while matching trimmed titles', () => {
    expect(buildUniqueNoteTitle('未命名笔记', ['  ', ' 未命名笔记 '])).toBe('未命名笔记 2');
  });

  it('does not treat a different suffix as taken', () => {
    expect(buildUniqueNoteTitle('未命名笔记', ['未命名笔记 3'])).toBe('未命名笔记');
  });
});
