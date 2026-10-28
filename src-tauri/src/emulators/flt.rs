// src-tauri/src/emulators/flt.rs
use crate::achievements::models::{Achievement, Emulator};
use crate::emulators::{ini_get_ci, parse_progress_str, EmulatorParser};
use ini::Ini;
use std::path::{Path, PathBuf};

pub struct Parser;

impl EmulatorParser for Parser {
    fn parse(&self, path: &str) -> Vec<Achievement> {
        let p = Path::new(path);
        let file_name = p
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("")
            .to_ascii_lowercase();

        // Format 1: Fairlight `achievements.txt` (one unlocked achievement key per line, optionally `KEY=timestamp`)
        if file_name == "achievements.txt" {
            let Ok(content) = std::fs::read_to_string(p) else {
                return Vec::new();
            };
            let mut out = Vec::new();
            for raw_line in content.lines() {
                let line = raw_line.trim();
                if line.is_empty()
                    || line.starts_with('#')
                    || line.starts_with(';')
                    || line.starts_with("//")
                {
                    continue;
                }
                let (key, ts) =
                    if let Some((k, v)) = line.split_once('=').or_else(|| line.split_once(':')) {
                        (
                            k.trim().trim_matches('"').to_string(),
                            v.trim().parse::<u64>().ok().filter(|&t| t > 0),
                        )
                    } else {
                        (line.trim_matches('"').to_string(), None)
                    };
                if !key.is_empty() {
                    out.push(Achievement {
                        key: key.clone(),
                        name: key,
                        unlocked: true,
                        unlocked_time: ts,
                        ..Default::default()
                    });
                }
            }
            return out;
        }

        // Format 2: `achievements.ini` (CODEX/RUNE or section-per-achievement)
        if let Ok(ini) = Ini::load_from_file(p) {
            let mut out = Vec::new();
            for (sec, prop) in &ini {
                let Some(sec_name) = sec else { continue };
                let clean = sec_name.trim().trim_matches('"');
                if clean.is_empty() || clean.eq_ignore_ascii_case("SteamAchievements") {
                    continue;
                }
                let unlocked = ini_get_ci(prop, &["Achieved", "achieved", "unlocked", "earned"])
                    .map(|v| matches!(v.trim().to_ascii_lowercase().as_str(), "1" | "true" | "yes"))
                    .unwrap_or(false);
                let unlocked_time = ini_get_ci(prop, &["UnlockTime", "unlock_time", "timestamp"])
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
            return out;
        }

        Vec::new()
    }

    fn known_locations(&self) -> Vec<PathBuf> {
        let appdata = std::env::var("APPDATA").unwrap_or_default();
        let userprofile = std::env::var("USERPROFILE").unwrap_or_default();
        let public = std::env::var("PUBLIC").unwrap_or_default();
        vec![
            PathBuf::from(&appdata).join("FLT"),
            PathBuf::from(&userprofile).join("Documents").join("FLT"),
            PathBuf::from(&public).join("Documents").join("FLT"),
        ]
    }

    fn emulator(&self) -> Emulator {
        Emulator::Flt
    }
}
