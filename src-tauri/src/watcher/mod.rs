// watcher: surveille les fichiers des émulateurs en temps réel
use notify::{Watcher, RecursiveMode, recommended_watcher};
use std::path::PathBuf;
use tauri::Emitter;
use crate::achievements::match_emulator;
use crate::achievements::models::Game;

pub fn start(games: Vec<Game>, app_handle: tauri::AppHandle) {
    let (tx, rx) = std::sync::mpsc::channel();
    std::thread::spawn(move || {
        let mut watcher = recommended_watcher(tx).unwrap();
        for game in &games {
            watcher.watch(&game.path_buf, RecursiveMode::Recursive).unwrap();

        }
        loop{
            if let Ok(unwrapped) = rx.recv() {
                if let Ok(event) = unwrapped {
                    if let Some(game) = games.iter().find(|g| {
                        event.paths.iter().any(|p| p.starts_with(&g.path_buf))
                    }) {
                        let achievements = match_emulator(game.clone());
                        app_handle.clone().emit("achievement-unlocked", &achievements);
                    }
                }
            }
        }
    });
}
