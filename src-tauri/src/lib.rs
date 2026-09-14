// src-tauri/src/lib.rs
use crate::achievements::models::{Achievement, Game, SourceType};
use crate::achievements::steam::{fetch_player_achievements, fetch_steam_metadata_with_client};
use futures::stream::{self, StreamExt};
use std::collections::{HashMap, HashSet};
use std::sync::Mutex;
use tauri::{window::Color, Emitter, Manager};
use tauri_plugin_autostart::MacosLauncher;

pub struct AppState {
    pub(crate) games: Mutex<Vec<Game>>,
    pub(crate) steam_api_key: Mutex<String>,
    pub(crate) ra_username: Mutex<String>,
    pub(crate) ra_api_key: Mutex<String>,
    pub(crate) language: Mutex<String>,
}

fn merge_schema_with_local(schema: Vec<Achievement>, local: &[Achievement]) -> Vec<Achievement> {
    let local_map: HashMap<String, &Achievement> = local
        .iter()
        .map(|a| (a.key.trim().to_lowercase(), a))
        .collect();
    let mut seen_schema_keys: HashSet<String> = HashSet::new();

    let mut merged: Vec<Achievement> = schema
        .into_iter()
        .map(|s| {
            let key = s.key.trim().to_lowercase();
            seen_schema_keys.insert(key.clone());

            if let Some(live) = local_map.get(&key) {
                Achievement {
                    key: s.key,
                    name: if s.name.is_empty() {
                        live.name.clone()
                    } else {
                        s.name
                    },
                    unlocked: live.unlocked,
                    icon: if s.icon.is_empty() {
                        live.icon.clone()
                    } else {
                        s.icon
                    },
                    icon_gray: if s.icon_gray.is_empty() {
                        live.icon_gray.clone()
                    } else {
                        s.icon_gray
                    },
                    unlocked_time: live.unlocked_time,
                    rarity: live.rarity.clone(),
                    completionpercentage: if s.completionpercentage.is_empty() {
                        live.completionpercentage.clone()
                    } else {
                        s.completionpercentage.clone()
                    },
                    desc: if s.desc.is_empty() {
                        live.desc.clone()
                    } else {
                        s.desc
                    },
                    hidden: s.hidden,
                }
            } else {
                s
            }
        })
        .collect();

    for ach in local {
        let key = ach.key.trim().to_lowercase();
        if !seen_schema_keys.contains(&key) {
            merged.push(ach.clone());
        }
    }

    merged
}

