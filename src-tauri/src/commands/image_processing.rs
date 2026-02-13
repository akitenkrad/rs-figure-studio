use tauri::Emitter;

use crate::db;
use crate::error::AppError;
use crate::models::{SpriteStatus, UpdateSpritePaths};
use crate::state::AppState;

/// Bounding Box（不透明領域の最小矩形）
#[derive(Debug, Clone)]
struct BoundingBox {
    x: u32,
    y: u32,
    width: u32,
    height: u32,
}

/// アルファチャンネルから不透明領域の bounding box を検出
fn find_alpha_bounding_box(img: &image::RgbaImage) -> Option<BoundingBox> {
    let (w, h) = img.dimensions();
    let mut min_x = w;
    let mut min_y = h;
    let mut max_x: u32 = 0;
    let mut max_y: u32 = 0;

    for y in 0..h {
        for x in 0..w {
            let pixel = img.get_pixel(x, y);
            if pixel[3] > 0 {
                min_x = min_x.min(x);
                min_y = min_y.min(y);
                max_x = max_x.max(x);
                max_y = max_y.max(y);
            }
        }
    }

    if max_x < min_x || max_y < min_y {
        return None; // 完全透明
    }

    Some(BoundingBox {
        x: min_x,
        y: min_y,
        width: max_x - min_x + 1,
        height: max_y - min_y + 1,
    })
}

/// アスペクト比を維持しつつ，タイルサイズに収まるようリサイズ（Lanczos3フィルタ）
fn fit_to_tile(img: &image::RgbaImage, tile_width: u32, tile_height: u32) -> image::RgbaImage {
    let (w, h) = img.dimensions();

    let scale_x = tile_width as f64 / w as f64;
    let scale_y = tile_height as f64 / h as f64;
    let scale = scale_x.min(scale_y); // 小さい方に合わせる

    let new_width = (w as f64 * scale).round() as u32;
    let new_height = (h as f64 * scale).round() as u32;

    image::imageops::resize(
        img,
        new_width.max(1),
        new_height.max(1),
        image::imageops::FilterType::Lanczos3,
    )
}

/// スプライト画像の正規化（bounding box 検出 → センタリング → リサイズ）
///
/// 処理手順:
/// 1. アルファチャンネルによる bounding box 検出
/// 2. bounding box 領域を切り出し
/// 3. アスペクト比維持でリサイズ（Lanczos3フィルタ）
/// 4. タイルサイズのキャンバスに底辺揃えでセンタリング配置
#[tauri::command]
pub async fn normalize_sprite(
    state: tauri::State<'_, AppState>,
    sprite_id: String,
    tile_width: u32,
    tile_height: u32,
) -> Result<String, AppError> {
    let conn = state
        .db
        .lock()
        .map_err(|e| AppError::Internal(e.to_string()))?;

    // スプライトIDで直接検索
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

    // 入力画像パスを決定（processed_path > raw_path の優先順）
    let input_path = sprite
        .processed_path
        .as_ref()
        .or(sprite.raw_path.as_ref())
        .ok_or_else(|| AppError::Validation("Sprite has no image path".into()))?;

    // キャラクターとプロジェクト情報取得
    let character = db::queries::character::get_character(&conn, &sprite.character_id)?
        .ok_or_else(|| AppError::NotFound("Character not found".into()))?;

    let project = db::queries::project::get_project(&conn, &character.project_id)?
        .ok_or_else(|| AppError::Internal("Project not found".into()))?;

    // 出力パス: {base_path}/{character}/normalized/{filename}
    let src_path = std::path::Path::new(input_path);
    let filename = src_path
        .file_name()
        .ok_or_else(|| AppError::Internal("Invalid input path".into()))?;

    let output_dir = std::path::Path::new(&project.base_path)
        .join(&character.name)
        .join("normalized");
    std::fs::create_dir_all(&output_dir)
        .map_err(|e| AppError::Io(format!("Failed to create normalized directory: {}", e)))?;

    let output_path = output_dir.join(filename);

    // 画像処理
    let img = image::open(input_path)
        .map_err(|e| AppError::ImageProcessing(format!("Failed to open image: {}", e)))?
        .to_rgba8();

    // アルファチャンネルによる bounding box 検出
    let bbox = find_alpha_bounding_box(&img)
        .ok_or_else(|| AppError::ImageProcessing("Image is fully transparent".into()))?;

    // bounding box 領域を切り出し
    let cropped =
        image::imageops::crop_imm(&img, bbox.x, bbox.y, bbox.width, bbox.height).to_image();

    // アスペクト比維持でリサイズ
    let resized = fit_to_tile(&cropped, tile_width, tile_height);

    // タイルサイズのキャンバスにセンタリング配置（底辺揃え）
    let mut canvas = image::RgbaImage::new(tile_width, tile_height);
    let offset_x = (tile_width as i64 - resized.width() as i64) / 2;
    let offset_y = tile_height as i64 - resized.height() as i64; // 底辺揃え
    image::imageops::overlay(&mut canvas, &resized, offset_x, offset_y);

    canvas.save(&output_path).map_err(|e| {
        AppError::ImageProcessing(format!("Failed to save normalized image: {}", e))
    })?;

    let output_path_str = output_path.to_string_lossy().to_string();

    // DB更新: final_path を設定し status を finalized に
    db::queries::sprite::update_sprite_paths(
        &conn,
        &sprite_id,
        &UpdateSpritePaths {
            raw_path: None,
            processed_path: None,
            final_path: Some(output_path_str.clone()),
        },
    )?;
    db::queries::sprite::update_sprite_status(&conn, &sprite_id, &SpriteStatus::Finalized)?;

    Ok(output_path_str)
}

