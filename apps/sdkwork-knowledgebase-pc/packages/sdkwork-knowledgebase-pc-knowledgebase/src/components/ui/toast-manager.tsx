import { useState, useEffect } from 'react';
import { ExternalLink, FolderOpen } from 'lucide-react';

export type ToastActionIcon = 'open' | 'folder';

export interface ToastAction {
  label: string;
  onClick: () => void;
  emphasis?: 'primary' | 'default';
  icon?: ToastActionIcon;
}

export interface ToastOptions {
  message: string;
  subtitle?: string;
  duration?: number;
  actions?: ToastAction[];
  onMessageClick?: () => void;
  /** Replace an existing toast with the same key. */
  key?: string;
}

export interface ToastConfig {
  id: string;
  key?: string;
  message: string;
  subtitle?: string;
  type: 'success' | 'error' | 'info';
  duration?: number;
  actions?: ToastAction[];
  onMessageClick?: () => void;
}

class ToastManager {
  private listener: ((toasts: ToastConfig[]) => void) | null = null;
  private toasts: ToastConfig[] = [];
  private timers = new Map<string, ReturnType<typeof setTimeout>>();

  subscribe(listener: (toasts: ToastConfig[]) => void) {
    this.listener = listener;
    listener(this.toasts);
    return () => {
      this.listener = null;
    };
  }

  dismiss(keyOrId: string) {
    const target = this.toasts.find((toastItem) => toastItem.key === keyOrId || toastItem.id === keyOrId);
    if (!target) {
      return;
    }
    this.remove(target.id);
  }

  dismissAll() {
    for (const toastItem of this.toasts) {
      this.clearTimer(toastItem.id);
    }
    this.toasts = [];
    if (this.listener) {
      this.listener(this.toasts);
    }
  }

  private clearTimer(id: string) {
    const timer = this.timers.get(id);
    if (timer) {
      clearTimeout(timer);
      this.timers.delete(id);
    }
  }

  private emit() {
    if (this.listener) {
      this.listener(this.toasts);
    }
  }

  show(
    messageOrOptions: string | ToastOptions,
    type: 'success' | 'error' | 'info' = 'info',
    duration: number = 3000,
    actions?: ToastAction[],
  ) {
    const payload =
      typeof messageOrOptions === 'string'
        ? { message: messageOrOptions, duration, actions }
        : messageOrOptions;

    if (payload.key) {
      const existing = this.toasts.find((toastItem) => toastItem.key === payload.key);
      if (existing) {
        this.clearTimer(existing.id);
        const nextToast: ToastConfig = {
          ...existing,
          message: payload.message,
          subtitle: payload.subtitle,
          type,
          duration: payload.duration ?? duration,
          actions: payload.actions,
          onMessageClick: payload.onMessageClick,
        };
        this.toasts = this.toasts.map((toastItem) =>
          toastItem.id === existing.id ? nextToast : toastItem,
        );
        this.emit();
        const timer = setTimeout(() => {
          this.remove(existing.id);
        }, nextToast.duration ?? duration);
        this.timers.set(existing.id, timer);
        return;
      }
    }

    const id = Math.random().toString(36).substring(2, 9);
    const toastItem: ToastConfig = {
      id,
      key: payload.key,
      message: payload.message,
      subtitle: payload.subtitle,
      type,
      duration: payload.duration ?? duration,
      actions: payload.actions,
      onMessageClick: payload.onMessageClick,
    };
    this.toasts = [...this.toasts, toastItem];
    this.emit();

    const timer = setTimeout(() => {
      this.remove(id);
    }, toastItem.duration ?? duration);
    this.timers.set(id, timer);
  }

  remove(id: string) {
    this.clearTimer(id);
    this.toasts = this.toasts.filter((toastItem) => toastItem.id !== id);
    this.emit();
  }
}

export const toastManager = new ToastManager();

function normalizeSuccessOptions(
  msgOrOptions: string | ToastOptions,
  duration?: number,
  actions?: ToastAction[],
): ToastOptions {
  if (typeof msgOrOptions === 'string') {
    return { message: msgOrOptions, duration, actions };
  }
  return msgOrOptions;
}

export const toast = {
  dismiss: (keyOrId: string) => {
    toastManager.dismiss(keyOrId);
  },
  dismissAll: () => {
    toastManager.dismissAll();
  },
  success: (msgOrOptions: string | ToastOptions, duration?: number, actions?: ToastAction[]) => {
    const options = normalizeSuccessOptions(msgOrOptions, duration, actions);
    toastManager.show(options, 'success', options.duration ?? duration ?? 3000, options.actions);
  },
  error: (msgOrOptions: string | ToastOptions, duration?: number, actions?: ToastAction[]) => {
    const options = normalizeSuccessOptions(msgOrOptions, duration, actions);
    toastManager.show(options, 'error', options.duration ?? duration ?? 3000, options.actions);
  },
  info: (msgOrOptions: string | ToastOptions, duration?: number, actions?: ToastAction[]) => {
    const options = normalizeSuccessOptions(msgOrOptions, duration, actions);
    toastManager.show(options, 'info', options.duration ?? duration ?? 3000, options.actions);
  },
};

