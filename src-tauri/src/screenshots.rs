use serde::Serialize;
use std::fs;
use std::io::BufWriter;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};
use tauri::{AppHandle, Manager, Runtime};

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

fn write_png(path: &Path, width: u32, height: u32, rgba: &[u8]) -> Result<(), String> {
    use image::codecs::png::{CompressionType, FilterType, PngEncoder};
    use image::{ExtendedColorType, ImageEncoder};

    let file = fs::File::create(path).map_err(|e| format!("FS write failed: {}", e))?;
    // Fast compression keeps the capture from stalling the game
    let encoder = PngEncoder::new_with_quality(
        BufWriter::new(file),
        CompressionType::Fast,
        FilterType::Adaptive,
    );
    encoder
        .write_image(rgba, width, height, ExtendedColorType::Rgba8)
        .map_err(|e| format!("Encoding failed: {}", e))
}

/// Captures the primary monitor through Windows Graphics Capture.
/// HDR / Advanced Color displays are captured in FP16 and tone-mapped to SDR.
fn capture_with_wgc(path: &Path) -> Result<(), String> {
    let info = crate::capture::primary_display_color_info();
    log::debug!(
        "[screenshot] Display: advanced_color={} hdr={} sdr_white={:.2}",
        info.advanced_color,
        info.hdr,
        info.sdr_white_scale
    );
    let raw = crate::capture::capture_primary_monitor(&info)?;
    let rgba = crate::capture::raw_to_rgba8(&raw, &info);
    write_png(path, raw.width, raw.height, &rgba)
}

/// Legacy GDI capture, used only when WGC is unavailable.
fn capture_with_gdi(path: &Path) -> Result<(), String> {
    let screens =
        screenshots::Screen::all().map_err(|e| format!("Failed to get screens: {}", e))?;
    let screen = screens
        .iter()
        .find(|s| s.display_info.is_primary)
        .or(screens.first())
        .ok_or_else(|| "No screen found".to_string())?;
    let image = screen
        .capture()
        .map_err(|e| format!("Capture failed: {}", e))?;
    write_png(path, image.width(), image.height(), image.as_raw())
}

pub fn capture_screenshot<R: Runtime>(
    app_handle: &AppHandle<R>,
    game_id: Option<&str>,
    achievement_key: Option<&str>,
) -> Result<String, String> {
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0);

    let filename = match (game_id, achievement_key) {
        (Some(gid), Some(akey)) => format!("{}--{}--{}.png", gid, akey, now),
        (Some(gid), None) => format!("{}--{}.png", gid, now),
        _ => format!("manual--{}.png", now),
    };

    let mut path = get_screenshots_dir(app_handle);
    path.push(&filename);
    log::debug!("[screenshot] Saving to: {:?}", path);

    match capture_with_wgc(&path) {
        Ok(()) => {
            log::info!("[screenshot] WGC capture saved: {}", filename);
            return Ok(filename);
        }
        Err(e) => log::warn!("[screenshot] WGC failed, falling back to GDI: {}", e),
    }

    capture_with_gdi(&path).map_err(|e| {
        log::error!("[screenshot] {}", e);
        e
    })?;
    log::info!("[screenshot] Saved: {}", filename);
    Ok(filename)
}

pub fn list_screenshots<R: Runtime>(app_handle: &AppHandle<R>) -> Vec<ScreenshotInfo> {
    let dir = get_screenshots_dir(app_handle);
    let mut list = Vec::new();

    if let Ok(entries) = fs::read_dir(dir) {
        for entry in entries.flatten() {
            let path = entry.path();
            let ext = path
                .extension()
                .and_then(|s| s.to_str())
                .map(|s| s.to_ascii_lowercase());
            let ext = match ext.as_deref() {
                Some("png") => ".png",
                Some("gif") => ".gif",
                _ => continue,
            };
            if !path.is_file() {
                continue;
            }
            if let Some(filename) = path.file_name().and_then(|s| s.to_str()) {
                let name = &filename[..filename.len() - ext.len()];

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
                    if let Some(ts_str) = name.strip_prefix("manual_") {
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

    // Sort by timestamp descending
    list.sort_by(|a, b| b.timestamp.cmp(&a.timestamp));
    list
}