/// バッチ正規化
///
/// 指定キャラクターの全スプライトを順次正規化する．
/// メモリ消費を制御するため，1枚ずつ処理する．
/// `normalize-progress` イベントで進捗を通知する．
#[tauri::command]
pub async fn normalize_batch(
    state: tauri::State<'_, AppState>,
    character_id: String,
    tile_width: u32,
    tile_height: u32,
    app: tauri::AppHandle,
) -> Result<Vec<String>, AppError> {
    let conn = state
        .db
        .lock()
        .map_err(|e| AppError::Internal(e.to_string()))?;

    let sprites = db::queries::sprite::get_sprites_by_character(&conn, &character_id)?;

    // 処理対象: raw_path または processed_path が存在するスプライト
    let target_sprites: Vec<_> = sprites
        .iter()
        .filter(|s| s.raw_path.is_some() || s.processed_path.is_some())
        .collect();

    let total = target_sprites.len() as u32;
    let mut results = Vec::with_capacity(target_sprites.len());

    // キャラクターとプロジェクト情報取得
    let character = db::queries::character::get_character(&conn, &character_id)?
        .ok_or_else(|| AppError::NotFound("Character not found".into()))?;

    let project = db::queries::project::get_project(&conn, &character.project_id)?
        .ok_or_else(|| AppError::Internal("Project not found".into()))?;

    let output_dir = std::path::Path::new(&project.base_path)
        .join(&character.name)
        .join("normalized");
    std::fs::create_dir_all(&output_dir)
        .map_err(|e| AppError::Io(format!("Failed to create normalized directory: {}", e)))?;

    for (i, sprite) in target_sprites.iter().enumerate() {
        // 入力画像パスを決定
        let input_path = sprite
            .processed_path
            .as_ref()
            .or(sprite.raw_path.as_ref())
            .ok_or_else(|| AppError::Validation("Sprite has no image path".into()))?;

        let src_path = std::path::Path::new(input_path);
        let filename = src_path
            .file_name()
            .ok_or_else(|| AppError::Internal("Invalid input path".into()))?;

        let output_path = output_dir.join(filename);

        // 画像処理
        let img = image::open(input_path)
            .map_err(|e| AppError::ImageProcessing(format!("Failed to open image: {}", e)))?
            .to_rgba8();

        let bbox = find_alpha_bounding_box(&img)
            .ok_or_else(|| AppError::ImageProcessing("Image is fully transparent".into()))?;

        let cropped =
            image::imageops::crop_imm(&img, bbox.x, bbox.y, bbox.width, bbox.height).to_image();

        let resized = fit_to_tile(&cropped, tile_width, tile_height);

        let mut canvas = image::RgbaImage::new(tile_width, tile_height);
        let offset_x = (tile_width as i64 - resized.width() as i64) / 2;
        let offset_y = tile_height as i64 - resized.height() as i64;
        image::imageops::overlay(&mut canvas, &resized, offset_x, offset_y);

        canvas.save(&output_path).map_err(|e| {
            AppError::ImageProcessing(format!("Failed to save normalized image: {}", e))
        })?;

        let output_path_str = output_path.to_string_lossy().to_string();

        // DB更新
        db::queries::sprite::update_sprite_paths(
            &conn,
            &sprite.id,
            &UpdateSpritePaths {
                raw_path: None,
                processed_path: None,
                final_path: Some(output_path_str.clone()),
            },
        )?;
        db::queries::sprite::update_sprite_status(&conn, &sprite.id, &SpriteStatus::Finalized)?;

        results.push(output_path_str);

        // プログレスイベント送信
        let _ = app.emit(
            "normalize-progress",
            serde_json::json!({
                "current": i + 1,
                "total": total,
                "sprite_id": sprite.id,
            }),
        );
    }

    Ok(results)
}
