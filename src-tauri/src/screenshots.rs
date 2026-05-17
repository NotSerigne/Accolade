use screenshots::Screen;
use serde::Serialize;
use std::fs;
use std::io::Cursor;
use std::path::PathBuf;
use std::time::{SystemTime, UNIX_EPOCH};
use tauri::{AppHandle, Manager, Runtime};
// Use the image crate re-exported by screenshots to avoid version mismatch
use screenshots::image::ImageFormat;

#[derive(Serialize, Clone)]
pub struct ScreenshotInfo {
    pub filename: String,
    pub path: String,
    pub timestamp: u64,
    pub game_id: Option<String>,
    pub achievement_key: Option<String>,
}

pub fn get_screenshots_dir<R: Runtime>(app_handle: &AppHandle<R>) -> PathBuf {
    let mut path = app_handle
        .path()
        .app_data_dir()
        .unwrap_or_else(|_| PathBuf::from("."));
    path.push("screenshots");
    if !path.exists() {
        let _ = fs::create_dir_all(&path);
    }
    path
}

pub fn capture_screenshot<R: Runtime>(
    app_handle: &AppHandle<R>,
    game_id: Option<&str>,
    achievement_key: Option<&str>,
) -> Result<String, String> {
    println!("[DEBUG] Starting screenshot capture...");
    let screens = Screen::all().map_err(|e| {
        let err = format!("Failed to get screens: {}", e);
        println!("[ERROR] {}", err);
        err
    })?;

    println!("[DEBUG] Found {} screens", screens.len());

    let screen = screens
        .iter()
        .find(|s| s.display_info.is_primary)
        .or(screens.first());

    if let Some(screen) = screen {
        println!("[DEBUG] Capturing screen: {:?}", screen.display_info.id);
        let image = screen.capture().map_err(|e| {
            let err = format!("Capture failed: {}", e);
            println!("[ERROR] {}", err);
            err
        })?;

        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs();

        let dir = get_screenshots_dir(app_handle);
        println!("[DEBUG] Screenshots directory: {:?}", dir);

        let filename = match (game_id, achievement_key) {
            (Some(gid), Some(akey)) => format!("{}--{}--{}.png", gid, akey, now),
            (Some(gid), None) => format!("{}--{}.png", gid, now),
            _ => format!("manual--{}.png", now),
        };

        let mut path = dir.clone();
        path.push(&filename);
        println!("[DEBUG] Saving to: {:?}", path);

        let mut buffer = Vec::new();
        image
            .write_to(&mut Cursor::new(&mut buffer), ImageFormat::Png)
            .map_err(|e| {
                let err = format!("Encoding failed: {}", e);
                println!("[ERROR] {}", err);
                err
            })?;

        fs::write(&path, buffer).map_err(|e| {
            let err = format!("FS write failed: {}", e);
            println!("[ERROR] {}", err);
            err
        })?;

        println!("[DEBUG] Screenshot saved successfully: {}", filename);
        Ok(filename)
    } else {
        println!("[ERROR] No screen found");
        Err("No screen found".to_string())
    }
}

pub fn list_screenshots<R: Runtime>(app_handle: &AppHandle<R>) -> Vec<ScreenshotInfo> {
    let dir = get_screenshots_dir(app_handle);
    let mut list = Vec::new();

    if let Ok(entries) = fs::read_dir(dir) {
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_file() && path.extension().and_then(|s| s.to_str()) == Some("png") {
                if let Some(filename) = path.file_name().and_then(|s| s.to_str()) {
                    let name = filename.trim_end_matches(".png");

                    let (game_id, achievement_key, timestamp) = if name.contains("--") {
                        let parts: Vec<&str> = name.split("--").collect();
                        if parts.len() == 3 {
                            (
                                Some(parts[0].to_string()),
                                Some(parts[1].to_string()),
                                parts[2].parse::<u64>().unwrap_or(0),
                            )
                        } else if parts.len() == 2 {
                            if parts[0] == "manual" {
                                (None, None, parts[1].parse::<u64>().unwrap_or(0))
                            } else {
                                (
                                    Some(parts[0].to_string()),
                                    None,
                                    parts[1].parse::<u64>().unwrap_or(0),
                                )
                            }
                        } else {
                            (None, None, 0)
                        }
                    } else {
                        // Legacy single underscore parsing
                        if name.starts_with("manual_") {
                            let ts_str = &name["manual_".len()..];
                            (None, None, ts_str.parse::<u64>().unwrap_or(0))
                        } else if let Some(last_idx) = name.rfind('_') {
                            let ts_str = &name[last_idx + 1..];
                            let timestamp = ts_str.parse::<u64>().unwrap_or(0);
                            let rest = &name[..last_idx];
                            if let Some(first_idx) = rest.find('_') {
                                let gid = &rest[..first_idx];
                                let akey = &rest[first_idx + 1..];
                                (Some(gid.to_string()), Some(akey.to_string()), timestamp)
                            } else {
                                (Some(rest.to_string()), None, timestamp)
                            }
                        } else {
                            (None, None, 0)
                        }
                    };

                    // Normalize path and manually strip UNC prefix (\\?\) which breaks some webview loads
                    let path_str = path.to_string_lossy().to_string();
                    let normalized_path = if path_str.starts_with(r"\\?\") {
                        path_str.trim_start_matches(r"\\?\").to_string()
                    } else {
                        path_str
                    };

                    list.push(ScreenshotInfo {
                        filename: filename.to_string(),
                        path: normalized_path,
                        timestamp,
                        game_id,
                        achievement_key,
                    });
                }
            }
        }
    }

    // Sort by timestamp descending
    list.sort_by(|a, b| b.timestamp.cmp(&a.timestamp));
    list
}
