// ============================================================
// Figurine Studio - Tauri Invoke Wrapper & Event Listeners
// ============================================================

import { invoke } from '@tauri-apps/api/core';
import { listen, type UnlistenFn } from '@tauri-apps/api/event';
import type {
  Project,
  CreateProject,
  UpdateProject,
  Character,
  CreateCharacter,
  UpdateCharacter,
  Sprite,
  OnnxModelStatus,
  SpritesheetResult,
  BevyExportResult,
  DownloadProgress,
  ProcessingProgress,
  ComfyUIProgress,
  BgRemovalProgress,
  Workflow,
  CreateWorkflow,
  ProcessingParams,
} from '$lib/types';

// --- Project API ---

export const projectApi = {
  async create(input: CreateProject): Promise<Project> {
    return invoke<Project>('create_project', { input });
  },

  async get(id: string): Promise<Project> {
    return invoke<Project>('get_project', { id });
  },

  async list(): Promise<Project[]> {
    return invoke<Project[]>('list_projects');
  },

  async update(id: string, input: UpdateProject): Promise<Project> {
    return invoke<Project>('update_project', { id, input });
  },

  async delete(id: string): Promise<void> {
    return invoke('delete_project', { id });
  },
};

// --- Character API ---

export const characterApi = {
  async create(input: CreateCharacter): Promise<Character> {
    return invoke<Character>('create_character', { input });
  },

  async get(id: string): Promise<Character> {
    return invoke<Character>('get_character', { id });
  },

  async list(projectId: string): Promise<Character[]> {
    return invoke<Character[]>('list_characters', { projectId });
  },

  async update(id: string, input: UpdateCharacter): Promise<Character> {
    return invoke<Character>('update_character', { id, input });
  },

  async delete(id: string): Promise<void> {
    return invoke('delete_character', { id });
  },

  async importSprites(characterId: string, filePaths: string[]): Promise<Sprite[]> {
    return invoke<Sprite[]>('import_sprites', { characterId, filePaths });
  },
};

// --- Settings API ---

export const settingsApi = {
  async getAll(): Promise<Record<string, string>> {
    return invoke<Record<string, string>>('get_settings');
  },

  async update(key: string, value: string): Promise<void> {
    return invoke('update_setting', { key, value });
  },
};

// --- ONNX Model API ---

export const onnxApi = {
  async checkModel(): Promise<OnnxModelStatus> {
    return invoke<OnnxModelStatus>('check_onnx_model');
  },

  async downloadModel(): Promise<string> {
    return invoke<string>('download_onnx_model');
  },

  async initialize(): Promise<void> {
    return invoke('initialize_onnx');
  },

  async removeBackground(inputPath: string, outputPath: string): Promise<string> {
    return invoke<string>('remove_background', { inputPath, outputPath });
  },

  async removeBackgroundBatch(characterId: string): Promise<void> {
    return invoke('remove_background_batch', { characterId });
  },
};

// --- ComfyUI API ---

export const comfyuiApi = {
  async checkConnection(endpoint?: string): Promise<boolean> {
    if (endpoint !== undefined) {
      return invoke<boolean>('check_comfyui_connection', { endpoint });
    }
    return invoke<boolean>('check_comfyui_connection');
  },

  async listWorkflows(): Promise<Workflow[]> {
    return invoke<Workflow[]>('list_workflows');
  },

  async createWorkflow(input: CreateWorkflow): Promise<Workflow> {
    return invoke<Workflow>('create_workflow', { input });
  },

  async importWorkflow(name: string, json: string): Promise<Workflow> {
    return invoke<Workflow>('import_workflow', { name, json });
  },

  async updateWorkflow(
    id: string,
    name?: string,
    description?: string,
    workflowJson?: string,
    isDefault?: boolean,
  ): Promise<Workflow> {
    return invoke<Workflow>('update_workflow', {
      id,
      name,
      description,
      workflowJson,
      isDefault,
    });
  },

  async deleteWorkflow(id: string): Promise<void> {
    return invoke('delete_workflow', { id });
  },

  async setDefaultWorkflow(id: string): Promise<void> {
    return invoke('set_default_workflow', { id });
  },

  async processBatch(
    characterId: string,
    workflowId: string,
    params: ProcessingParams,
  ): Promise<void> {
    return invoke('process_batch_comfyui', { characterId, workflowId, params });
  },
};

// --- Sprite API ---

export const spriteApi = {
  async list(characterId: string): Promise<Sprite[]> {
    return invoke<Sprite[]>('list_sprites', { characterId });
  },

  async updateAssignment(
    spriteId: string,
    direction: string,
    animation: string,
  ): Promise<void> {
    return invoke('update_sprite_assignment', { spriteId, direction, animation });
  },

  async normalizeBatch(
    characterId: string,
    tileWidth: number,
    tileHeight: number,
  ): Promise<void> {
    return invoke('normalize_batch', { characterId, tileWidth, tileHeight });
  },

  async generateSpritesheet(characterId: string): Promise<SpritesheetResult> {
    return invoke<SpritesheetResult>('generate_spritesheet', { characterId });
  },

  async exportForBevy(characterId: string, outputDir: string): Promise<BevyExportResult> {
    return invoke<BevyExportResult>('export_for_bevy', { characterId, outputDir });
  },
};

// --- Tauri Event Listener Helpers ---

export function onDownloadProgress(
  callback: (progress: DownloadProgress) => void,
): Promise<UnlistenFn> {
  return listen<DownloadProgress>('download-progress', (event) => {
    callback(event.payload);
  });
}

export function onProcessingProgress(
  callback: (progress: ProcessingProgress) => void,
): Promise<UnlistenFn> {
  return listen<ProcessingProgress>('processing-progress', (event) => {
    callback(event.payload);
  });
}

export function onComfyUIProgress(
  callback: (progress: ComfyUIProgress) => void,
): Promise<UnlistenFn> {
  return listen<ComfyUIProgress>('comfyui-progress', (event) => {
    callback(event.payload);
  });
}

export function onBgRemovalProgress(
  callback: (progress: BgRemovalProgress) => void,
): Promise<UnlistenFn> {
  return listen<BgRemovalProgress>('bg-removal-progress', (event) => {
    callback(event.payload);
  });
}
