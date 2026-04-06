// goldberg: parse le format JSON de Goldberg

use std::path::PathBuf;
use std::collections::HashMap;
use serde::{Deserialize};
use crate::achievements::models::Achievement;
use crate::emulators::EmulatorParser;
use crate::achievements::models::Emulator;

pub struct Parser;

#[derive(Deserialize)]
pub struct GoldbergAchievement {
    pub earned: bool,
    pub earned_time: Option<u64>,
}

impl EmulatorParser for Parser {
    fn parse(&self, path: &PathBuf) -> Vec<Achievement> {
        let content = match std::fs::read_to_string(path) {
            Ok(content) => content,
            Err(_) => return Vec::new(),
        };

        let data: HashMap<String, GoldbergAchievement> = match serde_json::from_str(&content) {
            Ok(data) => data,
            Err(_) => return Vec::new(),
        };
        data.into_iter().map(|(key, value)| {
            Achievement {
                key: key.clone(),
                name: key, // on a pas encore l'API Steam, on met la clé pour l'instant
                unlocked: value.earned,
                icon: String::new(), // vide pour l'instant
                unlocked_time: value.earned_time,
                rarity: String::new(),
                completionpercentage: String::new(),
                desc: String::new(),
            }
        }).collect()
    }
    fn known_locations(&self) -> Vec<PathBuf> {
        let appdata = std::env::var("APPDATA").unwrap_or_default();
        vec![
            PathBuf::from(&appdata).join("Goldberg SteamEmu Saves"),
        ]
    }

    fn emulator(&self) -> Emulator {
        Emulator::Goldberg
    }
}

