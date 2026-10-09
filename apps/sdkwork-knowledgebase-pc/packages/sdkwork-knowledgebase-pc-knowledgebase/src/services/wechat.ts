import type {
  KnowledgeWechatApplet,
  KnowledgeWechatArticle,
  KnowledgeWechatOfficialAccount,
} from 'sdkwork-knowledgebase-pc-core';
import {
  getKnowledgebaseAppSdkClient,
  isKnowledgebaseApiAvailable,
  KnowledgebaseErrorCodes,
  throwKnowledgebaseError,
} from 'sdkwork-knowledgebase-pc-core';

import { AIService } from './ai';
import {
  hydrateAppletSecrets,
  hydrateOfficialAccountSecrets,
  isDesktopSecureStorageAvailable,
  persistAppletSecrets,
  persistOfficialAccountSecrets,
} from './wechatCredentialStore';

function requireWechatSdk() {
  if (!isKnowledgebaseApiAvailable()) {
    throwKnowledgebaseError(KnowledgebaseErrorCodes.API_UNAVAILABLE_WECHAT);
  }
  return getKnowledgebaseAppSdkClient().client.knowledge.wechat;
}

function toOfficialAccount(account: KnowledgeWechatOfficialAccount): OfficialAccount {
  return {
    id: account.id,
    name: account.name,
    type: account.type === 'service' ? 'service' : 'subscription',
    avatar: account.avatar,
    description: account.description,
    appId: account.appId,
    appSecret: account.appSecret ?? '',
    serverUrl: account.serverUrl,
    token: account.token,
    encodingAesKey: account.encodingAesKey,
    encryptMode: account.encryptMode as OfficialAccount['encryptMode'],
    domainVerifyFileName: account.domainVerifyFileName,
    domainVerifyFileContent: account.domainVerifyFileContent,
    jsSecureDomains: account.jsSecureDomains,
    webAuthDomains: account.webAuthDomains,
    businessDomains: account.businessDomains,
    group: account.group,
  };
}

function fromOfficialAccount(account: OfficialAccount): KnowledgeWechatOfficialAccount {
  return {
    id: account.id,
    name: account.name,
    type: account.type,
    avatar: account.avatar,
    description: account.description,
    appId: account.appId,
    appSecret: account.appSecret || undefined,
    serverUrl: account.serverUrl,
    token: account.token,
    encodingAesKey: account.encodingAesKey,
    encryptMode: account.encryptMode,
    domainVerifyFileName: account.domainVerifyFileName,
    domainVerifyFileContent: account.domainVerifyFileContent,
    jsSecureDomains: account.jsSecureDomains,
    webAuthDomains: account.webAuthDomains,
    businessDomains: account.businessDomains,
    group: account.group,
  };
}

function toApplet(applet: KnowledgeWechatApplet): WechatAppletConfig {
  return {
    id: applet.id,
    name: applet.name,
    appId: applet.appId,
    originalId: applet.originalId,
    appSecret: applet.appSecret,
    path: applet.path,
    avatar: applet.avatar,
    group: applet.group,
    description: applet.description,
    requestDomain: applet.requestDomain,
    socketDomain: applet.socketDomain,
    uploadDomain: applet.uploadDomain,
    downloadDomain: applet.downloadDomain,
    udpDomain: applet.udpDomain,
    tcpDomain: applet.tcpDomain,
    businessDomain: applet.businessDomain,
    domainVerifyFileName: applet.domainVerifyFileName,
    domainVerifyFileContent: applet.domainVerifyFileContent,
    msgToken: applet.msgToken,
    msgEncodingAESKey: applet.msgEncodingAESKey,
    msgDataFormat: applet.msgDataFormat as WechatAppletConfig['msgDataFormat'],
    msgEncryptMode: applet.msgEncryptMode as WechatAppletConfig['msgEncryptMode'],
  };
}

function fromApplet(applet: WechatAppletConfig): KnowledgeWechatApplet {
  return {
    id: applet.id,
    name: applet.name,
    appId: applet.appId,
    originalId: applet.originalId,
    appSecret: applet.appSecret,
    path: applet.path,
    avatar: applet.avatar,
    group: applet.group,
    description: applet.description,
    requestDomain: applet.requestDomain,
    socketDomain: applet.socketDomain,
    uploadDomain: applet.uploadDomain,
    downloadDomain: applet.downloadDomain,
    udpDomain: applet.udpDomain,
    tcpDomain: applet.tcpDomain,
    businessDomain: applet.businessDomain,
    domainVerifyFileName: applet.domainVerifyFileName,
    domainVerifyFileContent: applet.domainVerifyFileContent,
    msgToken: applet.msgToken,
    msgEncodingAESKey: applet.msgEncodingAESKey,
    msgDataFormat: applet.msgDataFormat,
    msgEncryptMode: applet.msgEncryptMode,
  };
}

function toArticle(article: WechatArticle): KnowledgeWechatArticle {
  return {
    id: article.id,
    title: article.title,
    author: article.author,
    content: article.content,
    cover: article.cover,
    abstract: article.abstract,
  };
}

