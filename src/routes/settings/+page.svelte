<script lang="ts">
  import { goto } from '$app/navigation';
  import { Settings, Cpu, Download, Sun, Moon, FolderOpen, Package, Cloud, Layers } from 'lucide-svelte';
  import { open } from '@tauri-apps/plugin-dialog';
  import Breadcrumb from '$lib/components/Breadcrumb.svelte';
  import { settingsStore } from '$lib/stores/settings.svelte';

  const breadcrumbItems = [
    { label: 'ホーム', href: '/' },
    { label: '設定' },
  ];

  // Load settings on mount
  $effect(() => {
    if (!settingsStore.loaded) {
      settingsStore.loadSettings();
    }
  });

  const isDark = $derived(settingsStore.theme === 'dark');

  function toggleTheme() {
    settingsStore.setTheme(isDark ? 'light' : 'dark');
  }

  async function selectBevyExportPath() {
    const selected = await open({
      directory: true,
      title: 'デフォルトのBevyエクスポート先を選択',
    });
    if (selected) {
      settingsStore.setBevyExportPath(selected as string);
    }
  }

  const settingsCards = [
    {
      title: 'ComfyUI',
      description: 'ComfyUI接続設定，インストールガイド，接続テスト',
      href: '/settings/comfyui',
      icon: Cpu,
    },
    {
      title: 'モデル管理',
      description: 'ONNXモデル（U2-Net）のダウンロードと初期化',
      href: '/settings/models',
      icon: Download,
    },
    {
      title: 'クラウドAPI',
      description: 'PixelLab / fal.ai / Replicate の API キー設定',
      href: '/settings/cloud-api',
      icon: Cloud,
    },
    {
      title: 'LoRA管理',
      description: 'LoRA モデルのインポートとキャラクター割り当て',
      href: '/settings/lora',
      icon: Layers,
    },
  ];
</script>

<Breadcrumb items={breadcrumbItems} />

<div class="settings-page">
  <h1><Settings size={24} /> 設定</h1>

  <!-- Theme Setting -->
  <section class="settings-section">
    <h2>外観</h2>
    <div class="setting-row">
      <div class="setting-info">
        <h3>テーマ</h3>
        <p class="setting-desc">アプリケーションの配色を切り替えます</p>
      </div>
      <button class="theme-toggle" onclick={toggleTheme} aria-label="テーマ切替">
        {#if isDark}
          <Moon size={18} />
          <span>ダーク</span>
        {:else}
          <Sun size={18} />
          <span>ライト</span>
        {/if}
      </button>
    </div>
  </section>

  <!-- Bevy Export Setting -->
  <section class="settings-section">
    <h2>エクスポート</h2>
    <div class="setting-row">
      <div class="setting-info">
        <h3><Package size={16} /> Bevyエクスポートパス</h3>
        <p class="setting-desc">Bevyプロジェクトへのデフォルト出力先ディレクトリ</p>
        {#if settingsStore.bevyExportPath}
          <p class="setting-current-value">{settingsStore.bevyExportPath}</p>
        {:else}
          <p class="setting-current-value muted">未設定</p>
        {/if}
      </div>
      <button class="btn btn-secondary" onclick={selectBevyExportPath}>
        <FolderOpen size={16} />
        選択
      </button>
    </div>
  </section>

  <!-- Sub-pages -->
  <section class="settings-section">
    <h2>詳細設定</h2>
    <div class="settings-grid">
      {#each settingsCards as card}
        {@const CardIcon = card.icon}
        <button class="settings-card card" onclick={() => goto(card.href)}>
          <div class="card-icon-wrapper">
            <CardIcon size={24} />
          </div>
          <div>
            <h3>{card.title}</h3>
            <p class="card-desc">{card.description}</p>
          </div>
        </button>
      {/each}
    </div>
  </section>
</div>

<style>
  .settings-page h1 {
    display: flex;
    align-items: center;
    gap: var(--space-2);
    margin-bottom: var(--space-6);
  }

  .settings-section {
    margin-bottom: var(--space-8);
  }

  .settings-section h2 {
    font-size: var(--text-lg);
    font-weight: var(--font-weight-semibold);
    margin-bottom: var(--space-4);
    color: var(--text-primary);
  }

  .setting-row {
    display: flex;
    align-items: center;
    justify-content: space-between;
    padding: var(--space-4);
    background: var(--bg-secondary);
    border: 1px solid var(--border-default);
    border-radius: 8px;
    gap: var(--space-4);
  }

  .setting-info {
    flex: 1;
    min-width: 0;
  }

  .setting-info h3 {
    display: flex;
    align-items: center;
    gap: var(--space-2);
    font-size: var(--text-base);
    font-weight: var(--font-weight-medium);
    color: var(--text-primary);
    margin-bottom: var(--space-1);
  }

  .setting-desc {
    font-size: var(--text-sm);
    color: var(--text-secondary);
  }

  .setting-current-value {
    font-size: var(--text-sm);
    font-family: var(--font-mono);
    color: var(--accent-primary);
    margin-top: var(--space-1);
    word-break: break-all;
  }

  .setting-current-value.muted {
    color: var(--text-muted);
    font-family: var(--font-sans);
  }

  .theme-toggle {
    display: flex;
    align-items: center;
    gap: var(--space-2);
    padding: var(--space-2) var(--space-4);
    height: 40px;
    background: var(--bg-primary);
    border: 1px solid var(--border-default);
    border-radius: 6px;
    color: var(--text-primary);
    font-size: var(--text-sm);
    font-weight: var(--font-weight-medium);
    font-family: var(--font-sans);
    cursor: pointer;
    transition: background-color 150ms ease, border-color 150ms ease;
    white-space: nowrap;
  }

  .theme-toggle:hover {
    background: var(--bg-tertiary);
    border-color: var(--accent-primary);
  }

  .settings-grid {
    display: grid;
    grid-template-columns: repeat(auto-fill, minmax(280px, 1fr));
    gap: var(--space-4);
  }

  .settings-card {
    display: flex;
    gap: var(--space-4);
    align-items: flex-start;
    text-align: left;
    width: 100%;
    cursor: pointer;
    font-family: var(--font-sans);
    border: 1px solid var(--border-default);
  }

  .card-icon-wrapper {
    display: flex;
    align-items: center;
    justify-content: center;
    width: 48px;
    height: 48px;
    border-radius: 8px;
    background: var(--bg-tertiary);
    color: var(--accent-primary);
    flex-shrink: 0;
  }

  .settings-card h3 {
    margin-bottom: var(--space-1);
    color: var(--text-primary);
  }

  .card-desc {
    font-size: var(--text-sm);
    color: var(--text-secondary);
  }
</style>
