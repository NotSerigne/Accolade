// src-tauri/src/replay.rs
use crate::screenshots::get_screenshots_dir;
use crate::AppState;
use screenshots::image::{
    self,
    codecs::gif::{GifEncoder, Repeat},
    Delay, Frame,
};
use screenshots::Screen;
use std::collections::VecDeque;
use std::fs::File;
use std::path::PathBuf;
use std::sync::{Mutex, OnceLock};
use std::time::{Duration, SystemTime, UNIX_EPOCH};
use tauri::{AppHandle, Emitter, Manager, Runtime};

const MAX_FRAMES: usize = 36; // ~6 seconds at 6 FPS
const CAPTURE_INTERVAL_MS: u64 = 165;
const TARGET_WIDTH: u32 = 480;

#[derive(Clone)]
struct BufferedFrame {
    width: u32,
    height: u32,
    rgba: Vec<u8>,
}

static REPLAY_BUFFER: OnceLock<Mutex<VecDeque<BufferedFrame>>> = OnceLock::new();

fn buffer() -> &'static Mutex<VecDeque<BufferedFrame>> {
    REPLAY_BUFFER.get_or_init(|| Mutex::new(VecDeque::with_capacity(MAX_FRAMES)))
}

fn is_replay_enabled(app: &AppHandle) -> bool {
    let Ok(app_dir) = app.path().app_data_dir() else {
        return false;
    };
    let settings_path = app_dir.join("settings.json");
    let Ok(raw) = std::fs::read_to_string(settings_path) else {
        return false;
    };
    let Ok(json) = serde_json::from_str::<serde_json::Value>(&raw) else {
        return false;
    };
    json.get("replayBuffer")
        .and_then(|v| v.as_bool())
        .unwrap_or(false)
}

pub fn start(app_handle: AppHandle) {
    std::thread::spawn(move || loop {
        let enabled = is_replay_enabled(&app_handle);
        let in_game = app_handle
            .try_state::<AppState>()
            .and_then(|s| s.running_game.lock().ok().map(|g| g.is_some()))
            .unwrap_or(false);

        if !enabled {
            if let Ok(mut buf) = buffer().lock() {
                buf.clear();
            }
            std::thread::sleep(Duration::from_secs(3));
            continue;
        }

        // Capture frames when replayBuffer is enabled (always when in game, or at reduced rate when testing)
        if let Ok(screens) = Screen::all() {
            if let Some(screen) = screens
                .iter()
                .find(|s| s.display_info.is_primary)
                .or(screens.first())
            {
                if let Ok(img) = screen.capture() {
                    let src_w = img.width().max(1);
                    let src_h = img.height().max(1);
                    let dst_w = TARGET_WIDTH.min(src_w);
                    let dst_h = ((src_h as u64 * dst_w as u64) / src_w as u64).max(1) as u32;
                    let resized = image::imageops::resize(
                        &img,
                        dst_w,
                        dst_h,
                        image::imageops::FilterType::Nearest,
                    );
                    let frame = BufferedFrame {
                        width: dst_w,
                        height: dst_h,
                        rgba: resized.into_raw(),
                    };
                    if let Ok(mut buf) = buffer().lock() {
                        if buf.len() >= MAX_FRAMES {
                            buf.pop_front();
                        }
                        buf.push_back(frame);
                    }
                }
            }
        }

        let wait_ms = if in_game {
            CAPTURE_INTERVAL_MS
        } else {
            CAPTURE_INTERVAL_MS * 2
        };
        std::thread::sleep(Duration::from_millis(wait_ms));
    });
}

/// Flushes the current ring buffer to an animated `.gif` file in the screenshots directory.
pub fn flush_replay_gif<R: Runtime>(
    app_handle: &AppHandle<R>,
    game_id: Option<&str>,
    achievement_key: Option<&str>,
) -> Result<Option<String>, String> {
    let frames: Vec<BufferedFrame> = {
        let Ok(guard) = buffer().lock() else {
            return Ok(None);
        };
        if guard.len() < 4 {
            return Ok(None);
        }
        guard.iter().cloned().collect()
    };

    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0);

    let filename = match (game_id, achievement_key) {
        (Some(gid), Some(akey)) => format!("{}--{}--{}.gif", gid, akey, now),
        (Some(gid), None) => format!("{}--{}.gif", gid, now),
        _ => format!("manual--{}.gif", now),
    };

    let mut out_path: PathBuf = get_screenshots_dir(app_handle);
    out_path.push(&filename);

    let file = File::create(&out_path).map_err(|e| e.to_string())?;
    let mut encoder = GifEncoder::new_with_speed(file, 20);
    let _ = encoder.set_repeat(Repeat::Infinite);

    for f in frames {
        let Some(rgba_img) = image::RgbaImage::from_raw(f.width, f.height, f.rgba) else {
            continue;
        };
        let frame = Frame::from_parts(
            rgba_img,
            0,
            0,
            Delay::from_numer_denom_ms(CAPTURE_INTERVAL_MS as u32, 1),
        );
        encoder.encode_frame(frame).map_err(|e| e.to_string())?;
    }

    let _ = app_handle.emit("screenshot-taken", filename.clone());
    Ok(Some(filename))
}
