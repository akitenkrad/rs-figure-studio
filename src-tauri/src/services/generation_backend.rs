use std::collections::HashMap;
use std::path::{Path, PathBuf};

use async_trait::async_trait;
use serde::{Deserialize, Serialize};

use crate::error::AppError;
use crate::services::comfyui_client::ComfyUIClient;

// ============================================================
// パラメータ型
// ============================================================

/// コンセプト生成パラメータ
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConceptParams {
    pub positive_prompt: String,
    pub negative_prompt: String,
    pub style_preset: Option<String>,
    pub seed: Option<i64>,
    pub steps: Option<u32>,
    pub cfg_scale: Option<f64>,
    /// チェックポイントモデル名（デフォルト: "sd_xl_base_1.0.safetensors"）
    pub checkpoint_name: Option<String>,
    /// LoRA モデル名（デフォルト: "pixel-art-xl.safetensors"）
    pub lora_name: Option<String>,
    /// LoRA モデル強度（デフォルト: 1.2）
    pub lora_strength_model: Option<f64>,
    /// LoRA CLIP 強度（デフォルト: 1.0）
    pub lora_strength_clip: Option<f64>,
    /// 生成候補数（デフォルト: 3）
    pub num_candidates: u32,
}

/// 方向展開パラメータ
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DirectionParams {
    /// ターゲット方向リスト（例: ["down", "left", "right", "up"]）
    pub directions: Vec<String>,
    /// IP-Adapter FaceID ウェイト（0.7-0.85，デフォルト: 0.8）
    pub ipadapter_weight: f64,
    pub lora_name: Option<String>,
    pub lora_weight: Option<f64>,
    pub seed: Option<i64>,
    pub steps: Option<u32>,
    pub cfg_scale: Option<f64>,
    /// チェックポイントモデル名
    pub checkpoint_name: Option<String>,
    /// ControlNet Depth を使用するか（デフォルト: false）
    pub use_controlnet: Option<bool>,
    /// ControlNet 強度（0.0-1.0，デフォルト: 0.5）
    pub controlnet_strength: Option<f64>,
    /// カスタム深度マップパス（省略時はデフォルト使用）
    pub depth_map_path: Option<String>,
}

/// アニメーション展開パラメータ
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AnimationParams {
    /// アニメーション名（例: "idle", "walk"）
    pub animation_name: String,
    /// 生成フレーム数
    pub frame_count: u32,
    /// IP-Adapter ウェイト
    pub ipadapter_weight: f64,
    pub lora_name: Option<String>,
    pub lora_weight: Option<f64>,
    pub seed: Option<i64>,
    pub steps: Option<u32>,
    pub cfg_scale: Option<f64>,
    /// チェックポイントモデル名
    pub checkpoint_name: Option<String>,
    /// キーフレームインデックス（省略時=全フレーム AI 生成）
    pub keyframes: Option<Vec<u32>>,
    /// 補間方式（"crossfade" | "nearest"，デフォルト: "crossfade"）
    pub interpolation: Option<String>,
}

/// ピクセルアート変換パラメータ（コンセプトアート → ピクセルアート img2img）
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PixelArtConversionParams {
    pub positive_prompt: String,
    pub negative_prompt: String,
    /// デノイズ強度（0.3-0.8，デフォルト: 0.55）
    pub denoise_strength: f64,
    pub seed: Option<i64>,
    pub steps: Option<u32>,
    pub cfg_scale: Option<f64>,
    /// 生成候補数（デフォルト: 3）
    pub num_candidates: u32,
    /// ピクセルグリッド幅（後処理用，省略時はスキップ）
    pub pixel_grid_width: Option<u32>,
    /// ピクセルグリッド高さ
    pub pixel_grid_height: Option<u32>,
    /// チェックポイントモデル名
    pub checkpoint_name: Option<String>,
    /// LoRA モデル名
    pub lora_name: Option<String>,
    /// LoRA モデル強度
    pub lora_strength_model: Option<f64>,
    /// LoRA CLIP 強度
    pub lora_strength_clip: Option<f64>,
}

