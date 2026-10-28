// src-tauri/src/emulators/empress.rs
use crate::achievements::models::{Achievement, Emulator};
use crate::emulators::goldberg::json_number;
use crate::emulators::{normalize_progress, EmulatorParser};
use serde::Deserialize;
use std::collections::HashMap;
use std::path::PathBuf;

pub struct Parser;

#[derive(Deserialize)]
pub struct EmpressAchievement {
    pub earned: bool,
    pub earned_time: Option<u64>,
    #[serde(default)]
    pub progress: Option<serde_json::Value>,
    #[serde(default)]
    pub max_progress: Option<serde_json::Value>,
}

impl EmulatorParser for Parser {
    fn parse(&self, path: &str) -> Vec<Achievement> {
        let content = match std::fs::read_to_string(path) {
            Ok(content) => content,
            Err(_) => return Vec::new(),
        };
        let data: HashMap<String, EmpressAchievement> = match serde_json::from_str(&content) {
            Ok(data) => data,
            Err(_) => return Vec::new(),
        };
        data.into_iter()
            .map(|(key, value)| {
                let (progress, max_progress) = normalize_progress(
                    json_number(&value.progress),
                    json_number(&value.max_progress),
                );
                Achievement {
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
                    progress,
                    max_progress,
                }
            })
            .collect()
    }
    fn known_locations(&self) -> Vec<PathBuf> {
        let appdata = std::env::var("APPDATA").unwrap_or_default();
        vec![PathBuf::from(&appdata).join("Empress-Emulator")]
    }
    fn emulator(&self) -> Emulator {
        Emulator::Empress
    }
}
