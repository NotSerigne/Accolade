// onlinefix: parse le format INI de OnlineFix

use std::path::PathBuf;
use std::collections::HashMap;
use ini::Ini;
use crate::achievements::models::Achievement;
use crate::emulators::EmulatorParser;

pub struct Parser;

impl EmulatorParser for Parser {
    fn parse(&self, path: &PathBuf) -> Vec<Achievement> {
        let achievementfile = Ini::load_from_file(path).unwrap();
        let mut achievements: Vec<Achievement> = Vec::new();
        for (sec, prop) in &achievementfile {
            if let Some(name) = sec {
                let achieved = prop.get("achieved").unwrap_or("false");
                let unlock_time = prop.get("timestamp").unwrap_or("0");
                achievements.push(Achievement {
                    key: name.to_string(),
                    name: name.to_string(),
                    unlocked: achieved == "true",
                    icon: String::new(),
                    unlocked_time: Some(unlock_time.parse::<u64>().unwrap()),
                });
            }
        }
        achievements
    }
}