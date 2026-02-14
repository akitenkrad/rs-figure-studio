use serde::{Deserialize, Serialize};
use tauri::Emitter;

use crate::db;
use crate::error::AppError;
use crate::models::{CreateSprite, SpriteStatus, UpdateSpritePaths};
use crate::services::cloud_api_backend::CloudApiBackend;
use crate::services::cloud_api_client::CloudApiProvider;
use crate::services::generation_backend::{
    AnimationParams, ComfyUIBackend, ConceptParams, DirectionParams, GenerationBackend,
    GenerationProgress, PixelArtConversionParams,
};
use crate::state::AppState;

/// 設定に基づいてバックエンドを選択する
fn create_backend(
    conn: &rusqlite::Connection,
) -> Result<Box<dyn GenerationBackend>, AppError> {
    let backend_type = db::queries::settings::get_setting(conn, "generation_backend")?
        .unwrap_or_else(|| "comfyui".to_string());

    match backend_type.as_str() {
        "cloud_api" => {
            let provider_str = db::queries::settings::get_setting(conn, "cloud_api_provider")?
                .unwrap_or_else(|| "pixellab".to_string());
            let provider = CloudApiProvider::from_str(&provider_str);

            let key_name = match provider {
                CloudApiProvider::Pixellab => "pixellab_api_key",
                CloudApiProvider::FalAi => "fal_ai_api_key",
                CloudApiProvider::Replicate => "replicate_api_key",
            };

            let api_key = db::queries::settings::get_setting(conn, key_name)?
                .unwrap_or_default();

            if api_key.is_empty() {
                return Err(AppError::CloudApi(format!(
                    "{} の API キーが設定されていません",
                    provider_str
                )));
            }

            Ok(Box::new(CloudApiBackend::new(provider, api_key)))
        }
        _ => {
            let endpoint = db::queries::settings::get_setting(conn, "comfyui_endpoint")?
                .unwrap_or_else(|| "http://127.0.0.1:8188".to_string());
            Ok(Box::new(ComfyUIBackend::new(&endpoint)))
        }
    }
}

// ============================================================
// コンセプト生成
// ============================================================

/// テキストプロンプトからコンセプト画像を生成
///
/// ComfyUI ワークフローを使い，キャラクターのコンセプト画像を
/// 複数候補（num_candidates 枚）生成する．
/// 結果は `{base_path}/{character_name}/concepts/` に保存される．
#[tauri::command]
pub async fn generate_concept(
    state: tauri::State<'_, AppState>,
    app: tauri::AppHandle,
    character_id: String,
    params: ConceptParams,
) -> Result<Vec<String>, AppError> {
    // キャラクター・プロジェクト情報取得
    let (character, project) = {
        let conn = state
            .db
            .lock()
            .map_err(|e| AppError::Internal(e.to_string()))?;

        let character = db::queries::character::get_character(&conn, &character_id)?
            .ok_or_else(|| AppError::NotFound("キャラクターが見つかりません".into()))?;
        let project = db::queries::project::get_project(&conn, &character.project_id)?
            .ok_or_else(|| AppError::Internal("プロジェクトが見つかりません".into()))?;

        (character, project)
    };

    // 出力ディレクトリ
    let output_dir = std::path::Path::new(&project.base_path)
        .join(&character.name)
        .join("concepts");

    // バックエンド選択
    let backend = {
        let conn = state
            .db
            .lock()
            .map_err(|e| AppError::Internal(e.to_string()))?;
        create_backend(&conn)?
    };

    // プログレスコールバック
    let app_handle = app.clone();
    let progress_cb = Box::new(move |progress: GenerationProgress| {
        let _ = app_handle.emit("generation-progress", &progress);
    });

    let generated = backend
        .generate_concept(&params, &output_dir, progress_cb)
        .await?;

    let paths: Vec<String> = generated
        .iter()
        .map(|g| g.path.to_string_lossy().to_string())
        .collect();

    log::info!(
        "コンセプト生成完了: {} 枚 (character: {})",
        paths.len(),
        character.name
    );

    Ok(paths)
}

// ============================================================
// コンセプトアート生成（Step 1a: Text → Concept Art）
// ============================================================

