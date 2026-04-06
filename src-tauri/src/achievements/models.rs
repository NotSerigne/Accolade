use std::path::PathBuf;
use serde::{Serialize, Deserialize};
#[derive(Serialize, Deserialize, Clone)]
pub struct Achievement {
    pub key: String, // Nom de l'achievement du fichier
    pub name: String, // Nom de l'achievement retourné par l'API Steam/serveur
    pub unlocked: bool, // True/False
    pub icon: String,
    pub unlocked_time: Option<u64>, // Timestamp de déblocage, null si pas débloqué
    pub rarity: String,
    pub completionpercentage: String,
    pub desc: String,
}

#[derive(Serialize, Deserialize, Clone)]
pub enum Emulator {
    Goldberg,
    Empress,
    Codex,
    OnlineFix,
    Rune
}
#[derive(Serialize, Deserialize, Clone)]
pub struct Game {
    pub name: String,
    pub steam_id: u32,
    pub game_icon: String,
    pub achievements_total: u32,
    pub achievements: Vec<Achievement>,
    pub path_buf: PathBuf,
    pub emulator: Emulator
}

