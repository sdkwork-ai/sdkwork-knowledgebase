import type { KnowledgebaseHostCapability } from "../host/hostAdapter";

export interface SdkworkKnowledgebaseH5HostRegistration {
  hostId: string;
  capabilities: readonly KnowledgebaseHostCapability[];
}

/**
 * Registered host profiles for this application root. The browser profile is
 * always present; Capacitor iOS/Android profiles are additive and must expose
 * the same capability set and method signatures.
 */
export const sdkworkKnowledgebaseH5HostRegistry: readonly SdkworkKnowledgebaseH5HostRegistration[] = [
  {
    hostId: 'browser',
    capabilities: ['clipboard', 'deepLinks', 'networkStatus', 'shareSheet'],
  },
  {
    hostId: 'capacitor',
    capabilities: ['clipboard', 'deepLinks', 'filePicker', 'networkStatus', 'secureStorage', 'shareSheet'],
  },
];

export function listSdkworkKnowledgebaseH5Hosts() {
  return sdkworkKnowledgebaseH5HostRegistry;
}
