<script lang="ts">
  import {
    FileJson,
    Trash2,
    Star,
    ChevronDown,
    ChevronRight,
    Upload,
    Loader2,
    AlertTriangle,
  } from 'lucide-svelte';
  import { comfyuiStore } from '$lib/stores/comfyui.svelte';
  import { toastStore } from '$lib/stores/toast.svelte';

  // --- State ---
  let expandedWorkflowId = $state<string | null>(null);
  let deleteConfirmId = $state<string | null>(null);
  let importing = $state(false);

  // Load workflows on mount
  $effect(() => {
    comfyuiStore.loadWorkflows();
  });

  // --- Handlers ---
  function toggleExpand(id: string) {
    expandedWorkflowId = expandedWorkflowId === id ? null : id;
  }

  async function handleImport() {
    // Use a hidden file input for JSON import
    const input = document.createElement('input');
    input.type = 'file';
    input.accept = '.json';
    input.onchange = async () => {
      const file = input.files?.[0];
      if (!file) return;

      importing = true;
      try {
        const text = await file.text();
        // Validate JSON
        JSON.parse(text);

        const name = file.name.replace(/\.json$/i, '');
        await comfyuiStore.importWorkflow(name, text);
      } catch (e) {
        if (e instanceof SyntaxError) {
          toastStore.addToast('error', '無効なJSONファイルです');
        } else {
          toastStore.addToast('error', `インポートに失敗しました: ${e}`);
        }
      } finally {
        importing = false;
      }
    };
    input.click();
  }

  async function handleSetDefault(id: string) {
    await comfyuiStore.setDefaultWorkflow(id);
  }

  async function handleDelete(id: string) {
    if (deleteConfirmId === id) {
      await comfyuiStore.deleteWorkflow(id);
      deleteConfirmId = null;
    } else {
      deleteConfirmId = id;
      // Auto-cancel confirm after 3 seconds
      setTimeout(() => {
        deleteConfirmId = null;
      }, 3000);
    }
  }

  function formatJson(json: string): string {
    try {
      return JSON.stringify(JSON.parse(json), null, 2);
    } catch {
      return json;
    }
  }

  function formatDate(dateStr: string): string {
    return new Date(dateStr).toLocaleDateString('ja-JP', {
      year: 'numeric',
      month: '2-digit',
      day: '2-digit',
      hour: '2-digit',
      minute: '2-digit',
    });
  }
</script>

