// src-tauri/src/emulators/mod.rs
use crate::achievements::models::Achievement;
use crate::achievements::models::Emulator;
use crate::achievements::models::Game;
use crate::achievements::models::SourceType;
use std::path::PathBuf;

pub(crate) mod ali213;
pub(crate) mod codex;
pub(crate) mod creamapi;
pub(crate) mod empress;
pub(crate) mod flt;
pub(crate) mod goldberg;
pub(crate) mod onlinefix;
pub(crate) mod rune;
pub(crate) mod sse;
pub(crate) mod tenoke;

pub trait EmulatorParser {
    fn parse(&self, path: &str) -> Vec<Achievement>;
    fn known_locations(&self) -> Vec<PathBuf>;
    fn emulator(&self) -> Emulator;
}

pub fn default_parsers() -> Vec<Box<dyn EmulatorParser>> {
    vec![
        Box::new(goldberg::Parser),
        Box::new(empress::Parser),
        Box::new(onlinefix::Parser),
        Box::new(rune::Parser),
        Box::new(codex::Parser),
        Box::new(tenoke::Parser),
        Box::new(flt::Parser),
        Box::new(ali213::Parser),
        Box::new(sse::Parser),
        Box::new(creamapi::Parser),
    ]
}

/// Normalizes a raw (current, max) progress pair read from an emulator file.
///
/// Emulators write `0`/`0` (or nothing at all) for achievements that are not
/// progress-based, so progress is only reported when the target is strictly
/// positive. The current value is clamped to `[0, max]`.
pub(crate) fn normalize_progress(
    current: Option<f64>,
    max: Option<f64>,
) -> (Option<f64>, Option<f64>) {
    match max {
        Some(max) if max.is_finite() && max > 0.0 => {
            let current = current
                .filter(|v| v.is_finite())
                .unwrap_or(0.0)
                .clamp(0.0, max);
            (Some(current), Some(max))
        }
        _ => (None, None),
    }
}

/// Same as [`normalize_progress`] but from raw INI string values.
pub(crate) fn parse_progress_str(
    current: Option<&str>,
    max: Option<&str>,
) -> (Option<f64>, Option<f64>) {
    let parse =
        |raw: Option<&str>| raw.and_then(|v| v.trim().trim_matches('"').parse::<f64>().ok());
    normalize_progress(parse(current), parse(max))
}

/// Case-insensitive lookup of the first matching key in an INI section.
pub(crate) fn ini_get_ci<'a>(prop: &'a ini::Properties, keys: &[&str]) -> Option<&'a str> {
    keys.iter().find_map(|wanted| {
        prop.iter()
            .find(|(k, _)| k.trim().eq_ignore_ascii_case(wanted))
            .map(|(_, v)| v)
    })
}

pub fn game_scanner(parsers: Vec<Box<dyn EmulatorParser>>) -> Vec<Game> {
    let mut games: Vec<Game> = Vec::new();

    for parser in parsers {
        for base_path in parser.known_locations() {
            if base_path.exists() {
                games.extend(find_achievements(&base_path, &parser));
            }
        }
    }

    games
}

fn find_achievements(path: &PathBuf, parser: &Box<dyn EmulatorParser>) -> Vec<Game> {
    let mut games = Vec::new();

    if let Ok(entries) = std::fs::read_dir(path) {
        for entry in entries.flatten() {
            let entry_path = entry.path();

            if is_achievements_file(&entry_path) {
                let achievements = parser.parse(entry_path.to_str().unwrap_or(""));
                let steam_id = extract_steam_id_from_path(&entry_path.to_string_lossy());
                if steam_id == 0 {
                    continue;
                }
                games.push(Game {
                    name: String::new(),
                    id: format!("steam_{}", steam_id),
                    steam_id: Some(steam_id),
                    game_icon: String::new(),
                    steamgrid_icon_url: String::new(),
                    header_image_url: String::new(),
                    background_image_url: String::new(),
                    achievements_total: 0,
                    achievements,
                    path_buf: Some(entry_path.to_string_lossy().to_string()),
                    source: SourceType::Emulator(parser.emulator()),
                    genres: Vec::new(),
                    is_favorite: false,
                    tags: Vec::new(),
                    playtime_seconds: 0,
                });
            } else if entry_path.is_dir() {
                games.extend(find_achievements(&entry_path, parser));
            }
        }
    }

    games
}

