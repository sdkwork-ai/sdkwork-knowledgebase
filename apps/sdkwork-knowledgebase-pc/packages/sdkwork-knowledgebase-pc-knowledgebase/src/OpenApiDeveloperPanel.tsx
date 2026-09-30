import React, { useState } from 'react';
import { useTranslation } from 'react-i18next';
import {
  Code2,
  KeyRound,
  Copy,
  Check,
  Building2,
  ExternalLink,
  Terminal,
} from 'lucide-react';
import { useKnowledgebaseRuntimeConfig } from 'sdkwork-knowledgebase-pc-core';

import {
  buildOpenApiCurlExample,
  OPEN_API_ENDPOINTS,
  OPEN_API_PREFIX,
  type OpenApiEndpointDoc,
} from './services/openApiDeveloperContent';

const METHOD_BADGE_CLASS: Record<OpenApiEndpointDoc['method'], string> = {
  GET: 'bg-blue-50 text-blue-600 dark:bg-blue-950/60 dark:text-blue-300',
  POST: 'bg-emerald-50 text-emerald-600 dark:bg-emerald-950/60 dark:text-emerald-300',
};

export function OpenApiDeveloperPanel() {
  const { t } = useTranslation(['kb', 'common']);
  const runtimeConfig = useKnowledgebaseRuntimeConfig();
  const openApiBaseUrl = `${runtimeConfig.openApiBaseUrl.replace(/\/+$/, '')}${OPEN_API_PREFIX}`;
  const [copiedBaseUrl, setCopiedBaseUrl] = useState(false);
  const [copiedCurl, setCopiedCurl] = useState(false);

  const copyToClipboard = async (value: string, mark: 'base' | 'curl') => {
    try {
      await navigator.clipboard.writeText(value);
      if (mark === 'base') {
        setCopiedBaseUrl(true);
        window.setTimeout(() => setCopiedBaseUrl(false), 1500);
      } else {
        setCopiedCurl(true);
        window.setTimeout(() => setCopiedCurl(false), 1500);
      }
    } catch {
      // Clipboard unavailable (permissions/insecure context); surface nothing.
    }
  };

  const curlExample = buildOpenApiCurlExample(openApiBaseUrl);

  return (
    <div className="space-y-6 animate-in fade-in duration-300">
      {/* Base URL */}
      <div className="p-5 rounded-2xl border-2 border-zinc-200/80 dark:border-[var(--color-kb-panel-border)] bg-white dark:bg-[var(--color-kb-panel)] shadow-sm space-y-3">
        <div className="flex items-center gap-2.5">
          <div className="w-9 h-9 rounded-xl bg-zinc-100 dark:bg-zinc-800 text-zinc-600 dark:text-zinc-300 flex items-center justify-center shrink-0">
            <Code2 size={17} strokeWidth={2.5} />
          </div>
          <div>
            <div className="text-[14px] font-extrabold text-zinc-900 dark:text-[var(--color-kb-text-heading)] tracking-tight">
              {t('openApiPanelTitle', { defaultValue: 'Open API 接入' })}
            </div>
            <p className="text-[11.5px] text-zinc-500 dark:text-[var(--color-kb-text-muted)] font-medium">
              {t('openApiPanelDesc', { defaultValue: '面向组织开发者的程序化接入接口' })}
            </p>
          </div>
        </div>
        <div className="flex items-center gap-2 bg-[#fafafa] dark:bg-[var(--color-kb-input-bg)] border border-zinc-200/80 dark:border-[var(--color-kb-panel-border)] rounded-xl px-4 py-2.5">
          <code className="flex-1 text-[12px] font-mono text-zinc-700 dark:text-[var(--color-kb-text)] truncate">
            {openApiBaseUrl}
          </code>
          <button
            type="button"
            title={t('openApiCopyBaseUrl', { defaultValue: '复制 Base URL' })}
            onClick={() => copyToClipboard(openApiBaseUrl, 'base')}
            className="w-7 h-7 flex items-center justify-center rounded-lg text-zinc-400 hover:text-zinc-700 dark:hover:text-[var(--color-kb-text)] hover:bg-zinc-100 dark:hover:bg-[var(--color-kb-panel-hover)] transition-all"
          >
            {copiedBaseUrl ? <Check size={14} className="text-emerald-500" /> : <Copy size={14} />}
          </button>
        </div>
      </div>

      {/* Authentication */}
      <div className="p-5 rounded-2xl border-2 border-indigo-100 dark:border-indigo-900/40 bg-indigo-50/40 dark:bg-indigo-950/20 space-y-3">
        <div className="flex items-center gap-2.5">
          <div className="w-9 h-9 rounded-xl bg-indigo-100 dark:bg-indigo-900/50 text-indigo-600 dark:text-indigo-300 flex items-center justify-center shrink-0">
            <KeyRound size={16} strokeWidth={2.5} />
          </div>
          <div className="text-[13.5px] font-extrabold text-indigo-900 dark:text-indigo-200 tracking-tight">
            {t('openApiAuthTitle', { defaultValue: '认证方式（x-api-key）' })}
          </div>
        </div>
        <ul className="space-y-1.5 text-[12px] font-medium text-indigo-900/80 dark:text-indigo-200/80 leading-relaxed list-disc pl-5">
          <li>
            {t('openApiAuthHeader', {
              defaultValue: '每个请求携带 x-api-key 请求头；Open API Key 由组织平台（IAM）统一发放。',
            })}
          </li>
          <li>
            {t('openApiAuthIssue', {
              defaultValue: '本产品不直接发放或管理 Key——请前往平台控制台申请，审批通过后即刻生效。',
            })}
          </li>
          <li>
            {t('openApiAuthScope', {
              defaultValue: 'Key 绑定组织：请求以组织身份访问该组织内你有权限的知识空间，租户与组织上下文由服务端注入。',
            })}
          </li>
        </ul>
        <div className="flex items-center gap-1.5 pt-1 text-[11.5px] font-bold text-indigo-700 dark:text-indigo-300">
          <Building2 size={13} />
          {t('openApiAuthConsole', { defaultValue: '申请入口：平台控制台 → 组织设置 → Open API Keys' })}
          <ExternalLink size={12} className="opacity-70" />
        </div>
      </div>

      {/* Endpoint table */}
      <div className="space-y-3">
        <div className="text-[14px] font-extrabold text-zinc-900 dark:text-[var(--color-kb-text-heading)] flex items-center gap-2">
          {t('openApiEndpointsTitle', { defaultValue: '接口清单' })}
          <span className="text-[11px] font-bold bg-zinc-900 text-white dark:bg-white dark:text-zinc-900 px-2 py-0.5 rounded-full">
            {OPEN_API_ENDPOINTS.length}
          </span>
        </div>
        <div className="border-2 border-zinc-200/80 dark:border-[var(--color-kb-panel-border)] rounded-2xl overflow-hidden bg-white dark:bg-[var(--color-kb-editor)] shadow-sm">
          <table className="w-full text-left">
            <thead>
              <tr className="bg-[#fafafa] dark:bg-[var(--color-kb-panel)] border-b border-zinc-200/80 dark:border-[var(--color-kb-panel-border)]">
                {[
                  t('openApiColumnMethod', { defaultValue: '方法' }),
                  t('openApiColumnPath', { defaultValue: '路径' }),
                  t('openApiColumnDesc', { defaultValue: '说明' }),
                  t('openApiColumnIdempotency', { defaultValue: '幂等键' }),
                ].map((heading) => (
                  <th
                    key={heading}
                    className="px-3 py-2.5 text-[10.5px] font-extrabold uppercase tracking-wider text-zinc-400 dark:text-[var(--color-kb-text-muted)]"
                  >
                    {heading}
                  </th>
                ))}
              </tr>
            </thead>
            <tbody className="divide-y divide-zinc-100 dark:divide-[var(--color-kb-panel-border)]">
              {OPEN_API_ENDPOINTS.map((endpoint) => (
                <tr key={endpoint.operationId} className="hover:bg-[#fafafa] dark:hover:bg-zinc-900/40 transition-colors">
                  <td className="px-3 py-2.5">
                    <span className={`inline-flex items-center px-2 py-0.5 rounded-md text-[10.5px] font-bold tracking-wide ${METHOD_BADGE_CLASS[endpoint.method]}`}>
                      {endpoint.method}
                    </span>
                  </td>
                  <td className="px-3 py-2.5">
                    <code className="text-[11.5px] font-mono text-zinc-700 dark:text-[var(--color-kb-text)]">
                      {endpoint.path}
                    </code>
                  </td>
                  <td className="px-3 py-2.5">
                    <span className="text-[12px] font-semibold text-zinc-600 dark:text-[var(--color-kb-text)]">
                      {endpoint.summary}
                    </span>
                  </td>
                  <td className="px-3 py-2.5">
                    {endpoint.idempotent ? (
                      <code className="text-[10.5px] font-mono text-zinc-500 dark:text-[var(--color-kb-text-muted)]">
                        Idempotency-Key
                      </code>
                    ) : (
                      <span className="text-[11px] text-zinc-300 dark:text-zinc-600">—</span>
                    )}
                  </td>
                </tr>
              ))}
            </tbody>
          </table>
        </div>
      </div>

      {/* cURL example */}
      <div className="space-y-3">
        <div className="flex items-center justify-between">
          <div className="text-[14px] font-extrabold text-zinc-900 dark:text-[var(--color-kb-text-heading)] flex items-center gap-2">
            <Terminal size={15} />
            {t('openApiCurlTitle', { defaultValue: '快速开始（cURL）' })}
          </div>
          <button
            type="button"
            onClick={() => copyToClipboard(curlExample, 'curl')}
            className="px-3 py-1.5 rounded-lg border border-zinc-200 dark:border-[var(--color-kb-panel-border)] text-[11.5px] font-bold text-zinc-600 dark:text-[var(--color-kb-text)] hover:bg-zinc-50 dark:hover:bg-[var(--color-kb-panel-hover)] transition-colors flex items-center gap-1.5"
          >
            {copiedCurl ? <Check size={12} className="text-emerald-500" /> : <Copy size={12} />}
            {copiedCurl
              ? t('openApiCopied', { defaultValue: '已复制' })
              : t('openApiCopyCurl', { defaultValue: '复制示例' })}
          </button>
        </div>
        <pre className="bg-zinc-950 text-zinc-100 rounded-2xl p-4 text-[11.5px] font-mono leading-relaxed overflow-x-auto shadow-inner">
          {curlExample}
        </pre>
      </div>
    </div>
  );
}
