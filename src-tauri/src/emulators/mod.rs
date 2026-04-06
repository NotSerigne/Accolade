// emulators: parsers pour chaque format d'émulateur

use std::path::PathBuf;
use crate::achievements::models::Achievement;
use crate::achievements::models::Game;
use crate::achievements::models::Emulator;

pub(crate) mod goldberg;
pub(crate) mod rune;
pub(crate) mod onlinefix;
pub(crate) mod empress;
pub(crate) mod codex;

pub trait EmulatorParser {
    fn parse(&self, path: &PathBuf) -> Vec<Achievement>;
    fn known_locations(&self) -> Vec<PathBuf>;
    fn emulator(&self) -> Emulator;
}

pub fn game_scanner(parsers: Vec<Box<dyn EmulatorParser>>) -> Vec<Game> {
    let mut games: Vec<Game> = Vec::new();
    for parser in parsers {
        games.extend(parser.known_locations().into_iter().map(|path| Game {
            name: String::new(),
            steam_id: 0,
            game_icon: String::new(),
            achievements_total: 0,
            achievements: parser.parse(&path),
            path_buf: path,
            emulator: parser.emulator(),
        }));
    }
    games
}