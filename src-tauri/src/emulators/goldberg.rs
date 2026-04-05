use std::path::PathBuf;
// goldberg: parse le format JSON de Goldberg
use serde::{Deserialize};
use crate::achievements::models::Achievement;
use crate::emulators::EmulatorParser;

#[derive(Deserialize)]
pub struct Parser {
    pub earned : bool,
    pub earned_time : Option<u64>,
}

impl EmulatorParser for Parser {
    fn parse(&self, path: &PathBuf) -> Vec<Achievement> {
        todo!()
    }
}