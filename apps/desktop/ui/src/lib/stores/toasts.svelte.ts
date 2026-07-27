// Transient notices: "settings saved", "path copied", "the export failed".
//
// Before this, every page kept its own `saveStatus`/`error` string and rendered it in
// its own place, so the same event looked different depending on where you were standing
// — and a message about a background export had nowhere to appear at all. One queue at
// the root fixes both.
//
// A toast stores its translation *key*, not the translated string: errors never
// auto-dismiss (a message the user did not manage to read is the same as no message), so
// a danger toast can outlive a language switch, and a frozen English line under a Russian
// interface is exactly the kind of seam this store exists to remove.
import type { TranslationKey } from "$lib/i18n/en";

export type ToastTone = "info" | "success" | "warning" | "danger";

export interface Toast {
  id: number;
  tone: ToastTone;
  titleKey: TranslationKey;
  params?: Record<string, string | number>;
  /** Untranslated supporting text — a backend error message, a path. Shown verbatim
   * because it did not come from a dictionary and must not look as though it did. */
  detail?: string;
}

export interface ToastOptions {
  params?: Record<string, string | number>;
  detail?: string;
}

const AUTO_DISMISS_MS = 4000;

class ToastState {
  items = $state<Toast[]>([]);
}

export const toasts = new ToastState();

let nextId = 1;

export function pushToast(tone: ToastTone, titleKey: TranslationKey, options?: ToastOptions): void {
  const id = nextId++;
  toasts.items = [
    ...toasts.items,
    { id, tone, titleKey, params: options?.params, detail: options?.detail },
  ];
  if (tone !== "danger") {
    setTimeout(() => dismissToast(id), AUTO_DISMISS_MS);
  }
}

export function dismissToast(id: number): void {
  toasts.items = toasts.items.filter((toast) => toast.id !== id);
}

/** Every Tauri command rejects with `CommandError { message }`, already redacted by the
 * backend. Anything else reaching here failed before the call and is stringified. */
export function errorMessage(error: unknown): string {
  return typeof error === "object" && error !== null && "message" in error
    ? String((error as { message: unknown }).message)
    : String(error);
}

/** Shorthand for the "an operation failed" case, which is most of them. */
export function reportError(titleKey: TranslationKey, error: unknown): void {
  pushToast("danger", titleKey, { detail: errorMessage(error) });
}
