use anyhow::Result;
use reqwest::Client;
use serde::Deserialize;
use std::path::Path;
use uuid::Uuid;

/// ComfyUI API クライアント
///
/// ComfyUI の HTTP REST API と連携し，画像アップロード，
/// ワークフロー実行，結果取得を行う．
/// WebSocket は不使用で，ポーリング方式を採用している．
pub struct ComfyUIClient {
    /// ComfyUI のベースURL（例: http://127.0.0.1:8188）
    endpoint: String,
    /// HTTP クライアント（コネクションプール付き）
    client: Client,
    /// クライアント識別子（ComfyUI側でのセッション管理用）
    client_id: String,
}

/// 画像アップロード結果
#[derive(Debug, Clone, Deserialize)]
pub struct UploadResult {
    pub name: String,
    pub subfolder: String,
    #[serde(rename = "type")]
    pub image_type: Option<String>,
}

/// キュー投入レスポンス
#[derive(Debug, Deserialize)]
struct QueueResponse {
    prompt_id: String,
}

/// 出力画像情報
#[derive(Debug, Clone, Deserialize)]
pub struct OutputImage {
    pub filename: String,
    pub subfolder: String,
    #[serde(rename = "type")]
    pub image_type: Option<String>,
}

impl ComfyUIClient {
    /// 新しい ComfyUIClient を作成
    pub fn new(endpoint: &str) -> Self {
        let client = Client::builder()
            .timeout(std::time::Duration::from_secs(30))
            .pool_max_idle_per_host(5)
            .build()
            .expect("Failed to create HTTP client");

        Self {
            endpoint: endpoint.trim_end_matches('/').to_string(),
            client,
            client_id: Uuid::new_v4().to_string(),
        }
    }

    /// ComfyUI サーバーの生存確認
    ///
    /// GET /system_stats へのリクエストが成功するかチェックする．
    /// 接続エラーの場合は false を返し，パニックしない．
    pub async fn health_check(&self) -> Result<bool> {
        let url = format!("{}/system_stats", self.endpoint);

        match self
            .client
            .get(&url)
            .timeout(std::time::Duration::from_secs(5))
            .send()
            .await
        {
            Ok(resp) => Ok(resp.status().is_success()),
            Err(_) => Ok(false),
        }
    }

    /// ComfyUI に画像をアップロード
    ///
    /// POST /upload/image (multipart/form-data) で画像を送信する．
    /// リトライ付き（最大3回，Exponential Backoff）．
    pub async fn upload_image(&self, path: &Path, subfolder: &str) -> Result<UploadResult> {
        let file_bytes = std::fs::read(path)?;
        let filename = path
            .file_name()
            .and_then(|n| n.to_str())
            .ok_or_else(|| anyhow::anyhow!("Invalid file path: {:?}", path))?
            .to_string();

        let endpoint = self.endpoint.clone();
        let client = self.client.clone();
        let subfolder = subfolder.to_string();

        with_retry(3, || {
            let url = format!("{}/upload/image", endpoint);
            let file_bytes = file_bytes.clone();
            let filename = filename.clone();
            let subfolder = subfolder.clone();
            let client = client.clone();

            async move {
                let part = reqwest::multipart::Part::bytes(file_bytes)
                    .file_name(filename)
                    .mime_str("image/png")?;

                let form = reqwest::multipart::Form::new()
                    .part("image", part)
                    .text("subfolder", subfolder)
                    .text("overwrite", "true".to_string());

                let response = client.post(&url).multipart(form).send().await?;

                if !response.status().is_success() {
                    let status = response.status();
                    let body = response.text().await.unwrap_or_default();
                    return Err(anyhow::anyhow!("Upload failed ({}): {}", status, body));
                }

                let result: UploadResult = response.json().await?;
                Ok(result)
            }
        })
        .await
    }

    /// ワークフローを ComfyUI の実行キューに投入
    ///
    /// POST /prompt でワークフローJSONを送信し，prompt_id を返す．
    /// リトライ付き（最大3回，Exponential Backoff）．
    pub async fn queue_prompt(&self, workflow_json: serde_json::Value) -> Result<String> {
        let endpoint = self.endpoint.clone();
        let client = self.client.clone();
        let client_id = self.client_id.clone();

        with_retry(3, || {
            let url = format!("{}/prompt", endpoint);
            let client = client.clone();
            let client_id = client_id.clone();
            let workflow_json = workflow_json.clone();

            async move {
                let body = serde_json::json!({
                    "prompt": workflow_json,
                    "client_id": client_id,
                });

                let response = client.post(&url).json(&body).send().await?;

                if !response.status().is_success() {
                    let status = response.status();
                    let body = response.text().await.unwrap_or_default();
                    return Err(anyhow::anyhow!(
                        "Queue prompt failed ({}): {}",
                        status,
                        body
                    ));
                }

                let result: QueueResponse = response.json().await?;
                log::info!("Queued prompt: {}", result.prompt_id);
                Ok(result.prompt_id)
            }
        })
        .await
    }

    /// 実行履歴から結果を取得
    ///
    /// GET /history/{prompt_id} で実行結果を取得する．
    /// まだ完了していない場合は serde_json::Value を返す．
    pub async fn get_history(&self, prompt_id: &str) -> Result<serde_json::Value> {
        let url = format!("{}/history/{}", self.endpoint, prompt_id);
        let response = self.client.get(&url).send().await?;

        if !response.status().is_success() {
            return Err(anyhow::anyhow!(
                "Failed to get history ({})",
                response.status()
            ));
        }

        let body: serde_json::Value = response.json().await?;
        Ok(body)
    }

