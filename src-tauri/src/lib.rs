use tauri::Manager;
use std::collections::HashMap;
use std::sync::Mutex;
use crate::emulators::{EmulatorParser, goldberg, empress, onlinefix, rune, codex, game_scanner};
use crate::achievements::steam::fetch_steam_metadata;
use crate::achievements::models::{Achievement, Game};

pub struct AppState {
    pub(crate) games: Mutex<Vec<Game>>,
    pub(crate) steam_api_key: Mutex<String>,
}

fn merge_schema_with_local(schema: Vec<Achievement>, local: &[Achievement]) -> Vec<Achievement> {
    let local_map: HashMap<String, &Achievement> = local
        .iter()
        .map(|a| (a.key.trim().to_lowercase(), a))
        .collect();

    schema
        .into_iter()
        .map(|s| {
            if let Some(live) = local_map.get(&s.key.trim().to_lowercase()) {
                Achievement {
                    key: s.key,
                    name: if s.name.is_empty() { live.name.clone() } else { s.name },
                    unlocked: live.unlocked,
                    icon: if s.icon.is_empty() { live.icon.clone() } else { s.icon },
                    icon_gray: if s.icon_gray.is_empty() { live.icon_gray.clone() } else { s.icon_gray },
                    unlocked_time: live.unlocked_time,
                    rarity: live.rarity.clone(),
                    completionpercentage: if s.completionpercentage.is_empty() { live.completionpercentage.clone() } else { s.completionpercentage.clone() },
                    desc: if s.desc.is_empty() { live.desc.clone() } else { s.desc },
                    hidden: s.hidden,
                }
            } else {
                s
            }
        })
        .collect()
}

pub(crate) async fn enrich_games_with_steam(games: &mut [Game], api_key: &str) {
    if api_key.trim().is_empty() {
        return;
    }

    for game in games {
        let local_state = game.achievements.clone();
        if let Ok(metadata) = fetch_steam_metadata(game.steam_id, api_key).await {
            game.achievements = merge_schema_with_local(metadata.achievements, &local_state);
            game.achievements_total = game.achievements.len() as u32;

            if !metadata.name.is_empty() {
                game.name = metadata.name;
            }
            if !metadata.game_icon_url.is_empty() && game.game_icon.is_empty() {
                game.game_icon = metadata.game_icon_url;
            }
            if !metadata.header_image_url.is_empty() {
                game.header_image_url = metadata.header_image_url;
            }
            if !metadata.background_image_url.is_empty() {
                game.background_image_url = metadata.background_image_url;
            }
        }
    }
}

pub async fn apply_steamgriddb_icons(games: &mut [Game], api_key: &str) {
    let client = reqwest::Client::new();

    for game in games.iter_mut() {
        // Étape 1 : récupérer le game_id SteamGridDB depuis le steam_id
        let search_url = format!(
            "https://www.steamgriddb.com/api/v2/games/steam/{}",
            game.steam_id
        );

        let game_id = match client
            .get(&search_url)
            .header("Authorization", format!("Bearer {}", api_key))
            .send()
            .await
        {
            Ok(resp) => match resp.json::<serde_json::Value>().await {
                Ok(json) => match json["data"]["id"].as_u64() {
                    Some(id) => id,
                    None => continue,
                },
                Err(_) => continue,
            },
            Err(_) => continue,
        };

        // Étape 2 : récupérer les icônes pour ce game_id
        if game.game_icon.is_empty() {
            let icons_url = format!(
                "https://www.steamgriddb.com/api/v2/icons/game/{}",
                game_id
            );

            let icon_url = match client
                .get(&icons_url)
                .header("Authorization", format!("Bearer {}", api_key))
                .send()
                .await
            {
                Ok(resp) => match resp.json::<serde_json::Value>().await {
                    Ok(json) => match json["data"][0]["thumb"].as_str() {
                        Some(url) => url.to_string(),
                        None => {
                            println!("DEBUG sgdb no icon for app_id={}", game.steam_id);
                            continue;
                        }
                    },
                    Err(_) => continue,
                },
                Err(_) => continue,
            };

            println!("DEBUG sgdb icon: app_id={} url={}", game.steam_id, icon_url);
            game.game_icon = icon_url;
        }
    }
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_store::Builder::default().build())
        .setup(|app| {
            dotenv::dotenv().ok();
            let parsers: Vec<Box<dyn EmulatorParser>> = vec![
                Box::new(goldberg::Parser),
                Box::new(empress::Parser),
                Box::new(onlinefix::Parser),
                Box::new(rune::Parser),
                Box::new(codex::Parser),
            ];

            let mut games = game_scanner(parsers);
            let api_key = std::env::var("STEAM_API_KEY").unwrap_or_default();

            tauri::async_runtime::block_on(enrich_games_with_steam(&mut games, &api_key));

            let sgdb_key = std::env::var("STEAMGRIDDB_API_KEY").unwrap_or_default();
            tauri::async_runtime::block_on(apply_steamgriddb_icons(&mut games, &sgdb_key));

            app.manage(AppState {
                games: Mutex::new(games.clone()),
                steam_api_key: Mutex::new(api_key),
            });
            watcher::start(games, app.handle().clone());

            if cfg!(debug_assertions) {
                app.handle().plugin(
                    tauri_plugin_log::Builder::default()
                        .level(log::LevelFilter::Info)
                        .build(),
                )?;
            }
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            commands::get_achievements,
            commands::get_all_games,
            commands::sync_steam_metadata
        ])
        .run(tauri::generate_context!("tauri.conf.json"))
        .expect("error while running tauri application")
}

pub mod watcher;
pub mod emulators;
pub mod achievements;
pub mod commands;