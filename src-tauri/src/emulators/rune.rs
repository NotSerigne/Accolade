// rune: parse le format INI de Rune

use std::path::PathBuf;
use std::collections::HashMap;
use ini::Ini;
use serde::{Deserialize};
use crate::achievements::models::Achievement;
use crate::emulators::EmulatorParser;

pub struct Parser;

impl EmulatorParser for Parser {
    fn parse(&self, path: &PathBuf) -> Vec<Achievement> {
        let achievementfile = Ini::load_from_file(path).unwrap();
        let section = achievementfile.section(Some("SteamAchievements")).unwrap();
        let mut index: Vec<String>  = Vec::new();
        for (_key, value) in section.iter() {
            index.push(value.to_string());
        }
        todo!()
    }
}
