use serde::Serialize;

use crate::db;
use crate::error::AppError;
use crate::models::AnimationDef;
use crate::services::path_utils;
use crate::state::AppState;

// ============================================================
// メタデータ型定義
// ============================================================

/// スプライトシートメタデータのルート構造体
///
/// Bevy 側で読み込む `meta.json` の構造に対応する．
/// 設計書: Bevyアセット出力仕様 §3
#[derive(Debug, Clone, Serialize)]
pub struct SpritesheetMeta {
    /// メタデータフォーマットのバージョン
    pub version: String,
    /// キャラクター名
    pub character: String,
    /// スプライトシートの寸法情報
    pub spritesheet: SpritesheetInfo,
    /// 方向と行のマッピング
    pub directions: Vec<DirectionMeta>,
    /// アニメーション定義
    pub animations: Vec<AnimationMeta>,
}

/// スプライトシートの寸法情報
#[derive(Debug, Clone, Serialize)]
pub struct SpritesheetInfo {
    /// 総列数
    pub columns: u32,
    /// 総行数
    pub rows: u32,
    /// 1タイルの幅（ピクセル）
    pub tile_width: u32,
    /// 1タイルの高さ（ピクセル）
    pub tile_height: u32,
}

/// 方向メタデータ
#[derive(Debug, Clone, Serialize)]
pub struct DirectionMeta {
    /// 方向名 ("down", "left", "right", "up")
    pub name: String,
    /// スプライトシート上の行インデックス（0始まり）
    pub row: u32,
}

/// アニメーションメタデータ
#[derive(Debug, Clone, Serialize)]
pub struct AnimationMeta {
    /// アニメーション名 ("idle", "walk", "attack", "hit")
    pub name: String,
    /// 開始列インデックス（0始まり）
    pub start_col: u32,
    /// フレーム数
    pub frame_count: u32,
    /// 1フレームの表示時間（ミリ秒）
    pub frame_duration_ms: u32,
}

impl SpritesheetMeta {
    /// プロジェクトの設定からメタデータを生成する
    pub fn new(
        character_name: &str,
        tile_width: u32,
        tile_height: u32,
        directions: &[String],
        animations: &[AnimationDef],
    ) -> Self {
        let direction_metas: Vec<DirectionMeta> = directions
            .iter()
            .enumerate()
            .map(|(i, name)| DirectionMeta {
                name: name.clone(),
                row: i as u32,
            })
            .collect();

        let mut start_col: u32 = 0;
        let animation_metas: Vec<AnimationMeta> = animations
            .iter()
            .map(|anim| {
                let meta = AnimationMeta {
                    name: anim.name.clone(),
                    start_col,
                    frame_count: anim.frame_count,
                    frame_duration_ms: anim.frame_duration_ms,
                };
                start_col += anim.frame_count;
                meta
            })
            .collect();

        let total_columns: u32 = animations.iter().map(|a| a.frame_count).sum();

        Self {
            version: "1.0".to_string(),
            character: character_name.to_string(),
            spritesheet: SpritesheetInfo {
                columns: total_columns,
                rows: directions.len() as u32,
                tile_width,
                tile_height,
            },
            directions: direction_metas,
            animations: animation_metas,
        }
    }
}

/// エクスポート結果
#[derive(Debug, Clone, Serialize)]
pub struct ExportResult {
    /// コピーされたスプライトシートPNGのパス
    pub png_path: String,
    /// 生成されたメタデータJSONのパス
    pub meta_path: String,
    /// 生成されたメタデータの内容
    pub meta: SpritesheetMeta,
}

// ============================================================
// Tauri コマンド
// ============================================================

/// Bevy アセットエクスポートコマンド
///
/// キャラクターのスプライトシート PNG を出力先にコピーし，
/// Bevy 向けの `meta.json` を生成する．
///
/// 処理フロー:
/// 1. キャラクター・プロジェクト情報を DB から取得
/// 2. スプライトシートが生成済みか検証
/// 3. 出力先ディレクトリ `{output_dir}/characters/` を作成
/// 4. スプライトシート PNG をコピー（ファイル名はサニタイズ済み）
/// 5. SpritesheetMeta を JSON として書き出し
/// 6. ExportResult を返却
#[tauri::command]
pub async fn export_for_bevy(
    state: tauri::State<'_, AppState>,
    character_id: String,
    output_dir: String,
) -> Result<ExportResult, AppError> {
    let conn = state
        .db
        .lock()
        .map_err(|e| AppError::Internal(e.to_string()))?;

    // キャラクター情報取得
    let character = db::queries::character::get_character(&conn, &character_id)?
        .ok_or_else(|| {
            AppError::NotFound(format!("キャラクターが見つかりません: {}", character_id))
        })?;

    // プロジェクト情報取得
    let project = db::queries::project::get_project(&conn, &character.project_id)?
        .ok_or_else(|| AppError::Internal("プロジェクトが見つかりません".into()))?;

    // スプライトシート存在確認
    let spritesheet_path = character.spritesheet_path.as_ref().ok_or_else(|| {
        AppError::Export(format!(
            "スプライトシートが未生成です．先にスプライトシートを生成してください: {}",
            character.name
        ))
    })?;

    let sheet_src = std::path::Path::new(spritesheet_path);
    if !sheet_src.exists() {
        return Err(AppError::Export(format!(
            "スプライトシートファイルが見つかりません: {}",
            spritesheet_path
        )));
    }

    // ファイル名サニタイズ
    let safe_name = path_utils::sanitize_filename(&character.name)?;

    // 出力先ディレクトリ作成
    let output_base = std::path::Path::new(&output_dir).join("characters");
    path_utils::ensure_dir(&output_base)?;

    // スプライトシート PNG コピー
    let png_filename = format!("{}.png", safe_name);
    let png_dest = output_base.join(&png_filename);
    std::fs::copy(sheet_src, &png_dest).map_err(|e| {
        AppError::Export(format!(
            "スプライトシートのコピーに失敗しました: {}",
            e
        ))
    })?;

    // メタデータ生成
    let meta = SpritesheetMeta::new(
        &character.name,
        project.tile_width as u32,
        project.tile_height as u32,
        &project.directions,
        &project.animations,
    );

    // meta.json 書き出し
    let meta_filename = format!("{}.meta.json", safe_name);
    let meta_dest = output_base.join(&meta_filename);
    let meta_json = serde_json::to_string_pretty(&meta).map_err(|e| {
        AppError::Export(format!("メタデータのJSON変換に失敗しました: {}", e))
    })?;
    std::fs::write(&meta_dest, &meta_json).map_err(|e| {
        AppError::Export(format!("メタデータの書き出しに失敗しました: {}", e))
    })?;

    let png_path = png_dest.to_string_lossy().to_string();
    let meta_path = meta_dest.to_string_lossy().to_string();

    log::info!(
        "Bevy export completed for '{}': png={}, meta={}",
        character.name,
        png_path,
        meta_path
    );

    Ok(ExportResult {
        png_path,
        meta_path,
        meta,
    })
}
