// empress: parse le format JSON de Empress

use std::path::PathBuf;
use std::collections::HashMap;
use serde::{Deserialize};
use crate::achievements::models::{Achievement, Emulator};
use crate::emulators::EmulatorParser;

pub struct Parser;

#[derive(Deserialize)]
pub struct EmpressAchievement {
    pub earned: bool,
    pub earned_time: Option<u64>,
}

impl EmulatorParser for Parser {
    fn parse(&self, path: &PathBuf) -> Vec<Achievement> {
        let content = std::fs::read_to_string(path).unwrap();
        let data: HashMap<String, EmpressAchievement> = serde_json::from_str(&content).unwrap();
        data.into_iter().map(|(key, value)| {
            Achievement {
                key: key.clone(),
                name: key, // on a pas encore l'API Steam, on met la clé pour l'instant
                unlocked: value.earned,
                icon: String::new(), // vide pour l'instant
                unlocked_time: value.earned_time,
            }
        }).collect()
    }
    fn known_locations(&self) -> Vec<PathBuf> {
        let appdata = std::env::var("APPDATA").unwrap_or_default();
        vec![
            PathBuf::from(&appdata).join("Empress-Emulator"),
        ]
    }

    fn emulator(&self) -> Emulator {
        Emulator::Empress
    }
}
