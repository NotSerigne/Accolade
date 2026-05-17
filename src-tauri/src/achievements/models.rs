// src-tauri/src/achievements/models.rs
use serde::{Deserialize, Serialize};
#[derive(Serialize, Deserialize, Clone, Default)]
pub struct Achievement {
    pub key: String,
    pub name: String,
    pub unlocked: bool,
    pub icon: String,
    pub icon_gray: String,
    pub unlocked_time: Option<u64>,
    pub rarity: String,
    pub completionpercentage: String,
    pub desc: String,
    pub hidden: bool,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Default)]
pub enum Emulator {
    #[default]
    Goldberg,
    Empress,
    Codex,
    OnlineFix,
    Rune,
    Steam,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Default)]
#[serde(tag = "type", content = "value")]
pub enum SourceType {
    Emulator(Emulator),
    RetroAchievements,
    #[default]
    Custom,
}

#[derive(Serialize, Deserialize, Clone, Default)]
pub struct Game {
    pub name: String,
    pub id: String,
    pub steam_id: Option<u32>,
    pub game_icon: String,
    pub steamgrid_icon_url: String,
    pub header_image_url: String,
    pub background_image_url: String,
    pub achievements_total: u32,
    pub achievements: Vec<Achievement>,
    pub path_buf: Option<String>,
    pub source: SourceType,
    #[serde(default)]
    pub is_favorite: bool,
    #[serde(default)]
    pub tags: Vec<String>,
}
