<script lang="ts">
  import { page } from '$app/stores';
  import { convertFileSrc } from '@tauri-apps/api/core';
  import { Loader2, Grid, Wand2, LayoutGrid } from 'lucide-svelte';
  import ImageDropZone from '$lib/components/ImageDropZone.svelte';
  import { characterStore } from '$lib/stores/character.svelte';
  import { spriteStore } from '$lib/stores/sprite.svelte';
  import { projectStore } from '$lib/stores/project.svelte';
  import type { Sprite } from '$lib/types';

  const characterId = $derived($page.params.id);

  // Parse project directions and animations
  const directions = $derived(
    (() => {
      const p = projectStore.currentProject;
      if (!p?.directions) return [];
      if (Array.isArray(p.directions)) return p.directions as string[];
      try {
        return JSON.parse(p.directions as string) as string[];
      } catch {
        return [];
      }
    })(),
  );

  const animations = $derived(
    (() => {
      const p = projectStore.currentProject;
      if (!p?.animations) return [];
      if (Array.isArray(p.animations)) return p.animations as { name: string; frame_count: number }[];
      try {
        return JSON.parse(p.animations as string) as { name: string; frame_count: number }[];
      } catch {
        return [];
      }
    })(),
  );

  const tileWidth = $derived(projectStore.currentProject?.tile_width ?? 64);
  const tileHeight = $derived(projectStore.currentProject?.tile_height ?? 64);

  // Group sprites by direction x animation for matrix display
  const spriteMatrix = $derived(
    (() => {
      const matrix: Record<string, Record<string, Sprite[]>> = {};
      for (const dir of directions) {
        matrix[dir] = {};
        for (const anim of animations) {
          matrix[dir][anim.name] = [];
        }
      }
      // Also create an "unassigned" group
      matrix['unassigned'] = {};

      for (const sprite of spriteStore.sprites) {
        const dir = sprite.direction || 'unassigned';
        const anim = sprite.animation || 'unassigned';
        if (!matrix[dir]) {
          matrix[dir] = {};
        }
        if (!matrix[dir][anim]) {
          matrix[dir][anim] = [];
        }
        matrix[dir][anim].push(sprite);
      }

      return matrix;
    })(),
  );

  const hasUnassigned = $derived(
    spriteStore.sprites.some((s) => !s.direction || !s.animation),
  );

  // --- Handlers ---
  async function handleFilesSelected(filePaths: string[]) {
    if (!characterId) return;
    await spriteStore.importSprites(characterId, filePaths);
  }

  async function handleNormalize() {
    if (!characterId) return;
    await spriteStore.normalizeBatch(characterId, tileWidth, tileHeight);
  }

  async function handleGenerateSpritesheet() {
    if (!characterId) return;
    await spriteStore.generateSpritesheet(characterId);
  }

  function getSpriteImageSrc(sprite: Sprite): string {
    const path = sprite.final_path ?? sprite.processed_path ?? sprite.raw_path;
    if (!path) return '';
    return convertFileSrc(path);
  }

  async function handleAssignment(spriteId: string, field: 'direction' | 'animation', value: string) {
    const sprite = spriteStore.sprites.find((s) => s.id === spriteId);
    if (!sprite) return;
    const dir = field === 'direction' ? value : sprite.direction;
    const anim = field === 'animation' ? value : sprite.animation;
    await spriteStore.updateAssignment(spriteId, dir, anim);
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
</script>

<div class="import-page">
  <!-- Drop Zone -->
  <section class="import-section">
    <h2>画像インポート</h2>
    <p class="section-description">
      MagicaVoxelでレンダリングした画像をドラッグ&ドロップまたはファイル選択で取り込みます．
    </p>
    <ImageDropZone
      onFilesSelected={handleFilesSelected}
      disabled={spriteStore.importing}
    />

    {#if spriteStore.importing}
      <div class="processing-indicator">
        <Loader2 size={20} class="spin" />
        <span>インポート中...</span>
      </div>
    {/if}
  </section>

  <!-- Imported Sprites Grid -->
  {#if spriteStore.spriteCount > 0}
    <section class="sprites-section">
      <div class="section-header">
        <h2><Grid size={20} /> インポート済みスプライト ({spriteStore.spriteCount}枚)</h2>
      </div>

      <!-- Unassigned sprites -->
      {#if hasUnassigned}
        <div class="sprite-group">
          <h3 class="group-title warning-text">未割り当て</h3>
          <p class="assign-description">方向とアニメーションを選択して割り当ててください．</p>
          <div class="sprite-grid-assign">
            {#each spriteStore.sprites.filter((s) => !s.direction || !s.animation) as sprite}
              <div class="sprite-assign-card">
                <div class="sprite-thumbnail checkerboard-bg">
                  {#if getSpriteImageSrc(sprite)}
                    <img src={getSpriteImageSrc(sprite)} alt="sprite" />
                  {/if}
                </div>
                <div class="sprite-assign-controls">
                  <label class="assign-label">
                    <span>方向</span>
                    <select
                      value={sprite.direction || ''}
                      onchange={(e) => handleAssignment(sprite.id, 'direction', (e.target as HTMLSelectElement).value)}
                    >
                      <option value="" disabled>選択...</option>
                      {#each directions as dir}
                        <option value={dir}>{dir}</option>
                      {/each}
                    </select>
                  </label>
                  <label class="assign-label">
                    <span>アニメーション</span>
                    <select
                      value={sprite.animation || ''}
                      onchange={(e) => handleAssignment(sprite.id, 'animation', (e.target as HTMLSelectElement).value)}
                    >
                      <option value="" disabled>選択...</option>
                      {#each animations as anim}
                        <option value={anim.name}>{anim.name}</option>
                      {/each}
                    </select>
                  </label>
                  <div class="sprite-info">
                    <span class="sprite-status badge badge-{sprite.status}">
                      {getStatusLabel(sprite.status)}
                    </span>
                    <span class="sprite-frame">F{sprite.frame_index}</span>
                  </div>
                </div>
              </div>
            {/each}
          </div>
        </div>
      {/if}

      <!-- Direction x Animation Matrix -->
      {#each directions as dir}
        {@const dirSprites = spriteMatrix[dir]}
        {#if dirSprites && Object.values(dirSprites).some((arr) => arr.length > 0)}
          <div class="sprite-group">
            <h3 class="group-title">{dir}</h3>
            {#each animations as anim}
              {@const animSprites = dirSprites[anim.name] ?? []}
              {#if animSprites.length > 0}
                <div class="anim-row">
                  <span class="anim-label">{anim.name}</span>
                  <div class="sprite-row">
                    {#each animSprites.sort((a, b) => a.frame_index - b.frame_index) as sprite}
                      <div class="sprite-cell">
                        <div class="sprite-thumbnail checkerboard-bg">
                          {#if getSpriteImageSrc(sprite)}
                            <img src={getSpriteImageSrc(sprite)} alt="sprite" />
                          {/if}
                        </div>
                        <div class="sprite-info">
                          <span class="sprite-status badge badge-{sprite.status}">
                            {getStatusLabel(sprite.status)}
                          </span>
                          <span class="sprite-frame">F{sprite.frame_index}</span>
                        </div>
                      </div>
                    {/each}
                  </div>
                </div>
              {/if}
            {/each}
          </div>
        {/if}
      {/each}
    </section>

    <!-- Action Buttons -->
    <section class="actions-section">
      <div class="action-buttons">
        <button
          class="btn btn-secondary"
          onclick={handleNormalize}
          disabled={spriteStore.normalizing || spriteStore.spriteCount === 0}
        >
          {#if spriteStore.normalizing}
            <Loader2 size={16} class="spin" />
            正規化中...
          {:else}
            <Wand2 size={16} />
            正規化を実行 ({tileWidth}x{tileHeight}px)
          {/if}
        </button>

        <button
          class="btn btn-primary"
          onclick={handleGenerateSpritesheet}
          disabled={spriteStore.generating || spriteStore.spriteCount === 0}
        >
          {#if spriteStore.generating}
            <Loader2 size={16} class="spin" />
            生成中...
          {:else}
            <LayoutGrid size={16} />
            スプライトシート生成
          {/if}
        </button>
      </div>

      {#if spriteStore.normalizing}
        <div class="processing-indicator">
          <Loader2 size={20} class="spin" />
          <span>スプライトを正規化しています...</span>
        </div>
      {/if}

      {#if spriteStore.generating}
        <div class="processing-indicator">
          <Loader2 size={20} class="spin" />
          <span>スプライトシートを生成しています...</span>
        </div>
      {/if}
    </section>

    <!-- Spritesheet Result -->
    {#if spriteStore.spritesheetResult}
      <section class="result-section">
        <h2>生成結果</h2>
        <div class="result-info">
          <div class="info-item">
            <span class="info-label">パス</span>
            <span class="info-value path-value">{spriteStore.spritesheetResult.path}</span>
          </div>
          <div class="info-item">
            <span class="info-label">サイズ</span>
            <span class="info-value">
              {spriteStore.spritesheetResult.columns}列 x {spriteStore.spritesheetResult.rows}行
              ({spriteStore.spritesheetResult.tile_width * spriteStore.spritesheetResult.columns}
              x
              {spriteStore.spritesheetResult.tile_height * spriteStore.spritesheetResult.rows}px)
            </span>
          </div>
        </div>
        <div class="spritesheet-preview checkerboard-bg">
          <img
            src={convertFileSrc(spriteStore.spritesheetResult.path)}
            alt="Generated spritesheet"
          />
        </div>
      </section>
    {/if}
  {/if}
</div>

<style>
  .import-page {
    display: flex;
    flex-direction: column;
    gap: var(--space-8);
  }

  .import-section h2 {
    margin-bottom: var(--space-2);
  }

  .section-description {
    color: var(--text-secondary);
    font-size: var(--text-sm);
    margin-bottom: var(--space-4);
  }

  .processing-indicator {
    display: flex;
    align-items: center;
    gap: var(--space-2);
    padding: var(--space-3);
    margin-top: var(--space-3);
    background: var(--bg-secondary);
    border-radius: 6px;
    color: var(--text-secondary);
    font-size: var(--text-sm);
  }

  .sprites-section {
    display: flex;
    flex-direction: column;
    gap: var(--space-4);
  }

  .section-header {
    display: flex;
    align-items: center;
    justify-content: space-between;
  }

  .section-header h2 {
    display: flex;
    align-items: center;
    gap: var(--space-2);
  }

  .sprite-group {
    background: var(--bg-secondary);
    border: 1px solid var(--border-default);
    border-radius: 8px;
    padding: var(--space-4);
  }

  .group-title {
    margin-bottom: var(--space-3);
    font-size: var(--text-base);
    text-transform: capitalize;
  }

  .warning-text {
    color: var(--accent-warning);
  }

  .assign-description {
    color: var(--text-secondary);
    font-size: var(--text-sm);
    margin-bottom: var(--space-3);
  }

  .sprite-grid-assign {
    display: grid;
    grid-template-columns: repeat(auto-fill, minmax(200px, 1fr));
    gap: var(--space-3);
  }

  .sprite-assign-card {
    display: flex;
    gap: var(--space-3);
    padding: var(--space-3);
    background: var(--bg-primary);
    border: 1px solid var(--border-default);
    border-radius: 6px;
  }

  .sprite-assign-controls {
    display: flex;
    flex-direction: column;
    gap: var(--space-2);
    flex: 1;
    min-width: 0;
  }

  .assign-label {
    display: flex;
    flex-direction: column;
    gap: 2px;
    font-size: var(--text-xs);
    color: var(--text-secondary);
  }

  .assign-label select {
    padding: var(--space-1) var(--space-2);
    border: 1px solid var(--border-default);
    border-radius: 4px;
    background: var(--bg-secondary);
    color: var(--text-primary);
    font-size: var(--text-sm);
  }

  .anim-row {
    display: flex;
    align-items: flex-start;
    gap: var(--space-3);
    margin-bottom: var(--space-3);
  }

  .anim-row:last-child {
    margin-bottom: 0;
  }

  .anim-label {
    min-width: 80px;
    padding-top: var(--space-2);
    font-size: var(--text-sm);
    font-weight: var(--font-weight-medium);
    color: var(--text-secondary);
    text-transform: capitalize;
  }

  .sprite-row {
    display: flex;
    gap: var(--space-2);
    flex-wrap: wrap;
  }

  .sprite-grid {
    display: grid;
    grid-template-columns: repeat(auto-fill, minmax(80px, 1fr));
    gap: var(--space-2);
  }

  .sprite-cell {
    display: flex;
    flex-direction: column;
    align-items: center;
    gap: var(--space-1);
  }

  .sprite-thumbnail {
    width: 72px;
    height: 72px;
    border-radius: 4px;
    overflow: hidden;
    display: flex;
    align-items: center;
    justify-content: center;
    border: 1px solid var(--border-default);
  }

  .sprite-thumbnail img {
    max-width: 100%;
    max-height: 100%;
    object-fit: contain;
    image-rendering: pixelated;
  }

  .sprite-info {
    display: flex;
    flex-direction: column;
    align-items: center;
    gap: 2px;
  }

  .sprite-status {
    font-size: 0.625rem;
  }

  .sprite-frame {
    font-size: var(--text-xs);
    color: var(--text-muted);
    font-family: var(--font-mono);
  }

  .actions-section {
    display: flex;
    flex-direction: column;
    gap: var(--space-3);
  }

  .action-buttons {
    display: flex;
    gap: var(--space-3);
  }

  .result-section {
    display: flex;
    flex-direction: column;
    gap: var(--space-4);
  }

  .result-info {
    display: grid;
    grid-template-columns: 1fr 1fr;
    gap: var(--space-4);
  }

  .info-item {
    padding: var(--space-3);
    background: var(--bg-secondary);
    border-radius: 6px;
    border: 1px solid var(--border-default);
  }

  .info-label {
    display: block;
    font-size: var(--text-xs);
    font-weight: var(--font-weight-medium);
    color: var(--text-muted);
    margin-bottom: var(--space-1);
    text-transform: uppercase;
    letter-spacing: 0.05em;
  }

  .info-value {
    font-size: var(--text-sm);
    color: var(--text-primary);
  }

  .path-value {
    word-break: break-all;
    font-family: var(--font-mono);
    font-size: var(--text-xs);
  }

  .spritesheet-preview {
    border-radius: 8px;
    border: 1px solid var(--border-default);
    padding: var(--space-4);
    overflow: auto;
    max-height: 400px;
  }

  .spritesheet-preview img {
    image-rendering: pixelated;
    max-width: 100%;
  }

  :global(.spin) {
    animation: spin 1s linear infinite;
  }
</style>
