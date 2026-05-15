// src-tauri/src/lib.rs
use crate::achievements::models::{Achievement, Game, SourceType};
use crate::achievements::steam::{fetch_player_achievements, fetch_steam_metadata_with_client};
use futures::stream::{self, StreamExt};
use std::collections::{HashMap, HashSet};
use std::sync::Mutex;
use tauri::{window::Color, Manager};
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
                println!(
                    "[DEBUG] Steam API error for AppID {}: {}. Keeping game but achievements might be missing.",
                    steam_id, err
                );
            }
        }

        let Some(metadata) = metadata else {
            continue;
        };

        let mut merged_achievements =
            merge_schema_with_local(metadata.achievements.clone(), local_state);

        if let Some(Ok(pa)) = player_achievements_res {
            for ach in &mut merged_achievements {
                if let Some((unlocked, time)) = pa.get(&ach.key) {
                    if *unlocked {
                        ach.unlocked = true;
                        ach.unlocked_time = Some(*time);
                    }
                }
            }
        }

        game.achievements = merged_achievements;
        game.achievements_total = game.achievements.len() as u32;

        if !metadata.name.is_empty() {
            game.name = metadata.name.clone();
        }
        if !metadata.header_image_url.is_empty() {
            game.header_image_url = metadata.header_image_url.clone();
        } else {
            game.header_image_url = format!(
                "https://cdn.cloudflare.steamstatic.com/steam/apps/{}/header.jpg",
                steam_id
            );
        }
        if !metadata.game_icon_url.is_empty() {
            game.game_icon = metadata.game_icon_url.clone();
        }
        if !metadata.background_image_url.is_empty() {
            game.background_image_url = metadata.background_image_url.clone();
        }
    }

    games.retain(|g| g.achievements_total > 0);
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

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_shell::init())
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_autostart::init(
            MacosLauncher::LaunchAgent,
            Some(vec!["--minimized"]),
        ))
        .plugin(tauri_plugin_store::Builder::default().build())
        .setup(|app| {
            let args: Vec<String> = std::env::args().collect();
            let is_minimized = args.contains(&"--minimized".to_string());

            if is_minimized {
                if let Some(window) = app.get_webview_window("main") {
                    let _ = window.hide();
                }
            }

            dotenv::dotenv().ok();

            let api_key = std::env::var("STEAM_API_KEY").unwrap_or_default();

            let overlay_url = if cfg!(debug_assertions) {
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
            commands::exit_app,
            commands::hide_app
        ])
        .run(tauri::generate_context!("tauri.conf.json"))
        .expect("error while running tauri application")
}

pub mod achievements;
pub mod commands;
pub mod emulators;
pub mod watcher;
