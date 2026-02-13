<script lang="ts">
  import { page } from '$app/stores';
  import { convertFileSrc } from '@tauri-apps/api/core';
  import {
    Loader2,
    AlertTriangle,
    Check,
    Cpu,
    ImageMinus,
    RefreshCw,
    Sparkles,
    Wifi,
    WifiOff,
    Play,
  } from 'lucide-svelte';
  import { characterStore } from '$lib/stores/character.svelte';
  import { spriteStore } from '$lib/stores/sprite.svelte';
  import { projectStore } from '$lib/stores/project.svelte';
  import { onnxStore } from '$lib/stores/onnx.svelte';
  import { comfyuiStore } from '$lib/stores/comfyui.svelte';
  import { toastStore } from '$lib/stores/toast.svelte';
  import { onnxApi, comfyuiApi, onBgRemovalProgress, onComfyUIProgress } from '$lib/api/tauri';
  import ProgressBar from '$lib/components/ProgressBar.svelte';
  import type { Sprite, BgRemovalProgress, ComfyUIProgress, ProcessingParams } from '$lib/types';

  const characterId = $derived($page.params.id);

  // --- ONNX Status ---
  let onnxChecked = $state(false);

  $effect(() => {
    onnxStore.checkModel().then(() => {
      onnxChecked = true;
    });
  });

  // --- ComfyUI Status ---
  let comfyuiChecked = $state(false);

  $effect(() => {
    comfyuiStore.loadEndpoint();
    comfyuiStore.checkConnection().then(() => {
      comfyuiChecked = true;
    });
    comfyuiStore.loadWorkflows();
  });

  // --- Background Removal State ---
  type ProcessingState = 'idle' | 'processing' | 'complete' | 'error';
  let bgRemovalState = $state<ProcessingState>('idle');
  let bgRemovalProgress = $state<{ current: number; total: number }>({
    current: 0,
    total: 0,
  });
  let bgRemovalError = $state<string | null>(null);
  let bgRemovalResults = $state<{ success: number; failed: number }>({
    success: 0,
    failed: 0,
  });

  // Track which individual sprites are being processed
  let processingIndividual = $state<string | null>(null);

  // --- ComfyUI Processing State ---
  let comfyuiProcessState = $state<ProcessingState>('idle');
  let comfyuiProgressData = $state<{ current: number; total: number; currentFile: string }>({
    current: 0,
    total: 0,
    currentFile: '',
  });
  let comfyuiError = $state<string | null>(null);
  let comfyuiResults = $state<{ success: number; failed: number }>({
    success: 0,
    failed: 0,
  });

  // --- ComfyUI Parameters ---
  let denoiseStrength = $state(0.5);
  let controlnetWeight = $state(0.8);
  let cfgScale = $state(7);
  let steps = $state(20);
  let positivePrompt = $state('');
  let negativePrompt = $state('');

  // --- Sprites for bg-removal ---
  const spritesForRemoval = $derived(
    spriteStore.sprites.filter(
      (s) => s.status === 'raw' || s.status === 'ai_processed',
    ),
  );

  const bgRemovedSprites = $derived(
    spriteStore.sprites.filter(
      (s) => s.status === 'bg_removed' || s.status === 'finalized',
    ),
  );

  // --- Sprites for ComfyUI processing ---
  const spritesForComfyUI = $derived(
    spriteStore.sprites.filter((s) => s.status === 'raw'),
  );

  const aiProcessedSprites = $derived(
    spriteStore.sprites.filter(
      (s) =>
        s.status === 'ai_processed' ||
        s.status === 'bg_removed' ||
        s.status === 'finalized',
    ),
  );

  const allSpritesWithComparison = $derived(
    spriteStore.sprites.map((sprite) => ({
      sprite,
      originalSrc: sprite.raw_path ? convertFileSrc(sprite.raw_path) : '',
      processedSrc: sprite.processed_path
        ? convertFileSrc(sprite.processed_path)
        : '',
      hasBothImages: !!sprite.raw_path && !!sprite.processed_path,
    })),
  );

  const bgRemovalPercentage = $derived(
    bgRemovalProgress.total > 0
      ? Math.round(
          (bgRemovalProgress.current / bgRemovalProgress.total) * 100,
        )
      : 0,
  );

  // --- Handlers ---
  async function handleInitializeOnnx() {
    await onnxStore.initializeOnnx();
  }

  async function handleDownloadAndInit() {
    await onnxStore.downloadAndInitialize();
  }

  async function handleBatchRemoval() {
    if (!characterId) return;

    bgRemovalState = 'processing';
    bgRemovalError = null;
    bgRemovalProgress = { current: 0, total: spritesForRemoval.length };
    bgRemovalResults = { success: 0, failed: 0 };

    // Listen for progress events
    let unlisten: (() => void) | null = null;
    try {
      unlisten = await onBgRemovalProgress(
        (progress: BgRemovalProgress) => {
          bgRemovalProgress = {
            current: progress.current,
            total: progress.total,
          };
        },
      );

      await onnxApi.removeBackgroundBatch(characterId);

      // Reload sprites to get updated paths/statuses
      await spriteStore.loadSprites(characterId);

      const removed = spriteStore.sprites.filter(
        (s) => s.status === 'bg_removed' || s.status === 'finalized',
      ).length;

      bgRemovalResults = {
        success: removed,
        failed: spritesForRemoval.length - removed,
      };
      bgRemovalState = 'complete';
      toastStore.addToast(
        'success',
        `背景除去が完了しました（${removed}枚成功）`,
      );
    } catch (e) {
      bgRemovalError = String(e);
      bgRemovalState = 'error';
      toastStore.addToast('error', `背景除去に失敗しました: ${e}`);
    } finally {
      if (unlisten) unlisten();
    }
  }

  async function handleIndividualRemoval(sprite: Sprite) {
    if (!sprite.raw_path) return;

    processingIndividual = sprite.id;
    try {
      const outputPath = sprite.raw_path.replace(/\.(png|jpg|jpeg)$/i, '_nobg.png');
      await onnxApi.removeBackground(sprite.raw_path, outputPath);

      // Reload sprites
      if (characterId) {
        await spriteStore.loadSprites(characterId);
      }
      toastStore.addToast('success', '背景除去が完了しました');
    } catch (e) {
      toastStore.addToast('error', `背景除去に失敗しました: ${e}`);
    } finally {
      processingIndividual = null;
    }
  }

  // --- ComfyUI Batch Processing ---
  async function handleComfyUIBatch() {
    if (!characterId || !comfyuiStore.selectedWorkflowId) return;

    comfyuiProcessState = 'processing';
    comfyuiError = null;
    comfyuiProgressData = { current: 0, total: spritesForComfyUI.length, currentFile: '' };
    comfyuiResults = { success: 0, failed: 0 };

    let unlisten: (() => void) | null = null;
    try {
      unlisten = await onComfyUIProgress((progress: ComfyUIProgress) => {
        comfyuiProgressData = {
          current: progress.current,
          total: progress.total,
          currentFile: progress.current_file,
        };
      });

      const params: ProcessingParams = {
        positive_prompt: positivePrompt,
        negative_prompt: negativePrompt,
        denoise_strength: denoiseStrength,
        controlnet_weight: controlnetWeight,
        cfg_scale: cfgScale,
        steps,
      };

      await comfyuiApi.processBatch(
        characterId,
        comfyuiStore.selectedWorkflowId,
        params,
      );

      // Reload sprites
      await spriteStore.loadSprites(characterId);

      const processed = spriteStore.sprites.filter(
        (s) =>
          s.status === 'ai_processed' ||
          s.status === 'bg_removed' ||
          s.status === 'finalized',
      ).length;

      comfyuiResults = {
        success: processed,
        failed: spritesForComfyUI.length - processed,
      };
      comfyuiProcessState = 'complete';
      toastStore.addToast(
        'success',
        `AI質感生成が完了しました（${processed}枚処理）`,
      );
    } catch (e) {
      comfyuiError = String(e);
      comfyuiProcessState = 'error';
      toastStore.addToast('error', `AI質感生成に失敗しました: ${e}`);
    } finally {
      if (unlisten) unlisten();
    }
  }

  function resetComfyUIProcess() {
    comfyuiProcessState = 'idle';
    comfyuiProgressData = { current: 0, total: 0, currentFile: '' };
    comfyuiError = null;
    comfyuiResults = { success: 0, failed: 0 };
  }

  function getStatusLabel(status: string): string {
    const map: Record<string, string> = {
      raw: '未処理',
      ai_processed: 'AI処理済',
      bg_removed: '背景除去済',
      finalized: '完了',
    };
    return map[status] ?? status;
  }

  function resetBgRemoval() {
    bgRemovalState = 'idle';
    bgRemovalProgress = { current: 0, total: 0 };
    bgRemovalError = null;
    bgRemovalResults = { success: 0, failed: 0 };
  }
