// ============================================================
// Project Store - Svelte 5 runes based state management
// ============================================================

import type { Project, CreateProject, UpdateProject } from '$lib/types';
import { projectApi } from '$lib/api/tauri';
import { toastStore } from './toast.svelte';

// --- State ---
let currentProject = $state<Project | null>(null);
let projects = $state<Project[]>([]);
let loading = $state(false);
let error = $state<string | null>(null);

// --- Derived ---
const projectCount = $derived(projects.length);

// --- Actions ---
async function loadProjects(): Promise<void> {
  loading = true;
  error = null;
  try {
    projects = await projectApi.list();
  } catch (e) {
    error = String(e);
    toastStore.addToast('error', `プロジェクト一覧の取得に失敗しました: ${e}`);
  } finally {
    loading = false;
  }
}

async function loadProject(id: string): Promise<void> {
  loading = true;
  error = null;
  try {
    currentProject = await projectApi.get(id);
  } catch (e) {
    error = String(e);
    toastStore.addToast('error', `プロジェクトの取得に失敗しました: ${e}`);
  } finally {
    loading = false;
  }
}

async function createProject(data: CreateProject): Promise<Project | null> {
  loading = true;
  error = null;
  try {
    const project = await projectApi.create(data);
    projects = [...projects, project];
    toastStore.addToast('success', 'プロジェクトを作成しました');
    return project;
  } catch (e) {
    error = String(e);
    toastStore.addToast('error', `プロジェクトの作成に失敗しました: ${e}`);
    return null;
  } finally {
    loading = false;
  }
}

async function updateProject(id: string, data: UpdateProject): Promise<void> {
  loading = true;
  error = null;
  try {
    const updated = await projectApi.update(id, data);
    projects = projects.map((p) => (p.id === id ? updated : p));
    if (currentProject?.id === id) {
      currentProject = updated;
    }
    toastStore.addToast('success', 'プロジェクトを更新しました');
  } catch (e) {
    error = String(e);
    toastStore.addToast('error', `プロジェクトの更新に失敗しました: ${e}`);
  } finally {
    loading = false;
  }
}

async function deleteProject(id: string): Promise<void> {
  loading = true;
  error = null;
  try {
    await projectApi.delete(id);
    projects = projects.filter((p) => p.id !== id);
    if (currentProject?.id === id) {
      currentProject = null;
    }
    toastStore.addToast('success', 'プロジェクトを削除しました');
  } catch (e) {
    error = String(e);
    toastStore.addToast('error', `プロジェクトの削除に失敗しました: ${e}`);
  } finally {
    loading = false;
  }
}

function clearCurrentProject(): void {
  currentProject = null;
}

// --- Export ---
export const projectStore = {
  get currentProject() {
    return currentProject;
  },
  get projects() {
    return projects;
  },
  get loading() {
    return loading;
  },
  get error() {
    return error;
  },
  get projectCount() {
    return projectCount;
  },
  loadProjects,
  loadProject,
  createProject,
  updateProject,
  deleteProject,
  clearCurrentProject,
};
