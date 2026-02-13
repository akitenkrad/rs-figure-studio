<script lang="ts">
  import { goto } from '$app/navigation';
  import { FolderOpen } from 'lucide-svelte';
  import { projectStore } from '$lib/stores/project.svelte';
  import Breadcrumb from '$lib/components/Breadcrumb.svelte';
  import type { CreateProject, AnimationDef } from '$lib/types';

  const breadcrumbItems = [
    { label: 'ホーム', href: '/' },
    { label: '新規プロジェクト' },
  ];

  // --- Form State ---
  let name = $state('');
  let basePath = $state('');
  let tileWidth = $state(64);
  let tileHeight = $state(64);
  let stylePrompt = $state('');
  let negativePrompt = $state('');
  let controlnetWeight = $state(0.8);
  let submitting = $state(false);
  let errors = $state<Record<string, string>>({});

  // Default directions
  const directions = ['down', 'left', 'right', 'up'];

  // Default animations
  let animations = $state<AnimationDef[]>([
    { name: 'idle', frame_count: 4, frame_duration_ms: 200 },
    { name: 'walk', frame_count: 4, frame_duration_ms: 150 },
  ]);

  function addAnimation() {
    animations = [...animations, { name: '', frame_count: 4, frame_duration_ms: 150 }];
  }

  function removeAnimation(index: number) {
    animations = animations.filter((_, i) => i !== index);
  }

  async function selectDirectory() {
    try {
      const { open } = await import('@tauri-apps/plugin-dialog');
      const selected = await open({ directory: true, multiple: false });
      if (selected) {
        basePath = selected as string;
      }
    } catch {
      // Fallback: allow manual input (for dev without Tauri)
    }
  }

  function validate(): boolean {
    const newErrors: Record<string, string> = {};
    if (!name.trim()) newErrors.name = 'プロジェクト名を入力してください';
    if (!basePath.trim()) newErrors.basePath = 'ベースパスを選択してください';
    if (tileWidth < 8 || tileWidth > 512) newErrors.tileWidth = 'タイルサイズは8~512pxの範囲で指定してください';
    if (tileHeight < 8 || tileHeight > 512) newErrors.tileHeight = 'タイルサイズは8~512pxの範囲で指定してください';
    if (animations.some(a => !a.name.trim())) newErrors.animations = 'アニメーション名を入力してください';
    errors = newErrors;
    return Object.keys(newErrors).length === 0;
  }

  async function handleSubmit() {
    if (!validate()) return;
    submitting = true;

    const data: CreateProject = {
      name: name.trim(),
      base_path: basePath.trim(),
      tile_width: tileWidth,
      tile_height: tileHeight,
      directions,
      animations,
      style_prompt: stylePrompt.trim(),
      negative_prompt: negativePrompt.trim(),
      controlnet_weight: controlnetWeight,
    };

    const project = await projectStore.createProject(data);
    submitting = false;

    if (project) {
      goto(`/project/${project.id}`);
    }
  }

  function handleCancel() {
    goto('/');
  }
</script>

<Breadcrumb items={breadcrumbItems} />

