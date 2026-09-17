/**
 * Payload-free status primitive shared by capability pages.
 *
 * Capability packages map their own data onto this payload-free view.
 */
export type SdkworkKnowledgebaseScreenStatus = 'loading' | 'ready' | 'empty' | 'error';

export function resolveSdkworkKnowledgebaseScreenStatus(
  itemCount: number,
  loading: boolean,
  errorMessage: string,
): SdkworkKnowledgebaseScreenStatus {
  if (loading) {
    return 'loading';
  }
  if (errorMessage.length > 0) {
    return 'error';
  }
  return itemCount === 0 ? 'empty' : 'ready';
}

export function defaultSdkworkKnowledgebaseStatusMessage(
  status: SdkworkKnowledgebaseScreenStatus,
): string {
  switch (status) {
    case 'loading':
      return 'Loading...';
    case 'empty':
      return 'Nothing here yet';
    case 'error':
      return 'Something went wrong';
    default:
      return '';
  }
}