/// 生成進捗イベント
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GenerationProgress {
    /// 現在のステージ（"concept", "direction", "animation"）
    pub stage: String,
    pub current: u32,
    pub total: u32,
    /// ステータス（"processing", "completed", "error"）
    pub status: String,
    pub message: String,
}

/// 生成された画像情報
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GeneratedImage {
    pub path: PathBuf,
    pub direction: Option<String>,
    pub animation: Option<String>,
    pub frame_index: Option<u32>,
}

// ============================================================
// GenerationBackend trait
// ============================================================

/// AI キャラクター生成の抽象バックエンド
///
/// ComfyUI（ローカル GPU）とクラウド API（GPU 不要）の
/// 2つの実装を統一的に扱うための trait．
#[async_trait]
pub trait GenerationBackend: Send + Sync {
    /// テキストプロンプトからコンセプト画像を生成
    async fn generate_concept(
        &self,
        params: &ConceptParams,
        output_dir: &Path,
        progress_callback: Box<dyn Fn(GenerationProgress) + Send + Sync>,
    ) -> Result<Vec<GeneratedImage>, AppError>;

    /// コンセプト画像から方向バリエーションを生成
    async fn generate_directions(
        &self,
        concept_image: &Path,
        params: &DirectionParams,
        output_dir: &Path,
        progress_callback: Box<dyn Fn(GenerationProgress) + Send + Sync>,
    ) -> Result<Vec<GeneratedImage>, AppError>;

    /// テキストプロンプトからコンセプトアート画像を生成（手描き風・詳細イラスト）
    async fn generate_concept_art(
        &self,
        params: &ConceptParams,
        output_dir: &Path,
        progress_callback: Box<dyn Fn(GenerationProgress) + Send + Sync>,
    ) -> Result<Vec<GeneratedImage>, AppError>;

    /// コンセプトアート画像をピクセルアートに変換（img2img）
    async fn convert_to_pixel_art(
        &self,
        concept_art_path: &Path,
        params: &PixelArtConversionParams,
        output_dir: &Path,
        progress_callback: Box<dyn Fn(GenerationProgress) + Send + Sync>,
    ) -> Result<Vec<GeneratedImage>, AppError>;

    /// 方向ベースポーズからアニメーションフレームを生成
    async fn generate_animation_frames(
        &self,
        base_pose_image: &Path,
        direction: &str,
        params: &AnimationParams,
        output_dir: &Path,
        progress_callback: Box<dyn Fn(GenerationProgress) + Send + Sync>,
    ) -> Result<Vec<GeneratedImage>, AppError>;
}

// ============================================================
// ComfyUIBackend
// ============================================================

/// ComfyUI バックエンドのワークフローテンプレート
struct WorkflowTemplates {
    concept: String,
    concept_art: String,
    pixel_art_conversion: String,
    direction: String,
    animation: String,
}

/// ComfyUI ローカル GPU バックエンド
///
/// 既存の ComfyUIClient を使い，ワークフローテンプレートの
/// プレースホルダを置換して実行する．
pub struct ComfyUIBackend {
    client: ComfyUIClient,
    workflow_templates: WorkflowTemplates,
}

impl ComfyUIBackend {
    pub fn new(endpoint: &str) -> Self {
        let client = ComfyUIClient::new(endpoint);
        const CONCEPT_ART_WORKFLOW: &str =
            include_str!("../../workflows/concept_art_generation.json");
        const PIXEL_ART_WORKFLOW: &str =
            include_str!("../../workflows/pixel_art_conversion.json");

        let workflow_templates = WorkflowTemplates {
            concept: include_str!("../../workflows/concept_generation.json").to_string(),
            concept_art: CONCEPT_ART_WORKFLOW.to_string(),
            pixel_art_conversion: PIXEL_ART_WORKFLOW.to_string(),
            direction: include_str!("../../workflows/direction_expansion.json").to_string(),
            animation: include_str!("../../workflows/animation_expansion.json").to_string(),
        };
        Self {
            client,
            workflow_templates,
        }
    }

