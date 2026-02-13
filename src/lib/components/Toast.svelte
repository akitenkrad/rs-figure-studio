<script lang="ts">
  import { X, Check, AlertTriangle, Info } from 'lucide-svelte';
  import type { ToastItem } from '$lib/types';

  let { toasts, onRemove }: { toasts: ToastItem[]; onRemove: (id: string) => void } = $props();
</script>

{#if toasts.length > 0}
  <div class="toast-container">
    {#each toasts as toast (toast.id)}
      <div class="toast toast-{toast.type}" role="alert" aria-live="assertive">
        <span class="toast-icon">
          {#if toast.type === 'success'}
            <Check size={16} />
          {:else if toast.type === 'error'}
            <X size={16} />
          {:else if toast.type === 'warning'}
            <AlertTriangle size={16} />
          {:else}
            <Info size={16} />
          {/if}
        </span>
        <span class="toast-message">{toast.message}</span>
        <button class="toast-close" onclick={() => onRemove(toast.id)} aria-label="閉じる">
          <X size={14} />
        </button>
      </div>
    {/each}
  </div>
{/if}

<style>
  .toast-container {
    position: fixed;
    bottom: var(--space-6);
    right: var(--space-6);
    z-index: 9999;
    display: flex;
    flex-direction: column-reverse;
    gap: var(--space-2);
  }

  .toast {
    display: flex;
    align-items: center;
    gap: var(--space-3);
    min-width: 320px;
    max-width: 480px;
    padding: var(--space-3) var(--space-4);
    background: var(--bg-elevated);
    border-radius: 8px;
    border-left: 4px solid;
    box-shadow: var(--shadow-lg);
    animation: toast-slide-in 200ms ease-out;
    font-size: var(--text-sm);
    color: var(--text-primary);
  }

  .toast-success { border-left-color: var(--accent-success); }
  .toast-error   { border-left-color: var(--accent-danger); }
  .toast-warning { border-left-color: var(--accent-warning); }
  .toast-info    { border-left-color: var(--accent-info); }

  .toast-icon {
    display: flex;
    align-items: center;
    flex-shrink: 0;
  }

  .toast-success .toast-icon { color: var(--accent-success); }
  .toast-error .toast-icon   { color: var(--accent-danger); }
  .toast-warning .toast-icon { color: var(--accent-warning); }
  .toast-info .toast-icon    { color: var(--accent-info); }

  .toast-message {
    flex: 1;
  }

  .toast-close {
    display: flex;
    align-items: center;
    justify-content: center;
    width: 24px;
    height: 24px;
    border: none;
    border-radius: 4px;
    background: transparent;
    color: var(--text-muted);
    cursor: pointer;
    flex-shrink: 0;
    padding: 0;
    transition: background-color 150ms ease, color 150ms ease;
  }

  .toast-close:hover {
    background: var(--bg-tertiary);
    color: var(--text-primary);
  }

  @keyframes toast-slide-in {
    from { transform: translateX(100%); opacity: 0; }
    to   { transform: translateX(0); opacity: 1; }
  }
</style>
