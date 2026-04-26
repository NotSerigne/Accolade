// rune: parse le format INI de Rune

use std::path::PathBuf;
use ini::Ini;
use serde::{Deserialize};
use crate::achievements::models::{Achievement, Emulator};
use crate::emulators::EmulatorParser;

#[derive(Deserialize)]
pub struct Parser;

impl EmulatorParser for Parser {
    fn parse(&self, path: &str) -> Vec<Achievement> {
        let achievementfile = match Ini::load_from_file(path) {
            Ok(achievementfile) => achievementfile,
            Err(_) => return Vec::new(),
        };
        let Some(section) = achievementfile.section(Some("SteamAchievements")) else {
            return Vec::new();
        };
        let mut index: Vec<String> = Vec::new();
        for (_key, value) in section.iter() {
            index.push(value.to_string());
        }
        let mut achievements: Vec<Achievement> = Vec::new();
        for name in &index {
            let Some(ach_section) = achievementfile.section(Some(name.as_str())) else {
                continue;
            };
            let achieved = ach_section.get("Achieved").unwrap_or("0");
            let unlock_time = ach_section.get("UnlockTime").unwrap_or("0");
            let unlocked_time = unlock_time.parse::<u64>().ok();
            achievements.push(Achievement {
                key: name.clone(),
                name: name.clone(),
                unlocked: achieved == "1",
                icon: String::new(),
                icon_gray: String::new(),
                unlocked_time: unlocked_time,
                rarity: String::new(),
                completionpercentage: String::new(),
                desc: String::new(),
                hidden: false,
            });
        }
        achievements
    }
    fn known_locations(&self) -> Vec<PathBuf> {
        let public = std::env::var("PUBLIC").unwrap_or_default();
        vec![
            PathBuf::from(&public).join("Documents").join("Steam").join("RUNE"),
        ]
    }
    fn emulator(&self) -> Emulator {
        Emulator::Rune
    }
}