    /// テンプレート内のプレースホルダを置換
    fn replace_placeholders(template: &str, replacements: &HashMap<String, String>) -> String {
        let mut result = template.to_string();
        for (key, value) in replacements {
            let placeholder = format!("{{{{{}}}}}", key);
            let quoted_placeholder = format!("\"{}\"", placeholder);

            // 数値型: 引用符ごと置換
            if value.parse::<f64>().is_ok() || value.parse::<i64>().is_ok() {
                result = result.replace(&quoted_placeholder, value);
            }
            // 文字列型
            result = result.replace(&placeholder, value);
        }
        result
    }

    /// ComfyUI の実行結果から出力画像の (filename, subfolder) を取得
    fn find_output_images(history: &serde_json::Value) -> Vec<(String, String)> {
        let mut results = Vec::new();
        if let Some(outputs) = history.get("outputs").and_then(|o| o.as_object()) {
            for (_node_id, node_output) in outputs {
                if let Some(images) = node_output.get("images").and_then(|i| i.as_array()) {
                    for image in images {
                        if let Some(filename) = image.get("filename").and_then(|f| f.as_str()) {
                            let subfolder = image
                                .get("subfolder")
                                .and_then(|s| s.as_str())
                                .unwrap_or("");
                            results.push((filename.to_string(), subfolder.to_string()));
                        }
                    }
                }
            }
        }
        results
    }

    /// seed の決定（指定がなければランダム）
    fn resolve_seed(seed: Option<i64>) -> i64 {
        seed.unwrap_or_else(|| {
            use std::time::{SystemTime, UNIX_EPOCH};
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .map(|d| d.as_millis() as i64)
                .unwrap_or(42)
        })
    }

    /// 単一ワークフローを実行し，結果画像をダウンロード・保存
    async fn execute_workflow_and_save(
        &self,
        workflow_json_str: &str,
        output_path: &Path,
    ) -> Result<(), AppError> {
        let workflow: serde_json::Value = serde_json::from_str(workflow_json_str)
            .map_err(|e| AppError::ComfyUI(format!("Invalid workflow JSON: {}", e)))?;

        let prompt_id = self
            .client
            .queue_prompt(workflow)
            .await
            .map_err(|e| AppError::ComfyUI(format!("Queue failed: {}", e)))?;

        let history = self
            .client
            .wait_for_completion(&prompt_id, 180)
            .await
            .map_err(|e| AppError::ComfyUI(format!("Processing timeout/error: {}", e)))?;

        let output_images = Self::find_output_images(&history);
        let (filename, subfolder) = output_images
            .first()
            .ok_or_else(|| AppError::ComfyUI("No output images found".into()))?;

        let image_data = self
            .client
            .get_image(filename, subfolder)
            .await
            .map_err(|e| AppError::ComfyUI(format!("Download failed: {}", e)))?;

        if let Some(parent) = output_path.parent() {
            std::fs::create_dir_all(parent)
                .map_err(|e| AppError::Io(format!("ディレクトリ作成に失敗: {}", e)))?;
        }

        std::fs::write(output_path, &image_data)
            .map_err(|e| AppError::Io(format!("画像の保存に失敗: {}", e)))?;

        Ok(())
    }
}

