<script lang="ts">
  import { Loader2 } from 'lucide-svelte';

  let {
    size = 'md',
    label = '読み込み中',
  }: {
    size?: 'sm' | 'md' | 'lg';
    label?: string;
  } = $props();

  const iconSize = $derived(
    size === 'sm' ? 16 : size === 'md' ? 24 : 32,
  );
</script>

<div class="loading-spinner size-{size}" role="status" aria-label={label}>
  <Loader2 size={iconSize} class="spin" />
  <span class="sr-only">{label}</span>
</div>

<style>
  .loading-spinner {
    display: flex;
    align-items: center;
    justify-content: center;
    color: var(--text-muted);
  }

  .size-sm {
    padding: var(--space-2);
  }

  .size-md {
    padding: var(--space-4);
  }

  .size-lg {
    padding: var(--space-8);
  }

  :global(.loading-spinner .spin) {
    animation: spin 1s linear infinite;
  }
</style>
