use crate::achievements::models::{Emulator, Game, SourceType};
use crate::achievements::providers::AchievementProvider;
use crate::emulators::{game_scanner, EmulatorParser};
use async_trait::async_trait;

pub struct LocalEmulatorProvider;

#[async_trait]
impl AchievementProvider for LocalEmulatorProvider {
    async fn fetch_games(&self) -> Result<Vec<Game>, String> {
        let parsers: Vec<Box<dyn EmulatorParser>> = vec![
            Box::new(crate::emulators::goldberg::Parser),
            Box::new(crate::emulators::codex::Parser),
            Box::new(crate::emulators::empress::Parser),
            Box::new(crate::emulators::onlinefix::Parser),
            Box::new(crate::emulators::rune::Parser),
        ];

        Ok(game_scanner(parsers))
    }

    fn source_type(&self) -> SourceType {
        SourceType::Emulator(Emulator::Goldberg)
    }
}
