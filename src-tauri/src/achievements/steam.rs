// src-tauri/src/achievements/steam.rs
use crate::achievements::models::{
    deserialize_bool_or_int, filter_public_groups, Achievement, AchievementGroup,
};
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};
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

static STEAM_HTTP_CLIENT: OnceLock<reqwest::Client> = OnceLock::new();

pub(crate) fn steam_http_client() -> &'static reqwest::Client {
    STEAM_HTTP_CLIENT.get_or_init(|| {
        reqwest::Client::builder()
            .pool_max_idle_per_host(16)
            .tcp_nodelay(true)
            .timeout(Duration::from_secs(15))
            .build()
            .unwrap_or_else(|_| reqwest::Client::new())
    })
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
    pub achievement_groups: Vec<AchievementGroup>,
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

fn deserialize_optional_string_or_number<'de, D>(
    deserializer: D,
) -> Result<Option<String>, D::Error>
where
    D: serde::Deserializer<'de>,
{
    let val = Option::<serde_json::Value>::deserialize(deserializer)?;
    Ok(match val {
        Some(serde_json::Value::String(s)) => Some(s),
        Some(serde_json::Value::Number(n)) => Some(n.to_string()),
        _ => None,
    })
}

#[derive(Deserialize, Default)]
struct GetGameAchievementsEnvelope {
    #[serde(default)]
    response: GetGameAchievementsResponse,
}

#[derive(Deserialize, Default)]
struct GetGameAchievementsResponse {
    #[serde(default)]
    achievements: Vec<SteamApiAchievement>,
    #[serde(default)]
    groups: Vec<AchievementGroup>,
}

#[derive(Deserialize, Default)]
struct SteamApiAchievement {
    #[serde(default)]
    internal_name: String,
    #[serde(default)]
    localized_name: String,
    #[serde(default)]
    localized_desc: String,
    #[serde(default)]
    icon: String,
    #[serde(default)]
    icon_gray: String,
    #[serde(default, deserialize_with = "deserialize_bool_or_int")]
    hidden: bool,
    #[serde(default, deserialize_with = "deserialize_optional_string_or_number")]
    player_percent_unlocked: Option<String>,
    #[serde(default)]
    groupid: u32,
    #[serde(default, deserialize_with = "deserialize_bool_or_int")]
    archived: bool,
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

/// Parses `IPlayerService/GetGameAchievements/v1` JSON into `(achievements, public_groups)`.
/// Filters out `developeronly` / `!ispublic` groups and excludes achievements belonging to them,
/// propagates group-level `archived` flags to their achievements, and prefixes bare icon filenames
/// onto `shared.fastly.steamstatic.com/community_assets`.
pub fn parse_achievements_json(
    json: &str,
    steam_id: u32,
) -> Result<(Vec<Achievement>, Vec<AchievementGroup>), String> {
    let envelope: GetGameAchievementsEnvelope =
        serde_json::from_str(json).map_err(|e| e.to_string())?;

    let excluded_group_ids: HashSet<u32> = envelope
        .response
        .groups
        .iter()
        .filter(|g| g.developeronly || !g.ispublic)
        .map(|g| g.groupid)
        .collect();
    let archived_group_ids: HashSet<u32> = envelope
        .response
        .groups
        .iter()
        .filter(|g| g.archived)
        .map(|g| g.groupid)
        .collect();
    let public_groups = filter_public_groups(&envelope.response.groups);

    let mut achievements = Vec::with_capacity(envelope.response.achievements.len());
    for raw in envelope.response.achievements {
        let key = raw.internal_name.trim();
        if key.is_empty() {
            continue;
        }
        if raw.groupid != 0 && excluded_group_ids.contains(&raw.groupid) {
            continue;
        }
        let pct = raw
            .player_percent_unlocked
            .as_deref()
            .and_then(|s| s.trim().parse::<f32>().ok())
            .filter(|p| p.is_finite())
            .unwrap_or(0.0);

        achievements.push(Achievement {
            key: key.to_string(),
            name: raw.localized_name,
            unlocked: false,
            icon: build_icon_url(steam_id, &raw.icon),
            icon_gray: build_icon_url(steam_id, &raw.icon_gray),
            unlocked_time: None,
            rarity: String::new(),
            completionpercentage: format!("{:.1}", pct),
            desc: raw.localized_desc,
            hidden: raw.hidden,
            progress: None,
            max_progress: None,
            groupid: raw.groupid,
            archived: raw.archived || archived_group_ids.contains(&raw.groupid),
        });
    }

    Ok((achievements, public_groups))
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
    let client = steam_http_client();
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
    let client = steam_http_client();
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
    let client = steam_http_client();
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
    let client = steam_http_client();
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
    let client = steam_http_client();
    fetch_app_name_by_appid_with_client(client, steam_id, language).await
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
        "https://api.steampowered.com/IPlayerService/GetGameAchievements/v1/?key={api_key}&appid={steam_id}&language={steam_language}"
    );

