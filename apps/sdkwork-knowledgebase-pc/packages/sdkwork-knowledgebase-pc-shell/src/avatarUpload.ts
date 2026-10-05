import {
  createDriveNodesImagePreviewReader,
  createDriveUploadImageService,
  type DriveUploadImageService,
} from '@sdkwork/drive-upload-image-core';
import { KNOWLEDGEBASE_PC_AVATAR_UPLOAD } from '@sdkwork/sdkwork-knowledgebase-pc-core';

/**
 * Shell-side user-avatar capability for the profile modal.
 *
 * A thin facade over the shared `@sdkwork/drive-upload-image-core` factory:
 * the service binds this application's declared avatar intent constant
 * (`KNOWLEDGEBASE_PC_AVATAR_UPLOAD`, `DRIVE_SPEC.md` §18 — the service layer,
 * not the UI, supplies declared values) to the composed `drive.uploader`
 * surface, and `drive://` avatar references resolve their display through the
 * shared bounded same-origin preview reader over `drive.nodes.content.retrieve`
 * (§8). Preset/emoji avatars keep their own URL or literal value.
 */

export interface KnowledgebaseAvatarUploadClient {
  uploader: Parameters<typeof createDriveUploadImageService>[0]['uploader'];
  drive: { nodes: Parameters<typeof createDriveNodesImagePreviewReader>[0] };
}

/**
 * Builds the shared `DriveUploadImageService` for this application's declared
 * avatar upload intent. Created per composition from the runtime's drive
 * client; construction is pure, so the caller may memoize freely.
 */
export function createKnowledgebaseAvatarUploadService(
  driveClient: KnowledgebaseAvatarUploadClient,
): DriveUploadImageService {
  return createDriveUploadImageService({
    uploader: driveClient.uploader,
    declaration: KNOWLEDGEBASE_PC_AVATAR_UPLOAD,
    previewReader: createDriveNodesImagePreviewReader(driveClient.drive.nodes),
  });
}

/**
 * Uploads the picked avatar image for the signed-in user and returns the
 * `drive://` reference stored in the local profile (`DRIVE_SPEC.md` §9: the
 * stable Drive reference, never presigned or delivery URLs).
 */
export async function uploadKnowledgebaseUserAvatar(
  service: DriveUploadImageService,
  input: {
    appResourceId: string;
    file: File;
  },
): Promise<string> {
  const uploaded = await service.upload({
    file: {
      size: input.file.size,
      type: input.file.type || undefined,
      name: input.file.name,
      arrayBuffer: () => input.file.arrayBuffer(),
    },
    appResourceId: input.appResourceId,
  });
  return uploaded.uri;
}

/**
 * Transient display URL for a stored avatar value. Drive-backed references
 * resolve through the shared bounded preview reader; every other value
 * (preset URL, emoji, external URL) is displayable as-is. The result is
 * presentation-only state and never persisted.
 */
export async function resolveKnowledgebaseAvatarDisplayUrl(
  service: DriveUploadImageService,
  avatar: string | undefined,
): Promise<string | undefined> {
  if (!avatar || !avatar.startsWith('drive://')) {
    return avatar;
  }
  const previewUrl = await service.resolvePreview({ uri: avatar });
  return previewUrl ?? undefined;
}
