use crate::achievements::models::{Achievement, Game, SourceType};
use crate::achievements::providers::AchievementProvider;
use async_trait::async_trait;
use futures::stream::{self, StreamExt};
use reqwest::Client;
use serde::Deserialize;
use std::collections::HashMap;

pub struct RetroAchievementsProvider {
    client: Client,
    username: String,
    api_key: String,
}

impl RetroAchievementsProvider {
    pub fn new(username: String, api_key: String) -> Self {
        Self {
            client: Client::new(),
            username,
            api_key,
        }
    }
}

/// Parses a `"YYYY-MM-DD HH:MM:SS"` UTC timestamp returned by the RetroAchievements Web API
/// into a Unix epoch timestamp in seconds.
pub fn parse_ra_datetime(raw: &str) -> Option<u64> {
    let s = raw.trim();
    let (date_part, time_part) = s.split_once(' ')?;
    let mut d_it = date_part.split('-');
    let year: i64 = d_it.next()?.parse().ok()?;
    let month: u32 = d_it.next()?.parse().ok()?;
    let day: u32 = d_it.next()?.parse().ok()?;
    if !(1970..=2200).contains(&year) || !(1..=12).contains(&month) || !(1..=31).contains(&day) {
        return None;
    }

    let mut t_it = time_part.split(':');
    let hour: u64 = t_it.next()?.parse().ok()?;
    let min: u64 = t_it.next()?.parse().ok()?;
    let sec: u64 = t_it.next().unwrap_or("0").parse().ok()?;
    if hour > 23 || min > 59 || sec > 59 {
        return None;
    }

    // Howard Hinnant's civil-to-days algorithm
    let y = year - if month <= 2 { 1 } else { 0 };
    let era = if y >= 0 { y } else { y - 399 } / 400;
    let yoe = (y - era * 400) as u64;
    let m = month as u64;
    let doy = (153 * (if m > 2 { m - 3 } else { m + 9 }) + 2) / 5 + (day as u64) - 1;
    let doe = yoe * 365 + yoe / 4 - yoe / 100 + doy;
    let days = era * 146097 + (doe as i64) - 719468;
    if days < 0 {
        return None;
    }
    Some((days as u64) * 86_400 + hour * 3600 + min * 60 + sec)
}

fn normalize_ra_media_url(path_or_badge: &str) -> String {
    let trimmed = path_or_badge.trim();
    if trimmed.is_empty() {
        return String::new();
    }
    if trimmed.starts_with("http://") || trimmed.starts_with("https://") {
        return trimmed.to_string();
    }
    if trimmed.starts_with('/') {
        return format!("https://media.retroachievements.org{}", trimmed);
    }
    format!("https://media.retroachievements.org/Badge/{}.png", trimmed)
}

#[async_trait]
impl AchievementProvider for RetroAchievementsProvider {
    async fn fetch_games(&self) -> Result<Vec<Game>, String> {
        if self.username.trim().is_empty() || self.api_key.trim().is_empty() {
            return Ok(vec![]);
        }

        let url = format!(
            "https://retroachievements.org/API/API_GetUserRecentlyPlayedGames.php?z={}&y={}&u={}&c=50",
            self.username.trim(),
            self.api_key.trim(),
            self.username.trim()
        );

        let response = self
            .client
            .get(&url)
            .send()
            .await
            .map_err(|e| format!("Erreur réseau RA: {}", e))?;

        let ra_games = response
            .json::<Vec<RARecentGame>>()
            .await
            .map_err(|e| format!("Erreur désérialisation RA: {}", e))?;

        let base_games: Vec<RARecentGame> =
            ra_games.into_iter().filter(|g| g.game_id > 0).collect();

        // Fetch full achievement lists concurrently (max 4 at a time to respect RA rate limits)
        let fetched = stream::iter(base_games.into_iter().map(|ra_g| async move {
            let (achievements, header_opt, box_opt, console_opt) = self
                .get_game_details_and_progress(ra_g.game_id)
                .await
                .unwrap_or_else(|_| (Vec::new(), None, None, None));

            let total = if !achievements.is_empty() {
                achievements.len() as u32
            } else {
                ra_g.num_possible_achievements.max(ra_g.num_achievements)
            };

            let icon_url = normalize_ra_media_url(&ra_g.image_icon);
            let title_img = header_opt
                .filter(|s| !s.is_empty())
                .unwrap_or_else(|| normalize_ra_media_url(&ra_g.image_title));
            let box_img = box_opt
                .filter(|s| !s.is_empty())
                .unwrap_or_else(|| normalize_ra_media_url(&ra_g.image_box_art));
            let ingame_img = normalize_ra_media_url(&ra_g.image_ingame);

            let header_image_url = if !title_img.is_empty() {
                title_img
            } else if !box_img.is_empty() {
                box_img.clone()
            } else {
                icon_url.clone()
            };

            let background_image_url = if !ingame_img.is_empty() {
                ingame_img
            } else {
                header_image_url.clone()
            };

            let mut genres = Vec::new();
            let console = console_opt.unwrap_or(ra_g.console_name);
            if !console.trim().is_empty() {
                genres.push(console);
            }

            Game {
                name: ra_g.title,
                id: format!("ra_{}", ra_g.game_id),
                steam_id: None,
                game_icon: icon_url.clone(),
                steamgrid_icon_url: icon_url,
                header_image_url,
                background_image_url,
                achievements_total: total,
                achievements,
                path_buf: None,
                source: SourceType::RetroAchievements,
                genres,
                is_favorite: false,
                tags: vec!["RetroAchievements".to_string()],
                playtime_seconds: 0,
            }
        }))
        .buffer_unordered(4)
        .collect::<Vec<Game>>()
        .await;

        Ok(fetched)
    }

