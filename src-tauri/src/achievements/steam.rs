// src-tauri/src/achievements/steam.rs
use crate::achievements::models::Achievement;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Clone)]
pub struct SteamMetadata {
    pub name: String,
    pub game_icon_url: String,
    pub header_image_url: String,
    pub background_image_url: String,
    pub achievements: Vec<Achievement>,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct SteamUser {
    pub steamid: String,
    pub personaname: String,
    pub avatarfull: String,
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

#[derive(Deserialize)]
struct AppDetailsEnvelope {
    success: bool,
    data: Option<AppDetailsData>,
}

#[derive(Deserialize)]
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
}

fn build_icon_url(steam_id: u32, hash: &str) -> String {
    if hash.is_empty() {
        return String::new();
    }
    format!("https://steamcdn-a.akamaihd.net/steamcommunity/public/images/apps/{steam_id}/{hash}")
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
        "https://api.steampowered.com/IPlayerService/GetOwnedGames/v0001/?key={api_key}&steamid={steam_id}&include_appinfo=1&l={steam_language}&format=json"
    );
    let resp = client
        .get(url)
        .send()
        .await?
        .json::<OwnedGamesResponse>()
        .await?;
    Ok(resp.response.games.unwrap_or_default())
}

pub async fn fetch_player_achievements(
    api_key: &str,
    steam_id: &str,
    app_id: u32,
    language: &str,
) -> Result<HashMap<String, (bool, u64)>, String> {
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
                map.insert(ach.apiname, (ach.achieved == 1, ach.unlocktime));
            }
        }
    } else if let Some(err) = resp.playerstats.error {
        println!(
            "[DEBUG][fetch_player_achievements] Steam API Error for AppID {}: {}",
            app_id, err
        );
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

    let details_map = client
        .get(format!(
            "https://store.steampowered.com/api/appdetails?appids={steam_id}&l={steam_language}"
        ))
        .send()
        .await?
        .json::<HashMap<String, AppDetailsEnvelope>>()
        .await?;

    let app_name = details_map
        .get(&steam_id.to_string())
        .and_then(|e| if e.success { e.data.as_ref() } else { None })
        .map(|d| d.name.trim().to_string())
        .filter(|n| !n.is_empty());

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
    let details_url = format!(
        "https://store.steampowered.com/api/appdetails?appids={steam_id}&l={steam_language}"
    );

    let lang_req = client.get(lang_url).send();
    let en_req = client.get(en_url).send();
    let details_req = client.get(details_url).send();

    let (lang_res, en_res, details_res) = futures::join!(lang_req, en_req, details_req);

    let mut achievements_lang = Vec::new();
    if let Ok(r) = lang_res {
        if r.status().is_success() {
            if let Ok(text) = r.text().await {
                achievements_lang = parse_achievements_xml(&text, steam_id);
            }
        }
    }

    let mut achievements_en = Vec::new();
    if let Ok(r) = en_res {
        if r.status().is_success() {
            if let Ok(text) = r.text().await {
                achievements_en = parse_achievements_xml(&text, steam_id);
            }
        }
    }

    let details_map = details_res?
        .json::<HashMap<String, AppDetailsEnvelope>>()
        .await?;

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
                completionpercentage: a.player_percent_unlocked.to_string(),
                desc,
                hidden: a.hidden,
            }
        })
        .collect();

    let details = details_map.get(&steam_id.to_string()).and_then(|e| {
        if e.success {
            e.data.as_ref()
        } else {
            None
        }
    });

    let game_icon_url = details
        .map(|d| d.capsule_imagev5.clone())
        .filter(|s| !s.is_empty())
        .unwrap_or_default();

    let header_image_url = details.map(|d| d.header_image.clone()).unwrap_or_default();

    let background_image_url = details
        .map(|d| {
            if !d.background_raw.is_empty() {
                d.background_raw.clone()
            } else {
                d.background.clone()
            }
        })
        .unwrap_or_default();

    let game_name = details.map(|d| d.name.clone()).filter(|s| !s.is_empty());

    Ok(SteamMetadata {
        name: game_name.unwrap_or_else(|| format!("steam_{}", steam_id)),
        game_icon_url,
        header_image_url,
        background_image_url,
        achievements,
    })
}
