use crate::db;
use crate::error::AppError;
use crate::models::{Character, CreateCharacter, CreateSprite, Sprite, UpdateCharacter};
use crate::state::AppState;

#[tauri::command]
pub async fn create_character(
    state: tauri::State<'_, AppState>,
    input: CreateCharacter,
) -> Result<Character, AppError> {
    let conn = state
        .db
        .lock()
        .map_err(|e| AppError::Internal(e.to_string()))?;

    // バリデーション
    if input.name.trim().is_empty() {
        return Err(AppError::Validation("Character name is required".into()));
    }

    // プロジェクト存在確認
    db::queries::project::get_project(&conn, &input.project_id)?
        .ok_or_else(|| AppError::NotFound(format!("Project not found: {}", input.project_id)))?;

    let character = db::queries::character::create_character(&conn, &input)?;

    // キャラクターディレクトリ作成
    let project = db::queries::project::get_project(&conn, &input.project_id)?.unwrap();
    let char_dir = std::path::Path::new(&project.base_path).join(&character.name);
    for stage in &[
        "raw",
        "processed",
        "bg_removed",
        "normalized",
        "spritesheet",
    ] {
        std::fs::create_dir_all(char_dir.join(stage))
            .map_err(|e| AppError::Io(format!("Failed to create directory: {}", e)))?;
    }

    Ok(character)
}

#[tauri::command]
pub async fn get_character(
    state: tauri::State<'_, AppState>,
    id: String,
) -> Result<Character, AppError> {
    let conn = state
        .db
        .lock()
        .map_err(|e| AppError::Internal(e.to_string()))?;

    db::queries::character::get_character(&conn, &id)?
        .ok_or_else(|| AppError::NotFound(format!("Character not found: {}", id)))
}

#[tauri::command]
pub async fn list_characters(
    state: tauri::State<'_, AppState>,
    project_id: String,
) -> Result<Vec<Character>, AppError> {
    let conn = state
        .db
        .lock()
        .map_err(|e| AppError::Internal(e.to_string()))?;
    let characters = db::queries::character::list_characters_by_project(&conn, &project_id)?;
    Ok(characters)
}

#[tauri::command]
pub async fn update_character(
    state: tauri::State<'_, AppState>,
    id: String,
    input: UpdateCharacter,
) -> Result<Character, AppError> {
    let conn = state
        .db
        .lock()
        .map_err(|e| AppError::Internal(e.to_string()))?;
    let character = db::queries::character::update_character(&conn, &id, &input)?;
    Ok(character)
}

#[tauri::command]
pub async fn delete_character(
    state: tauri::State<'_, AppState>,
    id: String,
) -> Result<(), AppError> {
    let conn = state
        .db
        .lock()
        .map_err(|e| AppError::Internal(e.to_string()))?;

    let character = db::queries::character::get_character(&conn, &id)?
        .ok_or_else(|| AppError::NotFound(format!("Character not found: {}", id)))?;

    let project = db::queries::project::get_project(&conn, &character.project_id)?
        .ok_or_else(|| AppError::Internal("Orphaned character".into()))?;

    db::queries::character::delete_character(&conn, &id)?;

    // キャラクターディレクトリ削除（ベストエフォート）
    let char_dir = std::path::Path::new(&project.base_path).join(&character.name);
    if char_dir.exists() {
        if let Err(e) = std::fs::remove_dir_all(&char_dir) {
            log::warn!("Failed to remove character directory: {}", e);
        }
    }

    Ok(())
}

/// スプライト画像インポート
/// ファイルを {project_base}/{character}/raw/ にコピーし，DB登録する
#[tauri::command]
pub async fn import_sprites(
    state: tauri::State<'_, AppState>,
    character_id: String,
    file_paths: Vec<String>,
) -> Result<Vec<Sprite>, AppError> {
    let conn = state
        .db
        .lock()
        .map_err(|e| AppError::Internal(e.to_string()))?;

    let character = db::queries::character::get_character(&conn, &character_id)?
        .ok_or_else(|| AppError::NotFound(format!("Character not found: {}", character_id)))?;

    let project = db::queries::project::get_project(&conn, &character.project_id)?
        .ok_or_else(|| AppError::Internal("Project not found".into()))?;

    // ステータス更新
    db::queries::character::update_character_status(&conn, &character_id, "importing")?;

    // raw ディレクトリ
    let raw_dir = std::path::Path::new(&project.base_path)
        .join(&character.name)
        .join("raw");
    std::fs::create_dir_all(&raw_dir)
        .map_err(|e| AppError::Io(format!("Failed to create raw directory: {}", e)))?;

    let mut create_inputs = Vec::new();

    for file_path in &file_paths {
        let src = std::path::Path::new(file_path);
        let filename = src
            .file_stem()
            .and_then(|s| s.to_str())
            .ok_or_else(|| AppError::Validation("Invalid filename".into()))?;

        // ファイル名パース: {character}_{direction}_{animation}_{frame}
        let parts: Vec<&str> = filename.splitn(4, '_').collect();
        if parts.len() < 4 {
            return Err(AppError::Validation(format!(
                "Invalid filename format: {}. Expected: {{character}}_{{direction}}_{{animation}}_{{frame}}.png",
                filename
            )));
        }

        let direction = parts[1].to_string();
        let animation = parts[2].to_string();
        let frame_index: i32 = parts[3]
            .parse()
            .map_err(|_| AppError::Validation(format!("Invalid frame index: {}", parts[3])))?;

        // ファイルコピー
        let dest = raw_dir.join(src.file_name().unwrap());
        std::fs::copy(src, &dest)
            .map_err(|e| AppError::Io(format!("Failed to copy file: {}", e)))?;

        create_inputs.push(CreateSprite {
            character_id: character_id.clone(),
            direction,
            animation,
            frame_index,
            raw_path: Some(dest.to_string_lossy().to_string()),
        });
    }

    let sprites = db::queries::sprite::create_sprites_batch(&conn, &create_inputs)?;

    // ステータスを元に戻す
    db::queries::character::update_character_status(&conn, &character_id, "draft")?;

    Ok(sprites)
}
