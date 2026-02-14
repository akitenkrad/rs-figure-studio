<script lang="ts">
  import { page } from '$app/stores';
  import { convertFileSrc } from '@tauri-apps/api/core';
  import {
    Loader2,
    AlertTriangle,
    Sparkles,
    ChevronRight,
    Check,
    RefreshCw,
    RotateCcw,
    Wand2,
    Palette,
    Compass,
    Film,
    ArrowUpToLine,
    Download,
    HardDrive,
    Info,
    ChevronDown,
    ChevronUp,
    Heart,
    X,
  } from 'lucide-svelte';
  import { characterStore } from '$lib/stores/character.svelte';
  import { projectStore } from '$lib/stores/project.svelte';
  import { generationStore } from '$lib/stores/generation.svelte';
  import { comfyuiStore } from '$lib/stores/comfyui.svelte';
  import { spriteStore } from '$lib/stores/sprite.svelte';
  import { comfyuiModelApi, onComfyuiModelDownloadProgress, settingsApi } from '$lib/api/tauri';
  import ProgressBar from '$lib/components/ProgressBar.svelte';
  import type {
    AnimationDef,
    ConceptParams,
    PixelArtConversionParams,
    DirectionParams,
    AnimationParams,
    ComfyuiModelStatus,
    ComfyuiModelDownloadProgress,
  } from '$lib/types';

  const characterId = $derived($page.params.id);

  // --- Parse project config ---
  // Rust の Project.directions は Vec<String> なので，Tauri invoke() 経由で既に配列として届く．
  // ただし DB には JSON 文字列で保存されるため，string の場合は JSON.parse する．
  const projectDirections = $derived((() => {
    const p = projectStore.currentProject;
    if (!p?.directions) return [] as string[];
    if (Array.isArray(p.directions)) return p.directions as string[];
    try {
      return JSON.parse(p.directions as string) as string[];
    } catch {
      return [] as string[];
    }
  })());

  const projectAnimations = $derived((() => {
    const p = projectStore.currentProject;
    if (!p?.animations) return [] as AnimationDef[];
    if (Array.isArray(p.animations)) return p.animations as AnimationDef[];
    try {
      return JSON.parse(p.animations as string) as AnimationDef[];
    } catch {
      return [] as AnimationDef[];
    }
  })());

  // --- ComfyUI connection check ---
  let comfyuiChecked = $state(false);

  $effect(() => {
    comfyuiStore.loadEndpoint();
    comfyuiStore.checkConnection().then(() => {
      comfyuiChecked = true;
    });
  });

  // --- SDXL Checkpoint Model Check ---
  const SDXL_MODEL_NAME = 'sd_xl_base_1.0.safetensors';
  const SDXL_DOWNLOAD_URL =
    'https://huggingface.co/stabilityai/stable-diffusion-xl-base-1.0/resolve/main/sd_xl_base_1.0.safetensors';

  let modelStatus = $state<ComfyuiModelStatus | null>(null);
  let modelCheckLoading = $state(false);
  let modelCheckError = $state<string | null>(null);
  let modelDownloading = $state(false);
  let modelDownloadProgress = $state<ComfyuiModelDownloadProgress | null>(null);
  let modelDownloadComplete = $state(false);
  let checkpointsPath = $state('');
  let showCheckpointsPathInput = $state(false);
  let showPromptHints = $state(false);

  const modelAvailable = $derived(
    modelStatus?.available === true || modelDownloadComplete,
  );

  // Auto-check model availability after ComfyUI connection is confirmed
  $effect(() => {
    if (comfyuiChecked && comfyuiStore.isConnected && !modelStatus && !modelCheckLoading) {
      checkModelAvailability();
    }
  });

  async function checkModelAvailability() {
    modelCheckLoading = true;
    modelCheckError = null;
    try {
      modelStatus = await comfyuiModelApi.checkModel(SDXL_MODEL_NAME);
    } catch (e) {
      modelCheckError = String(e);
    } finally {
      modelCheckLoading = false;
    }
  }

  async function handleDownloadModel() {
    // If checkpoints path is not set, show input
    if (!checkpointsPath) {
      try {
        const savedPath = await comfyuiModelApi.getCheckpointsPath();
        if (savedPath) {
          checkpointsPath = savedPath;
        } else {
          showCheckpointsPathInput = true;
          return;
        }
      } catch {
        showCheckpointsPathInput = true;
        return;
      }
    }

    modelDownloading = true;
    modelDownloadProgress = null;
    modelCheckError = null;

    // Listen for progress events
    const unlisten = await onComfyuiModelDownloadProgress((progress) => {
      modelDownloadProgress = progress;
    });

    try {
      // Save the checkpoints path for future use
      await settingsApi.update('comfyui_checkpoints_path', checkpointsPath);

      await comfyuiModelApi.downloadModel(
        SDXL_MODEL_NAME,
        SDXL_DOWNLOAD_URL,
        checkpointsPath,
      );
      modelDownloadComplete = true;
      // Re-check model availability
      modelStatus = await comfyuiModelApi.checkModel(SDXL_MODEL_NAME);
    } catch (e) {
      modelCheckError = `ダウンロードに失敗しました: ${e}`;
    } finally {
      modelDownloading = false;
      unlisten();
    }
  }

  async function handleSetCheckpointsPath() {
    if (!checkpointsPath.trim()) return;
    showCheckpointsPathInput = false;
    await handleDownloadModel();
  }

  // Load persisted generation state when character changes
  $effect(() => {
    if (characterId) {
      generationStore.loadState(characterId);
    }
  });

  // --- Stage navigation ---
  const stages = [
    { key: 'concept_art' as const, label: 'コンセプトアート', icon: Wand2 },
    { key: 'concept' as const, label: 'ピクセルアート変換', icon: Palette },
    { key: 'direction' as const, label: '方向展開', icon: Compass },
    { key: 'animation' as const, label: 'アニメーション', icon: Film },
    { key: 'completed' as const, label: '完了', icon: Check },
  ];

  const stageOrder = ['idle', 'concept_art', 'concept', 'direction', 'animation', 'completed'] as const;

  function stageReached(target: string): boolean {
    const currentIdx = stageOrder.indexOf(generationStore.stage as typeof stageOrder[number]);
    const targetIdx = stageOrder.indexOf(target as typeof stageOrder[number]);
    return currentIdx >= targetIdx;
  }

  const stageIndex = $derived(
    (() => {
      const s = generationStore.stage;
      if (s === 'idle') return 0;
      return stages.findIndex((st) => st.key === s);
    })(),
  );

  // --- Model Selection ---
  let selectedCheckpoint = $state<string | undefined>(undefined);
  let selectedLora = $state<string | undefined>(undefined);
  let loraStrength = $state(1.0);

  // Fetch available checkpoints/loras on mount if empty
  $effect(() => {
    if (comfyuiChecked && comfyuiStore.isConnected) {
      if (generationStore.availableCheckpoints.length === 0) {
        generationStore.fetchCheckpoints();
      }
      if (generationStore.availableLoras.length === 0) {
        generationStore.fetchLoras();
      }
    }
  });

  // --- Concept Params ---
  let conceptPrompt = $state('');
  let conceptNegativePrompt = $state('');
  let conceptCandidates = $state(4);
  let conceptSteps = $state(20);
  let conceptCfgScale = $state(7);

  // Load project defaults into prompts
  $effect(() => {
    const p = projectStore.currentProject;
    if (p) {
      if (!conceptPrompt) conceptPrompt = p.style_prompt;
      if (!conceptNegativePrompt) conceptNegativePrompt = p.negative_prompt;
    }
  });

  // --- Pixel Art Conversion Params ---
  let pixelDenoiseStrength = $state(0.55);
  let pixelCandidates = $state(4);
  let pixelSteps = $state(20);
  let pixelCfgScale = $state(7);

  // --- Direction Params ---
  let ipadapterWeight = $state(0.8);
  let directionSteps = $state(20);
  let directionCfgScale = $state(7);
  let useControlnet = $state(false);
  let controlnetStrength = $state(0.5);

  // --- Animation Params ---
  let selectedDirection = $state<string | null>(null);
  let selectedAnimation = $state<string | null>(null);
  let animIpadapterWeight = $state(0.8);
  let animSteps = $state(20);
  let animCfgScale = $state(7);
  let useKeyframeInterpolation = $state(false);
  let keyframeIndices = $state<boolean[]>([]);
  let interpolationMethod = $state('crossfade');

  // Reset keyframe indices when animation changes
  const selectedAnimDef = $derived(projectAnimations.find((a) => a.name === selectedAnimation));
  $effect(() => {
    const frameCount = selectedAnimDef?.frame_count ?? 0;
    if (frameCount > 0 && keyframeIndices.length !== frameCount) {
      keyframeIndices = Array(frameCount).fill(false);
    }
  });

  // --- Handlers ---
  async function handleGenerateConceptArt() {
    if (!characterId) return;

    const params: ConceptParams = {
      positive_prompt: conceptPrompt,
      negative_prompt: conceptNegativePrompt,
      num_candidates: conceptCandidates,
      steps: conceptSteps,
      cfg_scale: conceptCfgScale,
      checkpoint_name: selectedCheckpoint,
      lora_name: selectedLora,
      lora_strength_model: selectedLora ? loraStrength : undefined,
      lora_strength_clip: selectedLora ? loraStrength : undefined,
    };

    await generationStore.generateConceptArt(characterId, params);
  }

  async function handleConvertToPixelArt() {
    if (!characterId) return;

    const params: PixelArtConversionParams = {
      positive_prompt: conceptPrompt,
      negative_prompt: conceptNegativePrompt,
      denoise_strength: pixelDenoiseStrength,
      num_candidates: pixelCandidates,
      steps: pixelSteps,
      cfg_scale: pixelCfgScale,
      checkpoint_name: selectedCheckpoint,
      lora_name: selectedLora,
      lora_strength_model: selectedLora ? loraStrength : undefined,
      lora_strength_clip: selectedLora ? loraStrength : undefined,
    };

    await generationStore.convertToPixelArt(characterId, params);
  }

  async function handleGenerateDirections() {
    if (!characterId) return;

    const params: DirectionParams = {
      directions: projectDirections,
      ipadapter_weight: ipadapterWeight,
      steps: directionSteps,
      cfg_scale: directionCfgScale,
      checkpoint_name: selectedCheckpoint,
      use_controlnet: useControlnet ? true : undefined,
      controlnet_strength: useControlnet ? controlnetStrength : undefined,
    };

    await generationStore.generateDirections(characterId, params);
  }

  async function handleGenerateAnimation() {
    if (!characterId || !selectedDirection || !selectedAnimation) return;

    // Find the direction image to use as base
    const dirIndex = projectDirections.indexOf(selectedDirection);
    const basePosePath = generationStore.directionImages[dirIndex];
    if (!basePosePath) return;

    const animDef = projectAnimations.find((a) => a.name === selectedAnimation);
    if (!animDef) return;

    const selectedKeyframes = useKeyframeInterpolation
      ? keyframeIndices.reduce<number[]>((acc, checked, idx) => {
          if (checked) acc.push(idx);
          return acc;
        }, [])
      : undefined;

    const params: AnimationParams = {
      animation_name: animDef.name,
      frame_count: animDef.frame_count,
      ipadapter_weight: animIpadapterWeight,
      steps: animSteps,
      cfg_scale: animCfgScale,
      checkpoint_name: selectedCheckpoint,
      keyframes: selectedKeyframes,
      interpolation: useKeyframeInterpolation ? interpolationMethod : undefined,
    };

    await generationStore.generateAnimationFrames(
      characterId,
      basePosePath,
      selectedDirection,
      params,
    );
  }

  async function handlePromote() {
    // Collect all generated sprite IDs from the sprite list
    const generatedSprites = spriteStore.sprites.filter(
      (s) => s.status === 'generated',
    );
    if (generatedSprites.length === 0) {
      return;
    }

    const ids = generatedSprites.map((s) => s.id);
    await generationStore.promoteToRaw(ids);

    // Reload sprites to reflect the change
    if (characterId) {
      await spriteStore.loadSprites(characterId);
    }
  }

  function handleReset() {
    generationStore.reset();
  }

  async function handleResetToIdle() {
    if (!characterId) return;
    await generationStore.resetToIdle(characterId);
  }

  async function handleResetToConceptArt() {
    if (!characterId) return;
    await generationStore.resetToConceptArt(characterId);
  }

  async function handleResetToPixelArt() {
    if (!characterId) return;
    await generationStore.resetToPixelArt(characterId);
  }

  async function handleResetToDirection() {
    if (!characterId) return;
    await generationStore.resetToDirection(characterId);
  }

  // --- Save / Bookmark ---
  async function handleSaveImage(imagePath: string, stage: string) {
    if (!characterId) return;
    if (isSavedImage(imagePath, stage)) return;
    await generationStore.saveImage(characterId, imagePath, stage);
  }

  async function handleDeleteSaved(imagePath: string) {
    if (!characterId) return;
    await generationStore.deleteSavedImage(characterId, imagePath);
  }

  function isSavedImage(imagePath: string, stage: string): boolean {
    const filename = imagePath.split('/').pop() || '';
    const savedList =
      stage === 'concept_art' ? generationStore.savedImages.concept_art
      : stage === 'pixel_art' ? generationStore.savedImages.pixel_art
      : stage === 'direction' ? generationStore.savedImages.direction
      : generationStore.savedImages.animation;
    return savedList.some((p) => p.endsWith(filename));
  }
