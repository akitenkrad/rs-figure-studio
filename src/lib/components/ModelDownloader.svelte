<script lang="ts">
  import { Download, Check, AlertCircle, Loader2, RefreshCw } from 'lucide-svelte';
  import { onnxStore } from '$lib/stores/onnx.svelte';

  let {
    onDownloadComplete,
  }: {
    onDownloadComplete?: () => void;
  } = $props();

  // Check model status on mount
  $effect(() => {
    onnxStore.checkModel();
  });

  async function handleDownload() {
    const success = await onnxStore.downloadAndInitialize();
    if (success && onDownloadComplete) {
      onDownloadComplete();
    }
  }

  async function handleRedownload() {
    onnxStore.reset();
    await handleDownload();
  }

  async function handleInitialize() {
    const success = await onnxStore.initializeOnnx();
    if (success && onDownloadComplete) {
      onDownloadComplete();
    }
  }
</script>

<div class="model-downloader">
  <div class="model-info-header">
    <h3>U2-Net モデル</h3>
    {#if onnxStore.modelStatus === 'ready'}
      <span class="badge badge-complete">初期化済み</span>
    {:else if onnxStore.isDownloaded}
      <span class="badge badge-processing">ダウンロード済み</span>
    {:else if onnxStore.modelStatus === 'downloading'}
      <span class="badge badge-importing">ダウンロード中</span>
    {:else}
      <span class="badge badge-draft">未ダウンロード</span>
    {/if}
  </div>

  <p class="model-description">
    背景除去に使用するAIモデル（U2-Net）です．スプライト画像から背景を自動で除去し，
    透過PNG画像を生成します．
  </p>

  <!-- Not Downloaded State -->
  {#if onnxStore.modelStatus === 'not_downloaded'}
    <div class="model-details">
      <div class="detail-row">
        <span class="detail-label">モデル名</span>
        <span class="detail-value">U2-Net (u2net.onnx)</span>
      </div>
      <div class="detail-row">
        <span class="detail-label">ファイルサイズ</span>
        <span class="detail-value">約 176 MB</span>
      </div>
      <div class="detail-row">
        <span class="detail-label">用途</span>
        <span class="detail-value">セマンティックセグメンテーションによる背景除去</span>
      </div>
    </div>

    <button class="btn btn-primary download-btn" onclick={handleDownload}>
      <Download size={16} />
      モデルをダウンロード
    </button>

  <!-- Downloading State -->
  {:else if onnxStore.modelStatus === 'downloading'}
    <div class="download-progress">
      <div class="progress-container">
        <div
          class="progress-bar default animated"
          style:width="{onnxStore.downloadPercentage}%"
        ></div>
        <span class="progress-label">{onnxStore.downloadPercentage}%</span>
      </div>
      <div class="progress-detail">
        <span>{onnxStore.downloadedMB} MB / {onnxStore.totalMB} MB</span>
      </div>
    </div>

  <!-- Initializing State -->
  {:else if onnxStore.modelStatus === 'initializing'}
    <div class="initializing-state">
      <Loader2 size={20} class="spin" />
      <span>ONNX Runtime を初期化しています...</span>
    </div>

  <!-- Downloaded (not yet initialized) -->
  {:else if onnxStore.modelStatus === 'downloaded'}
    <div class="downloaded-state">
      <div class="status-icon success-icon">
        <Check size={20} />
      </div>
      <div class="status-detail">
        <span class="status-text">モデルのダウンロードが完了しました</span>
        {#if onnxStore.modelPath}
          <span class="model-path">{onnxStore.modelPath}</span>
        {/if}
      </div>
    </div>
    <div class="action-buttons">
      <button class="btn btn-primary" onclick={handleInitialize}>
        <Loader2 size={16} />
        ONNX Runtime を初期化
      </button>
      <button class="btn btn-ghost" onclick={handleRedownload}>
        <RefreshCw size={16} />
        再ダウンロード
      </button>
    </div>

  <!-- Ready State -->
  {:else if onnxStore.modelStatus === 'ready'}
    <div class="ready-state">
      <div class="status-icon success-icon">
        <Check size={20} />
      </div>
      <div class="status-detail">
        <span class="status-text">背景除去機能が利用可能です</span>
        {#if onnxStore.modelPath}
          <span class="model-path">{onnxStore.modelPath}</span>
        {/if}
      </div>
    </div>
    <button class="btn btn-ghost" onclick={handleRedownload}>
      <RefreshCw size={16} />
      再ダウンロード
    </button>

  <!-- Error State -->
  {:else if onnxStore.modelStatus === 'error'}
    <div class="error-state">
      <div class="status-icon error-icon">
        <AlertCircle size={20} />
      </div>
      <div class="status-detail">
        <span class="status-text error-text">エラーが発生しました</span>
        {#if onnxStore.error}
          <span class="error-message">{onnxStore.error}</span>
        {/if}
      </div>
    </div>
    <button class="btn btn-primary" onclick={handleDownload}>
      <RefreshCw size={16} />
      再試行
    </button>
  {/if}

  <!-- Loading indicator -->
  {#if onnxStore.loading}
    <div class="loading-state">
      <Loader2 size={16} class="spin" />
      <span>モデルの状態を確認しています...</span>
    </div>
  {/if}
</div>

<style>
  .model-downloader {
    display: flex;
    flex-direction: column;
    gap: var(--space-4);
    padding: var(--space-6);
    background: var(--bg-secondary);
    border: 1px solid var(--border-default);
    border-radius: 12px;
  }

  .model-info-header {
    display: flex;
    align-items: center;
    justify-content: space-between;
  }

  .model-description {
    color: var(--text-secondary);
    font-size: var(--text-sm);
    line-height: var(--leading-relaxed);
  }

  .model-details {
    display: flex;
    flex-direction: column;
    gap: var(--space-2);
    padding: var(--space-4);
    background: var(--bg-tertiary);
    border-radius: 8px;
  }

  .detail-row {
    display: flex;
    align-items: center;
    gap: var(--space-3);
  }

  .detail-label {
    min-width: 120px;
    font-size: var(--text-xs);
    font-weight: var(--font-weight-medium);
    color: var(--text-muted);
    text-transform: uppercase;
    letter-spacing: 0.05em;
  }

  .detail-value {
    font-size: var(--text-sm);
    color: var(--text-primary);
  }

  .download-btn {
    align-self: flex-start;
  }

  .download-progress {
    display: flex;
    flex-direction: column;
    gap: var(--space-2);
  }

  .progress-detail {
    font-size: var(--text-sm);
    color: var(--text-secondary);
    text-align: center;
  }

  .initializing-state,
  .loading-state {
    display: flex;
    align-items: center;
    gap: var(--space-2);
    padding: var(--space-3);
    background: var(--bg-tertiary);
    border-radius: 6px;
    color: var(--text-secondary);
    font-size: var(--text-sm);
  }

  .downloaded-state,
  .ready-state,
  .error-state {
    display: flex;
    align-items: flex-start;
    gap: var(--space-3);
    padding: var(--space-4);
    background: var(--bg-tertiary);
    border-radius: 8px;
  }

  .status-icon {
    flex-shrink: 0;
    width: 36px;
    height: 36px;
    display: flex;
    align-items: center;
    justify-content: center;
    border-radius: 50%;
  }

  .success-icon {
    background: hsla(142, 60%, 45%, 0.15);
    color: var(--accent-success);
  }

  .error-icon {
    background: hsla(0, 70%, 55%, 0.15);
    color: var(--accent-danger);
  }

  .status-detail {
    display: flex;
    flex-direction: column;
    gap: var(--space-1);
  }

  .status-text {
    font-size: var(--text-sm);
    font-weight: var(--font-weight-medium);
    color: var(--text-primary);
  }

  .error-text {
    color: var(--accent-danger);
  }

  .model-path {
    font-size: var(--text-xs);
    font-family: var(--font-mono);
    color: var(--text-muted);
    word-break: break-all;
  }

  .error-message {
    font-size: var(--text-xs);
    color: var(--accent-danger);
    word-break: break-all;
  }

  .action-buttons {
    display: flex;
    gap: var(--space-3);
  }
</style>
