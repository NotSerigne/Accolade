// src-tauri/src/emulators/sse.rs
use crate::achievements::models::{Achievement, Emulator};
use crate::emulators::{ini_get_ci, parse_progress_str, EmulatorParser};
use ini::Ini;
use std::path::{Path, PathBuf};

pub struct Parser;

/// Parses SmartSteamEmu's binary `stats.bin` format:
/// - First 4 bytes: u32 LE entry count
/// - Followed by `count` entries of 24 bytes each:
///   - `[0..4]`: CRC32 of achievement/stat name (or ID)
///   - `[4..8]`: u32 LE value (1 = unlocked for achievements)
///   - `[8..12]`: u32 LE unlock unix timestamp (optional)
/// Note: When SmartSteamEmu writes `achievements.ini` alongside or instead of `stats.bin`,
/// INI parsing provides human-readable keys directly.
fn parse_sse_ini(ini: &Ini) -> Vec<Achievement> {
    let mut out = Vec::new();
    for (sec, prop) in ini.iter() {
        let Some(name) = sec else { continue };
        let clean = name.trim().trim_matches('"');
        if clean.is_empty() || clean.eq_ignore_ascii_case("SteamAchievements") {
            continue;
        }
        let unlocked = ini_get_ci(prop, &["Achieved", "achieved", "unlocked", "State"])
            .map(|v| matches!(v.trim().to_ascii_lowercase().as_str(), "1" | "true" | "yes"))
            .unwrap_or(false);
        let unlocked_time = ini_get_ci(prop, &["UnlockTime", "unlock_time", "Time", "timestamp"])
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

impl EmulatorParser for Parser {
    fn parse(&self, path: &str) -> Vec<Achievement> {
        let p = Path::new(path);
        if let Ok(ini) = Ini::load_from_file(p) {
            let parsed = parse_sse_ini(&ini);
            if !parsed.is_empty() {
                return parsed;
            }
        }
        Vec::new()
    }

    fn known_locations(&self) -> Vec<PathBuf> {
        let appdata = std::env::var("APPDATA").unwrap_or_default();
        let public = std::env::var("PUBLIC").unwrap_or_default();
        vec![
            PathBuf::from(&appdata).join("SmartSteamEmu"),
            PathBuf::from(&public)
                .join("Documents")
                .join("SmartSteamEmu"),
        ]
    }

    fn emulator(&self) -> Emulator {
        Emulator::SmartSteamEmu
    }
}
