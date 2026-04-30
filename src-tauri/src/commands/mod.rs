// commands: les commandes qui seront appelées depuis le frontend

use crate::achievements::match_emulator;
use crate::achievements::models::{Achievement, Emulator, Game};
use crate::achievements::steam::{fetch_owned_games, fetch_steam_user, OwnedGame, SteamUser};
use crate::{apply_steamgriddb_icons, enrich_games_with_steam, AppState};
use std::collections::HashSet;

#[tauri::command]
pub fn test_achievement_notif(app_handle: tauri::AppHandle) {
    crate::watcher::emit_test_notification(&app_handle);
}

#[tauri::command]
pub fn get_achievements(game: Game) -> Vec<Achievement> {
    match_emulator(game)
}

#[tauri::command]
pub fn get_all_games(state: tauri::State<'_, AppState>) -> Vec<Game> {
    let games = state
        .games
        .lock()
        .map(|gs| {
            gs.iter()
                .filter(|g| g.steam_id != 0)
                .cloned()
                .collect::<Vec<Game>>()
        })
        .unwrap_or_default();
    println!("[DEBUG][get_all_games] Returning {} games", games.len());
    games
}

#[tauri::command]
pub async fn get_steam_user(
    api_key: String,
    steam_id: String,
) -> Result<Option<SteamUser>, String> {
    fetch_steam_user(&api_key, &steam_id)
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn get_steam_owned_games(
    api_key: String,
    steam_id: String,
) -> Result<Vec<OwnedGame>, String> {
    fetch_owned_games(&api_key, &steam_id)
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub(crate) async fn sync_steam_metadata(
    api_key: String,
    steam_id: String,
    sgdb_api_key: String,
    state: tauri::State<'_, AppState>,
) -> Result<Vec<Game>, String> {
    println!(
        "[DEBUG][sync_steam_metadata] Start. steam_id='{}'",
        steam_id
    );
    dotenv::dotenv().ok();
    let effective_api_key = if !api_key.trim().is_empty() {
        api_key
    } else {
        std::env::var("STEAM_API_KEY").unwrap_or_default()
    };

    if effective_api_key.is_empty() {
        println!("[DEBUG][sync_steam_metadata] No API key found.");
    }

    {
        let mut key = state
            .steam_api_key
            .lock()
            .map_err(|_| String::from("Impossible d'acceder a la cle API"))?;
        *key = effective_api_key.clone();
    }

    let mut cloned_games: Vec<Game> = {
        let gs = state
            .games
            .lock()
            .map_err(|_| String::from("Impossible d'acceder a la liste des jeux"))?;
        gs.clone()
    };
    println!(
        "[DEBUG][sync_steam_metadata] Initial state has {} games",
        cloned_games.len()
    );

    // 1. Ajouter les jeux possédés si steam_id est fourni
    let mut added_steam_games = false;
    if !steam_id.trim().is_empty() && !effective_api_key.is_empty() {
        println!(
            "[DEBUG][sync_steam_metadata] Fetching owned games for {}",
            steam_id
        );
        match fetch_owned_games(&effective_api_key, &steam_id).await {
            Ok(owned) => {
                println!(
                    "[DEBUG][sync_steam_metadata] Found {} owned games",
                    owned.len()
                );
                let existing_ids: HashSet<u32> = cloned_games.iter().map(|g| g.steam_id).collect();
                let mut added_count = 0;
                for og in owned {
                    if !existing_ids.contains(&og.appid) {
                        cloned_games.push(Game {
                            name: og.name.clone().unwrap_or_else(|| format!("AppID {}", og.appid)),
                            steam_id: og.appid,
                            game_icon: og.img_icon_url.clone().unwrap_or_default(),
                            steamgrid_icon_url: String::new(),
                            header_image_url: format!(
                                "https://shared.akamai.steamstatic.com/store_item_assets/steam/apps/{}/header.jpg",
                                og.appid
                            ),
                            background_image_url: String::new(),
                            achievements_total: 0,
                            achievements: Vec::new(),
                            path_buf: String::new(),
                            emulator: Emulator::Steam,
                        });
                        added_count += 1;
                        added_steam_games = true;
                    }
                }
                println!(
                    "[DEBUG][sync_steam_metadata] Added {} new Steam games",
                    added_count
                );
            }
            Err(e) => {
                println!(
                    "[DEBUG][sync_steam_metadata] Failed to fetch owned games: {}",
                    e
                );
            }
        }
    }

    // Si on a ajouté des jeux, on met à jour l'état global immédiatement pour qu'ils apparaissent au moins en "squelette"
    if added_steam_games {
        if let Ok(mut games) = state.games.lock() {
            *games = cloned_games.clone();
        }
    }

    // 2. Enrichir avec les métadonnées Steam (achievements, images, progress)
    if !effective_api_key.is_empty() {
        println!(
            "[DEBUG][sync_steam_metadata] Enriching {} games with Steam metadata",
            cloned_games.len()
        );
        enrich_games_with_steam(&mut cloned_games, &effective_api_key, &steam_id).await;
    }

    // 3. Enrichir avec SteamGridDB si possible
    let sgdb_key = if !sgdb_api_key.trim().is_empty() {
        sgdb_api_key
    } else {
        std::env::var("STEAMGRIDDB_API_KEY").unwrap_or_default()
    };
    if !sgdb_key.is_empty() {
        println!("[DEBUG][sync_steam_metadata] Enriching with SteamGridDB");
        apply_steamgriddb_icons(&mut cloned_games, &sgdb_key).await;
    }

    // 4. Mettre à jour l'état global final
    {
        let mut games = state
            .games
            .lock()
            .map_err(|_| String::from("Impossible d'acceder a la liste des jeux"))?;
        *games = cloned_games.clone();
        println!(
            "[DEBUG][sync_steam_metadata] State updated with {} games",
            games.len()
        );
    }

    let result: Vec<Game> = cloned_games
        .into_iter()
        .filter(|g| g.steam_id != 0)
        .collect();
    println!(
        "[DEBUG][sync_steam_metadata] Returning {} games",
        result.len()
    );
    Ok(result)
}
