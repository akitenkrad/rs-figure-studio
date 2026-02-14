-- V3: Cloud API support, LoRA management, palette normalization

-- API key settings (inserted as app_settings rows)
INSERT OR IGNORE INTO app_settings (key, value) VALUES ('cloud_api_provider', 'pixellab');
INSERT OR IGNORE INTO app_settings (key, value) VALUES ('pixellab_api_key', '');
INSERT OR IGNORE INTO app_settings (key, value) VALUES ('fal_ai_api_key', '');
INSERT OR IGNORE INTO app_settings (key, value) VALUES ('replicate_api_key', '');

-- LoRA model management
CREATE TABLE IF NOT EXISTS lora_models (
    id TEXT PRIMARY KEY NOT NULL,
    name TEXT NOT NULL,
    file_path TEXT NOT NULL,
    description TEXT DEFAULT '',
    created_at TEXT NOT NULL DEFAULT (datetime('now'))
);

-- Character-LoRA assignment (many-to-many)
CREATE TABLE IF NOT EXISTS character_loras (
    character_id TEXT NOT NULL REFERENCES characters(id) ON DELETE CASCADE,
    lora_id TEXT NOT NULL REFERENCES lora_models(id) ON DELETE CASCADE,
    weight REAL NOT NULL DEFAULT 0.8,
    PRIMARY KEY (character_id, lora_id)
);