pub(crate) async fn enrich_games_with_steam(
    games: &mut Vec<Game>,
    api_key: &str,
    steam_id: &str,
    language: &str,
) {
    if api_key.trim().is_empty() || games.is_empty() {
        return;
    }

    const STEAM_METADATA_CONCURRENCY: usize = 6;

    let client = reqwest::Client::new();
    let api_key_str = api_key.to_string();
    let steam_id_str = steam_id.to_string();
    let language_str = language.to_string();

    let jobs: Vec<(u32, SourceType, Vec<Achievement>)> = games
        .iter()
        .filter_map(|game| {
            game.steam_id
                .map(|sid| (sid, game.source.clone(), game.achievements.clone()))
        })
        .collect();

    let updates = stream::iter(jobs.into_iter().map(|(steam_id, source, local_state)| {
        let client = client.clone();
        let api_key = api_key_str.clone();
        let steam_id_inner = steam_id_str.clone();
        let language = language_str.clone();
        async move {
            let metadata = fetch_steam_metadata_with_client(&client, steam_id, &api_key, &language)
                .await
                .ok();

            let player_achievements_res = if !steam_id_inner.is_empty() {
                Some(
                    fetch_player_achievements(&api_key, &steam_id_inner, steam_id, &language).await,
                )
            } else {
                None
            };

            (
                steam_id,
                source,
                local_state,
                metadata,
                player_achievements_res,
            )
        }
    }))
    .buffer_unordered(STEAM_METADATA_CONCURRENCY)
    .collect::<Vec<_>>()
    .await;

    let results = updates;

    for game in games.iter_mut() {
        let Some(steam_id) = game.steam_id else {
            continue;
        };
        let source = game.source.clone();

        let Some(res) = results.iter().find(|r| r.0 == steam_id) else {
            continue;
        };

        let (_, _, local_state, metadata, player_achievements_res) = res;

        if let Some(Err(err)) = player_achievements_res {
            if matches!(
                source,
                SourceType::Emulator(crate::achievements::models::Emulator::Steam)
            ) {
                log::warn!(
                    "[enrich] Steam API error for AppID {}: {}. Achievements may be missing.",
                    steam_id,
                    err
                );
            }
        }

        let mut merged_achievements = if let Some(meta) = metadata {
            merge_schema_with_local(meta.achievements.clone(), local_state)
        } else {
            local_state.clone()
        };

        if let Some(Ok(pa)) = player_achievements_res {
            for (key, info) in pa {
                if let Some(ach) = merged_achievements.iter_mut().find(|a| &a.key == key) {
                    if info.unlocked {
                        ach.unlocked = true;
                        ach.unlocked_time = Some(info.unlock_time);
                    }
                    if !info.name.is_empty() {
                        ach.name = info.name.clone();
                    }
                    if !info.description.is_empty() {
                        ach.desc = info.description.clone();
                    }
                } else {
                    // Achievement exists in player stats but not in metadata
                    merged_achievements.push(Achievement {
                        key: key.clone(),
                        name: if info.name.is_empty() {
                            key.clone()
                        } else {
                            info.name.clone()
                        },
                        unlocked: info.unlocked,
                        unlocked_time: Some(info.unlock_time),
                        desc: info.description.clone(),
                        ..Default::default()
                    });
                }
            }
        }

        game.achievements = merged_achievements;
        game.achievements_total = game.achievements.len() as u32;

        if let Some(meta) = metadata {
            game.genres = meta.genres.clone();
            if !meta.name.is_empty() {
                game.name = meta.name.clone();
            }
            if !meta.header_image_url.is_empty() {
                game.header_image_url = meta.header_image_url.clone();
            } else {
                game.header_image_url = format!(
                    "https://cdn.cloudflare.steamstatic.com/steam/apps/{}/header.jpg",
                    steam_id
                );
            }
            if !meta.game_icon_url.is_empty() {
                game.game_icon = meta.game_icon_url.clone();
            }
            if !meta.background_image_url.is_empty() {
                game.background_image_url = meta.background_image_url.clone();
            } else {
                game.background_image_url = format!(
                    "https://cdn.cloudflare.steamstatic.com/steam/apps/{}/library_hero.jpg",
                    steam_id
                );
            }
        } else if game.header_image_url.is_empty() {
            game.header_image_url = format!(
                "https://cdn.cloudflare.steamstatic.com/steam/apps/{}/header.jpg",
                steam_id
            );
        }

        log::info!(
            "[enrich] {} (AppID: {}) — {} achievements",
            game.name,
            steam_id,
            game.achievements_total
        );
    }
}

async fn fetch_sgdb_icon(
    client: &reqwest::Client,
    steam_id: u32,
    api_key: &str,
    styles: &str,
) -> Option<String> {
    let url = format!(
        "https://www.steamgriddb.com/api/v2/icons/steam/{}?styles={}&dimensions=256",
        steam_id, styles
    );

    match client
        .get(&url)
        .header("Authorization", format!("Bearer {}", api_key))
        .send()
        .await
    {
        Ok(resp) => match resp.json::<serde_json::Value>().await {
            Ok(json) => {
                if let Some(data) = json["data"].as_array() {
                    if !data.is_empty() {
                        if let Some(icon_url) = data[0]["thumb"].as_str() {
                            return Some(icon_url.to_string());
                        }
                    }
                }
                None
            }
            Err(_) => None,
        },
        Err(_) => None,
    }
}

pub async fn apply_steamgriddb_icons(games: &mut [Game], api_key: &str) {
    if api_key.trim().is_empty() || games.is_empty() {
        return;
    }

    const SGDB_CONCURRENCY: usize = 8;

    let client = reqwest::Client::new();
    let api_key = api_key.to_string();

    let steam_games: Vec<(u32, usize)> = games
        .iter()
        .enumerate()
        .filter_map(|(idx, game)| game.steam_id.map(|sid| (sid, idx)))
        .collect();

    let updates = stream::iter(steam_games.into_iter().map(|(steam_id, index)| {
        let client = client.clone();
        let api_key = api_key.clone();
        async move {
            let official = fetch_sgdb_icon(&client, steam_id, &api_key, "official").await;
            let icon_url = if official.is_some() {
                official
            } else {
                fetch_sgdb_icon(&client, steam_id, &api_key, "").await
            };
            (index, icon_url)
        }
    }))
    .buffer_unordered(SGDB_CONCURRENCY)
    .collect::<Vec<_>>()
    .await;

    for (index, icon_url) in updates {
        if let Some(url) = icon_url {
            games[index].steamgrid_icon_url = url;
        }
    }
}

