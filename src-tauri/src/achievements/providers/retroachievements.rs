use crate::achievements::models::{Achievement, Game, SourceType};
use crate::achievements::providers::AchievementProvider;
use async_trait::async_trait;
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

#[async_trait]
impl AchievementProvider for RetroAchievementsProvider {
    async fn fetch_games(&self) -> Result<Vec<Game>, String> {
        if self.username.is_empty() || self.api_key.is_empty() {
            return Ok(vec![]);
        }

        let url = format!(
            "https://retroachievements.org/API/API_GetUserRecentlyPlayedGames.php?z={}&y={}&u={}",
            self.username, self.api_key, self.username
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

        let mut games = Vec::new();
        for ra_g in ra_games {
            games.push(Game {
                name: ra_g.title.clone(),
                id: format!("ra_{}", ra_g.game_id),
                steam_id: None,
                game_icon: format!(
                    "https://media.retroachievements.org/Badge/{}.png",
                    ra_g.image_icon
                ),
                steamgrid_icon_url: String::new(),
                header_image_url: format!(
                    "https://media.retroachievements.org/Images/UserPic/{}.png",
                    ra_g.image_icon
                ), // Approximation
                background_image_url: String::new(),
                achievements_total: ra_g.num_achievements,
                achievements: Vec::new(), // Chargés à la demande ou via un autre appel
                path_buf: None,
                source: SourceType::RetroAchievements,
                genres: Vec::new(),
                is_favorite: false,
                tags: Vec::new(),
            });
        }

        Ok(games)
    }

    fn source_type(&self) -> SourceType {
        SourceType::RetroAchievements
    }
}

impl RetroAchievementsProvider {
    pub async fn get_game_progress(&self, game_id: u32) -> Result<Vec<Achievement>, String> {
        let url = format!(
            "https://retroachievements.org/API/API_GetGameInfoAndUserProgress.php?z={}&y={}&g={}&u={}",
            self.username, self.api_key, game_id, self.username
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

        let mut achievements = Vec::new();
        for (_key, ra_ach) in progress.achievements {
            let unlocked = ra_ach.date_earned.is_some();
            let unlocked_time = ra_ach.date_earned.as_ref().and_then(|_d| {
                // RA date format is "YYYY-MM-DD HH:MM:SS"
                // For simplicity, we can try to parse it or just return current time if unlocked
                Some(0) // TODO: Better date parsing
            });

            achievements.push(Achievement {
                key: ra_ach.id,
                name: ra_ach.title,
                unlocked,
                icon: format!(
                    "https://media.retroachievements.org/Badge/{}.png",
                    ra_ach.badge_name
                ),
                icon_gray: format!(
                    "https://media.retroachievements.org/Badge/{}_lock.png",
                    ra_ach.badge_name
                ), // Hypothétique
                unlocked_time,
                rarity: String::new(),
                completionpercentage: String::new(),
                desc: ra_ach.description,
                hidden: ra_ach.hidden == Some(1),
            });
        }

        Ok(achievements)
    }
}

#[derive(Deserialize)]
pub struct RARecentGame {
    #[serde(rename = "GameID")]
    pub game_id: u32,
    #[serde(rename = "Title")]
    pub title: String,
    #[serde(rename = "ImageIcon")]
    pub image_icon: String,
    #[serde(rename = "NumAchievements")]
    pub num_achievements: u32,
}

#[derive(Deserialize)]
pub struct RAGameProgress {
    #[serde(rename = "Title")]
    pub title: String,
    #[serde(rename = "ID")]
    pub id: u32,
    #[serde(rename = "ImageIcon")]
    pub image_icon: String,
    #[serde(rename = "Achievements")]
    pub achievements: HashMap<String, RAAchievement>,
}

#[derive(Deserialize)]
pub struct RAAchievement {
    #[serde(rename = "ID")]
    pub id: String,
    #[serde(rename = "Title")]
    pub title: String,
    #[serde(rename = "Description")]
    pub description: String,
    #[serde(rename = "DateEarned")]
    pub date_earned: Option<String>,
    #[serde(rename = "BadgeName")]
    pub badge_name: String,
    #[serde(default)]
    pub hidden: Option<u32>,
}
