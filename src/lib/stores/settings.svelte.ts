// ============================================================
// Settings Store - Application settings state management
// ============================================================

import type { ThemeMode } from '$lib/types';
import { settingsApi } from '$lib/api/tauri';
import { toastStore } from './toast.svelte';

// --- State ---
let theme = $state<ThemeMode>('dark');
let bevyExportPath = $state('');
let comfyuiEndpoint = $state('http://127.0.0.1:8188');
let loading = $state(false);
let loaded = $state(false);

// --- Actions ---
async function loadSettings(): Promise<void> {
  loading = true;
  try {
    const settings = await settingsApi.getAll();

    if (settings['theme'] === 'light' || settings['theme'] === 'dark') {
      theme = settings['theme'] as ThemeMode;
    }
    if (settings['bevy_export_path']) {
      bevyExportPath = settings['bevy_export_path'];
    }
    if (settings['comfyui_endpoint']) {
      comfyuiEndpoint = settings['comfyui_endpoint'];
    }

    applyTheme(theme);
    loaded = true;
  } catch (e) {
    toastStore.addToast('error', `設定の読み込みに失敗しました: ${e}`);
  } finally {
    loading = false;
  }
}

async function setTheme(newTheme: ThemeMode): Promise<void> {
  theme = newTheme;
  applyTheme(newTheme);
  try {
    await settingsApi.update('theme', newTheme);
  } catch (e) {
    toastStore.addToast('error', `テーマの保存に失敗しました: ${e}`);
  }
}

async function setBevyExportPath(path: string): Promise<void> {
  bevyExportPath = path;
  try {
    await settingsApi.update('bevy_export_path', path);
    toastStore.addToast('success', 'Bevyエクスポートパスを保存しました');
  } catch (e) {
    toastStore.addToast('error', `パスの保存に失敗しました: ${e}`);
  }
}

function applyTheme(t: ThemeMode): void {
  if (typeof document !== 'undefined') {
    document.documentElement.setAttribute('data-theme', t);
  }
}

// --- Export ---
export const settingsStore = {
  get theme() {
    return theme;
  },
  get bevyExportPath() {
    return bevyExportPath;
  },
  get comfyuiEndpoint() {
    return comfyuiEndpoint;
  },
  get loading() {
    return loading;
  },
  get loaded() {
    return loaded;
  },
  loadSettings,
  setTheme,
  setBevyExportPath,
};
