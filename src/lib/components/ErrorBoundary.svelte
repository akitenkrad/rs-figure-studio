<script lang="ts">
  import { AlertTriangle, RefreshCw } from 'lucide-svelte';
  import { onMount } from 'svelte';
  import type { Snippet } from 'svelte';

  let {
    children,
    fallback,
  }: {
    children: Snippet;
    fallback?: Snippet<[{ error: Error; retry: () => void }]>;
  } = $props();

  let error = $state<Error | null>(null);
  let key = $state(0);

  function handleError(e: unknown) {
    if (e instanceof Error) {
      error = e;
    } else {
      error = new Error(String(e));
    }
  }

  function retry() {
    error = null;
    key += 1;
  }

  onMount(() => {
    function onWindowError(e: ErrorEvent) {
      handleError(e.error);
    }
    function onUnhandledRejection(e: PromiseRejectionEvent) {
      handleError(e.reason);
    }

    window.addEventListener('error', onWindowError);
    window.addEventListener('unhandledrejection', onUnhandledRejection);

    return () => {
      window.removeEventListener('error', onWindowError);
      window.removeEventListener('unhandledrejection', onUnhandledRejection);
    };
  });
</script>

{#if error}
  {#if fallback}
    {@render fallback({ error, retry })}
  {:else}
    <div class="error-boundary">
      <div class="error-content">
        <div class="error-icon">
          <AlertTriangle size={32} />
        </div>
        <h3 class="error-title">エラーが発生しました</h3>
        <p class="error-message">{error.message}</p>
        <button class="btn btn-primary" onclick={retry}>
          <RefreshCw size={16} />
          再試行
        </button>
      </div>
    </div>
  {/if}
{:else}
  {#key key}
    {@render children()}
  {/key}
{/if}

<style>
  .error-boundary {
    display: flex;
    align-items: center;
    justify-content: center;
    padding: var(--space-12) var(--space-6);
  }

  .error-content {
    display: flex;
    flex-direction: column;
    align-items: center;
    text-align: center;
    gap: var(--space-3);
    max-width: 400px;
  }

  .error-icon {
    color: var(--accent-danger);
    margin-bottom: var(--space-2);
  }

  .error-title {
    color: var(--text-primary);
    font-size: var(--text-lg);
    font-weight: var(--font-weight-semibold);
  }

  .error-message {
    color: var(--text-secondary);
    font-size: var(--text-sm);
    word-break: break-word;
  }
</style>
