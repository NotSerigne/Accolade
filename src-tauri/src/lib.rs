use std::path::PathBuf;
use crate::achievements::models::Emulator;
use crate::achievements::models::Game;
use crate::emulators::{EmulatorParser, goldberg, empress, onlinefix, rune, codex, game_scanner};

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .setup(|app| {
            let parsers: Vec<Box<dyn EmulatorParser>> = vec![
                Box::new(goldberg::Parser),
                Box::new(empress::Parser),
                Box::new(onlinefix::Parser),
                Box::new(rune::Parser),
                Box::new(codex::Parser),
            ];

            watcher::start(game_scanner(parsers), app.handle().clone());
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