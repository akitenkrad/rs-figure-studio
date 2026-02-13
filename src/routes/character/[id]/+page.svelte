<script lang="ts">
  import { page } from '$app/stores';
  import { goto } from '$app/navigation';
  import { Pencil, Trash2, Save, X, Loader2 } from 'lucide-svelte';
  import { characterStore } from '$lib/stores/character.svelte';
  import { spriteStore } from '$lib/stores/sprite.svelte';
  import ConfirmDialog from '$lib/components/ConfirmDialog.svelte';
  import type { CharacterCategory } from '$lib/types';

  const characterId = $derived($page.params.id);

  // Edit mode
  let editing = $state(false);
  let editName = $state('');
  let editCategory = $state<CharacterCategory>('player');
  let editCustomPrompt = $state('');

  // Delete confirm
  let showDeleteConfirm = $state(false);

  const categories: { value: CharacterCategory; label: string }[] = [
    { value: 'player', label: 'プレイヤー' },
    { value: 'enemy', label: '敵キャラクター' },
    { value: 'npc', label: 'NPC' },
  ];

  const categoryLabel = $derived(
    (() => {
      const c = characterStore.currentCharacter?.category;
      return categories.find((cat) => cat.value === c)?.label ?? c ?? '';
    })(),
  );

  const statusLabel = $derived(
    (() => {
      const s = characterStore.currentCharacter?.status;
      const map: Record<string, string> = {
        draft: '下書き',
        importing: 'インポート中',
        processing: '処理中',
        complete: '完了',
      };
      return s ? (map[s] ?? s) : '';
    })(),
  );

  function startEdit() {
    const c = characterStore.currentCharacter;
    if (!c) return;
    editName = c.name;
    editCategory = c.category;
    editCustomPrompt = c.custom_prompt ?? '';
    editing = true;
  }

  function cancelEdit() {
    editing = false;
  }

  async function saveEdit() {
    if (!editName.trim() || !characterId) return;
    await characterStore.updateCharacter(characterId, {
      name: editName.trim(),
      category: editCategory,
      custom_prompt: editCustomPrompt.trim() || undefined,
    });
    editing = false;
  }

  async function confirmDelete() {
    if (!characterId) return;
    const projectId = characterStore.currentCharacter?.project_id;
    await characterStore.deleteCharacter(characterId);
    if (projectId) {
      goto(`/project/${projectId}`);
    } else {
      goto('/');
    }
  }
</script>

