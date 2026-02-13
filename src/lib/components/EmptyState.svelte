<script lang="ts">
  import { FolderOpen } from 'lucide-svelte';
  import type { Component } from 'svelte';

  let {
    icon,
    title,
    description = '',
    actionLabel = '',
    onAction,
  }: {
    icon?: Component<{ size: number }>;
    title: string;
    description?: string;
    actionLabel?: string;
    onAction?: () => void;
  } = $props();

  const IconComponent = $derived(icon ?? FolderOpen);
</script>

<div class="empty-state">
  <div class="empty-icon">
    <IconComponent size={48} />
  </div>
  <h3 class="empty-title">{title}</h3>
  {#if description}
    <p class="empty-description">{description}</p>
  {/if}
  {#if actionLabel && onAction}
    <button class="btn btn-primary empty-action" onclick={onAction}>
      {actionLabel}
    </button>
  {/if}
</div>

<style>
  .empty-state {
    display: flex;
    flex-direction: column;
    align-items: center;
    justify-content: center;
    padding: var(--space-12) var(--space-6);
    text-align: center;
  }

  .empty-icon {
    color: var(--text-muted);
    margin-bottom: var(--space-4);
  }

  .empty-title {
    color: var(--text-primary);
    margin-bottom: var(--space-2);
  }

  .empty-description {
    color: var(--text-secondary);
    font-size: var(--text-sm);
    max-width: 360px;
    margin-bottom: var(--space-6);
  }

  .empty-action {
    margin-top: var(--space-2);
  }
</style>
