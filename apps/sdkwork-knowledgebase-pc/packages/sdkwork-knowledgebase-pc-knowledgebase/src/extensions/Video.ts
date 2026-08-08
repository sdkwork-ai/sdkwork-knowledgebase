import { Node, mergeAttributes } from '@tiptap/core';

export const Video = Node.create({
  name: 'video',
  group: 'block',
  selectable: true,
  draggable: true,
  atom: true,

  addAttributes() {
    return {
      src: {
        default: null,
      },
      driveSpaceId: {
        default: null,
        parseHTML: element => element.getAttribute('data-drive-space-id'),
        renderHTML: attributes => attributes.driveSpaceId
          ? { 'data-drive-space-id': attributes.driveSpaceId }
          : {},
      },
      driveNodeId: {
        default: null,
        parseHTML: element => element.getAttribute('data-drive-node-id'),
        renderHTML: attributes => attributes.driveNodeId
          ? { 'data-drive-node-id': attributes.driveNodeId }
          : {},
      },
      driveUri: {
        default: null,
        parseHTML: element => element.getAttribute('data-drive-uri'),
        renderHTML: attributes => attributes.driveUri
          ? { 'data-drive-uri': attributes.driveUri }
          : {},
      },
      controls: {
        default: true,
      },
      class: {
        default: 'w-full rounded-xl my-4 border border-[var(--color-kb-panel-border)] shadow-sm'
      }
    };
  },

  parseHTML() {
    return [
      {
        tag: 'video',
      },
    ];
  },

  renderHTML({ HTMLAttributes }) {
    return ['video', mergeAttributes(HTMLAttributes)];
  },

  addNodeView() {
    return ({ HTMLAttributes }) => {
      const dom = document.createElement('video');
      Object.entries(HTMLAttributes).forEach(([key, value]) => {
        if (value !== null) {
          dom.setAttribute(key, value);
        }
      });
      dom.className = 'w-full rounded-xl my-4 border border-[var(--color-kb-panel-border)] shadow-sm';
      dom.style.maxWidth = '100%';
      return {
        dom,
      };
    };
  },
});
