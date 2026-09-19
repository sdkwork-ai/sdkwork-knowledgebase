/**
 * Upload declaration constants for the Knowledgebase PC application root.
 *
 * Source of truth: `apps/sdkwork-knowledgebase-pc/specs/upload.declaration.json`
 * (DRIVE_SPEC.md §18). Upload call sites MUST consume these constants instead of
 * repeating the declared literals inline — §18.3 forbids duplicating a declared
 * value as a bare literal, because a duplicate silently diverges from the
 * declaration that the gate validates.
 *
 * All six entries share one `(appResourceType, scene, source)` identity and
 * differ only by `uploadProfileCode`, because the real call site selects the
 * upload profile from the picked file's content type (see
 * `inferUploaderProfile` in `knowledgeFileUploadService.ts`).
 */

export interface KnowledgebaseUploadDeclarationEntry {
  readonly appResourceType: string;
  readonly appResourceIdKind: 'application' | 'entity' | 'draft';
  readonly scene: string;
  readonly source: string;
  readonly uploadProfileCode: string;
  readonly retention: 'long_term' | 'temporary';
  readonly retentionTtlSeconds?: number;
  readonly purpose: string;
}

export const KNOWLEDGEBASE_PC_APP_ID = 'sdkwork-knowledgebase-pc' as const;
export const KNOWLEDGEBASE_PC_UPLOAD_SOURCE = 'sdkwork-knowledgebase-pc' as const;
export const KNOWLEDGEBASE_PC_UPLOAD_APP_RESOURCE_TYPE = 'knowledgebase.document_source' as const;
export const KNOWLEDGEBASE_PC_UPLOAD_SCENE = 'document-upload' as const;

export const KNOWLEDGEBASE_PC_DOCUMENT_UPLOAD = {
  appResourceType: KNOWLEDGEBASE_PC_UPLOAD_APP_RESOURCE_TYPE,
  appResourceIdKind: 'entity',
  scene: KNOWLEDGEBASE_PC_UPLOAD_SCENE,
  source: KNOWLEDGEBASE_PC_UPLOAD_SOURCE,
  uploadProfileCode: 'document',
  retention: 'long_term',
  purpose:
    'PDF source document uploaded into a knowledge base so it can be imported and indexed.',
} as const satisfies KnowledgebaseUploadDeclarationEntry;

export const KNOWLEDGEBASE_PC_TEXT_UPLOAD = {
  appResourceType: KNOWLEDGEBASE_PC_UPLOAD_APP_RESOURCE_TYPE,
  appResourceIdKind: 'entity',
  scene: KNOWLEDGEBASE_PC_UPLOAD_SCENE,
  source: KNOWLEDGEBASE_PC_UPLOAD_SOURCE,
  uploadProfileCode: 'text',
  retention: 'long_term',
  purpose:
    'Plain-text, Markdown, or source-code file uploaded into a knowledge base so it can be imported and indexed.',
} as const satisfies KnowledgebaseUploadDeclarationEntry;

export const KNOWLEDGEBASE_PC_IMAGE_UPLOAD = {
  appResourceType: KNOWLEDGEBASE_PC_UPLOAD_APP_RESOURCE_TYPE,
  appResourceIdKind: 'entity',
  scene: KNOWLEDGEBASE_PC_UPLOAD_SCENE,
  source: KNOWLEDGEBASE_PC_UPLOAD_SOURCE,
  uploadProfileCode: 'image',
  retention: 'long_term',
  purpose: 'Image source uploaded into a knowledge base so it can be imported and indexed.',
} as const satisfies KnowledgebaseUploadDeclarationEntry;

export const KNOWLEDGEBASE_PC_AUDIO_UPLOAD = {
  appResourceType: KNOWLEDGEBASE_PC_UPLOAD_APP_RESOURCE_TYPE,
  appResourceIdKind: 'entity',
  scene: KNOWLEDGEBASE_PC_UPLOAD_SCENE,
  source: KNOWLEDGEBASE_PC_UPLOAD_SOURCE,
  uploadProfileCode: 'audio',
  retention: 'long_term',
  purpose: 'Audio source uploaded into a knowledge base so it can be transcribed and indexed.',
} as const satisfies KnowledgebaseUploadDeclarationEntry;

export const KNOWLEDGEBASE_PC_VIDEO_UPLOAD = {
  appResourceType: KNOWLEDGEBASE_PC_UPLOAD_APP_RESOURCE_TYPE,
  appResourceIdKind: 'entity',
  scene: KNOWLEDGEBASE_PC_UPLOAD_SCENE,
  source: KNOWLEDGEBASE_PC_UPLOAD_SOURCE,
  uploadProfileCode: 'video',
  retention: 'long_term',
  purpose: 'Video source uploaded into a knowledge base so it can be transcribed and indexed.',
} as const satisfies KnowledgebaseUploadDeclarationEntry;

export const KNOWLEDGEBASE_PC_ATTACHMENT_UPLOAD = {
  appResourceType: KNOWLEDGEBASE_PC_UPLOAD_APP_RESOURCE_TYPE,
  appResourceIdKind: 'entity',
  scene: KNOWLEDGEBASE_PC_UPLOAD_SCENE,
  source: KNOWLEDGEBASE_PC_UPLOAD_SOURCE,
  uploadProfileCode: 'attachment',
  retention: 'long_term',
  purpose:
    'Knowledge-base source file whose content shape is not one of the more specific profiles, stored as a generic attachment.',
} as const satisfies KnowledgebaseUploadDeclarationEntry;

export const KNOWLEDGEBASE_PC_UPLOAD_DECLARATIONS: readonly KnowledgebaseUploadDeclarationEntry[] = [
  KNOWLEDGEBASE_PC_DOCUMENT_UPLOAD,
  KNOWLEDGEBASE_PC_TEXT_UPLOAD,
  KNOWLEDGEBASE_PC_IMAGE_UPLOAD,
  KNOWLEDGEBASE_PC_AUDIO_UPLOAD,
  KNOWLEDGEBASE_PC_VIDEO_UPLOAD,
  KNOWLEDGEBASE_PC_ATTACHMENT_UPLOAD,
];
