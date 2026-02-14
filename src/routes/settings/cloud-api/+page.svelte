<script lang="ts">
  import {
    Cloud,
    Check,
    AlertTriangle,
    Loader2,
    Eye,
    EyeOff,
  } from 'lucide-svelte';
  import Breadcrumb from '$lib/components/Breadcrumb.svelte';
  import { generationApi, settingsApi } from '$lib/api/tauri';
  import { toastStore } from '$lib/stores/toast.svelte';
  import type { CloudApiProvider } from '$lib/types';

  const breadcrumbItems = [
    { label: 'ホーム', href: '/' },
    { label: '設定', href: '/settings' },
    { label: 'クラウドAPI' },
  ];

  let provider = $state<CloudApiProvider>('pixellab');
  let pixellabKey = $state('');
  let falAiKey = $state('');
  let replicateKey = $state('');
  let generationBackend = $state<string>('comfyui');
  let showKeys = $state<Record<string, boolean>>({});
  let testing = $state<string | null>(null);
  let testResults = $state<Record<string, boolean | null>>({});

  // Load saved settings
  $effect(() => {
    settingsApi.getAll().then((settings) => {
      provider = (settings['cloud_api_provider'] as CloudApiProvider) || 'pixellab';
      pixellabKey = settings['pixellab_api_key'] || '';
      falAiKey = settings['fal_ai_api_key'] || '';
      replicateKey = settings['replicate_api_key'] || '';
      generationBackend = settings['generation_backend'] || 'comfyui';
    });
  });

  async function saveProvider() {
    await settingsApi.update('cloud_api_provider', provider);
    toastStore.addToast('success', 'プロバイダを保存しました');
  }

  async function saveKey(key: string, value: string) {
    await settingsApi.update(key, value);
    toastStore.addToast('success', 'APIキーを保存しました');
  }

  async function testConnection(providerName: string, apiKey: string) {
    testing = providerName;
    testResults[providerName] = null;
    try {
      const result = await generationApi.testCloudApiConnection(providerName, apiKey);
      testResults[providerName] = result;
      toastStore.addToast(
        result ? 'success' : 'error',
        result ? `${providerName} に接続成功` : `${providerName} に接続失敗`,
      );
    } catch (e) {
      testResults[providerName] = false;
      toastStore.addToast('error', `接続テストに失敗: ${e}`);
    } finally {
      testing = null;
    }
  }

  function toggleShow(key: string) {
    showKeys[key] = !showKeys[key];
  }

  const providers: { id: CloudApiProvider; label: string; description: string }[] = [
    { id: 'pixellab', label: 'PixelLab', description: 'ピクセルアート特化の生成API' },
    { id: 'fal_ai', label: 'fal.ai', description: 'FLUX.2 マルチリファレンス生成' },
    { id: 'replicate', label: 'Replicate', description: 'Retro Diffusion ピクセルアート' },
  ];
</script>

<Breadcrumb items={breadcrumbItems} />

