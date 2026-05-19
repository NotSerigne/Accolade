// src-tauri/src/commands/mod.rs
use crate::achievements::models::{Achievement, Emulator, Game};
use crate::achievements::steam::{
    fetch_owned_games, fetch_recently_played_games, fetch_steam_user, get_local_steam_appids,
    OwnedGame, SteamUser,
};
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
pub async fn extract_game_theme_color(image_url: String) -> Result<String, String> {
    crate::color::extract_dominant_color(&image_url)
        .await
        .ok_or_else(|| "Failed to extract color".to_string())
}

#[tauri::command]
pub fn auto_group_games(
    state: tauri::State<'_, AppState>,
    app_handle: tauri::AppHandle,
) -> Result<(), String> {
    let mut games_lock = state
        .games
        .lock()
        .map_err(|_| "Impossible de verrouiller les jeux")?;
    let mut user_data = load_user_data(&app_handle);

    let mut series_map: std::collections::HashMap<String, Vec<String>> =
        std::collections::HashMap::new();

    // 1. Group by Series (Name similarity)
    for game in games_lock.iter() {
        let name = game.name.to_lowercase();
        // Extract base name by removing suffixes and numbers
        let base_name = if let Some(idx) = name.find(':') {
            name[..idx].trim().to_string()
        } else if let Some(idx) = name.find(" - ") {
            name[..idx].trim().to_string()
        } else {
            let mut n = name.clone();
            // Remove common suffixes
            for suffix in &[
                " remastered",
                " goty",
                " deluxe",
                " edition",
                " anthology",
                " bundle",
                " collection",
            ] {
                if n.ends_with(suffix) {
                    n = n[..n.len() - suffix.len()].to_string();
                }
            }
            // Remove trailing numbers and whitespace
            while n
                .chars()
                .last()
                .map(|c| c.is_numeric() || c.is_whitespace())
                .unwrap_or(false)
            {
                n.pop();
            }
            n.trim().to_string()
        };

        if base_name.len() > 3 {
            series_map
                .entry(base_name)
                .or_default()
                .push(game.id.clone());
        }
    }

    for (series, ids) in series_map {
        if ids.len() > 1 {
            // Title case the series name for the tag
            let tag = series
                .split_whitespace()
                .map(|w| {
                    let mut c = w.chars();
                    match c.next() {
                        None => String::new(),
                        Some(f) => f.to_uppercase().collect::<String>() + c.as_str(),
                    }
                })
                .collect::<Vec<_>>()
                .join(" ");

            for id in ids {
                let tags = user_data.tags.entry(id).or_insert_with(Vec::new);
                if !tags.contains(&tag) {
                    tags.push(tag.clone());
                }
            }
        }
    }

    // 2. Group by Genre
    for game in games_lock.iter() {
        for genre in &game.genres {
            let tags = user_data
                .tags
                .entry(game.id.clone())
                .or_insert_with(Vec::new);
            if !tags.contains(genre) {
                tags.push(genre.clone());
            }
        }
    }

    // Update current games in memory
    for game in games_lock.iter_mut() {
        if let Some(tags) = user_data.tags.get(&game.id) {
            game.tags = tags.clone();
        }
    }

    save_user_data(&app_handle, &user_data)?;
    Ok(())
}

#[tauri::command]
pub async fn export_to_json(
    app_handle: tauri::AppHandle,
    data: String,
    filename: String,
) -> Result<String, String> {
    let app_dir = app_handle
        .path()
        .app_data_dir()
        .map_err(|e| e.to_string())?;
    let profiles_dir = app_dir.join("profiles");

    if !profiles_dir.exists() {
        std::fs::create_dir_all(&profiles_dir).map_err(|e| e.to_string())?;
    }

    let file_path = profiles_dir.join(filename);
    std::fs::write(&file_path, data).map_err(|e| e.to_string())?;

    Ok(file_path.to_string_lossy().to_string())
}

#[tauri::command]
pub fn get_profiles_dir(app_handle: tauri::AppHandle) -> Result<String, String> {
    let app_dir = app_handle
        .path()
        .app_data_dir()
        .map_err(|e| e.to_string())?;
    let profiles_dir = app_dir.join("profiles");

    if !profiles_dir.exists() {
        std::fs::create_dir_all(&profiles_dir).map_err(|e| e.to_string())?;
    }

    Ok(profiles_dir.to_string_lossy().to_string())
}

