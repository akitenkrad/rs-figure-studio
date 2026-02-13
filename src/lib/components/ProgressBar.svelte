<script lang="ts">
  let {
    current,
    total,
    label = '',
    showPercentage = true,
    showCounts = false,
    variant = 'default',
    animated = true,
    statusText = '',
  }: {
    current: number;
    total: number;
    label?: string;
    showPercentage?: boolean;
    showCounts?: boolean;
    variant?: 'default' | 'success' | 'warning' | 'danger';
    animated?: boolean;
    statusText?: string;
  } = $props();

  const percentage = $derived(
    total > 0 ? Math.min(100, Math.round((current / total) * 100)) : 0,
  );
</script>

<div class="progress-wrapper">
  {#if label}
    <span class="progress-title">{label}</span>
  {/if}

  <div
    class="progress-container"
    role="progressbar"
    aria-valuenow={current}
    aria-valuemin={0}
    aria-valuemax={total}
  >
    <div
      class="progress-bar {variant}"
      class:animated={animated && percentage < 100}
      style:width="{percentage}%"
    ></div>
    {#if showPercentage}
      <span class="progress-label">{percentage}%</span>
    {/if}
  </div>

  <div class="progress-meta">
    {#if showCounts}
      <span class="progress-counts">{current} / {total}</span>
    {/if}
    {#if statusText}
      <span class="progress-status">{statusText}</span>
    {/if}
  </div>
</div>

<style>
  .progress-wrapper {
    display: flex;
    flex-direction: column;
    gap: var(--space-2);
  }

  .progress-title {
    font-size: var(--text-sm);
    font-weight: var(--font-weight-medium);
    color: var(--text-primary);
  }

  .progress-meta {
    display: flex;
    align-items: center;
    justify-content: space-between;
    min-height: 0;
  }

  .progress-meta:empty {
    display: none;
  }

  .progress-counts {
    font-size: var(--text-sm);
    color: var(--text-secondary);
  }

  .progress-status {
    font-size: var(--text-xs);
    color: var(--text-muted);
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
    max-width: 300px;
  }
</style>
