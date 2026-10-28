// src-tauri/src/emulators/ali213.rs
use crate::achievements::models::{Achievement, Emulator};
use crate::emulators::{ini_get_ci, parse_progress_str, EmulatorParser};
use ini::Ini;
use std::path::PathBuf;

pub struct Parser;

fn parse_unlocked(raw: Option<&str>) -> bool {
    let v = raw
        .unwrap_or("0")
        .trim()
        .trim_matches('"')
        .to_ascii_lowercase();
    matches!(v.as_str(), "1" | "true" | "yes" | "y" | "on")
}

pub(crate) fn parse_ali213_ini(ini: &Ini) -> Vec<Achievement> {
    // Variant A: `[Achievements]` key=value map (also used by 3DM / Valve INI)
    if let Some(sec) = ini
        .section(Some("Achievements"))
        .or_else(|| ini.section(Some("ACHIEVEMENTS")))
    {
        // Only treat as key=value map if there aren't dedicated `[AchName]` sections with `Achieved`/`HaveAchieved`
        let has_dedicated_sections = ini.iter().any(|(s, p)| {
            s.is_some_and(|name| {
                !name.eq_ignore_ascii_case("Achievements")
                    && !name.eq_ignore_ascii_case("SteamAchievements")
            }) && ini_get_ci(p, &["HaveAchieved", "Achieved", "achieved", "unlocked"]).is_some()
        });
        if !has_dedicated_sections {
            let mut out = Vec::new();
            for (k, v) in sec.iter() {
                let key = k.trim().trim_matches('"').to_string();
                if key.is_empty() || key.eq_ignore_ascii_case("Count") {
                    continue;
                }
                let unlocked = parse_unlocked(Some(v));
                out.push(Achievement {
                    key: key.clone(),
                    name: key,
                    unlocked,
                    ..Default::default()
                });
            }
            if !out.is_empty() {
                return out;
            }
        }
    }

    // Variant B: `[Achievement_Name]` with `HaveAchieved=1`, `HaveAchievedTime=...`, `nCurProgress`, `nMaxProgress`
    let mut out = Vec::new();
    for (sec, prop) in ini.iter() {
        let Some(name) = sec else { continue };
        let clean = name.trim().trim_matches('"');
        if clean.is_empty()
            || clean.eq_ignore_ascii_case("SteamAchievements")
            || clean.eq_ignore_ascii_case("Achievements")
            || clean.eq_ignore_ascii_case("Stats")
            || clean.eq_ignore_ascii_case("Settings")
        {
            continue;
        }
        let unlocked = parse_unlocked(ini_get_ci(
            prop,
            &["HaveAchieved", "Achieved", "achieved", "unlocked", "State"],
        ));
        let unlocked_time = ini_get_ci(
            prop,
            &[
                "HaveAchievedTime",
                "UnlockTime",
                "unlock_time",
                "Time",
                "timestamp",
            ],
        )
        .and_then(|v| v.trim().parse::<u64>().ok())
        .filter(|&t| t > 0);
        let (progress, max_progress) = parse_progress_str(
            ini_get_ci(
                prop,
                &["nCurProgress", "CurProgress", "progress", "cur_progress"],
            ),
            ini_get_ci(prop, &["nMaxProgress", "MaxProgress", "max_progress"]),
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
        parse_ali213_ini(&ini)
    }

    fn known_locations(&self) -> Vec<PathBuf> {
        let userprofile = std::env::var("USERPROFILE").unwrap_or_default();
        let public = std::env::var("PUBLIC").unwrap_or_default();
        let appdata = std::env::var("APPDATA").unwrap_or_default();
        vec![
            PathBuf::from(&userprofile).join("Documents").join("VALVE"),
            PathBuf::from(&public).join("Documents").join("VALVE"),
            PathBuf::from(&userprofile).join("Documents").join("ALI213"),
            PathBuf::from(&appdata).join("ALI213"),
        ]
    }

    fn emulator(&self) -> Emulator {
        Emulator::Ali213
    }
}
