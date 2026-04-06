// commands: les commandes qui seront appelées depuis le frontend

use crate::achievements::{match_emulator, models::Game, models::Achievement};
#[tauri::command]
pub fn get_achievements(game: Game) -> Vec<Achievement> {
    match_emulator(game)
}