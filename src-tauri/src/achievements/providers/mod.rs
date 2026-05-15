use crate::achievements::models::{Game, SourceType};
use async_trait::async_trait;

pub mod custom;
pub mod emulators;
pub mod retroachievements;

#[async_trait]
pub trait AchievementProvider {
    async fn fetch_games(&self) -> Result<Vec<Game>, String>;
    fn source_type(&self) -> SourceType;
}
