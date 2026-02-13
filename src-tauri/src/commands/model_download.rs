use futures_util::StreamExt;
use serde::Serialize;
use tauri::Emitter;

use crate::db;
use crate::error::AppError;
use crate::state::AppState;

/// ONNX モデルのダウンロード状態
#[derive(Debug, Clone, Serialize)]
pub struct OnnxModelStatus {
    pub downloaded: bool,
    pub path: String,
    pub file_size: Option<u64>,
}

/// ONNX モデルの存在確認
///
/// models_dir 内の u2net.onnx ファイルの存在とサイズを確認し，
/// ダウンロード状態を返す．ファイルが存在しない場合は DB フラグもリセットする．
#[tauri::command]
pub async fn check_onnx_model(
    state: tauri::State<'_, AppState>,
) -> Result<OnnxModelStatus, AppError> {
    let model_path = state.models_dir.join("u2net.onnx");
    let path_str = model_path.to_string_lossy().to_string();

    if model_path.exists() {
        let file_size = std::fs::metadata(&model_path)
            .map(|m| m.len())
            .ok();
        Ok(OnnxModelStatus {
            downloaded: true,
            path: path_str,
            file_size,
        })
    } else {
        // ファイルが存在しない場合，DB フラグをリセット
        let conn = state
            .db
            .lock()
            .map_err(|e| AppError::Internal(e.to_string()))?;
        let _ = db::queries::settings::set_setting(&conn, "onnx_model_downloaded", "false");

        Ok(OnnxModelStatus {
            downloaded: false,
            path: path_str,
            file_size: None,
        })
    }
}

/// ONNX モデルのダウンロード
///
/// U2-Net モデルを GitHub Releases からストリーミングダウンロードする．
/// ダウンロード中は `model-download-progress` イベントで進捗を通知する．
/// 一時ファイルに書き込み後，アトミックにリネームして破損を防ぐ．
#[tauri::command]
pub async fn download_onnx_model(
    state: tauri::State<'_, AppState>,
    app: tauri::AppHandle,
) -> Result<String, AppError> {
    let model_path = state.models_dir.join("u2net.onnx");

    // 既にダウンロード済みの場合はパスを返却
    if model_path.exists() {
        let file_size = std::fs::metadata(&model_path)
            .map(|m| m.len())
            .unwrap_or(0);
        if file_size > 0 {
            return Ok(model_path.to_string_lossy().to_string());
        }
    }

    let model_url = "https://github.com/danielgatis/rembg/releases/download/v0.0.0/u2net.onnx";
    let temp_path = state.models_dir.join("u2net.onnx.tmp");

    // models_dir が存在することを保証
    std::fs::create_dir_all(&state.models_dir)
        .map_err(|e| AppError::Io(format!("Failed to create models directory: {}", e)))?;

    // ストリーミングダウンロード
    let client = reqwest::Client::new();
    let response = client
        .get(model_url)
        .send()
        .await
        .map_err(|e| AppError::Io(format!("Download request failed: {}", e)))?;

    if !response.status().is_success() {
        return Err(AppError::Io(format!(
            "Download failed with status: {}",
            response.status()
        )));
    }

    let total_bytes = response.content_length().unwrap_or(0);
    let mut downloaded_bytes: u64 = 0;

    let mut file = std::fs::File::create(&temp_path)
        .map_err(|e| AppError::Io(format!("Failed to create temp file: {}", e)))?;

    let mut stream = response.bytes_stream();

    while let Some(chunk) = stream.next().await {
        let chunk = chunk.map_err(|e| AppError::Io(format!("Download stream error: {}", e)))?;

        std::io::Write::write_all(&mut file, &chunk)
            .map_err(|e| AppError::Io(format!("Write error: {}", e)))?;

        downloaded_bytes += chunk.len() as u64;

        // プログレスイベント発行
        let _ = app.emit(
            "model-download-progress",
            serde_json::json!({
                "downloaded_bytes": downloaded_bytes,
                "total_bytes": total_bytes,
                "percentage": if total_bytes > 0 {
                    (downloaded_bytes as f64 / total_bytes as f64 * 100.0) as u32
                } else {
                    0
                },
            }),
        );
    }

    // フラッシュしてクローズ
    std::io::Write::flush(&mut file)
        .map_err(|e| AppError::Io(format!("Flush error: {}", e)))?;
    drop(file);

    // ファイルサイズ検証（0 バイトでないことを確認）
    let file_size = std::fs::metadata(&temp_path)
        .map(|m| m.len())
        .unwrap_or(0);

    if file_size == 0 {
        // 一時ファイルを削除
        let _ = std::fs::remove_file(&temp_path);
        return Err(AppError::Io("Downloaded file is empty".into()));
    }

    // 一時ファイルを正式パスにアトミックリネーム
    std::fs::rename(&temp_path, &model_path)
        .map_err(|e| AppError::Io(format!("Rename error: {}", e)))?;

    // DB 設定更新
    let conn = state
        .db
        .lock()
        .map_err(|e| AppError::Internal(e.to_string()))?;
    db::queries::settings::set_setting(&conn, "onnx_model_downloaded", "true")
        .map_err(|e| AppError::Database(e.to_string()))?;
    db::queries::settings::set_setting(
        &conn,
        "onnx_model_path",
        &model_path.to_string_lossy(),
    )
    .map_err(|e| AppError::Database(e.to_string()))?;

    log::info!(
        "ONNX model downloaded: {} ({} bytes)",
        model_path.display(),
        file_size
    );

    Ok(model_path.to_string_lossy().to_string())
}
