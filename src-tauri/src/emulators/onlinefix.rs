// src-tauri/src/emulators/onlinefix.rs
use crate::achievements::models::{Achievement, Emulator};
use crate::emulators::{ini_get_ci, parse_progress_str, EmulatorParser};
use ini::Ini;
use std::path::PathBuf;

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
                // OnlineFix's progress format is not documented; accept the common
                // key spellings used by other emulators. Absent keys => no progress.
                let (progress, max_progress) = parse_progress_str(
                    ini_get_ci(prop, &["progress", "CurProgress", "cur_progress"]),
                    ini_get_ci(prop, &["max_progress", "MaxProgress", "maxprogress"]),
                );
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
                    progress,
                    max_progress,
                });
            }
        }
        achievements
    }
    fn known_locations(&self) -> Vec<PathBuf> {
        let public = std::env::var("PUBLIC").unwrap_or_default();
        vec![PathBuf::from(&public).join("Documents").join("OnlineFix")]
    }
    fn emulator(&self) -> Emulator {
        Emulator::OnlineFix
    }
}
