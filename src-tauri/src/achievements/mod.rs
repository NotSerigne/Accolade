// achievements: logique métier, détection des unlocks
use crate::achievements::models::{Achievement, Game};
use crate::emulators::codex;
use crate::emulators::empress;
use crate::emulators::goldberg;
use crate::emulators::onlinefix;
use crate::emulators::rune;
use crate::emulators::EmulatorParser;

pub mod appinfo;
pub mod models;
pub mod steam;

pub fn match_emulator(games: Game) -> Vec<Achievement> {
    match games.emulator {
        models::Emulator::Goldberg => goldberg::Parser.parse(&games.path_buf),
        models::Emulator::Empress => empress::Parser.parse(&games.path_buf),
        models::Emulator::Codex => codex::Parser.parse(&games.path_buf),
        models::Emulator::OnlineFix => onlinefix::Parser.parse(&games.path_buf),
        models::Emulator::Rune => rune::Parser.parse(&games.path_buf),
        models::Emulator::Steam => Vec::new(), // Pas encore de parsing local pour Steam officiel
    }
}
