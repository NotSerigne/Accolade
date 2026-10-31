// src-tauri/src/achievements/steam.rs
use crate::achievements::models::Achievement;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::OnceLock;
use std::time::{Duration, Instant};

/// Returns `true` only when `name` is a real game title and not a placeholder
/// like `"AppID 12345"`, `"steam_12345"`, or empty.
pub fn is_valid_game_name(name: &str) -> bool {
    let trimmed = name.trim();
    if trimmed.is_empty() {
        return false;
    }
    let lower = trimmed.to_ascii_lowercase();
    !lower.starts_with("appid ")
        && !lower.starts_with("steam_")
        && !lower.starts_with("id ")
        && !lower.starts_with("jeu inconnu")
        && !lower.starts_with("unknown game")
}

/// Global throttle for `store.steampowered.com/api/appdetails` so we never exceed
/// 1 request at a time and enforce at least 1.2s between consecutive Store requests.
static APPDETAILS_THROTTLE: OnceLock<futures::lock::Mutex<Instant>> = OnceLock::new();

const APPDETAILS_MIN_GAP: Duration = Duration::from_millis(1200);
const APPDETAILS_MAX_RETRIES: u32 = 3;

async fn async_sleep(dur: Duration) {
    let _ = tauri::async_runtime::spawn_blocking(move || {
        std::thread::sleep(dur);
    })
    .await;
}

async fn wait_appdetails_slot() {
    let lock = APPDETAILS_THROTTLE
        .get_or_init(|| futures::lock::Mutex::new(Instant::now() - APPDETAILS_MIN_GAP));
    let mut last_req = lock.lock().await;
    let elapsed = last_req.elapsed();
    if elapsed < APPDETAILS_MIN_GAP {
        async_sleep(APPDETAILS_MIN_GAP - elapsed).await;
    }
    *last_req = Instant::now();
}

