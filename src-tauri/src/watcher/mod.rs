// watcher: surveille les fichiers des émulateurs en temps réel
use notify::{Watcher, RecursiveMode, recommended_watcher};
use tauri::Emitter;
use crate::achievements::match_emulator;
use crate::achievements::models::Game;

pub fn start(games: Vec<Game>, app_handle: tauri::AppHandle) {
    let (tx, rx) = std::sync::mpsc::channel();

    std::thread::spawn(move || {
        let mut watcher = recommended_watcher(tx).unwrap();

        // Ajoute tous les jeux au watcher
        for game in &games {
            let path = std::path::Path::new(&game.path_buf);
            let _ = watcher.watch(path, RecursiveMode::Recursive);
        }

        // Écoute les changements
        while let Ok(Ok(event)) = rx.recv() {
            for game in &games {
                let game_path = std::path::Path::new(&game.path_buf);

                // Si le fichier est dans le dossier du jeu
                if event.paths.iter().any(|p| p.starts_with(game_path)) {
                    let achievements = match_emulator(game.clone());
                    let _ = app_handle.emit("achievement-unlocked", &achievements);
                }
            }
        }
    });
}