// Prevents additional console window on Windows in release, DO NOT REMOVE!!
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use app_lib::commands::get_achievements;
use tauri::Builder;
use tauri::generate_handler;
use tauri::generate_context;
fn main() {
}
