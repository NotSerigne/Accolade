// watcher: surveille les fichiers des émulateurs en temps réel
use notify::{Watcher, RecursiveMode, recommended_watcher};
use std::path::PathBuf;
use crate::achievements::models::Game;
pub fn start(games: Vec<Game>) -> std::sync::mpsc::Receiver<notify::Result<notify::Event>> {
    let (tx, rx) = std::sync::mpsc::channel();
    std::thread::spawn(move || {
        let mut watcher = recommended_watcher(tx).unwrap();
        for game in &games {
            watcher.watch(&game.path_buf, RecursiveMode::Recursive).unwrap();
        }
        loop{}
    });
    rx
}
