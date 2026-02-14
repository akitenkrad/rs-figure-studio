<script lang="ts">
  import {
    Layers,
    Upload,
    Trash2,
    FileText,
  } from 'lucide-svelte';
  import { open } from '@tauri-apps/plugin-dialog';
  import Breadcrumb from '$lib/components/Breadcrumb.svelte';
  import { loraApi } from '$lib/api/tauri';
  import { toastStore } from '$lib/stores/toast.svelte';
  import type { LoraModel } from '$lib/types';

  const breadcrumbItems = [
    { label: 'ホーム', href: '/' },
    { label: '設定', href: '/settings' },
    { label: 'LoRA管理' },
  ];

  let loras = $state<LoraModel[]>([]);
  let importing = $state(false);
  let deletingId = $state<string | null>(null);

  // Load LoRA models
  $effect(() => {
    loadLoras();
  });

  async function loadLoras() {
    try {
      loras = await loraApi.list();
    } catch (e) {
      toastStore.addToast('error', `LoRA一覧の取得に失敗: ${e}`);
    }
  }

  async function importLora() {
    const selected = await open({
      multiple: false,
      filters: [{ name: 'LoRA Models', extensions: ['safetensors', 'ckpt', 'pt'] }],
      title: 'LoRA モデルを選択',
    });
    if (!selected) return;

    const filePath = selected as string;
    const fileName = filePath.split('/').pop()?.replace(/\.[^.]+$/, '') || 'Unnamed';

    importing = true;
    try {
      await loraApi.import(fileName, filePath);
      toastStore.addToast('success', 'LoRA モデルをインポートしました');
      await loadLoras();
    } catch (e) {
      toastStore.addToast('error', `インポートに失敗: ${e}`);
    } finally {
      importing = false;
    }
  }

  async function deleteLora(id: string, name: string) {
    deletingId = id;
    try {
      await loraApi.delete(id);
      toastStore.addToast('success', `${name} を削除しました`);
      await loadLoras();
    } catch (e) {
      toastStore.addToast('error', `削除に失敗: ${e}`);
    } finally {
      deletingId = null;
    }
  }
</script>

<Breadcrumb items={breadcrumbItems} />

<div class="lora-page">
  <h1><Layers size={24} /> LoRA管理</h1>
  <p class="page-desc">
    LoRA モデルをインポートして管理します．
    キャラクターへの割り当ては各キャラクター詳細ページから行います．
  </p>

  <!-- Import -->
  <section class="settings-section">
    <div class="section-header">
      <h2>インポート済みモデル</h2>
      <button
        class="btn btn-primary"
        onclick={importLora}
        disabled={importing}
      >
        <Upload size={16} />
        {importing ? 'インポート中...' : 'LoRAをインポート'}
      </button>
    </div>

    {#if loras.length === 0}
      <div class="empty-state">
        <FileText size={48} />
        <p>LoRA モデルがまだインポートされていません</p>
        <p class="empty-hint">
          .safetensors / .ckpt / .pt ファイルをインポートしてください
        </p>
      </div>
    {:else}
      <div class="lora-list">
        {#each loras as lora}
          <div class="lora-item">
            <div class="lora-info">
              <h3>{lora.name}</h3>
              {#if lora.description}
                <p class="lora-desc">{lora.description}</p>
              {/if}
              <p class="lora-path">{lora.file_path}</p>
            </div>
            <button
              class="btn btn-danger btn-sm"
              onclick={() => deleteLora(lora.id, lora.name)}
              disabled={deletingId === lora.id}
            >
              <Trash2 size={14} />
              {deletingId === lora.id ? '削除中...' : '削除'}
            </button>
          </div>
        {/each}
      </div>
    {/if}
  </section>
</div>

<style>
  .lora-page h1 {
    display: flex;
    align-items: center;
    gap: var(--space-2);
    margin-bottom: var(--space-2);
  }

  .page-desc {
    font-size: var(--text-sm);
    color: var(--text-secondary);
    margin-bottom: var(--space-6);
    line-height: 1.6;
  }

  .settings-section {
    margin-bottom: var(--space-8);
  }

  .section-header {
    display: flex;
    align-items: center;
    justify-content: space-between;
    margin-bottom: var(--space-4);
  }

  .section-header h2 {
    font-size: var(--text-lg);
  }

  .empty-state {
    display: flex;
    flex-direction: column;
    align-items: center;
    justify-content: center;
    padding: var(--space-12) var(--space-4);
    background: var(--bg-secondary);
    border: 2px dashed var(--border-default);
    border-radius: 8px;
    color: var(--text-muted);
    text-align: center;
    gap: var(--space-2);
  }

  .empty-hint {
    font-size: var(--text-sm);
  }

  .lora-list {
    display: flex;
    flex-direction: column;
    gap: var(--space-3);
  }

  .lora-item {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: var(--space-4);
    padding: var(--space-4);
    background: var(--bg-secondary);
    border: 1px solid var(--border-default);
    border-radius: 8px;
  }

  .lora-info {
    flex: 1;
    min-width: 0;
  }

  .lora-info h3 {
    font-size: var(--text-base);
    font-weight: var(--font-weight-medium);
    margin-bottom: var(--space-1);
  }

  .lora-desc {
    font-size: var(--text-sm);
    color: var(--text-secondary);
    margin-bottom: var(--space-1);
  }

  .lora-path {
    font-size: var(--text-xs);
    font-family: var(--font-mono);
    color: var(--text-muted);
    word-break: break-all;
  }

  .btn-danger {
    background: hsla(0, 70%, 55%, 0.1);
    color: var(--accent-danger);
    border: 1px solid hsla(0, 70%, 55%, 0.2);
    display: flex;
    align-items: center;
    gap: var(--space-1);
    white-space: nowrap;
  }

  .btn-danger:hover {
    background: hsla(0, 70%, 55%, 0.2);
  }

  .btn-sm {
    height: 36px;
    padding: 0 var(--space-3);
    font-size: var(--text-sm);
  }
</style>
