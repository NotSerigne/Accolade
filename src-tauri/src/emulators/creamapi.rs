// src-tauri/src/emulators/creamapi.rs
use crate::achievements::models::{Achievement, Emulator};
use crate::emulators::{ini_get_ci, parse_progress_str, EmulatorParser};
use ini::Ini;
use std::path::PathBuf;

pub struct Parser;

impl EmulatorParser for Parser {
    fn parse(&self, path: &str) -> Vec<Achievement> {
        let Ok(ini) = Ini::load_from_file(path) else {
            return Vec::new();
        };
        let mut out = Vec::new();
        for (sec, prop) in &ini {
            let Some(name) = sec else { continue };
            let clean = name.trim().trim_matches('"');
            if clean.is_empty() || clean.eq_ignore_ascii_case("SteamAchievements") {
                continue;
            }
            let unlocked = ini_get_ci(prop, &["achieved", "Achieved", "unlocked", "earned"])
                .map(|v| matches!(v.trim().to_ascii_lowercase().as_str(), "1" | "true" | "yes"))
                .unwrap_or(false);
            let unlocked_time = ini_get_ci(
                prop,
                &["unlocktime", "UnlockTime", "unlock_time", "timestamp"],
            )
            .and_then(|v| v.trim().parse::<u64>().ok())
            .filter(|&t| t > 0);
            let (progress, max_progress) = parse_progress_str(
                ini_get_ci(prop, &["CurProgress", "progress"]),
                ini_get_ci(prop, &["MaxProgress", "max_progress"]),
            );
            out.push(Achievement {
                key: clean.to_string(),
                name: clean.to_string(),
                unlocked,
                unlocked_time,
                progress,
                max_progress,
                ..Default::default()
            });
        }
        out
    }

    fn known_locations(&self) -> Vec<PathBuf> {
        let appdata = std::env::var("APPDATA").unwrap_or_default();
        vec![PathBuf::from(&appdata).join("CreamAPI")]
    }

    fn emulator(&self) -> Emulator {
        Emulator::CreamApi
    }
}
