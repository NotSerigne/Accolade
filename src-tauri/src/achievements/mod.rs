// src-tauri/src/achievements/mod.rs
use crate::achievements::models::{Achievement, Game};
use crate::emulators::codex;
use crate::emulators::empress;
use crate::emulators::goldberg;
use crate::emulators::onlinefix;
use crate::emulators::rune;
use crate::emulators::EmulatorParser;

pub mod appinfo;
pub mod models;
pub mod providers;
pub mod steam;

pub async fn get_achievements_for_game(game: Game, state: &crate::AppState) -> Vec<Achievement> {
    match game.source {
        models::SourceType::Emulator(emulator) => {
            let path = game.path_buf.unwrap_or_default();
            match emulator {
                models::Emulator::Goldberg => goldberg::Parser.parse(&path),
                models::Emulator::Empress => empress::Parser.parse(&path),
                models::Emulator::Codex => codex::Parser.parse(&path),
                models::Emulator::OnlineFix => onlinefix::Parser.parse(&path),
                models::Emulator::Rune => rune::Parser.parse(&path),
                models::Emulator::Steam => Vec::new(),
            }
        }
        models::SourceType::RetroAchievements => {
            let username = state.ra_username.lock().unwrap().clone();
            let api_key = state.ra_api_key.lock().unwrap().clone();
            let provider =
                providers::retroachievements::RetroAchievementsProvider::new(username, api_key);

            let ra_id = game
                .id
                .strip_prefix("ra_")
                .and_then(|id| id.parse::<u32>().ok());
            if let Some(id) = ra_id {
                provider.get_game_progress(id).await.unwrap_or_default()
            } else {
                Vec::new()
            }
        }
        models::SourceType::Custom => game.achievements,
    }
}

pub fn match_emulator(game: Game) -> Vec<Achievement> {
    if let models::SourceType::Emulator(emulator) = game.source {
        let path = game.path_buf.unwrap_or_default();
        match emulator {
            models::Emulator::Goldberg => goldberg::Parser.parse(&path),
            models::Emulator::Empress => empress::Parser.parse(&path),
            models::Emulator::Codex => codex::Parser.parse(&path),
            models::Emulator::OnlineFix => onlinefix::Parser.parse(&path),
            models::Emulator::Rune => rune::Parser.parse(&path),
            models::Emulator::Steam => Vec::new(),
        }
    } else {
        game.achievements
    }
}
