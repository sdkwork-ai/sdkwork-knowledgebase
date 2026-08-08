import { dismissExportProgress } from './exportProgress';
import { toast, type ToastAction } from '../ui/toast-manager';
import { getDocumentExportCapabilities } from './documentExportCapabilities';
import { isDesktopExportHost } from './exportRuntime';
import {
  openExportFile,
  resolveSavedExportPath,
  revealExportInFolder,
} from './documentExportSave';
import type { SaveExportFileResult } from './types';

export function notifyExportCancelled() {
  dismissExportProgress();
  toast.info('已取消保存');
}

function buildExportOpenActions(
  result: SaveExportFileResult,
  initialPath?: string,
): ToastAction[] {
  const resolvePath = async (): Promise<string | undefined> => {
    if (initialPath?.trim()) {
      return initialPath;
    }
    return resolveSavedExportPath(result);
  };

  const openResolvedFile = () => {
    void (async () => {
      const path = await resolvePath();
      if (!path) {
        toast.error('无法定位已保存的文件');
        return;
      }
      await openExportFile(path);
    })();
  };

  const revealResolvedFolder = () => {
    void (async () => {
      const path = await resolvePath();
      if (!path) {
        toast.error('无法定位已保存的文件');
        return;
      }
      await revealExportInFolder(path);
    })();
  };

  return [
    {
      label: '打开文件',
      emphasis: 'primary',
      icon: 'open',
      onClick: openResolvedFile,
    },
    {
      label: '打开所在文件夹',
      icon: 'folder',
      onClick: revealResolvedFolder,
    },
  ];
}

export async function notifyExportSaveResult(result: SaveExportFileResult, extra?: string) {
  if (result.cancelled || !result.saved) {
    return;
  }

  dismissExportProgress();

  const capabilities = getDocumentExportCapabilities();
  const fileName = result.pathLabel ?? '文件';
  const locationText =
    result.mode === 'downloads'
      ? isDesktopExportHost()
        ? '已保存到「下载」'
        : '已触发浏览器下载'
      : isDesktopExportHost()
        ? '已另存为'
        : '已通过浏览器保存';
  const webHint =
    !capabilities.canOpenExportFile && result.mode === 'downloads'
      ? ` · ${capabilities.defaultSaveHint}`
      : '';
  const subtitle = extra
    ? `${extra.trim()} ${locationText}${webHint}`
    : `${locationText}${webHint}`;

  let resolvedPath: string | undefined;
  let actions: ToastAction[] | undefined;

  if (capabilities.canOpenExportFile) {
    resolvedPath = await resolveSavedExportPath(result);
    actions = buildExportOpenActions(result, resolvedPath);
  }

  toast.success({
    message: fileName,
    subtitle,
    duration: 12000,
    onMessageClick: capabilities.canOpenExportFile
      ? () => {
          void (async () => {
            const path = resolvedPath ?? (await resolveSavedExportPath(result));
            if (!path) {
              toast.error('无法定位已保存的文件');
              return;
            }
            await openExportFile(path);
          })();
        }
      : undefined,
    actions,
  });
}
