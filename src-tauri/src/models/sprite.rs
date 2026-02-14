use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum SpriteStatus {
    #[serde(rename = "generated")]
    Generated,
    #[serde(rename = "raw")]
    Raw,
    #[serde(rename = "ai_processed")]
    AiProcessed,
    #[serde(rename = "bg_removed")]
    BgRemoved,
    #[serde(rename = "finalized")]
    Finalized,
}

impl SpriteStatus {
    pub fn as_str(&self) -> &str {
        match self {
            Self::Generated => "generated",
            Self::Raw => "raw",
            Self::AiProcessed => "ai_processed",
            Self::BgRemoved => "bg_removed",
            Self::Finalized => "finalized",
        }
    }

    pub fn from_str(s: &str) -> anyhow::Result<Self> {
        match s {
            "generated" => Ok(Self::Generated),
            "raw" => Ok(Self::Raw),
            "ai_processed" => Ok(Self::AiProcessed),
            "bg_removed" => Ok(Self::BgRemoved),
            "finalized" => Ok(Self::Finalized),
            _ => Err(anyhow::anyhow!("Invalid sprite status: {}", s)),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Sprite {
    pub id: String,
    pub character_id: String,
    pub direction: String,
    pub animation: String,
    pub frame_index: i32,
    pub raw_path: Option<String>,
    pub processed_path: Option<String>,
    pub final_path: Option<String>,
    pub status: String,
    pub created_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateSprite {
    pub character_id: String,
    pub direction: String,
    pub animation: String,
    pub frame_index: i32,
    pub raw_path: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UpdateSpritePaths {
    pub raw_path: Option<String>,
    pub processed_path: Option<String>,
    pub final_path: Option<String>,
}
