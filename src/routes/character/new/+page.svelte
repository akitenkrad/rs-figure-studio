<script lang="ts">
  import { page } from '$app/stores';
  import { goto } from '$app/navigation';
  import Breadcrumb from '$lib/components/Breadcrumb.svelte';
  import { characterStore } from '$lib/stores/character.svelte';
  import { projectStore } from '$lib/stores/project.svelte';
  import { Plus, ArrowLeft, Loader2 } from 'lucide-svelte';
  import type { CharacterCategory } from '$lib/types';

  const projectId = $derived($page.url.searchParams.get('projectId') ?? '');

  // Load project info for breadcrumb
  $effect(() => {
    if (projectId) {
      projectStore.loadProject(projectId);
    }
  });

  const breadcrumbItems = $derived([
    { label: 'ホーム', href: '/' },
    {
      label: projectStore.currentProject?.name ?? 'プロジェクト',
      href: projectId ? `/project/${projectId}` : '/',
    },
    { label: '新規キャラクター' },
  ]);

  // Form state
  let name = $state('');
  let category = $state<CharacterCategory>('player');
  let customPrompt = $state('');
  let submitting = $state(false);
  let nameError = $state('');

  const categories: { value: CharacterCategory; label: string }[] = [
    { value: 'player', label: 'プレイヤー' },
    { value: 'enemy', label: '敵キャラクター' },
    { value: 'npc', label: 'NPC' },
  ];

  function validate(): boolean {
    nameError = '';
    if (!name.trim()) {
      nameError = 'キャラクター名を入力してください';
      return false;
    }
    if (name.trim().length > 50) {
      nameError = 'キャラクター名は50文字以内で入力してください';
      return false;
    }
    return true;
  }

  async function handleSubmit() {
    if (!validate() || !projectId) return;

    submitting = true;
    const character = await characterStore.createCharacter({
      project_id: projectId,
      name: name.trim(),
      category,
      custom_prompt: customPrompt.trim() || undefined,
    });

    submitting = false;

    if (character) {
      goto(`/character/${character.id}`);
    }
  }

  function handleCancel() {
    if (projectId) {
      goto(`/project/${projectId}`);
    } else {
      goto('/');
    }
  }
</script>

<Breadcrumb items={breadcrumbItems} />

<div class="new-character-page">
  <h1>新規キャラクター作成</h1>

  {#if !projectId}
    <div class="error-notice">
      <p>プロジェクトが指定されていません．プロジェクト詳細ページから作成してください．</p>
      <button class="btn btn-primary" onclick={() => goto('/')}>
        <ArrowLeft size={16} />
        ダッシュボードへ戻る
      </button>
    </div>
  {:else}
    <form
      class="character-form"
      onsubmit={(e) => {
        e.preventDefault();
        handleSubmit();
      }}
    >
      <!-- Name -->
      <div class="form-group">
        <label class="label" for="charName">キャラクター名 <span class="required">*</span></label>
        <input
          id="charName"
          class="input"
          class:input-error={nameError}
          type="text"
          bind:value={name}
          placeholder="例: 勇者，スライム，村人A"
          disabled={submitting}
        />
        {#if nameError}
          <p class="field-error">{nameError}</p>
        {/if}
      </div>

      <!-- Category -->
      <div class="form-group">
        <label class="label" for="charCategory">カテゴリ <span class="required">*</span></label>
        <select id="charCategory" class="select" bind:value={category} disabled={submitting}>
          {#each categories as cat}
            <option value={cat.value}>{cat.label}</option>
          {/each}
        </select>
      </div>

      <!-- Custom Prompt -->
      <div class="form-group">
        <label class="label" for="charPrompt">カスタムプロンプト</label>
        <textarea
          id="charPrompt"
          class="textarea"
          bind:value={customPrompt}
          placeholder="AI質感生成時に使用する追加プロンプト（省略可）"
          rows="3"
          disabled={submitting}
        ></textarea>
        <p class="field-hint">
          AI質感生成（ComfyUI）でこのキャラクター固有のプロンプトを追加できます．
        </p>
      </div>

      <!-- Actions -->
      <div class="form-actions">
        <button
          type="button"
          class="btn btn-secondary"
          onclick={handleCancel}
          disabled={submitting}
        >
          キャンセル
        </button>
        <button type="submit" class="btn btn-primary" disabled={submitting || !name.trim()}>
          {#if submitting}
            <Loader2 size={16} class="spin" />
            作成中...
          {:else}
            <Plus size={16} />
            作成
          {/if}
        </button>
      </div>
    </form>
  {/if}
</div>

<style>
  .new-character-page {
    max-width: 640px;
  }

  .new-character-page h1 {
    margin-bottom: var(--space-6);
  }

  .error-notice {
    padding: var(--space-6);
    background: var(--bg-secondary);
    border-radius: 8px;
    border: 1px dashed var(--border-default);
    text-align: center;
    display: flex;
    flex-direction: column;
    align-items: center;
    gap: var(--space-4);
    color: var(--text-muted);
  }

  .character-form {
    display: flex;
    flex-direction: column;
    gap: var(--space-4);
  }

  .required {
    color: var(--accent-danger);
  }

  .input-error {
    border-color: var(--accent-danger);
  }

  .field-error {
    margin-top: var(--space-1);
    font-size: var(--text-xs);
    color: var(--accent-danger);
  }

  .field-hint {
    margin-top: var(--space-1);
    font-size: var(--text-xs);
    color: var(--text-muted);
  }

  .form-actions {
    display: flex;
    justify-content: flex-end;
    gap: var(--space-3);
    margin-top: var(--space-4);
    padding-top: var(--space-4);
    border-top: 1px solid var(--border-default);
  }

  :global(.spin) {
    animation: spin 1s linear infinite;
  }
</style>
