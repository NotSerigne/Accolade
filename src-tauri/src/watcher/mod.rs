// watcher: surveille les fichiers des émulateurs en temps réel
use notify::{Watcher, RecursiveMode, recommended_watcher};
use tauri::Emitter;
use crate::achievements::match_emulator;
use crate::achievements::models::{Achievement, Game};
use crate::achievements::steam::fetch_app_name_by_appid;
use std::collections::{HashMap, HashSet};
use std::path::Path;
use serde::Serialize;

#[derive(Serialize, Clone)]
pub struct AchievementNotifPayload {
    pub name: String,
    pub desc: String,
    pub icon: Option<String>,
    pub rarity: String,
    pub completionpercentage: Option<f32>,
    pub unlocked: u32,
    pub total: u32,
    #[serde(default)]
    pub test: bool,
}

fn unlocked_keys(achievements: &[Achievement]) -> HashSet<String> {
    achievements
        .iter()
        .filter(|achievement| achievement.unlocked || achievement.unlocked_time.is_some())
        .map(|achievement| achievement.key.trim().to_lowercase())
        .collect()
}

fn is_technical_name(name: &str) -> bool {
    matches!(
        name.trim().to_ascii_lowercase().as_str(),
        "codex" | "empress" | "rune" | "goldberg" | "onlinefix" | "steam_settings" | "achievements" | "remote" | "stats"
    )
}

fn fallback_game_name_from_path(game: &Game) -> Option<String> {
    let mut cursor = Path::new(&game.path_buf);
    for _ in 0..6 {
        let Some(parent) = cursor.parent() else { break };
        if let Some(segment) = parent.file_name().and_then(|value| value.to_str()) {
            let candidate = segment.trim();
            if !candidate.is_empty() && candidate.parse::<u32>().is_err() && !is_technical_name(candidate) {
                return Some(candidate.to_string());
            }
        }
        cursor = parent;
    }

    None
}

fn display_game_name(game: &Game, name_cache: &mut HashMap<u32, String>, api_lookup_attempted: &mut HashSet<u32>) -> String {
    let direct_name = game.name.trim();
    if !direct_name.is_empty() && !is_technical_name(direct_name) {
        return direct_name.to_string();
    }

    if let Some(cached) = name_cache.get(&game.steam_id) {
        return cached.clone();
    }

    if game.steam_id > 0 && api_lookup_attempted.insert(game.steam_id) {
        if let Ok(Some(api_name)) = tauri::async_runtime::block_on(fetch_app_name_by_appid(game.steam_id)) {
            if !is_technical_name(&api_name) {
                name_cache.insert(game.steam_id, api_name.clone());
                return api_name;
            }
        }
    }

    if let Some(path_name) = fallback_game_name_from_path(game) {
        name_cache.insert(game.steam_id, path_name.clone());
        return path_name;
    }

    format!("Jeu inconnu (AppID {})", game.steam_id)
}

fn rarity_label(percentage: Option<f32>) -> String {
    match percentage {
        Some(p) if p <= 5.0  => "LEGENDARY".to_string(),
        Some(p) if p <= 15.0 => "EPIC".to_string(),
        Some(p) if p <= 30.0 => "RARE".to_string(),
        Some(p) if p <= 60.0 => "UNCOMMON".to_string(),
        Some(_)              => "COMMON".to_string(),
        None                 => "UNKNOWN".to_string(),
    }
}

pub fn start(games: Vec<Game>, app_handle: tauri::AppHandle) {
    let (tx, rx) = std::sync::mpsc::channel();

    std::thread::spawn(move || {
        let mut watcher = recommended_watcher(tx).unwrap();
        let mut unlocked_by_game: HashMap<u32, HashSet<String>> = games
            .iter()
            .map(|game| (game.steam_id, unlocked_keys(&game.achievements)))
            .collect();
        let mut game_name_cache: HashMap<u32, String> = HashMap::new();
        let mut api_lookup_attempted: HashSet<u32> = HashSet::new();

        for game in &games {
            let path = std::path::Path::new(&game.path_buf);
            let _ = watcher.watch(path, RecursiveMode::Recursive);
        }

        while let Ok(Ok(event)) = rx.recv() {
            for game in &games {
                let game_path = std::path::Path::new(&game.path_buf);

                if event.paths.iter().any(|p| p.starts_with(game_path)) {
                    let achievements = match_emulator(game.clone());
                    let current_unlocked = unlocked_keys(&achievements);
                    let previous_unlocked = unlocked_by_game.entry(game.steam_id).or_default();

                    let newly_unlocked: Vec<&Achievement> = achievements
                        .iter()
                        .filter(|a| {
                            let key = a.key.trim().to_lowercase();
                            current_unlocked.contains(&key) && !previous_unlocked.contains(&key)
                        })
                        .collect();

                    if !newly_unlocked.is_empty() {
                        let display_name = display_game_name(game, &mut game_name_cache, &mut api_lookup_attempted);
                        let total = achievements.len() as u32;
                        let unlocked_count = current_unlocked.len() as u32;

                        for ach in &newly_unlocked {
                            println!(
                                "[DEBUG][achievements] Nouveau succes debloque pour '{}' (AppID {}): {}",
                                display_name, game.steam_id, ach.key
                            );

                            let payload = AchievementNotifPayload {
                                name: ach.name.clone(),
                                desc: String::new(),
                                icon: Some(ach.icon.clone()),
                                rarity: "UNKNOWN".to_string(),
                                completionpercentage: None,
                                unlocked: unlocked_count,
                                total,
                                test: false,
                            };

                            let _ = app_handle.emit_to("achievement-overlay", "achievement-notif", &payload);
                        }
                    }

                    *previous_unlocked = current_unlocked;
                    // Event séparé pour la mise à jour de la liste complète dans l'UI principale
                    let _ = app_handle.emit("achievements-updated", &achievements);
                }
            }
        }
    });
}

pub fn emit_test_notification(app_handle: &tauri::AppHandle) {
    let payload = AchievementNotifPayload {
        name: "Test Achievement".to_string(),
        desc: "This is a test notification".to_string(),
        icon: None,
        rarity: "EPIC".to_string(),
        completionpercentage: Some(12.5),
        unlocked: 5,
        total: 50,
        test: true,
    };

    let _ = app_handle.emit_to("achievement-overlay", "achievement-notif", &payload);
}