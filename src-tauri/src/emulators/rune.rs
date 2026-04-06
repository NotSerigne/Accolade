// rune: parse le format INI de Rune

use std::path::PathBuf;
use ini::Ini;
use serde::{Deserialize};
use crate::achievements::models::{Achievement, Emulator};
use crate::emulators::EmulatorParser;

#[derive(Deserialize)]
pub struct Parser;

impl EmulatorParser for Parser {
    fn parse(&self, path: &PathBuf) -> Vec<Achievement> {
        let achievementfile = Ini::load_from_file(path).unwrap();
        let section = achievementfile.section(Some("SteamAchievements")).unwrap();
        let mut index: Vec<String>  = Vec::new();
        for (_key, value) in section.iter() {
            index.push(value.to_string());
        };
        let mut achievements: Vec<Achievement> = Vec::new();
        for name in &index {
            let ach_section = achievementfile.section(Some(name.as_str())).unwrap();
            let achieved = ach_section.get("Achieved").unwrap();
            let unlock_time = ach_section.get("UnlockTime").unwrap();
            achievements.push(Achievement {
                key: name.clone(),
                name: name.clone(),
                unlocked: achieved == "1",
                icon: String::new(),
                unlocked_time: Some(unlock_time.parse::<u64>().unwrap()),
            });
        }
        achievements
    }
    fn known_locations(&self) -> Vec<PathBuf> {
        let public = std::env::var("PUBLIC").unwrap_or_default();
        vec![
            PathBuf::from(&public).join("Rune-Emulator"),
        ]
    }

    fn emulator(&self) -> Emulator {
        Emulator::Rune
    }
}
