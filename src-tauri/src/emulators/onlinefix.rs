// onlinefix: parse le format INI de OnlineFix

use std::path::PathBuf;
use ini::Ini;
use crate::achievements::models::{Achievement, Emulator};
use crate::emulators::EmulatorParser;

pub struct Parser;

fn parse_unlocked(raw: Option<&str>) -> bool {
    let value = raw.unwrap_or("false").trim().to_ascii_lowercase();
    matches!(value.as_str(), "true" | "1" | "yes" | "y" | "on")
}

fn parse_unlock_time(prop: &ini::Properties) -> Option<u64> {
    ["timestamp", "unlock_time", "unlocked_time", "time", "date"]
        .iter()
        .find_map(|key| prop.get(*key))
        .and_then(|raw| raw.trim().parse::<u64>().ok())
        .filter(|&time| time > 0)
}

impl EmulatorParser for Parser {
    fn parse(&self, path: &str) -> Vec<Achievement> {
        let achievementfile = match Ini::load_from_file(path) {
            Ok(achievementfile) => achievementfile,
            Err(_) => return Vec::new(),
        };
        let mut achievements: Vec<Achievement> = Vec::new();
        for (sec, prop) in &achievementfile {
            if let Some(name) = sec {
                let unlocked = parse_unlocked(
                    prop.get("achieved")
                        .or_else(|| prop.get("unlocked"))
                        .or_else(|| prop.get("earned"))
                        .or_else(|| prop.get("done")),
                );
                let unlocked_time = parse_unlock_time(prop);
                achievements.push(Achievement {
                    key: name.to_string(),
                    name: name.to_string(),
                    unlocked,
                    icon: String::new(),
                    icon_gray: String::new(),
                    unlocked_time: unlocked_time,
                    rarity: String::new(),
                    completionpercentage: String::new(),
                    desc: String::new(),
                    hidden: false,
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