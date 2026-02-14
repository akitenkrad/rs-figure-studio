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

// ============================================================
// パレット正規化
// ============================================================

/// ディザリング方式
#[derive(Debug, Clone, serde::Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DitheringMethod {
    None,
    FloydSteinberg,
    Ordered,
}

/// パレット正規化パラメータ
#[derive(Debug, Clone, serde::Deserialize)]
pub struct PaletteParams {
    pub max_colors: u32,
    pub dithering: DitheringMethod,
    pub preserve_alpha: bool,
}

/// Floyd-Steinberg ディザリングによるパレット正規化
///
/// 指定色数に減色し，ディザリングで品質を維持する．
/// ピクセルアートでは `Ordered` or `None` が一般的．
#[tauri::command]
pub async fn normalize_palette(
    state: tauri::State<'_, AppState>,
    character_id: String,
    params: PaletteParams,
    app: tauri::AppHandle,
) -> Result<Vec<String>, AppError> {
    let conn = state
        .db
        .lock()
        .map_err(|e| AppError::Internal(e.to_string()))?;

    let sprites = db::queries::sprite::get_sprites_by_character(&conn, &character_id)?;

    let character = db::queries::character::get_character(&conn, &character_id)?
        .ok_or_else(|| AppError::NotFound("キャラクターが見つかりません".into()))?;
    let project = db::queries::project::get_project(&conn, &character.project_id)?
        .ok_or_else(|| AppError::Internal("プロジェクトが見つかりません".into()))?;

    let output_dir = std::path::Path::new(&project.base_path)
        .join(&character.name)
        .join("palette_normalized");
    std::fs::create_dir_all(&output_dir)
        .map_err(|e| AppError::Io(format!("ディレクトリ作成に失敗: {}", e)))?;

    // 処理対象: final_path > processed_path > raw_path
    let target_sprites: Vec<_> = sprites
        .iter()
        .filter(|s| s.final_path.is_some() || s.processed_path.is_some() || s.raw_path.is_some())
        .collect();

    let total = target_sprites.len() as u32;
    let mut results = Vec::new();

    for (i, sprite) in target_sprites.iter().enumerate() {
        let input_path = sprite
            .final_path
            .as_ref()
            .or(sprite.processed_path.as_ref())
            .or(sprite.raw_path.as_ref())
            .ok_or_else(|| AppError::Validation("画像パスがありません".into()))?;

        let img = image::open(input_path)
            .map_err(|e| AppError::ImageProcessing(format!("画像を開けません: {}", e)))?
            .to_rgba8();

        let reduced = reduce_palette(&img, params.max_colors, &params.dithering, params.preserve_alpha);

        let src_path = std::path::Path::new(input_path);
        let filename = src_path
            .file_name()
            .ok_or_else(|| AppError::Internal("無効なパス".into()))?;
        let output_path = output_dir.join(filename);

        reduced.save(&output_path).map_err(|e| {
            AppError::ImageProcessing(format!("パレット正規化画像の保存に失敗: {}", e))
        })?;

        let output_str = output_path.to_string_lossy().to_string();
        results.push(output_str);

        let _ = app.emit(
            "palette-progress",
            serde_json::json!({
                "current": i + 1,
                "total": total,
                "sprite_id": sprite.id,
            }),
        );
    }

    log::info!(
        "パレット正規化完了: {} 枚 ({}色, character: {})",
        results.len(),
        params.max_colors,
        character.name
    );

    Ok(results)
}

