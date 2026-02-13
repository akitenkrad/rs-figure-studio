CREATE TABLE IF NOT EXISTS projects (
    id TEXT PRIMARY KEY,
    name TEXT NOT NULL,
    base_path TEXT NOT NULL,
    tile_width INTEGER NOT NULL DEFAULT 64,
    tile_height INTEGER NOT NULL DEFAULT 64,
    directions TEXT NOT NULL DEFAULT '["down","left","right","up"]',
    animations TEXT NOT NULL DEFAULT '[{"name":"idle","frame_count":2,"frame_duration_ms":300},{"name":"walk","frame_count":4,"frame_duration_ms":150},{"name":"attack","frame_count":4,"frame_duration_ms":100},{"name":"hit","frame_count":2,"frame_duration_ms":200}]',
    style_prompt TEXT NOT NULL DEFAULT 'painted miniature figurine, soft PBR material, warm studio lighting, tabletop game piece',
    negative_prompt TEXT NOT NULL DEFAULT 'background, shadow on floor, realistic human',
    controlnet_weight REAL NOT NULL DEFAULT 0.8,
    bevy_output_path TEXT,
    created_at TEXT NOT NULL DEFAULT (datetime('now')),
    updated_at TEXT NOT NULL DEFAULT (datetime('now'))
);

CREATE TABLE IF NOT EXISTS characters (
    id TEXT PRIMARY KEY,
    project_id TEXT NOT NULL REFERENCES projects(id) ON DELETE CASCADE,
    name TEXT NOT NULL,
    category TEXT NOT NULL DEFAULT 'enemy',
    custom_prompt TEXT,
    status TEXT NOT NULL DEFAULT 'draft',
    spritesheet_path TEXT,
    created_at TEXT NOT NULL DEFAULT (datetime('now')),
    updated_at TEXT NOT NULL DEFAULT (datetime('now'))
);

CREATE TABLE IF NOT EXISTS sprites (
    id TEXT PRIMARY KEY,
    character_id TEXT NOT NULL REFERENCES characters(id) ON DELETE CASCADE,
    direction TEXT NOT NULL,
    animation TEXT NOT NULL,
    frame_index INTEGER NOT NULL,
    raw_path TEXT,
    processed_path TEXT,
    final_path TEXT,
    status TEXT NOT NULL DEFAULT 'raw',
    created_at TEXT NOT NULL DEFAULT (datetime('now'))
);

CREATE TABLE IF NOT EXISTS comfyui_workflows (
    id TEXT PRIMARY KEY,
    project_id TEXT NOT NULL REFERENCES projects(id) ON DELETE CASCADE,
    name TEXT NOT NULL,
    workflow_json TEXT NOT NULL,
    is_default INTEGER NOT NULL DEFAULT 0,
    created_at TEXT NOT NULL DEFAULT (datetime('now'))
);

CREATE TABLE IF NOT EXISTS processing_jobs (
    id TEXT PRIMARY KEY,
    character_id TEXT NOT NULL REFERENCES characters(id) ON DELETE CASCADE,
    job_type TEXT NOT NULL,
    status TEXT NOT NULL DEFAULT 'pending',
    prompt_id TEXT,
    total_items INTEGER NOT NULL DEFAULT 0,
    completed_items INTEGER NOT NULL DEFAULT 0,
    error_message TEXT,
    created_at TEXT NOT NULL DEFAULT (datetime('now')),
    updated_at TEXT NOT NULL DEFAULT (datetime('now'))
);

CREATE TABLE IF NOT EXISTS app_settings (
    key TEXT PRIMARY KEY,
    value TEXT NOT NULL
);

INSERT OR IGNORE INTO app_settings (key, value) VALUES
    ('comfyui_endpoint', 'http://127.0.0.1:8188'),
    ('onnx_model_path', ''),
    ('onnx_model_downloaded', 'false'),
    ('theme', 'dark');

CREATE INDEX IF NOT EXISTS idx_characters_project ON characters(project_id);
CREATE INDEX IF NOT EXISTS idx_sprites_character ON sprites(character_id);
CREATE INDEX IF NOT EXISTS idx_sprites_status ON sprites(status);
CREATE INDEX IF NOT EXISTS idx_processing_jobs_character ON processing_jobs(character_id);
CREATE INDEX IF NOT EXISTS idx_processing_jobs_status ON processing_jobs(status);
