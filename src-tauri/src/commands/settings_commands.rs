use std::collections::HashMap;

use crate::db;
use crate::error::AppError;
use crate::state::AppState;

/// 全設定取得
#[tauri::command]
pub async fn get_settings(
    state: tauri::State<'_, AppState>,
) -> Result<HashMap<String, String>, AppError> {
    let conn = state
        .db
        .lock()
        .map_err(|e| AppError::Internal(e.to_string()))?;

    let settings = db::queries::settings::get_all_settings(&conn)?;
    Ok(settings)
}

/// 設定値更新
#[tauri::command]
pub async fn update_setting(
    state: tauri::State<'_, AppState>,
    key: String,
    value: String,
) -> Result<(), AppError> {
    if key.trim().is_empty() {
        return Err(AppError::Validation("Setting key is required".into()));
    }

    let conn = state
        .db
        .lock()
        .map_err(|e| AppError::Internal(e.to_string()))?;

    db::queries::settings::set_setting(&conn, &key, &value)?;
    Ok(())
}
