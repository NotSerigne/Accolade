// commands: les commandes qui seront appelées depuis le frontend

use crate::achievements::{match_emulator, models::Game, models::Achievement};
use crate::{apply_steamgriddb_icons, enrich_games_with_steam, AppState};

#[tauri::command]
pub fn get_achievements(game: Game) -> Vec<Achievement> {
    match_emulator(game)
}

#[tauri::command]
pub fn get_all_games(state: tauri::State<'_, AppState>) -> Vec<Game> {
    state
        .games
        .lock()
        .map(|games| games.clone())
        .unwrap_or_default()
}

#[tauri::command]
pub(crate) async fn sync_steam_metadata(api_key: String, state: tauri::State<'_, AppState>) -> Result<Vec<Game>, String> {
    {
        let mut key = state
            .steam_api_key
            .lock()
            .map_err(|_| String::from("Impossible d'acceder a la cle API"))?;
        *key = api_key.clone();
    }

    let mut cloned_games = state
        .games
        .lock()
        .map_err(|_| String::from("Impossible d'acceder a la liste des jeux"))?
        .clone();

    enrich_games_with_steam(&mut cloned_games, &api_key).await;

    dotenv::dotenv().ok();
    let sgdb_key = std::env::var("STEAMGRIDDB_API_KEY")
        .map_err(|_| String::from("Clé SteamGridDB introuvable"))?;

        apply_steamgriddb_icons(&mut cloned_games, &sgdb_key).await;

    let mut games = state
        .games
        .lock()
        .map_err(|_| String::from("Impossible d'acceder a la liste des jeux"))?;
    *games = cloned_games.clone();

    Ok(cloned_games)
}