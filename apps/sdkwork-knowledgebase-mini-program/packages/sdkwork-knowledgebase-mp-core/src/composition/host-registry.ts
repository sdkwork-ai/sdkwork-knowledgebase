export interface SdkworkKnowledgebaseMpHostRegistration {
  hostId: string;
  platform: string;
}

export const sdkworkKnowledgebaseMpHostRegistry: readonly SdkworkKnowledgebaseMpHostRegistration[] = [
  { hostId: 'mp-weixin', platform: 'MP_WEIXIN' },
];

export function listSdkworkKnowledgebaseMpHosts() {
  return sdkworkKnowledgebaseMpHostRegistry;
}