/// テキストプロンプトからコンセプトアート画像を生成
///
/// 手描き風・詳細イラストスタイルのコンセプトアートを生成する．
/// 結果は `{base_path}/{character_name}/concept_art/` に保存される．
#[tauri::command]
pub async fn generate_concept_art(
    state: tauri::State<'_, AppState>,
    app: tauri::AppHandle,
    character_id: String,
    params: ConceptParams,
) -> Result<Vec<String>, AppError> {
    // キャラクター・プロジェクト情報取得
    let (character, project) = {
        let conn = state
            .db
            .lock()
            .map_err(|e| AppError::Internal(e.to_string()))?;

        let character = db::queries::character::get_character(&conn, &character_id)?
            .ok_or_else(|| AppError::NotFound("キャラクターが見つかりません".into()))?;
        let project = db::queries::project::get_project(&conn, &character.project_id)?
            .ok_or_else(|| AppError::Internal("プロジェクトが見つかりません".into()))?;

        (character, project)
    };

    // 出力ディレクトリ
    let output_dir = std::path::Path::new(&project.base_path)
        .join(&character.name)
        .join("concept_art");

    // バックエンド選択
    let backend = {
        let conn = state
            .db
            .lock()
            .map_err(|e| AppError::Internal(e.to_string()))?;
        create_backend(&conn)?
    };

    // プログレスコールバック
    let app_handle = app.clone();
    let progress_cb = Box::new(move |progress: GenerationProgress| {
        let _ = app_handle.emit("generation-progress", &progress);
    });

    let generated = backend
        .generate_concept_art(&params, &output_dir, progress_cb)
        .await?;

    let paths: Vec<String> = generated
        .iter()
        .map(|g| g.path.to_string_lossy().to_string())
        .collect();

    log::info!(
        "コンセプトアート生成完了: {} 枚 (character: {})",
        paths.len(),
        character.name
    );

    Ok(paths)
}

// ============================================================
// ピクセルアート変換（Step 1b: Concept Art → Pixel Art）
// ============================================================

/// コンセプトアート画像をピクセルアートに変換
///
/// img2img でコンセプトアートをピクセルアートスタイルに変換する．
/// デノイズ強度で変換の度合いを制御可能．
/// 結果は `{base_path}/{character_name}/concepts/` に保存される．
#[tauri::command]
pub async fn convert_to_pixel_art(
    state: tauri::State<'_, AppState>,
    app: tauri::AppHandle,
    character_id: String,
    concept_art_path: String,
    params: PixelArtConversionParams,
) -> Result<Vec<String>, AppError> {
    // キャラクター・プロジェクト情報取得
    let (character, project) = {
        let conn = state
            .db
            .lock()
            .map_err(|e| AppError::Internal(e.to_string()))?;

        let character = db::queries::character::get_character(&conn, &character_id)?
            .ok_or_else(|| AppError::NotFound("キャラクターが見つかりません".into()))?;
        let project = db::queries::project::get_project(&conn, &character.project_id)?
            .ok_or_else(|| AppError::Internal("プロジェクトが見つかりません".into()))?;

        (character, project)
    };

    // 出力ディレクトリ（既存の concepts/ と同じ場所）
    let output_dir = std::path::Path::new(&project.base_path)
        .join(&character.name)
        .join("concepts");

    // バックエンド選択
    let backend = {
        let conn = state
            .db
            .lock()
            .map_err(|e| AppError::Internal(e.to_string()))?;
        create_backend(&conn)?
    };

    // プログレスコールバック
    let app_handle = app.clone();
    let progress_cb = Box::new(move |progress: GenerationProgress| {
        let _ = app_handle.emit("generation-progress", &progress);
    });

    let concept_path = std::path::Path::new(&concept_art_path);
    let generated = backend
        .convert_to_pixel_art(concept_path, &params, &output_dir, progress_cb)
        .await?;

    let paths: Vec<String> = generated
        .iter()
        .map(|g| g.path.to_string_lossy().to_string())
        .collect();

    log::info!(
        "ピクセルアート変換完了: {} 枚 (character: {})",
        paths.len(),
        character.name
    );

    Ok(paths)
}

// ============================================================
// 方向展開
// ============================================================

