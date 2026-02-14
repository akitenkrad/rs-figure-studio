use std::path::{Path, PathBuf};
use std::time::Duration;

use reqwest::Client;
use serde::{Deserialize, Serialize};
use tokio::time::sleep;

use crate::error::AppError;

// ============================================================
// Cloud API Service Selection
// ============================================================

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CloudApiProvider {
    Pixellab,
    FalAi,
    Replicate,
}

impl CloudApiProvider {
    pub fn from_str(s: &str) -> Self {
        match s {
            "fal_ai" | "fal.ai" => Self::FalAi,
            "replicate" => Self::Replicate,
            _ => Self::Pixellab,
        }
    }
}

// ============================================================
// Shared Request/Response Types
// ============================================================

#[derive(Debug, Clone, Serialize)]
pub struct CloudGenerateRequest {
    pub prompt: String,
    pub negative_prompt: String,
    pub reference_image_base64: Option<String>,
    pub direction: Option<String>,
    pub animation_name: Option<String>,
    pub frame_index: Option<u32>,
    pub ipadapter_weight: Option<f64>,
    pub lora_name: Option<String>,
    pub lora_weight: Option<f64>,
    pub seed: Option<i64>,
    pub steps: Option<u32>,
    pub cfg_scale: Option<f64>,
    pub num_images: u32,
}

#[derive(Debug, Clone)]
pub struct CloudGenerateResult {
    pub image_bytes: Vec<u8>,
    pub index: u32,
}

// ============================================================
// CloudApiClient
// ============================================================

pub struct CloudApiClient {
    http: Client,
    max_retries: u32,
}

impl CloudApiClient {
    pub fn new() -> Self {
        let http = Client::builder()
            .timeout(Duration::from_secs(120))
            .build()
            .expect("Failed to create HTTP client");

        Self {
            http,
            max_retries: 3,
        }
    }

    /// Exponential backoff retry wrapper
    async fn with_retry<F, Fut, T>(&self, operation: F) -> Result<T, AppError>
    where
        F: Fn() -> Fut,
        Fut: std::future::Future<Output = Result<T, AppError>>,
    {
        let mut last_error = AppError::CloudApi("Unknown error".into());
        for attempt in 0..self.max_retries {
            match operation().await {
                Ok(result) => return Ok(result),
                Err(e) => {
                    last_error = e;
                    if attempt < self.max_retries - 1 {
                        let delay = Duration::from_millis(500 * 2u64.pow(attempt));
                        log::warn!(
                            "Cloud API attempt {} failed, retrying in {:?}",
                            attempt + 1,
                            delay
                        );
                        sleep(delay).await;
                    }
                }
            }
        }
        Err(last_error)
    }

    // ================================================================
    // PixelLab API
    // ================================================================

    pub async fn pixellab_generate(
        &self,
        api_key: &str,
        request: &CloudGenerateRequest,
    ) -> Result<Vec<CloudGenerateResult>, AppError> {
        self.with_retry(|| async {
            let mut body = serde_json::json!({
                "prompt": request.prompt,
                "negative_prompt": request.negative_prompt,
                "num_images": request.num_images,
            });

            if let Some(ref img) = request.reference_image_base64 {
                body["reference_image"] = serde_json::json!(img);
            }
            if let Some(ref dir) = request.direction {
                body["direction"] = serde_json::json!(dir);
            }
            if let Some(seed) = request.seed {
                body["seed"] = serde_json::json!(seed);
            }
            if let Some(steps) = request.steps {
                body["steps"] = serde_json::json!(steps);
            }
            if let Some(cfg) = request.cfg_scale {
                body["cfg_scale"] = serde_json::json!(cfg);
            }

            let resp = self
                .http
                .post("https://api.pixellab.ai/v1/generate")
                .header("Authorization", format!("Bearer {}", api_key))
                .json(&body)
                .send()
                .await
                .map_err(|e| AppError::CloudApi(format!("PixelLab request failed: {}", e)))?;

            if !resp.status().is_success() {
                let status = resp.status();
                let text = resp.text().await.unwrap_or_default();
                return Err(AppError::CloudApi(format!(
                    "PixelLab API error {}: {}",
                    status, text
                )));
            }

            let result: PixellabResponse = resp
                .json()
                .await
                .map_err(|e| AppError::CloudApi(format!("PixelLab parse error: {}", e)))?;

            let mut results = Vec::new();
            for (i, img_data) in result.images.into_iter().enumerate() {
                let bytes = base64_decode(&img_data.base64)?;
                results.push(CloudGenerateResult {
                    image_bytes: bytes,
                    index: i as u32,
                });
            }
            Ok(results)
        })
        .await
    }

