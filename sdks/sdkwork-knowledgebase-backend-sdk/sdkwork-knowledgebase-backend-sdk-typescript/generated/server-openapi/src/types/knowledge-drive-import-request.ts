export interface KnowledgeDriveImportRequest {
  spaceId: string;
  title: string;
  driveBucket: string;
  driveObjectKey: string;
  idempotencyKey: string;
  language?: string | null;
  driveSpaceId?: string | null;
  driveNodeId?: string | null;
  driveStorageProviderId: string;
}
