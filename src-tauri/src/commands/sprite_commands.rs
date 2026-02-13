use crate::db;
use crate::error::AppError;
use crate::models::Sprite;
use crate::state::AppState;

/// キャラクターに紐付くスプライト一覧取得
#[tauri::command]
pub async fn list_sprites(
    state: tauri::State<'_, AppState>,
    character_id: String,
) -> Result<Vec<Sprite>, AppError> {
    let conn = state
        .db
        .lock()
        .map_err(|e| AppError::Internal(e.to_string()))?;

    let sprites = db::queries::sprite::get_sprites_by_character(&conn, &character_id)?;
    Ok(sprites)
}

/// スプライトの方向・アニメーション割り当て更新
#[tauri::command]
pub async fn update_sprite_assignment(
    state: tauri::State<'_, AppState>,
    sprite_id: String,
    direction: String,
    animation: String,
) -> Result<(), AppError> {
    let conn = state
        .db
        .lock()
        .map_err(|e| AppError::Internal(e.to_string()))?;

    db::queries::sprite::update_sprite_assignment(&conn, &sprite_id, &direction, &animation)?;
    Ok(())
}
