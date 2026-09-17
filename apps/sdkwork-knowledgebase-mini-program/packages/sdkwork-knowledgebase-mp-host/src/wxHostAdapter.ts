import type { KnowledgebaseMpHostAdapter, KnowledgebaseMpHostOutcome } from "@sdkwork/knowledgebase-mp-core/host";

interface WxApi {
  navigateTo(options: { fail?: () => void; url: string }): void;
  removeStorageSync?(key: string): void;
  scanCode(options: { fail?: () => void; success?: (result: { result: string }) => void }): void;
  showToast(options: { fail?: () => void; title: string }): void;
}

function resolveWxApi(): WxApi | null {
  const globalObject = globalThis as { wx?: WxApi };
  return globalObject.wx ?? null;
}

function unsupported<T>(): KnowledgebaseMpHostOutcome<T> {
  return { status: "unsupported" };
}

/**
 * WeChat mini program host adapter.
 *
 * Host adapters expose typed methods and stable error statuses and never own
 * login, token refresh, permission evaluation, business authorization, or raw
 * business API transport (`APP_CLIENT_ARCHITECTURE_ALIGNMENT_SPEC.md` section 9).
 */
export function createKnowledgebaseMpWxHostAdapter(): KnowledgebaseMpHostAdapter {
  const wx = resolveWxApi();
  return {
    platform: "MP_WEIXIN",
    navigateTo(pagePath) {
      if (!wx) {
        return Promise.resolve(unsupported<void>());
      }
      return new Promise((resolve) => {
        wx.navigateTo({
          fail: () => resolve({ status: "unavailable" }),
          success: () => resolve({ status: "ok", value: undefined }),
          url: pagePath.startsWith("/") ? pagePath : `/${pagePath}`,
        });
      });
    },
    readSecureValue() {
      // The mini program platform offers no secure enclave for session values;
      // the platform storage boundary is exposed by core/session instead.
      return Promise.resolve(unsupported<string>());
    },
    scanQrCode() {
      if (!wx) {
        return Promise.resolve(unsupported<string>());
      }
      return new Promise((resolve) => {
        wx.scanCode({
          fail: () => resolve({ status: "cancelled" }),
          success: (result) => resolve({ status: "ok", value: result.result }),
        });
      });
    },
    showToast(message) {
      if (!wx) {
        return Promise.resolve(unsupported<void>());
      }
      return new Promise((resolve) => {
        wx.showToast({
          fail: () => resolve({ status: "unavailable" }),
          success: () => resolve({ status: "ok", value: undefined }),
          title: message,
        });
      });
    },
  };
}
