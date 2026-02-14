// ============================================================
// Generation Store - AI character generation pipeline state
// Stages: concept → direction → animation → completed
// ============================================================

import type {
  GenerationStage,
  GenerationProgress,
  GenerationState,
  ConceptParams,
  PixelArtConversionParams,
  DirectionParams,
  AnimationParams,
  SavedImages,
} from '$lib/types';
import { generationApi, comfyuiModelApi, onGenerationProgress } from '$lib/api/tauri';
import { toastStore } from './toast.svelte';

// --- State ---
let stage = $state<GenerationStage>('idle');
let conceptArtImages = $state<string[]>([]);
let selectedConceptArtPath = $state<string | null>(null);
let conceptImages = $state<string[]>([]);
let selectedConceptPath = $state<string | null>(null);
let directionImages = $state<string[]>([]);
let animationFrames = $state<string[]>([]);
let savedImages = $state<SavedImages>({ concept_art: [], pixel_art: [], direction: [], animation: [] });
let progress = $state<GenerationProgress | null>(null);
let loading = $state(false);
let error = $state<string | null>(null);
let availableCheckpoints = $state<string[]>([]);
let availableLoras = $state<string[]>([]);

// --- Derived ---
const hasConceptArtImages = $derived(conceptArtImages.length > 0);
const hasConceptImages = $derived(conceptImages.length > 0);
const hasDirectionImages = $derived(directionImages.length > 0);
const hasAnimationFrames = $derived(animationFrames.length > 0);
const progressPercentage = $derived(
  progress && progress.total > 0
    ? Math.round((progress.current / progress.total) * 100)
    : 0,
);

// --- Actions ---
async function generateConceptArt(
  characterId: string,
  params: ConceptParams,
): Promise<string[]> {
  loading = true;
  error = null;
  stage = 'concept_art';
  progress = null;

  let unlisten: (() => void) | null = null;
  try {
    unlisten = await onGenerationProgress((p) => {
      progress = p;
    });

    const images = await generationApi.generateConceptArt(characterId, params);
    conceptArtImages = [...conceptArtImages, ...images];
    toastStore.addToast('success', `コンセプトアート画像を${images.length}枚生成しました（合計${conceptArtImages.length}枚）`);
    return images;
  } catch (e) {
    error = String(e);
    toastStore.addToast('error', `コンセプトアート生成に失敗しました: ${e}`);
    return [];
  } finally {
    loading = false;
    progress = null;
    if (unlisten) unlisten();
  }
}

function selectConceptArt(path: string): void {
  selectedConceptArtPath = path;
}

async function convertToPixelArt(
  characterId: string,
  params: PixelArtConversionParams,
): Promise<string[]> {
  if (!selectedConceptArtPath) {
    toastStore.addToast('error', 'コンセプトアート画像を選択してください');
    return [];
  }

  loading = true;
  error = null;
  stage = 'concept';
  progress = null;

  let unlisten: (() => void) | null = null;
  try {
    unlisten = await onGenerationProgress((p) => {
      progress = p;
    });

    const images = await generationApi.convertToPixelArt(
      characterId,
      selectedConceptArtPath,
      params,
    );
    conceptImages = [...conceptImages, ...images];
    toastStore.addToast('success', `ピクセルアート画像を${images.length}枚生成しました（合計${conceptImages.length}枚）`);
    return images;
  } catch (e) {
    error = String(e);
    toastStore.addToast('error', `ピクセルアート変換に失敗しました: ${e}`);
    return [];
  } finally {
    loading = false;
    progress = null;
    if (unlisten) unlisten();
  }
}

async function generateConcept(
  characterId: string,
  params: ConceptParams,
): Promise<string[]> {
  loading = true;
  error = null;
  stage = 'concept';
  progress = null;

  let unlisten: (() => void) | null = null;
  try {
    unlisten = await onGenerationProgress((p) => {
      progress = p;
    });

    const images = await generationApi.generateConcept(characterId, params);
    conceptImages = images;
    toastStore.addToast('success', `コンセプト画像を${images.length}枚生成しました`);
    return images;
  } catch (e) {
    error = String(e);
    toastStore.addToast('error', `コンセプト生成に失敗しました: ${e}`);
    return [];
  } finally {
    loading = false;
    progress = null;
    if (unlisten) unlisten();
  }
}

function selectConcept(path: string): void {
  selectedConceptPath = path;
}

