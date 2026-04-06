// commands: les commandes qui seront appelées depuis le frontend

use crate::achievements::{match_emulator, models::Game, models::Achievement};
use crate::AppState;

#[tauri::command]
pub fn get_achievements(game: Game) -> Vec<Achievement> {
    match_emulator(game)
}

#[tauri::command]
pub fn get_all_games(state: tauri::State<'_, AppState>) -> Vec<Game> {
    state.games.clone()
}