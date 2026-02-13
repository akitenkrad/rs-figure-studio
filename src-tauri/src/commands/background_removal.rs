use tauri::Emitter;

use crate::db;
use crate::error::AppError;
use crate::models::{SpriteStatus, UpdateSpritePaths};
use crate::services::onnx_service::OnnxService;
use crate::state::AppState;

/// ONNX Runtime の初期化
///
/// モデルファイルから OnnxService を生成し，AppState に格納する．
/// フロントエンドからはダウンロード完了後に呼び出される．
#[tauri::command]
pub async fn initialize_onnx(
    state: tauri::State<'_, AppState>,
) -> Result<(), AppError> {
    let model_path = state.models_dir.join("u2net.onnx");

    if !model_path.exists() {
        return Err(AppError::Onnx("ONNX model not downloaded yet".into()));
    }

    let service = OnnxService::new(&model_path)
        .map_err(|e| AppError::Onnx(format!("Failed to initialize ONNX: {}", e)))?;

    let mut onnx = state
        .onnx_service
        .lock()
        .map_err(|e| AppError::Internal(e.to_string()))?;
    *onnx = Some(service);

    log::info!("ONNX service initialized successfully");
    Ok(())
}

/// 単一スプライトの背景除去
///
/// 指定されたスプライト ID の画像から背景を除去し，
/// bg_removed ディレクトリに出力する．
/// DB のスプライトステータスと processed_path を更新する．
#[tauri::command]
pub async fn remove_background(
    state: tauri::State<'_, AppState>,
    sprite_id: String,
) -> Result<String, AppError> {
    // DB からスプライト情報を取得
    let (sprite, character, project) = {
        let conn = state
            .db
            .lock()
            .map_err(|e| AppError::Internal(e.to_string()))?;

        let mut stmt = conn
            .prepare(
                "SELECT id, character_id, direction, animation, frame_index,
                        raw_path, processed_path, final_path, status, created_at
                 FROM sprites WHERE id = ?1",
            )
            .map_err(|e| AppError::Database(e.to_string()))?;

        let sprite = stmt
            .query_row(rusqlite::params![sprite_id], |row| {
                Ok(crate::models::Sprite {
                    id: row.get(0)?,
                    character_id: row.get(1)?,
                    direction: row.get(2)?,
                    animation: row.get(3)?,
                    frame_index: row.get(4)?,
                    raw_path: row.get(5)?,
                    processed_path: row.get(6)?,
                    final_path: row.get(7)?,
                    status: row.get(8)?,
                    created_at: row.get(9)?,
                })
            })
            .map_err(|_| AppError::NotFound(format!("Sprite not found: {}", sprite_id)))?;

        let character = db::queries::character::get_character(&conn, &sprite.character_id)?
            .ok_or_else(|| AppError::NotFound("Character not found".into()))?;

        let project = db::queries::project::get_project(&conn, &character.project_id)?
            .ok_or_else(|| AppError::Internal("Project not found".into()))?;

        (sprite, character, project)
    };

    // 入力画像パスを決定（processed_path > raw_path の優先順）
    let input_path = sprite
        .processed_path
        .as_ref()
        .or(sprite.raw_path.as_ref())
        .ok_or_else(|| AppError::Validation("Sprite has no image path".into()))?;

    // 出力パス: {base_path}/{character}/bg_removed/{filename}
    let src_path = std::path::Path::new(input_path);
    let filename = src_path
        .file_name()
        .ok_or_else(|| AppError::Internal("Invalid input path".into()))?;

    let output_dir = std::path::Path::new(&project.base_path)
        .join(&character.name)
        .join("bg_removed");
    std::fs::create_dir_all(&output_dir)
        .map_err(|e| AppError::Io(format!("Failed to create bg_removed directory: {}", e)))?;

    let output_path = output_dir.join(filename);
    let output_path_str = output_path.to_string_lossy().to_string();

    // ONNX サービスで背景除去（mutable access が必要）
    {
        let mut onnx = state
            .onnx_service
            .lock()
            .map_err(|e| AppError::Internal(e.to_string()))?;

        let service = onnx
            .as_mut()
            .ok_or_else(|| AppError::Onnx("ONNX service not initialized".into()))?;

        service
            .remove_background(input_path, &output_path_str)
            .map_err(|e| AppError::Onnx(format!("Background removal failed: {}", e)))?;
    }

    // DB 更新: processed_path を設定し status を bg_removed に
    {
        let conn = state
            .db
            .lock()
            .map_err(|e| AppError::Internal(e.to_string()))?;

        db::queries::sprite::update_sprite_paths(
            &conn,
            &sprite_id,
            &UpdateSpritePaths {
                raw_path: None,
                processed_path: Some(output_path_str.clone()),
                final_path: None,
            },
        )?;
        db::queries::sprite::update_sprite_status(
            &conn,
            &sprite_id,
            &SpriteStatus::BgRemoved,
        )?;
    }

    Ok(output_path_str)
}

