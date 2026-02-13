use serde::{Deserialize, Serialize};

/// ComfyUI ワークフロー
///
/// comfyui_workflows テーブルに対応するモデル．
/// プロジェクトに紐付き，AI質感生成のワークフローJSONを保持する．
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Workflow {
    pub id: String,
    pub project_id: String,
    pub name: String,
    pub workflow_json: serde_json::Value,
    pub is_default: bool,
    pub created_at: String,
}

/// ワークフロー作成入力
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateWorkflow {
    pub project_id: String,
    pub name: String,
    pub workflow_json: serde_json::Value,
    pub is_default: Option<bool>,
}

/// ComfyUI バッチ処理パラメータ
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProcessingParams {
    pub positive_prompt: String,
    pub negative_prompt: String,
    pub denoise_strength: f64,
    pub controlnet_weight: f64,
    pub cfg_scale: f64,
    pub steps: u32,
    pub seed: Option<i64>,
}
