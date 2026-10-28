// src-tauri/src/emulators/codex.rs
use crate::achievements::models::{Achievement, Emulator};
use crate::emulators::{ini_get_ci, parse_progress_str, EmulatorParser};
use ini::Ini;
use serde::Deserialize;
use std::path::PathBuf;

#[derive(Deserialize)]
pub struct Parser;

fn normalize_section_name(value: &str) -> String {
    value.trim().trim_matches('"').to_string()
}

fn parse_unlocked(raw: Option<&str>) -> bool {
    let value = raw.unwrap_or("0").trim().to_ascii_lowercase();
    matches!(value.as_str(), "1" | "true" | "yes" | "y" | "on")
}

fn find_section<'a>(achievementfile: &'a Ini, name: &str) -> Option<&'a ini::Properties> {
    let wanted = normalize_section_name(name);

    achievementfile.section(Some(wanted.as_str())).or_else(|| {
        achievementfile.iter().find_map(|(sec, prop)| {
            sec.filter(|current| normalize_section_name(current).eq_ignore_ascii_case(&wanted))
                .map(|_| prop)
        })
    })
}

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
            index.push(normalize_section_name(value));
        }
        let mut achievements: Vec<Achievement> = Vec::new();
        for name in &index {
            let Some(ach_section) = find_section(&achievementfile, name) else {
                continue;
            };
            let achieved = ach_section
                .get("Achieved")
                .or_else(|| ach_section.get("achieved"));
            let unlock_time = ach_section.get("UnlockTime").unwrap_or("0");
            let unlocked_time = unlock_time.parse::<u64>().ok();
            let (progress, max_progress) = parse_progress_str(
                ini_get_ci(ach_section, &["CurProgress"]),
                ini_get_ci(ach_section, &["MaxProgress"]),
            );
            achievements.push(Achievement {
                key: name.clone(),
                name: name.clone(),
                unlocked: parse_unlocked(achieved),
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
        achievements
    }
    fn known_locations(&self) -> Vec<PathBuf> {
        let appdata = std::env::var("APPDATA").unwrap_or_default();
        let public = std::env::var("PUBLIC").unwrap_or_default();
        vec![
            PathBuf::from(&appdata).join("Steam").join("CODEX"),
            PathBuf::from(&public)
                .join("Documents")
                .join("Steam")
                .join("CODEX"),
        ]
    }
    fn emulator(&self) -> Emulator {
        Emulator::Codex
    }
}
