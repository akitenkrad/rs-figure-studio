<script lang="ts">
  import { page } from '$app/stores';
  import { Loader2, LayoutGrid, Eye, Package, Check, AlertTriangle, FolderOpen } from 'lucide-svelte';
  import { convertFileSrc } from '@tauri-apps/api/core';
  import { open } from '@tauri-apps/plugin-dialog';
  import SpritePreview from '$lib/components/SpritePreview.svelte';
  import { characterStore } from '$lib/stores/character.svelte';
  import { projectStore } from '$lib/stores/project.svelte';
  import { spriteStore } from '$lib/stores/sprite.svelte';
  import { toastStore } from '$lib/stores/toast.svelte';
  import { spriteApi } from '$lib/api/tauri';
  import type { AnimationDef, BevyExportResult } from '$lib/types';

  const characterId = $derived($page.params.id);

  // Parse project directions and animations
  const directions = $derived(
    (() => {
      const p = projectStore.currentProject;
      if (!p?.directions) return [];
      try {
        return JSON.parse(p.directions) as string[];
      } catch {
        return [];
      }
    })(),
  );

  const animations = $derived(
    (() => {
      const p = projectStore.currentProject;
      if (!p?.animations) return [];
      try {
        return JSON.parse(p.animations) as AnimationDef[];
      } catch {
        return [];
      }
    })(),
  );

  const tileWidth = $derived(projectStore.currentProject?.tile_width ?? 64);
  const tileHeight = $derived(projectStore.currentProject?.tile_height ?? 64);

  const spritesheetPath = $derived(characterStore.currentCharacter?.spritesheet_path ?? null);

  const hasSprites = $derived(spriteStore.spriteCount > 0);

  // --- Export State ---
  let exporting = $state(false);
  let exportResult = $state<BevyExportResult | null>(null);
  let exportError = $state<string | null>(null);

  async function handleGenerateSpritesheet() {
    if (!characterId) return;
    try {
      await spriteStore.generateSpritesheet(characterId);
      // Reload character to get updated spritesheet_path
      await characterStore.loadCharacter(characterId);
    } catch (e) {
      toastStore.addToast('error', `スプライトシート生成に失敗しました: ${e}`);
    }
  }

  async function handleBevyExport() {
    if (!characterId) return;

    // Open directory selection dialog
    const selected = await open({
      directory: true,
      title: 'Bevyエクスポート先を選択',
    });

    if (!selected) return; // User cancelled

    exporting = true;
    exportResult = null;
    exportError = null;

    try {
      const result = await spriteApi.exportForBevy(characterId, selected as string);
      exportResult = result;
      toastStore.addToast('success', 'Bevyエクスポートが完了しました');
    } catch (e) {
      exportError = String(e);
      toastStore.addToast('error', `Bevyエクスポートに失敗しました: ${e}`);
    } finally {
      exporting = false;
    }
  }
</script>

