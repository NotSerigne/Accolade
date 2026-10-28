// src-tauri/src/hltb.rs
//! HowLongToBeat (HLTB) completion time integration with local disk caching
//! and manual override support.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::path::PathBuf;
use std::time::{SystemTime, UNIX_EPOCH};
use tauri::{AppHandle, Manager};

#[derive(Serialize, Deserialize, Clone, Debug, Default)]
pub struct HltbData {
    pub game_id: String,
    pub title: String,
    /// Main Story completion time in hours (e.g. 24.5).
    pub main_hours: Option<f32>,
    /// Main + Sides completion time in hours (e.g. 42.0).
    pub main_extra_hours: Option<f32>,
    /// Completionist (100%) time in hours (e.g. 68.5).
    pub completionist_hours: Option<f32>,
    /// `"hltb"`, `"custom"`, or `"none"`
    pub source: String,
    pub fetched_at: u64,
}

fn now_unix() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0)
}

fn get_cache_path(app_handle: &AppHandle) -> PathBuf {
    let mut path = app_handle
        .path()
        .app_data_dir()
        .unwrap_or_else(|_| PathBuf::from("."));
    if !path.exists() {
        let _ = fs::create_dir_all(&path);
    }
    path.push("hltb_cache.json");
    path
}

pub fn load_hltb_cache(app_handle: &AppHandle) -> HashMap<String, HltbData> {
    let path = get_cache_path(app_handle);
    if let Ok(content) = fs::read_to_string(path) {
        serde_json::from_str(&content).unwrap_or_default()
    } else {
        HashMap::new()
    }
}

pub fn save_hltb_cache(app_handle: &AppHandle, cache: &HashMap<String, HltbData>) {
    let path = get_cache_path(app_handle);
    if let Ok(content) = serde_json::to_string_pretty(cache) {
        let _ = fs::write(path, content);
    }
}

fn clean_game_title(raw: &str) -> String {
    let cleaned: String = raw
        .chars()
        .filter(|&c| c != '™' && c != '®' && c != '©')
        .map(|c| {
            if c == ':' || c == '-' || c == '—' {
                ' '
            } else {
                c
            }
        })
        .collect();
    cleaned.split_whitespace().collect::<Vec<_>>().join(" ")
}

fn round_hours(seconds: u64) -> Option<f32> {
    if seconds == 0 {
        return None;
    }
    let hours = (seconds as f32) / 3600.0;
    Some((hours * 10.0).round() / 10.0)
}

#[derive(Deserialize)]
struct HltbInitResponse {
    token: Option<String>,
}

#[derive(Deserialize)]
struct HltbSearchResponse {
    data: Option<Vec<HltbGameEntry>>,
}

#[derive(Deserialize)]
struct HltbGameEntry {
    #[serde(default)]
    game_name: String,
    #[serde(default)]
    comp_main: u64,
    #[serde(default)]
    comp_plus: u64,
    #[serde(default)]
    comp_100: u64,
}

async fn query_hltb_remote(game_name: &str) -> Option<HltbGameEntry> {
    let cleaned = clean_game_title(game_name);
    if cleaned.is_empty() || cleaned.starts_with("AppID ") {
        return None;
    }

    let client = reqwest::Client::builder()
        .user_agent("Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/131.0.0.0 Safari/537.36")
        .timeout(std::time::Duration::from_secs(7))
        .build()
        .ok()?;

    let ts = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_millis())
        .unwrap_or(0);

    // 1. Obtain HLTB auth token from /api/search/init or /api/find/init
    let mut token = None;
    let mut search_endpoint = "https://howlongtobeat.com/api/search";

    for (init_url, post_url) in [
        (
            format!("https://howlongtobeat.com/api/search/init?t={}", ts),
            "https://howlongtobeat.com/api/search",
        ),
        (
            format!("https://howlongtobeat.com/api/find/init?t={}", ts),
            "https://howlongtobeat.com/api/find",
        ),
        (
            format!("https://howlongtobeat.com/api/s/init?t={}", ts),
            "https://howlongtobeat.com/api/s",
        ),
    ] {
        if let Ok(resp) = client
            .get(&init_url)
            .header("Referer", "https://howlongtobeat.com/")
            .header("Origin", "https://howlongtobeat.com")
            .send()
            .await
        {
            if resp.status().is_success() {
                if let Ok(init_data) = resp.json::<HltbInitResponse>().await {
                    if let Some(tok) = init_data.token.filter(|s| !s.is_empty()) {
                        token = Some(tok);
                        search_endpoint = post_url;
                        break;
                    }
                }
            }
        }
    }

    let terms: Vec<String> = cleaned.split_whitespace().map(|s| s.to_string()).collect();

    let payload = serde_json::json!({
        "searchType": "games",
        "searchTerms": terms,
        "searchPage": 1,
        "size": 5,
        "searchOptions": {
            "games": {
                "userId": 0,
                "platform": "",
                "sortCategory": "popular",
                "rangeCategory": "main",
                "rangeTime": { "min": null, "max": null },
                "gameplay": { "perspective": "", "flow": "", "genre": "", "difficulty": "" },
                "rangeYear": { "min": "", "max": "" },
                "modifier": ""
            },
            "users": { "sortCategory": "postcount" },
            "lists": { "sortCategory": "follows" },
            "filter": "",
            "sort": 0,
            "randomizer": 0
        },
        "useCache": true
    });

    let endpoints = [
        search_endpoint,
        "https://howlongtobeat.com/api/search",
        "https://howlongtobeat.com/api/find",
    ];

    for ep in endpoints {
        let mut req = client
            .post(ep)
            .header("Content-Type", "application/json")
            .header("Referer", "https://howlongtobeat.com/")
            .header("Origin", "https://howlongtobeat.com");

        if let Some(ref tok) = token {
            req = req.header("x-auth-token", tok);
        }

        if let Ok(resp) = req.json(&payload).send().await {
            if resp.status().is_success() {
                if let Ok(parsed) = resp.json::<HltbSearchResponse>().await {
                    if let Some(entries) = parsed.data {
                        let lower_wanted = cleaned.to_lowercase();
                        // Prefer exact or closest title match with non-zero times
                        if let Some(best) = entries
                            .iter()
                            .filter(|e| e.comp_main > 0 || e.comp_plus > 0 || e.comp_100 > 0)
                            .min_by_key(|e| {
                                let entry_clean = clean_game_title(&e.game_name).to_lowercase();
                                if entry_clean == lower_wanted {
                                    0usize
                                } else if entry_clean.contains(&lower_wanted)
                                    || lower_wanted.contains(&entry_clean)
                                {
                                    1usize
                                } else {
                                    2usize
                                }
                            })
                        {
                            return Some(HltbGameEntry {
                                game_name: best.game_name.clone(),
                                comp_main: best.comp_main,
                                comp_plus: best.comp_plus,
                                comp_100: best.comp_100,
                            });
                        }
                    }
                }
            }
        }
    }

    None
}

