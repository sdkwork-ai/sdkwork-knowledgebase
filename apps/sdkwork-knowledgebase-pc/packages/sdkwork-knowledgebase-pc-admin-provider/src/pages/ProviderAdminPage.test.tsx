// @vitest-environment jsdom
import { cleanup, fireEvent, render, screen } from '@testing-library/react';
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';

const testState = vi.hoisted(() => ({
  apiAvailable: true,
  canAccess: true,
}));

const translation = {
  i18n: {
    language: 'en-US',
    resolvedLanguage: 'en-US',
    hasResourceBundle: () => true,
    addResourceBundle: () => undefined,
    getFixedT: () => (key: string) => key,
  },
  t: (key: string) => key,
};

vi.mock('react-i18next', () => ({ useTranslation: () => translation }));
vi.mock('react-router-dom', () => ({ useNavigate: () => vi.fn() }));
vi.mock('sdkwork-knowledgebase-pc-admin-core', () => ({
  canAccessKnowledgebaseAdminConsole: () => testState.canAccess,
  isKnowledgebaseBackendApiAvailable: () => testState.apiAvailable,
  extractSdkWorkListItems: (payload: { items?: unknown[] }) => payload.items ?? [],
  readStringField: (payload: Record<string, unknown>, key: string) => String(payload[key] ?? ''),
  getKnowledgebaseBackendSdkClient: vi.fn(),
}));
vi.mock('sdkwork-knowledgebase-pc-core', () => ({
  useKnowledgebaseRuntime: () => ({ session: {} }),
  useKnowledgebaseSessionSnapshot: () => ({ context: { permissionScope: ['knowledge.platform.manage'] } }),
  resolveUserFacingErrorMessage: () => 'safe-provider-error',
}));

import { ProviderAdminPage } from './ProviderAdminPage';
import { providerAdminService } from '../services/providerAdminService';

const firstSpace = { id: 'space-1', name: 'Operations', knowledgeMode: 'external' };

describe('ProviderAdminPage', () => {
  beforeEach(() => {
    testState.apiAvailable = true;
    testState.canAccess = true;
    vi.spyOn(providerAdminService, 'listSpaces').mockResolvedValue({ items: [firstSpace], hasMore: false });
    vi.spyOn(providerAdminService, 'listBindings').mockResolvedValue({ items: [], hasMore: false });
    vi.spyOn(providerAdminService, 'listCredentials').mockResolvedValue({ items: [], hasMore: false });
    vi.spyOn(providerAdminService, 'listMigrations').mockResolvedValue({ items: [], hasMore: false });
  });

  afterEach(() => {
    cleanup();
    vi.restoreAllMocks();
  });

  it('renders permission denied without calling Provider services', () => {
    testState.canAccess = false;
    render(<ProviderAdminPage />);
    expect(screen.getByText('forbiddenTitle')).toBeTruthy();
    expect(providerAdminService.listSpaces).not.toHaveBeenCalled();
  });

  it('renders a loading state while the selected space page is pending', async () => {
    let resolveBindings: ((value: { items: []; hasMore: false }) => void) | undefined;
    vi.spyOn(providerAdminService, 'listBindings').mockReturnValue(new Promise((resolve) => {
      resolveBindings = resolve;
    }));
    render(<ProviderAdminPage />);
    expect(await screen.findByText('loading')).toBeTruthy();
    resolveBindings?.({ items: [], hasMore: false });
    expect(await screen.findByText('emptyBindings')).toBeTruthy();
  });

  it('renders empty and safe error states', async () => {
    render(<ProviderAdminPage />);
    expect(await screen.findByText('emptyBindings')).toBeTruthy();
    vi.spyOn(providerAdminService, 'listBindings').mockRejectedValue(new Error('raw upstream body'));
    fireEvent.click(screen.getByRole('button', { name: 'refresh' }));
    expect(await screen.findByText('safe-provider-error')).toBeTruthy();
    expect(screen.queryByText('raw upstream body')).toBeNull();
  });

  it('renders a capability-aware Binding row', async () => {
    vi.spyOn(providerAdminService, 'listBindings').mockResolvedValue({
      hasMore: false,
      items: [{
        id: 'binding-1', uuid: 'binding-uuid', tenantId: '1', organizationId: '1', spaceId: 'space-1',
        implementationId: 'engine.knowledge.external.dify', remoteResourceType: 'dataset',
        remoteResourceId: 'remote-1', lifecycleState: 'testing', capabilitySnapshot: ['health', 'search'],
        capabilitySnapshotVersion: '1', createdBy: 'operator', updatedBy: 'operator',
        createdAt: '2026-07-20T00:00:00Z', updatedAt: '2026-07-20T00:00:00Z', version: '3',
      }],
    });
    render(<ProviderAdminPage />);
    expect(await screen.findByText('engine.knowledge.external.dify')).toBeTruthy();
    const activate = screen.getByRole('button', { name: 'activate' });
    expect(activate.hasAttribute('disabled')).toBe(false);
  });
});
