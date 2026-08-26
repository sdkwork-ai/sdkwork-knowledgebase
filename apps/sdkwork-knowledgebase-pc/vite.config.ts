import { resolveBrowserDistOutDir } from '../../../sdkwork-specs/tools/browser-dist-layout.mjs';
function resolveViteEnvironment(mode: string | undefined, processEnv = process.env) {
  const profileMatch = /^(standalone|cloud)\.(development|test|staging|production)$/u.exec(mode ?? '');
  return profileMatch?.[2]
    ?? (['development', 'test', 'staging', 'production'].includes(processEnv.SDKWORK_ENVIRONMENT ?? '')
      ? (processEnv.SDKWORK_ENVIRONMENT ?? 'production')
      : 'production');
}
import tailwindcss from '@tailwindcss/vite';
import react from '@vitejs/plugin-react';
import path from 'path';
import { fileURLToPath } from 'node:url';
import {defineConfig, loadEnv} from 'vite';
import { browserSecurityHeadersPlugin } from './config/browser/securityHeaders';
import { toKnowledgebaseViteBasePath } from './packages/sdkwork-knowledgebase-pc-core/src/config/browserBasePath';

const __dirname = path.dirname(fileURLToPath(import.meta.url));

const repoRoot = path.resolve(__dirname, '../..');
const appbaseRoot = path.resolve(repoRoot, '../sdkwork-appbase');
const iamRoot = path.resolve(repoRoot, '../sdkwork-iam');
const DEFAULT_PLATFORM_API_GATEWAY_TARGET = 'http://127.0.0.1:3900';

const APP_SOURCE_CHUNKS = [
  ['packages/sdkwork-knowledgebase-pc-knowledgebase/src/', 'feature-knowledgebase'],
  ['packages/sdkwork-knowledgebase-pc-knowledge/src/', 'feature-knowledge'],
  ['packages/sdkwork-knowledgebase-pc-search/src/', 'feature-search'],
  ['packages/sdkwork-knowledgebase-pc-shell/src/', 'feature-shell'],
  ['packages/sdkwork-knowledgebase-pc-admin-provider/src/', 'feature-admin-provider'],
  ['packages/sdkwork-knowledgebase-pc-core/src/', 'feature-core'],
  ['packages/sdkwork-knowledgebase-pc-commons/src/', 'feature-commons'],
] as const;

const APP_SOURCE_SUB_CHUNKS = [
  [
    [
      'packages/sdkwork-knowledgebase-pc-knowledgebase/src/components/players/',
      'packages/sdkwork-knowledgebase-pc-knowledgebase/src/MediaViewer.tsx',
      'packages/sdkwork-knowledgebase-pc-knowledgebase/src/PdfViewer.tsx',
    ],
    'feature-media-viewers',
  ],
] as const;

const VENDOR_CHUNKS = [
  [
    'vendor-react',
    [
      '/node_modules/react/',
      '/node_modules/react-dom/',
      '/node_modules/react-router',
      '/node_modules/scheduler/',
    ],
  ],
  ['vendor-monaco', ['@monaco-editor', 'monaco-editor']],
  ['vendor-editor', ['@tiptap', 'prosemirror', '@codemirror', '/codemirror/', 'orderedmap', 'w3c-keyname']],
  ['vendor-pdf', ['pdfjs', 'react-pdf']],
  ['vendor-canvas', ['html2canvas', 'canvg', 'rgbcolor']],
  ['vendor-pdf-export', ['jspdf']],
  [
    'vendor-markdown',
    ['@uiw/react-md-editor', 'marked', 'highlight.js', 'rehype', 'remark', 'mdast', 'hast', 'unist', 'dompurify'],
  ],
  ['vendor-ui', ['@radix-ui', '@dnd-kit', 'lucide-react', '/motion/', 'framer-motion']],
  ['vendor-i18n', ['i18next', 'react-i18next']],
  ['vendor-virtualization', ['@tanstack/react-virtual']],
] as const;

function resolveManualChunk(id: string): string | undefined {
  const normalizedId = id.replace(/\\/g, '/');
  const appSourceSubChunk = APP_SOURCE_SUB_CHUNKS.find(([fragments]) =>
    fragments.some((fragment) => normalizedId.includes(fragment)),
  );
  if (appSourceSubChunk) {
    return appSourceSubChunk[1];
  }
  const appSourceChunk = APP_SOURCE_CHUNKS.find(([fragment]) => normalizedId.includes(fragment));
  if (appSourceChunk) {
    return appSourceChunk[1];
  }
  if (!normalizedId.includes('/node_modules/')) {
    return undefined;
  }
  const vendorChunk = VENDOR_CHUNKS.find(([, fragments]) =>
    fragments.some((fragment) => normalizedId.includes(fragment)),
  );
  return vendorChunk?.[0] ?? 'vendor';
}