{#if characterStore.loading && !characterStore.currentCharacter}
  <div class="loading-state">
    <Loader2 size={32} class="spin" />
    <p>読み込み中...</p>
  </div>
{:else if characterStore.currentCharacter}
  {@const character = characterStore.currentCharacter}

  <!-- Action Bar -->
  <div class="action-bar">
    {#if editing}
      <button class="btn btn-ghost" onclick={cancelEdit}>
        <X size={16} />
        キャンセル
      </button>
      <button class="btn btn-primary" onclick={saveEdit}>
        <Save size={16} />
        保存
      </button>
    {:else}
      <button class="btn btn-ghost" onclick={startEdit}>
        <Pencil size={16} />
        編集
      </button>
      <button class="btn btn-danger" onclick={() => (showDeleteConfirm = true)}>
        <Trash2 size={16} />
        削除
      </button>
    {/if}
  </div>

  <!-- Character Info -->
  <section class="character-info">
    {#if editing}
      <div class="form-group">
        <label class="label" for="editCharName">キャラクター名</label>
        <input id="editCharName" class="input" type="text" bind:value={editName} />
      </div>
      <div class="form-group">
        <label class="label" for="editCharCategory">カテゴリ</label>
        <select id="editCharCategory" class="select" bind:value={editCategory}>
          {#each categories as cat}
            <option value={cat.value}>{cat.label}</option>
          {/each}
        </select>
      </div>
      <div class="form-group">
        <label class="label" for="editCharPrompt">カスタムプロンプト</label>
        <textarea
          id="editCharPrompt"
          class="textarea"
          bind:value={editCustomPrompt}
          rows="3"
        ></textarea>
      </div>
    {:else}
      <div class="info-grid">
        <div class="info-item">
          <span class="info-label">カテゴリ</span>
          <span class="info-value">{categoryLabel}</span>
        </div>
        <div class="info-item">
          <span class="info-label">ステータス</span>
          <span class="info-value">{statusLabel}</span>
        </div>
        <div class="info-item">
          <span class="info-label">スプライト数</span>
          <span class="info-value">{spriteStore.spriteCount}枚</span>
        </div>
        <div class="info-item">
          <span class="info-label">作成日</span>
          <span class="info-value">
            {new Date(character.created_at).toLocaleDateString('ja-JP')}
          </span>
        </div>
        {#if character.custom_prompt}
          <div class="info-item full-width">
            <span class="info-label">カスタムプロンプト</span>
            <span class="info-value">{character.custom_prompt}</span>
          </div>
        {/if}
        {#if character.spritesheet_path}
          <div class="info-item full-width">
            <span class="info-label">スプライトシート</span>
            <span class="info-value path-value">{character.spritesheet_path}</span>
          </div>
        {/if}
      </div>
    {/if}
  </section>

  <!-- Quick Actions -->
  {#if !editing}
    <section class="quick-actions">
      <h3>クイックアクション</h3>
      <div class="action-cards">
        <a href="/character/{characterId}/import" class="action-card">
          <h4>画像インポート</h4>
          <p>MagicaVoxelレンダリング画像を取り込む</p>
        </a>
        <a href="/character/{characterId}/process" class="action-card">
          <h4>AI処理</h4>
          <p>質感生成・背景除去・正規化</p>
        </a>
        <a href="/character/{characterId}/preview" class="action-card">
          <h4>プレビュー</h4>
          <p>スプライトシート確認・エクスポート</p>
        </a>
      </div>
    </section>
  {/if}

  <!-- Delete Confirm Dialog -->
  <ConfirmDialog
    open={showDeleteConfirm}
    title="キャラクターの削除"
    message="「{character.name}」を削除しますか？関連するスプライトもすべて削除されます．この操作は取り消せません．"
    confirmLabel="削除する"
    cancelLabel="キャンセル"
    variant="danger"
    onConfirm={confirmDelete}
    onCancel={() => (showDeleteConfirm = false)}
  />
{/if}

<style>
  .loading-state {
    display: flex;
    flex-direction: column;
    align-items: center;
    justify-content: center;
    gap: var(--space-3);
    padding: var(--space-12);
    color: var(--text-muted);
  }

  :global(.loading-state .spin) {
    animation: spin 1s linear infinite;
  }

  .action-bar {
    display: flex;
    gap: var(--space-2);
    margin-bottom: var(--space-6);
  }

  .character-info {
    margin-bottom: var(--space-8);
  }

  .info-grid {
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

  .info-item.full-width {
    grid-column: 1 / -1;
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

  .quick-actions {
    margin-top: var(--space-6);
  }

  .quick-actions h3 {
    margin-bottom: var(--space-4);
  }

  .action-cards {
    display: grid;
    grid-template-columns: repeat(3, 1fr);
    gap: var(--space-4);
  }

  .action-card {
    display: block;
    padding: var(--space-4);
    background: var(--bg-secondary);
    border: 1px solid var(--border-default);
    border-radius: 8px;
    text-decoration: none;
    transition:
      border-color 200ms ease,
      box-shadow 200ms ease;
  }

  .action-card:hover {
    border-color: var(--accent-primary);
    box-shadow: var(--shadow-md);
    text-decoration: none;
  }

  .action-card h4 {
    color: var(--text-primary);
    margin-bottom: var(--space-1);
  }

  .action-card p {
    color: var(--text-secondary);
    font-size: var(--text-sm);
  }
</style>