#[async_trait]
impl GenerationBackend for ComfyUIBackend {
    async fn generate_concept(
        &self,
        params: &ConceptParams,
        output_dir: &Path,
        progress_callback: Box<dyn Fn(GenerationProgress) + Send + Sync>,
    ) -> Result<Vec<GeneratedImage>, AppError> {
        std::fs::create_dir_all(output_dir)
            .map_err(|e| AppError::Io(format!("出力ディレクトリ作成に失敗: {}", e)))?;

        // ヘルスチェック
        let healthy = self
            .client
            .health_check()
            .await
            .map_err(|e| AppError::ComfyUI(format!("接続確認に失敗: {}", e)))?;
        if !healthy {
            return Err(AppError::ComfyUI(
                "ComfyUI が起動していません．ComfyUI を起動してください．".into(),
            ));
        }

        let total = params.num_candidates;
        let base_seed = Self::resolve_seed(params.seed);
        let mut results = Vec::new();

        for i in 0..total {
            progress_callback(GenerationProgress {
                stage: "concept".into(),
                current: i + 1,
                total,
                status: "processing".into(),
                message: format!("コンセプト画像 {}/{} を生成中...", i + 1, total),
            });

            let mut replacements = HashMap::new();
            replacements.insert("positive_prompt".into(), params.positive_prompt.clone());
            replacements.insert("negative_prompt".into(), params.negative_prompt.clone());
            replacements.insert("seed".into(), (base_seed + i as i64).to_string());
            replacements.insert(
                "steps".into(),
                params.steps.unwrap_or(30).to_string(),
            );
            replacements.insert(
                "cfg_scale".into(),
                params.cfg_scale.unwrap_or(7.0).to_string(),
            );
            replacements.insert(
                "checkpoint_name".into(),
                params.checkpoint_name.clone().unwrap_or_else(|| "sd_xl_base_1.0.safetensors".into()),
            );
            replacements.insert(
                "lora_name".into(),
                params.lora_name.clone().unwrap_or_else(|| "pixel-art-xl.safetensors".into()),
            );
            replacements.insert(
                "lora_strength_model".into(),
                params.lora_strength_model.unwrap_or(1.2).to_string(),
            );
            replacements.insert(
                "lora_strength_clip".into(),
                params.lora_strength_clip.unwrap_or(1.0).to_string(),
            );

            let resolved =
                Self::replace_placeholders(&self.workflow_templates.concept, &replacements);

            let ts = chrono::Utc::now().format("%Y%m%d%H%M%S");
            let output_path = output_dir.join(format!("concept_{}_{:03}.png", ts, i + 1));

            match self
                .execute_workflow_and_save(&resolved, &output_path)
                .await
            {
                Ok(()) => {
                    log::info!("コンセプト画像を保存: {:?}", output_path);
                    results.push(GeneratedImage {
                        path: output_path,
                        direction: None,
                        animation: None,
                        frame_index: None,
                    });
                }
                Err(e) => {
                    log::warn!("コンセプト画像 {} の生成に失敗: {}", i + 1, e);
                }
            }
        }

        progress_callback(GenerationProgress {
            stage: "concept".into(),
            current: total,
            total,
            status: "completed".into(),
            message: format!("コンセプト生成完了: {}/{} 枚成功", results.len(), total),
        });

        Ok(results)
    }

    async fn generate_concept_art(
        &self,
        params: &ConceptParams,
        output_dir: &Path,
        progress_callback: Box<dyn Fn(GenerationProgress) + Send + Sync>,
    ) -> Result<Vec<GeneratedImage>, AppError> {
        std::fs::create_dir_all(output_dir)
            .map_err(|e| AppError::Io(format!("出力ディレクトリ作成に失敗: {}", e)))?;

        // ヘルスチェック
        let healthy = self
            .client
            .health_check()
            .await
            .map_err(|e| AppError::ComfyUI(format!("接続確認に失敗: {}", e)))?;
        if !healthy {
            return Err(AppError::ComfyUI(
                "ComfyUI が起動していません．ComfyUI を起動してください．".into(),
            ));
        }

        let total = params.num_candidates;
        let base_seed = Self::resolve_seed(params.seed);
        let mut results = Vec::new();

        for i in 0..total {
            progress_callback(GenerationProgress {
                stage: "concept_art".into(),
                current: i + 1,
                total,
                status: "processing".into(),
                message: format!("コンセプトアート {}/{} を生成中...", i + 1, total),
            });

            let mut replacements = HashMap::new();
            replacements.insert("positive_prompt".into(), params.positive_prompt.clone());
            replacements.insert("negative_prompt".into(), params.negative_prompt.clone());
            replacements.insert("seed".into(), (base_seed + i as i64).to_string());
            replacements.insert(
                "steps".into(),
                params.steps.unwrap_or(30).to_string(),
            );
            replacements.insert(
                "cfg_scale".into(),
                params.cfg_scale.unwrap_or(7.0).to_string(),
            );
            replacements.insert(
                "checkpoint_name".into(),
                params.checkpoint_name.clone().unwrap_or_else(|| "sd_xl_base_1.0.safetensors".into()),
            );

            let resolved =
                Self::replace_placeholders(&self.workflow_templates.concept_art, &replacements);

            let ts = chrono::Utc::now().format("%Y%m%d%H%M%S");
            let output_path = output_dir.join(format!("concept_art_{}_{:03}.png", ts, i + 1));

            match self
                .execute_workflow_and_save(&resolved, &output_path)
                .await
            {
                Ok(()) => {
                    log::info!("コンセプトアート画像を保存: {:?}", output_path);
                    results.push(GeneratedImage {
                        path: output_path,
                        direction: None,
                        animation: None,
                        frame_index: None,
                    });
                }
                Err(e) => {
                    log::warn!("コンセプトアート画像 {} の生成に失敗: {}", i + 1, e);
                }
            }
        }

        progress_callback(GenerationProgress {
            stage: "concept_art".into(),
            current: total,
            total,
            status: "completed".into(),
            message: format!("コンセプトアート生成完了: {}/{} 枚成功", results.len(), total),
        });

        Ok(results)
    }

