import path from 'node:path';
import { fileURLToPath } from 'node:url';

const appRoot = path.resolve(path.dirname(fileURLToPath(import.meta.url)));

/** Resolve `@sdkwork/utils` via package exports / node_modules — no src path aliases. */
export const vitestSharedAliases = {};

export { appRoot };
