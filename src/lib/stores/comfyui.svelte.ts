// ============================================================
// ComfyUI Store - Connection & workflow state management
// ============================================================

import type {
  ComfyUIConnectionStatus,
  Workflow,
  CreateWorkflow,
  ProcessingParams,
} from '$lib/types';
import { comfyuiApi, settingsApi } from '$lib/api/tauri';
import { toastStore } from './toast.svelte';

// --- State ---
let endpoint = $state('http://127.0.0.1:8188');
let connectionStatus = $state<ComfyUIConnectionStatus>('disconnected');
let workflows = $state<Workflow[]>([]);
let selectedWorkflowId = $state<string | null>(null);
let loading = $state(false);
let error = $state<string | null>(null);

// --- Derived ---
const isConnected = $derived(connectionStatus === 'connected');

const selectedWorkflow = $derived(
  workflows.find((w) => w.id === selectedWorkflowId) ?? null,
);

const defaultWorkflow = $derived(
  workflows.find((w) => w.is_default) ?? null,
);

// --- Actions ---
async function loadEndpoint(): Promise<void> {
  try {
    const settings = await settingsApi.getAll();
    const saved = settings['comfyui_endpoint'];
    if (saved) {
      endpoint = saved;
    }
  } catch {
    // Use default endpoint
  }
}

async function setEndpoint(newEndpoint: string): Promise<void> {
  endpoint = newEndpoint;
  try {
    await settingsApi.update('comfyui_endpoint', newEndpoint);
  } catch {
    // Silently fail — endpoint is still updated locally
  }
}

async function checkConnection(customEndpoint?: string): Promise<boolean> {
  connectionStatus = 'connecting';
  error = null;
  try {
    const targetEndpoint = customEndpoint ?? endpoint;
    const result = await comfyuiApi.checkConnection(targetEndpoint);
    connectionStatus = result ? 'connected' : 'error';
    if (result && customEndpoint) {
      // Save the endpoint that succeeded
      await setEndpoint(customEndpoint);
    }
    return result;
  } catch (e) {
    error = String(e);
    connectionStatus = 'error';
    return false;
  }
}

async function loadWorkflows(): Promise<void> {
  loading = true;
  error = null;
  try {
    workflows = await comfyuiApi.listWorkflows();
    // Auto-select default workflow if none selected
    if (!selectedWorkflowId && defaultWorkflow) {
      selectedWorkflowId = defaultWorkflow.id;
    }
  } catch (e) {
    error = String(e);
    toastStore.addToast('error', `ワークフローの読み込みに失敗しました: ${e}`);
  } finally {
    loading = false;
  }
}

async function createWorkflow(input: CreateWorkflow): Promise<Workflow | null> {
  loading = true;
  error = null;
  try {
    const workflow = await comfyuiApi.createWorkflow(input);
    workflows = [...workflows, workflow];
    toastStore.addToast('success', `ワークフロー「${workflow.name}」を作成しました`);
    return workflow;
  } catch (e) {
    error = String(e);
    toastStore.addToast('error', `ワークフローの作成に失敗しました: ${e}`);
    return null;
  } finally {
    loading = false;
  }
}

async function importWorkflow(
  name: string,
  json: string,
): Promise<Workflow | null> {
  loading = true;
  error = null;
  try {
    const workflow = await comfyuiApi.importWorkflow(name, json);
    workflows = [...workflows, workflow];
    toastStore.addToast('success', `ワークフロー「${workflow.name}」をインポートしました`);
    return workflow;
  } catch (e) {
    error = String(e);
    toastStore.addToast('error', `ワークフローのインポートに失敗しました: ${e}`);
    return null;
  } finally {
    loading = false;
  }
}

async function deleteWorkflow(id: string): Promise<boolean> {
  error = null;
  try {
    await comfyuiApi.deleteWorkflow(id);
    workflows = workflows.filter((w) => w.id !== id);
    if (selectedWorkflowId === id) {
      selectedWorkflowId = defaultWorkflow?.id ?? null;
    }
    toastStore.addToast('success', 'ワークフローを削除しました');
    return true;
  } catch (e) {
    error = String(e);
    toastStore.addToast('error', `ワークフローの削除に失敗しました: ${e}`);
    return false;
  }
}

async function setDefaultWorkflow(id: string): Promise<boolean> {
  error = null;
  try {
    await comfyuiApi.setDefaultWorkflow(id);
    // Update local state
    workflows = workflows.map((w) => ({
      ...w,
      is_default: w.id === id,
    }));
    toastStore.addToast('success', 'デフォルトワークフローを設定しました');
    return true;
  } catch (e) {
    error = String(e);
    toastStore.addToast('error', `デフォルト設定に失敗しました: ${e}`);
    return false;
  }
}

function selectWorkflow(id: string): void {
  selectedWorkflowId = id;
}

// --- Export ---
export const comfyuiStore = {
  get endpoint() {
    return endpoint;
  },
  get connectionStatus() {
    return connectionStatus;
  },
  get workflows() {
    return workflows;
  },
  get selectedWorkflowId() {
    return selectedWorkflowId;
  },
  get selectedWorkflow() {
    return selectedWorkflow;
  },
  get defaultWorkflow() {
    return defaultWorkflow;
  },
  get isConnected() {
    return isConnected;
  },
  get loading() {
    return loading;
  },
  get error() {
    return error;
  },
  loadEndpoint,
  setEndpoint,
  checkConnection,
  loadWorkflows,
  createWorkflow,
  importWorkflow,
  deleteWorkflow,
  setDefaultWorkflow,
  selectWorkflow,
};
