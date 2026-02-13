// ============================================================
// Setup Store - First-time setup flow state management
// ============================================================

import { settingsApi } from '$lib/api/tauri';
import { onnxStore } from './onnx.svelte';
import { comfyuiStore } from './comfyui.svelte';

// --- State ---
let setupRequired = $state(false);
let currentStep = $state(1);
let checking = $state(false);
let setupCompleted = $state(false);
let comfyuiSkipped = $state(false);

// --- Derived ---
const totalSteps = 2; // Step 1: ONNX Model, Step 2: ComfyUI

const stepProgress = $derived(
  (() => {
    const steps: { label: string; completed: boolean; current: boolean }[] = [
      {
        label: 'ONNXモデルのセットアップ',
        completed: currentStep > 1 || setupCompleted,
        current: currentStep === 1,
      },
      {
        label: 'ComfyUI 接続設定',
        completed: currentStep > 2 || setupCompleted || comfyuiSkipped,
        current: currentStep === 2,
      },
    ];
    return steps;
  })(),
);

// --- Actions ---
async function checkSetupRequired(): Promise<boolean> {
  checking = true;
  try {
    const settings = await settingsApi.getAll();
    const isCompleted = settings['setup_completed'] === 'true';

    if (isCompleted) {
      setupRequired = false;
      setupCompleted = true;
      return false;
    }

    // Check ONNX model status
    await onnxStore.checkModel();
    if (onnxStore.isDownloaded || onnxStore.isReady) {
      // Model already downloaded, start at step 2
      currentStep = 2;
    }

    setupRequired = true;
    return true;
  } catch {
    // On error, don't block the app — skip setup
    setupRequired = false;
    return false;
  } finally {
    checking = false;
  }
}

async function markSetupCompleted(): Promise<void> {
  try {
    await settingsApi.update('setup_completed', 'true');
    setupCompleted = true;
    setupRequired = false;
  } catch {
    // Silently fail — app can still function
  }
}

function skipSetup(): void {
  setupRequired = false;
  setupCompleted = true;
  // Don't persist skip — user will see setup again on next launch
  // unless they complete it properly
}

function skipComfyUI(): void {
  comfyuiSkipped = true;
  completeSetup();
}

function nextStep(): void {
  if (currentStep < totalSteps) {
    currentStep += 1;
  }
}

function completeSetup(): void {
  setupRequired = false;
  setupCompleted = true;
  markSetupCompleted();
}

function resetSetup(): void {
  setupRequired = true;
  setupCompleted = false;
  comfyuiSkipped = false;
  currentStep = 1;
}

// --- Export ---
export const setupStore = {
  get setupRequired() {
    return setupRequired;
  },
  get currentStep() {
    return currentStep;
  },
  get checking() {
    return checking;
  },
  get setupCompleted() {
    return setupCompleted;
  },
  get stepProgress() {
    return stepProgress;
  },
  get totalSteps() {
    return totalSteps;
  },
  get comfyuiSkipped() {
    return comfyuiSkipped;
  },
  checkSetupRequired,
  markSetupCompleted,
  skipSetup,
  skipComfyUI,
  nextStep,
  completeSetup,
  resetSetup,
};
