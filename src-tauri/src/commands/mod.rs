// src-tauri/src/commands/mod.rs
use crate::achievements::models::{Achievement, Emulator, Game};
use crate::achievements::steam::{fetch_owned_games, fetch_steam_user, OwnedGame, SteamUser};
use crate::{apply_steamgriddb_icons, enrich_games_with_steam, AppState};
use std::collections::HashSet;
use tauri::Manager;

#[tauri::command]
pub fn test_achievement_notif(app_handle: tauri::AppHandle) {
    crate::watcher::emit_test_notification(&app_handle);
}

#[tauri::command]
pub async fn get_achievements(
    game: Game,
    state: tauri::State<'_, AppState>,
) -> Result<Vec<Achievement>, String> {
    Ok(crate::achievements::get_achievements_for_game(game, &state).await)
}

use crate::user_data::{load_user_data, save_user_data, UserData};

fn enrich_games_with_user_data(games: &mut [Game], user_data: &UserData) {
    for game in games.iter_mut() {
        game.is_favorite = user_data.favorites.contains(&game.id);
        if let Some(tags) = user_data.tags.get(&game.id) {
            game.tags = tags.clone();
        }
    }
}

#[tauri::command]
pub fn toggle_game_favorite(
    game_id: String,
    state: tauri::State<'_, AppState>,
    app_handle: tauri::AppHandle,
) -> Result<bool, String> {
    let mut games = state
        .games
        .lock()
        .map_err(|_| "Impossible de verrouiller les jeux")?;
    let mut user_data = load_user_data(&app_handle);

    let mut is_favorite = false;
    if user_data.favorites.contains(&game_id) {
        user_data.favorites.remove(&game_id);
    } else {
        user_data.favorites.insert(game_id.clone());
        is_favorite = true;
    }

    if let Some(game) = games.iter_mut().find(|g| g.id == game_id) {
        game.is_favorite = is_favorite;
    }

    save_user_data(&app_handle, &user_data)?;
    Ok(is_favorite)
}

#[tauri::command]
pub fn add_game_tag(
    game_id: String,
    tag: String,
    state: tauri::State<'_, AppState>,
    app_handle: tauri::AppHandle,
) -> Result<(), String> {
    let mut games = state
        .games
        .lock()
        .map_err(|_| "Impossible de verrouiller les jeux")?;
    let mut user_data = load_user_data(&app_handle);

    let tags = user_data
        .tags
        .entry(game_id.clone())
        .or_insert_with(Vec::new);
    if !tags.contains(&tag) {
        tags.push(tag);
    }

    if let Some(game) = games.iter_mut().find(|g| g.id == game_id) {
        game.tags = user_data.tags.get(&game_id).cloned().unwrap_or_default();
    }

    save_user_data(&app_handle, &user_data)?;
    Ok(())
}

#[tauri::command]
pub fn remove_game_tag(
    game_id: String,
    tag: String,
    state: tauri::State<'_, AppState>,
    app_handle: tauri::AppHandle,
) -> Result<(), String> {
    let mut games = state
        .games
        .lock()
        .map_err(|_| "Impossible de verrouiller les jeux")?;
    let mut user_data = load_user_data(&app_handle);

    if let Some(tags) = user_data.tags.get_mut(&game_id) {
        tags.retain(|t| t != &tag);
    }

    if let Some(game) = games.iter_mut().find(|g| g.id == game_id) {
        game.tags = user_data.tags.get(&game_id).cloned().unwrap_or_default();
    }

    save_user_data(&app_handle, &user_data)?;
    Ok(())
}

