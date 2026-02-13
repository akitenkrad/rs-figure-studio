<script lang="ts">
  import { page } from '$app/stores';
  import { Home, FolderOpen, Settings, ChevronLeft, ChevronRight, Cpu, Download } from 'lucide-svelte';

  let { collapsed, onToggle }: { collapsed: boolean; onToggle: () => void } = $props();

  const currentPath = $derived($page.url.pathname);

  function isActive(href: string): boolean {
    if (href === '/') return currentPath === '/';
    return currentPath.startsWith(href);
  }
</script>

<aside class="sidebar" class:collapsed aria-label="メインナビゲーション">
  <div class="sidebar-content">
    <!-- Logo -->
    <div class="sidebar-logo">
      {#if !collapsed}
        <span class="logo-text">Figurine Studio</span>
      {:else}
        <span class="logo-text-short">FS</span>
      {/if}
    </div>

    <!-- Navigation -->
    <nav class="sidebar-nav">
      <a href="/" class="nav-item" class:active={isActive('/')}>
        <Home size={20} />
        {#if !collapsed}
          <span>ダッシュボード</span>
        {/if}
      </a>

      <div class="nav-section">
        {#if !collapsed}
          <span class="nav-section-label">プロジェクト</span>
        {/if}
        <a href="/" class="nav-item" class:active={currentPath.startsWith('/project')}>
          <FolderOpen size={20} />
          {#if !collapsed}
            <span>プロジェクト</span>
          {/if}
        </a>
      </div>

      <div class="nav-section">
        {#if !collapsed}
          <span class="nav-section-label">設定</span>
        {/if}
        <a href="/settings" class="nav-item" class:active={isActive('/settings') && !isActive('/settings/comfyui') && !isActive('/settings/models')}>
          <Settings size={20} />
          {#if !collapsed}
            <span>設定</span>
          {/if}
        </a>
        <a href="/settings/comfyui" class="nav-item sub-item" class:active={isActive('/settings/comfyui')}>
          <Cpu size={20} />
          {#if !collapsed}
            <span>ComfyUI</span>
          {/if}
        </a>
        <a href="/settings/models" class="nav-item sub-item" class:active={isActive('/settings/models')}>
          <Download size={20} />
          {#if !collapsed}
            <span>モデル管理</span>
          {/if}
        </a>
      </div>
    </nav>
  </div>

  <!-- Toggle Button -->
  <button class="sidebar-toggle" onclick={onToggle} aria-label={collapsed ? 'サイドバーを展開' : 'サイドバーを折りたたみ'}>
    {#if collapsed}
      <ChevronRight size={16} />
    {:else}
      <ChevronLeft size={16} />
    {/if}
  </button>
</aside>

<style>
  .sidebar {
    display: flex;
    flex-direction: column;
    height: 100vh;
    background: var(--bg-secondary);
    border-right: 1px solid var(--border-default);
    padding: var(--space-3);
    width: 240px;
    transition: width 200ms ease;
    overflow: hidden;
  }

  .sidebar.collapsed {
    width: 60px;
  }

  .sidebar-content {
    flex: 1;
    overflow-y: auto;
    overflow-x: hidden;
  }

  .sidebar-logo {
    display: flex;
    align-items: center;
    height: 48px;
    padding: 0 var(--space-2);
    margin-bottom: var(--space-4);
  }

  .logo-text {
    font-size: var(--text-lg);
    font-weight: var(--font-weight-bold);
    color: var(--accent-primary);
    white-space: nowrap;
  }

  .logo-text-short {
    font-size: var(--text-lg);
    font-weight: var(--font-weight-bold);
    color: var(--accent-primary);
  }

  .sidebar-nav {
    display: flex;
    flex-direction: column;
    gap: var(--space-1);
  }

  .nav-item {
    display: flex;
    align-items: center;
    gap: var(--space-2);
    height: 40px;
    padding: 0 var(--space-3);
    border-radius: 6px;
    color: var(--text-secondary);
    font-size: var(--text-sm);
    text-decoration: none;
    white-space: nowrap;
    transition: background-color 150ms ease, color 150ms ease;
  }

  .nav-item:hover {
    background: var(--bg-tertiary);
    color: var(--text-primary);
    text-decoration: none;
  }

  .nav-item.active {
    background: var(--bg-tertiary);
    color: var(--accent-primary);
  }

  .nav-item.sub-item {
    padding-left: var(--space-6);
  }

  .sidebar.collapsed .nav-item.sub-item {
    padding-left: var(--space-3);
  }

  .nav-section {
    margin-top: var(--space-3);
  }

  .nav-section-label {
    display: block;
    padding: 0 var(--space-3);
    margin-bottom: var(--space-1);
    font-size: var(--text-xs);
    font-weight: var(--font-weight-semibold);
    color: var(--text-muted);
    text-transform: uppercase;
    letter-spacing: 0.05em;
  }

  .sidebar-toggle {
    display: flex;
    align-items: center;
    justify-content: center;
    width: 100%;
    height: 36px;
    border: none;
    border-radius: 6px;
    background: transparent;
    color: var(--text-muted);
    cursor: pointer;
    transition: background-color 150ms ease, color 150ms ease;
  }

  .sidebar-toggle:hover {
    background: var(--bg-tertiary);
    color: var(--text-primary);
  }
</style>
