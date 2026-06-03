use screenshots::Screen;
use serde::Serialize;
use std::fs;
use std::path::PathBuf;
use std::time::{SystemTime, UNIX_EPOCH};
use tauri::{AppHandle, Manager, Runtime};

#[cfg(target_os = "windows")]
use std::sync::{Arc, Mutex};
#[cfg(target_os = "windows")]
use winreg::enums::*;
#[cfg(target_os = "windows")]
use winreg::RegKey;

#[cfg(target_os = "windows")]
use windows_capture::{
    capture::{Context, GraphicsCaptureApiHandler},
    encoder::ImageFormat,
    frame::Frame,
    graphics_capture_api::InternalCaptureControl,
    monitor::Monitor,
    settings::{
        ColorFormat, CursorCaptureSettings, DirtyRegionSettings, DrawBorderSettings,
        MinimumUpdateIntervalSettings, SecondaryWindowSettings, Settings,
    },
};

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

#[cfg(target_os = "windows")]
fn is_hdr_enabled() -> bool {
    let hklm = RegKey::predef(HKEY_LOCAL_MACHINE);
    let paths = [
        "SYSTEM\\CurrentControlSet\\Control\\GraphicsDrivers\\MonitorDataStore",
        "SYSTEM\\CurrentControlSet\\Control\\GraphicsDrivers\\Configuration",
    ];

    for path in paths {
        if let Ok(key) = hklm.open_subkey(path) {
            for name in key.enum_keys().flatten() {
                if let Ok(sub) = key.open_subkey(&name) {
                    let keys = ["AdvancedColorEnabled", "HDREnabled", "HdrSupported"];
                    for k in keys {
                        if let Ok(val) = sub.get_value::<u32, _>(k) {
                            if val == 1 {
                                println!("[DEBUG] HDR detected via HKLM {} ({}={})", path, k, val);
                                return true;
                            }
                        }
                    }
                }
            }
        }
    }

    // Check HKCU VideoSettings
    let hkcu = RegKey::predef(HKEY_CURRENT_USER);
    if let Ok(key) = hkcu.open_subkey("Software\\Microsoft\\Windows\\CurrentVersion\\VideoSettings")
    {
        let keys = [
            "EnableAutoEnhanceDuringPlayback",
            "HDRWhiteLevel",
            "HdrEnabled",
        ];
        for k in keys {
            if let Ok(val) = key.get_value::<u32, _>(k) {
                if val > 0 {
                    println!(
                        "[DEBUG] HDR detected via HKCU VideoSettings ({}={})",
                        k, val
                    );
                    return true;
                }
            }
        }
    }

    false
}

#[cfg(not(target_os = "windows"))]
fn is_hdr_enabled() -> bool {
    false
}

#[cfg(target_os = "windows")]
struct CaptureFlags {
    save_path: PathBuf,
    result: Arc<Mutex<Option<Result<(), String>>>>,
}

#[cfg(target_os = "windows")]
struct OneShotCapture {
    flags: CaptureFlags,
}

#[cfg(target_os = "windows")]
impl GraphicsCaptureApiHandler for OneShotCapture {
    type Flags = CaptureFlags;
    type Error = Box<dyn std::error::Error + Send + Sync>;

    fn new(ctx: Context<Self::Flags>) -> Result<Self, Self::Error> {
        Ok(Self { flags: ctx.flags })
    }

    fn on_frame_arrived(
        &mut self,
        frame: &mut Frame,
        capture_control: InternalCaptureControl,
    ) -> Result<(), Self::Error> {
        let res = frame.save_as_image(&self.flags.save_path, ImageFormat::Png);
        let mut result_lock = self.flags.result.lock().unwrap();
        *result_lock = Some(res.map_err(|e| e.to_string()));
        capture_control.stop();
        Ok(())
    }
}

#[cfg(target_os = "windows")]
fn capture_with_wgc(path: PathBuf) -> Result<(), String> {
    let (tx, rx) = std::sync::mpsc::channel();

    std::thread::spawn(move || {
        let result = Arc::new(Mutex::new(None));
        let flags = CaptureFlags {
            save_path: path,
            result: Arc::clone(&result),
        };

        match Monitor::primary() {
            Ok(monitor) => {
                let settings = Settings::new(
                    monitor,
                    CursorCaptureSettings::WithoutCursor,
                    DrawBorderSettings::WithoutBorder,
                    SecondaryWindowSettings::Default,
                    MinimumUpdateIntervalSettings::Default,
                    DirtyRegionSettings::Default,
                    ColorFormat::Rgba8,
                    flags,
                );

                match OneShotCapture::start(settings) {
                    Ok(_) => {
                        let res = result.lock().unwrap().take().unwrap_or(Ok(()));
                        let _ = tx.send(res);
                    }
                    Err(e) => {
                        let _ = tx.send(Err(e.to_string()));
                    }
                }
            }
            Err(e) => {
                let _ = tx.send(Err(e.to_string()));
            }
        }
    });

    rx.recv().map_err(|e| e.to_string())?
}

