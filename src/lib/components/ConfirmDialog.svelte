<script lang="ts">
  import { AlertTriangle, Info, AlertCircle } from 'lucide-svelte';

  let {
    open = false,
    title,
    message,
    confirmLabel = '確認',
    cancelLabel = 'キャンセル',
    variant = 'danger',
    onConfirm,
    onCancel,
  }: {
    open: boolean;
    title: string;
    message: string;
    confirmLabel?: string;
    cancelLabel?: string;
    variant?: 'danger' | 'warning' | 'info';
    onConfirm: () => void;
    onCancel: () => void;
  } = $props();

  function handleOverlayClick(e: MouseEvent) {
    if (e.target === e.currentTarget) {
      onCancel();
    }
  }

  function handleKeydown(e: KeyboardEvent) {
    if (e.key === 'Escape') {
      onCancel();
    }
  }
</script>

{#if open}
  <!-- svelte-ignore a11y_no_noninteractive_element_interactions -->
  <!-- svelte-ignore a11y_interactive_supports_focus -->
  <div
    class="modal-overlay"
    role="dialog"
    aria-modal="true"
    aria-labelledby="confirm-dialog-title"
    onclick={handleOverlayClick}
    onkeydown={handleKeydown}
  >
    <div class="modal">
      <div class="modal-header">
        <div class="modal-title-row">
          <span class="modal-icon variant-{variant}">
            {#if variant === 'danger'}
              <AlertCircle size={20} />
            {:else if variant === 'warning'}
              <AlertTriangle size={20} />
            {:else}
              <Info size={20} />
            {/if}
          </span>
          <h2 id="confirm-dialog-title">{title}</h2>
        </div>
      </div>
      <div class="modal-body">
        <p>{message}</p>
      </div>
      <div class="modal-footer">
        <button class="btn btn-secondary" onclick={onCancel}>
          {cancelLabel}
        </button>
        <button
          class="btn {variant === 'danger' ? 'btn-danger' : variant === 'warning' ? 'btn-primary' : 'btn-primary'}"
          onclick={onConfirm}
        >
          {confirmLabel}
        </button>
      </div>
    </div>
  </div>
{/if}

<style>
  .modal-overlay {
    position: fixed;
    inset: 0;
    background: hsla(0, 0%, 0%, 0.6);
    z-index: 9000;
    display: flex;
    align-items: center;
    justify-content: center;
    animation: fade-in 150ms ease;
  }

  .modal {
    background: var(--bg-elevated);
    border-radius: 12px;
    padding: var(--space-6);
    min-width: 400px;
    max-width: 560px;
    box-shadow: var(--shadow-lg);
    animation: modal-scale-in 200ms ease-out;
  }

  .modal-header {
    margin-bottom: var(--space-4);
  }

  .modal-title-row {
    display: flex;
    align-items: center;
    gap: var(--space-2);
  }

  .modal-title-row h2 {
    font-size: var(--text-xl);
    font-weight: var(--font-weight-semibold);
  }

  .modal-icon {
    display: flex;
    align-items: center;
    flex-shrink: 0;
  }

  .modal-icon.variant-danger {
    color: var(--accent-danger);
  }

  .modal-icon.variant-warning {
    color: var(--accent-warning);
  }

  .modal-icon.variant-info {
    color: var(--accent-info);
  }

  .modal-body {
    color: var(--text-secondary);
    font-size: var(--text-sm);
    margin-bottom: var(--space-6);
  }

  .modal-footer {
    display: flex;
    justify-content: flex-end;
    gap: var(--space-3);
  }

  @keyframes fade-in {
    from { opacity: 0; }
    to   { opacity: 1; }
  }

  @keyframes modal-scale-in {
    from { transform: scale(0.95); opacity: 0; }
    to   { transform: scale(1); opacity: 1; }
  }
</style>
