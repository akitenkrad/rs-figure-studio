use std::path::Path;

use async_trait::async_trait;

use crate::error::AppError;
use crate::services::cloud_api_client::{
    CloudApiClient, CloudApiProvider, CloudGenerateRequest, CloudGenerateResult,
};
use crate::services::generation_backend::{
    AnimationParams, ConceptParams, DirectionParams, GeneratedImage, GenerationBackend,
    GenerationProgress, PixelArtConversionParams,
};

// ============================================================
// CloudAPIBackend
// ============================================================

/// クラウド API バックエンド
///
/// PixelLab / fal.ai / Replicate の3サービスから
/// ユーザー設定に基づいて自動選択する．
pub struct CloudApiBackend {
    client: CloudApiClient,
    provider: CloudApiProvider,
    api_key: String,
}

impl CloudApiBackend {
    pub fn new(provider: CloudApiProvider, api_key: String) -> Self {
        Self {
            client: CloudApiClient::new(),
            provider,
            api_key,
        }
    }

    /// 画像ファイルを base64 エンコード
    fn encode_image(path: &Path) -> Result<String, AppError> {
        use base64::Engine;
        let bytes = std::fs::read(path)
            .map_err(|e| AppError::Io(format!("Failed to read image: {}", e)))?;
        Ok(base64::engine::general_purpose::STANDARD.encode(&bytes))
    }

    /// プロバイダに応じた生成リクエストを実行
    async fn generate(
        &self,
        request: &CloudGenerateRequest,
    ) -> Result<Vec<CloudGenerateResult>, AppError> {
        match self.provider {
            CloudApiProvider::Pixellab => {
                self.client
                    .pixellab_generate(&self.api_key, request)
                    .await
            }
            CloudApiProvider::FalAi => {
                self.client.fal_generate(&self.api_key, request).await
            }
            CloudApiProvider::Replicate => {
                self.client
                    .replicate_generate(&self.api_key, request)
                    .await
            }
        }
    }

    /// プロバイダに応じた回転リクエストを実行
    async fn rotate(
        &self,
        request: &CloudGenerateRequest,
    ) -> Result<Vec<CloudGenerateResult>, AppError> {
        match self.provider {
            CloudApiProvider::Pixellab => {
                self.client.pixellab_rotate(&self.api_key, request).await
            }
            // fal.ai / Replicate は回転専用 API がないため通常生成で代替
            _ => self.generate(request).await,
        }
    }
}

#[async_trait]
impl GenerationBackend for CloudApiBackend {
    async fn generate_concept(
        &self,
        params: &ConceptParams,
        output_dir: &Path,
        progress_callback: Box<dyn Fn(GenerationProgress) + Send + Sync>,
    ) -> Result<Vec<GeneratedImage>, AppError> {
        progress_callback(GenerationProgress {
            stage: "concept".into(),
            current: 0,
            total: params.num_candidates,
            status: "processing".into(),
            message: "クラウドAPIでコンセプトを生成中...".into(),
        });

        let request = CloudGenerateRequest {
            prompt: params.positive_prompt.clone(),
            negative_prompt: params.negative_prompt.clone(),
            reference_image_base64: None,
            direction: None,
            animation_name: None,
            frame_index: None,
            ipadapter_weight: None,
            lora_name: None,
            lora_weight: None,
            seed: params.seed,
            steps: params.steps,
            cfg_scale: params.cfg_scale,
            num_images: params.num_candidates,
        };

        let results = self.generate(&request).await?;

        let mut images = Vec::new();
        for result in &results {
            let filename = format!("concept_{}.png", result.index);
            let path =
                CloudApiClient::save_image(&result.image_bytes, output_dir, &filename)?;

            images.push(GeneratedImage {
                path,
                direction: None,
                animation: None,
                frame_index: None,
            });

            progress_callback(GenerationProgress {
                stage: "concept".into(),
                current: result.index + 1,
                total: params.num_candidates,
                status: "processing".into(),
                message: format!("コンセプト {}/{} 完了", result.index + 1, params.num_candidates),
            });
        }

        progress_callback(GenerationProgress {
            stage: "concept".into(),
            current: params.num_candidates,
            total: params.num_candidates,
            status: "completed".into(),
            message: "コンセプト生成完了".into(),
        });

        Ok(images)
    }