/// コンセプト画像から4方向ポーズを生成
///
/// IP-Adapter FaceID でキャラクター一貫性を保ちつつ，
/// 各方向のベースポーズ画像を生成する．
/// 結果は `{base_path}/{character_name}/generated/` に保存され，
/// `sprites` テーブルに `generated` ステータスで登録される．
#[tauri::command]
pub async fn generate_directions(
    state: tauri::State<'_, AppState>,
    app: tauri::AppHandle,
    character_id: String,
    concept_image_path: String,
    params: DirectionParams,
) -> Result<Vec<String>, AppError> {
    let (character, project) = {
        let conn = state
            .db
            .lock()
            .map_err(|e| AppError::Internal(e.to_string()))?;

        let character = db::queries::character::get_character(&conn, &character_id)?
            .ok_or_else(|| AppError::NotFound("キャラクターが見つかりません".into()))?;
        let project = db::queries::project::get_project(&conn, &character.project_id)?
            .ok_or_else(|| AppError::Internal("プロジェクトが見つかりません".into()))?;

        // concept_image_path をキャラクターに保存
        let _ = conn.execute(
            "UPDATE characters SET concept_image_path = ?1, updated_at = ?2 WHERE id = ?3",
            rusqlite::params![
                concept_image_path,
                chrono::Utc::now().to_rfc3339(),
                character_id
            ],
        );

        (character, project)
    };

    let output_dir = std::path::Path::new(&project.base_path)
        .join(&character.name)
        .join("generated");

    let backend = {
        let conn = state
            .db
            .lock()
            .map_err(|e| AppError::Internal(e.to_string()))?;
        create_backend(&conn)?
    };

    let app_handle = app.clone();
    let progress_cb = Box::new(move |progress: GenerationProgress| {
        let _ = app_handle.emit("generation-progress", &progress);
    });

    let concept_path = std::path::Path::new(&concept_image_path);
    let generated = backend
        .generate_directions(concept_path, &params, &output_dir, progress_cb)
        .await?;

    // sprites テーブルに generated ステータスで登録
    let mut paths = Vec::new();
    {
        let conn = state
            .db
            .lock()
            .map_err(|e| AppError::Internal(e.to_string()))?;

        for img in &generated {
            let direction = img.direction.as_deref().unwrap_or("unknown");
            let input = CreateSprite {
                character_id: character_id.clone(),
                direction: direction.to_string(),
                animation: "base".to_string(),
                frame_index: 0,
                raw_path: Some(img.path.to_string_lossy().to_string()),
            };

            // generated ステータスで挿入するため，raw で作成してから更新
            match db::queries::sprite::create_sprite(&conn, &input) {
                Ok(sprite) => {
                    let _ = db::queries::sprite::update_sprite_status(
                        &conn,
                        &sprite.id,
                        &SpriteStatus::Generated,
                    );
                    paths.push(img.path.to_string_lossy().to_string());
                }
                Err(e) => {
                    log::warn!("スプライトレコード作成に失敗: {}", e);
                }
            }
        }
    }

    log::info!(
        "方向展開完了: {} 方向 (character: {})",
        paths.len(),
        character.name
    );

    Ok(paths)
}

// ============================================================
// アニメーション展開
// ============================================================

/// 方向ベースポーズからアニメーションフレームを生成
///
/// IP-Adapter + seed 固定でキャラクター一貫性を保ちつつ，
/// 各フレームを生成する．
/// 結果は `{base_path}/{character_name}/generated/` に保存され，
/// `sprites` テーブルに `generated` ステータスで登録される．
#[tauri::command]
pub async fn generate_animation_frames(
    state: tauri::State<'_, AppState>,
    app: tauri::AppHandle,
    character_id: String,
    base_pose_path: String,
    direction: String,
    params: AnimationParams,
) -> Result<Vec<String>, AppError> {
    let (character, project) = {
        let conn = state
            .db
            .lock()
            .map_err(|e| AppError::Internal(e.to_string()))?;

        let character = db::queries::character::get_character(&conn, &character_id)?
            .ok_or_else(|| AppError::NotFound("キャラクターが見つかりません".into()))?;
        let project = db::queries::project::get_project(&conn, &character.project_id)?
            .ok_or_else(|| AppError::Internal("プロジェクトが見つかりません".into()))?;

        (character, project)
    };

    let output_dir = std::path::Path::new(&project.base_path)
        .join(&character.name)
        .join("generated");

    let backend = {
        let conn = state
            .db
            .lock()
            .map_err(|e| AppError::Internal(e.to_string()))?;
        create_backend(&conn)?
    };

    let app_handle = app.clone();
    let progress_cb = Box::new(move |progress: GenerationProgress| {
        let _ = app_handle.emit("generation-progress", &progress);
    });

    let base_path = std::path::Path::new(&base_pose_path);
    let generated = backend
        .generate_animation_frames(base_path, &direction, &params, &output_dir, progress_cb)
        .await?;

    // sprites テーブルに generated ステータスで登録
    let mut paths = Vec::new();
    {
        let conn = state
            .db
            .lock()
            .map_err(|e| AppError::Internal(e.to_string()))?;

        for img in &generated {
            let input = CreateSprite {
                character_id: character_id.clone(),
                direction: direction.clone(),
                animation: params.animation_name.clone(),
                frame_index: img.frame_index.unwrap_or(0) as i32,
                raw_path: Some(img.path.to_string_lossy().to_string()),
            };

            match db::queries::sprite::create_sprite(&conn, &input) {
                Ok(sprite) => {
                    let _ = db::queries::sprite::update_sprite_status(
                        &conn,
                        &sprite.id,
                        &SpriteStatus::Generated,
                    );
                    paths.push(img.path.to_string_lossy().to_string());
                }
                Err(e) => {
                    log::warn!("スプライトレコード作成に失敗: {}", e);
                }
            }
        }
    }

    log::info!(
        "アニメーション展開完了: {} {} - {} フレーム (character: {})",
        direction,
        params.animation_name,
        paths.len(),
        character.name
    );

    Ok(paths)
}