/// パレット減色処理
///
/// color_quant（NeuQuant）によるメディアンカットで減色し，
/// ディザリングで品質を維持する．
fn reduce_palette(
    img: &image::RgbaImage,
    max_colors: u32,
    dithering: &DitheringMethod,
    preserve_alpha: bool,
) -> image::RgbaImage {
    let (w, h) = img.dimensions();

    // 不透明ピクセルのRGBデータを収集
    let mut rgb_pixels: Vec<[u8; 3]> = Vec::new();
    for pixel in img.pixels() {
        if !preserve_alpha || pixel[3] > 0 {
            rgb_pixels.push([pixel[0], pixel[1], pixel[2]]);
        }
    }

    if rgb_pixels.is_empty() || max_colors == 0 {
        return img.clone();
    }

    // NeuQuant でパレットを構築
    let flat: Vec<u8> = rgb_pixels.iter().flat_map(|p| p.iter().copied()).collect();
    let nq = color_quant::NeuQuant::new(
        10, // サンプリングファクタ（1-30, 低=品質高）
        max_colors.min(256) as usize,
        &flat,
    );

    let palette: Vec<[u8; 3]> = (0..max_colors.min(256) as usize)
        .filter_map(|i| {
            nq.lookup(i).map(|c| [c[0], c[1], c[2]])
        })
        .collect();

    // 各ピクセルを最近傍パレット色にマッピング
    let mut output = image::RgbaImage::new(w, h);

    match dithering {
        DitheringMethod::None => {
            for (x, y, pixel) in img.enumerate_pixels() {
                if preserve_alpha && pixel[3] == 0 {
                    output.put_pixel(x, y, *pixel);
                    continue;
                }
                let idx = nq.index_of(&[pixel[0], pixel[1], pixel[2]]);
                let c = palette.get(idx).copied().unwrap_or([pixel[0], pixel[1], pixel[2]]);
                output.put_pixel(x, y, image::Rgba([c[0], c[1], c[2], pixel[3]]));
            }
        }
        DitheringMethod::FloydSteinberg => {
            // Floyd-Steinberg エラー拡散
            let mut error_buf: Vec<Vec<[f32; 3]>> =
                vec![vec![[0.0; 3]; w as usize]; h as usize];

            for y in 0..h {
                for x in 0..w {
                    let pixel = img.get_pixel(x, y);
                    if preserve_alpha && pixel[3] == 0 {
                        output.put_pixel(x, y, *pixel);
                        continue;
                    }

                    let err = error_buf[y as usize][x as usize];
                    let r = (pixel[0] as f32 + err[0]).clamp(0.0, 255.0);
                    let g = (pixel[1] as f32 + err[1]).clamp(0.0, 255.0);
                    let b = (pixel[2] as f32 + err[2]).clamp(0.0, 255.0);

                    let idx = nq.index_of(&[r as u8, g as u8, b as u8]);
                    let c = palette.get(idx).copied().unwrap_or([r as u8, g as u8, b as u8]);

                    output.put_pixel(x, y, image::Rgba([c[0], c[1], c[2], pixel[3]]));

                    let quant_err = [r - c[0] as f32, g - c[1] as f32, b - c[2] as f32];

                    // 7/16 右, 3/16 左下, 5/16 下, 1/16 右下
                    if x + 1 < w {
                        let e = &mut error_buf[y as usize][(x + 1) as usize];
                        for i in 0..3 { e[i] += quant_err[i] * 7.0 / 16.0; }
                    }
                    if y + 1 < h {
                        if x > 0 {
                            let e = &mut error_buf[(y + 1) as usize][(x - 1) as usize];
                            for i in 0..3 { e[i] += quant_err[i] * 3.0 / 16.0; }
                        }
                        let e = &mut error_buf[(y + 1) as usize][x as usize];
                        for i in 0..3 { e[i] += quant_err[i] * 5.0 / 16.0; }
                        if x + 1 < w {
                            let e = &mut error_buf[(y + 1) as usize][(x + 1) as usize];
                            for i in 0..3 { e[i] += quant_err[i] * 1.0 / 16.0; }
                        }
                    }
                }
            }
        }
        DitheringMethod::Ordered => {
            // 4x4 Bayer matrix ordered dithering
            let bayer: [[f32; 4]; 4] = [
                [0.0, 8.0, 2.0, 10.0],
                [12.0, 4.0, 14.0, 6.0],
                [3.0, 11.0, 1.0, 9.0],
                [15.0, 7.0, 13.0, 5.0],
            ];

            let scale = 255.0 / max_colors.max(1) as f32;

            for (x, y, pixel) in img.enumerate_pixels() {
                if preserve_alpha && pixel[3] == 0 {
                    output.put_pixel(x, y, *pixel);
                    continue;
                }

                let threshold = (bayer[(y % 4) as usize][(x % 4) as usize] / 16.0 - 0.5) * scale;

                let r = (pixel[0] as f32 + threshold).clamp(0.0, 255.0);
                let g = (pixel[1] as f32 + threshold).clamp(0.0, 255.0);
                let b = (pixel[2] as f32 + threshold).clamp(0.0, 255.0);

                let idx = nq.index_of(&[r as u8, g as u8, b as u8]);
                let c = palette.get(idx).copied().unwrap_or([r as u8, g as u8, b as u8]);
                output.put_pixel(x, y, image::Rgba([c[0], c[1], c[2], pixel[3]]));
            }
        }
    }

    output
}
