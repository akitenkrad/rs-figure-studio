<script lang="ts">
  import { ChevronRight } from 'lucide-svelte';
  import type { BreadcrumbItem } from '$lib/types';

  let { items }: { items: BreadcrumbItem[] } = $props();
</script>

<nav aria-label="パンくずリスト" class="breadcrumb">
  {#each items as item, i}
    {#if i > 0}
      <ChevronRight size={14} class="separator" />
    {/if}
    {#if item.href && i < items.length - 1}
      <a href={item.href}>{item.label}</a>
    {:else}
      <span class="current" aria-current="page">{item.label}</span>
    {/if}
  {/each}
</nav>

<style>
  .breadcrumb {
    display: flex;
    align-items: center;
    gap: var(--space-2);
    margin-bottom: var(--space-4);
    font-size: var(--text-sm);
    color: var(--text-secondary);
  }

  .breadcrumb a {
    color: var(--text-secondary);
    text-decoration: none;
    transition: color 150ms ease;
  }

  .breadcrumb a:hover {
    color: var(--accent-primary);
    text-decoration: none;
  }

  .current {
    color: var(--text-primary);
    font-weight: var(--font-weight-medium);
  }

  :global(.breadcrumb .separator) {
    color: var(--text-muted);
    flex-shrink: 0;
  }
</style>
