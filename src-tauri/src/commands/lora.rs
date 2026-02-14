use crate::db;
use crate::db::queries::lora::{CharacterLora, CreateLoraModel, LoraModel};
use crate::error::AppError;
use crate::state::AppState;

#[tauri::command]
pub async fn import_lora(
    state: tauri::State<'_, AppState>,
    name: String,
    file_path: String,
    description: Option<String>,
) -> Result<LoraModel, AppError> {
    // ファイル存在確認
    if !std::path::Path::new(&file_path).exists() {
        return Err(AppError::Io(format!(
            "LoRA ファイルが見つかりません: {}",
            file_path
        )));
    }

    let conn = state
        .db
        .lock()
        .map_err(|e| AppError::Internal(e.to_string()))?;

    let input = CreateLoraModel {
        name,
        file_path,
        description,
    };

    db::queries::lora::create_lora(&conn, &input).map_err(|e| e.into())
}

#[tauri::command]
pub async fn list_loras(
    state: tauri::State<'_, AppState>,
) -> Result<Vec<LoraModel>, AppError> {
    let conn = state
        .db
        .lock()
        .map_err(|e| AppError::Internal(e.to_string()))?;

    db::queries::lora::list_loras(&conn).map_err(|e| e.into())
}

#[tauri::command]
pub async fn delete_lora(
    state: tauri::State<'_, AppState>,
    id: String,
) -> Result<(), AppError> {
    let conn = state
        .db
        .lock()
        .map_err(|e| AppError::Internal(e.to_string()))?;

    db::queries::lora::delete_lora(&conn, &id).map_err(|e| e.into())
}

#[tauri::command]
pub async fn assign_lora(
    state: tauri::State<'_, AppState>,
    character_id: String,
    lora_id: String,
    weight: f64,
) -> Result<(), AppError> {
    let conn = state
        .db
        .lock()
        .map_err(|e| AppError::Internal(e.to_string()))?;

    db::queries::lora::assign_lora(&conn, &character_id, &lora_id, weight).map_err(|e| e.into())
}

#[tauri::command]
pub async fn unassign_lora(
    state: tauri::State<'_, AppState>,
    character_id: String,
    lora_id: String,
) -> Result<(), AppError> {
    let conn = state
        .db
        .lock()
        .map_err(|e| AppError::Internal(e.to_string()))?;

    db::queries::lora::unassign_lora(&conn, &character_id, &lora_id).map_err(|e| e.into())
}

#[tauri::command]
pub async fn list_character_loras(
    state: tauri::State<'_, AppState>,
    character_id: String,
) -> Result<Vec<CharacterLora>, AppError> {
    let conn = state
        .db
        .lock()
        .map_err(|e| AppError::Internal(e.to_string()))?;

    db::queries::lora::list_character_loras(&conn, &character_id).map_err(|e| e.into())
}
