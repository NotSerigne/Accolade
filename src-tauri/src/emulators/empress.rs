// empress: parse le format JSON de Empress

use std::path::PathBuf;
use std::collections::HashMap;
use serde::{Deserialize};
use crate::achievements::models::Achievement;
use crate::emulators::EmulatorParser;

#[derive(Deserialize)]
pub struct Parser;

impl EmulatorParser for Parser {
    fn parse(&self, path: &PathBuf) -> Vec<Achievement> {
        todo!()
    }
}