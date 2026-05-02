// src-tauri/src/emulators/goldberg.rs
use crate::achievements::models::Achievement;
use crate::achievements::models::Emulator;
use crate::emulators::EmulatorParser;
use serde::Deserialize;
use std::collections::HashMap;
use std::path::PathBuf;

pub struct Parser;

#[derive(Deserialize)]
pub struct GoldbergAchievement {
    pub earned: bool,
    pub earned_time: Option<u64>,
}

impl EmulatorParser for Parser {
    fn parse(&self, path: &str) -> Vec<Achievement> {
        let content = match std::fs::read_to_string(path) {
            Ok(content) => content,
            Err(_) => return Vec::new(),
        };
        let data: HashMap<String, GoldbergAchievement> = match serde_json::from_str(&content) {
            Ok(data) => data,
            Err(_) => return Vec::new(),
        };
        data.into_iter()
            .map(|(key, value)| Achievement {
                key: key.clone(),
                name: key,
                unlocked: value.earned,
                icon: String::new(),
                icon_gray: String::new(),
                unlocked_time: value.earned_time,
                rarity: String::new(),
                completionpercentage: String::new(),
                desc: String::new(),
                hidden: false,
            })
            .collect()
    }
    fn known_locations(&self) -> Vec<PathBuf> {
        let appdata = std::env::var("APPDATA").unwrap_or_default();
        vec![
            PathBuf::from(&appdata).join("Goldberg SteamEmu Saves"),
            PathBuf::from(&appdata).join("GSE Saves"),
        ]
    }
    fn emulator(&self) -> Emulator {
        Emulator::Goldberg
    }
}
