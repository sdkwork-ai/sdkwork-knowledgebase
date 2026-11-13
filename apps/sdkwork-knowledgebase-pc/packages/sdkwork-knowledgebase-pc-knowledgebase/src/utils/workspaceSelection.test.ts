import { describe, expect, it } from 'vitest';

import {
  isStoredKnowledgeBaseCurrent,
  selectRenderableTabDocuments,
} from './workspaceSelection';

interface TreeNode {
  id: string;
  title: string;
}

describe('selectRenderableTabDocuments', () => {
  const tree: TreeNode[] = [
    { id: 'doc-1', title: 'Current title 1' },
    { id: 'doc-2', title: 'Current title 2' },
  ];

  it('drops cached tabs the current listing no longer contains', () => {
    const cached: TreeNode[] = [
      { id: 'doc-1', title: 'Stale 1' },
      { id: 'foreign-doc', title: 'Another account title' },
    ];

    expect(selectRenderableTabDocuments(cached, tree)).toEqual([
      { id: 'doc-1', title: 'Current title 1' },
    ]);
  });

  it('keeps tab order and replaces cached metadata with the listing copy', () => {
    const cached: TreeNode[] = [
      { id: 'doc-2', title: 'Stale 2' },
      { id: 'doc-1', title: 'Stale 1' },
    ];

    expect(selectRenderableTabDocuments(cached, tree).map((doc) => doc.id)).toEqual([
      'doc-2',
      'doc-1',
    ]);
    expect(selectRenderableTabDocuments(cached, tree)[0]?.title).toBe('Current title 2');
  });

  it('collapses duplicate cached rows and returns an empty list for an empty tree', () => {
    const cached: TreeNode[] = [
      { id: 'doc-1', title: 'a' },
      { id: 'doc-1', title: 'b' },
    ];

    expect(selectRenderableTabDocuments(cached, tree)).toHaveLength(1);
    expect(selectRenderableTabDocuments(cached, [])).toEqual([]);
  });
});

describe('isStoredKnowledgeBaseCurrent', () => {
  const registered = ['42', '43'];
  const available = ['42'];

  it('accepts a selection both the registry and the listing contain', () => {
    expect(isStoredKnowledgeBaseCurrent('42', registered, available)).toBe(true);
  });

  it('rejects a space missing from the account registry', () => {
    expect(isStoredKnowledgeBaseCurrent('99', registered, available)).toBe(false);
  });

  it('rejects a registered space the listing no longer returns', () => {
    expect(isStoredKnowledgeBaseCurrent('43', registered, available)).toBe(false);
  });

  it('rejects blank or missing selections', () => {
    expect(isStoredKnowledgeBaseCurrent(undefined, registered, available)).toBe(false);
    expect(isStoredKnowledgeBaseCurrent(null, registered, available)).toBe(false);
    expect(isStoredKnowledgeBaseCurrent('  ', registered, available)).toBe(false);
  });
});
