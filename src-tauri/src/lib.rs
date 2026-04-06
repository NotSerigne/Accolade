use tauri::Manager;
use crate::emulators::{EmulatorParser, goldberg, empress, onlinefix, rune, codex, game_scanner};
use crate::achievements::steam::fetch_game_schema;
use crate::achievements::models::Game;

struct AppState {
    games: Vec<Game>,
}
#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
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
            for game in &mut games {
                if let Ok(schema) = tauri::async_runtime::block_on(
                    fetch_game_schema(game.steam_id, &api_key)
                ) {
                    game.achievements = schema;
                }
            }
            app.manage(AppState { games: games.clone() });
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
        .invoke_handler(tauri::generate_handler![commands::get_achievements, commands::get_all_games])
        .run(tauri::generate_context!("tauri.conf.json"))
        .expect("error while running tauri application");
}

pub mod watcher;
pub mod emulators;
pub mod achievements;
pub mod commands;