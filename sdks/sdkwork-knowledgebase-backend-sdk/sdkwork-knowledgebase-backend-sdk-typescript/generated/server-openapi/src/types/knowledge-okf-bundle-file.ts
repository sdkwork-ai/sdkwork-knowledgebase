import type { OkfBundleFileKind } from './okf-bundle-file-kind';

export interface KnowledgeOkfBundleFile {
  id: string;
  spaceId: string;
  logicalPath: string;
  entryType: OkfBundleFileKind;
  artifactRole: string;
  driveBucket: string;
  driveObjectKey: string;
  checksumSha256Hex?: string | null;
  stagedImportRoot?: string;
  importId?: string;
}
