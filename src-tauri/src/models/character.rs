use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Character {
    pub id: String,
    pub project_id: String,
    pub name: String,
    pub category: String,
    pub custom_prompt: Option<String>,
    pub status: String,
    pub spritesheet_path: Option<String>,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateCharacter {
    pub project_id: String,
    pub name: String,
    pub category: Option<String>,
    pub custom_prompt: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UpdateCharacter {
    pub name: Option<String>,
    pub category: Option<String>,
    pub custom_prompt: Option<String>,
}