#[tauri::command]
fn update_screenshot_shortcut(app: tauri::AppHandle, shortcut_str: String) -> Result<(), String> {
    use tauri_plugin_global_shortcut::{GlobalShortcutExt, Shortcut};

    log::info!("[Shortcut] Manual registration request: {}", shortcut_str);

    let shortcut: Shortcut = shortcut_str.parse().map_err(|e| {
        let err = format!("Invalid shortcut format '{}': {}", shortcut_str, e);
        log::error!("{}", err);
        err
    })?;

    // Unregister all first to be clean
    let _ = app.global_shortcut().unregister_all();

    // Register new shortcut
    log::info!("[Shortcut] Registering: {:?}", shortcut);
    app.global_shortcut().register(shortcut).map_err(|e| {
        let err = format!("Failed to register shortcut: {}", e);
        log::error!("{}", err);
        err
    })?;

    log::info!("[Shortcut] Successfully updated to {}", shortcut_str);
    Ok(())
}

#[tauri::command]
async fn enable_autostart(_app: tauri::AppHandle) -> Result<(), String> {
    #[cfg(windows)]
    {
        // Refuse to register a debug binary in the registry — it would open a CMD
        // window and try to load localhost:5173 on every system boot.
        if cfg!(debug_assertions) {
            return Err("Cannot enable autostart from a debug build. Use a release build (cargo tauri build).".to_string());
        }

        use winreg::enums::*;
        use winreg::RegKey;
        let path = std::env::current_exe().map_err(|e| e.to_string())?;
        let path = dunce::canonicalize(path).map_err(|e| e.to_string())?;
        let path_str = path.to_string_lossy().to_string();
        let cmd = format!("\"{}\" --minimized", path_str);
        let hkcu = RegKey::predef(HKEY_CURRENT_USER);
        let key = hkcu
            .open_subkey_with_flags(
                "Software\\Microsoft\\Windows\\CurrentVersion\\Run",
                KEY_WRITE,
            )
            .map_err(|e| e.to_string())?;
        key.set_value("Accolade", &cmd).map_err(|e| e.to_string())?;
        Ok(())
    }
    #[cfg(not(windows))]
    {
        use tauri_plugin_autostart::ManagerExt;
        _app.autolaunch().enable().map_err(|e| format!("{}", e))
    }
}

