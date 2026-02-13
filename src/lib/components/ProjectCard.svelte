<script lang="ts">
  import type { Project } from '$lib/types';
  import { FolderOpen, Users } from 'lucide-svelte';

  let {
    project,
    onclick,
  }: {
    project: Project;
    onclick: (projectId: string) => void;
  } = $props();

  const formattedDate = $derived(
    new Date(project.updated_at).toLocaleDateString('ja-JP'),
  );
</script>

<button class="project-card card" onclick={() => onclick(project.id)}>
  <div class="card-icon">
    <FolderOpen size={24} />
  </div>
  <div class="card-content">
    <h3 class="card-title">{project.name}</h3>
    <div class="card-meta">
      <span class="meta-item">
        <Users size={14} />
        {project.character_count ?? 0}体
      </span>
      <span class="meta-item">
        {project.tile_width}x{project.tile_height}px
      </span>
    </div>
    <time class="card-time">{formattedDate}</time>
  </div>
</button>

<style>
  .project-card {
    display: flex;
    gap: var(--space-3);
    align-items: flex-start;
    text-align: left;
    width: 100%;
    cursor: pointer;
    font-family: var(--font-sans);
    border: 1px solid var(--border-default);
  }

  .card-icon {
    display: flex;
    align-items: center;
    justify-content: center;
    width: 48px;
    height: 48px;
    border-radius: 8px;
    background: var(--bg-tertiary);
    color: var(--accent-primary);
    flex-shrink: 0;
  }

  .card-content {
    flex: 1;
    min-width: 0;
  }

  .card-title {
    font-size: var(--text-base);
    font-weight: var(--font-weight-semibold);
    color: var(--text-primary);
    margin-bottom: var(--space-1);
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .card-meta {
    display: flex;
    align-items: center;
    gap: var(--space-3);
    margin-bottom: var(--space-1);
    font-size: var(--text-sm);
    color: var(--text-secondary);
  }

  .meta-item {
    display: flex;
    align-items: center;
    gap: var(--space-1);
  }

  .card-time {
    font-size: var(--text-xs);
    color: var(--text-muted);
  }
</style>