    async fn generate_concept_art(
        &self,
        params: &ConceptParams,
        output_dir: &Path,
        progress_callback: Box<dyn Fn(GenerationProgress) + Send + Sync>,
    ) -> Result<Vec<GeneratedImage>, AppError> {
        progress_callback(GenerationProgress {
            stage: "concept_art".into(),
            current: 0,
            total: params.num_candidates,
            status: "processing".into(),
            message: "クラウドAPIでコンセプトアートを生成中...".into(),
        });

        // Modify prompt for concept art style
        let concept_prompt = format!(
            "{}, concept art, detailed illustration, high quality, character design",
            params.positive_prompt
        );
        let concept_negative = format!(
            "{}, pixel art, pixelated, low resolution",
            params.negative_prompt
        );

        let request = CloudGenerateRequest {
            prompt: concept_prompt,
            negative_prompt: concept_negative,
            reference_image_base64: None,
            direction: None,
            animation_name: None,
            frame_index: None,
            ipadapter_weight: None,
            lora_name: None,
            lora_weight: None,
            seed: params.seed,
            steps: params.steps,
            cfg_scale: params.cfg_scale,
            num_images: params.num_candidates,
        };

        let results = self.generate(&request).await?;

        let mut images = Vec::new();
        for result in &results {
            let filename = format!("concept_art_{}.png", result.index);
            let path =
                CloudApiClient::save_image(&result.image_bytes, output_dir, &filename)?;

            images.push(GeneratedImage {
                path,
                direction: None,
                animation: None,
                frame_index: None,
            });

            progress_callback(GenerationProgress {
                stage: "concept_art".into(),
                current: result.index + 1,
                total: params.num_candidates,
                status: "processing".into(),
                message: format!(
                    "コンセプトアート {}/{} 完了",
                    result.index + 1,
                    params.num_candidates
                ),
            });
        }

        progress_callback(GenerationProgress {
            stage: "concept_art".into(),
            current: params.num_candidates,
            total: params.num_candidates,
            status: "completed".into(),
            message: "コンセプトアート生成完了".into(),
        });

        Ok(images)
    }

    async fn convert_to_pixel_art(
        &self,
        concept_art_path: &Path,
        params: &PixelArtConversionParams,
        output_dir: &Path,
        progress_callback: Box<dyn Fn(GenerationProgress) + Send + Sync>,
    ) -> Result<Vec<GeneratedImage>, AppError> {
        progress_callback(GenerationProgress {
            stage: "pixel_art_conversion".into(),
            current: 0,
            total: params.num_candidates,
            status: "processing".into(),
            message: "クラウドAPIでピクセルアート変換中...".into(),
        });

        // Encode concept art as base64 reference
        let ref_base64 = Self::encode_image(concept_art_path)?;

        // Modify prompt for pixel art conversion
        let pixel_prompt = format!(
            "{}, pixel art style, game character sprite, clean pixels, retro game",
            params.positive_prompt
        );

        let request = CloudGenerateRequest {
            prompt: pixel_prompt,
            negative_prompt: params.negative_prompt.clone(),
            reference_image_base64: Some(ref_base64),
            direction: None,
            animation_name: None,
            frame_index: None,
            ipadapter_weight: None,
            lora_name: None,
            lora_weight: None,
            seed: params.seed,
            steps: params.steps,
            cfg_scale: params.cfg_scale,
            num_images: params.num_candidates,
        };

        let results = self.generate(&request).await?;

        let mut images = Vec::new();
        for result in &results {
            let filename = format!("pixel_art_{}.png", result.index);
            let path =
                CloudApiClient::save_image(&result.image_bytes, output_dir, &filename)?;

            images.push(GeneratedImage {
                path,
                direction: None,
                animation: None,
                frame_index: None,
            });

            progress_callback(GenerationProgress {
                stage: "pixel_art_conversion".into(),
                current: result.index + 1,
                total: params.num_candidates,
                status: "processing".into(),
                message: format!(
                    "ピクセルアート変換 {}/{} 完了",
                    result.index + 1,
                    params.num_candidates
                ),
            });
        }

        progress_callback(GenerationProgress {
            stage: "pixel_art_conversion".into(),
            current: params.num_candidates,
            total: params.num_candidates,
            status: "completed".into(),
            message: "ピクセルアート変換完了".into(),
        });

        Ok(images)
    }

