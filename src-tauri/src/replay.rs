// src-tauri/src/replay.rs
//! Rolling replay buffer: keeps the last few seconds of the primary monitor in memory
//! (downscaled) and dumps them to an animated GIF when an achievement unlocks.
//!
//! Frames come from a long-lived Windows Graphics Capture session, which (unlike GDI)
//! works with fullscreen games and HDR displays.
use crate::capture::{self, DisplayColorInfo};
use crate::screenshots::get_screenshots_dir;
use crate::AppState;
use image::codecs::gif::{GifEncoder, Repeat};
use image::{Delay, Frame, RgbaImage};
use std::collections::VecDeque;
use std::fs::File;
use std::io::BufWriter;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::{Mutex, OnceLock};
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};
use tauri::{AppHandle, Emitter, Manager, Runtime};
use windows_capture::{
    capture::{CaptureControl, Context, GraphicsCaptureApiHandler},
    frame::Frame as CaptureFrame,
    graphics_capture_api::InternalCaptureControl,
};

/// Length of the clip saved to disk.
const CLIP_DURATION: Duration = Duration::from_secs(6);
const MAX_FRAMES: usize = 40;
/// Interval between buffered frames while a game is running (~6 FPS).
const IN_GAME_INTERVAL_MS: u64 = 165;
/// Interval while no game is detected (keeps a cheap buffer for undetected games).
const IDLE_INTERVAL_MS: u64 = 400;
const TARGET_WIDTH: u32 = 480;
/// Delay after the unlock before flushing, so the clip also shows the moment itself.
const FLUSH_DELAY: Duration = Duration::from_millis(1500);

struct BufferedFrame {
    at: Instant,
    width: u32,
    height: u32,
    rgba: Vec<u8>,
}

static REPLAY_BUFFER: OnceLock<Mutex<VecDeque<BufferedFrame>>> = OnceLock::new();
static CAPTURE_INTERVAL_MS: AtomicU64 = AtomicU64::new(IDLE_INTERVAL_MS);
static FLUSH_IN_PROGRESS: AtomicBool = AtomicBool::new(false);

fn buffer() -> &'static Mutex<VecDeque<BufferedFrame>> {
    REPLAY_BUFFER.get_or_init(|| Mutex::new(VecDeque::with_capacity(MAX_FRAMES)))
}

fn clear_buffer() {
    if let Ok(mut buf) = buffer().lock() {
        buf.clear();
    }
}