async function generateDirections(
  characterId: string,
  params: DirectionParams,
): Promise<string[]> {
  if (!selectedConceptPath) {
    toastStore.addToast('error', 'コンセプト画像を選択してください');
    return [];
  }

  loading = true;
  error = null;
  stage = 'direction';
  progress = null;

  let unlisten: (() => void) | null = null;
  try {
    unlisten = await onGenerationProgress((p) => {
      progress = p;
    });

    const images = await generationApi.generateDirections(
      characterId,
      selectedConceptPath,
      params,
    );
    directionImages = images;
    toastStore.addToast('success', `${images.length}方向の画像を生成しました`);
    return images;
  } catch (e) {
    error = String(e);
    toastStore.addToast('error', `方向展開に失敗しました: ${e}`);
    return [];
  } finally {
    loading = false;
    progress = null;
    if (unlisten) unlisten();
  }
}

async function generateAnimationFrames(
  characterId: string,
  basePosePath: string,
  direction: string,
  params: AnimationParams,
): Promise<string[]> {
  loading = true;
  error = null;
  stage = 'animation';
  progress = null;

  let unlisten: (() => void) | null = null;
  try {
    unlisten = await onGenerationProgress((p) => {
      progress = p;
    });

    const frames = await generationApi.generateAnimationFrames(
      characterId,
      basePosePath,
      direction,
      params,
    );
    animationFrames = [...animationFrames, ...frames];
    toastStore.addToast('success', `${direction}方向のアニメーションフレームを${frames.length}枚生成しました`);
    return frames;
  } catch (e) {
    error = String(e);
    toastStore.addToast('error', `アニメーション生成に失敗しました: ${e}`);
    return [];
  } finally {
    loading = false;
    progress = null;
    if (unlisten) unlisten();
  }
}

async function promoteToRaw(spriteIds: string[]): Promise<string[]> {
  loading = true;
  error = null;
  try {
    const promoted = await generationApi.promoteGeneratedToRaw(spriteIds);
    stage = 'completed';
    toastStore.addToast('success', `${promoted.length}枚のスプライトをインポートしました`);
    return promoted;
  } catch (e) {
    error = String(e);
    toastStore.addToast('error', `スプライト昇格に失敗しました: ${e}`);
    return [];
  } finally {
    loading = false;
  }
}

async function loadState(characterId: string): Promise<void> {
  try {
    const saved = await generationApi.getGenerationState(characterId);

    conceptArtImages = saved.concept_art_images;
    selectedConceptArtPath = saved.selected_concept_art_path;
    conceptImages = saved.concept_images;
    selectedConceptPath = saved.selected_concept_path;
    directionImages = saved.direction_images;
    animationFrames = saved.animation_frames;
    stage = saved.stage;
    // Clear transient state
    progress = null;
    loading = false;
    error = null;
    // Load saved images
    await loadSavedImages(characterId);
  } catch (e) {
    // State recovery failure is not critical - just start fresh
    console.warn('Failed to load generation state:', e);
    reset();
  }
}

async function resetToIdle(characterId: string): Promise<void> {
  loading = true;
  error = null;
  try {
    await generationApi.clearGenerationStage(characterId, 'concept_art');
    await generationApi.clearGenerationStage(characterId, 'concept');
    conceptArtImages = [];
    selectedConceptArtPath = null;
    conceptImages = [];
    selectedConceptPath = null;
    stage = 'idle';
    toastStore.addToast('success', 'コンセプトアートをクリアしました');
  } catch (e) {
    error = String(e);
    toastStore.addToast('error', `クリアに失敗しました: ${e}`);
  } finally {
    loading = false;
  }
}

async function resetToConceptArt(characterId: string): Promise<void> {
  loading = true;
  error = null;
  try {
    await generationApi.clearGenerationStage(characterId, 'concept');
    conceptImages = [];
    selectedConceptPath = null;
    stage = 'concept_art';
    toastStore.addToast('success', 'ピクセルアート変換をクリアしました');
  } catch (e) {
    error = String(e);
    toastStore.addToast('error', `クリアに失敗しました: ${e}`);
  } finally {
    loading = false;
  }
}

async function resetToPixelArt(characterId: string): Promise<void> {
  loading = true;
  error = null;
  try {
    await generationApi.clearGenerationStage(characterId, 'direction');
    await generationApi.clearGenerationStage(characterId, 'animation');
    directionImages = [];
    animationFrames = [];
    stage = 'concept';
    toastStore.addToast('success', '方向展開をクリアしました');
  } catch (e) {
    error = String(e);
    toastStore.addToast('error', `クリアに失敗しました: ${e}`);
  } finally {
    loading = false;
  }
}

