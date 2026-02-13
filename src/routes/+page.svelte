<script lang="ts">
  import { onMount } from 'svelte';
  import { goto } from '$app/navigation';
  import { Plus, Loader2 } from 'lucide-svelte';
  import { projectStore } from '$lib/stores/project.svelte';
  import ProjectCard from '$lib/components/ProjectCard.svelte';
  import EmptyState from '$lib/components/EmptyState.svelte';

  onMount(() => {
    projectStore.loadProjects();
  });

  function handleProjectClick(projectId: string) {
    goto(`/project/${projectId}`);
  }

  function handleCreateProject() {
    goto('/project/new');
  }
</script>

<div class="dashboard">
  <header class="dashboard-header">
    <h1>ダッシュボード</h1>
    <button class="btn btn-primary" onclick={handleCreateProject}>
      <Plus size={16} />
      新規プロジェクト
    </button>
  </header>

  {#if projectStore.loading}
    <div class="loading-state">
      <Loader2 size={32} class="spin" />
      <p>読み込み中...</p>
    </div>
  {:else if projectStore.projects.length === 0}
    <EmptyState
      title="プロジェクトがありません"
      description="新しいプロジェクトを作成して，ピクセルアートキャラクターの制作を始めましょう．"
      actionLabel="新規プロジェクトを作成"
      onAction={handleCreateProject}
    />
  {:else}
    <div class="card-grid">
      {#each projectStore.projects as project (project.id)}
        <ProjectCard
          {project}
          onclick={handleProjectClick}
        />
      {/each}
    </div>
  {/if}
</div>

<style>
  .dashboard {
    max-width: 1200px;
  }

  .dashboard-header {
    display: flex;
    align-items: center;
    justify-content: space-between;
    margin-bottom: var(--space-6);
  }

  .loading-state {
    display: flex;
    flex-direction: column;
    align-items: center;
    justify-content: center;
    gap: var(--space-3);
    padding: var(--space-12);
    color: var(--text-muted);
  }

  :global(.loading-state .spin) {
    animation: spin 1s linear infinite;
  }
</style>
