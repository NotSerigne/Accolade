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

// Struct intermédiaire pour parser un <message> de l'API XML
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
    format!(
        "https://steamcdn-a.akamaihd.net/steamcommunity/public/images/apps/{steam_id}/{hash}"
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

        // Détecte le début d'un achievement
        if trimmed.contains("<message>") {
            current = Some(RawAchievement::default());
        }

        // Parse les champs
        if let Some(ref mut ach) = current {
            if let Some(value) = extract_xml_value(trimmed, "internal_name") {
                ach.internal_name = decode_html_entities(&value);
            }
            if let Some(value) = extract_xml_value(trimmed, "localized_name") {
                ach.localized_name = decode_html_entities(&value);
            }
            if let Some(value) = extract_xml_value(trimmed, "localized_desc") {
                ach.localized_desc = decode_html_entities(&value);
            }
            if let Some(hash) = extract_xml_value(trimmed, "icon") {
                ach.icon = build_icon_url(steam_id, &hash);
            }
            if let Some(hash) = extract_xml_value(trimmed, "icon_gray") {
                ach.icon_gray = build_icon_url(steam_id, &hash);
            }
            if let Some(value) = extract_xml_value(trimmed, "hidden") {
                ach.hidden = value == "true";
            }
            if let Some(value) = extract_xml_value(trimmed, "player_percent_unlocked") {
                ach.player_percent_unlocked = value.parse().unwrap_or(0.0);
            }
        }

        // Détecte la fin d'un achievement
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

pub async fn fetch_app_name_by_appid(steam_id: u32) -> Result<Option<String>, reqwest::Error> {
    let client = reqwest::Client::new();
    let details_url = format!(
        "https://store.steampowered.com/api/appdetails?appids={steam_id}&l=french"
    );

    let details_map = client
        .get(details_url)
        .send()
        .await?
        .json::<HashMap<String, AppDetailsEnvelope>>()
        .await?;

    let app_name = details_map
        .get(&steam_id.to_string())
        .and_then(|entry| if entry.success { entry.data.as_ref() } else { None })
        .map(|d| d.name.trim().to_string())
        .filter(|name| !name.is_empty());

    Ok(app_name)
}

pub async fn fetch_steam_metadata(steam_id: u32, api_key: &str) -> Result<SteamMetadata, reqwest::Error> {
    let client = reqwest::Client::new();

    // Appel FR
    let url_fr = format!(
        "https://api.steampowered.com/IPlayerService/GetGameAchievements/v1/?key={api_key}&appid={steam_id}&language=french&format=xml"
    );
    let xml_fr = client.get(&url_fr).send().await?.text().await?;
    let achievements_fr = parse_achievements_xml(&xml_fr, steam_id);

    // Appel EN (fallback)
    let url_en = format!(
        "https://api.steampowered.com/IPlayerService/GetGameAchievements/v1/?key={api_key}&appid={steam_id}&language=english&format=xml"
    );
    let xml_en = client.get(&url_en).send().await?.text().await?;
    let achievements_en = parse_achievements_xml(&xml_en, steam_id);

    // Map EN pour fallback: key -> RawAchievement
    let en_map: HashMap<String, &RawAchievement> = achievements_en
        .iter()
        .map(|a| (a.internal_name.to_lowercase(), a))
        .collect();

    let mut achievements: Vec<Achievement> = achievements_fr
        .iter()
        .map(|a| {
            let fallback = en_map.get(&a.internal_name.to_lowercase());

            let name = if a.localized_name.trim().is_empty() {
                fallback.map(|f| f.localized_name.clone()).unwrap_or_default()
            } else {
                a.localized_name.clone()
            };

            let desc = if a.localized_desc.trim().is_empty() {
                fallback.map(|f| f.localized_desc.clone()).unwrap_or_default()
            } else {
                a.localized_desc.clone()
            };

            let icon = if a.icon.is_empty() {
                fallback.map(|f| f.icon.clone()).unwrap_or_default()
            } else {
                a.icon.clone()
            };

            let icon_gray = if a.icon_gray.is_empty() {
                fallback.map(|f| f.icon_gray.clone()).unwrap_or_default()
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

    // Si FR vide, fallback total sur EN
    if achievements.is_empty() {
        achievements = achievements_en
            .into_iter()
            .map(|a| Achievement {
                key: a.internal_name,
                name: a.localized_name,
                unlocked: false,
                icon: a.icon,
                icon_gray: a.icon_gray,
                unlocked_time: None,
                rarity: String::new(),
                completionpercentage: a.player_percent_unlocked.to_string(),
                desc: a.localized_desc,
                hidden: a.hidden,
            })
            .collect();
    }

    // Infos du jeu via appdetails
    let details_url = format!(
        "https://store.steampowered.com/api/appdetails?appids={steam_id}&l=french"
    );
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

    let game_name = details
        .map(|d| d.name.clone())
        .filter(|s| !s.is_empty());

    Ok(SteamMetadata {
        name: game_name.unwrap_or_default(),
        game_icon_url,
        header_image_url,
        background_image_url,
        achievements,
    })
}