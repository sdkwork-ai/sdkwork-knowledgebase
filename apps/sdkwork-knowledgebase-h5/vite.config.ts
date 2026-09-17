import { resolveBrowserDistOutDir } from '../../../sdkwork-specs/tools/browser-dist-layout.mjs';
import path from "node:path";
import { fileURLToPath } from "node:url";
import tailwindcss from "@tailwindcss/vite";
import react from "@vitejs/plugin-react";
import { defineConfig } from "vite";

const appRoot = path.dirname(fileURLToPath(import.meta.url));

/** Resolve the selected lifecycle environment from the profile-shaped Vite mode. */
function resolveViteEnvironment(mode: string | undefined, processEnv = process.env) {
  const profileMatch = /^(standalone|cloud)\.(development|test|staging|production|demo)$/u.exec(mode ?? '');
  if (profileMatch?.[2]) {
    return profileMatch[2];
  }
  const fromProcess = processEnv.SDKWORK_ENVIRONMENT ?? '';
  return ['development', 'test', 'staging', 'demo', 'production'].includes(fromProcess)
    ? fromProcess
    : 'production';
}

export default defineConfig(({ mode }) => ({
  build: {
    outDir: resolveBrowserDistOutDir(resolveViteEnvironment(mode, process.env)),
    emptyOutDir: true,
  },
  plugins: [react(), tailwindcss()],
  server: { port: 5197, host: '127.0.0.1' },
}));