    let (lang_res, details) = if need_appdetails {
        futures::join!(
            client.get(lang_url).send(),
            fetch_appdetails_throttled(client, steam_id, steam_language)
        )
    } else {
        (client.get(lang_url).send().await, None)
    };

    let mut achievements_lang = Vec::new();
    let mut groups_lang = Vec::new();
    match lang_res {
        Ok(r) if r.status().is_success() => {
            if let Ok(text) = r.text().await {
                match parse_achievements_json(&text, steam_id) {
                    Ok((achs, grps)) => {
                        achievements_lang = achs;
                        groups_lang = grps;
                    }
                    Err(e) => log::warn!(
                        "[steam metadata] Failed to parse GetGameAchievements JSON ({} lang) for AppID {}: {}",
                        steam_language,
                        steam_id,
                        e
                    ),
                }
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

    let needs_english_fallback = !steam_language.eq_ignore_ascii_case("english")
        && (achievements_lang.is_empty()
            || achievements_lang.iter().any(|a| {
                a.name.trim().is_empty()
                    || a.desc.trim().is_empty()
                    || a.icon.trim().is_empty()
                    || a.icon_gray.trim().is_empty()
            })
            || groups_lang.iter().any(|g| g.localized_name.trim().is_empty()));

    let mut achievements_en = Vec::new();
    let mut groups_en = Vec::new();
    if needs_english_fallback {
        let en_url = format!(
            "https://api.steampowered.com/IPlayerService/GetGameAchievements/v1/?key={api_key}&appid={steam_id}&language=english"
        );
        match client.get(en_url).send().await {
            Ok(r) if r.status().is_success() => {
                if let Ok(text) = r.text().await {
                    match parse_achievements_json(&text, steam_id) {
                        Ok((achs, grps)) => {
                            achievements_en = achs;
                            groups_en = grps;
                        }
                        Err(e) => log::warn!(
                            "[steam metadata] Failed to parse GetGameAchievements JSON (english) for AppID {}: {}",
                            steam_id,
                            e
                        ),
                    }
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
    }

    let en_map: HashMap<String, &Achievement> = achievements_en
        .iter()
        .map(|a| (a.key.to_lowercase(), a))
        .collect();

    let source = if !achievements_lang.is_empty() {
        achievements_lang
    } else {
        achievements_en.clone()
    };

    let achievements: Vec<Achievement> = source
        .into_iter()
        .map(|mut a| {
            let en = en_map.get(&a.key.to_lowercase());

            if a.name.trim().is_empty() {
                a.name = en
                    .map(|e| e.name.clone())
                    .filter(|s| !s.trim().is_empty())
                    .unwrap_or_else(|| a.key.clone());
            }
            if a.desc.trim().is_empty() {
                if let Some(en_ach) = en {
                    a.desc = en_ach.desc.clone();
                }
            }
            if a.icon.is_empty() {
                if let Some(en_ach) = en {
                    a.icon = en_ach.icon.clone();
                }
            }
            if a.icon_gray.is_empty() {
                if let Some(en_ach) = en {
                    a.icon_gray = en_ach.icon_gray.clone();
                }
            }
            if a.completionpercentage == "0.0" {
                if let Some(en_ach) = en {
                    if en_ach.completionpercentage != "0.0" {
                        a.completionpercentage = en_ach.completionpercentage.clone();
                    }
                }
            }

            a
        })
        .collect();

    let en_group_map: HashMap<u32, &AchievementGroup> =
        groups_en.iter().map(|g| (g.groupid, g)).collect();
    let achievement_groups = if groups_lang.is_empty() {
        groups_en
    } else {
        let mut merged_groups: Vec<AchievementGroup> = groups_lang
            .into_iter()
            .map(|mut g| {
                if g.localized_name.trim().is_empty() {
                    if let Some(en_g) = en_group_map.get(&g.groupid) {
                        g.localized_name = en_g.localized_name.clone();
                    }
                }
                g
            })
            .collect();
        for en_g in &groups_en {
            if !merged_groups.iter().any(|g| g.groupid == en_g.groupid) {
                merged_groups.push((*en_g).clone());
            }
        }
        filter_public_groups(&merged_groups)
    };

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
        achievement_groups,
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

    #[test]
    fn parses_json_with_groups_dlc_archived_and_filtering() {
        let json = r#"{
            "response": {
                "achievements": [
                    {
                        "internal_name": "BASE_1",
                        "localized_name": "First Step",
                        "localized_desc": "Start the game",
                        "icon": "abc123.jpg",
                        "icon_gray": "abc123_gray.jpg",
                        "hidden": false,
                        "player_percent_unlocked": "42.5",
                        "groupid": 0,
                        "archived": false
                    },
                    {
                        "internal_name": "DLC_2_ACH",
                        "localized_name": "Second Expansion",
                        "localized_desc": "Beat expansion 2",
                        "icon": "https://steamcdn-a.akamaihd.net/steamcommunity/public/images/apps/1234/dlc2.jpg",
                        "icon_gray": "dlc2_gray.jpg",
                        "hidden": true,
                        "player_percent_unlocked": 12.34,
                        "groupid": 2,
                        "archived": false
                    },
                    {
                        "internal_name": "GROUP_ARCHIVED_ACH",
                        "localized_name": "Old Event",
                        "localized_desc": "From archived set",
                        "icon": "old.jpg",
                        "icon_gray": "old_gray.jpg",
                        "player_percent_unlocked": null,
                        "groupid": 3
                    },
                    {
                        "internal_name": "DEV_ONLY_ACH",
                        "localized_name": "Internal Debug",
                        "icon": "dev.jpg",
                        "groupid": 4
                    },
                    {
                        "internal_name": "PRIVATE_ACH",
                        "localized_name": "Unpublished",
                        "icon": "priv.jpg",
                        "groupid": 5
                    }
                ],
                "groups": [
                    {
                        "groupid": 2,
                        "localized_name": "Expansion II",
                        "dlcappid": 5002,
                        "archived": false,
                        "developeronly": false,
                        "ispublic": true,
                        "order": 2
                    },
                    {
                        "groupid": 1,
                        "localized_name": "Expansion I",
                        "dlcappid": 5001,
                        "order": 1
                    },
                    {
                        "groupid": 3,
                        "localized_name": "Retired Events",
                        "dlcappid": 0,
                        "archived": true,
                        "ispublic": true,
                        "order": 3
                    },
                    {
                        "groupid": 4,
                        "localized_name": "Dev Testing",
                        "developeronly": true,
                        "ispublic": true,
                        "order": 0
                    },
                    {
                        "groupid": 5,
                        "localized_name": "Hidden Set",
                        "developeronly": false,
                        "ispublic": false,
                        "order": 0
                    }
                ]
            }
        }"#;

        let (achs, groups) = parse_achievements_json(json, 1234).unwrap();

        let group_ids: Vec<u32> = groups.iter().map(|g| g.groupid).collect();
        assert_eq!(group_ids, vec![1, 2, 3]);
        assert_eq!(groups[0].dlcappid, 5001);
        assert!(groups[0].ispublic);

        assert_eq!(achs.len(), 3);
        let base = &achs[0];
        assert_eq!(base.key, "BASE_1");
        assert_eq!(
            base.icon,
            "https://shared.fastly.steamstatic.com/community_assets/images/apps/1234/abc123.jpg"
        );
        assert_eq!(
            base.icon_gray,
            "https://shared.fastly.steamstatic.com/community_assets/images/apps/1234/abc123_gray.jpg"
        );
        assert_eq!(base.completionpercentage, "42.5");
        assert_eq!(base.groupid, 0);
        assert!(!base.archived);

        let dlc2 = &achs[1];
        assert_eq!(dlc2.key, "DLC_2_ACH");
        assert_eq!(
            dlc2.icon,
            "https://shared.fastly.steamstatic.com/community_assets/images/apps/1234/dlc2.jpg"
        );
        assert_eq!(dlc2.completionpercentage, "12.3");
        assert!(dlc2.hidden);
        assert_eq!(dlc2.groupid, 2);
        assert!(!dlc2.archived);

        let arch = &achs[2];
        assert_eq!(arch.key, "GROUP_ARCHIVED_ACH");
        assert_eq!(arch.completionpercentage, "0.0");
        assert_eq!(arch.groupid, 3);
        assert!(arch.archived);
    }

    #[test]
    fn parses_json_without_groups_backwards_compatibly() {
        let json = r#"{
            "response": {
                "achievements": [
                    {
                        "internal_name": "LEGACY_ACH",
                        "localized_name": "Legacy",
                        "localized_desc": "No groups array in payload",
                        "icon": "legacy.jpg",
                        "icon_gray": "legacy_g.jpg",
                        "hidden": false,
                        "player_percent_unlocked": "88.9"
                    }
                ]
            }
        }"#;

        let (achs, groups) = parse_achievements_json(json, 480).unwrap();
        assert!(groups.is_empty());
        assert_eq!(achs.len(), 1);
        assert_eq!(achs[0].key, "LEGACY_ACH");
        assert_eq!(achs[0].groupid, 0);
        assert!(!achs[0].archived);
        assert_eq!(achs[0].completionpercentage, "88.9");
    }
}
