use crate::achievements::models::{Game, SourceType};
use crate::achievements::providers::AchievementProvider;
use async_trait::async_trait;

pub struct CustomProvider;

#[async_trait]
impl AchievementProvider for CustomProvider {
    async fn fetch_games(&self) -> Result<Vec<Game>, String> {
        // TODO: Lire depuis le tauri-plugin-store ou un fichier JSON local
        Ok(vec![])
    }

    fn source_type(&self) -> SourceType {
        SourceType::Custom
    }
}
