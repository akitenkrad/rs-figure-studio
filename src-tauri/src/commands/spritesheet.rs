use serde::Serialize;

use crate::db;
use crate::error::AppError;
use crate::models::{Character, Project, Sprite};
use crate::state::AppState;

/// スプライトシート生成結果
#[derive(Debug, Serialize)]
pub struct SpritesheetResult {
    pub path: String,
    pub width: u32,
    pub height: u32,
    pub columns: u32,
    pub rows: u32,
    pub missing_frames: Vec<MissingFrame>,
}

/// 不足フレーム情報
#[derive(Debug, Serialize)]
pub struct MissingFrame {
    pub direction: String,
    pub animation: String,
    pub frame_index: i32,
}

/// スプライトシート画像を生成する内部関数
///
/// 配置ルール:
/// - 行 = directions（down, left, right, up）
/// - 列 = アニメーションフレーム連結（idle + walk + attack + hit）
/// - 不足フレームは透明で埋める
fn generate_spritesheet_image(
    sprites: &[&Sprite],
    project: &Project,
    character: &Character,
) -> Result<SpritesheetResult, AppError> {
    let tile_w = project.tile_width as u32;
    let tile_h = project.tile_height as u32;

    // 総列数を計算
    let total_columns: u32 = project.animations.iter().map(|a| a.frame_count).sum();

    let direction_count = project.directions.len() as u32;

    // シートサイズ
    let sheet_width = tile_w * total_columns;
    let sheet_height = tile_h * direction_count;

    // 透明キャンバス作成
    let mut canvas = image::RgbaImage::new(sheet_width, sheet_height);

    // スプライトをルックアップ用の HashMap に整理
    // key: (direction, animation, frame_index)
    let mut sprite_map: std::collections::HashMap<(&str, &str, i32), &Sprite> =
        std::collections::HashMap::new();
    for sprite in sprites {
        sprite_map.insert(
            (&sprite.direction, &sprite.animation, sprite.frame_index),
            sprite,
        );
    }

    let mut missing_frames = Vec::new();

    // 各方向（行）をループ
    for (row, direction) in project.directions.iter().enumerate() {
        let mut col_offset: u32 = 0;

        // 各アニメーション定義をループ
        for anim in &project.animations {
            // 各フレームをループ
            for frame in 0..anim.frame_count as i32 {
                let col = col_offset + frame as u32;

                // 対応するスプライトを検索
                match sprite_map.get(&(direction.as_str(), anim.name.as_str(), frame)) {
                    Some(sprite) => {
                        if let Some(ref final_path) = sprite.final_path {
                            // 画像読み込み
                            match image::open(final_path) {
                                Ok(img) => {
                                    let rgba = img.to_rgba8();
                                    let x = col * tile_w;
                                    let y = row as u32 * tile_h;

                                    // キャンバスに配置
                                    image::imageops::overlay(
                                        &mut canvas,
                                        &rgba,
                                        x as i64,
                                        y as i64,
                                    );
                                }
                                Err(e) => {
                                    log::warn!(
                                        "Failed to load sprite image {}: {}",
                                        final_path,
                                        e
                                    );
                                    missing_frames.push(MissingFrame {
                                        direction: direction.clone(),
                                        animation: anim.name.clone(),
                                        frame_index: frame,
                                    });
                                }
                            }
                        } else {
                            missing_frames.push(MissingFrame {
                                direction: direction.clone(),
                                animation: anim.name.clone(),
                                frame_index: frame,
                            });
                        }
                    }
                    None => {
                        // スプライトが存在しない → 透明のまま
                        missing_frames.push(MissingFrame {
                            direction: direction.clone(),
                            animation: anim.name.clone(),
                            frame_index: frame,
                        });
                    }
                }
            }

            col_offset += anim.frame_count;
        }
    }

    // 出力パス
    let output_path = std::path::Path::new(&project.base_path)
        .join(&character.name)
        .join("spritesheet")
        .join(format!("{}.png", character.name));

    // 出力ディレクトリ作成
    if let Some(parent) = output_path.parent() {
        std::fs::create_dir_all(parent)
            .map_err(|e| AppError::Io(format!("Failed to create spritesheet directory: {}", e)))?;
    }

    // PNG保存
    canvas.save(&output_path).map_err(|e| {
        AppError::ImageProcessing(format!("Failed to save spritesheet: {}", e))
    })?;

    if !missing_frames.is_empty() {
        log::warn!(
            "Spritesheet generated with {} missing frames for '{}'",
            missing_frames.len(),
            character.name
        );
    }

    Ok(SpritesheetResult {
        path: output_path.to_string_lossy().to_string(),
        width: sheet_width,
        height: sheet_height,
        columns: total_columns,
        rows: direction_count,
        missing_frames,
    })
}

/// スプライトシート生成コマンド
///
/// キャラクターの全 finalized スプライトをグリッド配置した
/// 1枚のスプライトシート PNG を生成する．
///
/// レイアウト:
/// - 行 = directions（デフォルト: down, left, right, up）
/// - 列 = animations 連結（デフォルト: idle[2] + walk[4] + attack[4] + hit[2] = 12列）
/// - 不足フレームは透明で埋め，missing_frames で報告する
#[tauri::command]
pub async fn generate_spritesheet(
    state: tauri::State<'_, AppState>,
    character_id: String,
) -> Result<SpritesheetResult, AppError> {
    let conn = state
        .db
        .lock()
        .map_err(|e| AppError::Internal(e.to_string()))?;

    let character = db::queries::character::get_character(&conn, &character_id)?
        .ok_or_else(|| AppError::NotFound(format!("Character not found: {}", character_id)))?;

    let project = db::queries::project::get_project(&conn, &character.project_id)?
        .ok_or_else(|| AppError::Internal("Project not found".into()))?;

    let sprites = db::queries::sprite::get_sprites_by_character(&conn, &character_id)?;

    // finalized 状態のスプライトのみ使用
    let finalized: Vec<&Sprite> = sprites
        .iter()
        .filter(|s| s.status == "finalized" && s.final_path.is_some())
        .collect();

    let result = generate_spritesheet_image(&finalized, &project, &character)?;

    // スプライトシートパスを DB に保存
    conn.execute(
        "UPDATE characters SET spritesheet_path = ?1, updated_at = ?2 WHERE id = ?3",
        rusqlite::params![result.path, chrono::Utc::now().to_rfc3339(), character_id],
    )
    .map_err(|e| AppError::Database(e.to_string()))?;

    Ok(result)
}
