// emulators: parsers pour chaque format d'émulateur

use std::path::PathBuf;
use crate::achievements::models::Achievement;

mod goldberg;
mod rune;
mod onlinefix;
mod empress;
mod codex;

pub trait EmulatorParser {
    fn parse(&self, path: &PathBuf) -> Vec<Achievement>;
}