#[tauri::command]
pub fn get_all_games(state: tauri::State<'_, AppState>, app_handle: tauri::AppHandle) -> Vec<Game> {
    let user_data = load_user_data(&app_handle);
    let mut games = state
        .games
        .lock()
        .map(|gs| {
            gs.iter()
                .filter(|g| !g.id.is_empty() && g.achievements_total > 0)
                .cloned()
                .collect::<Vec<Game>>()
        })
        .unwrap_or_default();

    enrich_games_with_user_data(&mut games, &user_data);

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
    language: String,
) -> Result<Vec<OwnedGame>, String> {
    fetch_owned_games(&api_key, &steam_id, &language)
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub fn exit_app(app_handle: tauri::AppHandle) {
    app_handle.exit(0);
}

#[tauri::command]
pub fn hide_app(app_handle: tauri::AppHandle) {
    if let Some(window) = app_handle.get_webview_window("main") {
        let _ = window.hide();
    }
}

#[tauri::command]
pub(crate) async fn sync_steam_metadata(
    api_key: String,
    steam_id: String,
    ra_username: String,
    ra_api_key: String,
    sgdb_api_key: String,
    language: String,
    state: tauri::State<'_, AppState>,
    app_handle: tauri::AppHandle,
) -> Result<Vec<Game>, String> {
    dotenv::dotenv().ok();
    let effective_api_key = if !api_key.trim().is_empty() {
        api_key
    } else {
        std::env::var("STEAM_API_KEY").unwrap_or_default()
    };

    println!(
        "[DEBUG][sync_steam_metadata] Start. steam_id='{}', language='{}', api_key_len={}, sgdb_key_len={}",
        steam_id, language, effective_api_key.len(), sgdb_api_key.len()
    );

    if effective_api_key.is_empty() {
        println!("[DEBUG][sync_steam_metadata] No API key found.");
    }

    {
        let mut key = state
            .steam_api_key
            .lock()
            .map_err(|_| String::from("Impossible d'acceder a la cle API"))?;
        *key = effective_api_key.clone();

        let mut ra_u = state
            .ra_username
            .lock()
            .map_err(|_| String::from("Impossible d'acceder au pseudo RA"))?;
        *ra_u = ra_username.clone();

        let mut ra_k = state
            .ra_api_key
            .lock()
            .map_err(|_| String::from("Impossible d'acceder a la cle API RA"))?;
        *ra_k = ra_api_key.clone();

        let mut lang = state
            .language
            .lock()
            .map_err(|_| String::from("Impossible d'acceder a la langue"))?;
        *lang = language.clone();
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

    let local_games = tauri::async_runtime::spawn_blocking(|| {
        let parsers: Vec<Box<dyn crate::emulators::EmulatorParser>> = vec![
            Box::new(crate::emulators::goldberg::Parser),
            Box::new(crate::emulators::empress::Parser),
            Box::new(crate::emulators::onlinefix::Parser),
            Box::new(crate::emulators::rune::Parser),
            Box::new(crate::emulators::codex::Parser),
        ];
        crate::emulators::game_scanner(parsers)
    })
    .await
    .unwrap_or_default();

    let mut existing_ids: HashSet<String> = cloned_games.iter().map(|g| g.id.clone()).collect();
    let mut added_local = 0;

    for lg in local_games {
        if !existing_ids.contains(&lg.id) {
            cloned_games.push(lg.clone());
            existing_ids.insert(lg.id.clone());
            added_local += 1;
        }
    }
    if added_local > 0 {
        println!(
            "[DEBUG][sync_steam_metadata] Added {} new local cracked games",
            added_local
        );
    }

    let mut added_steam_games = false;
    if !steam_id.trim().is_empty() && !effective_api_key.is_empty() {
        println!(
            "[DEBUG][sync_steam_metadata] Fetching owned games for {}",
            steam_id
        );
        match fetch_owned_games(&effective_api_key, &steam_id, &language).await {
            Ok(owned) => {
                println!(
                    "[DEBUG][sync_steam_metadata] Found {} owned games",
                    owned.len()
                );
                let mut added_count = 0;
                for og in owned {
                    let game_id = format!("steam_{}", og.appid);
                    if !existing_ids.contains(&game_id) {
                        cloned_games.push(Game {
                            name: og.name.clone().unwrap_or_else(|| format!("AppID {}", og.appid)),
                            id: game_id.clone(),
                            steam_id: Some(og.appid),
                            game_icon: og.img_icon_url.clone().unwrap_or_default(),
                            steamgrid_icon_url: String::new(),
                            header_image_url: format!(
                                "https://shared.akamai.steamstatic.com/store_item_assets/steam/apps/{}/header.jpg",
                                og.appid
                            ),
                            background_image_url: String::new(),
                            achievements_total: 0,
                            achievements: Vec::new(),
                            path_buf: None,
                            source: crate::achievements::models::SourceType::Emulator(Emulator::Steam),
                            is_favorite: false,
                            tags: Vec::new(),
                        });
                        existing_ids.insert(game_id);
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

    if added_steam_games || added_local > 0 {
        if let Ok(mut games) = state.games.lock() {
            *games = cloned_games.clone();
        }
    }

    // RetroAchievements Sync
    if !ra_username.trim().is_empty() && !ra_api_key.trim().is_empty() {
        println!("[DEBUG][sync_steam_metadata] Fetching RetroAchievements games");
        let ra_provider =
            crate::achievements::providers::retroachievements::RetroAchievementsProvider::new(
                ra_username.clone(),
                ra_api_key.clone(),
            );
        use crate::achievements::providers::AchievementProvider;
        match ra_provider.fetch_games().await {
            Ok(ra_games) => {
                println!(
                    "[DEBUG][sync_steam_metadata] Found {} RA games",
                    ra_games.len()
                );
                for rg in ra_games {
                    if !existing_ids.contains(&rg.id) {
                        cloned_games.push(rg.clone());
                        existing_ids.insert(rg.id);
                    }
                }
            }
            Err(e) => println!("[DEBUG][sync_steam_metadata] RA sync failed: {}", e),
        }
    }

    if !effective_api_key.is_empty() {
        println!(
            "[DEBUG][sync_steam_metadata] Enriching {} games with Steam metadata",
            cloned_games.len()
        );
        enrich_games_with_steam(&mut cloned_games, &effective_api_key, &steam_id, &language).await;
    }

    let sgdb_key = if !sgdb_api_key.trim().is_empty() {
        sgdb_api_key
    } else {
        std::env::var("STEAMGRIDDB_API_KEY").unwrap_or_default()
    };
    if !sgdb_key.is_empty() {
        println!("[DEBUG][sync_steam_metadata] Enriching with SteamGridDB");
        apply_steamgriddb_icons(&mut cloned_games, &sgdb_key).await;
    }

    let mut filtered_games: Vec<Game> = cloned_games
        .into_iter()
        .filter(|g| {
            let has_id = !g.id.is_empty();
            let has_achievements = g.achievements_total > 0;
            has_id && has_achievements
        })
        .collect();

    let user_data = load_user_data(&app_handle);
    enrich_games_with_user_data(&mut filtered_games, &user_data);

    {
        let mut games = state
            .games
            .lock()
            .map_err(|_| String::from("Impossible d'acceder a la liste des jeux"))?;
        *games = filtered_games.clone();
        println!(
            "[DEBUG][sync_steam_metadata] State updated with {} games",
            games.len()
        );
    }

    println!(
        "[DEBUG][sync_steam_metadata] Returning {} games",
        filtered_games.len()
    );
    Ok(filtered_games)
}
