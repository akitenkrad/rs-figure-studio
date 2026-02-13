<script lang="ts">
  import { page } from '$app/stores';
  import { goto } from '$app/navigation';
  import { Pencil, Trash2, Plus, Users, Loader2, Save, X } from 'lucide-svelte';
  import { projectStore } from '$lib/stores/project.svelte';
  import { characterStore } from '$lib/stores/character.svelte';
  import ConfirmDialog from '$lib/components/ConfirmDialog.svelte';

  const projectId = $derived($page.params.id);

  // Load characters when project ID changes
  $effect(() => {
    if (projectId) {
      characterStore.loadCharacters(projectId);
    }
  });

  // Edit mode
  let editing = $state(false);
  let editName = $state('');
  let editStylePrompt = $state('');
  let editNegativePrompt = $state('');

  // Delete confirm
  let showDeleteConfirm = $state(false);

  function startEdit() {
    const p = projectStore.currentProject;
    if (!p) return;
    editName = p.name;
    editStylePrompt = p.style_prompt;
    editNegativePrompt = p.negative_prompt;
    editing = true;
  }

  function cancelEdit() {
    editing = false;
  }

  async function saveEdit() {
    if (!editName.trim() || !projectId) return;
    await projectStore.updateProject(projectId, {
      name: editName.trim(),
      style_prompt: editStylePrompt.trim(),
      negative_prompt: editNegativePrompt.trim(),
    });
    editing = false;
  }

  async function confirmDelete() {
    if (!projectId) return;
    await projectStore.deleteProject(projectId);
    goto('/');
  }

  function handleAddCharacter() {
    goto(`/character/new?projectId=${projectId}`);
  }
</script>

