export interface OkfBundleExportRequest {
  spaceId: string;
  exportType: string;
  stageForImport?: boolean;
  importId?: string;
}
