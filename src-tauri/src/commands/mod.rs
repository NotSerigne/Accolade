// src-tauri/src/commands/mod.rs
use crate::achievements::models::{Achievement, Emulator, Game};
use crate::achievements::steam::{
    fetch_owned_games, fetch_recently_played_games, fetch_steam_user, get_local_steam_appids,
    OwnedGame, SteamUser,
};
use crate::user_data::{load_user_data, save_user_data, ManualAchievementOverride, UserData};
use crate::{apply_steamgriddb_icons, enrich_games_with_steam, AppState};
use std::collections::HashSet;
use tauri::Manager;

#[tauri::command]
pub fn test_achievement_notif(app_handle: tauri::AppHandle) {
    crate::watcher::emit_test_notification(&app_handle);
}

fn apply_manual_overrides_to_list(
    game_id: &str,
    achievements: &mut [Achievement],
    user_data: &UserData,
) {
    if let Some(overrides) = user_data.manual_achievements.get(game_id) {
        for ach in achievements.iter_mut() {
            let key_lower = ach.key.trim().to_lowercase();
            if let Some(ov) = overrides.get(&key_lower) {
                ach.unlocked = ov.unlocked;
                ach.unlocked_time = if ov.unlocked { ov.unlocked_time } else { None };
            }
        }
    }
}

#[tauri::command]
pub async fn get_achievements(
    game: Game,
    state: tauri::State<'_, AppState>,
    app_handle: tauri::AppHandle,
) -> Result<Vec<Achievement>, String> {
    let game_id = game.id.clone();
    let mut achs = crate::achievements::get_achievements_for_game(game, &state).await;
    let user_data = load_user_data(&app_handle);
    apply_manual_overrides_to_list(&game_id, &mut achs, &user_data);
    Ok(achs)
}