    /// 結果画像をバイト列としてダウンロード
    ///
    /// GET /view?filename={}&subfolder={}&type=output で画像を取得する．
    /// リトライ付き（最大3回，Exponential Backoff）．
    pub async fn get_image(&self, filename: &str, subfolder: &str) -> Result<Vec<u8>> {
        let endpoint = self.endpoint.clone();
        let client = self.client.clone();
        let filename = filename.to_string();
        let subfolder = subfolder.to_string();

        with_retry(3, || {
            let url = format!(
                "{}/view?filename={}&subfolder={}&type=output",
                endpoint,
                urlencoding_encode(&filename),
                urlencoding_encode(&subfolder),
            );
            let client = client.clone();
            let filename_clone = filename.clone();
            let subfolder_clone = subfolder.clone();

            async move {
                let response = client.get(&url).send().await?;

                if !response.status().is_success() {
                    let status = response.status();
                    return Err(anyhow::anyhow!(
                        "Failed to download image ({}): {}/{}",
                        status,
                        subfolder_clone,
                        filename_clone
                    ));
                }

                let bytes = response.bytes().await?;
                Ok(bytes.to_vec())
            }
        })
        .await
    }

    /// ComfyUI のノード情報を取得
    ///
    /// GET /object_info/{node_name} でノードの入力パラメータ情報を取得する．
    /// CheckpointLoaderSimple の場合，利用可能なチェックポイント一覧を返す．
    pub async fn get_object_info(&self, node_name: &str) -> Result<serde_json::Value> {
        let url = format!("{}/object_info/{}", self.endpoint, node_name);
        let response = self
            .client
            .get(&url)
            .timeout(std::time::Duration::from_secs(10))
            .send()
            .await?;

        if !response.status().is_success() {
            return Err(anyhow::anyhow!(
                "Failed to get object_info ({})",
                response.status()
            ));
        }

        let body: serde_json::Value = response.json().await?;
        Ok(body)
    }

    /// ポーリングで処理完了を待つ
    ///
    /// 1秒間隔で /history/{prompt_id} をポーリングし，
    /// 完了またはタイムアウトまで待機する．
    /// 完了時は出力画像情報を返す．
    pub async fn wait_for_completion(
        &self,
        prompt_id: &str,
        timeout_secs: u64,
    ) -> Result<serde_json::Value> {
        let start = std::time::Instant::now();
        let timeout = std::time::Duration::from_secs(timeout_secs);
        let interval = std::time::Duration::from_secs(1);

        loop {
            if start.elapsed() > timeout {
                return Err(anyhow::anyhow!(
                    "Timeout waiting for ComfyUI completion ({}s): {}",
                    timeout_secs,
                    prompt_id
                ));
            }

            let history = self.get_history(prompt_id).await?;

            // prompt_id をキーとする履歴オブジェクトを取得
            if let Some(entry) = history.get(prompt_id) {
                // status オブジェクトを確認
                if let Some(status) = entry.get("status") {
                    let completed = status
                        .get("completed")
                        .and_then(|v| v.as_bool())
                        .unwrap_or(false);

                    if completed {
                        let status_str = status
                            .get("status_str")
                            .and_then(|v| v.as_str())
                            .unwrap_or("");

                        if status_str == "error" {
                            return Err(anyhow::anyhow!(
                                "ComfyUI processing failed for prompt: {}",
                                prompt_id
                            ));
                        }

                        return Ok(entry.clone());
                    }
                }
            }

            tokio::time::sleep(interval).await;
        }
    }
}

/// 簡易URLエンコーディング（urlencoding クレート不使用）
fn urlencoding_encode(s: &str) -> String {
    let mut encoded = String::with_capacity(s.len());
    for byte in s.bytes() {
        match byte {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' => {
                encoded.push(byte as char);
            }
            _ => {
                encoded.push_str(&format!("%{:02X}", byte));
            }
        }
    }
    encoded
}

/// リトライ付き非同期操作実行
///
/// Exponential Backoff（1s, 2s, 4s）で最大 max_retries 回リトライする．
async fn with_retry<T, F, Fut>(max_retries: u32, operation: F) -> Result<T>
where
    F: Fn() -> Fut,
    Fut: std::future::Future<Output = Result<T>>,
{
    let mut last_error = None;

    for attempt in 0..=max_retries {
        match operation().await {
            Ok(result) => return Ok(result),
            Err(e) => {
                log::warn!(
                    "Attempt {}/{} failed: {}",
                    attempt + 1,
                    max_retries + 1,
                    e
                );
                last_error = Some(e);

                if attempt < max_retries {
                    let delay = std::time::Duration::from_secs(2u64.pow(attempt));
                    tokio::time::sleep(delay).await;
                }
            }
        }
    }

    Err(last_error.unwrap_or_else(|| anyhow::anyhow!("All retries exhausted")))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_urlencoding_encode() {
        assert_eq!(urlencoding_encode("hello"), "hello");
        assert_eq!(urlencoding_encode("hello world"), "hello%20world");
        assert_eq!(urlencoding_encode("file.png"), "file.png");
        assert_eq!(urlencoding_encode("a/b"), "a%2Fb");
    }

    #[test]
    fn test_comfyui_client_new() {
        let client = ComfyUIClient::new("http://127.0.0.1:8188");
        assert_eq!(client.endpoint, "http://127.0.0.1:8188");
        assert!(!client.client_id.is_empty());
    }

    #[test]
    fn test_comfyui_client_trailing_slash() {
        let client = ComfyUIClient::new("http://127.0.0.1:8188/");
        assert_eq!(client.endpoint, "http://127.0.0.1:8188");
    }
}
