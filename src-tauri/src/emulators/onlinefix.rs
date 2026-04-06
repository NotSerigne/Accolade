// onlinefix: parse le format INI de OnlineFix

use std::path::PathBuf;
use ini::Ini;
use crate::achievements::models::{Achievement, Emulator};
use crate::emulators::EmulatorParser;

pub struct Parser;

impl EmulatorParser for Parser {
    fn parse(&self, path: &PathBuf) -> Vec<Achievement> {
        let achievementfile = match Ini::load_from_file(path) {
            Ok(achievementfile) => achievementfile,
            Err(_) => return Vec::new(),
        };
        let mut achievements: Vec<Achievement> = Vec::new();
        for (sec, prop) in &achievementfile {
            if let Some(name) = sec {
                let achieved = prop.get("achieved").unwrap_or("false");
                let unlock_time = prop.get("timestamp").unwrap_or("0");
                achievements.push(Achievement {
                    key: name.to_string(),
                    name: name.to_string(),
                    unlocked: achieved == "true",
                    icon: String::new(),
                    unlocked_time: Some(unlock_time.parse::<u64>().unwrap()),
                    rarity: String::new(),
                    completionpercentage: String::new(),
                    desc: String::new(),
                });
            }
        }
        achievements
    }
    fn known_locations(&self) -> Vec<PathBuf> {
        let public = std::env::var("PUBLIC").unwrap_or_default();
        vec![
            PathBuf::from(&public).join("Documents").join("OnlineFix"),
        ]
    }

    fn emulator(&self) -> Emulator {
        Emulator::OnlineFix
    }
}