fn enrich_games_with_user_data(games: &mut [Game], user_data: &UserData) {
    for game in games.iter_mut() {
        game.is_favorite = user_data.favorites.contains(&game.id);
        if let Some(tags) = user_data.tags.get(&game.id) {
            game.tags = tags.clone();
        }
        if let Some(&secs) = user_data.playtime_seconds.get(&game.id) {
            game.playtime_seconds = game.playtime_seconds.max(secs);
        }
        apply_manual_overrides_to_list(&game.id, &mut game.achievements, user_data);
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
        game.tags = user_data.tags.get(&game.id).cloned().unwrap_or_default();
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
        game.tags = user_data.tags.get(&game.id).cloned().unwrap_or_default();
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

    log::debug!("[cmd] get_all_games returning {} games", games.len());
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
    let mut font_dir = std::path::PathBuf::from("static/fonts");

    if !font_dir.exists() {
        font_dir = std::path::PathBuf::from("../static/fonts");
    }

    if !font_dir.exists() {
        // Fallback for packaged app
        if let Ok(res_dir) = app_handle.path().resource_dir() {
            // Try different possible resource structures in the bundle
            let possible_paths = [
                res_dir.join("static").join("fonts"),
                res_dir.join("fonts"),
                res_dir.join("_up_").join("static").join("fonts"),
            ];

            for path in possible_paths {
                if path.exists() {
                    font_dir = path;
                    break;
                }
            }
        }
    }

    if !font_dir.exists() {
        let current_dir = std::env::current_dir().unwrap_or_default();
        let res_dir_path = app_handle
            .path()
            .resource_dir()
            .map(|p| p.to_string_lossy().to_string())
            .unwrap_or_else(|_| "Unknown".to_string());

        return Err(format!(
            "Dossier fonts introuvable (recherche dans static/fonts). \nExploration : \n- Dossier actuel : {:?}\n- Dossier ressources : {}\n\nVeuillez vous assurer que 'static/fonts' est inclus dans le bundle.",
            current_dir, res_dir_path
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
pub fn is_launched_minimized() -> bool {
    std::env::args().any(|a| a == "--minimized")
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

    log::info!(
        "[sync] Start. steam_id='{}', language='{}', api_key_len={}, sgdb_key_len={}",
        steam_id,
        language,
        effective_api_key.len(),
        sgdb_api_key.len()
    );

    if effective_api_key.is_empty() {
        log::warn!("[sync] No Steam API key found in parameters or env.");
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
    log::debug!("[sync] Initial state has {} games", cloned_games.len());

    let local_games = tauri::async_runtime::spawn_blocking(|| {
        crate::emulators::game_scanner(crate::emulators::default_parsers())
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
        log::info!("[sync] Added {} new local cracked games", added_local);
    }

    let mut added_steam_games = false;
    if !steam_id.trim().is_empty() && !effective_api_key.is_empty() {
        let steam_ids: Vec<&str> = steam_id
            .split(',')
            .map(|s| s.trim())
            .filter(|s| !s.is_empty())
            .collect();
        for s_id in steam_ids {
            log::info!("[sync] Fetching Steam library for {}", s_id);

            match fetch_steam_user(&effective_api_key, s_id).await {
                Ok(Some(profile)) if profile.communityvisibilitystate == Some(3) => {}
                Ok(Some(profile)) => {
                    log::info!(
                        "[sync] Skipping Steam library for {}: profile is not public (visibility state {:?})",
                        s_id,
                        profile.communityvisibilitystate
                    );
                    continue;
                }
                Ok(None) => {
                    log::warn!(
                        "[sync] Skipping Steam library for {}: profile was not found",
                        s_id
                    );
                    continue;
                }
                Err(err) => {
                    log::warn!(
                        "[sync] Skipping Steam library for {}: unable to verify profile visibility: {}",
                        s_id,
                        err
                    );
                    continue;
                }
            }

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
                        playtime_forever: None,
                    });
                }
            }

            log::info!(
                "[sync] Found {} potential games for {}",
                all_owned.len(),
                s_id
            );

            let mut added_count = 0;
            for og in all_owned {
                let game_id = format!("steam_{}", og.appid);
                let steam_playtime_secs = og.playtime_forever.unwrap_or(0).saturating_mul(60);

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
                        playtime_seconds: steam_playtime_secs,
                    });
                    existing_ids.insert(game_id);
                    added_count += 1;
                    added_steam_games = true;
                } else if steam_playtime_secs > 0 {
                    if let Some(existing) = cloned_games.iter_mut().find(|g| g.id == game_id) {
                        existing.playtime_seconds =
                            existing.playtime_seconds.max(steam_playtime_secs);
                    }
                }
            }
            log::info!("[sync] Added {} new Steam games for {}", added_count, s_id);
        }
    }

    if added_steam_games || added_local > 0 {
        if let Ok(mut games) = state.games.lock() {
            *games = cloned_games.clone();
        }
    }

    // RetroAchievements Sync
    if !ra_username.trim().is_empty() && !ra_api_key.trim().is_empty() {
        log::info!("[sync] Fetching RetroAchievements games");
        let ra_provider =
            crate::achievements::providers::retroachievements::RetroAchievementsProvider::new(
                ra_username.clone(),
                ra_api_key.clone(),
            );
        use crate::achievements::providers::AchievementProvider;
        match ra_provider.fetch_games().await {
            Ok(ra_games) => {
                log::info!("[sync] Found {} RetroAchievements games", ra_games.len());
                for rg in ra_games {
                    if !existing_ids.contains(&rg.id) {
                        cloned_games.push(rg.clone());
                        existing_ids.insert(rg.id);
                    }
                }
            }
            Err(e) => log::warn!("[sync] RetroAchievements sync failed: {}", e),
        }
    }

    if !effective_api_key.is_empty() {
        log::info!(
            "[sync] Enriching {} games with Steam metadata",
            cloned_games.len()
        );
        let main_steam_id = steam_id.split(',').next().unwrap_or("").trim();
        enrich_games_with_steam(
            &mut cloned_games,
            &effective_api_key,
            main_steam_id,
            &language,
            &app_handle,
        )
        .await;
    }

    let sgdb_key = if !sgdb_api_key.trim().is_empty() {
        sgdb_api_key
    } else {
        std::env::var("STEAMGRIDDB_API_KEY").unwrap_or_default()
    };
    if !sgdb_key.is_empty() {
        log::info!(
            "[sync] Enriching with SteamGridDB (key len: {})",
            sgdb_key.len()
        );
        apply_steamgriddb_icons(&mut cloned_games, &sgdb_key).await;
    }

    let mut filtered_games: Vec<Game> = cloned_games
        .into_iter()
        .filter(|g| !g.id.is_empty() && g.achievements_total > 0)
        .collect();

    let mut user_data = load_user_data(&app_handle);
    let mut user_data_dirty = false;
    for g in &filtered_games {
        if g.playtime_seconds > 0 {
            let entry = user_data.playtime_seconds.entry(g.id.clone()).or_insert(0);
            if g.playtime_seconds > *entry {
                *entry = g.playtime_seconds;
                user_data_dirty = true;
            }
        }
    }
    if user_data_dirty {
        let _ = save_user_data(&app_handle, &user_data);
    }

    enrich_games_with_user_data(&mut filtered_games, &user_data);

    {
        let mut games = state
            .games
            .lock()
            .map_err(|_| String::from("Impossible d'acceder a la liste des jeux"))?;
        *games = filtered_games.clone();
        log::info!("[sync] State updated with {} games", games.len());
    }

    log::info!("[sync] Done — returning {} games", filtered_games.len());
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
    match crate::screenshots::capture_screenshot(&app_handle, Some(&game_id), Some(&ach_key), true)
    {
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
    match crate::screenshots::capture_screenshot(&app_handle, None, None, true) {
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

#[tauri::command]
pub fn get_pinned_achievements(app_handle: tauri::AppHandle, game_id: String) -> Vec<String> {
    load_user_data(&app_handle)
        .pinned_achievements
        .remove(&game_id)
        .unwrap_or_default()
}

#[tauri::command]
pub fn toggle_pinned_achievement(
    app_handle: tauri::AppHandle,
    game_id: String,
    ach_key: String,
) -> Result<Vec<String>, String> {
    let mut user_data = load_user_data(&app_handle);
    let pins = user_data
        .pinned_achievements
        .entry(game_id.clone())
        .or_default();
    let wanted = ach_key.trim().to_lowercase();
    if let Some(pos) = pins.iter().position(|k| k.trim().to_lowercase() == wanted) {
        pins.remove(pos);
    } else {
        pins.push(ach_key);
    }
    let updated = pins.clone();
    save_user_data(&app_handle, &user_data)?;
    let _ = tauri::Emitter::emit(
        &app_handle,
        "pinned-achievements-updated",
        serde_json::json!({ "game_id": game_id, "pins": updated }),
    );
    Ok(updated)
}

#[derive(serde::Serialize, Clone)]
pub struct HudData {
    pub game_id: String,
    pub game_name: String,
    pub achievements: Vec<Achievement>,
    pub pinned_keys: Vec<String>,
}

#[tauri::command]
pub fn get_hud_data(
    app_handle: tauri::AppHandle,
    state: tauri::State<'_, AppState>,
    fallback_game_id: Option<String>,
) -> Option<HudData> {
    let running = state.running_game.lock().ok().and_then(|g| g.clone());
    let games = state.games.lock().ok()?;
    let user_data = load_user_data(&app_handle);

    let target = running
        .as_ref()
        .and_then(|r| {
            games.iter().find(|g| {
                r.game_id.as_ref().is_some_and(|id| &g.id == id) || g.steam_id == Some(r.app_id)
            })
        })
        .or_else(|| {
            fallback_game_id
                .as_ref()
                .and_then(|id| games.iter().find(|g| &g.id == id))
        })
        .or_else(|| {
            // Fall back to the first game that has at least one pinned achievement.
            games.iter().find(|g| {
                user_data
                    .pinned_achievements
                    .get(&g.id)
                    .is_some_and(|p| !p.is_empty())
            })
        })?;

    let pinned_keys = user_data
        .pinned_achievements
        .get(&target.id)
        .cloned()
        .unwrap_or_default();

    Some(HudData {
        game_id: target.id.clone(),
        game_name: if target.name.is_empty() {
            target.id.clone()
        } else {
            target.name.clone()
        },
        achievements: target.achievements.clone(),
        pinned_keys,
    })
}

#[tauri::command]
pub fn get_all_pinned_achievements(
    app_handle: tauri::AppHandle,
) -> std::collections::HashMap<String, Vec<String>> {
    load_user_data(&app_handle).pinned_achievements
}

#[tauri::command]
pub fn read_json_file(path: String) -> Result<String, String> {
    let trimmed = path.trim();
    if !trimmed.to_ascii_lowercase().ends_with(".json") {
        return Err("Only .json files are allowed".to_string());
    }
    std::fs::read_to_string(trimmed).map_err(|e| e.to_string())
}

#[derive(serde::Serialize, Clone)]
pub struct CustomSoundInfo {
    pub filename: String,
    pub path: String,
}

fn custom_sounds_dir(app_handle: &tauri::AppHandle) -> Result<std::path::PathBuf, String> {
    let dir = app_handle
        .path()
        .app_data_dir()
        .map_err(|e| e.to_string())?
        .join("sounds");
    if !dir.exists() {
        std::fs::create_dir_all(&dir).map_err(|e| e.to_string())?;
    }
    Ok(dir)
}

#[tauri::command]
pub fn get_custom_sounds(app_handle: tauri::AppHandle) -> Vec<CustomSoundInfo> {
    let Ok(dir) = custom_sounds_dir(&app_handle) else {
        return Vec::new();
    };
    let mut out = Vec::new();
    if let Ok(entries) = std::fs::read_dir(dir) {
        for entry in entries.flatten() {
            let p = entry.path();
            if !p.is_file() {
                continue;
            }
            let ext = p
                .extension()
                .and_then(|s| s.to_str())
                .map(|s| s.to_ascii_lowercase());
            if !matches!(
                ext.as_deref(),
                Some("mp3") | Some("wav") | Some("ogg") | Some("m4a")
            ) {
                continue;
            }
            let Some(fname) = p.file_name().and_then(|s| s.to_str()) else {
                continue;
            };
            let path_str = p.to_string_lossy().trim_start_matches(r"\\?\").to_string();
            out.push(CustomSoundInfo {
                filename: fname.to_string(),
                path: path_str,
            });
        }
    }
    out.sort_by(|a, b| a.filename.to_lowercase().cmp(&b.filename.to_lowercase()));
    out
}

#[tauri::command]
pub fn import_custom_sound(
    app_handle: tauri::AppHandle,
    source_path: String,
) -> Result<CustomSoundInfo, String> {
    let src = std::path::Path::new(source_path.trim());
    if !src.is_file() {
        return Err("Source audio file not found".to_string());
    }
    let ext = src
        .extension()
        .and_then(|s| s.to_str())
        .map(|s| s.to_ascii_lowercase());
    if !matches!(
        ext.as_deref(),
        Some("mp3") | Some("wav") | Some("ogg") | Some("m4a")
    ) {
        return Err("Unsupported audio format (use .mp3, .wav, .ogg, or .m4a)".to_string());
    }
    let filename = src
        .file_name()
        .and_then(|s| s.to_str())
        .ok_or_else(|| "Invalid file name".to_string())?
        .to_string();
    let dest = custom_sounds_dir(&app_handle)?.join(&filename);
    std::fs::copy(src, &dest).map_err(|e| e.to_string())?;
    let path_str = dest
        .to_string_lossy()
        .trim_start_matches(r"\\?\")
        .to_string();
    Ok(CustomSoundInfo {
        filename,
        path: path_str,
    })
}

#[tauri::command]
pub fn capture_replay_clip(app_handle: tauri::AppHandle) -> Result<Option<String>, String> {
    crate::replay::flush_replay_gif(&app_handle, None, None)
}

#[tauri::command]
pub fn create_backup_zip(
    app_handle: tauri::AppHandle,
    dest_path: String,
) -> Result<String, String> {
    use std::io::{Read, Write};
    let file = std::fs::File::create(&dest_path).map_err(|e| e.to_string())?;
    let mut zip = zip::ZipWriter::new(file);
    let options = zip::write::SimpleFileOptions::default()
        .compression_method(zip::CompressionMethod::Deflated);

    // 1. Add Accolade app_data_dir config/sounds/user_data
    if let Ok(app_dir) = app_handle.path().app_data_dir() {
        for fname in [
            "user_data.json",
            "settings.json",
            "steam_appdetails_cache.json",
            "hltb_cache.json",
        ] {
            let p = app_dir.join(fname);
            if p.is_file() {
                if let Ok(mut f) = std::fs::File::open(&p) {
                    let mut buf = Vec::new();
                    if f.read_to_end(&mut buf).is_ok() {
                        let _ = zip.start_file(format!("accolade/{}", fname), options);
                        let _ = zip.write_all(&buf);
                    }
                }
            }
        }
        let sounds_dir = app_dir.join("sounds");
        if sounds_dir.is_dir() {
            if let Ok(entries) = std::fs::read_dir(&sounds_dir) {
                for entry in entries.flatten() {
                    let p = entry.path();
                    if p.is_file() {
                        if let Some(name) = p.file_name().and_then(|n| n.to_str()) {
                            if let Ok(mut f) = std::fs::File::open(&p) {
                                let mut buf = Vec::new();
                                if f.read_to_end(&mut buf).is_ok() {
                                    let _ = zip
                                        .start_file(format!("accolade/sounds/{}", name), options);
                                    let _ = zip.write_all(&buf);
                                }
                            }
                        }
                    }
                }
            }
        }
    }

    // 2. Add local emulator achievement folders
    fn zip_dir_recursive<W: Write + std::io::Seek>(
        zip: &mut zip::ZipWriter<W>,
        base: &std::path::Path,
        current: &std::path::Path,
        prefix: &str,
        options: zip::write::SimpleFileOptions,
    ) {
        let Ok(entries) = std::fs::read_dir(current) else {
            return;
        };
        for entry in entries.flatten() {
            let p = entry.path();
            if p.is_dir() {
                zip_dir_recursive(zip, base, &p, prefix, options);
            } else if p.is_file() {
                if let Ok(rel) = p.strip_prefix(base) {
                    let rel_str = rel.to_string_lossy().replace('\\', "/");
                    if let Ok(mut f) = std::fs::File::open(&p) {
                        let mut buf = Vec::new();
                        if std::io::Read::read_to_end(&mut f, &mut buf).is_ok() {
                            let _ = zip.start_file(format!("{}/{}", prefix, rel_str), options);
                            let _ = zip.write_all(&buf);
                        }
                    }
                }
            }
        }
    }

    let parsers = crate::emulators::default_parsers();
    for (idx, parser) in parsers.iter().enumerate() {
        let emu_name = format!("{:?}", parser.emulator());
        for (loc_idx, loc) in parser.known_locations().into_iter().enumerate() {
            if loc.is_dir() {
                let prefix = format!("emulators/{}_{}_{}", emu_name, idx, loc_idx);
                // Store origin path marker so restore knows where to put it back
                let _ = zip.start_file(format!("{}/__origin_path.txt", prefix), options);
                let _ = zip.write_all(loc.to_string_lossy().as_bytes());
                zip_dir_recursive(&mut zip, &loc, &loc, &prefix, options);
            }
        }
    }

    zip.finish().map_err(|e| e.to_string())?;
    Ok(dest_path)
}

#[tauri::command]
pub fn restore_backup_zip(app_handle: tauri::AppHandle, zip_path: String) -> Result<u32, String> {
    use std::io::Read;
    let file = std::fs::File::open(&zip_path).map_err(|e| e.to_string())?;
    let mut archive = zip::ZipArchive::new(file).map_err(|e| e.to_string())?;
    let app_dir = app_handle
        .path()
        .app_data_dir()
        .map_err(|e| e.to_string())?;

    // First pass: collect __origin_path.txt mappings for emulator folders
    let mut origins: std::collections::HashMap<String, std::path::PathBuf> =
        std::collections::HashMap::new();
    for i in 0..archive.len() {
        let Ok(mut entry) = archive.by_index(i) else {
            continue;
        };
        let name = entry.name().replace('\\', "/");
        if let Some(prefix) = name.strip_suffix("/__origin_path.txt") {
            let mut s = String::new();
            if entry.read_to_string(&mut s).is_ok() && !s.trim().is_empty() {
                origins.insert(prefix.to_string(), std::path::PathBuf::from(s.trim()));
            }
        }
    }

    let mut restored_files = 0u32;
    for i in 0..archive.len() {
        let Ok(mut entry) = archive.by_index(i) else {
            continue;
        };
        if entry.is_dir() {
            continue;
        }
        let name = entry.name().replace('\\', "/");
        if name.ends_with("/__origin_path.txt") || name.contains("..") {
            continue;
        }

        let target_path = if let Some(rel) = name.strip_prefix("accolade/") {
            Some(app_dir.join(rel))
        } else if name.starts_with("emulators/") {
            let parts: Vec<&str> = name.splitn(3, '/').collect();
            if parts.len() == 3 {
                let prefix = format!("{}/{}", parts[0], parts[1]);
                origins.get(&prefix).map(|base| base.join(parts[2]))
            } else {
                None
            }
        } else {
            None
        };

        if let Some(dest) = target_path {
            if let Some(parent) = dest.parent() {
                let _ = std::fs::create_dir_all(parent);
            }
            let mut buf = Vec::new();
            if entry.read_to_end(&mut buf).is_ok() {
                if std::fs::write(&dest, &buf).is_ok() {
                    restored_files += 1;
                }
            }
        }
    }

    Ok(restored_files)
}

#[tauri::command]
pub fn set_manual_playtime(
    app_handle: tauri::AppHandle,
    state: tauri::State<'_, AppState>,
    game_id: String,
    playtime_seconds: u64,
) -> Result<u64, String> {
    let mut user_data = load_user_data(&app_handle);
    user_data
        .playtime_seconds
        .insert(game_id.clone(), playtime_seconds);
    save_user_data(&app_handle, &user_data)?;

    if let Ok(mut games) = state.games.lock() {
        if let Some(game) = games.iter_mut().find(|g| g.id == game_id) {
            game.playtime_seconds = playtime_seconds;
        }
    }

    let _ = tauri::Emitter::emit(
        &app_handle,
        "playtime-updated",
        serde_json::json!({
            "game_id": game_id,
            "playtime_seconds": playtime_seconds
        }),
    );

    Ok(playtime_seconds)
}

/// Writes an achievement's unlocked state and timestamp back to the local emulator file on disk
/// (`achievements.json`, `achievement.json`, `achievements.ini`, `user_stats.ini`, or `achievements.txt`).
pub(crate) fn write_achievement_to_emulator_file(
    path_buf: &str,
    ach_key: &str,
    unlocked: bool,
    unlocked_time: Option<u64>,
) -> Result<(), String> {
    let path = std::path::Path::new(path_buf.trim());
    if !path.is_file() {
        return Ok(());
    }

    let ts = if unlocked {
        unlocked_time.unwrap_or_else(|| {
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map(|d| d.as_secs())
                .unwrap_or(0)
        })
    } else {
        0
    };

    let ext = path
        .extension()
        .and_then(|s| s.to_str())
        .map(|s| s.to_ascii_lowercase())
        .unwrap_or_default();

    if ext == "json" {
        let content = std::fs::read_to_string(path).unwrap_or_else(|_| "{}".to_string());
        let mut root: serde_json::Value =
            serde_json::from_str(&content).unwrap_or_else(|_| serde_json::json!({}));

        if let Some(obj) = root.as_object_mut() {
            let target_key = obj
                .keys()
                .find(|k| k.trim().eq_ignore_ascii_case(ach_key.trim()))
                .cloned()
                .unwrap_or_else(|| ach_key.trim().to_string());

            let entry = obj
                .entry(target_key)
                .or_insert_with(|| serde_json::json!({}));
            if let Some(ach_obj) = entry.as_object_mut() {
                ach_obj.insert("earned".to_string(), serde_json::Value::Bool(unlocked));
                ach_obj.insert("earned_time".to_string(), serde_json::json!(ts));
            } else {
                *entry = serde_json::json!({
                    "earned": unlocked,
                    "earned_time": ts
                });
            }

            let pretty = serde_json::to_string_pretty(&root).map_err(|e| e.to_string())?;
            std::fs::write(path, pretty).map_err(|e| e.to_string())?;
        }
    } else if ext == "txt" {
        let content = std::fs::read_to_string(path).unwrap_or_default();
        let wanted = ach_key.trim().to_lowercase();
        let mut lines: Vec<String> = Vec::new();
        let mut found = false;

        for line in content.lines() {
            let trimmed = line.trim();
            if trimmed.is_empty() {
                continue;
            }
            let key_part = trimmed
                .split_once(['=', ':', ' ', '\t'])
                .map(|(k, _)| k.trim())
                .unwrap_or(trimmed);
            if key_part.to_lowercase() == wanted {
                found = true;
                if unlocked {
                    lines.push(format!("{}={}", ach_key.trim(), ts));
                }
            } else {
                lines.push(line.to_string());
            }
        }
        if unlocked && !found {
            lines.push(format!("{}={}", ach_key.trim(), ts));
        }
        std::fs::write(path, lines.join("\n")).map_err(|e| e.to_string())?;
    } else if ext == "ini" {
        let mut ini = ini::Ini::load_from_file(path).unwrap_or_default();
        let wanted = ach_key.trim();

        // 1. Check TENOKE [ACHIEVEMENTS] section
        let has_tenoke_sec = ini
            .iter()
            .any(|(sec, _)| sec.is_some_and(|s| s.trim().eq_ignore_ascii_case("ACHIEVEMENTS")));

        if has_tenoke_sec {
            let val = if unlocked {
                format!("{{unlocked = true, time = {}}}", ts)
            } else {
                "false".to_string()
            };
            ini.with_section(Some("ACHIEVEMENTS")).set(wanted, val);
        } else {
            // 2. Ensure indexed in [SteamAchievements] if present (CODEX / RUNE)
            if let Some(steam_achs) = ini.section(Some("SteamAchievements")) {
                let already_indexed = steam_achs
                    .iter()
                    .any(|(_, v)| v.trim().trim_matches('"').eq_ignore_ascii_case(wanted));
                if !already_indexed {
                    let count = steam_achs.len();
                    let idx_key = format!("{:05}", count);
                    ini.with_section(Some("SteamAchievements"))
                        .set(idx_key, wanted);
                }
            }

            // Find matching section name (preserving case)
            let sec_name = ini
                .iter()
                .find_map(|(sec, _)| {
                    sec.filter(|s| s.trim().trim_matches('"').eq_ignore_ascii_case(wanted))
                        .map(|s| s.to_string())
                })
                .unwrap_or_else(|| wanted.to_string());

            let uses_have_achieved = ini
                .section(Some(&sec_name))
                .is_some_and(|p| p.get("HaveAchieved").is_some());
            let uses_lower_achieved = ini
                .section(Some(&sec_name))
                .is_some_and(|p| p.get("achieved").is_some() && p.get("Achieved").is_none());

            let mut sec = ini.with_section(Some(sec_name));
            if uses_have_achieved {
                sec.set("HaveAchieved", if unlocked { "1" } else { "0" });
                sec.set("HaveAchievedTime", ts.to_string());
            } else if uses_lower_achieved {
                sec.set("achieved", if unlocked { "true" } else { "false" });
                sec.set("timestamp", ts.to_string());
                sec.set("UnlockTime", ts.to_string());
            } else {
                sec.set("Achieved", if unlocked { "1" } else { "0" });
                sec.set("UnlockTime", ts.to_string());
            }
        }

        ini.write_to_file(path).map_err(|e| e.to_string())?;
    }

    Ok(())
}

#[tauri::command]
pub fn set_achievement_state(
    app_handle: tauri::AppHandle,
    state: tauri::State<'_, AppState>,
    game_id: String,
    ach_key: String,
    unlocked: bool,
    unlocked_time: Option<u64>,
) -> Result<Vec<Achievement>, String> {
    let effective_time = if unlocked {
        Some(unlocked_time.unwrap_or_else(|| {
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map(|d| d.as_secs())
                .unwrap_or(0)
        }))
    } else {
        None
    };

    // 1. Persist override in user_data.json
    let mut user_data = load_user_data(&app_handle);
    let key_lower = ach_key.trim().to_lowercase();
    user_data
        .manual_achievements
        .entry(game_id.clone())
        .or_default()
        .insert(
            key_lower.clone(),
            ManualAchievementOverride {
                unlocked,
                unlocked_time: effective_time,
            },
        );
    save_user_data(&app_handle, &user_data)?;

    // 2. Update in-memory AppState.games and write back to emulator file if path_buf exists
    let mut updated_achievements = Vec::new();
    let mut emulator_path: Option<String> = None;

    if let Ok(mut games) = state.games.lock() {
        if let Some(game) = games.iter_mut().find(|g| g.id == game_id) {
            emulator_path = game.path_buf.clone();
            for ach in game.achievements.iter_mut() {
                if ach.key.trim().to_lowercase() == key_lower {
                    ach.unlocked = unlocked;
                    ach.unlocked_time = effective_time;
                }
            }
            updated_achievements = game.achievements.clone();
        }
    }

    if let Some(ref path_buf) = emulator_path {
        if let Err(e) =
            write_achievement_to_emulator_file(path_buf, &ach_key, unlocked, effective_time)
        {
            log::warn!(
                "[set_achievement_state] Could not write to emulator file '{}': {}",
                path_buf,
                e
            );
        }
    }

    let _ = tauri::Emitter::emit(&app_handle, "achievements-updated", ());
    Ok(updated_achievements)
}