<div class="create-page">
  <h1>新規プロジェクト作成</h1>

  <form class="project-form" onsubmit={(e) => { e.preventDefault(); handleSubmit(); }}>
    <!-- Project Name -->
    <div class="form-group">
      <label class="label" for="name">プロジェクト名 <span class="required">*</span></label>
      <input
        id="name"
        class="input"
        type="text"
        bind:value={name}
        placeholder="例: My RPG Characters"
      />
      {#if errors.name}
        <p class="error-text">{errors.name}</p>
      {/if}
    </div>

    <!-- Base Path -->
    <div class="form-group">
      <label class="label" for="basePath">ベースパス <span class="required">*</span></label>
      <div class="path-input">
        <input
          id="basePath"
          class="input"
          type="text"
          bind:value={basePath}
          placeholder="/path/to/project"
          readonly
        />
        <button type="button" class="btn btn-secondary" onclick={selectDirectory}>
          <FolderOpen size={16} />
          選択
        </button>
      </div>
      {#if errors.basePath}
        <p class="error-text">{errors.basePath}</p>
      {/if}
    </div>

    <!-- Tile Size -->
    <div class="form-row">
      <div class="form-group">
        <label class="label" for="tileWidth">タイル幅 (px)</label>
        <input
          id="tileWidth"
          class="input"
          type="number"
          bind:value={tileWidth}
          min="8"
          max="512"
          step="8"
        />
        {#if errors.tileWidth}
          <p class="error-text">{errors.tileWidth}</p>
        {/if}
      </div>
      <div class="form-group">
        <label class="label" for="tileHeight">タイル高さ (px)</label>
        <input
          id="tileHeight"
          class="input"
          type="number"
          bind:value={tileHeight}
          min="8"
          max="512"
          step="8"
        />
        {#if errors.tileHeight}
          <p class="error-text">{errors.tileHeight}</p>
        {/if}
      </div>
    </div>

    <!-- Animations -->
    <div class="form-group">
      <!-- svelte-ignore a11y_label_has_associated_control -->
      <label class="label">アニメーション定義</label>
      <div class="animation-list">
        {#each animations as anim, i}
          <div class="animation-row">
            <input
              class="input"
              type="text"
              bind:value={anim.name}
              placeholder="アニメーション名"
            />
            <input
              class="input input-small"
              type="number"
              bind:value={anim.frame_count}
              min="1"
              max="32"
              title="フレーム数"
            />
            <input
              class="input input-small"
              type="number"
              bind:value={anim.frame_duration_ms}
              min="16"
              max="2000"
              step="10"
              title="フレーム時間 (ms)"
            />
            <button type="button" class="btn btn-ghost" onclick={() => removeAnimation(i)} aria-label="削除">
              &times;
            </button>
          </div>
        {/each}
      </div>
      <button type="button" class="btn btn-ghost" onclick={addAnimation}>
        + アニメーション追加
      </button>
      {#if errors.animations}
        <p class="error-text">{errors.animations}</p>
      {/if}
    </div>

    <!-- Style Prompt -->
    <div class="form-group">
      <label class="label" for="stylePrompt">スタイルプロンプト</label>
      <textarea
        id="stylePrompt"
        class="textarea"
        bind:value={stylePrompt}
        placeholder="pixel art style, 16-bit RPG character..."
        rows="3"
      ></textarea>
    </div>

    <!-- Negative Prompt -->
    <div class="form-group">
      <label class="label" for="negativePrompt">ネガティブプロンプト</label>
      <textarea
        id="negativePrompt"
        class="textarea"
        bind:value={negativePrompt}
        placeholder="blurry, low quality, watermark..."
        rows="2"
      ></textarea>
    </div>

    <!-- ControlNet Weight -->
    <div class="form-group">
      <label class="label" for="controlnetWeight">ControlNet Weight: {controlnetWeight.toFixed(2)}</label>
      <input
        id="controlnetWeight"
        type="range"
        min="0"
        max="2"
        step="0.05"
        bind:value={controlnetWeight}
        class="range-input"
      />
    </div>

    <!-- Actions -->
    <div class="form-actions">
      <button type="button" class="btn btn-secondary" onclick={handleCancel}>
        キャンセル
      </button>
      <button type="submit" class="btn btn-primary" disabled={submitting}>
        {submitting ? '作成中...' : 'プロジェクトを作成'}
      </button>
    </div>
  </form>
</div>

<style>
  .create-page {
    max-width: 640px;
  }

  .create-page h1 {
    margin-bottom: var(--space-6);
  }

  .project-form {
    display: flex;
    flex-direction: column;
  }

  .path-input {
    display: flex;
    gap: var(--space-2);
  }

  .path-input .input {
    flex: 1;
  }

  .form-row {
    display: grid;
    grid-template-columns: 1fr 1fr;
    gap: var(--space-4);
  }

  .required {
    color: var(--accent-danger);
  }

  .error-text {
    margin-top: var(--space-1);
    font-size: var(--text-xs);
    color: var(--accent-danger);
  }

  .animation-list {
    display: flex;
    flex-direction: column;
    gap: var(--space-2);
    margin-bottom: var(--space-2);
  }

  .animation-row {
    display: flex;
    gap: var(--space-2);
    align-items: center;
  }

  .animation-row .input:first-child {
    flex: 1;
  }

  .input-small {
    width: 80px;
    flex-shrink: 0;
  }

  .range-input {
    width: 100%;
    accent-color: var(--accent-primary);
  }

  .form-actions {
    display: flex;
    justify-content: flex-end;
    gap: var(--space-3);
    margin-top: var(--space-6);
    padding-top: var(--space-4);
    border-top: 1px solid var(--border-default);
  }
</style>
