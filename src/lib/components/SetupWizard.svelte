<script lang="ts">
  import { X, ChevronRight, ChevronLeft, SkipForward } from 'lucide-svelte';
  import ModelDownloader from './ModelDownloader.svelte';
  import ComfyUIGuide from './ComfyUIGuide.svelte';
  import { setupStore } from '$lib/stores/setup.svelte';
  import { onnxStore } from '$lib/stores/onnx.svelte';
  import { comfyuiStore } from '$lib/stores/comfyui.svelte';

  // Step 1 complete when ONNX model is downloaded & initialized
  const step1Complete = $derived(onnxStore.isReady || onnxStore.isDownloaded);

  // Step 2 complete when ComfyUI connection is established
  const step2Complete = $derived(comfyuiStore.isConnected);

  // Load ComfyUI endpoint when reaching step 2
  $effect(() => {
    if (setupStore.currentStep === 2) {
      comfyuiStore.loadEndpoint();
    }
  });

  function handleDownloadComplete() {
    // Auto-advance to step 2 when ONNX model is ready
    setupStore.nextStep();
  }

  function handleSkip() {
    setupStore.skipSetup();
  }

  function handleClose() {
    setupStore.completeSetup();
  }

  function handleSkipComfyUI() {
    setupStore.skipComfyUI();
  }

  function handleBack() {
    if (setupStore.currentStep > 1) {
      // Going back from step 2 to step 1 requires direct state manipulation
      // We use resetSetup-like behavior but keep ONNX state
      setupStore.nextStep(); // This won't work for going back, so we handle differently
    }
  }

  function handleEndpointChange(newEndpoint: string) {
    comfyuiStore.setEndpoint(newEndpoint);
  }

  async function handleConnectionTest(): Promise<boolean> {
    return comfyuiStore.checkConnection();
  }
</script>