    pub async fn pixellab_rotate(
        &self,
        api_key: &str,
        request: &CloudGenerateRequest,
    ) -> Result<Vec<CloudGenerateResult>, AppError> {
        self.with_retry(|| async {
            let mut body = serde_json::json!({
                "prompt": request.prompt,
                "negative_prompt": request.negative_prompt,
                "num_images": 1,
            });

            if let Some(ref img) = request.reference_image_base64 {
                body["reference_image"] = serde_json::json!(img);
            }
            if let Some(ref dir) = request.direction {
                body["target_direction"] = serde_json::json!(dir);
            }
            if let Some(weight) = request.ipadapter_weight {
                body["reference_weight"] = serde_json::json!(weight);
            }

            let resp = self
                .http
                .post("https://api.pixellab.ai/v1/rotate")
                .header("Authorization", format!("Bearer {}", api_key))
                .json(&body)
                .send()
                .await
                .map_err(|e| AppError::CloudApi(format!("PixelLab rotate failed: {}", e)))?;

            if !resp.status().is_success() {
                let status = resp.status();
                let text = resp.text().await.unwrap_or_default();
                return Err(AppError::CloudApi(format!(
                    "PixelLab rotate error {}: {}",
                    status, text
                )));
            }

            let result: PixellabResponse = resp
                .json()
                .await
                .map_err(|e| AppError::CloudApi(format!("PixelLab parse error: {}", e)))?;

            let mut results = Vec::new();
            for (i, img_data) in result.images.into_iter().enumerate() {
                let bytes = base64_decode(&img_data.base64)?;
                results.push(CloudGenerateResult {
                    image_bytes: bytes,
                    index: i as u32,
                });
            }
            Ok(results)
        })
        .await
    }

    // ================================================================
    // fal.ai API
    // ================================================================

    pub async fn fal_generate(
        &self,
        api_key: &str,
        request: &CloudGenerateRequest,
    ) -> Result<Vec<CloudGenerateResult>, AppError> {
        self.with_retry(|| async {
            let mut body = serde_json::json!({
                "prompt": request.prompt,
                "negative_prompt": request.negative_prompt,
                "num_images": request.num_images,
                "image_size": "square",
            });

            if let Some(ref img) = request.reference_image_base64 {
                body["image"] = serde_json::json!(format!("data:image/png;base64,{}", img));
            }
            if let Some(seed) = request.seed {
                body["seed"] = serde_json::json!(seed);
            }
            if let Some(steps) = request.steps {
                body["num_inference_steps"] = serde_json::json!(steps);
            }
            if let Some(cfg) = request.cfg_scale {
                body["guidance_scale"] = serde_json::json!(cfg);
            }

            // Submit to queue
            let resp = self
                .http
                .post("https://fal.run/fal-ai/flux/dev")
                .header("Authorization", format!("Key {}", api_key))
                .json(&body)
                .send()
                .await
                .map_err(|e| AppError::CloudApi(format!("fal.ai request failed: {}", e)))?;

            if !resp.status().is_success() {
                let status = resp.status();
                let text = resp.text().await.unwrap_or_default();
                return Err(AppError::CloudApi(format!(
                    "fal.ai API error {}: {}",
                    status, text
                )));
            }

            let result: FalResponse = resp
                .json()
                .await
                .map_err(|e| AppError::CloudApi(format!("fal.ai parse error: {}", e)))?;

            let mut results = Vec::new();
            for (i, img_data) in result.images.into_iter().enumerate() {
                // fal.ai returns URLs — download each
                let img_resp = self
                    .http
                    .get(&img_data.url)
                    .send()
                    .await
                    .map_err(|e| AppError::CloudApi(format!("fal.ai image download failed: {}", e)))?;

                let bytes = img_resp
                    .bytes()
                    .await
                    .map_err(|e| AppError::CloudApi(format!("fal.ai image read failed: {}", e)))?;

                results.push(CloudGenerateResult {
                    image_bytes: bytes.to_vec(),
                    index: i as u32,
                });
            }
            Ok(results)
        })
        .await
    }

    // ================================================================
    // Replicate API
    // ================================================================

    pub async fn replicate_generate(
        &self,
        api_key: &str,
        request: &CloudGenerateRequest,
    ) -> Result<Vec<CloudGenerateResult>, AppError> {
        self.with_retry(|| async {
            let mut input = serde_json::json!({
                "prompt": request.prompt,
                "negative_prompt": request.negative_prompt,
                "num_outputs": request.num_images,
            });

            if let Some(ref img) = request.reference_image_base64 {
                input["image"] = serde_json::json!(format!("data:image/png;base64,{}", img));
            }
            if let Some(seed) = request.seed {
                input["seed"] = serde_json::json!(seed);
            }
            if let Some(steps) = request.steps {
                input["num_inference_steps"] = serde_json::json!(steps);
            }
            if let Some(cfg) = request.cfg_scale {
                input["guidance_scale"] = serde_json::json!(cfg);
            }

            // Create prediction
            let body = serde_json::json!({
                "version": "retro-diffusion",
                "input": input,
            });

            let resp = self
                .http
                .post("https://api.replicate.com/v1/predictions")
                .header("Authorization", format!("Bearer {}", api_key))
                .json(&body)
                .send()
                .await
                .map_err(|e| AppError::CloudApi(format!("Replicate request failed: {}", e)))?;

            if !resp.status().is_success() {
                let status = resp.status();
                let text = resp.text().await.unwrap_or_default();
                return Err(AppError::CloudApi(format!(
                    "Replicate API error {}: {}",
                    status, text
                )));
            }

            let prediction: ReplicateCreateResponse = resp
                .json()
                .await
                .map_err(|e| AppError::CloudApi(format!("Replicate parse error: {}", e)))?;

            // Poll for completion
            let output_urls = self
                .replicate_poll(api_key, &prediction.id)
                .await?;

            let mut results = Vec::new();
            for (i, url) in output_urls.into_iter().enumerate() {
                let img_resp = self
                    .http
                    .get(&url)
                    .send()
                    .await
                    .map_err(|e| AppError::CloudApi(format!("Replicate download failed: {}", e)))?;

                let bytes = img_resp
                    .bytes()
                    .await
                    .map_err(|e| AppError::CloudApi(format!("Replicate read failed: {}", e)))?;

                results.push(CloudGenerateResult {
                    image_bytes: bytes.to_vec(),
                    index: i as u32,
                });
            }
            Ok(results)
        })
        .await
    }

