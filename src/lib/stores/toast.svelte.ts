// ============================================================
// Toast Store - Notification message management
// ============================================================

import type { ToastItem } from '$lib/types';

let toasts = $state<ToastItem[]>([]);

function addToast(
  type: ToastItem['type'],
  message: string,
  duration: number = 5000,
): void {
  const id = crypto.randomUUID();
  const toast: ToastItem = { id, type, message, duration };
  toasts = [...toasts, toast];

  if (duration > 0) {
    setTimeout(() => {
      removeToast(id);
    }, duration);
  }
}

function removeToast(id: string): void {
  toasts = toasts.filter((t) => t.id !== id);
}

function clearAll(): void {
  toasts = [];
}

export const toastStore = {
  get toasts() {
    return toasts;
  },
  addToast,
  removeToast,
  clearAll,
};