#[derive(Clone)]
pub struct SteamMetadata {
    pub name: String,
    pub game_icon_url: String,
    pub header_image_url: String,
    pub background_image_url: String,
    pub achievements: Vec<Achievement>,
    pub genres: Vec<String>,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct SteamUser {
    pub steamid: String,
    pub personaname: String,
    pub avatarfull: String,
    #[serde(default)]
    pub communityvisibilitystate: Option<u32>,
}

#[derive(Deserialize)]
struct PlayerSummariesResponse {
    response: PlayerSummariesData,
}

#[derive(Deserialize)]
struct PlayerSummariesData {
    players: Vec<SteamUser>,
}

#[derive(Deserialize)]
struct OwnedGamesResponse {
    response: OwnedGamesData,
}

#[derive(Deserialize)]
struct OwnedGamesData {
    games: Option<Vec<OwnedGame>>,
}

#[derive(Deserialize, Serialize, Clone, Debug)]
pub struct OwnedGame {
    pub appid: u32,
    pub name: Option<String>,
    pub img_icon_url: Option<String>,
    /// Total playtime in minutes as reported by Steam IPlayerService/GetOwnedGames.
    #[serde(default)]
    pub playtime_forever: Option<u64>,
}

#[derive(Deserialize, Debug)]
struct PlayerAchievementsResponse {
    playerstats: PlayerStats,
}

#[derive(Deserialize, Debug)]
struct PlayerStats {
    achievements: Option<Vec<PlayerAchievement>>,
    success: bool,
    error: Option<String>,
}

#[derive(Deserialize, Debug)]
struct PlayerAchievement {
    apiname: String,
    achieved: u32,
    unlocktime: u64,
    #[serde(default)]
    name: String,
    #[serde(default)]
    description: String,
}

pub struct PlayerAchievementInfo {
    pub unlocked: bool,
    pub unlock_time: u64,
    pub name: String,
    pub description: String,
}

#[derive(Deserialize, Clone)]
struct AppDetailsEnvelope {
    success: bool,
    data: Option<AppDetailsData>,
}

#[derive(Deserialize, Clone)]
struct AppDetailsData {
    #[serde(default)]
    name: String,
    #[serde(default)]
    header_image: String,
    #[serde(default)]
    capsule_imagev5: String,
    #[serde(default)]
    background: String,
    #[serde(default)]
    background_raw: String,
    #[serde(default)]
    genres: Vec<SteamGenre>,
}

#[derive(Deserialize, Clone)]
struct SteamGenre {
    description: String,
}

#[derive(Default)]
struct RawAchievement {
    internal_name: String,
    localized_name: String,
    localized_desc: String,
    icon: String,
    icon_gray: String,
    hidden: bool,
    player_percent_unlocked: f64,
}

/// Normalizes any Steam achievement icon URL to the current `shared.fastly.steamstatic.com/community_assets`
/// CDN endpoint. Valve no longer mirrors newly added DLC achievement icons onto the legacy
/// `steamcdn-a.akamaihd.net/steamcommunity/public/images/apps/...` path (which returns 404 for new DLCs).
pub fn normalize_steam_icon_url(url: &str) -> String {
    let trimmed = url.trim();
    if trimmed.is_empty() {
        return String::new();
    }
    let https = if trimmed.starts_with("http://") {
        trimmed.replacen("http://", "https://", 1)
    } else {
        trimmed.to_string()
    };
    if let Some(rest) = https
        .strip_prefix("https://steamcdn-a.akamaihd.net/steamcommunity/public/images/apps/")
        .or_else(|| {
            https.strip_prefix(
                "https://cdn.cloudflare.steamstatic.com/steamcommunity/public/images/apps/",
            )
        })
        .or_else(|| {
            https.strip_prefix(
                "https://cdn.akamai.steamstatic.com/steamcommunity/public/images/apps/",
            )
        })
        .or_else(|| {
            https.strip_prefix("https://media.steampowered.com/steamcommunity/public/images/apps/")
        })
    {
        return format!(
            "https://shared.fastly.steamstatic.com/community_assets/images/apps/{rest}"
        );
    }
    https
}

fn build_icon_url(steam_id: u32, hash: &str) -> String {
    let trimmed = hash.trim();
    if trimmed.is_empty() {
        return String::new();
    }
    if trimmed.starts_with("https://") || trimmed.starts_with("http://") {
        return normalize_steam_icon_url(trimmed);
    }
    format!(
        "https://shared.fastly.steamstatic.com/community_assets/images/apps/{steam_id}/{trimmed}"
    )
}

fn extract_xml_value(line: &str, tag: &str) -> Option<String> {
    let open = format!("<{}>", tag);
    let close = format!("</{}>", tag);
    if let Some(start) = line.find(&open) {
        let start_pos = start + open.len();
        if let Some(end) = line[start_pos..].find(&close) {
            return Some(line[start_pos..start_pos + end].to_string());
        }
    }
    None
}

fn decode_html_entities(s: &str) -> String {
    s.replace("&apos;", "'")
        .replace("&quot;", "\"")
        .replace("&amp;", "&")
        .replace("&lt;", "<")
        .replace("&gt;", ">")
}

fn parse_achievements_xml(xml: &str, steam_id: u32) -> Vec<RawAchievement> {
    let mut achievements = Vec::new();
    let mut current: Option<RawAchievement> = None;

    for line in xml.lines() {
        let trimmed = line.trim();

        if trimmed.contains("<message>") {
            current = Some(RawAchievement::default());
        }

        if let Some(ref mut ach) = current {
            if let Some(v) = extract_xml_value(trimmed, "internal_name") {
                ach.internal_name = decode_html_entities(&v);
            }
            if let Some(v) = extract_xml_value(trimmed, "localized_name") {
                ach.localized_name = decode_html_entities(&v);
            }
            if let Some(v) = extract_xml_value(trimmed, "localized_desc") {
                ach.localized_desc = decode_html_entities(&v);
            }
            if let Some(hash) = extract_xml_value(trimmed, "icon") {
                ach.icon = build_icon_url(steam_id, &hash);
            }
            if let Some(hash) = extract_xml_value(trimmed, "icon_gray") {
                ach.icon_gray = build_icon_url(steam_id, &hash);
            }
            if let Some(v) = extract_xml_value(trimmed, "hidden") {
                ach.hidden = v == "true" || v == "1";
            }
            if let Some(v) = extract_xml_value(trimmed, "player_percent_unlocked") {
                ach.player_percent_unlocked = v.parse().unwrap_or(0.0);
            }
        }

        if trimmed.contains("</message>") {
            if let Some(ach) = current.take() {
                if !ach.internal_name.is_empty() {
                    achievements.push(ach);
                }
            }
        }
    }

    achievements
}

/// Fetches `store.steampowered.com/api/appdetails` with:
/// - Global rate limiting (1 request at a time, >= 1.2s gap)
/// - Exponential backoff retry on HTTP 403 / 429 (2s, 4s, 8s)
async fn fetch_appdetails_throttled(
    client: &reqwest::Client,
    steam_id: u32,
    steam_language: &str,
) -> Option<AppDetailsData> {
    let details_url = format!(
        "https://store.steampowered.com/api/appdetails?appids={steam_id}&l={steam_language}"
    );

    for attempt in 0..=APPDETAILS_MAX_RETRIES {
        wait_appdetails_slot().await;

        match client.get(&details_url).send().await {
            Ok(resp) => {
                let status = resp.status();
                if status.as_u16() == 429 || status.as_u16() == 403 {
                    if attempt < APPDETAILS_MAX_RETRIES {
                        let backoff_secs = 2u64 << attempt; // 2s, 4s, 8s
                        log::warn!(
                            "[steam appdetails] HTTP {} for AppID {} (attempt {}/{}), retrying in {}s...",
                            status,
                            steam_id,
                            attempt + 1,
                            APPDETAILS_MAX_RETRIES + 1,
                            backoff_secs
                        );
                        async_sleep(Duration::from_secs(backoff_secs)).await;
                        continue;
                    } else {
                        log::warn!(
                            "[steam appdetails] HTTP {} for AppID {} — exhausted retries",
                            status,
                            steam_id
                        );
                        return None;
                    }
                }

                if !status.is_success() {
                    log::warn!("[steam appdetails] HTTP {} for AppID {}", status, steam_id);
                    return None;
                }

                let details_map = resp
                    .json::<HashMap<String, AppDetailsEnvelope>>()
                    .await
                    .unwrap_or_default();

                return details_map.get(&steam_id.to_string()).and_then(|e| {
                    if e.success {
                        e.data.clone()
                    } else {
                        None
                    }
                });
            }
            Err(e) => {
                log::warn!(
                    "[steam appdetails] request failed for AppID {}: {}",
                    steam_id,
                    e
                );
                return None;
            }
        }
    }

    None
}

pub async fn fetch_steam_user(
    api_key: &str,
    steam_id: &str,
) -> Result<Option<SteamUser>, reqwest::Error> {
    let client = reqwest::Client::new();
    let url = format!(
        "https://api.steampowered.com/ISteamUser/GetPlayerSummaries/v0002/?key={api_key}&steamids={steam_id}"
    );
    let resp = client
        .get(url)
        .send()
        .await?
        .json::<PlayerSummariesResponse>()
        .await?;
    Ok(resp.response.players.into_iter().next())
}

pub async fn fetch_owned_games(
    api_key: &str,
    steam_id: &str,
    language: &str,
) -> Result<Vec<OwnedGame>, reqwest::Error> {
    let client = reqwest::Client::new();
    let steam_language = match language {
        "fr" => "french",
        "en" => "english",
        "es" => "spanish",
        "de" => "german",
        "it" => "italian",
        _ => "english",
    };
    let url = format!(
        "https://api.steampowered.com/IPlayerService/GetOwnedGames/v0001/?key={api_key}&steamid={steam_id}&include_appinfo=1&include_played_free_games=1&l={steam_language}&format=json"
    );
    let resp = client
        .get(url)
        .send()
        .await?
        .json::<OwnedGamesResponse>()
        .await?;
    Ok(resp.response.games.unwrap_or_default())
}

pub async fn fetch_recently_played_games(
    api_key: &str,
    steam_id: &str,
) -> Result<Vec<OwnedGame>, reqwest::Error> {
    let client = reqwest::Client::new();
    let url = format!(
        "https://api.steampowered.com/IPlayerService/GetRecentlyPlayedGames/v0001/?key={api_key}&steamid={steam_id}&format=json"
    );
    let resp = client
        .get(url)
        .send()
        .await?
        .json::<OwnedGamesResponse>()
        .await?;
    Ok(resp.response.games.unwrap_or_default())
}

pub fn get_local_steam_appids(steam_id64: &str) -> Vec<u32> {
    let mut appids = Vec::new();

    #[cfg(target_os = "windows")]
    {
        use winreg::enums::*;
        use winreg::RegKey;

        let hkcu = RegKey::predef(HKEY_CURRENT_USER);
        if let Ok(steam_key) = hkcu.open_subkey("Software\\Valve\\Steam") {
            if let Ok(steam_path) = steam_key.get_value::<String, _>("SteamPath") {
                let steam_id64_num: u64 = steam_id64.parse().unwrap_or(0);
                if steam_id64_num > 76561197960265728 {
                    let account_id = steam_id64_num - 76561197960265728;
                    let userdata_path = std::path::Path::new(&steam_path)
                        .join("userdata")
                        .join(account_id.to_string());

                    if let Ok(entries) = std::fs::read_dir(userdata_path) {
                        for entry in entries.flatten() {
                            if entry.path().is_dir() {
                                if let Some(name) = entry.file_name().to_str() {
                                    if let Ok(appid) = name.parse::<u32>() {
                                        if appid > 10 {
                                            // Filter out Steam internal appids
                                            appids.push(appid);
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
    }
    appids
}

pub async fn fetch_player_achievements(
    api_key: &str,
    steam_id: &str,
    app_id: u32,
    language: &str,
) -> Result<HashMap<String, PlayerAchievementInfo>, String> {
    let client = reqwest::Client::new();
    let steam_language = match language {
        "fr" => "french",
        "en" => "english",
        "es" => "spanish",
        "de" => "german",
        "it" => "italian",
        _ => "english",
    };
    let url = format!(
        "https://api.steampowered.com/ISteamUserStats/GetPlayerAchievements/v0001/?appid={app_id}&key={api_key}&steamid={steam_id}&l={steam_language}"
    );

    let resp = client
        .get(url)
        .send()
        .await
        .map_err(|e| e.to_string())?
        .json::<PlayerAchievementsResponse>()
        .await
        .map_err(|e| e.to_string())?;
    let mut map = HashMap::new();

    if resp.playerstats.success {
        if let Some(achievements) = resp.playerstats.achievements {
            for ach in achievements {
                map.insert(
                    ach.apiname,
                    PlayerAchievementInfo {
                        unlocked: ach.achieved == 1,
                        unlock_time: ach.unlocktime,
                        name: ach.name,
                        description: ach.description,
                    },
                );
            }
        }
    } else if let Some(err) = resp.playerstats.error {
        log::warn!("[steam] API error for AppID {}: {}", app_id, err);
        return Err(err);
    }

    Ok(map)
}

pub async fn fetch_app_name_by_appid_with_client(
    client: &reqwest::Client,
    steam_id: u32,
    language: &str,
) -> Result<Option<String>, reqwest::Error> {
    let steam_language = match language {
        "fr" => "french",
        "en" => "english",
        "es" => "spanish",
        "de" => "german",
        "it" => "italian",
        _ => "english",
    };

    let app_name = fetch_appdetails_throttled(client, steam_id, steam_language)
        .await
        .map(|d| d.name.trim().to_string())
        .filter(|n| is_valid_game_name(n));

    Ok(app_name)
}

pub async fn fetch_app_name_by_appid(
    steam_id: u32,
    language: &str,
) -> Result<Option<String>, reqwest::Error> {
    let client = reqwest::Client::new();
    fetch_app_name_by_appid_with_client(&client, steam_id, language).await
}

pub async fn fetch_steam_metadata_with_client(
    client: &reqwest::Client,
    steam_id: u32,
    api_key: &str,
    language: &str,
    need_appdetails: bool,
) -> Result<SteamMetadata, reqwest::Error> {
    let steam_language = match language {
        "fr" => "french",
        "en" => "english",
        "es" => "spanish",
        "de" => "german",
        "it" => "italian",
        _ => "english",
    };

    let lang_url = format!(
        "https://api.steampowered.com/IPlayerService/GetGameAchievements/v1/?key={api_key}&appid={steam_id}&language={steam_language}&format=xml"
    );
    let en_url = format!(
        "https://api.steampowered.com/IPlayerService/GetGameAchievements/v1/?key={api_key}&appid={steam_id}&language=english&format=xml"
    );

    let lang_req = client.get(lang_url).send();
    let en_req = client.get(en_url).send();

    let (lang_res, en_res) = futures::join!(lang_req, en_req);

    let mut achievements_lang = Vec::new();
    match lang_res {
        Ok(r) if r.status().is_success() => {
            if let Ok(text) = r.text().await {
                achievements_lang = parse_achievements_xml(&text, steam_id);
            }
        }
        Ok(r) => log::warn!(
            "[steam metadata] GetGameAchievements ({} lang) HTTP {} for AppID {}",
            steam_language,
            r.status(),
            steam_id
        ),
        Err(e) => log::warn!(
            "[steam metadata] GetGameAchievements ({} lang) request failed for AppID {}: {}",
            steam_language,
            steam_id,
            e
        ),
    }

    let mut achievements_en = Vec::new();
    match en_res {
        Ok(r) if r.status().is_success() => {
            if let Ok(text) = r.text().await {
                achievements_en = parse_achievements_xml(&text, steam_id);
            }
        }
        Ok(r) => log::warn!(
            "[steam metadata] GetGameAchievements (english) HTTP {} for AppID {}",
            r.status(),
            steam_id
        ),
        Err(e) => log::warn!(
            "[steam metadata] GetGameAchievements (english) request failed for AppID {}: {}",
            steam_id,
            e
        ),
    }

    let details = if need_appdetails {
        fetch_appdetails_throttled(client, steam_id, steam_language).await
    } else {
        None
    };

    let en_map: HashMap<String, &RawAchievement> = achievements_en
        .iter()
        .map(|a| (a.internal_name.to_lowercase(), a))
        .collect();

    let source = if !achievements_lang.is_empty() {
        &achievements_lang
    } else {
        &achievements_en
    };

    let achievements: Vec<Achievement> = source
        .iter()
        .map(|a| {
            let en = en_map.get(&a.internal_name.to_lowercase());

            let name = if a.localized_name.trim().is_empty() {
                en.map(|e| e.localized_name.clone())
                    .unwrap_or_else(|| a.internal_name.clone())
            } else {
                a.localized_name.clone()
            };
            let desc = if a.localized_desc.trim().is_empty() {
                en.map(|e| e.localized_desc.clone()).unwrap_or_default()
            } else {
                a.localized_desc.clone()
            };
            let icon = if a.icon.is_empty() {
                en.map(|e| e.icon.clone()).unwrap_or_default()
            } else {
                a.icon.clone()
            };
            let icon_gray = if a.icon_gray.is_empty() {
                en.map(|e| e.icon_gray.clone()).unwrap_or_default()
            } else {
                a.icon_gray.clone()
            };

            Achievement {
                key: a.internal_name.clone(),
                name,
                unlocked: false,
                icon,
                icon_gray,
                unlocked_time: None,
                rarity: String::new(),
                completionpercentage: format!("{:.1}", a.player_percent_unlocked),
                desc,
                hidden: a.hidden,
                ..Default::default()
            }
        })
        .collect();

    let game_icon_url = details
        .as_ref()
        .map(|d| d.capsule_imagev5.clone())
        .filter(|s| !s.is_empty())
        .unwrap_or_default();

    let header_image_url = details
        .as_ref()
        .map(|d| d.header_image.clone())
        .unwrap_or_default();

    let background_image_url = details
        .as_ref()
        .map(|d| {
            if !d.background_raw.is_empty() {
                d.background_raw.clone()
            } else {
                d.background.clone()
            }
        })
        .unwrap_or_default();

    // IMPORTANT: Never return "AppID X" as `name` here — return an empty string if appdetails
    // was skipped or failed, so the caller preserves its cached/existing name or falls back to Plan B.
    let game_name = details
        .as_ref()
        .map(|d| d.name.trim().to_string())
        .filter(|s| is_valid_game_name(s))
        .unwrap_or_default();

    Ok(SteamMetadata {
        name: game_name,
        game_icon_url,
        header_image_url,
        background_image_url,
        achievements,
        genres: details
            .map(|d| d.genres.iter().map(|g| g.description.clone()).collect())
            .unwrap_or_default(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn validates_game_names_against_placeholders() {
        assert!(!is_valid_game_name(""));
        assert!(!is_valid_game_name("   "));
        assert!(!is_valid_game_name("AppID 1245620"));
        assert!(!is_valid_game_name("appid 480"));
        assert!(!is_valid_game_name("steam_1245620"));
        assert!(!is_valid_game_name("Jeu inconnu (AppID 1245620)"));
        assert!(!is_valid_game_name("Unknown Game (AppID 1245620)"));
        assert!(is_valid_game_name("ELDEN RING"));
        assert!(is_valid_game_name("Jurassic World Evolution 3"));
    }

    #[test]
    fn normalizes_and_builds_fastly_community_assets_icon_urls() {
        let built = build_icon_url(2958130, "3800f6ef861ed486fa3d1eafe4cdfea0245de936.jpg");
        assert_eq!(
            built,
            "https://shared.fastly.steamstatic.com/community_assets/images/apps/2958130/3800f6ef861ed486fa3d1eafe4cdfea0245de936.jpg"
        );

        let legacy = "https://steamcdn-a.akamaihd.net/steamcommunity/public/images/apps/2958130/3800f6ef861ed486fa3d1eafe4cdfea0245de936.jpg";
        assert_eq!(
            normalize_steam_icon_url(legacy),
            "https://shared.fastly.steamstatic.com/community_assets/images/apps/2958130/3800f6ef861ed486fa3d1eafe4cdfea0245de936.jpg"
        );
    }
}
