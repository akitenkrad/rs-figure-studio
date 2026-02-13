use std::collections::HashMap;

use tauri::Emitter;

use crate::db;
use crate::error::AppError;
use crate::models::{CreateWorkflow, ProcessingParams, SpriteStatus, UpdateSpritePaths, Workflow};
use crate::services::comfyui_client::ComfyUIClient;
use crate::state::AppState;

// ============================================================
// 接続確認
// ============================================================

/// ComfyUI サーバーへの接続確認
///
/// 指定エンドポイントの ComfyUI に対して GET /system_stats を実行し，
/// 接続可能かどうかを返す．
#[tauri::command]
pub async fn check_comfyui_connection(endpoint: String) -> Result<bool, AppError> {
    let client = ComfyUIClient::new(&endpoint);
    let result = client
        .health_check()
        .await
        .map_err(|e| AppError::ComfyUI(format!("Health check failed: {}", e)))?;
    Ok(result)
}

// ============================================================
// ワークフロー CRUD
// ============================================================

/// ワークフロー一覧取得
///
/// project_id が指定された場合はそのプロジェクトのワークフローのみ，
/// 未指定の場合は全ワークフローを返す．
#[tauri::command]
pub async fn list_workflows(
    state: tauri::State<'_, AppState>,
    project_id: Option<String>,
) -> Result<Vec<Workflow>, AppError> {
    let conn = state
        .db
        .lock()
        .map_err(|e| AppError::Internal(e.to_string()))?;

    let workflows = match project_id {
        Some(pid) => db::queries::workflow::list_workflows(&conn, &pid)?,
        None => db::queries::workflow::list_all_workflows(&conn)?,
    };
    Ok(workflows)
}

/// ワークフロー作成
#[tauri::command]
pub async fn create_workflow(
    state: tauri::State<'_, AppState>,
    input: CreateWorkflow,
) -> Result<Workflow, AppError> {
    // バリデーション
    if input.name.trim().is_empty() {
        return Err(AppError::Validation("Workflow name is required".into()));
    }

    // workflow_json が有効なJSONオブジェクトかチェック
    if !input.workflow_json.is_object() {
        return Err(AppError::Validation(
            "workflow_json must be a JSON object".into(),
        ));
    }

    let conn = state
        .db
        .lock()
        .map_err(|e| AppError::Internal(e.to_string()))?;

    let workflow = db::queries::workflow::create_workflow(&conn, &input)?;
    Ok(workflow)
}

/// ワークフローインポート（JSON文字列をパースして作成）
///
/// ComfyUI からエクスポートした API形式 JSON 文字列を受け取り，
/// バリデーション後にワークフローとして保存する．
/// project_id は任意．未指定の場合は空文字列が使用される．
#[tauri::command]
pub async fn import_workflow(
    state: tauri::State<'_, AppState>,
    name: String,
    json: String,
    project_id: Option<String>,
) -> Result<Workflow, AppError> {
    if name.trim().is_empty() {
        return Err(AppError::Validation("Workflow name is required".into()));
    }

    // JSON パース
    let workflow_json: serde_json::Value =
        serde_json::from_str(&json).map_err(|e| {
            AppError::Validation(format!("Invalid JSON: {}", e))
        })?;

    // オブジェクトであることを確認
    if !workflow_json.is_object() {
        return Err(AppError::Validation(
            "Workflow JSON must be an object".into(),
        ));
    }

    let conn = state
        .db
        .lock()
        .map_err(|e| AppError::Internal(e.to_string()))?;

    let input = CreateWorkflow {
        project_id: project_id.unwrap_or_default(),
        name,
        workflow_json,
        is_default: Some(false),
    };

    let workflow = db::queries::workflow::create_workflow(&conn, &input)?;
    Ok(workflow)
}

/// ワークフロー更新
#[tauri::command]
pub async fn update_workflow(
    state: tauri::State<'_, AppState>,
    id: String,
    name: String,
    workflow_json: String,
) -> Result<(), AppError> {
    if name.trim().is_empty() {
        return Err(AppError::Validation("Workflow name is required".into()));
    }

    let parsed: serde_json::Value = serde_json::from_str(&workflow_json).map_err(|e| {
        AppError::Validation(format!("Invalid JSON: {}", e))
    })?;

    if !parsed.is_object() {
        return Err(AppError::Validation(
            "Workflow JSON must be an object".into(),
        ));
    }

    let conn = state
        .db
        .lock()
        .map_err(|e| AppError::Internal(e.to_string()))?;

    db::queries::workflow::update_workflow(&conn, &id, &name, &parsed)?;
    Ok(())
}

