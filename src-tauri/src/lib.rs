use commands::get_achievements;#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .setup(|app| {
            let games: Vec<Game> = vec![
                Game {
                    name: "Elden Ring",
                    steam_id: 1245620,
                    game_icon: "icon.png",
                    achievements_total: 42,
                    achievements: [],
                    path_buf: "C:/Users/Serigne/AppData/Roaming/Goldberg",
                    emulator: "Goldberg"
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