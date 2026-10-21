// @vitest-environment jsdom

import React, { act } from 'react';
import { createRoot, type Root } from 'react-dom/client';
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import {
  configureKnowledgebaseAppSdk,
  KnowledgebaseAppError,
  KnowledgebaseErrorCodes,
  setKnowledgebaseApiEnabled,
} from 'sdkwork-knowledgebase-pc-core';

import { WechatAppletModal } from '../components/WechatAppletModal';
import { WechatAiImageModal } from '../components/WechatAiImageModal';
import { OfficialAccountModal } from '../components/OfficialAccountModal';
import { WechatPublishModal } from '../components/WechatPublishModal';
import { WechatPublishPage } from '../WechatPublishPage';
import { AIService } from './ai';
import { WechatService, type OfficialAccount } from './wechat';

const toastSpies = vi.hoisted(() => ({
  success: vi.fn(),
  error: vi.fn(),
  info: vi.fn(),
}));

vi.mock('react-i18next', () => {
  const t = (
    key: string,
    options?: { defaultValue?: string; returnObjects?: boolean },
  ) => options?.returnObjects ? [] : options?.defaultValue ?? key;
  return {
    useTranslation: () => ({ t }),
    Trans: ({ i18nKey }: { i18nKey: string }) => i18nKey,
  };
});

vi.mock('react-router-dom', () => ({
  useLocation: () => ({ state: null }),
  useNavigate: () => vi.fn(),
}));

vi.mock('../components/ui/toast-manager', () => ({
  toast: toastSpies,
}));

vi.mock('../TiptapEditor', async () => {
  const ReactModule = await import('react');
  return {
    TiptapEditor: ({ onOpenImageGallery }: { onOpenImageGallery?: () => void }) =>
      ReactModule.createElement(
        'button',
        { type: 'button', 'data-testid': 'open-image-library', onClick: onOpenImageGallery },
        'open-image-library',
      ),
  };
});

vi.mock('../WechatArticleSettings', () => ({
  WechatArticleSettings: () => null,
}));

vi.mock('../components/AssetLibraryModal', async () => {
  const ReactModule = await import('react');
  return {
    AssetLibraryModal: ({ isOpen, kbId }: { isOpen: boolean; kbId?: string | null }) =>
      ReactModule.createElement('div', {
        'data-testid': 'asset-library-probe',
        'data-open': String(isOpen),
        'data-kb-id': kbId ?? '',
      }),
  };
});

vi.mock('./knowledgeFileUploadService', () => ({
  resolvePrimaryKnowledgebaseKbId: () => 'kb-real',
  uploadKnowledgebaseMediaUrl: vi.fn(),
}));

const officialAccount: OfficialAccount = {
  id: 'official-1',
  name: 'Official account',
  type: 'service',
  avatar: 'OA',
  appId: 'wx-official-1',
  appSecret: 'secret',
};

const secondaryOfficialAccount: OfficialAccount = {
  id: 'official-2',
  name: 'Secondary official account',
  type: 'subscription',
  avatar: 'OA2',
  appId: 'wx-official-2',
  appSecret: 'secondary-secret',
};

const applet = {
  id: 'applet-1',
  name: 'Real applet from SDK',
  appId: 'wx-applet-1',
  path: 'pages/index/index',
  avatar: 'APP',
};

function findButton(label: string): HTMLButtonElement {
  const button = Array.from(document.querySelectorAll('button')).find((candidate) =>
    candidate.textContent?.includes(label),
  );
  if (!(button instanceof HTMLButtonElement)) {
    throw new Error(`Button not found: ${label}`);
  }
  return button;
}

function setInputValue(input: HTMLInputElement, value: string): void {
  const setter = Object.getOwnPropertyDescriptor(
    HTMLInputElement.prototype,
    'value',
  )?.set;
  setter?.call(input, value);
  input.dispatchEvent(new Event('input', { bubbles: true }));
  input.dispatchEvent(new Event('change', { bubbles: true }));
}

async function click(element: HTMLElement): Promise<void> {
  await act(async () => {
    element.dispatchEvent(new MouseEvent('click', { bubbles: true }));
    await Promise.resolve();
  });
}