/// ワークフロー削除
#[tauri::command]
pub async fn delete_workflow(
    state: tauri::State<'_, AppState>,
    id: String,
) -> Result<(), AppError> {
    let conn = state
        .db
        .lock()
        .map_err(|e| AppError::Internal(e.to_string()))?;

    db::queries::workflow::delete_workflow(&conn, &id)?;
    Ok(())
}

/// デフォルトワークフロー設定
#[tauri::command]
pub async fn set_default_workflow(
    state: tauri::State<'_, AppState>,
    id: String,
    project_id: String,
) -> Result<(), AppError> {
    let conn = state
        .db
        .lock()
        .map_err(|e| AppError::Internal(e.to_string()))?;

    db::queries::workflow::set_default_workflow(&conn, &id, &project_id)?;
    Ok(())
}

// ============================================================
// プレースホルダ置換
// ============================================================

/// ワークフローJSON 内のプレースホルダを実際の値で置換する
///
/// `{{key}}` 形式のプレースホルダを replacements マップの値に置換する．
/// 数値プレースホルダ（引用符付き `"{{key}}"` と引用符なし `{{key}}`）の両方に対応する．
pub fn replace_placeholders(
    workflow: &serde_json::Value,
    replacements: &HashMap<String, String>,
) -> serde_json::Value {
    let mut json_str = serde_json::to_string(workflow).unwrap_or_default();

    for (key, value) in replacements {
        let placeholder = format!("{{{{{}}}}}", key);
        let quoted_placeholder = format!("\"{}\"", placeholder);

        // 数値型プレースホルダの処理:
        // JSON内で "{{seed}}" のように引用符付きで記述されている場合，
        // 数値として置換するには引用符ごと置換する必要がある
        if is_numeric_value(value) {
            json_str = json_str.replace(&quoted_placeholder, value);
        }

        // 文字列プレースホルダの処理:
        // {{positive_prompt}} → 実際の値
        json_str = json_str.replace(&placeholder, value);
    }

    serde_json::from_str(&json_str).unwrap_or_else(|_| workflow.clone())
}

/// 値が数値として有効かチェック
fn is_numeric_value(s: &str) -> bool {
    s.parse::<f64>().is_ok() || s.parse::<i64>().is_ok()
}

// ============================================================
// バッチ処理
// ============================================================