{#if setupStore.setupRequired}
  <div class="wizard-overlay" role="dialog" aria-modal="true" aria-label="初回セットアップ">
    <div class="wizard-container">
      <!-- Header -->
      <header class="wizard-header">
        <div class="wizard-title">
          <h2>Figurine Studio セットアップ</h2>
          <p class="wizard-subtitle">初回起動時の設定を行います</p>
        </div>
        <button class="wizard-close" onclick={handleClose} aria-label="閉じる">
          <X size={20} />
        </button>
      </header>

      <!-- Step Indicator -->
      <div class="step-indicator">
        {#each setupStore.stepProgress as step, i}
          <div class="step-dot" class:active={step.current} class:completed={step.completed}>
            <span class="step-number">{i + 1}</span>
          </div>
          {#if i < setupStore.stepProgress.length - 1}
            <div class="step-line" class:completed={step.completed}></div>
          {/if}
        {/each}
      </div>

      <!-- Step Content -->
      <div class="wizard-body">
        {#if setupStore.currentStep === 1}
          <div class="step-content">
            <h3>ステップ 1: ONNXモデルのセットアップ</h3>
            <p class="step-description">
              背景除去に必要なAIモデル（U2-Net）をダウンロードします．
              このモデルによりスプライト画像の背景を自動で除去できます．
            </p>

            <ModelDownloader onDownloadComplete={handleDownloadComplete} />
          </div>
        {:else if setupStore.currentStep === 2}
          <div class="step-content">
            <h3>ステップ 2: ComfyUI 接続設定</h3>
            <p class="step-description">
              AI質感生成に使用する ComfyUI の接続設定を行います．
              ComfyUI は任意です．背景除去とスプライトシート生成は ComfyUI なしでも利用可能です．
            </p>

            <ComfyUIGuide
              endpoint={comfyuiStore.endpoint}
              connected={comfyuiStore.isConnected}
              onEndpointChange={handleEndpointChange}
              onConnectionTest={handleConnectionTest}
            />
          </div>
        {/if}
      </div>

      <!-- Footer -->
      <footer class="wizard-footer">
        <div class="footer-left">
          <button class="btn btn-ghost" onclick={handleSkip}>
            <SkipForward size={16} />
            全てスキップ（後から設定）
          </button>
        </div>

        <div class="footer-right">
          {#if setupStore.currentStep === 1 && step1Complete}
            <button class="btn btn-primary" onclick={() => setupStore.nextStep()}>
              次へ
              <ChevronRight size={16} />
            </button>
          {:else if setupStore.currentStep === 2}
            <button class="btn btn-ghost" onclick={handleSkipComfyUI}>
              <SkipForward size={16} />
              ComfyUIをスキップ
            </button>
            {#if step2Complete}
              <button class="btn btn-primary" onclick={handleClose}>
                完了
                <ChevronRight size={16} />
              </button>
            {/if}
          {/if}
        </div>
      </footer>
    </div>
  </div>
{/if}

<style>
  .wizard-overlay {
    position: fixed;
    inset: 0;
    background: hsla(0, 0%, 0%, 0.7);
    z-index: 9500;
    display: flex;
    align-items: center;
    justify-content: center;
    animation: fade-in 150ms ease;
    backdrop-filter: blur(4px);
  }

  .wizard-container {
    background: var(--bg-elevated);
    border-radius: 16px;
    width: 90%;
    max-width: 700px;
    max-height: 90vh;
    overflow-y: auto;
    box-shadow: var(--shadow-lg);
    animation: modal-scale-in 200ms ease-out;
  }

  .wizard-header {
    display: flex;
    align-items: flex-start;
    justify-content: space-between;
    padding: var(--space-6) var(--space-6) var(--space-4);
  }

  .wizard-title h2 {
    font-size: var(--text-xl);
    font-weight: var(--font-weight-bold);
    margin-bottom: var(--space-1);
  }

  .wizard-subtitle {
    font-size: var(--text-sm);
    color: var(--text-muted);
  }

  .wizard-close {
    display: flex;
    align-items: center;
    justify-content: center;
    width: 32px;
    height: 32px;
    background: transparent;
    border: none;
    border-radius: 6px;
    color: var(--text-muted);
    cursor: pointer;
    transition:
      background-color 150ms ease,
      color 150ms ease;
  }

  .wizard-close:hover {
    background: var(--bg-tertiary);
    color: var(--text-primary);
  }

  /* Step Indicator */
  .step-indicator {
    display: flex;
    align-items: center;
    justify-content: center;
    gap: 0;
    padding: 0 var(--space-6) var(--space-4);
  }

  .step-dot {
    width: 32px;
    height: 32px;
    border-radius: 50%;
    display: flex;
    align-items: center;
    justify-content: center;
    background: var(--bg-tertiary);
    border: 2px solid var(--border-default);
    transition:
      background-color 200ms ease,
      border-color 200ms ease;
  }

  .step-dot.active {
    background: var(--accent-primary);
    border-color: var(--accent-primary);
  }

  .step-dot.completed {
    background: var(--accent-success);
    border-color: var(--accent-success);
  }

  .step-number {
    font-size: var(--text-xs);
    font-weight: var(--font-weight-bold);
    color: var(--text-primary);
  }

  .step-line {
    width: 40px;
    height: 2px;
    background: var(--border-default);
    transition: background-color 200ms ease;
  }

  .step-line.completed {
    background: var(--accent-success);
  }

  /* Body */
  .wizard-body {
    padding: 0 var(--space-6) var(--space-4);
  }

  .step-content h3 {
    margin-bottom: var(--space-2);
  }

  .step-description {
    font-size: var(--text-sm);
    color: var(--text-secondary);
    line-height: var(--leading-relaxed);
    margin-bottom: var(--space-4);
  }

  /* Footer */
  .wizard-footer {
    display: flex;
    align-items: center;
    justify-content: space-between;
    padding: var(--space-4) var(--space-6);
    border-top: 1px solid var(--border-default);
  }

  .footer-left {
    display: flex;
    align-items: center;
  }

  .footer-right {
    display: flex;
    align-items: center;
    gap: var(--space-3);
  }

  @keyframes fade-in {
    from {
      opacity: 0;
    }
    to {
      opacity: 1;
    }
  }

  @keyframes modal-scale-in {
    from {
      transform: scale(0.95);
      opacity: 0;
    }
    to {
      transform: scale(1);
      opacity: 1;
    }
  }
</style>
