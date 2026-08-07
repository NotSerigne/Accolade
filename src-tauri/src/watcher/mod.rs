// src-tauri/src/watcher/mod.rs
use crate::achievements::match_emulator;
use crate::achievements::models::{Achievement, Game};
use crate::achievements::steam::fetch_app_name_by_appid;
use crate::AppState;
use notify::{recommended_watcher, RecursiveMode, Watcher};
use serde::Serialize;
use std::collections::{HashMap, HashSet};
use std::path::Path;
use std::time::Duration;
use tauri::Emitter;
use tauri::Manager;

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
    pub is_platinum: bool,
    pub unlocked_time: Option<u64>,
    pub game_id: Option<String>,
    pub ach_key: Option<String>,
}

#[derive(Serialize, Clone)]
pub struct AchievementsUpdatedPayload {
    pub game_id: String,
    pub achievements: Vec<Achievement>,
    pub achievements_total: u32,
}

fn live_keys(achievements: &[Achievement]) -> HashSet<String> {
    achievements
        .iter()
        .map(|achievement| achievement.key.trim().to_lowercase())
        .collect()
}

fn merge_live_achievements(
    schema: &[Achievement],
    live: &[Achievement],
    previous_live_keys: &HashSet<String>,
    current_live_keys: &HashSet<String>,
) -> Vec<Achievement> {
    let live_map: HashMap<String, &Achievement> = live
        .iter()
        .map(|a| (a.key.trim().to_lowercase(), a))
        .collect();
    let mut seen_schema_keys: HashSet<String> = HashSet::new();

    let mut merged: Vec<Achievement> = schema
        .iter()
        .cloned()
        .map(|mut s| {
            let key = s.key.trim().to_lowercase();
            seen_schema_keys.insert(key.clone());

            if let Some(live_ach) = live_map.get(&key) {
                s.unlocked = live_ach.unlocked;
                if live_ach.unlocked {
                    if let Some(t) = live_ach.unlocked_time {
                        if t > 0 {
                            s.unlocked_time = Some(t);
                        } else if s.unlocked_time.is_none() {
                            s.unlocked_time = Some(0);
                        }
                    } else if s.unlocked_time.is_none() {
                        s.unlocked_time = Some(0);
                    }
                } else {
                    s.unlocked_time = None;
                }

                if s.name.is_empty() && !live_ach.name.is_empty() {
                    s.name = live_ach.name.clone();
                }
                if s.desc.is_empty() && !live_ach.desc.is_empty() {
                    s.desc = live_ach.desc.clone();
                }
                if s.icon.is_empty() && !live_ach.icon.is_empty() {
                    s.icon = live_ach.icon.clone();
                }
            } else if previous_live_keys.contains(&key) && !current_live_keys.contains(&key) {
                s.unlocked = false;
                s.unlocked_time = None;
            }

            s
        })
        .collect();

    for ach in live {
        let key = ach.key.trim().to_lowercase();
        if !seen_schema_keys.contains(&key) {
            merged.push(ach.clone());
        }
    }

    merged
}

fn is_technical_name(name: &str) -> bool {
    matches!(
        name.trim().to_ascii_lowercase().as_str(),
        "codex"
            | "empress"
            | "rune"
            | "goldberg"
            | "onlinefix"
            | "steam_settings"
            | "achievements"
            | "remote"
            | "stats"
    )
}

fn fallback_game_name_from_path(game: &Game) -> Option<String> {
    let path_str = game.path_buf.as_ref()?;
    let mut cursor = Path::new(path_str);
    for _ in 0..6 {
        let Some(parent) = cursor.parent() else { break };
        if let Some(segment) = parent.file_name().and_then(|value| value.to_str()) {
            let candidate = segment.trim();
            if !candidate.is_empty()
                && candidate.parse::<u32>().is_err()
                && !is_technical_name(candidate)
            {
                return Some(candidate.to_string());
            }
        }
        cursor = parent;
    }

    None
}