<div class="preview-page">
  {#if spritesheetPath}
    <!-- Spritesheet exists: show preview -->
    <section class="preview-section">
      <h2><Eye size={20} /> アニメーションプレビュー</h2>
      <SpritePreview
        {spritesheetPath}
        {tileWidth}
        {tileHeight}
        {directions}
        {animations}
      />
    </section>

    <!-- Full spritesheet view -->
    <section class="spritesheet-section">
      <h2>スプライトシート全体</h2>
      <div class="spritesheet-info">
        <div class="info-item">
          <span class="info-label">パス</span>
          <span class="info-value path-value">{spritesheetPath}</span>
        </div>
        <div class="info-item">
          <span class="info-label">タイルサイズ</span>
          <span class="info-value">{tileWidth} x {tileHeight}px</span>
        </div>
      </div>
      <div class="spritesheet-viewer checkerboard-bg">
        <img
          src={convertFileSrc(spritesheetPath)}
          alt="スプライトシート全体"
        />
      </div>
    </section>

    <!-- Action Buttons -->
    <section class="actions-section">
      <button
        class="btn btn-secondary"
        onclick={handleGenerateSpritesheet}
        disabled={spriteStore.generating}
      >
        {#if spriteStore.generating}
          <Loader2 size={16} class="spin" />
          再生成中...
        {:else}
          <LayoutGrid size={16} />
          スプライトシートを再生成
        {/if}
      </button>

      <button
        class="btn btn-primary"
        onclick={handleBevyExport}
        disabled={exporting}
      >
        {#if exporting}
          <Loader2 size={16} class="spin" />
          エクスポート中...
        {:else}
          <Package size={16} />
          Bevyエクスポート
        {/if}
      </button>
    </section>

    <!-- Export Result Card -->
    {#if exportResult}
      <section class="export-result">
        <div class="result-card success">
          <div class="result-header">
            <Check size={20} />
            <h3>エクスポート完了</h3>
          </div>
          <div class="result-body">
            <div class="result-item">
              <span class="result-label">スプライトシート</span>
              <span class="result-value">{exportResult.spritesheet_path}</span>
            </div>
            <div class="result-item">
              <span class="result-label">メタデータ</span>
              <span class="result-value">{exportResult.metadata_path}</span>
            </div>
          </div>
        </div>
      </section>
    {/if}

    <!-- Export Error -->
    {#if exportError}
      <section class="export-result">
        <div class="result-card error">
          <div class="result-header">
            <AlertTriangle size={20} />
            <h3>エクスポート失敗</h3>
          </div>
          <div class="result-body">
            <p class="error-message">{exportError}</p>
          </div>
        </div>
      </section>
    {/if}
  {:else if hasSprites}
    <!-- No spritesheet yet, but has sprites -->
    <div class="empty-state">
      <LayoutGrid size={48} />
      <h2>スプライトシートが未生成です</h2>
      <p class="empty-description">
        インポートされたスプライトからスプライトシートを生成してください．
      </p>
      <button
        class="btn btn-primary"
        onclick={handleGenerateSpritesheet}
        disabled={spriteStore.generating}
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
  {:else}
    <!-- No sprites at all -->
    <div class="empty-state">
      <Eye size={48} />
      <h2>スプライトがありません</h2>
      <p class="empty-description">
        まずインポートタブから画像を取り込んでください．
      </p>
      <a href="/character/{characterId}/import" class="btn btn-primary">
        インポートページへ
      </a>
    </div>
  {/if}
</div>

<style>
  .preview-page {
    display: flex;
    flex-direction: column;
    gap: var(--space-8);
  }

  .preview-section h2,
  .spritesheet-section h2 {
    display: flex;
    align-items: center;
    gap: var(--space-2);
    margin-bottom: var(--space-4);
  }

  .spritesheet-info {
    display: grid;
    grid-template-columns: 1fr 1fr;
    gap: var(--space-4);
    margin-bottom: var(--space-4);
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

  .spritesheet-viewer {
    border-radius: 8px;
    border: 1px solid var(--border-default);
    padding: var(--space-4);
    overflow: auto;
    max-height: 500px;
  }

  .spritesheet-viewer img {
    image-rendering: pixelated;
    max-width: 100%;
  }

  .actions-section {
    display: flex;
    gap: var(--space-3);
  }

  /* Export Result Card */
  .export-result {
    margin-top: calc(-1 * var(--space-4));
  }

  .result-card {
    border-radius: 8px;
    padding: var(--space-4);
  }

  .result-card.success {
    background: hsla(142, 60%, 45%, 0.08);
    border: 1px solid hsla(142, 60%, 45%, 0.3);
  }

  .result-card.error {
    background: hsla(0, 70%, 55%, 0.08);
    border: 1px solid hsla(0, 70%, 55%, 0.3);
  }

  .result-header {
    display: flex;
    align-items: center;
    gap: var(--space-2);
    margin-bottom: var(--space-3);
  }

  .result-card.success .result-header {
    color: var(--accent-success);
  }

  .result-card.error .result-header {
    color: var(--accent-danger);
  }

  .result-body {
    display: flex;
    flex-direction: column;
    gap: var(--space-2);
  }

  .result-item {
    display: flex;
    flex-direction: column;
    gap: var(--space-1);
  }

  .result-label {
    font-size: var(--text-xs);
    font-weight: var(--font-weight-medium);
    color: var(--text-muted);
    text-transform: uppercase;
    letter-spacing: 0.05em;
  }

  .result-value {
    font-size: var(--text-sm);
    font-family: var(--font-mono);
    color: var(--text-primary);
    word-break: break-all;
  }

  .error-message {
    font-size: var(--text-sm);
    color: var(--accent-danger);
  }

  .empty-state {
    display: flex;
    flex-direction: column;
    align-items: center;
    justify-content: center;
    gap: var(--space-4);
    padding: var(--space-12) var(--space-6);
    text-align: center;
    color: var(--text-muted);
  }

  .empty-state h2 {
    color: var(--text-secondary);
  }

  .empty-description {
    color: var(--text-muted);
    font-size: var(--text-sm);
    max-width: 400px;
  }
</style>
