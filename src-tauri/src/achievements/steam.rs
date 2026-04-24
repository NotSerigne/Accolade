use crate::achievements::models::Achievement;
use serde::Deserialize;
use std::collections::HashMap;

#[derive(Clone)]
pub struct SteamMetadata {
    pub name: String,
    pub game_icon_url: String,
    pub header_image_url: String,
    pub background_image_url: String,
    pub achievements: Vec<Achievement>,
}

#[derive(Deserialize)]
struct SteamResponse {
    game: SteamGame,
}

#[derive(Deserialize)]
struct SteamGame {
    #[serde(rename = "availableGameStats")]
    available_game_stats: Option<SteamGameStats>,
    #[serde(rename = "gameName")]
    game_name: Option<String>,
}

#[derive(Deserialize)]
struct SteamGameStats {
    achievements: Option<Vec<SteamAchievement>>,
}

#[derive(Deserialize)]
struct SteamAchievement {
    name: String,
    #[serde(rename = "displayName")]
    display_name: Option<String>,
    #[serde(default)]
    description: String,
    #[serde(default)]
    icon: String,
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

pub async fn fetch_steam_metadata(steam_id: u32, api_key: &str) -> Result<SteamMetadata, reqwest::Error> {
    let client = reqwest::Client::new();

    let schema_url = format!(
        "https://api.steampowered.com/ISteamUserStats/GetSchemaForGame/v2/?appid={steam_id}&key={api_key}&l=french"
    );
    let schema_response = client.get(schema_url).send().await?.json::<SteamResponse>().await?;

    let game_name = schema_response.game.game_name.clone();

    let achievements = schema_response
        .game
        .available_game_stats
        .and_then(|stats| stats.achievements)
        .unwrap_or_default()
        .into_iter()
        .map(|a| Achievement {
            key: a.name,
            name: a.display_name.unwrap_or_default(),
            unlocked: false,
            icon: a.icon,
            unlocked_time: None,
            rarity: String::new(),
            completionpercentage: String::new(),
            desc: a.description,
        })
        .collect::<Vec<_>>();

    let details_url = format!("https://store.steampowered.com/api/appdetails?appids={steam_id}&l=french");
    let details_map = client
        .get(details_url)
        .send()
        .await?
        .json::<HashMap<String, AppDetailsEnvelope>>()
        .await?;

    let details = details_map
        .get(&steam_id.to_string())
        .and_then(|entry| if entry.success { entry.data.as_ref() } else { None });

    let game_icon_url = details
        .map(|d| d.capsule_imagev5.clone())
        .filter(|s| !s.is_empty())
        .unwrap_or_default();

    let header_image_url = details
        .map(|d| d.header_image.clone())
        .unwrap_or_default();

    let background_image_url = details
        .map(|d| {
            if !d.background_raw.is_empty() {
                d.background_raw.clone()
            } else {
                d.background.clone()
            }
        })
        .unwrap_or_default();

    Ok(SteamMetadata {
        name: details
            .map(|d| d.name.clone())
            .or(game_name)
            .unwrap_or_default(),
        game_icon_url,
        header_image_url,
        background_image_url,
        achievements,
    })
}