#[tauri::command]
pub fn open_profiles_dir(app_handle: tauri::AppHandle) -> Result<(), String> {
    let app_dir = app_handle
        .path()
        .app_data_dir()
        .map_err(|e| e.to_string())?;
    let profiles_dir = app_dir.join("profiles");

    if !profiles_dir.exists() {
        std::fs::create_dir_all(&profiles_dir).map_err(|e| e.to_string())?;
    }

    tauri_plugin_opener::OpenerExt::opener(&app_handle)
        .open_path(profiles_dir.to_string_lossy().to_string(), None::<String>)
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn export_to_pdf(
    app_handle: tauri::AppHandle,
    title: String,
    content: String,
    filename: String,
) -> Result<String, String> {
    let app_dir = app_handle
        .path()
        .app_data_dir()
        .map_err(|e| e.to_string())?;
    let profiles_dir = app_dir.join("profiles");

    if !profiles_dir.exists() {
        std::fs::create_dir_all(&profiles_dir).map_err(|e| e.to_string())?;
    }

    let file_path = profiles_dir.join(filename);

    // Load fonts - try multiple locations
    let mut font_dir = std::path::PathBuf::from("assets/fonts");

    if !font_dir.exists() {
        font_dir = std::path::PathBuf::from("../assets/fonts");
    }

    if !font_dir.exists() {
        font_dir = std::path::PathBuf::from("src/lib/assets/fonts");
    }

    if !font_dir.exists() {
        font_dir = std::path::PathBuf::from("../src/lib/assets/fonts");
    }

    if !font_dir.exists() {
        // Fallback for packaged app
        if let Ok(res_dir) = app_handle.path().resource_dir() {
            font_dir = res_dir.join("assets").join("fonts");
        }
    }

    if !font_dir.exists() {
        let current_dir = std::env::current_dir().unwrap_or_default();
        return Err(format!(
            "Dossier fonts introuvable. Dossier actuel: {:?}. Veuillez vous assurer que 'assets/fonts' existe à la racine du projet.",
            current_dir
        ));
    }

    let font_family = genpdf::fonts::from_files(&font_dir, "Roboto", None).map_err(|e| {
        let abs_path = std::fs::canonicalize(&font_dir).unwrap_or(font_dir.clone());
        format!(
            "Erreur lors du chargement des polices dans {:?}: {}",
            abs_path, e
        )
    })?;

    let mut doc = genpdf::Document::new(font_family);
    doc.set_title(title.clone());

    let mut decorator = genpdf::SimplePageDecorator::new();
    decorator.set_margins(10);
    doc.set_page_decorator(decorator);

    use genpdf::Element as _;
    let title_style = genpdf::style::Style::new().with_font_size(18).bold();
    doc.push(genpdf::elements::Text::new(title).styled(title_style));

    doc.push(genpdf::elements::Break::new(1.5));

    for line in content.lines() {
        doc.push(genpdf::elements::Text::new(line));
    }
    doc.render_to_file(&file_path).map_err(|e| e.to_string())?;

    Ok(file_path.to_string_lossy().to_string())
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
    let mut effective_api_key = api_key.trim().to_string();
    if effective_api_key.is_empty() {
        effective_api_key = std::env::var("STEAM_API_KEY").unwrap_or_default();
    }

    println!(
        "[DEBUG][sync_steam_metadata] Start. steam_id='{}', language='{}', api_key_len={}, sgdb_key_len={}",
        steam_id, language, effective_api_key.len(), sgdb_api_key.len()
    );

    if effective_api_key.is_empty() {
        println!(
            "[DEBUG][sync_steam_metadata] CRITICAL: No Steam API key found in parameters or env."
        );
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
        let steam_ids: Vec<&str> = steam_id
            .split(',')
            .map(|s| s.trim())
            .filter(|s| !s.is_empty())
            .collect();
        for s_id in steam_ids {
            println!("[DEBUG][sync_steam_metadata] Fetching games for {}", s_id);

            let mut all_owned = Vec::new();

            // 1. Owned games
            if let Ok(owned) = fetch_owned_games(&effective_api_key, s_id, &language).await {
                all_owned.extend(owned);
            }

            // 2. Recently played (covers some shared/free games)
            if let Ok(recent) = fetch_recently_played_games(&effective_api_key, s_id).await {
                for rg in recent {
                    if !all_owned.iter().any(|og| og.appid == rg.appid) {
                        all_owned.push(rg);
                    }
                }
            }

            // 3. Local userdata scan (covers all family shared games played on this machine)
            let local_appids = get_local_steam_appids(s_id);
            for appid in local_appids {
                if !all_owned.iter().any(|og| og.appid == appid) {
                    all_owned.push(OwnedGame {
                        appid,
                        name: None,
                        img_icon_url: None,
                    });
                }
            }

            println!(
                "[DEBUG][sync_steam_metadata] Found {} potential games for {}",
                all_owned.len(),
                s_id
            );

            let mut added_count = 0;
            for og in all_owned {
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
                        genres: Vec::new(),
                        is_favorite: false,
                        tags: Vec::new(),
                        });
                    existing_ids.insert(game_id);
                    added_count += 1;
                    added_steam_games = true;
                }
            }
            println!(
                "[DEBUG][sync_steam_metadata] Added {} new Steam games for {}",
                added_count, s_id
            );
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
        let main_steam_id = steam_id.split(',').next().unwrap_or("").trim();
        enrich_games_with_steam(
            &mut cloned_games,
            &effective_api_key,
            main_steam_id,
            &language,
        )
        .await;
    }

    let sgdb_key = if !sgdb_api_key.trim().is_empty() {
        sgdb_api_key
    } else {
        std::env::var("STEAMGRIDDB_API_KEY").unwrap_or_default()
    };
    if !sgdb_key.is_empty() {
        println!(
            "[DEBUG][sync_steam_metadata] Enriching with SteamGridDB, key len: {}",
            sgdb_key.len()
        );
        apply_steamgriddb_icons(&mut cloned_games, &sgdb_key).await;
    }

    let mut filtered_games: Vec<Game> = cloned_games
        .into_iter()
        .filter(|g| !g.id.is_empty() && g.achievements_total > 0)
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

