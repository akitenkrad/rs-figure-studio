<script lang="ts">
  import Breadcrumb from '$lib/components/Breadcrumb.svelte';
  import ComfyUIGuide from '$lib/components/ComfyUIGuide.svelte';
  import WorkflowManager from '$lib/components/WorkflowManager.svelte';
  import { comfyuiStore } from '$lib/stores/comfyui.svelte';

  const breadcrumbItems = [
    { label: 'ホーム', href: '/' },
    { label: '設定', href: '/settings' },
    { label: 'ComfyUI' },
  ];

  // Load saved endpoint on mount
  $effect(() => {
    comfyuiStore.loadEndpoint();
  });

  function handleEndpointChange(newEndpoint: string) {
    comfyuiStore.setEndpoint(newEndpoint);
  }

  async function handleConnectionTest(): Promise<boolean> {
    return comfyuiStore.checkConnection();
  }
</script>

<Breadcrumb items={breadcrumbItems} />

<div class="settings-page">
  <h1>ComfyUI 設定</h1>
  <p class="page-description">
    ComfyUI のインストール，起動方法，接続テストを行います．
    ComfyUI はAI質感生成（ControlNet img2img）に使用されます．
  </p>

  <section class="guide-section">
    <ComfyUIGuide
      endpoint={comfyuiStore.endpoint}
      connected={comfyuiStore.isConnected}
      onEndpointChange={handleEndpointChange}
      onConnectionTest={handleConnectionTest}
    />
  </section>

  <section class="workflow-section">
    <WorkflowManager />
  </section>
</div>

<style>
  .settings-page {
    display: flex;
    flex-direction: column;
    gap: var(--space-6);
  }

  .settings-page h1 {
    margin-bottom: 0;
  }

  .page-description {
    color: var(--text-secondary);
    font-size: var(--text-sm);
    line-height: var(--leading-relaxed);
  }

  .guide-section {
    max-width: 800px;
  }

  .workflow-section {
    max-width: 800px;
  }
</style>