fn display_game_name(
    game: &Game,
    name_cache: &mut HashMap<String, String>,
    api_lookup_attempted: &mut HashSet<u32>,
    language: &str,
) -> String {
    let direct_name = game.name.trim();
    if !direct_name.is_empty() && !is_technical_name(direct_name) {
        return direct_name.to_string();
    }

    if let Some(cached) = name_cache.get(&game.id) {
        return cached.clone();
    }

    if let Some(steam_id) = game.steam_id {
        if steam_id > 0 && api_lookup_attempted.insert(steam_id) {
            let api_res =
                tauri::async_runtime::block_on(fetch_app_name_by_appid(steam_id, language));
            if let Ok(Some(api_name)) = api_res {
                if !is_technical_name(&api_name) {
                    name_cache.insert(game.id.clone(), api_name.clone());
                    return api_name;
                }
            }
        }
    }

    if let Some(path_name) = fallback_game_name_from_path(game) {
        name_cache.insert(game.id.clone(), path_name.clone());
        return path_name;
    }

    let unknown = match language {
        "fr" => "Jeu inconnu",
        "en" => "Unknown Game",
        "es" => "Juego desconocido",
        "de" => "Unbekanntes Spiel",
        "it" => "Gioco sconosciuto",
        _ => "Unknown Game",
    };

    if let Some(sid) = game.steam_id {
        format!("{} (AppID {})", unknown, sid)
    } else {
        format!("{} (ID {})", unknown, game.id)
    }
}

fn rarity_label(percentage: Option<f32>, language: &str) -> String {
    match language {
        "fr" => match percentage {
            Some(p) if p <= 0.1 => "Mythique".to_string(),
            Some(p) if p <= 1.0 => "Légendaire".to_string(),
            Some(p) if p <= 3.0 => "Épique".to_string(),
            Some(p) if p <= 7.0 => "Très rare".to_string(),
            Some(p) if p <= 15.0 => "Rare".to_string(),
            Some(p) if p <= 35.0 => "Peu commun".to_string(),
            Some(_) => "Commun".to_string(),
            None => String::new(),
        },
        "en" => match percentage {
            Some(p) if p <= 0.1 => "Mythic".to_string(),
            Some(p) if p <= 1.0 => "Legendary".to_string(),
            Some(p) if p <= 3.0 => "Epic".to_string(),
            Some(p) if p <= 7.0 => "Very Rare".to_string(),
            Some(p) if p <= 15.0 => "Rare".to_string(),
            Some(p) if p <= 35.0 => "Uncommon".to_string(),
            Some(_) => "Common".to_string(),
            None => String::new(),
        },
        "es" => match percentage {
            Some(p) if p <= 0.1 => "Mítico".to_string(),
            Some(p) if p <= 1.0 => "Legendario".to_string(),
            Some(p) if p <= 3.0 => "Épico".to_string(),
            Some(p) if p <= 7.0 => "Muy raro".to_string(),
            Some(p) if p <= 15.0 => "Raro".to_string(),
            Some(p) if p <= 35.0 => "Poco común".to_string(),
            Some(_) => "Común".to_string(),
            None => String::new(),
        },
        "de" => match percentage {
            Some(p) if p <= 0.1 => "Mythisch".to_string(),
            Some(p) if p <= 1.0 => "Legendär".to_string(),
            Some(p) if p <= 3.0 => "Episch".to_string(),
            Some(p) if p <= 7.0 => "Sehr selten".to_string(),
            Some(p) if p <= 15.0 => "Selten".to_string(),
            Some(p) if p <= 35.0 => "Ungewöhnlich".to_string(),
            Some(_) => "Häufig".to_string(),
            None => String::new(),
        },
        "it" => match percentage {
            Some(p) if p <= 0.1 => "Mitico".to_string(),
            Some(p) if p <= 1.0 => "Leggendario".to_string(),
            Some(p) if p <= 3.0 => "Epico".to_string(),
            Some(p) if p <= 7.0 => "Molto raro".to_string(),
            Some(p) if p <= 15.0 => "Raro".to_string(),
            Some(p) if p <= 35.0 => "Non comune".to_string(),
            Some(_) => "Comune".to_string(),
            None => String::new(),
        },
        _ => match percentage {
            Some(p) if p <= 0.1 => "Mythic".to_string(),
            Some(p) if p <= 1.0 => "Legendary".to_string(),
            Some(p) if p <= 3.0 => "Epic".to_string(),
            Some(p) if p <= 7.0 => "Very Rare".to_string(),
            Some(p) if p <= 15.0 => "Rare".to_string(),
            Some(p) if p <= 35.0 => "Uncommon".to_string(),
            Some(_) => "Common".to_string(),
            None => String::new(),
        },
    }
}

