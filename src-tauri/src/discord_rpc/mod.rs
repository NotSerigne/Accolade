// src-tauri/src/discord_rpc/mod.rs
//! Lightweight Discord Rich Presence client over Windows named pipes (`\\.\pipe\discord-ipc-0..9`).
//!
//! No external crate needed: the Discord IPC framing is a 8-byte header
//! (`opcode: u32 LE` + `length: u32 LE`) followed by a UTF-8 JSON payload.

use crate::achievements::models::Game;
use crate::game_detector::RunningGame;
use crate::AppState;
use serde_json::json;
use std::fs::{File, OpenOptions};
use std::io::{Read, Write};
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::Duration;
use tauri::Manager;
use tauri_plugin_store::StoreExt;

/// Default public Discord Application ID used by Accolade when the user has not
/// configured a custom one. Can be overridden via `DISCORD_CLIENT_ID` env var.
const DEFAULT_CLIENT_ID: &str = "1373355586959380570";
const POLL_INTERVAL: Duration = Duration::from_secs(5);

static NONCE: AtomicU64 = AtomicU64::new(1);

#[derive(Clone, PartialEq, Debug)]
struct PresenceData {
    game_name: String,
    details: String,
    state_line: String,
    large_image: String,
    started_at: u64,
}

struct IpcClient {
    pipe: File,
}

impl IpcClient {
    fn connect(client_id: &str) -> Option<Self> {
        for idx in 0..10 {
            let path = format!(r"\\.\pipe\discord-ipc-{}", idx);
            let Ok(pipe) = OpenOptions::new().read(true).write(true).open(&path) else {
                continue;
            };
            let mut client = Self { pipe };
            let handshake = json!({
                "v": 1,
                "client_id": client_id,
            });
            if client.send_frame(0, &handshake).is_ok() && client.read_frame().is_some() {
                log::info!("[discord-rpc] Connected on {}", path);
                return Some(client);
            }
        }
        None
    }

    fn send_frame(&mut self, opcode: u32, payload: &serde_json::Value) -> std::io::Result<()> {
        let body = payload.to_string();
        let bytes = body.as_bytes();
        let mut buf = Vec::with_capacity(8 + bytes.len());
        buf.extend_from_slice(&opcode.to_le_bytes());
        buf.extend_from_slice(&(bytes.len() as u32).to_le_bytes());
        buf.extend_from_slice(bytes);
        self.pipe.write_all(&buf)?;
        self.pipe.flush()
    }

    fn read_frame(&mut self) -> Option<(u32, String)> {
        let mut header = [0u8; 8];
        self.pipe.read_exact(&mut header).ok()?;
        let op = u32::from_le_bytes([header[0], header[1], header[2], header[3]]);
        let len = u32::from_le_bytes([header[4], header[5], header[6], header[7]]) as usize;
        if len > 64 * 1024 {
            return None;
        }
        let mut body = vec![0u8; len];
        self.pipe.read_exact(&mut body).ok()?;
        Some((op, String::from_utf8_lossy(&body).into_owned()))
    }

    fn set_activity(&mut self, data: &PresenceData) -> std::io::Result<()> {
        let nonce = NONCE.fetch_add(1, Ordering::Relaxed).to_string();
        let mut activity = json!({
            "details": data.details,
            "state": data.state_line,
            "timestamps": {
                "start": data.started_at,
            },
            "assets": {
                "large_image": data.large_image,
                "large_text": data.game_name,
            },
        });
        if data.large_image.is_empty() {
            if let Some(obj) = activity.as_object_mut() {
                obj.remove("assets");
            }
        }
        let frame = json!({
            "cmd": "SET_ACTIVITY",
            "args": {
                "pid": std::process::id(),
                "activity": activity,
            },
            "nonce": nonce,
        });
        self.send_frame(1, &frame)?;
        let _ = self.read_frame();
        Ok(())
    }

    fn clear_activity(&mut self) -> std::io::Result<()> {
        let nonce = NONCE.fetch_add(1, Ordering::Relaxed).to_string();
        let frame = json!({
            "cmd": "SET_ACTIVITY",
            "args": {
                "pid": std::process::id(),
                "activity": serde_json::Value::Null,
            },
            "nonce": nonce,
        });
        self.send_frame(1, &frame)?;
        let _ = self.read_frame();
        Ok(())
    }
}

