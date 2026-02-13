use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AnimationDef {
    pub name: String,
    pub frame_count: u32,
    pub frame_duration_ms: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Project {
    pub id: String,
    pub name: String,
    pub base_path: String,
    pub tile_width: i32,
    pub tile_height: i32,
    pub directions: Vec<String>,
    pub animations: Vec<AnimationDef>,
    pub style_prompt: String,
    pub negative_prompt: String,
    pub controlnet_weight: f64,
    pub bevy_output_path: Option<String>,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateProject {
    pub name: String,
    pub base_path: String,
    pub tile_width: Option<i32>,
    pub tile_height: Option<i32>,
    pub directions: Option<Vec<String>>,
    pub animations: Option<Vec<AnimationDef>>,
    pub style_prompt: Option<String>,
    pub negative_prompt: Option<String>,
    pub controlnet_weight: Option<f64>,
    pub bevy_output_path: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UpdateProject {
    pub name: Option<String>,
    pub base_path: Option<String>,
    pub tile_width: Option<i32>,
    pub tile_height: Option<i32>,
    pub directions: Option<Vec<String>>,
    pub animations: Option<Vec<AnimationDef>>,
    pub style_prompt: Option<String>,
    pub negative_prompt: Option<String>,
    pub controlnet_weight: Option<f64>,
    pub bevy_output_path: Option<String>,
}