    async fn generate_directions(
        &self,
        concept_image: &Path,
        params: &DirectionParams,
        output_dir: &Path,
        progress_callback: Box<dyn Fn(GenerationProgress) + Send + Sync>,
    ) -> Result<Vec<GeneratedImage>, AppError> {
        let total = params.directions.len() as u32;
        let ref_base64 = Self::encode_image(concept_image)?;

        progress_callback(GenerationProgress {
            stage: "direction".into(),
            current: 0,
            total,
            status: "processing".into(),
            message: "方向展開を開始...".into(),
        });

        let mut images = Vec::new();

        for (i, direction) in params.directions.iter().enumerate() {
            let request = CloudGenerateRequest {
                prompt: format!("pixel art character facing {}, game sprite", direction),
                negative_prompt: String::new(),
                reference_image_base64: Some(ref_base64.clone()),
                direction: Some(direction.clone()),
                animation_name: None,
                frame_index: None,
                ipadapter_weight: Some(params.ipadapter_weight),
                lora_name: params.lora_name.clone(),
                lora_weight: params.lora_weight,
                seed: params.seed,
                steps: params.steps,
                cfg_scale: params.cfg_scale,
                num_images: 1,
            };

            let results = self.rotate(&request).await?;

            if let Some(result) = results.first() {
                let filename = format!("direction_{}.png", direction);
                let path =
                    CloudApiClient::save_image(&result.image_bytes, output_dir, &filename)?;

                images.push(GeneratedImage {
                    path,
                    direction: Some(direction.clone()),
                    animation: None,
                    frame_index: None,
                });
            }

            progress_callback(GenerationProgress {
                stage: "direction".into(),
                current: (i + 1) as u32,
                total,
                status: "processing".into(),
                message: format!("{} 方向完了 ({}/{})", direction, i + 1, total),
            });
        }

        progress_callback(GenerationProgress {
            stage: "direction".into(),
            current: total,
            total,
            status: "completed".into(),
            message: "方向展開完了".into(),
        });

        Ok(images)
    }

    async fn generate_animation_frames(
        &self,
        base_pose_image: &Path,
        direction: &str,
        params: &AnimationParams,
        output_dir: &Path,
        progress_callback: Box<dyn Fn(GenerationProgress) + Send + Sync>,
    ) -> Result<Vec<GeneratedImage>, AppError> {
        let ref_base64 = Self::encode_image(base_pose_image)?;

        progress_callback(GenerationProgress {
            stage: "animation".into(),
            current: 0,
            total: params.frame_count,
            status: "processing".into(),
            message: format!("{} {} のフレーム生成開始", direction, params.animation_name),
        });

        let mut images = Vec::new();

        for frame in 0..params.frame_count {
            let request = CloudGenerateRequest {
                prompt: format!(
                    "pixel art character {} animation frame {}, facing {}, game sprite",
                    params.animation_name, frame, direction
                ),
                negative_prompt: String::new(),
                reference_image_base64: Some(ref_base64.clone()),
                direction: Some(direction.to_string()),
                animation_name: Some(params.animation_name.clone()),
                frame_index: Some(frame),
                ipadapter_weight: Some(params.ipadapter_weight),
                lora_name: params.lora_name.clone(),
                lora_weight: params.lora_weight,
                seed: params.seed.map(|s| s + frame as i64),
                steps: params.steps,
                cfg_scale: params.cfg_scale,
                num_images: 1,
            };

            let results = self.generate(&request).await?;

            if let Some(result) = results.first() {
                let filename = format!(
                    "{}_{}_f{:02}.png",
                    direction, params.animation_name, frame
                );
                let path =
                    CloudApiClient::save_image(&result.image_bytes, output_dir, &filename)?;

                images.push(GeneratedImage {
                    path,
                    direction: Some(direction.to_string()),
                    animation: Some(params.animation_name.clone()),
                    frame_index: Some(frame),
                });
            }

            progress_callback(GenerationProgress {
                stage: "animation".into(),
                current: frame + 1,
                total: params.frame_count,
                status: "processing".into(),
                message: format!(
                    "フレーム {}/{} 完了",
                    frame + 1,
                    params.frame_count
                ),
            });
        }

        progress_callback(GenerationProgress {
            stage: "animation".into(),
            current: params.frame_count,
            total: params.frame_count,
            status: "completed".into(),
            message: format!("{} {} のアニメーション生成完了", direction, params.animation_name),
        });

        Ok(images)
    }
}