pub fn is_replay_enabled<R: Runtime>(app: &AppHandle<R>) -> bool {
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

struct ReplayCapture {
    info: DisplayColorInfo,
    last: Option<Instant>,
}

impl GraphicsCaptureApiHandler for ReplayCapture {
    type Flags = DisplayColorInfo;
    type Error = Box<dyn std::error::Error + Send + Sync>;

    fn new(ctx: Context<Self::Flags>) -> Result<Self, Self::Error> {
        Ok(Self {
            info: ctx.flags,
            last: None,
        })
    }

    fn on_frame_arrived(
        &mut self,
        frame: &mut CaptureFrame,
        _capture_control: InternalCaptureControl,
    ) -> Result<(), Self::Error> {
        let interval = Duration::from_millis(CAPTURE_INTERVAL_MS.load(Ordering::Relaxed));
        let now = Instant::now();
        if self
            .last
            .is_some_and(|last| now.duration_since(last) < interval)
        {
            return Ok(());
        }
        self.last = Some(now);

        let mut fb = frame.buffer()?;
        let (width, height, pitch, format) = (
            fb.width(),
            fb.height(),
            fb.row_pitch() as usize,
            fb.color_format(),
        );
        let (dst_w, dst_h, rgba) = capture::downscale_to_rgba8(
            fb.as_raw_buffer(),
            width,
            height,
            pitch,
            format,
            &self.info,
            TARGET_WIDTH,
        );

        if let Ok(mut buf) = buffer().lock() {
            while buf.len() >= MAX_FRAMES {
                buf.pop_front();
            }
            buf.push_back(BufferedFrame {
                at: now,
                width: dst_w,
                height: dst_h,
                rgba,
            });
        }
        Ok(())
    }
}

type ReplaySession = CaptureControl<ReplayCapture, Box<dyn std::error::Error + Send + Sync>>;

fn start_session(info: DisplayColorInfo) -> Result<ReplaySession, String> {
    // Ask WGC not to wake us more often than needed (Win11 24H2+; ignored elsewhere).
    let settings = capture::primary_monitor_settings(
        &info,
        Some(Duration::from_millis(IN_GAME_INTERVAL_MS / 2)),
        info,
    )?;
    ReplayCapture::start_free_threaded(settings).map_err(|e| e.to_string())
}

pub fn start(app_handle: AppHandle) {
    std::thread::spawn(move || {
        let mut session: Option<(ReplaySession, DisplayColorInfo)> = None;
        loop {
            let enabled = is_replay_enabled(&app_handle);
            let in_game = app_handle
                .try_state::<AppState>()
                .and_then(|s| s.running_game.lock().ok().map(|g| g.is_some()))
                .unwrap_or(false);
            CAPTURE_INTERVAL_MS.store(
                if in_game {
                    IN_GAME_INTERVAL_MS
                } else {
                    IDLE_INTERVAL_MS
                },
                Ordering::Relaxed,
            );

            if enabled {
                let info = capture::primary_display_color_info();
                let needs_restart = match &session {
                    None => true,
                    // Restart when the session died or when HDR was toggled (pixel format changes)
                    Some((control, current)) => {
                        control.is_finished()
                            || current.advanced_color != info.advanced_color
                            || (current.sdr_white_scale - info.sdr_white_scale).abs() > 0.01
                    }
                };
                if needs_restart {
                    if let Some((control, _)) = session.take() {
                        let _ = control.stop();
                    }
                    clear_buffer();
                    match start_session(info) {
                        Ok(control) => {
                            log::info!(
                                "[replay] Capture session started (hdr={}, advanced_color={})",
                                info.hdr,
                                info.advanced_color
                            );
                            session = Some((control, info));
                        }
                        Err(e) => log::warn!("[replay] Failed to start capture session: {}", e),
                    }
                }
            } else if let Some((control, _)) = session.take() {
                let _ = control.stop();
                clear_buffer();
                log::info!("[replay] Capture session stopped");
            }

            std::thread::sleep(Duration::from_secs(2));
        }
    });
}

/// Saves a replay for a freshly unlocked achievement, slightly after the unlock so the clip
/// covers the moment itself. No-op if the replay buffer is disabled.
pub fn schedule_flush(app_handle: AppHandle, game_id: String, achievement_key: String) {
    if !is_replay_enabled(&app_handle) {
        return;
    }
    std::thread::spawn(move || {
        std::thread::sleep(FLUSH_DELAY);
        match flush_replay_gif(&app_handle, Some(&game_id), Some(&achievement_key)) {
            Ok(Some(name)) => log::info!("[replay] Saved replay clip: {}", name),
            Ok(None) => log::warn!("[replay] Not enough buffered frames to save a replay"),
            Err(e) => log::error!("[replay] Failed to save replay: {}", e),
        }
    });
}

/// Flushes the current ring buffer to an animated `.gif` file in the screenshots directory.
pub fn flush_replay_gif<R: Runtime>(
    app_handle: &AppHandle<R>,
    game_id: Option<&str>,
    achievement_key: Option<&str>,
) -> Result<Option<String>, String> {
    // Serialise flushes: several achievements unlocked at once would otherwise encode in parallel.
    while FLUSH_IN_PROGRESS
        .compare_exchange(false, true, Ordering::Acquire, Ordering::Relaxed)
        .is_err()
    {
        std::thread::sleep(Duration::from_millis(50));
    }
    let result = encode_gif(app_handle, game_id, achievement_key);
    FLUSH_IN_PROGRESS.store(false, Ordering::Release);
    result
}

fn encode_gif<R: Runtime>(
    app_handle: &AppHandle<R>,
    game_id: Option<&str>,
    achievement_key: Option<&str>,
) -> Result<Option<String>, String> {
    let frames: Vec<(Instant, u32, u32, Vec<u8>)> = {
        let Ok(guard) = buffer().lock() else {
            return Ok(None);
        };
        let Some(newest) = guard.back().map(|f| f.at) else {
            return Ok(None);
        };
        guard
            .iter()
            .filter(|f| newest.duration_since(f.at) <= CLIP_DURATION)
            .map(|f| (f.at, f.width, f.height, f.rgba.clone()))
            .collect()
    };
    if frames.len() < 4 {
        return Ok(None);
    }

    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0);

    let filename = match (game_id, achievement_key) {
        (Some(gid), Some(akey)) => format!("{}--{}--{}.gif", gid, akey, now),
        (Some(gid), None) => format!("{}--{}.gif", gid, now),
        _ => format!("manual--{}.gif", now),
    };

    let dir = get_screenshots_dir(app_handle);
    let out_path = dir.join(&filename);
    // Write to a temp name so the gallery never lists a half-written file.
    let tmp_path = dir.join(format!("{}.part", filename));

    {
        let file = File::create(&tmp_path).map_err(|e| e.to_string())?;
        let mut encoder = GifEncoder::new_with_speed(BufWriter::new(file), 20);
        let _ = encoder.set_repeat(Repeat::Infinite);

        let (first_w, first_h) = (frames[0].1, frames[0].2);
        for (i, (at, w, h, rgba)) in frames.iter().enumerate() {
            // A resolution change mid-buffer (e.g. game switching modes) can't go in one GIF.
            if *w != first_w || *h != first_h {
                continue;
            }
            let delay_ms = frames
                .get(i + 1)
                .map(|next| next.0.duration_since(*at).as_millis() as u32)
                .unwrap_or(IN_GAME_INTERVAL_MS as u32)
                .clamp(20, 1000);
            let Some(img) = RgbaImage::from_raw(*w, *h, rgba.clone()) else {
                continue;
            };
            let frame = Frame::from_parts(img, 0, 0, Delay::from_numer_denom_ms(delay_ms, 1));
            if let Err(e) = encoder.encode_frame(frame) {
                drop(encoder);
                let _ = std::fs::remove_file(&tmp_path);
                return Err(e.to_string());
            }
        }
    }

    std::fs::rename(&tmp_path, &out_path).map_err(|e| e.to_string())?;
    let _ = app_handle.emit("screenshot-taken", filename.clone());
    Ok(Some(filename))
}
