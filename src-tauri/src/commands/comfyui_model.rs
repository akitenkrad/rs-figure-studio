use futures_util::StreamExt;
use serde::Serialize;
use tauri::Emitter;

use crate::db;
use crate::error::AppError;
use crate::services::comfyui_client::ComfyUIClient;
use crate::state::AppState;

/// ComfyUI チェックポイントモデルのステータス
#[derive(Debug, Clone, Serialize)]
pub struct ComfyuiModelStatus {
    /// モデルが ComfyUI で利用可能か
    pub available: bool,
    /// ComfyUI で利用可能なチェックポイント一覧
    pub available_models: Vec<String>,
}

/// ComfyUI でチェックポイントモデルが利用可能か確認
///
/// ComfyUI API の /object_info/CheckpointLoaderSimple を呼び出し，
/// 指定モデルが利用可能なチェックポイント一覧に含まれるか確認する．
#[tauri::command]
pub async fn check_comfyui_model(
    state: tauri::State<'_, AppState>,
    model_name: String,
) -> Result<ComfyuiModelStatus, AppError> {
    // Get ComfyUI endpoint from settings
    let endpoint = {
        let conn = state
            .db
            .lock()
            .map_err(|e| AppError::Internal(e.to_string()))?;
        db::queries::settings::get_setting(&conn, "comfyui_endpoint")?
            .unwrap_or_else(|| "http://127.0.0.1:8188".to_string())
    };

    let client = ComfyUIClient::new(&endpoint);

    // Check connection first
    let is_healthy = client
        .health_check()
        .await
        .map_err(|e| AppError::ComfyUI(format!("Health check failed: {}", e)))?;

    if !is_healthy {
        return Err(AppError::ComfyUI(
            "ComfyUI に接続できません．ComfyUI を起動してください．".into(),
        ));
    }

    // Query object_info for CheckpointLoaderSimple
    let info = client
        .get_object_info("CheckpointLoaderSimple")
        .await
        .map_err(|e| AppError::ComfyUI(format!("Failed to query model info: {}", e)))?;

    // Extract available checkpoint names from response
    // Structure: { "CheckpointLoaderSimple": { "input": { "required": { "ckpt_name": [["model1.safetensors", "model2.safetensors"], ...] } } } }
    let available_models = info
        .get("CheckpointLoaderSimple")
        .and_then(|node| node.get("input"))
        .and_then(|input| input.get("required"))
        .and_then(|req| req.get("ckpt_name"))
        .and_then(|ckpt| ckpt.get(0))
        .and_then(|list| list.as_array())
        .map(|arr| {
            arr.iter()
                .filter_map(|v| v.as_str().map(String::from))
                .collect::<Vec<String>>()
        })
        .unwrap_or_default();

    let available = available_models.iter().any(|name| name == &model_name);

    Ok(ComfyuiModelStatus {
        available,
        available_models,
    })
}

/// ComfyUI チェックポイントモデルのダウンロード
///
/// 指定URLからモデルファイルをストリーミングダウンロードし，
/// ComfyUI の checkpoints ディレクトリに保存する．
/// ダウンロード中は `comfyui-model-download-progress` イベントで進捗を通知する．
/// 一時ファイルに書き込み後，アトミックにリネームして破損を防ぐ．
#[tauri::command]
pub async fn download_comfyui_model(
    _state: tauri::State<'_, AppState>,
    app: tauri::AppHandle,
    model_name: String,
    download_url: String,
    checkpoints_path: String,
) -> Result<String, AppError> {
    let dest_dir = std::path::Path::new(&checkpoints_path);
    let model_path = dest_dir.join(&model_name);

    // Already exists check
    if model_path.exists() {
        let file_size = std::fs::metadata(&model_path)
            .map(|m| m.len())
            .unwrap_or(0);
        if file_size > 0 {
            return Ok(model_path.to_string_lossy().to_string());
        }
    }

    // Ensure directory exists
    std::fs::create_dir_all(dest_dir)
        .map_err(|e| AppError::Io(format!("Failed to create checkpoints directory: {}", e)))?;

    let temp_path = dest_dir.join(format!("{}.tmp", model_name));

    // Streaming download (same pattern as model_download.rs)
    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(3600)) // 1 hour timeout for large files
        .build()
        .map_err(|e| AppError::Internal(format!("Failed to create HTTP client: {}", e)))?;

    let response = client
        .get(&download_url)
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

        // Progress event
        let _ = app.emit(
            "comfyui-model-download-progress",
            serde_json::json!({
                "downloaded_bytes": downloaded_bytes,
                "total_bytes": total_bytes,
                "percentage": if total_bytes > 0 {
                    (downloaded_bytes as f64 / total_bytes as f64 * 100.0) as u32
                } else {
                    0
                },
                "model_name": model_name,
            }),
        );
    }

    // Flush and close
    std::io::Write::flush(&mut file)
        .map_err(|e| AppError::Io(format!("Flush error: {}", e)))?;
    drop(file);

    // Validate file size
    let file_size = std::fs::metadata(&temp_path)
        .map(|m| m.len())
        .unwrap_or(0);

    if file_size == 0 {
        let _ = std::fs::remove_file(&temp_path);
        return Err(AppError::Io("Downloaded file is empty".into()));
    }

    // Atomic rename
    std::fs::rename(&temp_path, &model_path)
        .map_err(|e| AppError::Io(format!("Rename error: {}", e)))?;

    log::info!(
        "ComfyUI model downloaded: {} ({} bytes)",
        model_path.display(),
        file_size
    );

    Ok(model_path.to_string_lossy().to_string())
}