async function resetToDirection(characterId: string): Promise<void> {
  loading = true;
  error = null;
  try {
    await generationApi.clearGenerationStage(characterId, 'animation');
    animationFrames = [];
    stage = 'direction';
    toastStore.addToast('success', 'アニメーションをクリアしました');
  } catch (e) {
    error = String(e);
    toastStore.addToast('error', `クリアに失敗しました: ${e}`);
  } finally {
    loading = false;
  }
}

async function loadSavedImages(characterId: string): Promise<void> {
  try {
    savedImages = await generationApi.getSavedImages(characterId);
  } catch (e) {
    console.warn('Failed to load saved images:', e);
  }
}

async function saveImage(characterId: string, imagePath: string, stage: string): Promise<void> {
  try {
    await generationApi.saveGenerationImage(characterId, imagePath, stage);
    await loadSavedImages(characterId);
    toastStore.addToast('success', '画像を保存しました');
  } catch (e) {
    toastStore.addToast('error', `画像の保存に失敗しました: ${e}`);
  }
}

async function deleteSavedImage(characterId: string, imagePath: string): Promise<void> {
  try {
    await generationApi.deleteSavedImage(imagePath);
    await loadSavedImages(characterId);
    toastStore.addToast('success', '保存済み画像を削除しました');
  } catch (e) {
    toastStore.addToast('error', `画像の削除に失敗しました: ${e}`);
  }
}

function isSaved(imagePath: string, stage: string): boolean {
  const filename = imagePath.split('/').pop() || '';
  const list =
    stage === 'concept_art' ? savedImages.concept_art
    : stage === 'pixel_art' ? savedImages.pixel_art
    : stage === 'direction' ? savedImages.direction
    : savedImages.animation;
  return list.some((p) => p.endsWith(filename));
}

async function fetchCheckpoints(): Promise<void> {
  try {
    availableCheckpoints = await comfyuiModelApi.listCheckpoints();
  } catch (e) {
    console.warn('Failed to fetch checkpoints:', e);
  }
}

async function fetchLoras(): Promise<void> {
  try {
    availableLoras = await comfyuiModelApi.listLoras();
  } catch (e) {
    console.warn('Failed to fetch loras:', e);
  }
}

function reset(): void {
  stage = 'idle';
  conceptArtImages = [];
  selectedConceptArtPath = null;
  conceptImages = [];
  selectedConceptPath = null;
  directionImages = [];
  animationFrames = [];
  savedImages = { concept_art: [], pixel_art: [], direction: [], animation: [] };
  progress = null;
  loading = false;
  error = null;
}

// --- Export ---
export const generationStore = {
  get stage() {
    return stage;
  },
  get conceptArtImages() {
    return conceptArtImages;
  },
  get selectedConceptArtPath() {
    return selectedConceptArtPath;
  },
  get conceptImages() {
    return conceptImages;
  },
  get selectedConceptPath() {
    return selectedConceptPath;
  },
  get directionImages() {
    return directionImages;
  },
  get animationFrames() {
    return animationFrames;
  },
  get progress() {
    return progress;
  },
  get loading() {
    return loading;
  },
  get error() {
    return error;
  },
  get hasConceptArtImages() {
    return hasConceptArtImages;
  },
  get hasConceptImages() {
    return hasConceptImages;
  },
  get hasDirectionImages() {
    return hasDirectionImages;
  },
  get hasAnimationFrames() {
    return hasAnimationFrames;
  },
  get progressPercentage() {
    return progressPercentage;
  },
  get savedImages() {
    return savedImages;
  },
  get availableCheckpoints() {
    return availableCheckpoints;
  },
  get availableLoras() {
    return availableLoras;
  },
  get hasSavedImages() {
    return (
      savedImages.concept_art.length > 0 ||
      savedImages.pixel_art.length > 0 ||
      savedImages.direction.length > 0 ||
      savedImages.animation.length > 0
    );
  },
  loadState,
  generateConceptArt,
  selectConceptArt,
  convertToPixelArt,
  generateConcept,
  selectConcept,
  generateDirections,
  generateAnimationFrames,
  promoteToRaw,
  resetToIdle,
  resetToConceptArt,
  resetToPixelArt,
  resetToDirection,
  reset,
  loadSavedImages,
  saveImage,
  deleteSavedImage,
  isSaved,
  fetchCheckpoints,
  fetchLoras,
};