<div class="cloud-api-page">
  <h1><Cloud size={24} /> クラウドAPI設定</h1>
  <p class="page-desc">
    GPU を持たない環境でもクラウド API 経由で AI キャラクター生成が利用できます．
    使用するプロバイダの API キーを設定してください．
  </p>

  <!-- Provider Selection -->
  <section class="settings-section">
    <h2>デフォルトプロバイダ</h2>
    <div class="provider-grid">
      {#each providers as p}
        <button
          class="provider-card"
          class:selected={provider === p.id}
          onclick={() => { provider = p.id; saveProvider(); }}
        >
          <div class="provider-name">{p.label}</div>
          <div class="provider-desc">{p.description}</div>
          {#if provider === p.id}
            <div class="provider-check"><Check size={14} /></div>
          {/if}
        </button>
      {/each}
    </div>
  </section>

  <!-- API Keys -->
  <section class="settings-section">
    <h2>APIキー</h2>

    <!-- PixelLab -->
    <div class="key-row">
      <div class="key-header">
        <h3>PixelLab</h3>
        {#if testResults['pixellab'] === true}
          <span class="test-badge success"><Check size={12} /> 接続済み</span>
        {:else if testResults['pixellab'] === false}
          <span class="test-badge error"><AlertTriangle size={12} /> 失敗</span>
        {/if}
      </div>
      <div class="key-input-row">
        <div class="input-wrapper">
          <input
            type={showKeys['pixellab'] ? 'text' : 'password'}
            class="input"
            placeholder="pk-..."
            bind:value={pixellabKey}
            onblur={() => saveKey('pixellab_api_key', pixellabKey)}
          />
          <button class="btn-icon" onclick={() => toggleShow('pixellab')}>
            {#if showKeys['pixellab']}<EyeOff size={14} />{:else}<Eye size={14} />{/if}
          </button>
        </div>
        <button
          class="btn btn-secondary btn-sm"
          onclick={() => testConnection('pixellab', pixellabKey)}
          disabled={!pixellabKey || testing !== null}
        >
          {#if testing === 'pixellab'}<Loader2 size={14} class="spin" />{:else}テスト{/if}
        </button>
      </div>
    </div>

    <!-- fal.ai -->
    <div class="key-row">
      <div class="key-header">
        <h3>fal.ai</h3>
        {#if testResults['fal_ai'] === true}
          <span class="test-badge success"><Check size={12} /> 接続済み</span>
        {:else if testResults['fal_ai'] === false}
          <span class="test-badge error"><AlertTriangle size={12} /> 失敗</span>
        {/if}
      </div>
      <div class="key-input-row">
        <div class="input-wrapper">
          <input
            type={showKeys['fal_ai'] ? 'text' : 'password'}
            class="input"
            placeholder="fal-..."
            bind:value={falAiKey}
            onblur={() => saveKey('fal_ai_api_key', falAiKey)}
          />
          <button class="btn-icon" onclick={() => toggleShow('fal_ai')}>
            {#if showKeys['fal_ai']}<EyeOff size={14} />{:else}<Eye size={14} />{/if}
          </button>
        </div>
        <button
          class="btn btn-secondary btn-sm"
          onclick={() => testConnection('fal_ai', falAiKey)}
          disabled={!falAiKey || testing !== null}
        >
          {#if testing === 'fal_ai'}<Loader2 size={14} class="spin" />{:else}テスト{/if}
        </button>
      </div>
    </div>

    <!-- Replicate -->
    <div class="key-row">
      <div class="key-header">
        <h3>Replicate</h3>
        {#if testResults['replicate'] === true}
          <span class="test-badge success"><Check size={12} /> 接続済み</span>
        {:else if testResults['replicate'] === false}
          <span class="test-badge error"><AlertTriangle size={12} /> 失敗</span>
        {/if}
      </div>
      <div class="key-input-row">
        <div class="input-wrapper">
          <input
            type={showKeys['replicate'] ? 'text' : 'password'}
            class="input"
            placeholder="r8_..."
            bind:value={replicateKey}
            onblur={() => saveKey('replicate_api_key', replicateKey)}
          />
          <button class="btn-icon" onclick={() => toggleShow('replicate')}>
            {#if showKeys['replicate']}<EyeOff size={14} />{:else}<Eye size={14} />{/if}
          </button>
        </div>
        <button
          class="btn btn-secondary btn-sm"
          onclick={() => testConnection('replicate', replicateKey)}
          disabled={!replicateKey || testing !== null}
        >
          {#if testing === 'replicate'}<Loader2 size={14} class="spin" />{:else}テスト{/if}
        </button>
      </div>
    </div>
  </section>

  <!-- Backend Selection -->
  <section class="settings-section">
    <h2>生成バックエンド</h2>
    <p class="section-desc">
      AI キャラクター生成で使用するバックエンドを選択します．
      ComfyUI（ローカル GPU）またはクラウド API から選択できます．
    </p>
    <div class="backend-selector">
      <label class="backend-option">
        <input
          type="radio"
          name="backend"
          value="comfyui"
          checked={generationBackend === 'comfyui'}
          onchange={() => { generationBackend = 'comfyui'; settingsApi.update('generation_backend', 'comfyui'); }}
        />
        <div class="backend-label">
          <strong>ComfyUI（ローカル）</strong>
          <span>ローカル GPU で生成．高速だが GPU が必要</span>
        </div>
      </label>
      <label class="backend-option">
        <input
          type="radio"
          name="backend"
          value="cloud_api"
          checked={generationBackend === 'cloud_api'}
          onchange={() => { generationBackend = 'cloud_api'; settingsApi.update('generation_backend', 'cloud_api'); }}
        />
        <div class="backend-label">
          <strong>クラウドAPI</strong>
          <span>外部 API 経由で生成．GPU 不要だが API キーと課金が必要</span>
        </div>
      </label>
    </div>
  </section>
</div>

<style>
  .cloud-api-page h1 {
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

  .settings-section h2 {
    font-size: var(--text-lg);
    margin-bottom: var(--space-4);
  }

  .section-desc {
    font-size: var(--text-sm);
    color: var(--text-secondary);
    margin-bottom: var(--space-4);
  }

  .provider-grid {
    display: grid;
    grid-template-columns: repeat(3, 1fr);
    gap: var(--space-3);
  }

  .provider-card {
    position: relative;
    padding: var(--space-4);
    background: var(--bg-secondary);
    border: 2px solid var(--border-default);
    border-radius: 8px;
    cursor: pointer;
    text-align: left;
    font-family: var(--font-sans);
    transition: border-color 150ms ease;
  }

  .provider-card:hover {
    border-color: var(--accent-primary);
  }

  .provider-card.selected {
    border-color: var(--accent-primary);
    background: hsla(var(--accent-hue, 220), 60%, 50%, 0.05);
  }

  .provider-name {
    font-weight: var(--font-weight-semibold);
    margin-bottom: var(--space-1);
  }

  .provider-desc {
    font-size: var(--text-xs);
    color: var(--text-secondary);
  }

  .provider-check {
    position: absolute;
    top: var(--space-2);
    right: var(--space-2);
    color: var(--accent-primary);
  }

  .key-row {
    padding: var(--space-4);
    background: var(--bg-secondary);
    border: 1px solid var(--border-default);
    border-radius: 8px;
    margin-bottom: var(--space-3);
  }

  .key-header {
    display: flex;
    align-items: center;
    gap: var(--space-2);
    margin-bottom: var(--space-2);
  }

  .key-header h3 {
    font-size: var(--text-base);
    font-weight: var(--font-weight-medium);
  }

  .test-badge {
    display: flex;
    align-items: center;
    gap: 4px;
    font-size: var(--text-xs);
    padding: 2px 8px;
    border-radius: 4px;
  }

  .test-badge.success {
    background: hsla(142, 60%, 45%, 0.1);
    color: var(--accent-success);
  }

  .test-badge.error {
    background: hsla(0, 70%, 55%, 0.1);
    color: var(--accent-danger);
  }

  .key-input-row {
    display: flex;
    gap: var(--space-2);
    align-items: center;
  }

  .input-wrapper {
    flex: 1;
    position: relative;
    display: flex;
    align-items: center;
  }

  .input-wrapper .input {
    flex: 1;
    padding-right: 36px;
  }

  .btn-icon {
    position: absolute;
    right: 8px;
    background: none;
    border: none;
    color: var(--text-muted);
    cursor: pointer;
    padding: 4px;
  }

  .btn-sm {
    height: 36px;
    padding: 0 var(--space-3);
    font-size: var(--text-sm);
    white-space: nowrap;
  }

  .backend-selector {
    display: flex;
    flex-direction: column;
    gap: var(--space-3);
  }

  .backend-option {
    display: flex;
    align-items: flex-start;
    gap: var(--space-3);
    padding: var(--space-4);
    background: var(--bg-secondary);
    border: 1px solid var(--border-default);
    border-radius: 8px;
    cursor: pointer;
  }

  .backend-option:hover {
    border-color: var(--accent-primary);
  }

  .backend-label {
    display: flex;
    flex-direction: column;
    gap: var(--space-1);
  }

  .backend-label span {
    font-size: var(--text-sm);
    color: var(--text-secondary);
  }
</style>
