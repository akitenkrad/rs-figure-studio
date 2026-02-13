// ============================================================
// Character Store - Svelte 5 runes based state management
// ============================================================

import type { Character, CreateCharacter, UpdateCharacter } from '$lib/types';
import { characterApi } from '$lib/api/tauri';
import { toastStore } from './toast.svelte';

// --- State ---
let currentCharacter = $state<Character | null>(null);
let characters = $state<Character[]>([]);
let loading = $state(false);
let error = $state<string | null>(null);

// --- Derived ---
const characterCount = $derived(characters.length);

const charactersByStatus = $derived(
  characters.reduce<Record<string, Character[]>>((acc, char) => {
    const status = char.status;
    if (!acc[status]) acc[status] = [];
    acc[status].push(char);
    return acc;
  }, {}),
);

// --- Actions ---
async function loadCharacters(projectId: string): Promise<void> {
  loading = true;
  error = null;
  try {
    characters = await characterApi.list(projectId);
  } catch (e) {
    error = String(e);
    toastStore.addToast('error', `キャラクター一覧の取得に失敗しました: ${e}`);
  } finally {
    loading = false;
  }
}

async function loadCharacter(id: string): Promise<void> {
  loading = true;
  error = null;
  try {
    currentCharacter = await characterApi.get(id);
  } catch (e) {
    error = String(e);
    toastStore.addToast('error', `キャラクター情報の取得に失敗しました: ${e}`);
  } finally {
    loading = false;
  }
}

async function createCharacter(input: CreateCharacter): Promise<Character | null> {
  loading = true;
  error = null;
  try {
    const character = await characterApi.create(input);
    characters = [...characters, character];
    toastStore.addToast('success', 'キャラクターを作成しました');
    return character;
  } catch (e) {
    error = String(e);
    toastStore.addToast('error', `キャラクターの作成に失敗しました: ${e}`);
    return null;
  } finally {
    loading = false;
  }
}

async function updateCharacter(id: string, input: UpdateCharacter): Promise<void> {
  loading = true;
  error = null;
  try {
    const updated = await characterApi.update(id, input);
    characters = characters.map((c) => (c.id === id ? updated : c));
    if (currentCharacter?.id === id) {
      currentCharacter = updated;
    }
    toastStore.addToast('success', 'キャラクター情報を更新しました');
  } catch (e) {
    error = String(e);
    toastStore.addToast('error', `キャラクターの更新に失敗しました: ${e}`);
  } finally {
    loading = false;
  }
}

async function deleteCharacter(id: string): Promise<void> {
  loading = true;
  error = null;
  try {
    await characterApi.delete(id);
    characters = characters.filter((c) => c.id !== id);
    if (currentCharacter?.id === id) {
      currentCharacter = null;
    }
    toastStore.addToast('success', 'キャラクターを削除しました');
  } catch (e) {
    error = String(e);
    toastStore.addToast('error', `キャラクターの削除に失敗しました: ${e}`);
  } finally {
    loading = false;
  }
}

function clearCurrentCharacter(): void {
  currentCharacter = null;
}

// --- Export ---
export const characterStore = {
  get currentCharacter() {
    return currentCharacter;
  },
  get characters() {
    return characters;
  },
  get loading() {
    return loading;
  },
  get error() {
    return error;
  },
  get characterCount() {
    return characterCount;
  },
  get charactersByStatus() {
    return charactersByStatus;
  },
  loadCharacters,
  loadCharacter,
  createCharacter,
  updateCharacter,
  deleteCharacter,
  clearCurrentCharacter,
};