{#if projectStore.loading && !projectStore.currentProject}
  <div class="loading-state">
    <Loader2 size={32} class="spin" />
    <p>読み込み中...</p>
  </div>
{:else if projectStore.currentProject}
  {@const project = projectStore.currentProject}

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
      <button class="btn btn-danger" onclick={() => showDeleteConfirm = true}>
        <Trash2 size={16} />
        削除
      </button>
    {/if}
  </div>

  <!-- Project Info -->
  <section class="project-info">
    {#if editing}
      <div class="form-group">
        <label class="label" for="editName">プロジェクト名</label>
        <input id="editName" class="input" type="text" bind:value={editName} />
      </div>
      <div class="form-group">
        <label class="label" for="editStylePrompt">スタイルプロンプト</label>
        <textarea id="editStylePrompt" class="textarea" bind:value={editStylePrompt} rows="2"></textarea>
      </div>
      <div class="form-group">
        <label class="label" for="editNegativePrompt">ネガティブプロンプト</label>
        <textarea id="editNegativePrompt" class="textarea" bind:value={editNegativePrompt} rows="2"></textarea>
      </div>
    {:else}
      <div class="info-grid">
        <div class="info-item">
          <span class="info-label">ベースパス</span>
          <span class="info-value">{project.base_path}</span>
        </div>
        <div class="info-item">
          <span class="info-label">タイルサイズ</span>
          <span class="info-value">{project.tile_width} x {project.tile_height} px</span>
        </div>
        <div class="info-item">
          <span class="info-label">ControlNet Weight</span>
          <span class="info-value">{project.controlnet_weight}</span>
        </div>
        {#if project.style_prompt}
          <div class="info-item full-width">
            <span class="info-label">スタイルプロンプト</span>
            <span class="info-value">{project.style_prompt}</span>
          </div>
        {/if}
      </div>
    {/if}
  </section>

  <!-- Character List -->
  <section class="character-section">
    <div class="section-header">
      <h2><Users size={20} /> キャラクター ({characterStore.characterCount})</h2>
      <button class="btn btn-primary" onclick={handleAddCharacter}>
        <Plus size={16} />
        キャラクター追加
      </button>
    </div>

    {#if characterStore.loading && characterStore.characters.length === 0}
      <div class="character-loading">
        <Loader2 size={24} class="spin" />
        <p>キャラクター読み込み中...</p>
      </div>
    {:else if characterStore.characters.length === 0}
      <div class="character-empty">
        <Users size={40} />
        <p>キャラクターがまだありません</p>
        <button class="btn btn-primary" onclick={handleAddCharacter}>
          <Plus size={16} />
          最初のキャラクターを追加
        </button>
      </div>
    {:else}
      <div class="character-grid">
        {#each characterStore.characters as character}
          <button
            class="character-card card"
            onclick={() => goto(`/character/${character.id}`)}
          >
            <div class="card-content">
              <div class="card-title-row">
                <h3 class="card-title">{character.name}</h3>
                <span class="badge badge-{character.status}">
                  {character.status === 'draft'
                    ? '下書き'
                    : character.status === 'importing'
                      ? 'インポート中'
                      : character.status === 'processing'
                        ? '処理中'
                        : '完了'}
                </span>
              </div>
              <div class="card-meta">
                <span class="meta-category">
                  {character.category === 'player'
                    ? 'プレイヤー'
                    : character.category === 'enemy'
                      ? '敵キャラクター'
                      : 'NPC'}
                </span>
                <span class="meta-date">
                  {new Date(character.updated_at).toLocaleDateString('ja-JP')}
                </span>
              </div>
            </div>
          </button>
        {/each}
      </div>
    {/if}
  </section>

  <!-- Delete Confirm Dialog -->
  <ConfirmDialog
    open={showDeleteConfirm}
    title="プロジェクトの削除"
    message="「{project.name}」を削除しますか？この操作は取り消せません．"
    confirmLabel="削除する"
    cancelLabel="キャンセル"
    variant="danger"
    onConfirm={confirmDelete}
    onCancel={() => showDeleteConfirm = false}
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

  .project-info {
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
    word-break: break-all;
  }

  .character-section {
    margin-top: var(--space-6);
  }

  .section-header {
    display: flex;
    align-items: center;
    justify-content: space-between;
    margin-bottom: var(--space-4);
  }

  .section-header h2 {
    display: flex;
    align-items: center;
    gap: var(--space-2);
  }

  .character-loading {
    display: flex;
    align-items: center;
    justify-content: center;
    gap: var(--space-2);
    padding: var(--space-8);
    color: var(--text-muted);
    font-size: var(--text-sm);
  }

  .character-empty {
    display: flex;
    flex-direction: column;
    align-items: center;
    justify-content: center;
    gap: var(--space-3);
    padding: var(--space-8);
    text-align: center;
    background: var(--bg-secondary);
    border-radius: 8px;
    border: 1px dashed var(--border-default);
    color: var(--text-muted);
  }

  .character-empty p {
    font-size: var(--text-sm);
  }

  .character-grid {
    display: grid;
    grid-template-columns: repeat(auto-fill, minmax(280px, 1fr));
    gap: var(--space-4);
  }

  .character-card {
    display: flex;
    text-align: left;
    width: 100%;
    cursor: pointer;
    font-family: var(--font-sans);
  }

  .character-card .card-content {
    flex: 1;
    min-width: 0;
  }

  .card-title-row {
    display: flex;
    align-items: center;
    justify-content: space-between;
    margin-bottom: var(--space-2);
  }

  .character-card .card-title {
    font-size: var(--text-base);
    font-weight: var(--font-weight-semibold);
    color: var(--text-primary);
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .character-card .card-meta {
    display: flex;
    align-items: center;
    gap: var(--space-3);
    font-size: var(--text-sm);
    color: var(--text-secondary);
  }

  .meta-category {
    font-size: var(--text-xs);
    color: var(--text-muted);
  }

  .meta-date {
    font-size: var(--text-xs);
    color: var(--text-muted);
  }
</style>