    async fn convert_to_pixel_art(
        &self,
        concept_art_path: &Path,
        params: &PixelArtConversionParams,
        output_dir: &Path,
        progress_callback: Box<dyn Fn(GenerationProgress) + Send + Sync>,
    ) -> Result<Vec<GeneratedImage>, AppError> {
        std::fs::create_dir_all(output_dir)
            .map_err(|e| AppError::Io(format!("出力ディレクトリ作成に失敗: {}", e)))?;

        // ヘルスチェック
        let healthy = self
            .client
            .health_check()
            .await
            .map_err(|e| AppError::ComfyUI(format!("接続確認に失敗: {}", e)))?;
        if !healthy {
            return Err(AppError::ComfyUI(
                "ComfyUI が起動していません．ComfyUI を起動してください．".into(),
            ));
        }

        // コンセプトアート画像をアップロード
        let upload = self
            .client
            .upload_image(concept_art_path, "")
            .await
            .map_err(|e| AppError::ComfyUI(format!("画像アップロードに失敗: {}", e)))?;

        let total = params.num_candidates;
        let base_seed = Self::resolve_seed(params.seed);
        let mut results = Vec::new();

        for i in 0..total {
            progress_callback(GenerationProgress {
                stage: "pixel_art_conversion".into(),
                current: i + 1,
                total,
                status: "processing".into(),
                message: format!("ピクセルアート変換 {}/{} を生成中...", i + 1, total),
            });

            let mut replacements = HashMap::new();
            replacements.insert("positive_prompt".into(), params.positive_prompt.clone());
            replacements.insert("negative_prompt".into(), params.negative_prompt.clone());
            replacements.insert("input_image".into(), upload.name.clone());
            replacements.insert(
                "denoise_strength".into(),
                params.denoise_strength.to_string(),
            );
            replacements.insert("seed".into(), (base_seed + i as i64).to_string());
            replacements.insert(
                "steps".into(),
                params.steps.unwrap_or(30).to_string(),
            );
            replacements.insert(
                "cfg_scale".into(),
                params.cfg_scale.unwrap_or(7.0).to_string(),
            );
            replacements.insert(
                "checkpoint_name".into(),
                params.checkpoint_name.clone().unwrap_or_else(|| "sd_xl_base_1.0.safetensors".into()),
            );
            replacements.insert(
                "lora_name".into(),
                params.lora_name.clone().unwrap_or_else(|| "pixel-art-xl.safetensors".into()),
            );
            replacements.insert(
                "lora_strength_model".into(),
                params.lora_strength_model.unwrap_or(1.2).to_string(),
            );
            replacements.insert(
                "lora_strength_clip".into(),
                params.lora_strength_clip.unwrap_or(1.0).to_string(),
            );

            let resolved = Self::replace_placeholders(
                &self.workflow_templates.pixel_art_conversion,
                &replacements,
            );

            let ts = chrono::Utc::now().format("%Y%m%d%H%M%S");
            let output_path = output_dir.join(format!("concept_{}_{:03}.png", ts, i + 1));

            match self
                .execute_workflow_and_save(&resolved, &output_path)
                .await
            {
                Ok(()) => {
                    // ピクセルグリッド整合化（後処理）
                    if let (Some(grid_w), Some(grid_h)) = (params.pixel_grid_width, params.pixel_grid_height) {
                        if grid_w > 0 && grid_h > 0 {
                            if let Ok(img) = image::open(&output_path) {
                                let rgba = img.to_rgba8();
                                let pixelated = crate::commands::image_processing::pixelate_image(&rgba, grid_w, grid_h);
                                let _ = pixelated.save(&output_path);
                                log::info!("ピクセルグリッド整合化を適用: {}x{}", grid_w, grid_h);
                            }
                        }
                    }
                    log::info!("ピクセルアート変換画像を保存: {:?}", output_path);
                    results.push(GeneratedImage {
                        path: output_path,
                        direction: None,
                        animation: None,
                        frame_index: None,
                    });
                }
                Err(e) => {
                    log::warn!("ピクセルアート変換 {} の生成に失敗: {}", i + 1, e);
                }
            }
        }

        progress_callback(GenerationProgress {
            stage: "pixel_art_conversion".into(),
            current: total,
            total,
            status: "completed".into(),
            message: format!(
                "ピクセルアート変換完了: {}/{} 枚成功",
                results.len(),
                total
            ),
        });

        Ok(results)
    }

