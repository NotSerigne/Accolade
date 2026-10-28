// src-tauri/src/emulators/goldberg.rs
use crate::achievements::models::Achievement;
use crate::achievements::models::Emulator;
use crate::emulators::{normalize_progress, EmulatorParser};
use serde::Deserialize;
use std::collections::HashMap;
use std::path::PathBuf;

pub struct Parser;

#[derive(Deserialize)]
pub struct GoldbergAchievement {
    pub earned: bool,
    pub earned_time: Option<u64>,
    /// Written by gbe_fork (GSE) when a game calls `IndicateAchievementProgress`.
    /// Kept as raw JSON so an unexpected type never makes the whole file unreadable.
    #[serde(default)]
    pub progress: Option<serde_json::Value>,
    #[serde(default)]
    pub max_progress: Option<serde_json::Value>,
}

/// Lenient number extraction: accepts JSON numbers and numeric strings.
pub(crate) fn json_number(value: &Option<serde_json::Value>) -> Option<f64> {
    match value.as_ref()? {
        serde_json::Value::Number(n) => n.as_f64(),
        serde_json::Value::String(s) => s.trim().parse::<f64>().ok(),
        _ => None,
    }
}

impl EmulatorParser for Parser {
    fn parse(&self, path: &str) -> Vec<Achievement> {
        let content = match std::fs::read_to_string(path) {
            Ok(content) => content,
            Err(_) => return Vec::new(),
        };
        let data: HashMap<String, GoldbergAchievement> = match serde_json::from_str(&content) {
            Ok(data) => data,
            Err(_) => return Vec::new(),
        };
        data.into_iter()
            .map(|(key, value)| {
                let (progress, max_progress) = normalize_progress(
                    json_number(&value.progress),
                    json_number(&value.max_progress),
                );
                Achievement {
                    key: key.clone(),
                    name: key,
                    unlocked: value.earned,
                    icon: String::new(),
                    icon_gray: String::new(),
                    unlocked_time: value.earned_time,
                    rarity: String::new(),
                    completionpercentage: String::new(),
                    desc: String::new(),
                    hidden: false,
                    progress,
                    max_progress,
                }
            })
            .collect()
    }
    fn known_locations(&self) -> Vec<PathBuf> {
        let appdata = std::env::var("APPDATA").unwrap_or_default();
        vec![
            PathBuf::from(&appdata).join("Goldberg SteamEmu Saves"),
            PathBuf::from(&appdata).join("GSE Saves"),
        ]
    }
    fn emulator(&self) -> Emulator {
        Emulator::Goldberg
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_gse_progress_fields() {
        let dir = std::env::temp_dir().join(format!("accolade_gse_test_{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let file = dir.join("achievements.json");
        std::fs::write(
            &file,
            r#"{
                "ACH_PLAIN": { "earned": true, "earned_time": 1700000000 },
                "ACH_PROGRESS": { "earned": false, "earned_time": 0, "progress": 37, "max_progress": 100 },
                "ACH_STRINGY": { "earned": false, "earned_time": 0, "progress": "2", "max_progress": "5" },
                "ACH_ZERO": { "earned": false, "earned_time": 0, "progress": 0, "max_progress": 0 }
            }"#,
        )
        .unwrap();

        let achs = Parser.parse(file.to_str().unwrap());
        let get = |k: &str| achs.iter().find(|a| a.key == k).unwrap();

        assert_eq!(achs.len(), 4);
        assert!(get("ACH_PLAIN").unlocked);
        assert_eq!(get("ACH_PLAIN").max_progress, None);
        assert_eq!(get("ACH_PROGRESS").progress, Some(37.0));
        assert_eq!(get("ACH_PROGRESS").max_progress, Some(100.0));
        assert_eq!(get("ACH_STRINGY").progress, Some(2.0));
        assert_eq!(get("ACH_ZERO").max_progress, None);

        let _ = std::fs::remove_dir_all(&dir);
    }
}