// ============================================================
// パイプライン合流
// ============================================================

/// AI 生成スプライトを raw ステータスに昇格
///
/// `generated/` ディレクトリから `raw/` ディレクトリへファイルをコピーし，
/// DB のステータスを `generated` → `raw` に更新する．
/// これにより既存パイプライン（BG Removal → Normalize → Spritesheet）に合流する．
#[tauri::command]
pub async fn promote_generated_to_raw(
    state: tauri::State<'_, AppState>,
    sprite_ids: Vec<String>,
) -> Result<Vec<String>, AppError> {
    let conn = state
        .db
        .lock()
        .map_err(|e| AppError::Internal(e.to_string()))?;

    let mut promoted_ids = Vec::new();

    for sprite_id in &sprite_ids {
        // スプライト取得
        let sprite = {
            let mut stmt = conn
                .prepare(
                    "SELECT id, character_id, direction, animation, frame_index,
                            raw_path, processed_path, final_path, status, created_at
                     FROM sprites WHERE id = ?1",
                )
                .map_err(|e| AppError::Database(e.to_string()))?;

            stmt.query_row(rusqlite::params![sprite_id], |row| {
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
            .map_err(|e| match e {
                rusqlite::Error::QueryReturnedNoRows => {
                    AppError::NotFound(format!("スプライトが見つかりません: {}", sprite_id))
                }
                _ => AppError::Database(e.to_string()),
            })?
        };

        // ステータス確認
        if sprite.status != SpriteStatus::Generated.as_str() {
            return Err(AppError::Validation(format!(
                "スプライト {} は 'generated' ステータスではありません (現在: {})",
                sprite_id, sprite.status
            )));
        }

        // キャラクター・プロジェクト情報取得
        let character = db::queries::character::get_character(&conn, &sprite.character_id)?
            .ok_or_else(|| {
                AppError::Internal(format!(
                    "キャラクターが見つかりません: {}",
                    sprite.character_id
                ))
            })?;
        let project = db::queries::project::get_project(&conn, &character.project_id)?
            .ok_or_else(|| {
                AppError::Internal(format!(
                    "プロジェクトが見つかりません: {}",
                    character.project_id
                ))
            })?;

        let base = std::path::Path::new(&project.base_path).join(&character.name);
        let raw_dir = base.join("raw");
        std::fs::create_dir_all(&raw_dir)
            .map_err(|e| AppError::Io(format!("raw ディレクトリ作成に失敗: {}", e)))?;

        // generated ファイルのパスを取得
        let generated_path = sprite.raw_path.as_ref().ok_or_else(|| {
            AppError::Validation(format!(
                "スプライト {} にファイルパスがありません",
                sprite_id
            ))
        })?;

        let src = std::path::Path::new(generated_path);
        if !src.exists() {
            return Err(AppError::Io(format!(
                "生成ファイルが見つかりません: {}",
                generated_path
            )));
        }

        let filename = src
            .file_name()
            .ok_or_else(|| AppError::Internal("無効なファイルパス".into()))?;
        let dest = raw_dir.join(filename);

        // ファイルコピー
        std::fs::copy(src, &dest)
            .map_err(|e| AppError::Io(format!("ファイルコピーに失敗: {}", e)))?;

        let new_raw_path = dest.to_string_lossy().to_string();

        // DB 更新: raw_path とステータス
        db::queries::sprite::update_sprite_paths(
            &conn,
            sprite_id,
            &UpdateSpritePaths {
                raw_path: Some(new_raw_path),
                processed_path: None,
                final_path: None,
            },
        )
        .map_err(|e| AppError::Database(e.to_string()))?;

        db::queries::sprite::update_sprite_status(&conn, sprite_id, &SpriteStatus::Raw)
            .map_err(|e| AppError::Database(e.to_string()))?;

        promoted_ids.push(sprite_id.clone());
    }

    log::info!(
        "パイプライン合流完了: {} スプライトを promoted",
        promoted_ids.len()
    );

    Ok(promoted_ids)
}

// ============================================================
// クラウド API 接続テスト
// ============================================================

/// クラウド API の接続をテスト
#[tauri::command]
pub async fn test_cloud_api_connection(
    _state: tauri::State<'_, AppState>,
    provider: String,
    api_key: String,
) -> Result<bool, AppError> {
    let provider = CloudApiProvider::from_str(&provider);
    let client = crate::services::cloud_api_client::CloudApiClient::new();
    client.test_connection(&provider, &api_key).await
}

// ============================================================
// ステージクリア（やり直し）
// ============================================================

/// 指定ステージの生成結果をクリアする
///
/// ステージに応じてファイルを削除し，DB レコードも更新する．
/// - `concept_art` → concept_art/ ディレクトリ内のファイルを削除
/// - `concept` → concepts/ ディレクトリ内のファイルを削除し，character.concept_image_path を NULL に
/// - `direction` → generated/ 内の方向ベース画像と，sprites テーブルの該当レコードを削除
/// - `animation` → generated/ 内のアニメーションフレームと，sprites テーブルの該当レコードを削除
#[tauri::command]
pub async fn clear_generation_stage(
    state: tauri::State<'_, AppState>,
    character_id: String,
    stage: String,
) -> Result<(), AppError> {
    let (character, project) = {
        let conn = state
            .db
            .lock()
            .map_err(|e| AppError::Internal(e.to_string()))?;

        let character = db::queries::character::get_character(&conn, &character_id)?
            .ok_or_else(|| AppError::NotFound("キャラクターが見つかりません".into()))?;
        let project = db::queries::project::get_project(&conn, &character.project_id)?
            .ok_or_else(|| AppError::Internal("プロジェクトが見つかりません".into()))?;

        (character, project)
    };

    let base = std::path::Path::new(&project.base_path).join(&character.name);

    match stage.as_str() {
        "concept_art" => {
            // concept_art/ ディレクトリ内のファイルをすべて削除
            let dir = base.join("concept_art");
            if dir.exists() {
                if let Ok(entries) = std::fs::read_dir(&dir) {
                    for entry in entries.filter_map(|e| e.ok()) {
                        let path = entry.path();
                        if path.is_file() {
                            let _ = std::fs::remove_file(&path);
                        }
                    }
                }
            }
        }
        "concept" => {
            // concepts/ ディレクトリ内のファイルをすべて削除
            let dir = base.join("concepts");
            if dir.exists() {
                if let Ok(entries) = std::fs::read_dir(&dir) {
                    for entry in entries.filter_map(|e| e.ok()) {
                        let path = entry.path();
                        if path.is_file() {
                            let _ = std::fs::remove_file(&path);
                        }
                    }
                }
            }

            // character.concept_image_path を NULL に設定
            let conn = state
                .db
                .lock()
                .map_err(|e| AppError::Internal(e.to_string()))?;
            conn.execute(
                "UPDATE characters SET concept_image_path = NULL, updated_at = ?1 WHERE id = ?2",
                rusqlite::params![chrono::Utc::now().to_rfc3339(), character_id],
            )
            .map_err(|e| AppError::Database(e.to_string()))?;
        }
        "direction" => {
            // generated/ 内の方向ベース画像（*_base.png）を削除
            let dir = base.join("generated");
            if dir.exists() {
                if let Ok(entries) = std::fs::read_dir(&dir) {
                    for entry in entries.filter_map(|e| e.ok()) {
                        let path = entry.path();
                        if let Some(name) = path.file_name().and_then(|n| n.to_str()) {
                            if name.ends_with("_base.png") {
                                let _ = std::fs::remove_file(&path);
                            }
                        }
                    }
                }
            }

            // sprites テーブルから generated ステータスで animation="base" のレコードを削除
            let conn = state
                .db
                .lock()
                .map_err(|e| AppError::Internal(e.to_string()))?;
            conn.execute(
                "DELETE FROM sprites WHERE character_id = ?1 AND status = 'generated' AND animation = 'base'",
                rusqlite::params![character_id],
            )
            .map_err(|e| AppError::Database(e.to_string()))?;
        }
        "animation" => {
            // generated/ 内のアニメーションフレーム（*_frame_*.png）を削除
            let dir = base.join("generated");
            if dir.exists() {
                if let Ok(entries) = std::fs::read_dir(&dir) {
                    for entry in entries.filter_map(|e| e.ok()) {
                        let path = entry.path();
                        if let Some(name) = path.file_name().and_then(|n| n.to_str()) {
                            if name.contains("_frame_") {
                                let _ = std::fs::remove_file(&path);
                            }
                        }
                    }
                }
            }

            // sprites テーブルから generated ステータスで animation!="base" のレコードを削除
            let conn = state
                .db
                .lock()
                .map_err(|e| AppError::Internal(e.to_string()))?;
            conn.execute(
                "DELETE FROM sprites WHERE character_id = ?1 AND status = 'generated' AND animation != 'base'",
                rusqlite::params![character_id],
            )
            .map_err(|e| AppError::Database(e.to_string()))?;
        }
        _ => {
            return Err(AppError::Validation(format!(
                "不正なステージ名です: {}",
                stage
            )));
        }
    }

    log::info!(
        "ステージクリア完了: stage={}, character={}",
        stage,
        character.name
    );

    Ok(())
}

// ============================================================
// 生成パイプライン状態復元
// ============================================================

/// 生成パイプラインの復元状態
#[derive(Debug, Clone, Serialize)]
pub struct GenerationState {
    /// concept_art/ ディレクトリ内の画像パス一覧
    pub concept_art_images: Vec<String>,
    /// 選択済みコンセプトアート画像パス
    pub selected_concept_art_path: Option<String>,
    /// concepts/ ディレクトリ内の画像パス一覧
    pub concept_images: Vec<String>,
    /// 選択済みコンセプト画像パス（character.concept_image_path）
    pub selected_concept_path: Option<String>,
    /// generated ステータスで direction=*, animation="base" のスプライト画像パス
    pub direction_images: Vec<String>,
    /// generated ステータスで animation!="base" のスプライト画像パス
    pub animation_frames: Vec<String>,
    /// 復元されたステージ（"idle", "concept_art", "concept", "direction", "animation", "completed"）
    pub stage: String,
}

/// ディスク / DB から生成パイプラインの状態を復元する
///
/// アプリ再起動後にフロントエンドの generation store を復元するために使用．
/// concepts ディレクトリのスキャン，character.concept_image_path の取得，
/// sprites テーブルの generated ステータスレコードからステージを判定する．
#[tauri::command]
pub async fn get_generation_state(
    state: tauri::State<'_, AppState>,
    character_id: String,
) -> Result<GenerationState, AppError> {
    let (character, project) = {
        let conn = state
            .db
            .lock()
            .map_err(|e| AppError::Internal(e.to_string()))?;
        let character = db::queries::character::get_character(&conn, &character_id)?
            .ok_or_else(|| AppError::NotFound("キャラクターが見つかりません".into()))?;
        let project = db::queries::project::get_project(&conn, &character.project_id)?
            .ok_or_else(|| AppError::Internal("プロジェクトが見つかりません".into()))?;
        (character, project)
    };

    // 1a. concept_art ディレクトリから画像ファイルを走査
    let concept_art_dir = std::path::Path::new(&project.base_path)
        .join(&character.name)
        .join("concept_art");
    let mut concept_art_images = Vec::new();
    if concept_art_dir.exists() {
        if let Ok(entries) = std::fs::read_dir(&concept_art_dir) {
            let mut paths: Vec<String> = entries
                .filter_map(|e| e.ok())
                .filter(|e| {
                    e.path()
                        .extension()
                        .map(|ext| ext == "png" || ext == "jpg" || ext == "jpeg")
                        .unwrap_or(false)
                })
                .map(|e| e.path().to_string_lossy().to_string())
                .collect();
            paths.sort();
            concept_art_images = paths;
        }
    }

    // 1b. concepts ディレクトリから画像ファイルを走査
    let concepts_dir = std::path::Path::new(&project.base_path)
        .join(&character.name)
        .join("concepts");
    let mut concept_images = Vec::new();
    if concepts_dir.exists() {
        if let Ok(entries) = std::fs::read_dir(&concepts_dir) {
            let mut paths: Vec<String> = entries
                .filter_map(|e| e.ok())
                .filter(|e| {
                    e.path()
                        .extension()
                        .map(|ext| ext == "png" || ext == "jpg" || ext == "jpeg")
                        .unwrap_or(false)
                })
                .map(|e| e.path().to_string_lossy().to_string())
                .collect();
            paths.sort();
            concept_images = paths;
        }
    }

    // 2. キャラクターレコードから選択済みコンセプトを取得
    let selected_concept_path = character.concept_image_path.clone();

    // 選択済みコンセプトアートパスを推定
    // concept_art/ に画像があり，concepts/ にも画像がある場合は選択済みとみなす
    let selected_concept_art_path: Option<String> = if !concept_art_images.is_empty()
        && !concept_images.is_empty()
    {
        // concept_art/ の最初の画像を選択済みとみなす（フロントエンドで上書き可能）
        Some(concept_art_images[0].clone())
    } else {
        None
    };

    // 3. generated ステータスのスプライトを方向画像とアニメーションフレームに分類
    let (direction_images, animation_frames) = {
        let conn = state
            .db
            .lock()
            .map_err(|e| AppError::Internal(e.to_string()))?;
        let sprites = db::queries::sprite::get_sprites_by_character(&conn, &character_id)
            .map_err(|e| AppError::Database(e.to_string()))?;

        let mut dir_imgs = Vec::new();
        let mut anim_frames = Vec::new();

        for sprite in &sprites {
            let status = &sprite.status;
            if status != "generated" && status != "raw" {
                continue;
            }
            if let Some(ref path) = sprite.raw_path {
                if sprite.animation == "base" {
                    dir_imgs.push(path.clone());
                } else {
                    anim_frames.push(path.clone());
                }
            }
        }

        dir_imgs.sort();
        anim_frames.sort();
        (dir_imgs, anim_frames)
    };

    // 4. 復元ステージを判定
    let stage = if !animation_frames.is_empty() {
        "animation".to_string()
    } else if !direction_images.is_empty() {
        "direction".to_string()
    } else if selected_concept_path.is_some() {
        "concept".to_string()
    } else if !concept_images.is_empty() {
        "concept".to_string()
    } else if !concept_art_images.is_empty() && concept_images.is_empty() {
        // concept_art/ に画像があるが concepts/ にはまだない → concept_art ステージ
        "concept_art".to_string()
    } else {
        "idle".to_string()
    };

    Ok(GenerationState {
        concept_art_images,
        selected_concept_art_path,
        concept_images,
        selected_concept_path,
        direction_images,
        animation_frames,
        stage,
    })
}

// ============================================================
// 画像保存（お気に入り / ブックマーク）
// ============================================================

/// 保存済み画像の一覧
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SavedImages {
    pub concept_art: Vec<String>,
    pub pixel_art: Vec<String>,
    pub direction: Vec<String>,
    pub animation: Vec<String>,
}

/// 生成画像をプロジェクトに保存する
///
/// 指定された画像ファイルを `{base_path}/{character_name}/saved/{stage}/` にコピーする．
/// 同名ファイルが既に存在する場合はそのまま既存パスを返す（冪等）．
#[tauri::command]
pub async fn save_generation_image(
    state: tauri::State<'_, AppState>,
    character_id: String,
    image_path: String,
    stage: String,
) -> Result<String, AppError> {
    // ステージ名のバリデーション
    match stage.as_str() {
        "concept_art" | "pixel_art" | "direction" | "animation" => {}
        _ => {
            return Err(AppError::Validation(format!(
                "不正なステージ名です: {}",
                stage
            )));
        }
    }

    // キャラクター・プロジェクト情報取得
    let (character, project) = {
        let conn = state
            .db
            .lock()
            .map_err(|e| AppError::Internal(e.to_string()))?;

        let character = db::queries::character::get_character(&conn, &character_id)?
            .ok_or_else(|| AppError::NotFound("キャラクターが見つかりません".into()))?;
        let project = db::queries::project::get_project(&conn, &character.project_id)?
            .ok_or_else(|| AppError::Internal("プロジェクトが見つかりません".into()))?;

        (character, project)
    };

    // 保存先ディレクトリを作成
    let saved_dir = std::path::Path::new(&project.base_path)
        .join(&character.name)
        .join("saved")
        .join(&stage);
    std::fs::create_dir_all(&saved_dir)
        .map_err(|e| AppError::Io(format!("保存ディレクトリの作成に失敗: {}", e)))?;

    // ソースファイルの確認
    let src = std::path::Path::new(&image_path);
    if !src.exists() {
        return Err(AppError::Io(format!(
            "保存元ファイルが見つかりません: {}",
            image_path
        )));
    }

    let filename = src
        .file_name()
        .ok_or_else(|| AppError::Internal("無効なファイルパス".into()))?;
    let dest = saved_dir.join(filename);

    // 冪等: 既にコピー済みならそのまま返す
    if !dest.exists() {
        std::fs::copy(src, &dest)
            .map_err(|e| AppError::Io(format!("ファイルコピーに失敗: {}", e)))?;
    }

    let saved_path = dest.to_string_lossy().to_string();
    log::info!("画像を保存しました: {} (stage: {})", saved_path, stage);

    Ok(saved_path)
}

/// キャラクターの保存済み画像一覧を取得する
///
/// `{base_path}/{character_name}/saved/` 配下のサブディレクトリを走査し，
/// ステージ別に絶対パスの一覧を返す．
#[tauri::command]
pub async fn get_saved_images(
    state: tauri::State<'_, AppState>,
    character_id: String,
) -> Result<SavedImages, AppError> {
    let (character, project) = {
        let conn = state
            .db
            .lock()
            .map_err(|e| AppError::Internal(e.to_string()))?;

        let character = db::queries::character::get_character(&conn, &character_id)?
            .ok_or_else(|| AppError::NotFound("キャラクターが見つかりません".into()))?;
        let project = db::queries::project::get_project(&conn, &character.project_id)?
            .ok_or_else(|| AppError::Internal("プロジェクトが見つかりません".into()))?;

        (character, project)
    };

    let saved_base = std::path::Path::new(&project.base_path)
        .join(&character.name)
        .join("saved");

    let scan_stage = |stage_name: &str| -> Vec<String> {
        let dir = saved_base.join(stage_name);
        if !dir.exists() {
            return Vec::new();
        }
        let Ok(entries) = std::fs::read_dir(&dir) else {
            return Vec::new();
        };
        let mut paths: Vec<String> = entries
            .filter_map(|e| e.ok())
            .filter(|e| {
                e.path()
                    .extension()
                    .map(|ext| ext == "png" || ext == "jpg" || ext == "jpeg")
                    .unwrap_or(false)
            })
            .map(|e| e.path().to_string_lossy().to_string())
            .collect();
        paths.sort();
        paths
    };

    Ok(SavedImages {
        concept_art: scan_stage("concept_art"),
        pixel_art: scan_stage("pixel_art"),
        direction: scan_stage("direction"),
        animation: scan_stage("animation"),
    })
}

/// 保存済み画像を削除する
///
/// 安全チェックとして，パスに "/saved/" が含まれていることを確認してから削除する．
#[tauri::command]
pub async fn delete_saved_image(image_path: String) -> Result<(), AppError> {
    // 安全チェック: "/saved/" を含むパスのみ削除を許可
    if !image_path.contains("/saved/") {
        return Err(AppError::Validation(
            "保存済み画像のパスではありません．削除できるのは saved/ ディレクトリ内のファイルのみです．"
                .into(),
        ));
    }

    let path = std::path::Path::new(&image_path);
    if path.exists() {
        std::fs::remove_file(path)
            .map_err(|e| AppError::Io(format!("ファイル削除に失敗: {}", e)))?;
    }

    log::info!("保存済み画像を削除しました: {}", image_path);

    Ok(())
}
