import { Node, mergeAttributes } from '@tiptap/core';

export const Audio = Node.create({
  name: 'audio',
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
      preload: {
        default: 'metadata'
      }
    };
  },

  parseHTML() {
    return [
      {
        tag: 'audio',
      },
    ];
  },

  renderHTML({ HTMLAttributes }) {
    return ['audio', mergeAttributes(HTMLAttributes, { style: 'width: 100%; outline: none; margin: 20px 0;' })];
  },

  addNodeView() {
    return ({ HTMLAttributes }) => {
      const dom = document.createElement('audio');
      Object.entries(HTMLAttributes).forEach(([key, value]) => {
        if (value !== null) {
          dom.setAttribute(key, value);
        }
      });
      dom.style.width = '100%';
      dom.style.outline = 'none';
      dom.style.margin = '20px 0';
      return {
        dom,
      };
    };
  },
});