impl Drop for IpcClient {
    fn drop(&mut self) {
        let _ = self.clear_activity();
    }
}

fn is_enabled(app: &tauri::AppHandle) -> bool {
    app.store("settings.json")
        .ok()
        .and_then(|s| s.get("discordRpc"))
        .and_then(|v| v.as_bool())
        .unwrap_or(true)
}

fn build_presence(running: &RunningGame, games: &[Game], language: &str) -> PresenceData {
    let matched = games.iter().find(|g| {
        running.game_id.as_ref().is_some_and(|id| &g.id == id) || g.steam_id == Some(running.app_id)
    });

    let (unlocked, total, icon_url) = match matched {
        Some(g) => {
            let unlocked = g
                .achievements
                .iter()
                .filter(|a| a.unlocked || a.unlocked_time.is_some())
                .count() as u32;
            let total = g.achievements_total.max(g.achievements.len() as u32);
            let icon = if !g.steamgrid_icon_url.is_empty() {
                g.steamgrid_icon_url.clone()
            } else if g.game_icon.starts_with("http") {
                g.game_icon.clone()
            } else if !g.header_image_url.is_empty() {
                g.header_image_url.clone()
            } else {
                format!(
                    "https://cdn.cloudflare.steamstatic.com/steam/apps/{}/header.jpg",
                    running.app_id
                )
            };
            (unlocked, total, icon)
        }
        None => (
            0,
            0,
            format!(
                "https://cdn.cloudflare.steamstatic.com/steam/apps/{}/header.jpg",
                running.app_id
            ),
        ),
    };

    let state_line = if total > 0 {
        let pct = ((unlocked as f64 / total as f64) * 100.0).round() as u32;
        match language {
            "fr" => format!("🏆 {}/{} succès ({}%)", unlocked, total, pct),
            "es" => format!("🏆 {}/{} logros ({}%)", unlocked, total, pct),
            "de" => format!("🏆 {}/{} Erfolge ({}%)", unlocked, total, pct),
            "it" => format!("🏆 {}/{} obiettivi ({}%)", unlocked, total, pct),
            _ => format!("🏆 {}/{} achievements ({}%)", unlocked, total, pct),
        }
    } else {
        "Accolade".to_string()
    };

    PresenceData {
        game_name: running.name.clone(),
        details: running.name.clone(),
        state_line,
        large_image: icon_url,
        started_at: running.started_at,
    }
}

pub fn start(app_handle: tauri::AppHandle) {
    std::thread::spawn(move || {
        let client_id = std::env::var("DISCORD_CLIENT_ID")
            .ok()
            .filter(|s| !s.trim().is_empty())
            .unwrap_or_else(|| DEFAULT_CLIENT_ID.to_string());

        let mut client: Option<IpcClient> = None;
        let mut last_sent: Option<PresenceData> = None;

        loop {
            std::thread::sleep(POLL_INTERVAL);

            if !is_enabled(&app_handle) {
                if let Some(mut c) = client.take() {
                    let _ = c.clear_activity();
                }
                last_sent = None;
                continue;
            }

            let (running, games, language) = {
                let state = app_handle.state::<AppState>();
                let running = state.running_game.lock().ok().and_then(|g| g.clone());
                let games = state.games.lock().map(|g| g.clone()).unwrap_or_default();
                let language = state
                    .language
                    .lock()
                    .map(|l| l.clone())
                    .unwrap_or_else(|_| "fr".to_string());
                (running, games, language)
            };

            let Some(running) = running else {
                if last_sent.is_some() {
                    if let Some(c) = client.as_mut() {
                        if c.clear_activity().is_err() {
                            client = None;
                        }
                    }
                    last_sent = None;
                }
                continue;
            };

            let desired = build_presence(&running, &games, &language);
            if last_sent.as_ref() == Some(&desired) && client.is_some() {
                continue;
            }

            if client.is_none() {
                client = IpcClient::connect(&client_id);
            }
            if let Some(c) = client.as_mut() {
                match c.set_activity(&desired) {
                    Ok(()) => last_sent = Some(desired),
                    Err(_) => {
                        client = None;
                        last_sent = None;
                    }
                }
            }
        }
    });
}