    async fn replicate_poll(
        &self,
        api_key: &str,
        prediction_id: &str,
    ) -> Result<Vec<String>, AppError> {
        let url = format!(
            "https://api.replicate.com/v1/predictions/{}",
            prediction_id
        );

        for _ in 0..120 {
            let resp = self
                .http
                .get(&url)
                .header("Authorization", format!("Bearer {}", api_key))
                .send()
                .await
                .map_err(|e| AppError::CloudApi(format!("Replicate poll failed: {}", e)))?;

            let status_resp: ReplicatePollResponse = resp
                .json()
                .await
                .map_err(|e| AppError::CloudApi(format!("Replicate poll parse: {}", e)))?;

            match status_resp.status.as_str() {
                "succeeded" => {
                    return Ok(status_resp.output.unwrap_or_default());
                }
                "failed" | "canceled" => {
                    let err_msg = status_resp.error.unwrap_or_else(|| "Unknown error".into());
                    return Err(AppError::CloudApi(format!(
                        "Replicate prediction failed: {}",
                        err_msg
                    )));
                }
                _ => {
                    sleep(Duration::from_secs(2)).await;
                }
            }
        }

        Err(AppError::CloudApi(
            "Replicate prediction timed out".into(),
        ))
    }

    // ================================================================
    // Connection Test
    // ================================================================

    pub async fn test_connection(
        &self,
        provider: &CloudApiProvider,
        api_key: &str,
    ) -> Result<bool, AppError> {
        match provider {
            CloudApiProvider::Pixellab => {
                let resp = self
                    .http
                    .get("https://api.pixellab.ai/v1/models")
                    .header("Authorization", format!("Bearer {}", api_key))
                    .send()
                    .await
                    .map_err(|e| AppError::CloudApi(format!("PixelLab connection test: {}", e)))?;
                Ok(resp.status().is_success())
            }
            CloudApiProvider::FalAi => {
                let resp = self
                    .http
                    .get("https://fal.run/health")
                    .header("Authorization", format!("Key {}", api_key))
                    .send()
                    .await
                    .map_err(|e| AppError::CloudApi(format!("fal.ai connection test: {}", e)))?;
                Ok(resp.status().is_success())
            }
            CloudApiProvider::Replicate => {
                let resp = self
                    .http
                    .get("https://api.replicate.com/v1/models")
                    .header("Authorization", format!("Bearer {}", api_key))
                    .send()
                    .await
                    .map_err(|e| {
                        AppError::CloudApi(format!("Replicate connection test: {}", e))
                    })?;
                Ok(resp.status().is_success())
            }
        }
    }

    // ================================================================
    // Utility: Save image bytes to disk
    // ================================================================

    pub fn save_image(
        bytes: &[u8],
        output_dir: &Path,
        filename: &str,
    ) -> Result<PathBuf, AppError> {
        std::fs::create_dir_all(output_dir)
            .map_err(|e| AppError::Io(format!("Failed to create output dir: {}", e)))?;

        let path = output_dir.join(filename);
        std::fs::write(&path, bytes)
            .map_err(|e| AppError::Io(format!("Failed to write image: {}", e)))?;

        Ok(path)
    }
}

// ============================================================
// API Response Types
// ============================================================

#[derive(Debug, Deserialize)]
struct PixellabResponse {
    images: Vec<PixellabImage>,
}

#[derive(Debug, Deserialize)]
struct PixellabImage {
    base64: String,
}

#[derive(Debug, Deserialize)]
struct FalResponse {
    images: Vec<FalImage>,
}

#[derive(Debug, Deserialize)]
struct FalImage {
    url: String,
}

#[derive(Debug, Deserialize)]
struct ReplicateCreateResponse {
    id: String,
}

#[derive(Debug, Deserialize)]
struct ReplicatePollResponse {
    status: String,
    output: Option<Vec<String>>,
    error: Option<String>,
}

// ============================================================
// Utility
// ============================================================

fn base64_decode(input: &str) -> Result<Vec<u8>, AppError> {
    use base64::Engine;
    base64::engine::general_purpose::STANDARD
        .decode(input)
        .map_err(|e| AppError::CloudApi(format!("Base64 decode error: {}", e)))
}