#[tauri::command]
pub fn capture_automatic_screenshot(
    app_handle: tauri::AppHandle,
    game_id: String,
    ach_key: String,
) -> Result<String, String> {
    log::info!(
        "[Command] Automatic screenshot requested from overlay for game: {}, ach: {}",
        game_id,
        ach_key
    );
    match crate::screenshots::capture_screenshot(&app_handle, Some(&game_id), Some(&ach_key)) {
        Ok(filename) => {
            log::info!("[Command] Automatic screenshot captured: {}", filename);
            let _ = tauri::Emitter::emit(&app_handle, "screenshot-taken", filename.clone());
            Ok(filename)
        }
        Err(e) => {
            log::error!("[Command] Automatic screenshot failed: {}", e);
            Err(e)
        }
    }
}

#[tauri::command]
pub fn capture_manual_screenshot(app_handle: tauri::AppHandle) -> Result<String, String> {
    log::info!("[Command] Manual screenshot requested via button");
    match crate::screenshots::capture_screenshot(&app_handle, None, None) {
        Ok(filename) => {
            log::info!("[Command] Manual screenshot captured: {}", filename);
            let _ = tauri::Emitter::emit(&app_handle, "screenshot-taken", filename.clone());
            Ok(filename)
        }
        Err(e) => {
            log::error!("[Command] Manual screenshot failed: {}", e);
            Err(e)
        }
    }
}

#[tauri::command]
pub fn get_screenshots(app_handle: tauri::AppHandle) -> Vec<crate::screenshots::ScreenshotInfo> {
    crate::screenshots::list_screenshots(&app_handle)
}

#[tauri::command]
pub fn delete_screenshot(app_handle: tauri::AppHandle, filename: String) -> Result<(), String> {
    let mut path = crate::screenshots::get_screenshots_dir(&app_handle);
    path.push(filename);
    if path.exists() {
        std::fs::remove_file(path).map_err(|e| e.to_string())?;
    }
    Ok(())
}

#[tauri::command]
pub fn open_screenshots_dir(app_handle: tauri::AppHandle) -> Result<(), String> {
    let path = crate::screenshots::get_screenshots_dir(&app_handle);
    tauri_plugin_opener::OpenerExt::opener(&app_handle)
        .open_path(path.to_string_lossy().to_string(), None::<String>)
        .map_err(|e| e.to_string())
}