    fn source_type(&self) -> SourceType {
        SourceType::RetroAchievements
    }
}

impl RetroAchievementsProvider {
    pub async fn get_game_progress(&self, game_id: u32) -> Result<Vec<Achievement>, String> {
        let (achs, _, _, _) = self.get_game_details_and_progress(game_id).await?;
        Ok(achs)
    }

    async fn get_game_details_and_progress(
        &self,
        game_id: u32,
    ) -> Result<
        (
            Vec<Achievement>,
            Option<String>,
            Option<String>,
            Option<String>,
        ),
        String,
    > {
        let url = format!(
            "https://retroachievements.org/API/API_GetGameInfoAndUserProgress.php?z={}&y={}&g={}&u={}",
            self.username.trim(),
            self.api_key.trim(),
            game_id,
            self.username.trim()
        );

        let response = self
            .client
            .get(&url)
            .send()
            .await
            .map_err(|e| e.to_string())?;

        let progress = response
            .json::<RAGameProgress>()
            .await
            .map_err(|e| e.to_string())?;

        let distinct_players = progress
            .num_distinct_players_casual
            .and_then(|v| v.as_u64().or_else(|| v.as_str()?.parse::<u64>().ok()))
            .unwrap_or(0)
            .max(1);

        let mut list: Vec<(u32, Achievement)> = Vec::new();
        for (_key, ra_ach) in progress.achievements {
            let date_str = ra_ach
                .date_earned_hardcore
                .as_deref()
                .or(ra_ach.date_earned.as_deref());
            let unlocked = date_str.is_some();
            let unlocked_time = date_str.and_then(parse_ra_datetime);

            let awarded = ra_ach
                .num_awarded
                .and_then(|v| v.as_u64().or_else(|| v.as_str()?.parse::<u64>().ok()))
                .unwrap_or(0);
            let pct = ((awarded as f64 / distinct_players as f64) * 100.0).clamp(0.1, 100.0);

            let id_str = match &ra_ach.id {
                serde_json::Value::Number(n) => n.to_string(),
                serde_json::Value::String(s) => s.clone(),
                _ => String::new(),
            };
            let order = ra_ach.display_order.unwrap_or(u32::MAX);

            list.push((
                order,
                Achievement {
                    key: id_str,
                    name: ra_ach.title,
                    unlocked,
                    icon: format!(
                        "https://media.retroachievements.org/Badge/{}.png",
                        ra_ach.badge_name
                    ),
                    icon_gray: format!(
                        "https://media.retroachievements.org/Badge/{}_lock.png",
                        ra_ach.badge_name
                    ),
                    unlocked_time,
                    rarity: String::new(),
                    completionpercentage: format!("{:.1}", pct),
                    desc: ra_ach.description,
                    hidden: false,
                    ..Default::default()
                },
            ));
        }

        list.sort_by_key(|(order, _)| *order);
        let achievements = list.into_iter().map(|(_, a)| a).collect();

        Ok((
            achievements,
            Some(normalize_ra_media_url(&progress.image_title)),
            Some(normalize_ra_media_url(&progress.image_box_art)),
            progress.console_name,
        ))
    }
}

#[derive(Deserialize)]
pub struct RARecentGame {
    #[serde(rename = "GameID")]
    pub game_id: u32,
    #[serde(rename = "Title")]
    pub title: String,
    #[serde(rename = "ConsoleName", default)]
    pub console_name: String,
    #[serde(rename = "ImageIcon", default)]
    pub image_icon: String,
    #[serde(rename = "ImageTitle", default)]
    pub image_title: String,
    #[serde(rename = "ImageIngame", default)]
    pub image_ingame: String,
    #[serde(rename = "ImageBoxArt", default)]
    pub image_box_art: String,
    #[serde(rename = "NumPossibleAchievements", default)]
    pub num_possible_achievements: u32,
    #[serde(rename = "NumAchievements", default)]
    pub num_achievements: u32,
}

#[derive(Deserialize)]
pub struct RAGameProgress {
    #[serde(rename = "Title", default)]
    pub title: String,
    #[serde(rename = "ID", default)]
    pub id: u32,
    #[serde(rename = "ConsoleName", default)]
    pub console_name: Option<String>,
    #[serde(rename = "ImageIcon", default)]
    pub image_icon: String,
    #[serde(rename = "ImageTitle", default)]
    pub image_title: String,
    #[serde(rename = "ImageBoxArt", default)]
    pub image_box_art: String,
    #[serde(rename = "NumDistinctPlayersCasual", default)]
    pub num_distinct_players_casual: Option<serde_json::Value>,
    #[serde(rename = "Achievements", default)]
    pub achievements: HashMap<String, RAAchievement>,
}

#[derive(Deserialize)]
pub struct RAAchievement {
    #[serde(rename = "ID")]
    pub id: serde_json::Value,
    #[serde(rename = "Title", default)]
    pub title: String,
    #[serde(rename = "Description", default)]
    pub description: String,
    #[serde(rename = "DateEarned", default)]
    pub date_earned: Option<String>,
    #[serde(rename = "DateEarnedHardcore", default)]
    pub date_earned_hardcore: Option<String>,
    #[serde(rename = "BadgeName", default)]
    pub badge_name: String,
    #[serde(rename = "NumAwarded", default)]
    pub num_awarded: Option<serde_json::Value>,
    #[serde(rename = "DisplayOrder", default)]
    pub display_order: Option<u32>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_ra_utc_datetime_to_unix_seconds() {
        assert_eq!(parse_ra_datetime("1970-01-01 00:00:00"), Some(0));
        assert_eq!(parse_ra_datetime("2024-03-10 15:30:45"), Some(1710084645));
        assert_eq!(parse_ra_datetime("invalid"), None);
    }
}