pub fn start(app_handle: tauri::AppHandle) {
    let (tx, rx) = std::sync::mpsc::channel();

    std::thread::spawn(move || {
        let mut watcher = recommended_watcher(tx).unwrap();

        let parsers: Vec<Box<dyn crate::emulators::EmulatorParser>> = vec![
            Box::new(crate::emulators::goldberg::Parser),
            Box::new(crate::emulators::empress::Parser),
            Box::new(crate::emulators::onlinefix::Parser),
            Box::new(crate::emulators::rune::Parser),
            Box::new(crate::emulators::codex::Parser),
        ];

        let mut watch_paths = HashSet::new();
        for parser in &parsers {
            for loc in parser.known_locations() {
                if loc.exists() {
                    watch_paths.insert(loc);
                }
            }
        }

        for path in watch_paths {
            let _ = watcher.watch(&path, RecursiveMode::Recursive);
        }

        let mut unlocked_by_game: HashMap<String, HashSet<String>> = HashMap::new();
        let mut live_keys_by_game: HashMap<String, HashSet<String>> = HashMap::new();
        let mut game_name_cache: HashMap<String, String> = HashMap::new();
        let mut api_lookup_attempted: HashSet<u32> = HashSet::new();

        // Initialize state from AppState if any
        {
            if let Ok(games) = app_handle.state::<AppState>().games.lock() {
                for game in games.iter() {
                    let unlocked = game
                        .achievements
                        .iter()
                        .filter(|a| a.unlocked || a.unlocked_time.is_some())
                        .map(|a| a.key.trim().to_lowercase())
                        .collect();
                    unlocked_by_game.insert(game.id.clone(), unlocked);
                    live_keys_by_game.insert(game.id.clone(), live_keys(&game.achievements));
                }
            }
        }

        let _ = app_handle.emit("watcher-status", true);

        while let Ok(Ok(first_event)) = rx.recv() {
            let mut paths_changed = HashSet::new();
            for p in first_event.paths {
                paths_changed.insert(p);
            }

            std::thread::sleep(Duration::from_millis(200));

            while let Ok(Ok(evt)) = rx.try_recv() {
                for p in evt.paths {
                    paths_changed.insert(p);
                }
            }

            let games = {
                let state = app_handle.state::<AppState>();
                state.games.lock().map(|g| g.clone()).unwrap_or_default()
            };

            for game in &games {
                let Some(ref path_buf) = game.path_buf else {
                    continue;
                };
                let game_path = std::path::Path::new(path_buf);

                // Normalize a path for robust case-insensitive comparison on Windows:
                // strip UNC prefix (\\?\), lowercase, unify separators.
                let normalize_path = |p: &std::path::Path| -> String {
                    let s = p.to_string_lossy();
                    s.trim_start_matches(r"\\?\")
                        .to_lowercase()
                        .replace('/', "\\")
                };

                let norm_game = normalize_path(game_path);
                // Also precompute the parent directory of the game path (for file-based paths like achievements.json)
                let norm_game_parent = game_path.parent().map(normalize_path).unwrap_or_default();

                let path_matches = !path_buf.is_empty()
                    && paths_changed.iter().any(|p| {
                        let norm_p = normalize_path(p);
                        // Match if:
                        // 1. The changed path IS exactly the game file (e.g. achievements.json)
                        // 2. The changed path is inside the game directory (when path_buf is a dir)
                        // 3. The changed path is the parent dir of the game file (notify sometimes emits the folder)
                        // 4. The changed path is a sibling file inside the game file's parent dir
                        norm_p == norm_game
                            || norm_p.starts_with(&format!("{}\\", norm_game))
                            || (!norm_game_parent.is_empty()
                                && (norm_p == norm_game_parent
                                    || norm_p.starts_with(&format!("{}\\", norm_game_parent))))
                    });

                if path_matches {
                    let achievements = match_emulator(game.clone());
                    let current_live_keys = live_keys(&achievements);

                    // Snapshot the baseline BEFORE the merge so we can diff against it.
                    // Seed unlocked_by_game / live_keys_by_game from AppState if not yet seen.
                    if !unlocked_by_game.contains_key(&game.id)
                        || !live_keys_by_game.contains_key(&game.id)
                    {
                        let stored_achs: Option<Vec<Achievement>> = {
                            let state = app_handle.state::<AppState>();
                            state.games.lock().ok().and_then(|gs| {
                                gs.iter()
                                    .find(|g| g.id == game.id)
                                    .map(|g| g.achievements.clone())
                            })
                        };
                        if let Some(ref achs) = stored_achs {
                            if !unlocked_by_game.contains_key(&game.id) {
                                let baseline: HashSet<String> = achs
                                    .iter()
                                    .filter(|a| a.unlocked || a.unlocked_time.is_some())
                                    .map(|a| a.key.trim().to_lowercase())
                                    .collect();
                                unlocked_by_game.insert(game.id.clone(), baseline);
                            }
                            if !live_keys_by_game.contains_key(&game.id) {
                                live_keys_by_game.insert(game.id.clone(), live_keys(achs));
                            }
                        }
                    }

                    // Snapshot previous state before updating AppState.
                    let snapshot_previous_unlocked: HashSet<String> =
                        unlocked_by_game.get(&game.id).cloned().unwrap_or_default();
                    let snapshot_previous_live_keys: HashSet<String> =
                        live_keys_by_game.get(&game.id).cloned().unwrap_or_default();

                    let (ui_achievements, ui_total) = {
                        let state = app_handle.state::<AppState>();
                        state
                            .games
                            .lock()
                            .ok()
                            .and_then(|mut gs| {
                                gs.iter_mut().find(|g| g.id == game.id).map(|stored| {
                                    let merged = merge_live_achievements(
                                        &stored.achievements,
                                        &achievements,
                                        &snapshot_previous_live_keys,
                                        &current_live_keys,
                                    );
                                    let total = merged.len() as u32;
                                    stored.achievements = merged.clone();
                                    stored.achievements_total = total;
                                    (merged, total)
                                })
                            })
                            .unwrap_or_else(|| {
                                let fallback = achievements.clone();
                                let total = fallback.len() as u32;
                                (fallback, total)
                            })
                    };

                    let mut true_current_unlocked = HashSet::new();
                    for a in &ui_achievements {
                        if a.unlocked || a.unlocked_time.is_some() {
                            true_current_unlocked.insert(a.key.trim().to_lowercase());
                        }
                    }

                    let previous_unlocked = &snapshot_previous_unlocked;

                    let newly_unlocked: Vec<Achievement> = ui_achievements
                        .iter()
                        .filter(|a| {
                            let key = a.key.trim().to_lowercase();
                            true_current_unlocked.contains(&key)
                                && !previous_unlocked.contains(&key)
                        })
                        .cloned()
                        .collect();

                    if !newly_unlocked.is_empty() {
                        let total = ui_total;
                        let unlocked_count_total = true_current_unlocked.len() as u32;
                        let is_game_completed = total > 0 && unlocked_count_total == total;

                        let language = {
                            let state = app_handle.state::<AppState>();
                            state
                                .language
                                .lock()
                                .map(|l| l.clone())
                                .unwrap_or_else(|_| "fr".to_string())
                        };

                        let display_name = display_game_name(
                            game,
                            &mut game_name_cache,
                            &mut api_lookup_attempted,
                            &language,
                        );
                        let enriched_game_achievements: Vec<Achievement> = ui_achievements.clone();

                        let num_new = newly_unlocked.len();
                        for (idx, ach) in newly_unlocked.iter().enumerate() {
                            log::info!(
                                "[watcher] Achievement unlocked for '{}' (ID {}): {}",
                                display_name,
                                game.id,
                                ach.key
                            );

                            let enriched = enriched_game_achievements.iter().find(|a| {
                                a.key.trim().to_lowercase() == ach.key.trim().to_lowercase()
                            });

                            let ach_name = enriched
                                .map(|e| e.name.clone())
                                .filter(|n| !n.is_empty())
                                .unwrap_or_else(|| ach.name.clone());

                            let ach_desc = enriched
                                .map(|e| e.desc.clone())
                                .filter(|d| !d.is_empty())
                                .unwrap_or_default();

                            let ach_icon = enriched
                                .map(|e| e.icon.clone())
                                .filter(|s| !s.is_empty())
                                .unwrap_or_else(|| ach.icon.clone());

                            let completion_pct: Option<f32> = enriched
                                .and_then(|e| e.completionpercentage.parse::<f32>().ok())
                                .filter(|&p| p > 0.0);

                            let rarity = rarity_label(completion_pct, &language);

                            // Only the very last achievement in the batch gets the platinum flag if the game is now 100%
                            let is_platinum = is_game_completed && (idx == num_new - 1);

                            // We need to calculate the intermediate unlocked count for the UI ring
                            // If we unlock 3 at once and end at 50/50, the first should show 48/50, second 49/50, third 50/50
                            let current_unlocked_display =
                                (unlocked_count_total - (num_new as u32)) + (idx as u32) + 1;

                            let payload = AchievementNotifPayload {
                                name: ach_name,
                                desc: ach_desc,
                                icon: if ach_icon.is_empty() {
                                    None
                                } else {
                                    Some(ach_icon)
                                },
                                rarity,
                                completionpercentage: completion_pct,
                                unlocked: current_unlocked_display,
                                total,
                                test: false,
                                is_platinum,
                                unlocked_time: ach.unlocked_time,
                                game_id: Some(game.id.clone()),
                                ach_key: Some(ach.key.clone()),
                            };

                            let _ = app_handle.emit_to(
                                "achievement-overlay",
                                "achievement-notif",
                                &payload,
                            );
                        }
                    }

                    unlocked_by_game.insert(game.id.clone(), true_current_unlocked);
                    live_keys_by_game.insert(game.id.clone(), current_live_keys);

                    let updated_payload = AchievementsUpdatedPayload {
                        game_id: game.id.clone(),
                        achievements: ui_achievements,
                        achievements_total: ui_total,
                    };
                    let _ = app_handle.emit("achievements-updated", &updated_payload);
                }
            }
        }
    });
}

pub fn emit_test_notification(app_handle: &tauri::AppHandle) {
    let language = {
        let state = app_handle.state::<AppState>();
        state
            .language
            .lock()
            .map(|l| l.clone())
            .unwrap_or_else(|_| "fr".to_string())
    };

    let payload = AchievementNotifPayload {
        name: "Test Achievement".to_string(),
        desc: "This is a test notification".to_string(),
        icon: None,
        rarity: rarity_label(Some(2.5), &language),
        completionpercentage: Some(2.5),
        unlocked: 5,
        total: 50,
        test: true,
        is_platinum: false,
        unlocked_time: Some(1714470000),
        game_id: None,
        ach_key: None,
    };

    let _ = app_handle.emit_to("achievement-overlay", "achievement-notif", &payload);
}