</script>

<div class="process-page">
  <!-- ONNX Status Section -->
  <section class="onnx-status-section">
    <h2><Cpu size={20} /> ONNX Runtime ステータス</h2>

    {#if !onnxChecked}
      <div class="status-card loading">
        <Loader2 size={16} class="spin" />
        <span>ONNXモデルの状態を確認しています...</span>
      </div>
    {:else if onnxStore.isReady}
      <div class="status-card ready">
        <Check size={16} />
        <span>ONNX Runtime は初期化済みです．背景除去機能が利用可能です．</span>
      </div>
    {:else if onnxStore.isDownloaded}
      <div class="status-card warning">
        <AlertTriangle size={16} />
        <div class="status-content">
          <span>ONNXモデルはダウンロード済みですが，初期化されていません．</span>
          <button
            class="btn btn-primary btn-sm"
            onclick={handleInitializeOnnx}
            disabled={onnxStore.modelStatus === 'initializing'}
          >
            {#if onnxStore.modelStatus === 'initializing'}
              <Loader2 size={14} class="spin" />
              初期化中...
            {:else}
              初期化する
            {/if}
          </button>
        </div>
      </div>
    {:else}
      <div class="status-card error">
        <AlertTriangle size={16} />
        <div class="status-content">
          <span>ONNXモデルが見つかりません．背景除去にはモデルのダウンロードが必要です．</span>
          <div class="status-actions">
            <button
              class="btn btn-primary btn-sm"
              onclick={handleDownloadAndInit}
              disabled={onnxStore.modelStatus === 'downloading'}
            >
              {#if onnxStore.modelStatus === 'downloading'}
                <Loader2 size={14} class="spin" />
                ダウンロード中...
              {:else}
                モデルをダウンロード
              {/if}
            </button>
            <a href="/settings/models" class="btn btn-ghost btn-sm">
              モデル管理ページへ
            </a>
          </div>
        </div>
      </div>
    {/if}
  </section>

  <!-- Background Removal Section -->
  <section class="bg-removal-section">
    <h2><ImageMinus size={20} /> 背景除去</h2>

    {#if spriteStore.spriteCount === 0}
      <div class="empty-notice">
        <p>スプライトがインポートされていません．先にインポートタブから画像を取り込んでください．</p>
        <a href="/character/{characterId}/import" class="btn btn-secondary">
          インポートページへ
        </a>
      </div>
    {:else}
      <!-- Batch Action Bar -->
      <div class="action-bar">
        <div class="action-info">
          <span class="action-count">
            {spritesForRemoval.length}枚が背景除去対象
          </span>
          <span class="action-done">
            {bgRemovedSprites.length}枚が背景除去済み
          </span>
        </div>

        <div class="action-buttons">
          {#if bgRemovalState === 'idle' || bgRemovalState === 'error'}
            <button
              class="btn btn-primary"
              onclick={handleBatchRemoval}
              disabled={!onnxStore.isReady || spritesForRemoval.length === 0}
            >
              <ImageMinus size={16} />
              全て背景除去
            </button>
          {:else if bgRemovalState === 'processing'}
            <button class="btn btn-secondary" disabled>
              <Loader2 size={16} class="spin" />
              処理中... ({bgRemovalProgress.current}/{bgRemovalProgress.total})
            </button>
          {:else if bgRemovalState === 'complete'}
            <button class="btn btn-ghost" onclick={resetBgRemoval}>
              <RefreshCw size={16} />
              リセット
            </button>
          {/if}
        </div>
      </div>

      <!-- Progress Bar (during processing) -->
      {#if bgRemovalState === 'processing'}
        <div class="processing-progress">
          <div class="progress-container">
            <div
              class="progress-bar default animated"
              style:width="{bgRemovalPercentage}%"
            ></div>
            <span class="progress-label">{bgRemovalPercentage}%</span>
          </div>
          <div class="progress-detail">
            <span>
              {bgRemovalProgress.current} / {bgRemovalProgress.total}枚 処理済み
            </span>
          </div>
        </div>
      {/if}

      <!-- Error Message -->
      {#if bgRemovalState === 'error' && bgRemovalError}
        <div class="error-banner">
          <AlertTriangle size={16} />
          <span>{bgRemovalError}</span>
        </div>
      {/if}

      <!-- Results Summary -->
      {#if bgRemovalState === 'complete'}
        <div class="results-summary">
          <div class="result-item success">
            <Check size={16} />
            <span>{bgRemovalResults.success}枚 成功</span>
          </div>
          {#if bgRemovalResults.failed > 0}
            <div class="result-item failed">
              <AlertTriangle size={16} />
              <span>{bgRemovalResults.failed}枚 失敗</span>
            </div>
          {/if}
        </div>
      {/if}

      <!-- Before/After Comparison Grid -->
      <div class="comparison-section">
        <h3>スプライト一覧</h3>
        <div class="comparison-grid">
          {#each allSpritesWithComparison as item}
            <div class="comparison-card">
              <div class="comparison-images">
                <!-- Original -->
                <div class="comparison-image">
                  <span class="image-label">元画像</span>
                  <div class="thumbnail checkerboard-bg">
                    {#if item.originalSrc}
                      <img src={item.originalSrc} alt="元画像" />
                    {:else}
                      <span class="no-image">-</span>
                    {/if}
                  </div>
                </div>

                <!-- Processed (bg removed) -->
                <div class="comparison-image">
                  <span class="image-label">背景除去後</span>
                  <div class="thumbnail checkerboard-bg">
                    {#if item.processedSrc}
                      <img src={item.processedSrc} alt="背景除去後" />
                    {:else}
                      <span class="no-image">未処理</span>
                    {/if}
                  </div>
                </div>
              </div>

              <div class="comparison-footer">
                <div class="sprite-meta">
                  <span class="badge badge-{item.sprite.status}">
                    {getStatusLabel(item.sprite.status)}
                  </span>
                  <span class="sprite-label">
                    {item.sprite.direction} / {item.sprite.animation} F{item.sprite.frame_index}
                  </span>
                </div>

                {#if onnxStore.isReady && (item.sprite.status === 'raw' || item.sprite.status === 'ai_processed')}
                  <button
                    class="btn btn-ghost btn-sm"
                    onclick={() => handleIndividualRemoval(item.sprite)}
                    disabled={processingIndividual === item.sprite.id}
                  >
                    {#if processingIndividual === item.sprite.id}
                      <Loader2 size={14} class="spin" />
                    {:else}
                      <ImageMinus size={14} />
                    {/if}
                  </button>
                {/if}
              </div>
            </div>
          {/each}
        </div>
      </div>
    {/if}
  </section>

  <!-- AI Texture Generation Section (ComfyUI) -->
  <section class="comfyui-section">
    <h2><Sparkles size={20} /> AI質感生成</h2>

    <!-- ComfyUI Connection Status -->
    <div class="comfyui-connection-bar">
      {#if !comfyuiChecked}
        <div class="connection-indicator checking">
          <Loader2 size={14} class="spin" />
          <span>ComfyUI の接続状態を確認中...</span>
        </div>
      {:else if comfyuiStore.isConnected}
        <div class="connection-indicator connected">
          <Wifi size={14} />
          <span>ComfyUI に接続済み</span>
          <span class="endpoint-text">({comfyuiStore.endpoint})</span>
        </div>
      {:else}
        <div class="connection-indicator disconnected">
          <WifiOff size={14} />
          <span>ComfyUI に未接続</span>
          <a href="/settings/comfyui" class="btn btn-ghost btn-sm">
            接続設定へ
          </a>
        </div>
      {/if}
    </div>

    {#if spriteStore.spriteCount === 0}
      <div class="empty-notice">
        <p>スプライトがインポートされていません．先にインポートタブから画像を取り込んでください．</p>
      </div>
    {:else}
      <!-- Workflow Selection -->
      <div class="comfyui-config">
        <div class="form-group">
          <label class="label" for="workflow-select">ワークフロー</label>
          <div class="workflow-select-row">
            <select
              id="workflow-select"
              class="select"
              value={comfyuiStore.selectedWorkflowId ?? ''}
              onchange={(e) => {
                const val = e.currentTarget.value;
                if (val) comfyuiStore.selectWorkflow(val);
              }}
              disabled={!comfyuiStore.isConnected || comfyuiStore.workflows.length === 0}
            >
              {#if comfyuiStore.workflows.length === 0}
                <option value="">ワークフローなし</option>
              {:else}
                <option value="" disabled>ワークフローを選択...</option>
                {#each comfyuiStore.workflows as wf}
                  <option value={wf.id}>
                    {wf.name}{wf.is_default ? ' (デフォルト)' : ''}
                  </option>
                {/each}
              {/if}
            </select>
            <a href="/settings/comfyui" class="btn btn-ghost btn-sm">
              管理
            </a>
          </div>
        </div>

        <!-- Parameter Sliders -->
        <div class="params-grid">
          <div class="param-item">
            <label class="label" for="denoise-strength">
              Denoise Strength
              <span class="param-value">{denoiseStrength.toFixed(2)}</span>
            </label>
            <input
              id="denoise-strength"
              type="range"
              class="slider"
              min="0"
              max="1"
              step="0.05"
              bind:value={denoiseStrength}
              disabled={!comfyuiStore.isConnected}
            />
          </div>

          <div class="param-item">
            <label class="label" for="controlnet-weight">
              ControlNet Weight
              <span class="param-value">{controlnetWeight.toFixed(2)}</span>
            </label>
            <input
              id="controlnet-weight"
              type="range"
              class="slider"
              min="0"
              max="1"
              step="0.05"
              bind:value={controlnetWeight}
              disabled={!comfyuiStore.isConnected}
            />
          </div>

          <div class="param-item">
            <label class="label" for="cfg-scale">
              CFG Scale
              <span class="param-value">{cfgScale}</span>
            </label>
            <input
              id="cfg-scale"
              type="range"
              class="slider"
              min="1"
              max="20"
              step="0.5"
              bind:value={cfgScale}
              disabled={!comfyuiStore.isConnected}
            />
          </div>

          <div class="param-item">
            <label class="label" for="steps">
              Steps
              <span class="param-value">{steps}</span>
            </label>
            <input
              id="steps"
              type="range"
              class="slider"
              min="10"
              max="50"
              step="1"
              bind:value={steps}
              disabled={!comfyuiStore.isConnected}
            />
          </div>
        </div>

        <!-- Prompt Text Areas -->
        <div class="prompt-group">
          <div class="form-group">
            <label class="label" for="positive-prompt">Positive Prompt</label>
            <textarea
              id="positive-prompt"
              class="textarea"
              placeholder="pixel art, game sprite, high quality..."
              bind:value={positivePrompt}
              disabled={!comfyuiStore.isConnected}
              rows="3"
            ></textarea>
          </div>

          <div class="form-group">
            <label class="label" for="negative-prompt">Negative Prompt</label>
            <textarea
              id="negative-prompt"
              class="textarea"
              placeholder="blurry, low quality, watermark..."
              bind:value={negativePrompt}
              disabled={!comfyuiStore.isConnected}
              rows="3"
            ></textarea>
          </div>
        </div>

        <!-- Execute Button -->
        <div class="comfyui-action-bar">
          <div class="action-info">
            <span class="action-count">
              {spritesForComfyUI.length}枚がAI質感生成対象
            </span>
            <span class="action-done">
              {aiProcessedSprites.length}枚がAI処理済み
            </span>
          </div>

          <div class="action-buttons">
            {#if comfyuiProcessState === 'idle' || comfyuiProcessState === 'error'}
              <button
                class="btn btn-primary"
                onclick={handleComfyUIBatch}
                disabled={
                  !comfyuiStore.isConnected ||
                  !comfyuiStore.selectedWorkflowId ||
                  spritesForComfyUI.length === 0
                }
              >
                <Play size={16} />
                AI質感生成を実行
              </button>
            {:else if comfyuiProcessState === 'processing'}
              <button class="btn btn-secondary" disabled>
                <Loader2 size={16} class="spin" />
                処理中...
              </button>
            {:else if comfyuiProcessState === 'complete'}
              <button class="btn btn-ghost" onclick={resetComfyUIProcess}>
                <RefreshCw size={16} />
                リセット
              </button>
            {/if}
          </div>
        </div>

        <!-- ComfyUI Progress Bar -->
        {#if comfyuiProcessState === 'processing'}
          <div class="comfyui-progress-section">
            <ProgressBar
              current={comfyuiProgressData.current}
              total={comfyuiProgressData.total}
              label="AI質感生成"
              showPercentage={true}
              showCounts={true}
              statusText={comfyuiProgressData.currentFile ? `処理中: ${comfyuiProgressData.currentFile}` : ''}
              animated={true}
            />
          </div>
        {/if}

        <!-- ComfyUI Error -->
        {#if comfyuiProcessState === 'error' && comfyuiError}
          <div class="error-banner">
            <AlertTriangle size={16} />
            <span>{comfyuiError}</span>
          </div>
        {/if}

        <!-- ComfyUI Results Summary -->
        {#if comfyuiProcessState === 'complete'}
          <div class="results-summary">
            <div class="result-item success">
              <Check size={16} />
              <span>{comfyuiResults.success}枚 処理成功</span>
            </div>
            {#if comfyuiResults.failed > 0}
              <div class="result-item failed">
                <AlertTriangle size={16} />
                <span>{comfyuiResults.failed}枚 失敗</span>
              </div>
            {/if}
          </div>
        {/if}
      </div>
    {/if}
  </section>
</div>

<style>
  .process-page {
    display: flex;
    flex-direction: column;
    gap: var(--space-8);
  }

  /* ONNX Status */
  .onnx-status-section h2 {
    display: flex;
    align-items: center;
    gap: var(--space-2);
    margin-bottom: var(--space-4);
  }

  .status-card {
    display: flex;
    align-items: flex-start;
    gap: var(--space-3);
    padding: var(--space-4);
    border-radius: 8px;
    font-size: var(--text-sm);
  }

  .status-card.loading {
    background: var(--bg-secondary);
    color: var(--text-secondary);
  }

  .status-card.ready {
    background: hsla(142, 60%, 45%, 0.1);
    border: 1px solid hsla(142, 60%, 45%, 0.3);
    color: var(--accent-success);
  }

  .status-card.warning {
    background: hsla(45, 90%, 55%, 0.1);
    border: 1px solid hsla(45, 90%, 55%, 0.3);
    color: var(--accent-warning);
  }

  .status-card.error {
    background: hsla(0, 70%, 55%, 0.1);
    border: 1px solid hsla(0, 70%, 55%, 0.3);
    color: var(--accent-danger);
  }

  .status-content {
    display: flex;
    flex-direction: column;
    gap: var(--space-2);
  }

  .status-actions {
    display: flex;
    gap: var(--space-2);
  }

  .btn-sm {
    height: 28px;
    padding: 0 var(--space-2);
    font-size: var(--text-xs);
  }

  /* Background Removal */
  .bg-removal-section h2 {
    display: flex;
    align-items: center;
    gap: var(--space-2);
    margin-bottom: var(--space-4);
  }

  .empty-notice {
    padding: var(--space-6);
    background: var(--bg-secondary);
    border: 1px dashed var(--border-default);
    border-radius: 8px;
    text-align: center;
    display: flex;
    flex-direction: column;
    align-items: center;
    gap: var(--space-3);
    color: var(--text-muted);
    font-size: var(--text-sm);
  }

  .action-bar,
  .comfyui-action-bar {
    display: flex;
    align-items: center;
    justify-content: space-between;
    padding: var(--space-3) var(--space-4);
    background: var(--bg-secondary);
    border: 1px solid var(--border-default);
    border-radius: 8px;
    margin-bottom: var(--space-4);
  }

  .action-info {
    display: flex;
    gap: var(--space-4);
    font-size: var(--text-sm);
  }

  .action-count {
    color: var(--text-secondary);
  }

  .action-done {
    color: var(--accent-success);
  }

  .action-buttons {
    display: flex;
    gap: var(--space-2);
  }

  .processing-progress {
    display: flex;
    flex-direction: column;
    gap: var(--space-2);
    margin-bottom: var(--space-4);
  }

  .progress-detail {
    font-size: var(--text-sm);
    color: var(--text-secondary);
    text-align: center;
  }

  .error-banner {
    display: flex;
    align-items: center;
    gap: var(--space-2);
    padding: var(--space-3);
    background: hsla(0, 70%, 55%, 0.1);
    border: 1px solid var(--accent-danger);
    border-radius: 6px;
    color: var(--accent-danger);
    font-size: var(--text-sm);
    margin-bottom: var(--space-4);
  }

  .results-summary {
    display: flex;
    gap: var(--space-4);
    margin-bottom: var(--space-4);
  }

  .result-item {
    display: flex;
    align-items: center;
    gap: var(--space-2);
    padding: var(--space-2) var(--space-3);
    border-radius: 6px;
    font-size: var(--text-sm);
    font-weight: var(--font-weight-medium);
  }

  .result-item.success {
    background: hsla(142, 60%, 45%, 0.1);
    color: var(--accent-success);
  }

  .result-item.failed {
    background: hsla(0, 70%, 55%, 0.1);
    color: var(--accent-danger);
  }

  /* Comparison Grid */
  .comparison-section h3 {
    margin-bottom: var(--space-3);
  }

  .comparison-grid {
    display: grid;
    grid-template-columns: repeat(auto-fill, minmax(240px, 1fr));
    gap: var(--space-3);
  }

  .comparison-card {
    background: var(--bg-secondary);
    border: 1px solid var(--border-default);
    border-radius: 8px;
    padding: var(--space-3);
    display: flex;
    flex-direction: column;
    gap: var(--space-2);
  }

  .comparison-images {
    display: grid;
    grid-template-columns: 1fr 1fr;
    gap: var(--space-2);
  }

  .comparison-image {
    display: flex;
    flex-direction: column;
    gap: var(--space-1);
  }

  .image-label {
    font-size: var(--text-xs);
    color: var(--text-muted);
    text-align: center;
  }

  .thumbnail {
    width: 100%;
    aspect-ratio: 1;
    border-radius: 4px;
    overflow: hidden;
    display: flex;
    align-items: center;
    justify-content: center;
    border: 1px solid var(--border-default);
  }

  .thumbnail img {
    max-width: 100%;
    max-height: 100%;
    object-fit: contain;
    image-rendering: pixelated;
  }

  .no-image {
    font-size: var(--text-xs);
    color: var(--text-muted);
  }

  .comparison-footer {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: var(--space-2);
    padding-top: var(--space-2);
    border-top: 1px solid var(--border-default);
  }

  .sprite-meta {
    display: flex;
    flex-direction: column;
    gap: 2px;
  }

  .sprite-label {
    font-size: var(--text-xs);
    color: var(--text-muted);
    text-transform: capitalize;
  }

  /* ComfyUI Section */
  .comfyui-section h2 {
    display: flex;
    align-items: center;
    gap: var(--space-2);
    margin-bottom: var(--space-4);
  }

  .comfyui-connection-bar {
    margin-bottom: var(--space-4);
  }

  .connection-indicator {
    display: flex;
    align-items: center;
    gap: var(--space-2);
    padding: var(--space-3) var(--space-4);
    border-radius: 8px;
    font-size: var(--text-sm);
  }

  .connection-indicator.checking {
    background: var(--bg-secondary);
    color: var(--text-secondary);
  }

  .connection-indicator.connected {
    background: hsla(142, 60%, 45%, 0.1);
    border: 1px solid hsla(142, 60%, 45%, 0.3);
    color: var(--accent-success);
  }

  .connection-indicator.disconnected {
    background: hsla(45, 90%, 55%, 0.1);
    border: 1px solid hsla(45, 90%, 55%, 0.3);
    color: var(--accent-warning);
  }

  .endpoint-text {
    font-size: var(--text-xs);
    color: var(--text-muted);
  }

  .comfyui-config {
    display: flex;
    flex-direction: column;
    gap: var(--space-4);
  }

  .workflow-select-row {
    display: flex;
    gap: var(--space-2);
    align-items: center;
  }

  .workflow-select-row .select {
    flex: 1;
  }

  /* Parameter Grid */
  .params-grid {
    display: grid;
    grid-template-columns: repeat(auto-fill, minmax(280px, 1fr));
    gap: var(--space-4);
    padding: var(--space-4);
    background: var(--bg-secondary);
    border: 1px solid var(--border-default);
    border-radius: 8px;
  }

  .param-item {
    display: flex;
    flex-direction: column;
    gap: var(--space-2);
  }

  .param-item .label {
    display: flex;
    align-items: center;
    justify-content: space-between;
    margin-bottom: 0;
  }

  .param-value {
    font-family: var(--font-mono);
    font-size: var(--text-xs);
    color: var(--accent-primary);
    font-weight: var(--font-weight-normal);
  }

  .slider {
    -webkit-appearance: none;
    appearance: none;
    width: 100%;
    height: 6px;
    background: var(--bg-tertiary);
    border-radius: 3px;
    outline: none;
    cursor: pointer;
  }

  .slider::-webkit-slider-thumb {
    -webkit-appearance: none;
    appearance: none;
    width: 16px;
    height: 16px;
    background: var(--accent-primary);
    border-radius: 50%;
    cursor: pointer;
    transition: transform 100ms ease;
  }

  .slider::-webkit-slider-thumb:hover {
    transform: scale(1.2);
  }

  .slider::-moz-range-thumb {
    width: 16px;
    height: 16px;
    background: var(--accent-primary);
    border: none;
    border-radius: 50%;
    cursor: pointer;
  }

  .slider:disabled {
    opacity: 0.5;
    cursor: not-allowed;
  }

  .slider:disabled::-webkit-slider-thumb {
    cursor: not-allowed;
  }

  /* Prompt Group */
  .prompt-group {
    display: flex;
    flex-direction: column;
    gap: var(--space-4);
  }

  .prompt-group .textarea {
    min-height: 80px;
  }

  /* ComfyUI Progress */
  .comfyui-progress-section {
    margin-bottom: var(--space-4);
  }
</style>