</script>

<div class="generate-page">
  <!-- Stage Progress Indicator -->
  <div class="stage-progress">
    {#each stages as stg, i}
      <div
        class="stage-step"
        class:active={i === stageIndex}
        class:completed={i < stageIndex}
      >
        <div class="stage-icon">
          {#if i < stageIndex}
            <Check size={14} />
          {:else}
            <stg.icon size={14} />
          {/if}
        </div>
        <span class="stage-label">{stg.label}</span>
      </div>
      {#if i < stages.length - 1}
        <div class="stage-connector" class:active={i < stageIndex}></div>
      {/if}
    {/each}
  </div>

  <!-- ComfyUI Connection Status -->
  <div class="connection-bar">
    {#if !comfyuiChecked}
      <div class="connection-indicator checking">
        <Loader2 size={14} class="spin" />
        <span>ComfyUI の接続状態を確認中...</span>
      </div>
    {:else if comfyuiStore.isConnected}
      <div class="connection-indicator connected">
        <Check size={14} />
        <span>ComfyUI に接続済み ({comfyuiStore.endpoint})</span>
      </div>
    {:else}
      <div class="connection-indicator disconnected">
        <AlertTriangle size={14} />
        <span>ComfyUI に未接続 — 接続設定を確認してください</span>
        <a href="/settings/comfyui" class="btn btn-ghost btn-sm">設定へ</a>
      </div>
    {/if}
  </div>

  <!-- SDXL Checkpoint Model Status -->
  {#if comfyuiChecked && comfyuiStore.isConnected}
    {#if modelCheckLoading}
      <div class="connection-indicator checking">
        <Loader2 size={14} class="spin" />
        <span>チェックポイントモデルを確認中...</span>
      </div>
    {:else if modelAvailable}
      <div class="connection-indicator connected">
        <Check size={14} />
        <span>SDXL Base 1.0 モデルが利用可能です</span>
      </div>
    {:else if modelStatus && !modelStatus.available}
      <div class="model-check-section">
        <div class="connection-indicator warning">
          <AlertTriangle size={14} />
          <span>
            SDXL Base 1.0 チェックポイントモデルが見つかりません．
            生成にはこのモデルが必要です．
          </span>
        </div>

        {#if modelCheckError}
          <div class="error-banner">
            <AlertTriangle size={16} />
            <span>{modelCheckError}</span>
          </div>
        {/if}

        {#if showCheckpointsPathInput}
          <div class="checkpoints-path-form">
            <label class="label" for="checkpoints-path">
              <HardDrive size={14} />
              ComfyUI checkpoints ディレクトリのパスを入力してください
            </label>
            <p class="stage-description" style="margin-bottom: var(--space-2);">
              通常は ComfyUI インストールフォルダ内の
              <code>models/checkpoints</code> です．
            </p>
            <div class="path-input-row">
              <input
                id="checkpoints-path"
                type="text"
                class="input"
                placeholder="/path/to/ComfyUI/models/checkpoints"
                bind:value={checkpointsPath}
              />
              <button
                class="btn btn-primary btn-sm"
                onclick={handleSetCheckpointsPath}
                disabled={!checkpointsPath.trim()}
              >
                設定してダウンロード
              </button>
            </div>
          </div>
        {:else if modelDownloading}
          <div class="model-download-progress">
            <ProgressBar
              current={modelDownloadProgress?.downloaded_bytes ?? 0}
              total={modelDownloadProgress?.total_bytes ?? 0}
              label="SDXL Base 1.0 をダウンロード中..."
              showPercentage={true}
              showCounts={false}
              statusText={modelDownloadProgress
                ? `${(modelDownloadProgress.downloaded_bytes / 1024 / 1024).toFixed(0)} MB / ${(modelDownloadProgress.total_bytes / 1024 / 1024).toFixed(0)} MB`
                : '開始中...'}
              animated={true}
            />
          </div>
        {:else}
          <div class="model-download-actions">
            <button
              class="btn btn-primary"
              onclick={handleDownloadModel}
            >
              <Download size={16} />
              SDXL Base 1.0 をダウンロード（約 6.94 GB）
            </button>
            {#if modelStatus.available_models.length > 0}
              <p class="stage-description">
                利用可能なモデル: {modelStatus.available_models.join('，')}
              </p>
            {/if}
          </div>
        {/if}
      </div>
    {/if}
  {/if}

  <!-- Model Selection -->
  {#if comfyuiChecked && comfyuiStore.isConnected}
    <section class="stage-section model-selection-section">
      <h2>モデル選択</h2>
      <p class="stage-description">
        全ステージで使用するチェックポイントと LoRA を選択します．未選択の場合はデフォルトが使用されます．
      </p>

      <div class="params-grid">
        <div class="param-item">
          <label class="label" for="checkpoint-select">チェックポイント選択</label>
          <select
            id="checkpoint-select"
            class="select"
            bind:value={selectedCheckpoint}
            disabled={generationStore.loading}
          >
            <option value={undefined}>デフォルト</option>
            {#each generationStore.availableCheckpoints as ckpt}
              <option value={ckpt}>{ckpt}</option>
            {/each}
          </select>
        </div>

        <div class="param-item">
          <label class="label" for="lora-select">LoRA 選択</label>
          <select
            id="lora-select"
            class="select"
            bind:value={selectedLora}
            disabled={generationStore.loading}
          >
            <option value={undefined}>なし</option>
            {#each generationStore.availableLoras as lora}
              <option value={lora}>{lora}</option>
            {/each}
          </select>
        </div>

        {#if selectedLora}
          <div class="param-item">
            <label class="label" for="lora-strength">
              LoRA 強度
              <span class="param-value">{loraStrength.toFixed(1)}</span>
            </label>
            <input
              id="lora-strength"
              type="range"
              class="slider"
              min="0"
              max="2"
              step="0.1"
              bind:value={loraStrength}
              disabled={generationStore.loading}
            />
          </div>
        {/if}
      </div>
    </section>
  {/if}

  <!-- Progress Bar (during any generation) -->
  {#if generationStore.loading && generationStore.progress}
    <div class="generation-progress">
      <ProgressBar
        current={generationStore.progress.current}
        total={generationStore.progress.total}
        label={generationStore.progress.status}
        showPercentage={true}
        showCounts={true}
        statusText={generationStore.progress.message}
        animated={true}
      />
    </div>
  {/if}

  <!-- Error Display -->
  {#if generationStore.error}
    <div class="error-banner">
      <AlertTriangle size={16} />
      <span>{generationStore.error}</span>
    </div>
  {/if}

  <!-- ===== Stage 1a: Concept Art Generation ===== -->
    <section class="stage-section">
      <div class="stage-header">
        <h2><Wand2 size={20} /> Step 1a: コンセプトアート生成</h2>
        {#if generationStore.hasConceptArtImages}
          <button class="btn-reset" onclick={handleResetToIdle} disabled={generationStore.loading}>
            <RotateCcw size={16} />
            コンセプトアートをやり直す
          </button>
        {/if}
      </div>
      <p class="stage-description">
        テキストからコンセプトアート（手描き風）を生成します．
        候補から1枚を選択し，次のステップでピクセルアートに変換します．
      </p>

      <div class="params-section">
        <div class="form-group">
          <label class="label" for="concept-prompt">Positive Prompt</label>
          <textarea
            id="concept-prompt"
            class="textarea"
            placeholder="character concept art, detailed illustration, fantasy warrior..."
            bind:value={conceptPrompt}
            disabled={!comfyuiStore.isConnected || generationStore.loading}
            rows="3"
          ></textarea>
        </div>

        <div class="form-group">
          <label class="label" for="concept-neg-prompt">Negative Prompt</label>
          <textarea
            id="concept-neg-prompt"
            class="textarea"
            placeholder="blurry, low quality, watermark..."
            bind:value={conceptNegativePrompt}
            disabled={!comfyuiStore.isConnected || generationStore.loading}
            rows="2"
          ></textarea>
        </div>

        <!-- Prompt Hints -->
        <button
          class="prompt-hints-toggle"
          onclick={() => (showPromptHints = !showPromptHints)}
        >
          <Info size={14} />
          <span>プロンプト設定のコツ</span>
          {#if showPromptHints}
            <ChevronUp size={14} />
          {:else}
            <ChevronDown size={14} />
          {/if}
        </button>

        {#if showPromptHints}
          <div class="prompt-hints">
            <div class="hint-section">
              <h4>コンセプトアート生成について</h4>
              <p>
                このステップでは手描き風のコンセプトアートを生成します．
                ピクセルアート関連のキーワードは次のステップで自動付与されるため，
                ここではキャラクターのデザインや特徴に集中してください．
              </p>
            </div>

            <div class="hint-section">
              <h4>効果的なプロンプトの書き方</h4>
              <ul class="hint-list">
                <li>重要なキーワードをプロンプトの<strong>前半</strong>に配置する</li>
                <li>キャラクターの特徴（武器，服装，色）を具体的に記述する</li>
                <li><code>front facing</code>, <code>centered on canvas</code> で正面向き・中央配置を指示</li>
                <li><code>concept art</code>, <code>character design</code> でイラスト調を強化</li>
              </ul>
            </div>

            <div class="hint-section">
              <h4>プロンプト例</h4>
              <div class="hint-examples">
                <div class="hint-example">
                  <code>chibi warrior character, front facing, character design sheet, sword and shield, blue armor, fantasy RPG</code>
                </div>
                <div class="hint-example">
                  <code>cute slime monster, green translucent body, simple design, game enemy concept art, centered on canvas</code>
                </div>
              </div>
            </div>

            <div class="hint-section">
              <h4>モデル情報</h4>
              <p>
                <strong>SDXL Base 1.0</strong>（1024x1024）·
                サンプラー: euler · スケジューラ: normal · denoise: 1.0
              </p>
            </div>
          </div>
        {/if}

        <div class="params-grid">
          <div class="param-item">
            <label class="label" for="concept-candidates">
              候補数
              <span class="param-value">{conceptCandidates}</span>
            </label>
            <input
              id="concept-candidates"
              type="range"
              class="slider"
              min="1"
              max="8"
              step="1"
              bind:value={conceptCandidates}
              disabled={!comfyuiStore.isConnected || generationStore.loading}
            />
          </div>

          <div class="param-item">
            <label class="label" for="concept-steps">
              Steps
              <span class="param-value">{conceptSteps}</span>
            </label>
            <input
              id="concept-steps"
              type="range"
              class="slider"
              min="10"
              max="50"
              step="1"
              bind:value={conceptSteps}
              disabled={!comfyuiStore.isConnected || generationStore.loading}
            />
          </div>

          <div class="param-item">
            <label class="label" for="concept-cfg">
              CFG Scale
              <span class="param-value">{conceptCfgScale}</span>
            </label>
            <input
              id="concept-cfg"
              type="range"
              class="slider"
              min="1"
              max="20"
              step="0.5"
              bind:value={conceptCfgScale}
              disabled={!comfyuiStore.isConnected || generationStore.loading}
            />
          </div>
        </div>

        <button
          class="btn btn-primary"
          onclick={handleGenerateConceptArt}
          disabled={!comfyuiStore.isConnected || generationStore.loading || !conceptPrompt}
        >
          {#if generationStore.loading && generationStore.stage === 'concept_art'}
            <Loader2 size={16} class="spin" />
            生成中...
          {:else}
            <Sparkles size={16} />
            コンセプトアートを生成
          {/if}
        </button>
      </div>

      <!-- Concept Art Image Gallery -->
      {#if generationStore.hasConceptArtImages}
        <div class="gallery-section">
          <h3>コンセプトアート候補（クリックで選択）</h3>
          <div class="image-grid">
            {#each generationStore.conceptArtImages as imgPath}
              <div
                class="image-card"
                class:selected={generationStore.selectedConceptArtPath === imgPath}
                role="button"
                tabindex="0"
                onclick={() => generationStore.selectConceptArt(imgPath)}
                onkeydown={(e) => { if (e.key === 'Enter' || e.key === ' ') generationStore.selectConceptArt(imgPath); }}
              >
                <img src={convertFileSrc(imgPath)} alt="コンセプトアート候補" />
                {#if generationStore.selectedConceptArtPath === imgPath}
                  <div class="selected-overlay">
                    <Check size={24} />
                  </div>
                {/if}
                <button
                  class="save-btn"
                  class:saved={isSavedImage(imgPath, 'concept_art')}
                  onclick={(e) => { e.stopPropagation(); handleSaveImage(imgPath, 'concept_art'); }}
                  title={isSavedImage(imgPath, 'concept_art') ? '保存済み' : 'プロジェクトに保存'}
                >
                  <Heart size={16} fill={isSavedImage(imgPath, 'concept_art') ? 'currentColor' : 'none'} />
                </button>
              </div>
            {/each}
          </div>
        </div>
      {/if}
    </section>

  <!-- ===== Stage 1b: Pixel Art Conversion ===== -->
  {#if generationStore.selectedConceptArtPath !== null || generationStore.conceptImages.length > 0}
    <section class="stage-section">
      <div class="stage-header">
        <h2><Palette size={20} /> Step 1b: ピクセルアート変換</h2>
        {#if generationStore.hasConceptImages}
          <button class="btn-reset" onclick={handleResetToConceptArt} disabled={generationStore.loading}>
            <RotateCcw size={16} />
            ピクセルアート変換をやり直す
          </button>
        {/if}
      </div>
      <p class="stage-description">
        選択したコンセプトアートを img2img でピクセルアートスタイルに変換します．
        デノイズ強度で変換の度合いを調整できます．
      </p>

      <div class="selected-concept-preview">
        <span class="preview-label">選択中のコンセプトアート:</span>
        {#if generationStore.selectedConceptArtPath}
          <div class="thumbnail checkerboard-bg">
            <img src={convertFileSrc(generationStore.selectedConceptArtPath)} alt="選択コンセプトアート" />
          </div>
        {/if}
      </div>

      <div class="params-section">
        <div class="params-grid">
          <div class="param-item">
            <label class="label" for="pixel-denoise">
              ピクセル化強度
              <span class="param-value">{pixelDenoiseStrength.toFixed(2)}</span>
            </label>
            <input
              id="pixel-denoise"
              type="range"
              class="slider"
              min="0.3"
              max="0.8"
              step="0.05"
              bind:value={pixelDenoiseStrength}
              disabled={!comfyuiStore.isConnected || generationStore.loading}
            />
            <span class="param-hint">低い値＝元画像に近い，高い値＝よりピクセルアート的</span>
          </div>

          <div class="param-item">
            <label class="label" for="pixel-candidates">
              候補数
              <span class="param-value">{pixelCandidates}</span>
            </label>
            <input
              id="pixel-candidates"
              type="range"
              class="slider"
              min="1"
              max="8"
              step="1"
              bind:value={pixelCandidates}
              disabled={!comfyuiStore.isConnected || generationStore.loading}
            />
          </div>

          <div class="param-item">
            <label class="label" for="pixel-steps">
              Steps
              <span class="param-value">{pixelSteps}</span>
            </label>
            <input
              id="pixel-steps"
              type="range"
              class="slider"
              min="10"
              max="50"
              step="1"
              bind:value={pixelSteps}
              disabled={!comfyuiStore.isConnected || generationStore.loading}
            />
          </div>

          <div class="param-item">
            <label class="label" for="pixel-cfg">
              CFG Scale
              <span class="param-value">{pixelCfgScale}</span>
            </label>
            <input
              id="pixel-cfg"
              type="range"
              class="slider"
              min="1"
              max="20"
              step="0.5"
              bind:value={pixelCfgScale}
              disabled={!comfyuiStore.isConnected || generationStore.loading}
            />
          </div>
        </div>

        <button
          class="btn btn-primary"
          onclick={handleConvertToPixelArt}
          disabled={!comfyuiStore.isConnected || generationStore.loading || !generationStore.selectedConceptArtPath}
        >
          {#if generationStore.loading && generationStore.stage === 'concept'}
            <Loader2 size={16} class="spin" />
            変換中...
          {:else}
            <Palette size={16} />
            ピクセルアートに変換
          {/if}
        </button>
      </div>

      <!-- Pixel Art Image Gallery -->
      {#if generationStore.hasConceptImages}
        <div class="gallery-section">
          <h3>ピクセルアート候補（クリックで選択）</h3>
          <div class="image-grid">
            {#each generationStore.conceptImages as imgPath}
              <div
                class="image-card"
                class:selected={generationStore.selectedConceptPath === imgPath}
                role="button"
                tabindex="0"
                onclick={() => generationStore.selectConcept(imgPath)}
                onkeydown={(e) => { if (e.key === 'Enter' || e.key === ' ') generationStore.selectConcept(imgPath); }}
              >
                <img src={convertFileSrc(imgPath)} alt="ピクセルアート候補" />
                {#if generationStore.selectedConceptPath === imgPath}
                  <div class="selected-overlay">
                    <Check size={24} />
                  </div>
                {/if}
                <button
                  class="save-btn"
                  class:saved={isSavedImage(imgPath, 'pixel_art')}
                  onclick={(e) => { e.stopPropagation(); handleSaveImage(imgPath, 'pixel_art'); }}
                  title={isSavedImage(imgPath, 'pixel_art') ? '保存済み' : 'プロジェクトに保存'}
                >
                  <Heart size={16} fill={isSavedImage(imgPath, 'pixel_art') ? 'currentColor' : 'none'} />
                </button>
              </div>
            {/each}
          </div>
        </div>
      {/if}
    </section>
  {/if}

  <!-- ===== Stage 2: Direction Expansion ===== -->
  {#if generationStore.selectedConceptPath !== null || generationStore.directionImages.length > 0}
    <section class="stage-section">
      <div class="stage-header">
        <h2><Compass size={20} /> Step 2: 方向展開</h2>
        {#if generationStore.hasDirectionImages}
          <button class="btn-reset" onclick={handleResetToPixelArt} disabled={generationStore.loading}>
            <RotateCcw size={16} />
            方向展開をやり直す
          </button>
        {/if}
      </div>
      <p class="stage-description">
        選択したコンセプトをIP-Adapterで各方向（{projectDirections.join('，')}）に展開します．
      </p>

      {#if generationStore.selectedConceptPath}
        <div class="selected-concept-preview">
          <span class="preview-label">選択中のコンセプト:</span>
          <div class="thumbnail checkerboard-bg">
            <img src={convertFileSrc(generationStore.selectedConceptPath)} alt="選択コンセプト" />
          </div>
        </div>
      {/if}

      <div class="params-grid">
        <div class="param-item">
          <label class="label" for="dir-ipadapter">
            IP-Adapter Weight
            <span class="param-value">{ipadapterWeight.toFixed(2)}</span>
          </label>
          <input
            id="dir-ipadapter"
            type="range"
            class="slider"
            min="0"
            max="1"
            step="0.05"
            bind:value={ipadapterWeight}
            disabled={!comfyuiStore.isConnected || generationStore.loading}
          />
        </div>

        <div class="param-item">
          <label class="label" for="dir-steps">
            Steps
            <span class="param-value">{directionSteps}</span>
          </label>
          <input
            id="dir-steps"
            type="range"
            class="slider"
            min="10"
            max="50"
            step="1"
            bind:value={directionSteps}
            disabled={!comfyuiStore.isConnected || generationStore.loading}
          />
        </div>

        <div class="param-item">
          <label class="label" for="dir-cfg">
            CFG Scale
            <span class="param-value">{directionCfgScale}</span>
          </label>
          <input
            id="dir-cfg"
            type="range"
            class="slider"
            min="1"
            max="20"
            step="0.5"
            bind:value={directionCfgScale}
            disabled={!comfyuiStore.isConnected || generationStore.loading}
          />
        </div>
      </div>

      <!-- ControlNet Depth -->
      <div class="controlnet-section">
        <label class="checkbox-label">
          <input
            type="checkbox"
            bind:checked={useControlnet}
            disabled={generationStore.loading}
          />
          <span>ControlNet Depth を使用</span>
        </label>

        {#if useControlnet}
          <div class="params-grid" style="margin-top: var(--space-3);">
            <div class="param-item">
              <label class="label" for="controlnet-strength">
                ControlNet 強度
                <span class="param-value">{controlnetStrength.toFixed(2)}</span>
              </label>
              <input
                id="controlnet-strength"
                type="range"
                class="slider"
                min="0"
                max="1"
                step="0.05"
                bind:value={controlnetStrength}
                disabled={!comfyuiStore.isConnected || generationStore.loading}
              />
            </div>
          </div>
        {/if}
      </div>

      <button
        class="btn btn-primary"
        onclick={handleGenerateDirections}
        disabled={!comfyuiStore.isConnected || generationStore.loading}
      >
        {#if generationStore.loading && generationStore.stage === 'direction'}
          <Loader2 size={16} class="spin" />
          展開中...
        {:else}
          <ChevronRight size={16} />
          方向展開を実行
        {/if}
      </button>

      <!-- Direction Results -->
      {#if generationStore.hasDirectionImages}
        <div class="gallery-section">
          <h3>方向別画像</h3>
          <div class="image-grid direction-grid">
            {#each generationStore.directionImages as imgPath, i}
              <div class="image-card direction-card">
                <img src={convertFileSrc(imgPath)} alt={projectDirections[i] ?? `方向${i}`} />
                <span class="direction-label">{projectDirections[i] ?? `方向${i}`}</span>
                <button
                  class="save-btn"
                  class:saved={isSavedImage(imgPath, 'direction')}
                  onclick={(e) => { e.stopPropagation(); handleSaveImage(imgPath, 'direction'); }}
                  title={isSavedImage(imgPath, 'direction') ? '保存済み' : 'プロジェクトに保存'}
                >
                  <Heart size={16} fill={isSavedImage(imgPath, 'direction') ? 'currentColor' : 'none'} />
                </button>
              </div>
            {/each}
          </div>
        </div>
      {/if}
    </section>
  {/if}

  <!-- ===== Stage 3: Animation Frames ===== -->
  {#if generationStore.directionImages.length > 0 || generationStore.animationFrames.length > 0}
    <section class="stage-section">
      <div class="stage-header">
        <h2><Film size={20} /> Step 3: アニメーション生成</h2>
        {#if generationStore.hasAnimationFrames}
          <button class="btn-reset" onclick={handleResetToDirection} disabled={generationStore.loading}>
            <RotateCcw size={16} />
            アニメーションをやり直す
          </button>
        {/if}
      </div>
      <p class="stage-description">
        各方向のポーズ画像からアニメーションフレームを生成します．
        方向とアニメーションを選択して実行してください．
      </p>

      <div class="animation-selectors">
        <div class="form-group">
          <label class="label" for="anim-direction">方向</label>
          <select
            id="anim-direction"
            class="select"
            bind:value={selectedDirection}
            disabled={generationStore.loading}
          >
            <option value={null}>方向を選択...</option>
            {#each projectDirections as dir}
              <option value={dir}>{dir}</option>
            {/each}
          </select>
        </div>

        <div class="form-group">
          <label class="label" for="anim-type">アニメーション</label>
          <select
            id="anim-type"
            class="select"
            bind:value={selectedAnimation}
            disabled={generationStore.loading}
          >
            <option value={null}>アニメーションを選択...</option>
            {#each projectAnimations as anim}
              <option value={anim.name}>{anim.name} ({anim.frame_count}フレーム)</option>
            {/each}
          </select>
        </div>
      </div>

      <div class="params-grid">
        <div class="param-item">
          <label class="label" for="anim-ipadapter">
            IP-Adapter Weight
            <span class="param-value">{animIpadapterWeight.toFixed(2)}</span>
          </label>
          <input
            id="anim-ipadapter"
            type="range"
            class="slider"
            min="0"
            max="1"
            step="0.05"
            bind:value={animIpadapterWeight}
            disabled={!comfyuiStore.isConnected || generationStore.loading}
          />
        </div>

        <div class="param-item">
          <label class="label" for="anim-steps">
            Steps
            <span class="param-value">{animSteps}</span>
          </label>
          <input
            id="anim-steps"
            type="range"
            class="slider"
            min="10"
            max="50"
            step="1"
            bind:value={animSteps}
            disabled={!comfyuiStore.isConnected || generationStore.loading}
          />
        </div>

        <div class="param-item">
          <label class="label" for="anim-cfg">
            CFG Scale
            <span class="param-value">{animCfgScale}</span>
          </label>
          <input
            id="anim-cfg"
            type="range"
            class="slider"
            min="1"
            max="20"
            step="0.5"
            bind:value={animCfgScale}
            disabled={!comfyuiStore.isConnected || generationStore.loading}
          />
        </div>
      </div>

      <!-- Keyframe Interpolation -->
      <div class="controlnet-section">
        <label class="checkbox-label">
          <input
            type="checkbox"
            bind:checked={useKeyframeInterpolation}
            disabled={generationStore.loading}
          />
          <span>キーフレーム補間を使用</span>
        </label>

        {#if useKeyframeInterpolation && selectedAnimDef}
          <div class="keyframe-options" style="margin-top: var(--space-3);">
            <div class="form-group">
              <label class="label">キーフレーム選択</label>
              <div class="keyframe-checkboxes">
                {#each Array(selectedAnimDef.frame_count) as _, i}
                  <label class="keyframe-checkbox-label">
                    <input
                      type="checkbox"
                      bind:checked={keyframeIndices[i]}
                      disabled={generationStore.loading}
                    />
                    <span>F{i}</span>
                  </label>
                {/each}
              </div>
            </div>

            <div class="form-group">
              <label class="label" for="interpolation-method">補間方式</label>
              <select
                id="interpolation-method"
                class="select"
                bind:value={interpolationMethod}
                disabled={generationStore.loading}
              >
                <option value="crossfade">crossfade</option>
                <option value="nearest">nearest</option>
              </select>
            </div>
          </div>
        {/if}
      </div>

      <button
        class="btn btn-primary"
        onclick={handleGenerateAnimation}
        disabled={
          !comfyuiStore.isConnected ||
          generationStore.loading ||
          !selectedDirection ||
          !selectedAnimation
        }
      >
        {#if generationStore.loading && generationStore.stage === 'animation'}
          <Loader2 size={16} class="spin" />
          生成中...
        {:else}
          <Film size={16} />
          アニメーション生成
        {/if}
      </button>

      <!-- Animation Frame Results -->
      {#if generationStore.hasAnimationFrames}
        <div class="gallery-section">
          <h3>生成されたフレーム</h3>
          <div class="image-grid frame-grid">
            {#each generationStore.animationFrames as framePath, i}
              <div class="image-card frame-card">
                <img src={convertFileSrc(framePath)} alt={`フレーム ${i}`} />
                <span class="frame-label">F{i}</span>
                <button
                  class="save-btn"
                  class:saved={isSavedImage(framePath, 'animation')}
                  onclick={(e) => { e.stopPropagation(); handleSaveImage(framePath, 'animation'); }}
                  title={isSavedImage(framePath, 'animation') ? '保存済み' : 'プロジェクトに保存'}
                >
                  <Heart size={16} fill={isSavedImage(framePath, 'animation') ? 'currentColor' : 'none'} />
                </button>
              </div>
            {/each}
          </div>
        </div>
      {/if}
    </section>
  {/if}

  <!-- ===== Promote / Finalize ===== -->
  {#if generationStore.animationFrames.length > 0}
    <section class="stage-section promote-section">
      <h2><ArrowUpToLine size={20} /> スプライトとして取り込み</h2>
      <p class="stage-description">
        生成した画像をスプライトパイプラインに取り込みます．
        取り込み後はAI処理タブで背景除去や正規化を行えます．
      </p>

      <div class="promote-actions">
        <button
          class="btn btn-primary"
          onclick={handlePromote}
          disabled={generationStore.loading}
        >
          {#if generationStore.loading && generationStore.stage === 'completed'}
            <Loader2 size={16} class="spin" />
            取り込み中...
          {:else}
            <ArrowUpToLine size={16} />
            全てスプライトに取り込む
          {/if}
        </button>

        <button
          class="btn btn-ghost"
          onclick={handleReset}
          disabled={generationStore.loading}
        >
          <RefreshCw size={16} />
          リセット
        </button>
      </div>

      {#if generationStore.stage === 'completed'}
        <div class="completion-banner">
          <Check size={16} />
          <span>スプライトの取り込みが完了しました．AI処理タブで次のステップに進めます．</span>
        </div>
      {/if}
    </section>
  {/if}

  <!-- ===== Saved Images ===== -->
  {#if generationStore.hasSavedImages}
    <section class="stage-section saved-section">
      <h2><Heart size={20} /> 保存済み画像</h2>
      <p class="stage-description">
        やり直しても保持される保存済み画像です．不要な画像は削除できます．
      </p>

      {#if generationStore.savedImages.concept_art.length > 0}
        <div class="saved-stage">
          <h3>コンセプトアート</h3>
          <div class="image-grid">
            {#each generationStore.savedImages.concept_art as img}
              <div class="image-card">
                <img src={convertFileSrc(img)} alt="保存済みコンセプトアート" />
                <button
                  class="delete-saved-btn"
                  onclick={() => handleDeleteSaved(img)}
                  title="保存済み画像を削除"
                >
                  <X size={14} />
                </button>
              </div>
            {/each}
          </div>
        </div>
      {/if}

      {#if generationStore.savedImages.pixel_art.length > 0}
        <div class="saved-stage">
          <h3>ピクセルアート</h3>
          <div class="image-grid">
            {#each generationStore.savedImages.pixel_art as img}
              <div class="image-card">
                <img src={convertFileSrc(img)} alt="保存済みピクセルアート" />
                <button
                  class="delete-saved-btn"
                  onclick={() => handleDeleteSaved(img)}
                  title="保存済み画像を削除"
                >
                  <X size={14} />
                </button>
              </div>
            {/each}
          </div>
        </div>
      {/if}

      {#if generationStore.savedImages.direction.length > 0}
        <div class="saved-stage">
          <h3>方向別画像</h3>
          <div class="image-grid direction-grid">
            {#each generationStore.savedImages.direction as img}
              <div class="image-card">
                <img src={convertFileSrc(img)} alt="保存済み方向画像" />
                <button
                  class="delete-saved-btn"
                  onclick={() => handleDeleteSaved(img)}
                  title="保存済み画像を削除"
                >
                  <X size={14} />
                </button>
              </div>
            {/each}
          </div>
        </div>
      {/if}

      {#if generationStore.savedImages.animation.length > 0}
        <div class="saved-stage">
          <h3>アニメーションフレーム</h3>
          <div class="image-grid frame-grid">
            {#each generationStore.savedImages.animation as img}
              <div class="image-card">
                <img src={convertFileSrc(img)} alt="保存済みアニメーション" />
                <button
                  class="delete-saved-btn"
                  onclick={() => handleDeleteSaved(img)}
                  title="保存済み画像を削除"
                >
                  <X size={14} />
                </button>
              </div>
            {/each}
          </div>
        </div>
      {/if}
    </section>
  {/if}
</div>

<style>
  .generate-page {
    display: flex;
    flex-direction: column;
    gap: var(--space-6);
  }

  /* Stage Progress Indicator */
  .stage-progress {
    display: flex;
    align-items: center;
    gap: 0;
    padding: var(--space-4);
    background: var(--bg-secondary);
    border: 1px solid var(--border-default);
    border-radius: 8px;
  }

  .stage-step {
    display: flex;
    align-items: center;
    gap: var(--space-2);
    padding: var(--space-1) var(--space-3);
    font-size: var(--text-sm);
    color: var(--text-muted);
    white-space: nowrap;
  }

  .stage-step.active {
    color: var(--accent-primary);
    font-weight: var(--font-weight-medium);
  }

  .stage-step.completed {
    color: var(--accent-success);
  }

  .stage-icon {
    display: flex;
    align-items: center;
    justify-content: center;
    width: 24px;
    height: 24px;
    border-radius: 50%;
    border: 1.5px solid var(--border-default);
  }

  .stage-step.active .stage-icon {
    border-color: var(--accent-primary);
    background: hsla(var(--accent-hue, 220), 60%, 50%, 0.1);
  }

  .stage-step.completed .stage-icon {
    border-color: var(--accent-success);
    background: hsla(142, 60%, 45%, 0.1);
  }

  .stage-connector {
    flex: 1;
    height: 1.5px;
    background: var(--border-default);
    min-width: 16px;
  }

  .stage-connector.active {
    background: var(--accent-success);
  }

  /* Connection Bar */
  .connection-bar {
    margin-bottom: var(--space-2);
  }

  .connection-indicator {
    display: flex;
    align-items: center;
    gap: var(--space-2);
    padding: var(--space-2) var(--space-3);
    border-radius: 6px;
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

  .connection-indicator.warning {
    background: hsla(45, 90%, 55%, 0.1);
    border: 1px solid hsla(45, 90%, 55%, 0.3);
    color: var(--accent-warning);
  }

  /* Model Check Section */
  .model-check-section {
    display: flex;
    flex-direction: column;
    gap: var(--space-3);
  }

  .model-download-actions {
    display: flex;
    flex-direction: column;
    gap: var(--space-2);
  }

  .model-download-progress {
    padding: var(--space-4);
    background: var(--bg-secondary);
    border: 1px solid var(--border-default);
    border-radius: 8px;
  }

  .checkpoints-path-form {
    padding: var(--space-4);
    background: var(--bg-secondary);
    border: 1px solid var(--border-default);
    border-radius: 8px;
    display: flex;
    flex-direction: column;
    gap: var(--space-2);
  }

  .checkpoints-path-form .label {
    display: flex;
    align-items: center;
    gap: var(--space-2);
    font-weight: var(--font-weight-medium);
  }

  .checkpoints-path-form code {
    padding: 1px 4px;
    background: var(--bg-tertiary);
    border-radius: 3px;
    font-size: var(--text-xs);
    font-family: var(--font-mono);
  }

  .path-input-row {
    display: flex;
    gap: var(--space-2);
    align-items: center;
  }

  .path-input-row .input {
    flex: 1;
  }

  /* Progress */
  .generation-progress {
    padding: var(--space-4);
    background: var(--bg-secondary);
    border: 1px solid var(--border-default);
    border-radius: 8px;
  }

  /* Error */
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
  }

  /* Stage Sections */
  .stage-section {
    padding: var(--space-5);
    background: var(--bg-primary);
    border: 1px solid var(--border-default);
    border-radius: 8px;
  }

  .stage-header {
    display: flex;
    align-items: center;
    justify-content: space-between;
    margin-bottom: var(--space-2);
  }

  .stage-section h2 {
    display: flex;
    align-items: center;
    gap: var(--space-2);
    margin-bottom: var(--space-2);
  }

  .stage-header h2 {
    margin-bottom: 0;
  }

  .btn-reset {
    display: inline-flex;
    align-items: center;
    gap: 0.375rem;
    padding: 0.375rem 0.75rem;
    font-size: 0.8125rem;
    color: var(--text-secondary, #6b7280);
    background: transparent;
    border: 1px solid var(--border-default, #d1d5db);
    border-radius: 0.375rem;
    cursor: pointer;
    transition: all 0.15s ease;
    white-space: nowrap;
  }

  .btn-reset:hover:not(:disabled) {
    color: var(--accent-warning, #d97706);
    border-color: var(--accent-warning, #d97706);
    background: rgba(217, 119, 6, 0.05);
  }

  .btn-reset:disabled {
    opacity: 0.5;
    cursor: not-allowed;
  }

  .stage-description {
    font-size: var(--text-sm);
    color: var(--text-secondary);
    margin-bottom: var(--space-4);
    line-height: 1.6;
  }

  /* Params */
  .params-section {
    display: flex;
    flex-direction: column;
    gap: var(--space-4);
  }

  .params-grid {
    display: grid;
    grid-template-columns: repeat(auto-fill, minmax(220px, 1fr));
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

  .param-hint {
    font-size: var(--text-xs);
    color: var(--text-muted);
    line-height: 1.4;
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

  .slider:disabled {
    opacity: 0.5;
    cursor: not-allowed;
  }

  /* Image Grid / Gallery */
  .gallery-section {
    margin-top: var(--space-5);
  }

  .gallery-section h3 {
    margin-bottom: var(--space-3);
    font-size: var(--text-base);
  }

  .image-grid {
    display: grid;
    grid-template-columns: repeat(auto-fill, minmax(160px, 1fr));
    gap: var(--space-3);
  }

  .direction-grid {
    grid-template-columns: repeat(auto-fill, minmax(140px, 1fr));
  }

  .frame-grid {
    grid-template-columns: repeat(auto-fill, minmax(100px, 1fr));
  }

  .image-card {
    position: relative;
    background: var(--bg-secondary);
    border: 2px solid var(--border-default);
    border-radius: 8px;
    overflow: hidden;
    cursor: pointer;
    transition: border-color 150ms ease, transform 100ms ease;
  }

  .image-card:hover {
    border-color: var(--accent-primary);
    transform: translateY(-2px);
  }

  .image-card.selected {
    border-color: var(--accent-primary);
    box-shadow: 0 0 0 2px hsla(var(--accent-hue, 220), 60%, 50%, 0.3);
  }

  .image-card img {
    width: 100%;
    aspect-ratio: 1;
    object-fit: contain;
    image-rendering: pixelated;
  }

  .direction-card,
  .frame-card {
    cursor: default;
  }

  .direction-card:hover,
  .frame-card:hover {
    transform: none;
  }

  .selected-overlay {
    position: absolute;
    inset: 0;
    display: flex;
    align-items: center;
    justify-content: center;
    background: hsla(var(--accent-hue, 220), 60%, 50%, 0.2);
    color: var(--accent-primary);
  }

  .direction-label,
  .frame-label {
    display: block;
    text-align: center;
    padding: var(--space-1) var(--space-2);
    font-size: var(--text-xs);
    color: var(--text-secondary);
    background: var(--bg-secondary);
    border-top: 1px solid var(--border-default);
    text-transform: capitalize;
  }

  /* Selected Concept Preview */
  .selected-concept-preview {
    display: flex;
    align-items: center;
    gap: var(--space-3);
    margin-bottom: var(--space-4);
    padding: var(--space-3);
    background: var(--bg-secondary);
    border: 1px solid var(--border-default);
    border-radius: 8px;
  }

  .preview-label {
    font-size: var(--text-sm);
    color: var(--text-secondary);
    white-space: nowrap;
  }

  .thumbnail {
    width: 80px;
    height: 80px;
    border-radius: 4px;
    overflow: hidden;
    border: 1px solid var(--border-default);
  }

  .thumbnail img {
    width: 100%;
    height: 100%;
    object-fit: contain;
    image-rendering: pixelated;
  }

  /* Animation Selectors */
  .animation-selectors {
    display: grid;
    grid-template-columns: 1fr 1fr;
    gap: var(--space-4);
    margin-bottom: var(--space-4);
  }

  /* Promote Section */
  .promote-section {
    border-color: var(--accent-primary);
  }

  .promote-actions {
    display: flex;
    gap: var(--space-3);
    margin-bottom: var(--space-4);
  }

  .completion-banner {
    display: flex;
    align-items: center;
    gap: var(--space-2);
    padding: var(--space-3);
    background: hsla(142, 60%, 45%, 0.1);
    border: 1px solid hsla(142, 60%, 45%, 0.3);
    border-radius: 6px;
    color: var(--accent-success);
    font-size: var(--text-sm);
  }

  /* Button Styles (shared) */
  .btn-sm {
    height: 28px;
    padding: 0 var(--space-2);
    font-size: var(--text-xs);
  }

  /* Prompt Hints */
  .prompt-hints-toggle {
    display: flex;
    align-items: center;
    gap: var(--space-2);
    padding: var(--space-2) var(--space-3);
    background: none;
    border: 1px solid var(--border-default);
    border-radius: 6px;
    color: var(--text-secondary);
    font-size: var(--text-sm);
    cursor: pointer;
    transition: background 150ms ease, color 150ms ease;
    width: fit-content;
  }

  .prompt-hints-toggle:hover {
    background: var(--bg-secondary);
    color: var(--text-primary);
  }

  .prompt-hints {
    padding: var(--space-4);
    background: var(--bg-secondary);
    border: 1px solid var(--border-default);
    border-radius: 8px;
    display: flex;
    flex-direction: column;
    gap: var(--space-4);
  }

  .hint-section h4 {
    font-size: var(--text-sm);
    font-weight: var(--font-weight-medium);
    color: var(--text-primary);
    margin-bottom: var(--space-2);
  }

  .hint-section p {
    font-size: var(--text-sm);
    color: var(--text-secondary);
    line-height: 1.6;
  }

  .hint-keywords {
    display: flex;
    flex-direction: column;
    gap: var(--space-2);
  }

  .hint-keyword-group {
    display: flex;
    align-items: baseline;
    gap: var(--space-2);
    font-size: var(--text-sm);
  }

  .hint-label {
    font-weight: var(--font-weight-medium);
    color: var(--text-secondary);
    white-space: nowrap;
    min-width: 70px;
  }

  .hint-keyword-group code {
    font-family: var(--font-mono);
    font-size: var(--text-xs);
    color: var(--accent-primary);
    background: var(--bg-tertiary);
    padding: var(--space-1) var(--space-2);
    border-radius: 4px;
    line-height: 1.6;
    word-break: break-word;
  }

  .hint-list {
    list-style: none;
    padding: 0;
    margin: 0;
    display: flex;
    flex-direction: column;
    gap: var(--space-2);
  }

  .hint-list li {
    font-size: var(--text-sm);
    color: var(--text-secondary);
    line-height: 1.6;
    padding-left: var(--space-4);
    position: relative;
  }

  .hint-list li::before {
    content: '·';
    position: absolute;
    left: var(--space-1);
    color: var(--text-muted);
    font-weight: bold;
  }

  .hint-list li code {
    font-family: var(--font-mono);
    font-size: var(--text-xs);
    color: var(--accent-primary);
    background: var(--bg-tertiary);
    padding: 1px var(--space-1);
    border-radius: 3px;
  }

  .hint-examples {
    display: flex;
    flex-direction: column;
    gap: var(--space-2);
  }

  .hint-example code {
    display: block;
    font-family: var(--font-mono);
    font-size: var(--text-xs);
    color: var(--text-secondary);
    background: var(--bg-tertiary);
    padding: var(--space-2) var(--space-3);
    border-radius: 6px;
    border-left: 3px solid var(--accent-primary);
    line-height: 1.6;
    word-break: break-word;
  }

  /* Save / Bookmark Button */
  .save-btn {
    position: absolute;
    top: 0.375rem;
    right: 0.375rem;
    display: flex;
    align-items: center;
    justify-content: center;
    width: 1.75rem;
    height: 1.75rem;
    padding: 0;
    background: rgba(0, 0, 0, 0.5);
    border: none;
    border-radius: 50%;
    color: white;
    cursor: pointer;
    opacity: 0;
    transition: opacity 0.15s ease, color 0.15s ease;
    z-index: 2;
  }

  .image-card:hover .save-btn,
  .save-btn.saved {
    opacity: 1;
  }

  .save-btn.saved {
    color: #ef4444;
    background: rgba(0, 0, 0, 0.6);
  }

  .save-btn:hover:not(.saved) {
    color: #ef4444;
  }

  /* Delete Saved Button */
  .delete-saved-btn {
    position: absolute;
    top: 0.25rem;
    right: 0.25rem;
    display: flex;
    align-items: center;
    justify-content: center;
    width: 1.5rem;
    height: 1.5rem;
    padding: 0;
    background: rgba(0, 0, 0, 0.6);
    border: none;
    border-radius: 50%;
    color: white;
    cursor: pointer;
    opacity: 0;
    transition: opacity 0.15s ease;
    z-index: 2;
  }

  .image-card:hover .delete-saved-btn {
    opacity: 1;
  }

  .delete-saved-btn:hover {
    background: rgba(239, 68, 68, 0.8);
  }

  /* Saved Images Section */
  .saved-section {
    margin-top: 2rem;
    padding-top: 1.5rem;
    border-top: 1px solid var(--border-default, #e5e7eb);
  }

  .saved-stage {
    margin-bottom: 1.5rem;
  }

  .saved-stage h3 {
    font-size: 0.9375rem;
    font-weight: 600;
    margin-bottom: 0.75rem;
    color: var(--text-secondary, #6b7280);
  }

  /* Model Selection Section */
  .model-selection-section {
    border-color: var(--border-default);
  }

  /* ControlNet / Checkbox Sections */
  .controlnet-section {
    padding: var(--space-3) var(--space-4);
    background: var(--bg-secondary);
    border: 1px solid var(--border-default);
    border-radius: 8px;
    margin-top: var(--space-3);
  }

  .checkbox-label {
    display: flex;
    align-items: center;
    gap: var(--space-2);
    font-size: var(--text-sm);
    color: var(--text-primary);
    cursor: pointer;
  }

  .checkbox-label input[type="checkbox"] {
    width: 16px;
    height: 16px;
    cursor: pointer;
  }

  /* Keyframe Options */
  .keyframe-options {
    display: flex;
    flex-direction: column;
    gap: var(--space-3);
  }

  .keyframe-checkboxes {
    display: flex;
    flex-wrap: wrap;
    gap: var(--space-2);
    padding: var(--space-2);
  }

  .keyframe-checkbox-label {
    display: flex;
    align-items: center;
    gap: var(--space-1);
    font-size: var(--text-sm);
    color: var(--text-secondary);
    cursor: pointer;
    padding: var(--space-1) var(--space-2);
    background: var(--bg-tertiary);
    border-radius: 4px;
    transition: background 150ms ease;
  }

  .keyframe-checkbox-label:hover {
    background: var(--bg-primary);
  }

  .keyframe-checkbox-label input[type="checkbox"] {
    width: 14px;
    height: 14px;
    cursor: pointer;
  }
</style>