export interface OfficialAccount {
  id: string;
  name: string;
  type: 'subscription' | 'service';
  avatar: string;
  description?: string;
  appId: string;
  appSecret: string;
  serverUrl?: string;
  token?: string;
  encodingAesKey?: string;
  encryptMode?: 'plain' | 'compatible' | 'safe';
  domainVerifyFileName?: string;
  domainVerifyFileContent?: string;
  jsSecureDomains?: string[];
  webAuthDomains?: string[];
  businessDomains?: string[];
  group?: string;
}

export interface WechatAppletConfig {
  id: string;
  name: string;
  appId: string;
  originalId?: string;
  appSecret?: string;
  path: string;
  avatar: string;
  group?: string;
  description?: string;
  requestDomain?: string[];
  socketDomain?: string[];
  uploadDomain?: string[];
  downloadDomain?: string[];
  udpDomain?: string[];
  tcpDomain?: string[];
  businessDomain?: string[];
  domainVerifyFileName?: string;
  domainVerifyFileContent?: string;
  msgToken?: string;
  msgEncodingAESKey?: string;
  msgDataFormat?: 'json' | 'xml';
  msgEncryptMode?: 'plain' | 'compatible' | 'safe';
}

export interface WechatArticle {
  id: string;
  title: string;
  author: string;
  content?: string;
  cover?: string;
  abstract?: string;
  isOriginal?: boolean;
  commentType?: 'everyone' | 'follower' | 'none';
  coverZoom?: number;
  coverOffsetX?: number;
  coverOffsetY?: number;
  coverAspect?: '2.35' | '1:1';
}

export interface WechatCommandResult {
  accepted: boolean;
  status: string;
}

export class WechatService {
  static async getOfficialAccounts(): Promise<OfficialAccount[]> {
    const list = await requireWechatSdk().officialAccounts.list();
    const accounts = (list.items ?? []).map(toOfficialAccount);
    if (isDesktopSecureStorageAvailable()) {
      return Promise.all(accounts.map((account) => hydrateOfficialAccountSecrets(account)));
    }
    return accounts;
  }

  static async saveOfficialAccounts(accounts: OfficialAccount[]): Promise<boolean> {
    const sdk = requireWechatSdk();
    const prepared = isDesktopSecureStorageAvailable()
      ? await Promise.all(
          accounts.map(async (account) => {
            await persistOfficialAccountSecrets(account);
            return hydrateOfficialAccountSecrets(account);
          }),
        )
      : accounts;
    await sdk.officialAccounts.update({
      accounts: prepared.map(fromOfficialAccount),
    });
    return true;
  }

  static async getApplets(): Promise<WechatAppletConfig[]> {
    const list = await requireWechatSdk().applets.list();
    const applets = (list.items ?? []).map(toApplet);
    if (isDesktopSecureStorageAvailable()) {
      return Promise.all(applets.map((applet) => hydrateAppletSecrets(applet)));
    }
    return applets;
  }

  static async saveApplets(applets: WechatAppletConfig[]): Promise<boolean> {
    const sdk = requireWechatSdk();
    const prepared = isDesktopSecureStorageAvailable()
      ? await Promise.all(
          applets.map(async (applet) => {
            await persistAppletSecrets(applet);
            return hydrateAppletSecrets(applet);
          }),
        )
      : applets;
    await sdk.applets.update({
      applets: prepared.map(fromApplet),
    });
    return true;
  }

  static async listFanTags(accountId: string): Promise<Array<{ id: string; name: string; fanCount: number }>> {
    if (!accountId) {
      throwKnowledgebaseError(KnowledgebaseErrorCodes.WECHAT_INVALID_ARGS);
    }
    const list = await requireWechatSdk().officialAccounts.fanTags.list(accountId);
    return (list.items ?? []).map((tag) => ({
      id: tag.id,
      name: tag.name,
      fanCount: Number(tag.fanCount ?? 0),
    }));
  }

  static async publishArticles(
    selectedAccountIds: string[],
    articles: WechatArticle[],
    options?: {
      sendNotification?: boolean;
      groupNotification?: boolean;
      selectedGroupId?: string;
      scheduleTime?: string | null;
    },
  ): Promise<WechatCommandResult> {
    if (!selectedAccountIds.length || !articles.length) {
      throwKnowledgebaseError(KnowledgebaseErrorCodes.WECHAT_INVALID_ARGS);
    }
    return requireWechatSdk().articles.publish({
      accountIds: selectedAccountIds,
      articles: articles.map(toArticle),
      sendNotification: options?.sendNotification,
      groupNotification: options?.groupNotification,
      selectedGroupId: options?.selectedGroupId,
      scheduleTime: options?.scheduleTime ?? undefined,
    });
  }

  static async sendPreview(
    accountId: string,
    wechatIds: string[],
    articles: WechatArticle[],
  ): Promise<WechatCommandResult> {
    if (!accountId || !wechatIds.length || !articles.length) {
      throwKnowledgebaseError(KnowledgebaseErrorCodes.WECHAT_INVALID_ARGS);
    }
    return requireWechatSdk().articles.preview({
      accountId,
      wechatIds,
      articles: articles.map(toArticle),
    });
  }

  static async autoFormatContent(content: string, type: string): Promise<string> {
    requireWechatSdk();
    const wrapped = `<article data-wechat-format="${type}">${content}</article>`;
    return AIService.streamRewrite(wrapped, () => undefined);
  }
}