const MAX_JAVASCRIPT_CHUNK_BYTES = 650 * 1024;

function bundleSizeBudgetPlugin() {
  return {
    name: 'sdkwork-knowledgebase-bundle-size-budget',
    apply: 'build' as const,
    generateBundle(_options: unknown, bundle: Record<string, { type: string; code?: string }>) {
      const oversized = Object.entries(bundle)
        .filter(([, output]) => output.type === 'chunk')
        .map(([fileName, output]) => ({ fileName, bytes: Buffer.byteLength(output.code || '', 'utf8') }))
        .filter(({ bytes }) => bytes > MAX_JAVASCRIPT_CHUNK_BYTES);
      if (oversized.length > 0) {
        const details = oversized
          .map(({ fileName, bytes }) => `${fileName}: ${bytes} bytes`)
          .join(', ');
        throw new Error(
          `JavaScript chunk budget exceeded (${MAX_JAVASCRIPT_CHUNK_BYTES} bytes): ${details}`,
        );
      }
    },
  };
}

export default defineConfig(({mode}) => {
  const env = loadEnv(mode, __dirname, '');
  const browserBasePath = toKnowledgebaseViteBasePath(
    env.VITE_SDKWORK_KNOWLEDGEBASE_BROWSER_BASE_PATH
    || process.env.VITE_SDKWORK_KNOWLEDGEBASE_BROWSER_BASE_PATH,
  );
  const deploymentProfile = (
    env.VITE_SDKWORK_KNOWLEDGEBASE_DEPLOYMENT_PROFILE
    || process.env.VITE_SDKWORK_KNOWLEDGEBASE_DEPLOYMENT_PROFILE
    || 'standalone'
  ).toLowerCase();
  const isStandaloneProfile = deploymentProfile === 'standalone';

  // Build-time guard: refuse to bundle dev-only auth credentials into production.
  // These env vars are dev-only auth form prefill and must never ship to production.
  // See sdkwork-specs/ENVIRONMENT_SPEC.md §3 (VITE_*_AUTH_DEV_* gating).
  if (mode === 'production') {
    const devEmail = env.VITE_SDKWORK_KNOWLEDGEBASE_AUTH_DEV_EMAIL;
    const devPassword = env.VITE_SDKWORK_KNOWLEDGEBASE_AUTH_DEV_PASSWORD;
    if (devEmail || devPassword) {
      throw new Error(
        'VITE_SDKWORK_KNOWLEDGEBASE_AUTH_DEV_* must not be set in production builds. '
        + 'These credentials are dev-only auth form prefill and must not ship to production. '
        + 'Remove them from the build environment before retrying.',
      );
    }
  }

  const platformApiGatewayTarget =
    isStandaloneProfile
      ? (
        env.VITE_SDKWORK_APPBASE_APP_API_BASE_URL
        || process.env.VITE_SDKWORK_APPBASE_APP_API_BASE_URL
        || env.VITE_SDKWORK_KNOWLEDGEBASE_APPLICATION_PUBLIC_HTTP_URL
        || process.env.VITE_SDKWORK_KNOWLEDGEBASE_APPLICATION_PUBLIC_HTTP_URL
        || DEFAULT_PLATFORM_API_GATEWAY_TARGET
      )
      : (
        env.VITE_SDKWORK_KNOWLEDGEBASE_PLATFORM_API_GATEWAY_HTTP_URL
        || process.env.VITE_SDKWORK_KNOWLEDGEBASE_PLATFORM_API_GATEWAY_HTTP_URL
        || env.VITE_SDKWORK_APPBASE_APP_API_BASE_URL
        || process.env.VITE_SDKWORK_APPBASE_APP_API_BASE_URL
        || DEFAULT_PLATFORM_API_GATEWAY_TARGET
      );
  const appApiTarget =
    env.VITE_SDKWORK_KNOWLEDGEBASE_APPLICATION_PUBLIC_HTTP_URL
    || process.env.VITE_SDKWORK_KNOWLEDGEBASE_APPLICATION_PUBLIC_HTTP_URL
    || 'http://127.0.0.1:18081';
  const openApiTarget =
    env.VITE_SDKWORK_KNOWLEDGEBASE_APPLICATION_OPEN_HTTP_URL
    || process.env.VITE_SDKWORK_KNOWLEDGEBASE_APPLICATION_OPEN_HTTP_URL
    || appApiTarget;
  const iamAppApiTarget =
    env.VITE_SDKWORK_IAM_APP_API_BASE_URL
    || process.env.VITE_SDKWORK_IAM_APP_API_BASE_URL
    || platformApiGatewayTarget;

  const appIngressTarget = isStandaloneProfile ? appApiTarget : platformApiGatewayTarget;
  const standaloneProxy = {
    '/app/v3/api': {
      target: appIngressTarget,
      changeOrigin: true,
    },
    '/knowledge/v3/api': {
      target: openApiTarget,
      changeOrigin: true,
    },
  };
  const cloudProxy = {
    '/app/v3/api/oauth': {
      target: iamAppApiTarget,
      changeOrigin: true,
    },
    '/app/v3/api/auth': {
      target: iamAppApiTarget,
      changeOrigin: true,
    },
    '/app/v3/api/knowledge': {
      target: appApiTarget,
      changeOrigin: true,
    },
    '/app/v3/api': {
      target: platformApiGatewayTarget,
      changeOrigin: true,
    },
    '/knowledge/v3/api': {
      target: platformApiGatewayTarget,
      changeOrigin: true,
    },
  };

  return {
    base: browserBasePath,
    ...(mode === 'development' || mode === 'standalone.docker'
      ? {
          define: {
            // Injects the credential-entry bootstrap Access-Token into the
            // bundle (dev servers and the docker standalone container build;
            // read from SDKWORK_ACCESS_TOKEN at build time). Other modes must
            // not bake a token into the static bundle.
            'process.env.SDKWORK_ACCESS_TOKEN': JSON.stringify(
              env.SDKWORK_ACCESS_TOKEN ?? process.env.SDKWORK_ACCESS_TOKEN ?? '',
            ),
          },
        }
      : {}),
    plugins: [
      react(),
      tailwindcss(),
      browserSecurityHeadersPlugin(mode === 'development'),
      bundleSizeBudgetPlugin(),
    ],
    build: {
      outDir: resolveBrowserDistOutDir(resolveViteEnvironment(mode, process.env)),
      rollupOptions: {
        output: {
          manualChunks(id) {
            return resolveManualChunk(id);
          },
        },
      },
    },
    optimizeDeps: {
      include: [
        'react',
        'react-dom',
        'react-dom/client',
        'react/jsx-dev-runtime',
        'react/jsx-runtime',
        'react-router-dom',
        'lucide-react',
        'i18next',
        'react-i18next',
        'marked',
        'dompurify',
        'clsx',
        'tailwind-merge',
        'html2canvas-pro',
        '@monaco-editor/react',
        '@radix-ui/react-context-menu',
        '@radix-ui/react-dropdown-menu',
        '@tiptap/core',
        '@tiptap/react',
        '@tiptap/react/menus',
        '@tiptap/starter-kit',
        '@tiptap/extension-bubble-menu',
        '@tiptap/extension-image',
        '@tiptap/extension-placeholder',
        'tiptap-markdown',
        'react-pdf',
      ],
      exclude: ['pdfjs-dist'],
    },
    resolve: {
      dedupe: [
        'react',
        'react-dom',
        'react-router-dom',
        'i18next',
        'react-i18next',
      ],
      alias: [
        { find: '@', replacement: path.resolve(__dirname, '.') },
        {
          find: /^@sdkwork\/sdkwork-knowledgebase-pc-commons\/(.*)$/,
          replacement: `${path.resolve(__dirname, 'packages/sdkwork-knowledgebase-pc-commons/src')}/$1`,
        },
        {
          find: '@sdkwork/sdkwork-knowledgebase-pc-commons',
          replacement: path.resolve(__dirname, 'packages/sdkwork-knowledgebase-pc-commons/src/index.ts'),
        },
        {
          find: /^@sdkwork\/sdkwork-knowledgebase-pc-knowledgebase\/(.*)$/,
          replacement: `${path.resolve(__dirname, 'packages/sdkwork-knowledgebase-pc-knowledgebase/src')}/$1`,
        },
        {
          find: '@sdkwork/sdkwork-knowledgebase-pc-knowledgebase',
          replacement: path.resolve(__dirname, 'packages/sdkwork-knowledgebase-pc-knowledgebase/src/index.ts'),
        },
        {
          find: /^@sdkwork\/sdkwork-knowledgebase-pc-shell\/(.*)$/,
          replacement: `${path.resolve(__dirname, 'packages/sdkwork-knowledgebase-pc-shell/src')}/$1`,
        },
        {
          find: '@sdkwork/sdkwork-knowledgebase-pc-shell',
          replacement: path.resolve(__dirname, 'packages/sdkwork-knowledgebase-pc-shell/src/index.ts'),
        },
        {
          find: 'sdkwork-knowledgebase-pc-admin-provider',
          replacement: path.resolve(__dirname, 'packages/sdkwork-knowledgebase-pc-admin-provider/src/index.ts'),
        },
        {
          find: '@sdkwork/auth-pc-react',
          replacement: path.resolve(
            iamRoot,
            'apps/sdkwork-iam-pc/packages/sdkwork-auth-pc-react/src/index.ts',
          ),
        },
        {
          find: '@sdkwork/auth-runtime-pc-react',
          replacement: path.resolve(
            iamRoot,
            'apps/sdkwork-iam-pc/packages/sdkwork-auth-runtime-pc-react/src/index.ts',
          ),
        },
        {
          find: '@sdkwork/knowledgebase-app-sdk',
          replacement: path.resolve(
            repoRoot,
            'sdks/sdkwork-knowledgebase-app-sdk/sdkwork-knowledgebase-app-sdk-typescript/src/index.ts',
          ),
        },
        {
          find: '@sdkwork/iam-app-sdk',
          replacement: path.resolve(
            iamRoot,
            'sdks/sdkwork-iam-app-sdk/sdkwork-iam-app-sdk-typescript/src/index.ts',
          ),
        },
        {
          find: '@sdkwork/appbase-pc-react',
          replacement: path.resolve(
            appbaseRoot,
            'packages/pc-react/foundation/sdkwork-appbase-pc-react/src/index.ts',
          ),
        },
        {
          find: '@sdkwork/iam-contracts',
          replacement: path.resolve(
            iamRoot,
            'apps/sdkwork-iam-common/packages/sdkwork-iam-contracts/src/index.ts',
          ),
        },
        {
          find: '@sdkwork/sdk-common',
          replacement: path.resolve(
            repoRoot,
            '../sdkwork-sdk-commons/sdkwork-sdk-common-typescript/src/index.ts',
          ),
        },
        {
          find: '@sdkwork/ui-pc-react',
          replacement: path.resolve(repoRoot, '../sdkwork-ui/sdkwork-ui-pc-react/src/index.ts'),
        },
        {
          find: '@sdkwork/core-pc-react',
          replacement: path.resolve(__dirname, 'src/bootstrap/sdkworkCorePcReactShim.ts'),
        },
        {
          find: '@sdkwork/knowledgebase-pc-search',
          replacement: path.resolve(__dirname, 'packages/sdkwork-knowledgebase-pc-search/src/index.ts'),
        },
        {
          find: 'sdkwork-knowledgebase-pc-core',
          replacement: path.resolve(__dirname, 'packages/sdkwork-knowledgebase-pc-core/src'),
        },
        {
          find: 'sdkwork-knowledgebase-pc-core/host/hostAdapter',
          replacement: path.resolve(__dirname, 'packages/sdkwork-knowledgebase-pc-core/src/host/hostAdapter.ts'),
        },
        {
          find: '@sdkwork/sdkwork-knowledgebase-pc-commons/reactKeyedProps',
          replacement: path.resolve(__dirname, 'packages/sdkwork-knowledgebase-pc-commons/src/reactKeyedProps.ts'),
        },
        {
          find: '@sdkwork/sdkwork-knowledgebase-pc-commons/htmlSanitizer',
          replacement: path.resolve(__dirname, 'packages/sdkwork-knowledgebase-pc-commons/src/htmlSanitizer.ts'),
        },
      ],
    },
    server: {
      port: 5184,
      strictPort: true,
      // HMR is disabled in AI Studio via DISABLE_HMR env var.
      // Do not modifyâfile watching is disabled to prevent flickering during agent edits.
      hmr: process.env.DISABLE_HMR !== 'true',
      // Disable file watching when DISABLE_HMR is true to save CPU during agent edits.
      watch: process.env.DISABLE_HMR === 'true' ? null : {},
      proxy: isStandaloneProfile ? standaloneProxy : cloudProxy,
    },
  };
});