#[tauri::command]
async fn disable_autostart(_app: tauri::AppHandle) -> Result<(), String> {
    #[cfg(windows)]
    {
        use winreg::enums::*;
        use winreg::RegKey;
        let hkcu = RegKey::predef(HKEY_CURRENT_USER);
        let key = hkcu
            .open_subkey_with_flags(
                "Software\\Microsoft\\Windows\\CurrentVersion\\Run",
                KEY_WRITE,
            )
            .map_err(|e| e.to_string())?;
        let _ = key.delete_value("Accolade");
        Ok(())
    }
    #[cfg(not(windows))]
    {
        use tauri_plugin_autostart::ManagerExt;
        app.autolaunch().disable().map_err(|e| format!("{}", e))
    }
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_single_instance::init(|app, _args, _cwd| {
            if let Some(window) = app.get_webview_window("main") {
                let _ = window.show();
                let _ = window.set_focus();
            }
        }))
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_shell::init())
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_updater::Builder::new().build())
        .plugin(tauri_plugin_process::init())
        .plugin(tauri_plugin_autostart::init(
            MacosLauncher::LaunchAgent,
            Some(vec!["--minimized"]),
        ))
        .plugin(
            tauri_plugin_global_shortcut::Builder::new()
                .with_handler(|app, _shortcut, event| {
                    if event.state() == tauri_plugin_global_shortcut::ShortcutState::Pressed {
                        log::info!("[Shortcut] Native Triggered!");
                        match screenshots::capture_screenshot(app, None, None, true) {
                            Ok(filename) => {
                                log::info!("[Shortcut] Manual screenshot captured: {}", filename);
                                let _ = app.emit("screenshot-taken", filename);
                            }
                            Err(e) => log::error!("[Shortcut] Manual screenshot failed: {}", e),
                        }
                    }
                })
                .build(),
        )
        .plugin(tauri_plugin_store::Builder::default().build())
        .setup(|app| {
            let args: Vec<String> = std::env::args().collect();
            let is_minimized = args.contains(&"--minimized".to_string());

            if !is_minimized {
                if let Some(window) = app.get_webview_window("main") {
                    let _ = window.show();
                }
            }

            dotenv::dotenv().ok();

            let api_key = std::env::var("STEAM_API_KEY").unwrap_or_default();

            // In debug mode with --minimized (autostart), the Vite dev server is not running,
            // so we must fall back to the production path to avoid a blank overlay window.
            let overlay_url = if cfg!(debug_assertions) && !is_minimized {
                "http://localhost:5173/overlay"
            } else {
                "/overlay"
            };

            let overlay_win = tauri::WebviewWindowBuilder::new(
                app,
                "achievement-overlay",
                tauri::WebviewUrl::App(overlay_url.into()),
            )
            .title("")
            .inner_size(460.0, 260.0)
            .position(30.0, 30.0)
            .transparent(true)
            .decorations(false)
            .shadow(false)
            .background_color(Color(0, 0, 0, 0))
            .always_on_top(true)
            .skip_taskbar(true)
            .visible(false)
            .build()?;

            overlay_win.set_ignore_cursor_events(true)?;

            let quit_i =
                tauri::menu::MenuItem::with_id(app, "quit", "Quitter", true, None::<&str>)?;
            let menu = tauri::menu::Menu::with_items(app, &[&quit_i])?;

            let _tray = tauri::tray::TrayIconBuilder::new()
                .icon(app.default_window_icon().unwrap().clone())
                .tooltip("Accolade")
                .menu(&menu)
                .show_menu_on_left_click(false)
                .on_menu_event(|app, event| {
                    if event.id.as_ref() == "quit" {
                        app.exit(0);
                    }
                })
                .on_tray_icon_event(|tray, event| {
                    if let tauri::tray::TrayIconEvent::Click {
                        button: tauri::tray::MouseButton::Left,
                        button_state: tauri::tray::MouseButtonState::Up,
                        ..
                    } = event
                    {
                        if let Some(window) = tray.app_handle().get_webview_window("main") {
                            let is_visible = window.is_visible().unwrap_or(false);
                            if is_visible {
                                let _ = window.hide();
                            } else {
                                let _ = window.show();
                                let _ = window.set_focus();
                            }
                        }
                    }
                })
                .build(app)?;

            #[cfg(debug_assertions)]
            {
                if !is_minimized {
                    let overlay_win = app.get_webview_window("achievement-overlay").unwrap();
                    overlay_win.open_devtools();
                }
            }

            app.manage(AppState {
                games: Mutex::new(Vec::new()),
                steam_api_key: Mutex::new(api_key),
                ra_username: Mutex::new(String::new()),
                ra_api_key: Mutex::new(String::new()),
                language: Mutex::new("fr".to_string()),
            });

            let app_handle = app.handle().clone();
            watcher::start(app_handle);

            if cfg!(debug_assertions) && !is_minimized {
                app.handle().plugin(
                    tauri_plugin_log::Builder::default()
                        .level(log::LevelFilter::Info)
                        .build(),
                )?;
            }
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            commands::test_achievement_notif,
            commands::get_achievements,
            commands::get_all_games,
            commands::sync_steam_metadata,
            commands::get_steam_user,
            commands::get_steam_owned_games,
            commands::extract_game_theme_color,
            commands::export_to_json,
            commands::export_to_pdf,
            commands::get_profiles_dir,
            commands::open_profiles_dir,
            commands::exit_app,
            commands::is_launched_minimized,
            commands::hide_app,
            commands::toggle_game_favorite,
            commands::add_game_tag,
            commands::remove_game_tag,
            commands::capture_automatic_screenshot,
            commands::capture_manual_screenshot,
            commands::get_screenshots,
            commands::delete_screenshot,
            commands::open_screenshots_dir,
            commands::auto_group_games,
            update_screenshot_shortcut,
            enable_autostart,
            disable_autostart
        ])
        .run(tauri::generate_context!("tauri.conf.json"))
        .expect("error while running tauri application")
}

pub mod achievements;
pub mod color;
pub mod commands;
pub mod emulators;
pub mod screenshots;
pub mod user_data;
pub mod watcher;
