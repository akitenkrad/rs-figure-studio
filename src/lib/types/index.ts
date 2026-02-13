// ============================================================
// Figurine Studio - TypeScript Type Definitions
// Matches Rust backend models
// ============================================================

// --- Project ---

export interface Project {
  id: string;
  name: string;
  base_path: string;
  tile_width: number;
  tile_height: number;
  directions: string;
  animations: string;
  style_prompt: string;
  negative_prompt: string;
  controlnet_weight: number;
  bevy_output_path: string | null;
  created_at: string;
  updated_at: string;
  character_count?: number;
}

export interface CreateProject {
  name: string;
  base_path: string;
  tile_width: number;
  tile_height: number;
  directions: string[];
  animations: AnimationDef[];
  style_prompt: string;
  negative_prompt: string;
  controlnet_weight: number;
  bevy_output_path?: string;
}

export interface UpdateProject {
  name?: string;
  base_path?: string;
  tile_width?: number;
  tile_height?: number;
  directions?: string[];
  animations?: AnimationDef[];
  style_prompt?: string;
  negative_prompt?: string;
  controlnet_weight?: number;
  bevy_output_path?: string;
}

export interface AnimationDef {
  name: string;
  frame_count: number;
  frame_duration_ms: number;
}

// --- Character ---

export interface Character {
  id: string;
  project_id: string;
  name: string;
  category: CharacterCategory;
  custom_prompt: string | null;
  status: CharacterStatus;
  spritesheet_path: string | null;
  created_at: string;
  updated_at: string;
}

export type CharacterCategory = 'player' | 'enemy' | 'npc';

export type CharacterStatus = 'draft' | 'importing' | 'processing' | 'complete';

export interface CreateCharacter {
  project_id: string;
  name: string;
  category: CharacterCategory;
  custom_prompt?: string;
}

export interface UpdateCharacter {
  name?: string;
  category?: CharacterCategory;
  custom_prompt?: string;
}

// --- Sprite ---

export interface Sprite {
  id: string;
  character_id: string;
  direction: string;
  animation: string;
  frame_index: number;
  raw_path: string | null;
  processed_path: string | null;
  final_path: string | null;
  status: SpriteStatus;
  created_at: string;
}

export type SpriteStatus = 'raw' | 'ai_processed' | 'bg_removed' | 'finalized';

// --- Processing ---

export interface ProcessingJob {
  id: string;
  character_id: string;
  job_type: 'comfyui' | 'bg_removal' | 'normalize' | 'spritesheet';
  status: 'pending' | 'running' | 'completed' | 'failed';
  prompt_id: string | null;
  total_items: number;
  completed_items: number;
  error_message: string | null;
  created_at: string;
  updated_at: string;
}

// --- Spritesheet ---

export interface DirectionDef {
  name: string;
  row: number;
}

export interface SpritesheetResult {
  path: string;
  columns: number;
  rows: number;
  tile_width: number;
  tile_height: number;
}

export interface BevyExportResult {
  spritesheet_path: string;
  metadata_path: string;
}

export interface ExportResult {
  png_path: string;
  meta_path: string;
}

export interface SpritesheetMeta {
  character: string;
  spritesheet: string;
  directions: string[];
  animations: AnimationDef[];
  version: string;
}

// --- Settings ---

export type ThemeMode = 'dark' | 'light';

export interface AppSettings {
  theme: ThemeMode;
  bevy_export_path: string;
  comfyui_endpoint: string;
}

// --- ONNX Model ---

export interface OnnxModelStatus {
  downloaded: boolean;
  model_path: string | null;
  initialized: boolean;
}

export type OnnxModelState =
  | 'not_downloaded'
  | 'downloading'
  | 'downloaded'
  | 'initializing'
  | 'ready'
  | 'error';

export interface BgRemovalProgress {
  current: number;
  total: number;
  sprite_id: string;
}

// --- Toast ---

export interface ToastItem {
  id: string;
  type: 'success' | 'error' | 'warning' | 'info';
  message: string;
  duration: number;
}

// --- Events ---

export interface DownloadProgress {
  downloaded: number;
  total: number;
  percentage: number;
}

export interface ProcessingProgress {
  stage: 'normalize' | 'bg_removal';
  current: number;
  total: number;
}

export interface ComfyUIProgress {
  current: number;
  total: number;
  status: string;
  current_file: string;
}

// --- Workflow ---

export interface Workflow {
  id: string;
  name: string;
  description: string;
  workflow_json: string;
  is_default: boolean;
  created_at: string;
  updated_at: string;
}

export interface CreateWorkflow {
  name: string;
  description: string;
  workflow_json: string;
  is_default?: boolean;
}

// --- Processing Params ---

export interface ProcessingParams {
  positive_prompt: string;
  negative_prompt: string;
  denoise_strength: number;
  controlnet_weight: number;
  cfg_scale: number;
  steps: number;
  seed?: number;
}

// --- Connection Status ---

export type ComfyUIConnectionStatus = 'disconnected' | 'connecting' | 'connected' | 'error';

// --- Breadcrumb ---

export interface BreadcrumbItem {
  label: string;
  href?: string;
}
