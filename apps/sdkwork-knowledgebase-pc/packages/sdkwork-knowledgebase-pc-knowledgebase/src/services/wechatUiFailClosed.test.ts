import { readFileSync } from 'node:fs';
import React from 'react';
import { renderToStaticMarkup } from 'react-dom/server';
import { afterEach, describe, expect, it, vi } from 'vitest';
import { setKnowledgebaseApiEnabled } from 'sdkwork-knowledgebase-pc-core';

import { WechatAiImageModal } from '../components/WechatAiImageModal';
import { WechatAppletModal } from '../components/WechatAppletModal';
import { WechatScanModal } from '../components/WechatScanModal';

vi.mock('react-i18next', () => ({
  useTranslation: () => ({
    t: (key: string) => key,
  }),
}));

function readSource(relativePath: string): string {
  return readFileSync(new URL(relativePath, import.meta.url), 'utf8');
}

afterEach(() => {
  setKnowledgebaseApiEnabled(false);
  vi.unstubAllEnvs();
});

describe('WeChat interaction fail-closed UI', () => {
  it('renders scan upload as unavailable without a simulation action', () => {
    const markup = renderToStaticMarkup(
      React.createElement(WechatScanModal, {
        isOpen: true,
        onClose: vi.fn(),
      }),
    );

    expect(markup).toContain('wechatScanUnavailable');
    expect(markup).not.toContain('mockScanUpload');
  });

  it('renders image generation as unavailable instead of seeding demo media', () => {
    setKnowledgebaseApiEnabled(false);
    vi.stubEnv('VITE_SDKWORK_KNOWLEDGEBASE_ENABLE_DEMO_MODE', 'true');

    const markup = renderToStaticMarkup(
      React.createElement(WechatAiImageModal, {
        isOpen: true,
        onClose: vi.fn(),
        onConfirm: vi.fn(),
      }),
    );

    expect(markup).toContain('wechatAiImageUnavailable');
    expect(markup).not.toContain('images.unsplash.com');
  });

  it('offers user-supplied applet image URLs without fixed upload or library results', () => {
    const markup = renderToStaticMarkup(
      React.createElement(WechatAppletModal, {
        onClose: vi.fn(),
        onConfirm: vi.fn(),
      }),
    );

    expect(markup).toContain('imageUrlPlaceholder');
    expect(markup).not.toContain('images.unsplash.com');
  });

  it('contains no WeChat demo stores, synthetic command success, or scan simulator', () => {
    const serviceSource = readSource('./wechat.ts');
    const aiImageSource = readSource('../components/WechatAiImageModal.tsx');
    const appletSource = readSource('../components/WechatAppletModal.tsx');
    const publishPageSource = readSource('../WechatPublishPage.tsx');

    expect(serviceSource).not.toMatch(/demoOfficialAccounts|demoApplets/);
    expect(serviceSource).not.toContain("{ accepted: true, status: 'completed' }");
    expect(aiImageSource).not.toMatch(/OFFLINE_DEMO_MESSAGES|images\.unsplash\.com/);
    expect(appletSource).not.toMatch(/shouldUseKnowledgebaseDemoFallback|images\.unsplash\.com/);
    expect(publishPageSource).not.toMatch(
      /triggerScanSimulation|mobileCovers|scanStatus|scannedCover/,
    );
  });

  it('never swallows a failed desktop secure credential write', () => {
    const credentialStoreSource = readSource('./wechatCredentialStore.ts');

    expect(credentialStoreSource).toContain("'write_secure_session_value'");
    // A silenced write failure would let saveOfficialAccounts fall back to the
    // plaintext form secret and persist it server-side; the write must stay
    // fail-closed (typed rejection, no `.catch(() => undefined)`).
    expect(credentialStoreSource).not.toMatch(
      /write_secure_session_value[\s\S]{0,400}\.catch\(\s*\(\s*\)\s*=>\s*undefined\s*\)/,
    );
  });
});