fn analyze_clipping(img: &screenshots::image::RgbaImage) -> f32 {
    let mut bright_pixels = 0;
    let sample_step = 8;
    let mut total_samples = 0;

    for y in (0..img.height()).step_by(sample_step) {
        for x in (0..img.width()).step_by(sample_step) {
            let pixel = img.get_pixel(x, y);
            // In HDR games captured in SDR, clipped pixels are exactly 255
            if pixel[0] == 255 || pixel[1] == 255 || pixel[2] == 255 {
                bright_pixels += 1;
            }
            total_samples += 1;
        }
    }

    if total_samples == 0 {
        return 0.0;
    }
    bright_pixels as f32 / total_samples as f32
}

pub fn capture_screenshot<R: Runtime>(
    app_handle: &AppHandle<R>,
    game_id: Option<&str>,
    achievement_key: Option<&str>,
    hdr_to_sdr: bool,
) -> Result<String, String> {
    let actual_hdr_fix = hdr_to_sdr && is_hdr_enabled();
    println!(
        "[DEBUG] Starting screenshot capture... (Setting HDR fix: {}, Actual HDR fix: {})",
        hdr_to_sdr, actual_hdr_fix
    );

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

    #[cfg(target_os = "windows")]
    {
        println!("[DEBUG] Attempting WGC capture for perfect HDR/SDR mapping...");
        match capture_with_wgc(path.clone()) {
            Ok(_) => {
                println!("[DEBUG] WGC Screenshot saved successfully: {}", filename);
                return Ok(filename);
            }
            Err(e) => {
                println!("[ERROR] WGC Capture failed, falling back to legacy: {}", e);
            }
        }
    }

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

        let mut dynamic_image = screenshots::image::DynamicImage::ImageRgba8(image);

        if actual_hdr_fix {
            let mut rgba_image = dynamic_image.to_rgba8();
            let clipping_ratio = analyze_clipping(&rgba_image);
            let needs_tonemap = clipping_ratio > 0.05;

            println!(
                "[DEBUG] HDR Analysis: {:.2}% bright pixels. Adaptive tone mapping: {}",
                clipping_ratio * 100.0,
                if needs_tonemap { "ACTIVE" } else { "INACTIVE" }
            );

            if needs_tonemap {
                println!("[DEBUG] Applying Intelligent HDR to SDR conversion (ACES)...");

                // Precompute LUT for performance
                let mut lut = [0u8; 256];

                fn to_linear(c: f32) -> f32 {
                    if c <= 0.04045 {
                        c / 12.92
                    } else {
                        ((c + 0.055) / 1.055).powf(2.4)
                    }
                }

                fn to_srgb(c: f32) -> f32 {
                    if c <= 0.0031308 {
                        12.92 * c
                    } else {
                        1.055 * c.powf(1.0 / 2.4) - 0.055
                    }
                }

                fn aces_tonemap(x: f32) -> f32 {
                    let a = 2.51;
                    let b = 0.03;
                    let c = 2.43;
                    let d = 0.59;
                    let e = 0.14;
                    ((x * (a * x + b)) / (x * (c * x + d) + e)).clamp(0.0, 1.0)
                }

                for i in 0..256 {
                    let srgb_in = i as f32 / 255.0;
                    let linear_in = to_linear(srgb_in);

                    // Tone mapping
                    // We apply a slight exposure bias to "recover" highlights before mapping
                    let exposure = 0.85;
                    let mapped = aces_tonemap(linear_in * exposure);

                    // Apply gamma correction back to sRGB
                    let final_val = (to_srgb(mapped) * 255.0).round() as u8;
                    lut[i] = final_val;
                }

                // Apply LUT
                for pixel in rgba_image.pixels_mut() {
                    pixel[0] = lut[pixel[0] as usize];
                    pixel[1] = lut[pixel[1] as usize];
                    pixel[2] = lut[pixel[2] as usize];

                    // Optional: Slight saturation boost as ACES can wash out colors
                    let r = pixel[0] as f32;
                    let g = pixel[1] as f32;
                    let b = pixel[2] as f32;
                    let gray = 0.2126 * r + 0.7152 * g + 0.0722 * b;
                    let saturation = 1.15;
                    pixel[0] = (gray + (r - gray) * saturation).clamp(0.0, 255.0) as u8;
                    pixel[1] = (gray + (g - gray) * saturation).clamp(0.0, 255.0) as u8;
                    pixel[2] = (gray + (b - gray) * saturation).clamp(0.0, 255.0) as u8;
                }
                dynamic_image = screenshots::image::DynamicImage::ImageRgba8(rgba_image);
            }
        }

        let mut buffer = Vec::new();
        // Use fast compression to reduce freeze
        let encoder = screenshots::image::codecs::png::PngEncoder::new_with_quality(
            &mut buffer,
            screenshots::image::codecs::png::CompressionType::Fast,
            screenshots::image::codecs::png::FilterType::NoFilter,
        );

        use screenshots::image::ImageEncoder;
        encoder
            .write_image(
                dynamic_image.as_bytes(),
                dynamic_image.width(),
                dynamic_image.height(),
                dynamic_image.color().into(),
            )
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