/// ComfyUI の checkpoints ディレクトリパスを取得
///
/// DB 設定 `comfyui_checkpoints_path` を確認し，未設定の場合はデフォルト値を返す．
#[tauri::command]
pub async fn get_comfyui_checkpoints_path(
    state: tauri::State<'_, AppState>,
) -> Result<String, AppError> {
    let conn = state
        .db
        .lock()
        .map_err(|e| AppError::Internal(e.to_string()))?;

    let path = db::queries::settings::get_setting(&conn, "comfyui_checkpoints_path")?
        .unwrap_or_default();

    Ok(path)
}

/// ComfyUI で利用可能なチェックポイント一覧を取得
#[tauri::command]
pub async fn list_available_checkpoints(
    state: tauri::State<'_, AppState>,
) -> Result<Vec<String>, AppError> {
    let endpoint = {
        let conn = state
            .db
            .lock()
            .map_err(|e| AppError::Internal(e.to_string()))?;
        db::queries::settings::get_setting(&conn, "comfyui_endpoint")?
            .unwrap_or_else(|| "http://127.0.0.1:8188".to_string())
    };

    let client = ComfyUIClient::new(&endpoint);

    let info = client
        .get_object_info("CheckpointLoaderSimple")
        .await
        .map_err(|e| AppError::ComfyUI(format!("チェックポイント一覧の取得に失敗: {}", e)))?;

    let checkpoints = info
        .get("CheckpointLoaderSimple")
        .and_then(|node| node.get("input"))
        .and_then(|input| input.get("required"))
        .and_then(|req| req.get("ckpt_name"))
        .and_then(|ckpt| ckpt.get(0))
        .and_then(|list| list.as_array())
        .map(|arr| {
            arr.iter()
                .filter_map(|v| v.as_str().map(String::from))
                .collect::<Vec<String>>()
        })
        .unwrap_or_default();

    Ok(checkpoints)
}

/// ComfyUI で利用可能な LoRA 一覧を取得
#[tauri::command]
pub async fn list_available_loras(
    state: tauri::State<'_, AppState>,
) -> Result<Vec<String>, AppError> {
    let endpoint = {
        let conn = state
            .db
            .lock()
            .map_err(|e| AppError::Internal(e.to_string()))?;
        db::queries::settings::get_setting(&conn, "comfyui_endpoint")?
            .unwrap_or_else(|| "http://127.0.0.1:8188".to_string())
    };

    let client = ComfyUIClient::new(&endpoint);

    let info = client
        .get_object_info("LoraLoader")
        .await
        .map_err(|e| AppError::ComfyUI(format!("LoRA 一覧の取得に失敗: {}", e)))?;

    let loras = info
        .get("LoraLoader")
        .and_then(|node| node.get("input"))
        .and_then(|input| input.get("required"))
        .and_then(|req| req.get("lora_name"))
        .and_then(|lora| lora.get(0))
        .and_then(|list| list.as_array())
        .map(|arr| {
            arr.iter()
                .filter_map(|v| v.as_str().map(String::from))
                .collect::<Vec<String>>()
        })
        .unwrap_or_default();

    Ok(loras)
}