/// バッチ背景除去
///
/// 指定キャラクターの全スプライトを順次背景除去する．
/// `bg-removal-progress` イベントで進捗を通知する．
/// 個別の失敗はスキップし，処理を継続する．
#[tauri::command]
pub async fn remove_background_batch(
    state: tauri::State<'_, AppState>,
    app: tauri::AppHandle,
    character_id: String,
) -> Result<Vec<String>, AppError> {
    // キャラクターとプロジェクト情報，スプライト一覧を取得
    let (sprites, character, project) = {
        let conn = state
            .db
            .lock()
            .map_err(|e| AppError::Internal(e.to_string()))?;

        let character = db::queries::character::get_character(&conn, &character_id)?
            .ok_or_else(|| AppError::NotFound("Character not found".into()))?;

        let project = db::queries::project::get_project(&conn, &character.project_id)?
            .ok_or_else(|| AppError::Internal("Project not found".into()))?;

        let sprites = db::queries::sprite::get_sprites_by_character(&conn, &character_id)?;

        (sprites, character, project)
    };

    // 処理対象: raw_path または processed_path が存在するスプライト
    let target_sprites: Vec<_> = sprites
        .iter()
        .filter(|s| s.raw_path.is_some() || s.processed_path.is_some())
        .collect();

    let total = target_sprites.len() as u32;

    let output_dir = std::path::Path::new(&project.base_path)
        .join(&character.name)
        .join("bg_removed");
    std::fs::create_dir_all(&output_dir)
        .map_err(|e| AppError::Io(format!("Failed to create bg_removed directory: {}", e)))?;

    let mut results = Vec::with_capacity(target_sprites.len());

    for (i, sprite) in target_sprites.iter().enumerate() {
        // 入力画像パスを決定
        let input_path = match sprite.processed_path.as_ref().or(sprite.raw_path.as_ref()) {
            Some(path) => path.clone(),
            None => continue,
        };

        let src_path = std::path::Path::new(&input_path);
        let filename = match src_path.file_name() {
            Some(f) => f,
            None => {
                log::warn!("Invalid input path for sprite {}: {}", sprite.id, input_path);
                continue;
            }
        };

        let output_path = output_dir.join(filename);
        let output_path_str = output_path.to_string_lossy().to_string();

        // ONNX サービスで背景除去（個別失敗はスキップ）
        let remove_result = {
            let mut onnx = state
                .onnx_service
                .lock()
                .map_err(|e| AppError::Internal(e.to_string()))?;

            let service = onnx
                .as_mut()
                .ok_or_else(|| AppError::Onnx("ONNX service not initialized".into()))?;

            service.remove_background(&input_path, &output_path_str)
        };

        match remove_result {
            Ok(()) => {
                // DB 更新
                let conn = state
                    .db
                    .lock()
                    .map_err(|e| AppError::Internal(e.to_string()))?;

                let _ = db::queries::sprite::update_sprite_paths(
                    &conn,
                    &sprite.id,
                    &UpdateSpritePaths {
                        raw_path: None,
                        processed_path: Some(output_path_str.clone()),
                        final_path: None,
                    },
                );
                let _ = db::queries::sprite::update_sprite_status(
                    &conn,
                    &sprite.id,
                    &SpriteStatus::BgRemoved,
                );

                results.push(output_path_str);
            }
            Err(e) => {
                log::warn!(
                    "Background removal failed for sprite {}: {}",
                    sprite.id,
                    e
                );
            }
        }

        // プログレスイベント送信
        let _ = app.emit(
            "bg-removal-progress",
            serde_json::json!({
                "current": i + 1,
                "total": total,
                "sprite_id": sprite.id,
            }),
        );
    }

    Ok(results)
}
