<script lang="ts">
  import { page } from '$app/stores';
  import Breadcrumb from '$lib/components/Breadcrumb.svelte';
  import { projectStore } from '$lib/stores/project.svelte';

  let { children } = $props();

  const projectId = $derived($page.params.id);

  $effect(() => {
    if (projectId) {
      projectStore.loadProject(projectId);
    }
  });

  const breadcrumbItems = $derived([
    { label: 'ホーム', href: '/' },
    { label: projectStore.currentProject?.name ?? '読み込み中...', href: `/project/${projectId}` },
  ]);
</script>

<Breadcrumb items={breadcrumbItems} />

<header class="project-header">
  <h1>{projectStore.currentProject?.name ?? ''}</h1>
</header>

{@render children()}

<style>
  .project-header {
    margin-bottom: var(--space-6);
  }
</style>
