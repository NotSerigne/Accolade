// emulators: parsers pour chaque format d'émulateur

use std::path::PathBuf;
use crate::achievements::models::Achievement;

pub(crate) mod goldberg;
pub(crate) mod rune;
pub(crate) mod onlinefix;
pub(crate) mod empress;
pub(crate) mod codex;

pub trait EmulatorParser {
    fn parse(&self, path: &PathBuf) -> Vec<Achievement>;
}