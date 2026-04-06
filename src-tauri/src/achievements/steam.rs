use crate::achievements::models::Achievement;
use serde::Deserialize;

#[derive(Deserialize)]
struct SteamResponse {
    game: SteamGame,  // "game" dans le JSON → type SteamGame
}

#[derive(Deserialize)]
struct SteamGame {
    #[serde(rename = "availableGameStats")]
    available_game_stats: SteamGameStats,
}

#[derive(Deserialize)]
struct SteamGameStats {
    achievements: Vec<SteamAchievement>,
}

#[derive(Deserialize)]
struct SteamAchievement {
    name: String,
    #[serde(rename = "displayName")]
    display_name: String,
    icon: String,
}

pub async fn fetch_game_schema(steam_id: u32, api_key: &str) -> Result<Vec<Achievement>, reqwest::Error> {
    let url = format!("https://api.steampowered.com/ISteamUserStats/GetSchemaForGame/v2/?appid={steam_id}&key={api_key}");
    let response = reqwest::get(&url)
        .await?
        .json::<SteamResponse>()
        .await?;

    // convertir Vec<SteamAchievement> en Vec<Achievement>
    Ok(response.game.available_game_stats.achievements
        .into_iter()
        .map(|a| Achievement {
            key: a.name,
            name: a.display_name,
            unlocked: false,
            icon: a.icon,
            unlocked_time: None,
        })
        .collect())
}