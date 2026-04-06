use std::path::PathBuf;
use crate::achievements::models::Emulator;
use crate::achievements::models::Game;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .setup(|app| {
            let games: Vec<Game> = vec![
                Game {
                    name: String::from("Elden Ring"),
                    steam_id: 1245620,
                    game_icon: String::from("icon.png"),
                    achievements_total: 42,
                    achievements: vec![],
                    path_buf: PathBuf::from("C:/Users/Serigne/AppData/Roaming/Goldberg"),
                    emulator: Emulator::Goldberg,
                }
            ];
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
        .invoke_handler(tauri::generate_handler![commands::get_achievements])
        .run(tauri::generate_context!("tauri.conf.json"))
        .expect("error while running tauri application");
}

pub mod watcher;
pub mod emulators;
pub mod achievements;
pub mod commands;