function renderActionIcon(icon?: ToastActionIcon) {
  if (icon === 'open') {
    return <ExternalLink size={12} className="shrink-0" />;
  }
  if (icon === 'folder') {
    return <FolderOpen size={12} className="shrink-0" />;
  }
  return null;
}

export function ToastContainer() {
  const [toasts, setToasts] = useState<ToastConfig[]>([]);

  useEffect(() => {
    return toastManager.subscribe(setToasts);
  }, []);

  return (
    <div className="fixed top-8 left-1/2 transform -translate-x-1/2 z-[10000] flex flex-col items-center gap-2 pointer-events-none">
      {toasts.map((t) => (
        <div
          key={t.id}
          className={`px-4 py-3 rounded-xl shadow-lg border text-sm font-medium flex flex-col gap-2.5 transition-all animate-in fade-in slide-in-from-top-4 pointer-events-auto max-w-[min(92vw,620px)] ${
            t.type === 'success'
              ? 'bg-emerald-50 text-emerald-700 border-emerald-200 dark:bg-emerald-950/60 dark:text-emerald-300 dark:border-emerald-800'
              : t.type === 'error'
                ? 'bg-red-50 text-red-700 border-red-200 dark:bg-red-950/60 dark:text-red-300 dark:border-red-800'
                : 'bg-zinc-900 text-zinc-100 border-zinc-800'
          }`}
        >
          <div className="flex items-start min-w-0 gap-2">
            <span className="mt-0.5 shrink-0">
              {t.type === 'success' && '🎉'}
              {t.type === 'error' && '🚨'}
              {t.type === 'info' && 'ℹ️'}
            </span>
            <div className="min-w-0 flex-1">
              {t.onMessageClick ? (
                <button
                  type="button"
                  onClick={t.onMessageClick}
                  className={`font-semibold text-[13px] leading-snug break-all text-left underline underline-offset-2 hover:opacity-80 ${
                    t.type === 'success'
                      ? 'text-emerald-800'
                      : t.type === 'error'
                        ? 'text-red-800'
                        : 'text-white'
                  }`}
                >
                  {t.message}
                </button>
              ) : (
                <div className="font-semibold text-[13px] leading-snug break-all">{t.message}</div>
              )}
              {t.subtitle && (
                <div
                  className={`mt-0.5 text-xs leading-snug ${
                    t.type === 'success'
                      ? 'text-emerald-600/90'
                      : t.type === 'error'
                        ? 'text-red-600/90'
                        : 'text-zinc-300'
                  }`}
                >
                  {t.subtitle}
                </div>
              )}
            </div>
          </div>
          {t.actions && t.actions.length > 0 && (
            <div className="flex items-center justify-end gap-2 pl-7">
              {t.actions.map((action) => (
                <button
                  key={action.label}
                  type="button"
                  onClick={() => {
                    action.onClick();
                  }}
                  className={`inline-flex items-center gap-1 px-3 py-1.5 rounded-md text-xs font-semibold whitespace-nowrap transition-all hover:scale-[1.02] active:scale-[0.98] ${
                    action.emphasis === 'primary'
                      ? t.type === 'success'
                        ? 'bg-emerald-600 text-white hover:bg-emerald-700 shadow-sm'
                        : t.type === 'error'
                          ? 'bg-red-600 text-white hover:bg-red-700 shadow-sm'
                          : 'bg-zinc-700 text-white hover:bg-zinc-600 shadow-sm'
                      : t.type === 'success'
                        ? 'bg-white/80 text-emerald-800 border border-emerald-300 hover:bg-white hover:border-emerald-400 dark:bg-white/5 dark:text-emerald-300 dark:border-emerald-800 dark:hover:bg-white/10 dark:hover:border-emerald-700'
                        : t.type === 'error'
                          ? 'bg-white/80 text-red-800 border border-red-300 hover:bg-white dark:bg-white/5 dark:text-red-300 dark:border-red-800 dark:hover:bg-white/10 dark:hover:border-red-700'
                          : 'bg-zinc-800 text-zinc-100 border border-zinc-700 hover:bg-zinc-700'
                  }`}
                >
                  {renderActionIcon(action.icon)}
                  {action.label}
                </button>
              ))}
            </div>
          )}
        </div>
      ))}
    </div>
  );
}