fn is_achievements_file(path: &PathBuf) -> bool {
    if !path.is_file() {
        return false;
    }

    let filename = path
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or("")
        .to_ascii_lowercase();

    matches!(
        filename.as_str(),
        "achievements.json"
            | "achievement.json"
            | "achievements.ini"
            | "user_stats.ini"
            | "achievements.txt"
    )
}

fn extract_steam_id_from_path(path: &str) -> u32 {
    let parts: Vec<&str> = path.split(['\\', '/']).collect();

    for part in parts.iter().rev() {
        if let Ok(id) = part.parse::<u32>() {
            return id;
        }
        // Also support `SteamUserID_2623190` or `AppID_2623190` folder conventions
        for token in part.split(['_', '-', ' ']) {
            if token.len() >= 3 {
                if let Ok(id) = token.parse::<u32>() {
                    return id;
                }
            }
        }
    }

    0
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn progress_ignored_when_max_is_zero_or_missing() {
        assert_eq!(parse_progress_str(Some("0"), Some("0")), (None, None));
        assert_eq!(parse_progress_str(Some("5"), None), (None, None));
        assert_eq!(parse_progress_str(None, None), (None, None));
        assert_eq!(parse_progress_str(Some("abc"), Some("xyz")), (None, None));
    }

    #[test]
    fn progress_parsed_and_clamped() {
        assert_eq!(
            parse_progress_str(Some("37"), Some("100")),
            (Some(37.0), Some(100.0))
        );
        assert_eq!(
            parse_progress_str(Some("150"), Some("100")),
            (Some(100.0), Some(100.0))
        );
        assert_eq!(
            parse_progress_str(Some("-3"), Some("10")),
            (Some(0.0), Some(10.0))
        );
        assert_eq!(
            parse_progress_str(None, Some("10")),
            (Some(0.0), Some(10.0))
        );
        assert_eq!(
            parse_progress_str(Some(" 2.5 "), Some("\"5\"")),
            (Some(2.5), Some(5.0))
        );
    }

    #[test]
    fn tenoke_parses_user_stats_and_section_ini() {
        let raw = r#"
[ACHIEVEMENTS]
"ACH_FIRST_BLOOD" = {unlocked = true, time = 1710000123}
"ACH_LOCKED" = false
"#;
        let ini = ini::Ini::load_from_str(raw).unwrap();
        let achs = tenoke::parse_tenoke_ini(&ini);
        assert_eq!(achs.len(), 2);
        let first = achs.iter().find(|a| a.key == "ACH_FIRST_BLOOD").unwrap();
        assert!(first.unlocked);
        assert_eq!(first.unlocked_time, Some(1710000123));
        let second = achs.iter().find(|a| a.key == "ACH_LOCKED").unwrap();
        assert!(!second.unlocked);
    }

    #[test]
    fn ali213_parses_have_achieved_and_progress() {
        let raw = r#"
[ACH_COLLECTOR]
HaveAchieved=1
HaveAchievedTime=1700000555
nCurProgress=42
nMaxProgress=100
"#;
        let ini = ini::Ini::load_from_str(raw).unwrap();
        let achs = ali213::parse_ali213_ini(&ini);
        assert_eq!(achs.len(), 1);
        assert!(achs[0].unlocked);
        assert_eq!(achs[0].unlocked_time, Some(1700000555));
        assert_eq!(achs[0].progress, Some(42.0));
        assert_eq!(achs[0].max_progress, Some(100.0));
    }
}
