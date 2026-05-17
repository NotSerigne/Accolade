// src-tauri/src/user_data.rs
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};
use std::fs;
use std::path::PathBuf;
use tauri::{AppHandle, Manager};

#[derive(Serialize, Deserialize, Default, Clone)]
pub struct UserData {
    pub favorites: HashSet<String>,
    pub tags: HashMap<String, Vec<String>>,
}

fn get_user_data_path(app_handle: &AppHandle) -> PathBuf {
    let mut path = app_handle
        .path()
        .app_data_dir()
        .unwrap_or_else(|_| PathBuf::from("."));
    if !path.exists() {
        let _ = fs::create_dir_all(&path);
    }
    path.push("user_data.json");
    path
}

pub fn load_user_data(app_handle: &AppHandle) -> UserData {
    let path = get_user_data_path(app_handle);
    if let Ok(content) = fs::read_to_string(path) {
        serde_json::from_str(&content).unwrap_or_default()
    } else {
        UserData::default()
    }
}

pub fn save_user_data(app_handle: &AppHandle, data: &UserData) -> Result<(), String> {
    let path = get_user_data_path(app_handle);
    let content = serde_json::to_string_pretty(data).map_err(|e| e.to_string())?;
    fs::write(path, content).map_err(|e| e.to_string())?;
    Ok(())
}
