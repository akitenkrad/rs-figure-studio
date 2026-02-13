// ============================================================
// ONNX Store - Model download & runtime state management
// ============================================================

import type { OnnxModelState, DownloadProgress } from '$lib/types';
import { onnxApi, onDownloadProgress } from '$lib/api/tauri';
import { toastStore } from './toast.svelte';

// --- State ---
let modelStatus = $state<OnnxModelState>('not_downloaded');
let downloadProgress = $state<{ downloadedBytes: number; totalBytes: number }>({
  downloadedBytes: 0,
  totalBytes: 0,
});
let modelPath = $state<string | null>(null);
let error = $state<string | null>(null);
let loading = $state(false);

// --- Derived ---
const isDownloaded = $derived(
  modelStatus === 'downloaded' ||
    modelStatus === 'initializing' ||
    modelStatus === 'ready',
);

const isReady = $derived(modelStatus === 'ready');

const downloadPercentage = $derived(
  downloadProgress.totalBytes > 0
    ? Math.round((downloadProgress.downloadedBytes / downloadProgress.totalBytes) * 100)
    : 0,
);

const downloadedMB = $derived(
  Math.round(downloadProgress.downloadedBytes / 1_048_576),
);

const totalMB = $derived(
  downloadProgress.totalBytes > 0
    ? Math.round(downloadProgress.totalBytes / 1_048_576)
    : 176,
);

// --- Actions ---
async function checkModel(): Promise<void> {
  loading = true;
  error = null;
  try {
    const status = await onnxApi.checkModel();
    if (status.downloaded) {
      modelPath = status.model_path;
      modelStatus = status.initialized ? 'ready' : 'downloaded';
    } else {
      modelStatus = 'not_downloaded';
      modelPath = null;
    }
  } catch (e) {
    error = String(e);
    modelStatus = 'error';
  } finally {
    loading = false;
  }
}

async function downloadModel(): Promise<boolean> {
  if (modelStatus === 'downloading') return false;

  modelStatus = 'downloading';
  error = null;
  downloadProgress = { downloadedBytes: 0, totalBytes: 0 };

  // Listen for download progress events
  let unlisten: (() => void) | null = null;
  try {
    unlisten = await onDownloadProgress((progress: DownloadProgress) => {
      downloadProgress = {
        downloadedBytes: progress.downloaded,
        totalBytes: progress.total,
      };
    });

    const path = await onnxApi.downloadModel();
    modelPath = path;
    modelStatus = 'downloaded';
    toastStore.addToast('success', 'ONNXモデルのダウンロードが完了しました');
    return true;
  } catch (e) {
    error = String(e);
    modelStatus = 'error';
    toastStore.addToast('error', `モデルのダウンロードに失敗しました: ${e}`);
    return false;
  } finally {
    if (unlisten) unlisten();
  }
}

async function initializeOnnx(): Promise<boolean> {
  modelStatus = 'initializing';
  error = null;
  try {
    await onnxApi.initialize();
    modelStatus = 'ready';
    toastStore.addToast('success', 'ONNX Runtimeの初期化が完了しました');
    return true;
  } catch (e) {
    error = String(e);
    modelStatus = 'error';
    toastStore.addToast('error', `ONNX Runtimeの初期化に失敗しました: ${e}`);
    return false;
  }
}

async function downloadAndInitialize(): Promise<boolean> {
  const downloaded = await downloadModel();
  if (!downloaded) return false;
  return await initializeOnnx();
}

function reset(): void {
  modelStatus = 'not_downloaded';
  downloadProgress = { downloadedBytes: 0, totalBytes: 0 };
  modelPath = null;
  error = null;
  loading = false;
}

// --- Export ---
export const onnxStore = {
  get modelStatus() {
    return modelStatus;
  },
  get downloadProgress() {
    return downloadProgress;
  },
  get modelPath() {
    return modelPath;
  },
  get error() {
    return error;
  },
  get loading() {
    return loading;
  },
  get isDownloaded() {
    return isDownloaded;
  },
  get isReady() {
    return isReady;
  },
  get downloadPercentage() {
    return downloadPercentage;
  },
  get downloadedMB() {
    return downloadedMB;
  },
  get totalMB() {
    return totalMB;
  },
  checkModel,
  downloadModel,
  initializeOnnx,
  downloadAndInitialize,
  reset,
};
