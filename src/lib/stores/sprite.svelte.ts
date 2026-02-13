// ============================================================
// Sprite Store - Svelte 5 runes based state management
// ============================================================

import type { Sprite, SpritesheetResult } from '$lib/types';
import { characterApi, spriteApi } from '$lib/api/tauri';
import { toastStore } from './toast.svelte';

// --- State ---
let sprites = $state<Sprite[]>([]);
let loading = $state(false);
let error = $state<string | null>(null);
let importing = $state(false);
let normalizing = $state(false);
let generating = $state(false);
let spritesheetResult = $state<SpritesheetResult | null>(null);

// --- Derived ---
const spriteCount = $derived(sprites.length);

const spritesByDirection = $derived(
  sprites.reduce<Record<string, Sprite[]>>((acc, sprite) => {
    const dir = sprite.direction || 'unassigned';
    if (!acc[dir]) acc[dir] = [];
    acc[dir].push(sprite);
    return acc;
  }, {}),
);

const spritesByStatus = $derived(
  sprites.reduce<Record<string, Sprite[]>>((acc, sprite) => {
    const status = sprite.status;
    if (!acc[status]) acc[status] = [];
    acc[status].push(sprite);
    return acc;
  }, {}),
);

const isProcessing = $derived(importing || normalizing || generating);

// --- Actions ---
async function loadSprites(characterId: string): Promise<void> {
  loading = true;
  error = null;
  try {
    sprites = await spriteApi.list(characterId);
  } catch (e) {
    error = String(e);
    toastStore.addToast('error', `スプライト一覧の取得に失敗しました: ${e}`);
  } finally {
    loading = false;
  }
}

async function importSprites(characterId: string, filePaths: string[]): Promise<Sprite[]> {
  importing = true;
  error = null;
  try {
    const imported = await characterApi.importSprites(characterId, filePaths);
    sprites = [...sprites, ...imported];
    toastStore.addToast('success', `${imported.length}枚の画像をインポートしました`);
    return imported;
  } catch (e) {
    error = String(e);
    toastStore.addToast('error', `画像のインポートに失敗しました: ${e}`);
    return [];
  } finally {
    importing = false;
  }
}

async function normalizeBatch(
  characterId: string,
  tileWidth: number,
  tileHeight: number,
): Promise<void> {
  normalizing = true;
  error = null;
  try {
    await spriteApi.normalizeBatch(characterId, tileWidth, tileHeight);
    // Reload sprites to get updated paths
    sprites = await spriteApi.list(characterId);
    toastStore.addToast('success', 'スプライトの正規化が完了しました');
  } catch (e) {
    error = String(e);
    toastStore.addToast('error', `正規化に失敗しました: ${e}`);
  } finally {
    normalizing = false;
  }
}

async function generateSpritesheet(characterId: string): Promise<SpritesheetResult | null> {
  generating = true;
  error = null;
  try {
    const result = await spriteApi.generateSpritesheet(characterId);
    spritesheetResult = result;
    toastStore.addToast('success', 'スプライトシートを生成しました');
    return result;
  } catch (e) {
    error = String(e);
    toastStore.addToast('error', `スプライトシートの生成に失敗しました: ${e}`);
    return null;
  } finally {
    generating = false;
  }
}

async function updateAssignment(
  spriteId: string,
  direction: string,
  animation: string,
): Promise<boolean> {
  error = null;
  try {
    await spriteApi.updateAssignment(spriteId, direction, animation);
    // Update local state
    sprites = sprites.map((s) =>
      s.id === spriteId ? { ...s, direction, animation } : s,
    );
    return true;
  } catch (e) {
    error = String(e);
    toastStore.addToast('error', `割り当て更新に失敗しました: ${e}`);
    return false;
  }
}

function clearSprites(): void {
  sprites = [];
  spritesheetResult = null;
}

// --- Export ---
export const spriteStore = {
  get sprites() {
    return sprites;
  },
  get loading() {
    return loading;
  },
  get error() {
    return error;
  },
  get importing() {
    return importing;
  },
  get normalizing() {
    return normalizing;
  },
  get generating() {
    return generating;
  },
  get isProcessing() {
    return isProcessing;
  },
  get spriteCount() {
    return spriteCount;
  },
  get spritesByDirection() {
    return spritesByDirection;
  },
  get spritesByStatus() {
    return spritesByStatus;
  },
  get spritesheetResult() {
    return spritesheetResult;
  },
  loadSprites,
  importSprites,
  normalizeBatch,
  generateSpritesheet,
  updateAssignment,
  clearSprites,
};