<div class="workflow-manager">
  <header class="manager-header">
    <h3>ワークフロー管理</h3>
    <button
      class="btn btn-secondary"
      onclick={handleImport}
      disabled={importing}
    >
      {#if importing}
        <Loader2 size={16} class="spin" />
        インポート中...
      {:else}
        <Upload size={16} />
        JSONインポート
      {/if}
    </button>
  </header>

  {#if comfyuiStore.loading}
    <div class="loading-state">
      <Loader2 size={16} class="spin" />
      <span>ワークフローを読み込んでいます...</span>
    </div>
  {:else if comfyuiStore.workflows.length === 0}
    <div class="empty-state">
      <FileJson size={32} />
      <p>ワークフローがありません</p>
      <p class="empty-hint">
        ComfyUI のワークフローJSONファイルをインポートして管理できます．
      </p>
    </div>
  {:else}
    <div class="workflow-list">
      {#each comfyuiStore.workflows as workflow (workflow.id)}
        <div class="workflow-item" class:expanded={expandedWorkflowId === workflow.id}>
          <!-- Workflow Header -->
          <div class="workflow-header">
            <button
              class="expand-toggle"
              onclick={() => toggleExpand(workflow.id)}
              aria-label={expandedWorkflowId === workflow.id ? '折りたたむ' : '展開する'}
            >
              {#if expandedWorkflowId === workflow.id}
                <ChevronDown size={16} />
              {:else}
                <ChevronRight size={16} />
              {/if}
            </button>

            <div class="workflow-info">
              <div class="workflow-name-row">
                <FileJson size={16} />
                <span class="workflow-name">{workflow.name}</span>
                {#if workflow.is_default}
                  <span class="badge badge-complete">
                    <Star size={10} />
                    デフォルト
                  </span>
                {/if}
              </div>
              {#if workflow.description}
                <span class="workflow-description">{workflow.description}</span>
              {/if}
              <span class="workflow-date">
                更新: {formatDate(workflow.updated_at)}
              </span>
            </div>

            <div class="workflow-actions">
              {#if !workflow.is_default}
                <button
                  class="btn btn-ghost btn-sm"
                  onclick={() => handleSetDefault(workflow.id)}
                  title="デフォルトに設定"
                >
                  <Star size={14} />
                </button>
              {/if}
              <button
                class="btn btn-sm"
                class:btn-danger={deleteConfirmId === workflow.id}
                class:btn-ghost={deleteConfirmId !== workflow.id}
                onclick={() => handleDelete(workflow.id)}
                title={deleteConfirmId === workflow.id ? 'もう一度押して削除' : '削除'}
              >
                {#if deleteConfirmId === workflow.id}
                  <AlertTriangle size={14} />
                  確認
                {:else}
                  <Trash2 size={14} />
                {/if}
              </button>
            </div>
          </div>

          <!-- Workflow JSON Preview (expandable) -->
          {#if expandedWorkflowId === workflow.id}
            <div class="workflow-preview">
              <div class="code-block">
                <pre><code>{formatJson(workflow.workflow_json)}</code></pre>
              </div>
            </div>
          {/if}
        </div>
      {/each}
    </div>
  {/if}
</div>

<style>
  .workflow-manager {
    display: flex;
    flex-direction: column;
    gap: var(--space-4);
  }

  .manager-header {
    display: flex;
    align-items: center;
    justify-content: space-between;
  }

  .manager-header h3 {
    margin: 0;
  }

  /* Loading & Empty States */
  .loading-state {
    display: flex;
    align-items: center;
    gap: var(--space-2);
    padding: var(--space-4);
    background: var(--bg-secondary);
    border-radius: 8px;
    color: var(--text-secondary);
    font-size: var(--text-sm);
  }

  .empty-state {
    display: flex;
    flex-direction: column;
    align-items: center;
    gap: var(--space-3);
    padding: var(--space-8);
    background: var(--bg-secondary);
    border: 1px dashed var(--border-default);
    border-radius: 8px;
    text-align: center;
    color: var(--text-muted);
  }

  .empty-hint {
    font-size: var(--text-sm);
    max-width: 360px;
  }

  /* Workflow List */
  .workflow-list {
    display: flex;
    flex-direction: column;
    gap: var(--space-2);
  }

  .workflow-item {
    background: var(--bg-secondary);
    border: 1px solid var(--border-default);
    border-radius: 8px;
    overflow: hidden;
    transition: border-color 150ms ease;
  }

  .workflow-item:hover {
    border-color: var(--accent-primary);
  }

  .workflow-item.expanded {
    border-color: var(--accent-primary);
  }

  .workflow-header {
    display: flex;
    align-items: center;
    gap: var(--space-3);
    padding: var(--space-3) var(--space-4);
  }

  .expand-toggle {
    display: flex;
    align-items: center;
    justify-content: center;
    width: 28px;
    height: 28px;
    background: transparent;
    border: none;
    border-radius: 4px;
    color: var(--text-muted);
    cursor: pointer;
    flex-shrink: 0;
    transition:
      background-color 150ms ease,
      color 150ms ease;
  }

  .expand-toggle:hover {
    background: var(--bg-tertiary);
    color: var(--text-primary);
  }

  .workflow-info {
    flex: 1;
    min-width: 0;
    display: flex;
    flex-direction: column;
    gap: 2px;
  }

  .workflow-name-row {
    display: flex;
    align-items: center;
    gap: var(--space-2);
  }

  .workflow-name {
    font-size: var(--text-sm);
    font-weight: var(--font-weight-medium);
    color: var(--text-primary);
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .workflow-description {
    font-size: var(--text-xs);
    color: var(--text-secondary);
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .workflow-date {
    font-size: var(--text-xs);
    color: var(--text-muted);
  }

  .workflow-actions {
    display: flex;
    gap: var(--space-1);
    flex-shrink: 0;
  }

  .btn-sm {
    height: 28px;
    padding: 0 var(--space-2);
    font-size: var(--text-xs);
  }

  /* JSON Preview */
  .workflow-preview {
    border-top: 1px solid var(--border-default);
    padding: var(--space-4);
    background: var(--bg-primary);
  }

  .code-block {
    max-height: 400px;
    overflow: auto;
    border-radius: 6px;
  }

  .code-block pre {
    margin: 0;
  }

  .code-block code {
    font-family: var(--font-mono);
    font-size: var(--text-xs);
    color: var(--text-secondary);
    line-height: var(--leading-relaxed);
    white-space: pre;
  }
</style>