    async fn generate_directions(
        &self,
        concept_image: &Path,
        params: &DirectionParams,
        output_dir: &Path,
        progress_callback: Box<dyn Fn(GenerationProgress) + Send + Sync>,
    ) -> Result<Vec<GeneratedImage>, AppError> {
        std::fs::create_dir_all(output_dir)
            .map_err(|e| AppError::Io(format!("出力ディレクトリ作成に失敗: {}", e)))?;

        // コンセプト画像をアップロード
        let upload = self
            .client
            .upload_image(concept_image, "")
            .await
            .map_err(|e| AppError::ComfyUI(format!("画像アップロードに失敗: {}", e)))?;

        let total = params.directions.len() as u32;
        let base_seed = Self::resolve_seed(params.seed);
        let mut results = Vec::new();

        for (i, direction) in params.directions.iter().enumerate() {
            progress_callback(GenerationProgress {
                stage: "direction".into(),
                current: i as u32 + 1,
                total,
                status: "processing".into(),
                message: format!("方向 {} ({}/{}) を生成中...", direction, i + 1, total),
            });

            let mut replacements = HashMap::new();
            replacements.insert("reference_image".into(), upload.name.clone());
            replacements.insert("positive_prompt".into(), format!("{} view character", direction));
            replacements.insert("negative_prompt".into(), String::new());
            replacements.insert("direction".into(), direction.clone());
            replacements.insert(
                "ipadapter_weight".into(),
                params.ipadapter_weight.to_string(),
            );
            replacements.insert("seed".into(), (base_seed + i as i64).to_string());
            replacements.insert(
                "steps".into(),
                params.steps.unwrap_or(30).to_string(),
            );
            replacements.insert(
                "cfg_scale".into(),
                params.cfg_scale.unwrap_or(7.0).to_string(),
            );
            replacements.insert(
                "checkpoint_name".into(),
                params.checkpoint_name.clone().unwrap_or_else(|| "sd_xl_base_1.0.safetensors".into()),
            );
            if let Some(ref lora) = params.lora_name {
                replacements.insert("lora_name".into(), lora.clone());
                replacements.insert(
                    "lora_weight".into(),
                    params.lora_weight.unwrap_or(1.0).to_string(),
                );
            }

            let resolved =
                Self::replace_placeholders(&self.workflow_templates.direction, &replacements);

            let output_path = output_dir.join(format!("{}_base.png", direction));

            match self
                .execute_workflow_and_save(&resolved, &output_path)
                .await
            {
                Ok(()) => {
                    log::info!("方向画像を保存: {:?}", output_path);
                    results.push(GeneratedImage {
                        path: output_path,
                        direction: Some(direction.clone()),
                        animation: None,
                        frame_index: None,
                    });
                }
                Err(e) => {
                    log::warn!("方向 {} の生成に失敗: {}", direction, e);
                }
            }
        }

        progress_callback(GenerationProgress {
            stage: "direction".into(),
            current: total,
            total,
            status: "completed".into(),
            message: format!("方向展開完了: {}/{} 方向成功", results.len(), total),
        });

        Ok(results)
    }

