// src-tauri/src/emulators/tenoke.rs
use crate::achievements::models::{Achievement, Emulator};
use crate::emulators::{ini_get_ci, parse_progress_str, EmulatorParser};
use ini::Ini;
use std::path::PathBuf;

pub struct Parser;

fn parse_unlocked(raw: Option<&str>) -> bool {
    let value = raw
        .unwrap_or("false")
        .trim()
        .trim_matches('"')
        .to_ascii_lowercase();
    matches!(value.as_str(), "true" | "1" | "yes" | "y" | "on")
}

fn parse_unlock_time(raw: Option<&str>) -> Option<u64> {
    raw.and_then(|v| v.trim().trim_matches('"').parse::<u64>().ok())
        .filter(|&t| t > 0)
}

/// Parses TENOKE's `user_stats.ini` (with `[ACHIEVEMENTS]` / `[STATS]` sections)
/// or section-per-achievement `achievements.ini`.
pub(crate) fn parse_tenoke_ini(ini: &Ini) -> Vec<Achievement> {
    if let Some(ach_sec) = ini
        .section(Some("ACHIEVEMENTS"))
        .or_else(|| ini.section(Some("Achievements")))
        .or_else(|| ini.section(Some("achievements")))
    {
        let mut out = Vec::new();
        for (raw_key, raw_val) in ach_sec.iter() {
            let key = raw_key.trim().trim_matches('"').to_string();
            if key.is_empty() {
                continue;
            }
            let val = raw_val.trim().trim_matches('"');
            // TENOKE formats values either as `true` / `false`, `{unlocked=true, time=1700000000}`, or `1@1700000000`
            let mut unlocked = false;
            let mut unlocked_time = None;
            let lower = val.to_ascii_lowercase();
            if lower.contains("true")
                || lower == "1"
                || lower.starts_with("1@")
                || lower.starts_with("1,")
            {
                unlocked = true;
            }
            // Extract unix timestamp if present (e.g. `time=1700000` or `1@1700000` or pure >100000 number)
            for token in val.split(|c: char| !c.is_ascii_digit()) {
                if let Ok(ts) = token.parse::<u64>() {
                    if ts > 946_684_800 {
                        unlocked = true;
                        unlocked_time = Some(ts);
                        break;
                    }
                }
            }
            out.push(Achievement {
                key: key.clone(),
                name: key,
                unlocked,
                unlocked_time,
                ..Default::default()
            });
        }
        return out;
    }

    let mut out = Vec::new();
    for (sec, prop) in ini.iter() {
        let Some(sec_name) = sec else { continue };
        let clean = sec_name.trim().trim_matches('"');
        if clean.is_empty()
            || clean.eq_ignore_ascii_case("SteamAchievements")
            || clean.eq_ignore_ascii_case("Stats")
        {
            continue;
        }
        let unlocked = parse_unlocked(ini_get_ci(
            prop,
            &[
                "Achieved",
                "achieved",
                "unlocked",
                "earned",
                "State",
                "HaveAchieved",
            ],
        ));
        let unlocked_time = parse_unlock_time(ini_get_ci(
            prop,
            &[
                "UnlockTime",
                "unlock_time",
                "Time",
                "HaveAchievedTime",
                "timestamp",
            ],
        ));
        let (progress, max_progress) = parse_progress_str(
            ini_get_ci(prop, &["CurProgress", "progress", "cur_progress"]),
            ini_get_ci(prop, &["MaxProgress", "max_progress", "maxprogress"]),
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
        let Ok(ini) = Ini::load_from_file(path) else {
            return Vec::new();
        };
        parse_tenoke_ini(&ini)
    }

    fn known_locations(&self) -> Vec<PathBuf> {
        let appdata = std::env::var("APPDATA").unwrap_or_default();
        let localappdata = std::env::var("LOCALAPPDATA").unwrap_or_default();
        let public = std::env::var("PUBLIC").unwrap_or_default();
        vec![
            PathBuf::from(&appdata).join("TENOKE"),
            PathBuf::from(&localappdata).join("TENOKE"),
            PathBuf::from(&public).join("Documents").join("TENOKE"),
            PathBuf::from(&public)
                .join("Documents")
                .join("Steam")
                .join("TENOKE"),
        ]
    }

    fn emulator(&self) -> Emulator {
        Emulator::Tenoke
    }
}