describe('WeChat async failure UI', () => {
  let container: HTMLDivElement;
  let root: Root;

  beforeEach(() => {
    (globalThis as Record<string, unknown>).IS_REACT_ACT_ENVIRONMENT = true;
    container = document.createElement('div');
    document.body.appendChild(container);
    root = createRoot(container);
    localStorage.clear();
    configureKnowledgebaseAppSdk({
      client: {} as never,
      setTokenManager() {
        // The interaction tests stub the service boundary; the registry still requires a client.
      },
    });
    setKnowledgebaseApiEnabled(false);
    vi.restoreAllMocks();
    Object.values(toastSpies).forEach((spy) => spy.mockReset());
  });

  afterEach(async () => {
    await act(async () => {
      root.unmount();
    });
    document.body.innerHTML = '';
    setKnowledgebaseApiEnabled(false);
  });

  it('keeps the applet editor open and withholds success when SDK persistence fails', async () => {
    vi.spyOn(WechatService, 'getApplets').mockResolvedValue([]);
    vi.spyOn(WechatService, 'saveApplets').mockRejectedValue(
      new KnowledgebaseAppError(KnowledgebaseErrorCodes.API_UNAVAILABLE_WECHAT),
    );

    await act(async () => {
      root.render(React.createElement(WechatAppletModal, {
        onClose: vi.fn(),
        onConfirm: vi.fn(),
      }));
      await Promise.resolve();
    });

    await click(findButton('selectApplet'));
    await click(findButton('addNewApplet'));
    const nameInput = document.querySelector<HTMLInputElement>(
      'input[placeholder="appNamePlaceholder"]',
    );
    const appIdInput = document.querySelector<HTMLInputElement>(
      'input[placeholder="appIdPlaceholder"]',
    );
    expect(nameInput).not.toBeNull();
    expect(appIdInput).not.toBeNull();
    await act(async () => {
      setInputValue(nameInput!, 'Real applet');
      setInputValue(appIdInput!, 'wx-applet');
    });
    await click(findButton('saveConfig'));

    expect(WechatService.saveApplets).toHaveBeenCalledOnce();
    expect(toastSpies.success).not.toHaveBeenCalled();
    expect(document.querySelector('input[placeholder="appNamePlaceholder"]')).not.toBeNull();
    expect(document.querySelector('[role="alert"]')?.textContent).toBeTruthy();
  });

  it('shows applets that arrive from the SDK after the manager first mounted closed', async () => {
    vi.spyOn(WechatService, 'getApplets').mockResolvedValue([applet]);

    await act(async () => {
      root.render(React.createElement(WechatAppletModal, {
        onClose: vi.fn(),
        onConfirm: vi.fn(),
      }));
      await Promise.resolve();
    });
    await click(findButton('selectApplet'));

    expect(document.body.textContent).toContain(applet.name);
  });

  it('keeps the official account modal open and displays a typed save error', async () => {
    const onClose = vi.fn();
    const rejection = Promise.reject(
      new KnowledgebaseAppError(KnowledgebaseErrorCodes.API_UNAVAILABLE_WECHAT),
    );
    void rejection.catch(() => undefined);
    const onConfirm = vi.fn(() => rejection);

    await act(async () => {
      root.render(React.createElement(OfficialAccountModal, {
        isOpen: true,
        onClose,
        onConfirm,
        initialOfficialAccounts: [officialAccount],
        initialSelectedAccountIds: [officialAccount.id],
        initialOaGroups: [],
      }));
    });
    await click(findButton('confirmAndContinue'));

    expect(onConfirm).toHaveBeenCalledOnce();
    expect(onClose).not.toHaveBeenCalled();
    expect(toastSpies.success).not.toHaveBeenCalled();
    expect(document.querySelector('[role="alert"]')?.textContent).toBeTruthy();
  });

  it('shows a typed error when official account loading fails', async () => {
    const error = new KnowledgebaseAppError(
      KnowledgebaseErrorCodes.API_UNAVAILABLE_WECHAT,
    );
    vi.spyOn(WechatService, 'getOfficialAccounts').mockRejectedValue(error);

    await act(async () => {
      root.render(React.createElement(WechatPublishPage));
      await Promise.resolve();
      await Promise.resolve();
    });

    expect(document.querySelector('[role="alert"]')?.textContent).toBeTruthy();
  });

  it('shows official accounts that load after the modal first mounted closed', async () => {
    vi.spyOn(WechatService, 'getOfficialAccounts').mockResolvedValue([officialAccount]);

    await act(async () => {
      root.render(React.createElement(WechatPublishPage));
      await Promise.resolve();
    });
    const selector = document.querySelector<HTMLElement>('[title="switchManageOA"]');
    expect(selector).not.toBeNull();
    await click(selector!);

    const accountNameOccurrences = document.body.textContent?.split(officialAccount.name).length ?? 0;
    expect(accountNameOccurrences).toBeGreaterThan(2);
  });

  it('does not expose unsaved official account selection when persistence fails', async () => {
    vi.spyOn(WechatService, 'getOfficialAccounts').mockResolvedValue([
      officialAccount,
      secondaryOfficialAccount,
    ]);
    let rejectSave: (error: unknown) => void = () => undefined;
    vi.spyOn(WechatService, 'saveOfficialAccounts').mockReturnValue(
      new Promise<boolean>((_resolve, reject) => {
        rejectSave = reject;
      }),
    );

    await act(async () => {
      root.render(React.createElement(WechatPublishPage));
      await Promise.resolve();
    });
    const selector = document.querySelector<HTMLElement>('[title="switchManageOA"]');
    expect(selector).not.toBeNull();
    await click(selector!);

    const accountCards = Array.from(document.querySelectorAll<HTMLElement>('div')).filter(
      (element) => element.classList.contains('cursor-pointer')
        && element.classList.contains('relative'),
    );
    const primaryCard = accountCards.find((element) =>
      element.textContent?.includes(officialAccount.appId),
    );
    const secondaryCard = accountCards.find((element) =>
      element.textContent?.includes(secondaryOfficialAccount.appId),
    );
    expect(primaryCard).not.toBeUndefined();
    expect(secondaryCard).not.toBeUndefined();
    await click(primaryCard!);
    await click(secondaryCard!);
    await click(findButton('confirmAndContinue'));

    expect(selector?.textContent).toContain(officialAccount.name);
    expect(selector?.textContent).not.toContain(secondaryOfficialAccount.name);

    await act(async () => {
      rejectSave(
        new KnowledgebaseAppError(KnowledgebaseErrorCodes.API_UNAVAILABLE_WECHAT),
      );
      await Promise.resolve();
    });

    expect(document.querySelector('[role="alert"]')?.textContent).toBeTruthy();
    expect(selector?.textContent).toContain(officialAccount.name);
  });

  it('keeps preview open and displays the typed error when SDK sending fails', async () => {
    setKnowledgebaseApiEnabled(true);
    vi.spyOn(WechatService, 'getOfficialAccounts').mockResolvedValue([officialAccount]);
    vi.spyOn(WechatService, 'sendPreview').mockRejectedValue(
      new KnowledgebaseAppError(KnowledgebaseErrorCodes.API_UNAVAILABLE_WECHAT),
    );

    await act(async () => {
      root.render(React.createElement(WechatPublishPage));
      await Promise.resolve();
    });
    await click(findButton('sendPreview'));
    const recipientInput = document.querySelector<HTMLInputElement>(
      '#wechat-preview-recipient-input',
    );
    expect(recipientInput).not.toBeNull();
    await act(async () => {
      setInputValue(recipientInput!, 'wx-preview-user');
    });
    await click(document.querySelector<HTMLButtonElement>('#wechat-preview-validate')!);

    expect(WechatService.sendPreview).toHaveBeenCalledOnce();
    expect(toastSpies.success).not.toHaveBeenCalled();
    expect(document.querySelector('#wechat-preview-recipient-input')).not.toBeNull();
    expect(document.querySelector('[role="alert"]')?.textContent).toBeTruthy();
  });

  it('shows a typed fan-tag loading error instead of silently replacing it', async () => {
    vi.spyOn(WechatService, 'listFanTags').mockRejectedValue(
      new KnowledgebaseAppError(KnowledgebaseErrorCodes.API_UNAVAILABLE_WECHAT),
    );

    await act(async () => {
      root.render(React.createElement(WechatPublishModal, {
        isOpen: true,
        onClose: vi.fn(),
        isPublishing: false,
        onConfirmPublish: vi.fn().mockResolvedValue(undefined),
        officialAccountId: officialAccount.id,
        officialAccountName: officialAccount.name,
        officialAccountType: officialAccount.type,
      }));
      await Promise.resolve();
    });

    expect(document.querySelector('[role="alert"]')?.textContent).toBeTruthy();
    const groupToggle = document.querySelector<HTMLButtonElement>('#toggle-group-send');
    expect(groupToggle?.disabled).toBe(true);
    await click(groupToggle!);
    expect(document.querySelector('option[value="all"]')).toBeNull();
  });

  it('passes the active knowledgebase id to the real asset library', async () => {
    setKnowledgebaseApiEnabled(true);
    vi.spyOn(WechatService, 'getOfficialAccounts').mockResolvedValue([officialAccount]);

    await act(async () => {
      root.render(React.createElement(WechatPublishPage));
      await Promise.resolve();
    });
    await click(document.querySelector<HTMLElement>('[data-testid="open-image-library"]')!);

    const probe = document.querySelector('[data-testid="asset-library-probe"]');
    expect(probe?.getAttribute('data-kb-id')).toBe('kb-real');
    expect(probe?.getAttribute('data-open')).toBe('true');
  });

  it('does not open an empty asset library when the API is unavailable', async () => {
    vi.spyOn(WechatService, 'getOfficialAccounts').mockResolvedValue([]);

    await act(async () => {
      root.render(React.createElement(WechatPublishPage));
      await Promise.resolve();
    });
    await click(document.querySelector<HTMLElement>('[data-testid="open-image-library"]')!);

    const probe = document.querySelector('[data-testid="asset-library-probe"]');
    expect(probe?.getAttribute('data-open')).toBe('false');
    expect(toastSpies.error).toHaveBeenCalled();
  });

  it('sends the selected aspect ratio and style to real image generation', async () => {
    setKnowledgebaseApiEnabled(true);
    const generateImage = vi.spyOn(AIService, 'generateImage').mockResolvedValue({
      url: 'https://media.example.test/generated.png',
      resolution: '1600x900',
      suggestions: [],
      similars: [],
    });

    await act(async () => {
      root.render(React.createElement(WechatAiImageModal, {
        isOpen: true,
        onClose: vi.fn(),
        onConfirm: vi.fn(),
      }));
    });

    const aspectSelect = document.querySelector<HTMLSelectElement>(
      'select[aria-label="Aspect ratio"]',
    );
    const styleSelect = document.querySelector<HTMLSelectElement>(
      'select[aria-label="Image style"]',
    );
    const promptInput = document.querySelector<HTMLTextAreaElement>('textarea');
    expect(aspectSelect).not.toBeNull();
    expect(styleSelect).not.toBeNull();
    expect(promptInput).not.toBeNull();

    await act(async () => {
      const selectSetter = Object.getOwnPropertyDescriptor(
        HTMLSelectElement.prototype,
        'value',
      )?.set;
      selectSetter?.call(aspectSelect, '16:9');
      aspectSelect!.dispatchEvent(new Event('change', { bubbles: true }));
      selectSetter?.call(styleSelect, 'photography');
      styleSelect!.dispatchEvent(new Event('change', { bubbles: true }));

      const textAreaSetter = Object.getOwnPropertyDescriptor(
        HTMLTextAreaElement.prototype,
        'value',
      )?.set;
      textAreaSetter?.call(promptInput, 'A real product screenshot');
      promptInput!.dispatchEvent(new Event('input', { bubbles: true }));
      promptInput!.dispatchEvent(new Event('change', { bubbles: true }));
    });

    await click(document.querySelector<HTMLElement>('[aria-label="Generate image"]')!);

    expect(generateImage).toHaveBeenCalledWith(
      'A real product screenshot',
      '16:9',
      'photography',
      undefined,
    );
  });
});