    async fn generate_animation_frames(
        &self,
        base_pose_image: &Path,
        direction: &str,
        params: &AnimationParams,
        output_dir: &Path,
        progress_callback: Box<dyn Fn(GenerationProgress) + Send + Sync>,
    ) -> Result<Vec<GeneratedImage>, AppError> {
        std::fs::create_dir_all(output_dir)
            .map_err(|e| AppError::Io(format!("出力ディレクトリ作成に失敗: {}", e)))?;

        // ベースポーズ画像をアップロード
        let upload = self
            .client
            .upload_image(base_pose_image, "")
            .await
            .map_err(|e| AppError::ComfyUI(format!("画像アップロードに失敗: {}", e)))?;

        let total = params.frame_count;
        let base_seed = Self::resolve_seed(params.seed);
        let mut results = Vec::new();

        // キーフレーム補間: keyframes 指定時はキーフレームのみ AI 生成
        let frames_to_generate: Vec<u32> = if let Some(ref kf) = params.keyframes {
            kf.iter().filter(|&&f| f < total).copied().collect()
        } else {
            (0..total).collect()
        };

        for frame_idx in &frames_to_generate {
            let frame_idx = *frame_idx;
            progress_callback(GenerationProgress {
                stage: "animation".into(),
                current: frame_idx + 1,
                total,
                status: "processing".into(),
                message: format!(
                    "{} {} フレーム {}/{} を生成中...",
                    direction,
                    params.animation_name,
                    frame_idx + 1,
                    total
                ),
            });

            let mut replacements = HashMap::new();
            replacements.insert("reference_image".into(), upload.name.clone());
            replacements.insert(
                "positive_prompt".into(),
                format!(
                    "{} view, {} animation, frame {}",
                    direction, params.animation_name, frame_idx
                ),
            );
            replacements.insert("negative_prompt".into(), String::new());
            replacements.insert("direction".into(), direction.to_string());
            replacements.insert("animation_name".into(), params.animation_name.clone());
            replacements.insert("frame_index".into(), frame_idx.to_string());
            replacements.insert(
                "ipadapter_weight".into(),
                params.ipadapter_weight.to_string(),
            );
            // seed 固定でキャラクター一貫性を確保
            replacements.insert("seed".into(), (base_seed + frame_idx as i64).to_string());
            replacements.insert(
                "steps".into(),
                params.steps.unwrap_or(30).to_string(),
            );
            replacements.insert(
                "cfg_scale".into(),
                params.cfg_scale.unwrap_or(7.0).to_string(),
            );
            replacements.insert(
                "checkpoint_name".into(),
                params.checkpoint_name.clone().unwrap_or_else(|| "sd_xl_base_1.0.safetensors".into()),
            );
            if let Some(ref lora) = params.lora_name {
                replacements.insert("lora_name".into(), lora.clone());
                replacements.insert(
                    "lora_weight".into(),
                    params.lora_weight.unwrap_or(1.0).to_string(),
                );
            }

            let resolved =
                Self::replace_placeholders(&self.workflow_templates.animation, &replacements);

            let output_path = output_dir.join(format!(
                "{}_{}_{}_{:02}.png",
                direction, params.animation_name, "frame", frame_idx
            ));

            match self
                .execute_workflow_and_save(&resolved, &output_path)
                .await
            {
                Ok(()) => {
                    log::info!("アニメーションフレームを保存: {:?}", output_path);
                    results.push(GeneratedImage {
                        path: output_path,
                        direction: Some(direction.to_string()),
                        animation: Some(params.animation_name.clone()),
                        frame_index: Some(frame_idx),
                    });
                }
                Err(e) => {
                    log::warn!(
                        "{} {} フレーム {} の生成に失敗: {}",
                        direction,
                        params.animation_name,
                        frame_idx,
                        e
                    );
                }
            }
        }

        // キーフレーム補間が有効な場合，中割りフレームを生成
        if params.keyframes.is_some() && results.len() < total as usize {
            let method_str = params.interpolation.as_deref().unwrap_or("crossfade");
            let method = crate::services::frame_interpolation::InterpolationMethod::from_str(method_str);

            // キーフレーム画像を読み込み
            let mut keyframe_data: Vec<(u32, image::RgbaImage)> = Vec::new();
            for img in &results {
                if let Some(idx) = img.frame_index {
                    if let Ok(loaded) = image::open(&img.path) {
                        keyframe_data.push((idx, loaded.to_rgba8()));
                    }
                }
            }
            keyframe_data.sort_by_key(|(idx, _)| *idx);

            // 補間実行
            match crate::services::frame_interpolation::interpolate_frames(&keyframe_data, total, &method) {
                Ok(interpolated) => {
                    for (frame_idx, frame_img) in interpolated.iter().enumerate() {
                        let fidx = frame_idx as u32;
                        // キーフレームはスキップ（既に生成済み）
                        if results.iter().any(|r| r.frame_index == Some(fidx)) {
                            continue;
                        }
                        let interp_path = output_dir.join(format!(
                            "{}_{}_{}_{:02}.png",
                            direction, params.animation_name, "frame", fidx
                        ));
                        if let Err(e) = frame_img.save(&interp_path) {
                            log::warn!("補間フレーム {} の保存に失敗: {}", fidx, e);
                            continue;
                        }
                        results.push(GeneratedImage {
                            path: interp_path,
                            direction: Some(direction.to_string()),
                            animation: Some(params.animation_name.clone()),
                            frame_index: Some(fidx),
                        });
                    }
                    // Sort by frame index
                    results.sort_by_key(|r| r.frame_index.unwrap_or(0));
                }
                Err(e) => {
                    log::warn!("キーフレーム補間に失敗: {}", e);
                }
            }
        }

        progress_callback(GenerationProgress {
            stage: "animation".into(),
            current: total,
            total,
            status: "completed".into(),
            message: format!(
                "アニメーション展開完了: {}/{} フレーム成功",
                results.len(),
                total
            ),
        });

        Ok(results)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_replace_string_placeholder() {
        let template = r#"{"text": "{{positive_prompt}}, pixel art"}"#;
        let mut replacements = HashMap::new();
        replacements.insert("positive_prompt".into(), "a cute cat".into());
        let result = ComfyUIBackend::replace_placeholders(template, &replacements);
        assert_eq!(result, r#"{"text": "a cute cat, pixel art"}"#);
    }

    #[test]
    fn test_replace_numeric_placeholder() {
        let template = r#"{"steps": "{{steps}}", "cfg": "{{cfg_scale}}"}"#;
        let mut replacements = HashMap::new();
        replacements.insert("steps".into(), "30".into());
        replacements.insert("cfg_scale".into(), "7.5".into());
        let result = ComfyUIBackend::replace_placeholders(template, &replacements);
        // Numeric values should have quotes removed
        assert_eq!(result, r#"{"steps": 30, "cfg": 7.5}"#);
    }

    #[test]
    fn test_unknown_placeholder_preserved() {
        let template = r#"{"value": "{{unknown_key}}"}"#;
        let replacements = HashMap::new();
        let result = ComfyUIBackend::replace_placeholders(template, &replacements);
        assert_eq!(result, r#"{"value": "{{unknown_key}}"}"#);
    }
}