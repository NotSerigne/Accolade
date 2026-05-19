// src-tauri/src/emulators/mod.rs
use crate::achievements::models::Achievement;
use crate::achievements::models::Emulator;
use crate::achievements::models::Game;
use crate::achievements::models::SourceType;
use std::path::PathBuf;

pub(crate) mod codex;
pub(crate) mod empress;
pub(crate) mod goldberg;
pub(crate) mod onlinefix;
pub(crate) mod rune;

pub trait EmulatorParser {
    fn parse(&self, path: &str) -> Vec<Achievement>;
    fn known_locations(&self) -> Vec<PathBuf>;
    fn emulator(&self) -> Emulator;
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

    let filename = path.file_name().and_then(|n| n.to_str()).unwrap_or("");

    filename == "achievements.json"
        || filename == "achievement.json"
        || filename == "achievements.ini"
}

fn extract_steam_id_from_path(path: &str) -> u32 {
    let parts: Vec<&str> = path.split('\\').collect();

    for part in parts.iter().rev() {
        if let Ok(id) = part.parse::<u32>() {
            return id;
        }
    }

    0
}
