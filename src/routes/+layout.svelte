<script lang="ts">
  import '../app.css';
  import Sidebar from '$lib/components/Sidebar.svelte';
  import Toast from '$lib/components/Toast.svelte';
  import SetupWizard from '$lib/components/SetupWizard.svelte';
  import { toastStore } from '$lib/stores/toast.svelte';
  import { setupStore } from '$lib/stores/setup.svelte';
  import { settingsStore } from '$lib/stores/settings.svelte';

  let { children } = $props();
  let sidebarCollapsed = $state(false);

  // Load settings (theme etc.) on app startup
  $effect(() => {
    settingsStore.loadSettings();
  });

  // Check if first-time setup is needed on app startup
  $effect(() => {
    setupStore.checkSetupRequired();
  });
</script>

<div class="app-layout" class:sidebar-collapsed={sidebarCollapsed}>
  <Sidebar
    collapsed={sidebarCollapsed}
    onToggle={() => sidebarCollapsed = !sidebarCollapsed}
  />
  <main class="main-content">
    {@render children()}
  </main>
  <Toast toasts={toastStore.toasts} onRemove={(id) => toastStore.removeToast(id)} />
</div>

<!-- Setup wizard overlay (shown on first launch) -->
<SetupWizard />

<style>
  .app-layout {
    display: grid;
    grid-template-columns: 240px 1fr;
    height: 100vh;
    overflow: hidden;
  }

  .app-layout.sidebar-collapsed {
    grid-template-columns: 60px 1fr;
  }

  .main-content {
    overflow-y: auto;
    padding: var(--space-6);
    background: var(--bg-primary);
  }
</style>