/// ComfyUI バッチ処理
///
/// 指定キャラクターの全スプライトに対して ComfyUI 質感変換を実行する．
/// 処理フロー（1スプライトあたり）:
///   1. 画像を ComfyUI にアップロード
///   2. プレースホルダ置換でワークフローJSON を構築
///   3. ワークフローをキューに投入
///   4. ポーリングで完了待ち
///   5. 結果画像をダウンロード・保存
///
/// 個別のスプライト処理失敗はスキップし，バッチ全体は継続する．
/// `comfyui-progress` イベントで進捗を通知する．
#[tauri::command]
pub async fn process_batch_comfyui(
    state: tauri::State<'_, AppState>,
    app: tauri::AppHandle,
    character_id: String,
    workflow_id: String,
    params: ProcessingParams,
) -> Result<Vec<String>, AppError> {
    // DB からキャラクター，プロジェクト，スプライト，ワークフロー情報を取得
    let (sprites, character, project, workflow) = {
        let conn = state
            .db
            .lock()
            .map_err(|e| AppError::Internal(e.to_string()))?;

        let character = db::queries::character::get_character(&conn, &character_id)?
            .ok_or_else(|| AppError::NotFound("Character not found".into()))?;

        let project = db::queries::project::get_project(&conn, &character.project_id)?
            .ok_or_else(|| AppError::Internal("Project not found".into()))?;

        let sprites = db::queries::sprite::get_sprites_by_character(&conn, &character_id)?;

        let workflow = db::queries::workflow::get_workflow(&conn, &workflow_id)?
            .ok_or_else(|| AppError::NotFound("Workflow not found".into()))?;

        // processing_jobs レコードを作成
        let job_id = uuid::Uuid::new_v4().to_string();
        let now = chrono::Utc::now().to_rfc3339();
        conn.execute(
            "INSERT INTO processing_jobs (id, character_id, job_type, status, total_items, created_at, updated_at)
             VALUES (?1, ?2, 'comfyui', 'running', ?3, ?4, ?5)",
            rusqlite::params![job_id, character_id, sprites.len() as i32, now, now],
        )
        .map_err(|e| AppError::Database(e.to_string()))?;

        (sprites, character, project, workflow)
    };

    // ComfyUI エンドポイント取得
    let endpoint = {
        let conn = state
            .db
            .lock()
            .map_err(|e| AppError::Internal(e.to_string()))?;
        db::queries::settings::get_setting(&conn, "comfyui_endpoint")?
            .unwrap_or_else(|| "http://127.0.0.1:8188".to_string())
    };

    let client = ComfyUIClient::new(&endpoint);

    // ヘルスチェック
    let is_healthy = client
        .health_check()
        .await
        .map_err(|e| AppError::ComfyUI(format!("Health check failed: {}", e)))?;

    if !is_healthy {
        return Err(AppError::ComfyUI(
            "ComfyUI is not running. Please start ComfyUI first.".into(),
        ));
    }

    // 処理対象: raw_path または processed_path が存在するスプライト
    let target_sprites: Vec<_> = sprites
        .iter()
        .filter(|s| s.raw_path.is_some() || s.processed_path.is_some())
        .collect();

    let total = target_sprites.len() as u32;

    // 出力ディレクトリ作成
    let output_dir = std::path::Path::new(&project.base_path)
        .join(&character.name)
        .join("ai_processed");
    std::fs::create_dir_all(&output_dir)
        .map_err(|e| AppError::Io(format!("Failed to create ai_processed directory: {}", e)))?;

    let mut results = Vec::with_capacity(target_sprites.len());
    let mut completed_count: u32 = 0;

    // seed の決定（指定がなければランダム）
    let base_seed = params.seed.unwrap_or_else(|| {
        use std::time::{SystemTime, UNIX_EPOCH};
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|d| d.as_millis() as i64)
            .unwrap_or(42)
    });

    for (i, sprite) in target_sprites.iter().enumerate() {
        let input_path = match sprite.processed_path.as_ref().or(sprite.raw_path.as_ref()) {
            Some(path) => path.clone(),
            None => continue,
        };

        let src_path = std::path::Path::new(&input_path);
        let filename = match src_path.file_name() {
            Some(f) => f,
            None => {
                log::warn!(
                    "Invalid input path for sprite {}: {}",
                    sprite.id,
                    input_path
                );
                continue;
            }
        };

        let output_path = output_dir.join(filename);
        let output_path_str = output_path.to_string_lossy().to_string();

        // プログレスイベント送信（処理開始）
        let _ = app.emit(
            "comfyui-progress",
            serde_json::json!({
                "current": i + 1,
                "total": total,
                "status": "processing",
                "current_file": input_path,
            }),
        );

        // 個別スプライトの処理（失敗時はスキップ）
        match process_single_sprite(
            &client,
            &input_path,
            &output_path_str,
            &workflow.workflow_json,
            &params,
            base_seed + i as i64,
        )
        .await
        {
            Ok(()) => {
                // DB 更新: processed_path を設定し status を ai_processed に
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
                    &SpriteStatus::AiProcessed,
                );

                completed_count += 1;
                results.push(output_path_str);
            }
            Err(e) => {
                log::warn!(
                    "ComfyUI processing failed for sprite {}: {}",
                    sprite.id,
                    e
                );
            }
        }

        // プログレスイベント送信（処理完了）
        let _ = app.emit(
            "comfyui-progress",
            serde_json::json!({
                "current": i + 1,
                "total": total,
                "status": "completed",
                "current_file": input_path,
            }),
        );
    }

    // processing_jobs の更新
    {
        let conn = state
            .db
            .lock()
            .map_err(|e| AppError::Internal(e.to_string()))?;
        let now = chrono::Utc::now().to_rfc3339();
        let status = if completed_count == total {
            "completed"
        } else if completed_count > 0 {
            "partial"
        } else {
            "failed"
        };
        let _ = conn.execute(
            "UPDATE processing_jobs SET status = ?1, completed_items = ?2, updated_at = ?3
             WHERE character_id = ?4 AND job_type = 'comfyui' AND status = 'running'",
            rusqlite::params![status, completed_count, now, character_id],
        );
    }

    Ok(results)
}