#[tauri::command]
pub async fn get_hltb_data(
    app_handle: AppHandle,
    game_id: String,
    game_name: String,
    force_refresh: Option<bool>,
) -> Result<HltbData, String> {
    let force = force_refresh.unwrap_or(false);
    let mut cache = load_hltb_cache(&app_handle);

    if !force {
        if let Some(existing) = cache.get(&game_id) {
            // Never overwrite a custom user entry unless force_refresh is true
            if existing.source == "custom" || existing.source == "hltb" {
                return Ok(existing.clone());
            }
            // Retry "none" entries only after 1 hour
            if now_unix().saturating_sub(existing.fetched_at) < 3600 {
                return Ok(existing.clone());
            }
        }
    }

    if let Some(entry) = query_hltb_remote(&game_name).await {
        let data = HltbData {
            game_id: game_id.clone(),
            title: entry.game_name,
            main_hours: round_hours(entry.comp_main),
            main_extra_hours: round_hours(entry.comp_plus),
            completionist_hours: round_hours(entry.comp_100),
            source: "hltb".to_string(),
            fetched_at: now_unix(),
        };
        cache.insert(game_id, data.clone());
        save_hltb_cache(&app_handle, &cache);
        return Ok(data);
    }

    // If a custom entry was already saved and remote failed during force_refresh, preserve it
    if let Some(existing) = cache.get(&game_id) {
        if existing.source == "custom" {
            return Ok(existing.clone());
        }
    }

    let empty = HltbData {
        game_id: game_id.clone(),
        title: game_name,
        main_hours: None,
        main_extra_hours: None,
        completionist_hours: None,
        source: "none".to_string(),
        fetched_at: now_unix(),
    };
    cache.insert(game_id, empty.clone());
    save_hltb_cache(&app_handle, &cache);
    Ok(empty)
}

#[tauri::command]
pub fn set_custom_hltb_data(
    app_handle: AppHandle,
    game_id: String,
    game_name: String,
    main_hours: Option<f32>,
    main_extra_hours: Option<f32>,
    completionist_hours: Option<f32>,
) -> Result<HltbData, String> {
    let mut cache = load_hltb_cache(&app_handle);
    let data = HltbData {
        game_id: game_id.clone(),
        title: game_name,
        main_hours: main_hours.filter(|&v| v > 0.0),
        main_extra_hours: main_extra_hours.filter(|&v| v > 0.0),
        completionist_hours: completionist_hours.filter(|&v| v > 0.0),
        source: "custom".to_string(),
        fetched_at: now_unix(),
    };
    cache.insert(game_id, data.clone());
    save_hltb_cache(&app_handle, &cache);
    Ok(data)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cleans_game_titles_and_rounds_hours() {
        assert_eq!(
            clean_game_title("The Witcher® 3: Wild Hunt™ - Complete Edition"),
            "The Witcher 3 Wild Hunt Complete Edition"
        );
        assert_eq!(round_hours(0), None);
        assert_eq!(round_hours(3600 * 12 + 1800), Some(12.5));
    }
}
