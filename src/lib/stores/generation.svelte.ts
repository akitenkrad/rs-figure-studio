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
} from '$lib/types';
import { generationApi, onGenerationProgress } from '$lib/api/tauri';
import { toastStore } from './toast.svelte';

// --- State ---
let stage = $state<GenerationStage>('idle');
let conceptArtImages = $state<string[]>([]);
let selectedConceptArtPath = $state<string | null>(null);
let conceptImages = $state<string[]>([]);
let selectedConceptPath = $state<string | null>(null);
let directionImages = $state<string[]>([]);
let animationFrames = $state<string[]>([]);
let progress = $state<GenerationProgress | null>(null);
let loading = $state(false);
let error = $state<string | null>(null);

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
    conceptArtImages = images;
    toastStore.addToast('success', `コンセプトアート画像を${images.length}枚生成しました`);
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
    conceptImages = images;
    toastStore.addToast('success', `ピクセルアート画像を${images.length}枚生成しました`);
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
  } catch (e) {
    // State recovery failure is not critical - just start fresh
    console.warn('Failed to load generation state:', e);
    reset();
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
  loadState,
  generateConceptArt,
  selectConceptArt,
  convertToPixelArt,
  generateConcept,
  selectConcept,
  generateDirections,
  generateAnimationFrames,
  promoteToRaw,
  reset,
};
