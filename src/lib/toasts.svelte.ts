export type ToastKind = "error" | "success";

export interface Toast {
  id: number;
  kind: ToastKind;
  message: string;
  /** Total display time in milliseconds; drives the countdown bar. */
  duration: number;
}

const durations: Record<ToastKind, number> = { error: 12000, success: 7000 };
const maximumVisible = 5;
const timers = new Map<
  number,
  {
    remaining: number;
    startedAt: number;
    handle?: ReturnType<typeof setTimeout>;
  }
>();
/** Toasts pushed out by the visible limit rather than dismissed or expired. */
const evicted = new Set<number>();
let nextId = 1;

/** Visible notifications, oldest first. Rendered once by `Toasts.svelte`. */
export const toasts = $state<Toast[]>([]);
/** True while the pointer or focus is on the notifications; timers and bars stop. */
export const toastState = $state({ paused: false });

function start(id: number) {
  const timer = timers.get(id);
  if (!timer) return;
  timer.startedAt = performance.now();
  timer.handle = setTimeout(() => dismiss(id), timer.remaining);
}

/** Shows a transient notification. The oldest are dropped beyond five. */
export function notify(message: string, kind: ToastKind = "success") {
  const toast = { id: nextId++, kind, message, duration: durations[kind] };
  toasts.push(toast);
  timers.set(toast.id, { remaining: toast.duration, startedAt: 0 });
  if (!toastState.paused) start(toast.id);
  while (toasts.length > maximumVisible) {
    evicted.add(toasts[0].id);
    dismiss(toasts[0].id);
  }
}

/** Reports (once) whether a removed toast was pushed out by the visible limit. */
export function takeEvicted(id: number) {
  return evicted.delete(id);
}

/** Reports a rejected operation, without the redundant `Error:` prefix. */
export function notifyError(reason: unknown) {
  notify(String(reason).replace(/^Error:\s*/, ""), "error");
}

export function dismiss(id: number) {
  clearTimeout(timers.get(id)?.handle);
  timers.delete(id);
  const index = toasts.findIndex((toast) => toast.id === id);
  if (index >= 0) toasts.splice(index, 1);
}

export function pauseToasts() {
  if (toastState.paused) return;
  toastState.paused = true;
  const now = performance.now();
  for (const timer of timers.values()) {
    if (timer.handle === undefined) continue;
    clearTimeout(timer.handle);
    timer.handle = undefined;
    timer.remaining = Math.max(0, timer.remaining - (now - timer.startedAt));
  }
}

export function resumeToasts() {
  if (!toastState.paused) return;
  toastState.paused = false;
  for (const id of timers.keys()) start(id);
}