/// 単一スプライトの ComfyUI 処理
///
/// 1. 画像アップロード
/// 2. プレースホルダ置換
/// 3. キュー投入
/// 4. 完了待ち（120秒タイムアウト）
/// 5. 結果画像ダウンロード・保存
async fn process_single_sprite(
    client: &ComfyUIClient,
    input_path: &str,
    output_path: &str,
    workflow_json: &serde_json::Value,
    params: &ProcessingParams,
    seed: i64,
) -> Result<(), AppError> {
    // 1. 画像アップロード
    let upload = client
        .upload_image(std::path::Path::new(input_path), "input")
        .await
        .map_err(|e| AppError::ComfyUI(format!("Upload failed: {}", e)))?;

    log::info!("Uploaded: {} -> {}/{}", input_path, upload.subfolder, upload.name);

    // 2. プレースホルダ置換
    let mut replacements = HashMap::new();
    replacements.insert("input_image".to_string(), upload.name.clone());
    replacements.insert("positive_prompt".to_string(), params.positive_prompt.clone());
    replacements.insert("negative_prompt".to_string(), params.negative_prompt.clone());
    replacements.insert("seed".to_string(), seed.to_string());
    replacements.insert("steps".to_string(), params.steps.to_string());
    replacements.insert("cfg_scale".to_string(), params.cfg_scale.to_string());
    replacements.insert(
        "denoise_strength".to_string(),
        params.denoise_strength.to_string(),
    );
    replacements.insert(
        "controlnet_weight".to_string(),
        params.controlnet_weight.to_string(),
    );

    let resolved_workflow = replace_placeholders(workflow_json, &replacements);

    // 3. キュー投入
    let prompt_id = client
        .queue_prompt(resolved_workflow)
        .await
        .map_err(|e| AppError::ComfyUI(format!("Queue failed: {}", e)))?;

    log::info!("Queued prompt: {}", prompt_id);

    // 4. 完了待ち（120秒タイムアウト）
    let history = client
        .wait_for_completion(&prompt_id, 120)
        .await
        .map_err(|e| AppError::ComfyUI(format!("Processing timeout/error: {}", e)))?;

    // 5. 出力画像を探してダウンロード
    let output_image = find_output_image(&history)
        .ok_or_else(|| AppError::ComfyUI("No output images found".into()))?;

    let image_data = client
        .get_image(&output_image.0, &output_image.1)
        .await
        .map_err(|e| AppError::ComfyUI(format!("Download failed: {}", e)))?;

    // 出力ディレクトリ作成
    if let Some(parent) = std::path::Path::new(output_path).parent() {
        std::fs::create_dir_all(parent)
            .map_err(|e| AppError::Io(format!("Failed to create directory: {}", e)))?;
    }

    std::fs::write(output_path, &image_data)
        .map_err(|e| AppError::Io(format!("Failed to save result image: {}", e)))?;

    log::info!("Saved result: {}", output_path);
    Ok(())
}

/// ComfyUI の履歴レスポンスから出力画像の (filename, subfolder) を取得
fn find_output_image(history: &serde_json::Value) -> Option<(String, String)> {
    // outputs オブジェクトを走査
    let outputs = history.get("outputs")?;
    let outputs_obj = outputs.as_object()?;

    for (_node_id, node_output) in outputs_obj {
        if let Some(images) = node_output.get("images") {
            if let Some(images_arr) = images.as_array() {
                if let Some(image) = images_arr.first() {
                    let filename = image.get("filename")?.as_str()?.to_string();
                    let subfolder = image
                        .get("subfolder")
                        .and_then(|v| v.as_str())
                        .unwrap_or("")
                        .to_string();
                    return Some((filename, subfolder));
                }
            }
        }
    }

    None
}
