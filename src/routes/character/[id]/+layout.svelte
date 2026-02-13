<script lang="ts">
  import { page } from '$app/stores';
  import Breadcrumb from '$lib/components/Breadcrumb.svelte';
  import { characterStore } from '$lib/stores/character.svelte';
  import { projectStore } from '$lib/stores/project.svelte';
  import { spriteStore } from '$lib/stores/sprite.svelte';
  import { FileText, ImageDown, Cpu, Eye } from 'lucide-svelte';

  let { children } = $props();

  const characterId = $derived($page.params.id);
  const currentPath = $derived($page.url.pathname);

  // Load character and sprites on mount/change
  $effect(() => {
    if (characterId) {
      characterStore.loadCharacter(characterId);
      spriteStore.loadSprites(characterId);
    }
  });

  // Load the parent project when character is loaded
  $effect(() => {
    const char = characterStore.currentCharacter;
    if (char && char.project_id) {
      projectStore.loadProject(char.project_id);
    }
  });

  const breadcrumbItems = $derived([
    { label: 'ホーム', href: '/' },
    {
      label: projectStore.currentProject?.name ?? 'プロジェクト',
      href: characterStore.currentCharacter
        ? `/project/${characterStore.currentCharacter.project_id}`
        : '/',
    },
    {
      label: characterStore.currentCharacter?.name ?? '読み込み中...',
      href: `/character/${characterId}`,
    },
    // Add sub-page label to breadcrumb if on a sub-page
    ...(currentPath.endsWith('/import')
      ? [{ label: 'インポート' }]
      : currentPath.endsWith('/process')
        ? [{ label: 'AI処理' }]
        : currentPath.endsWith('/preview')
          ? [{ label: 'プレビュー' }]
          : []),
  ]);

  const statusLabel = $derived(
    (() => {
      const s = characterStore.currentCharacter?.status;
      if (!s) return '';
      const map: Record<string, string> = {
        draft: '下書き',
        importing: 'インポート中',
        processing: '処理中',
        complete: '完了',
      };
      return map[s] ?? s;
    })(),
  );

  const statusClass = $derived(`badge badge-${characterStore.currentCharacter?.status ?? 'draft'}`);

  // Tab definitions
  const tabs = $derived([
    {
      label: '概要',
      href: `/character/${characterId}`,
      icon: FileText,
      active: currentPath === `/character/${characterId}`,
    },
    {
      label: 'インポート',
      href: `/character/${characterId}/import`,
      icon: ImageDown,
      active: currentPath === `/character/${characterId}/import`,
    },
    {
      label: 'AI処理',
      href: `/character/${characterId}/process`,
      icon: Cpu,
      active: currentPath === `/character/${characterId}/process`,
    },
    {
      label: 'プレビュー',
      href: `/character/${characterId}/preview`,
      icon: Eye,
      active: currentPath === `/character/${characterId}/preview`,
    },
  ]);
</script>

<Breadcrumb items={breadcrumbItems} />

<header class="character-header">
  <div class="character-title-row">
    <h1>{characterStore.currentCharacter?.name ?? ''}</h1>
    {#if characterStore.currentCharacter}
      <span class={statusClass}>{statusLabel}</span>
    {/if}
  </div>
</header>

<!-- Tab Navigation -->
<div class="tab-nav" role="tablist">
  {#each tabs as tab}
    <a
      href={tab.href}
      class="tab-item"
      class:active={tab.active}
      role="tab"
      aria-selected={tab.active}
    >
      <tab.icon size={16} />
      {tab.label}
    </a>
  {/each}
</div>

<!-- Tab Content -->
<div class="tab-content">
  {@render children()}
</div>

<style>
  .character-header {
    margin-bottom: var(--space-4);
  }

  .character-title-row {
    display: flex;
    align-items: center;
    gap: var(--space-3);
  }

  .tab-nav {
    display: flex;
    gap: var(--space-1);
    border-bottom: 1px solid var(--border-default);
    margin-bottom: var(--space-6);
  }

  .tab-item {
    display: flex;
    align-items: center;
    gap: var(--space-2);
    padding: var(--space-2) var(--space-4);
    font-size: var(--text-sm);
    color: var(--text-secondary);
    text-decoration: none;
    border-bottom: 2px solid transparent;
    transition:
      color 150ms ease,
      border-color 150ms ease;
  }

  .tab-item:hover {
    color: var(--text-primary);
    text-decoration: none;
  }

  .tab-item.active {
    color: var(--accent-primary);
    border-bottom-color: var(--accent-primary);
  }

  .tab-content {
    min-height: 0;
  }
</style>
