// src-tauri/src/game_detector/mod.rs
//! Detects which game is currently running, including cracked / emulated games.
//!
//! Every few seconds the running processes are listed and, for each executable,
//! we look for the files that Steam emulators always ship next to the game
//! (`steam_settings/steam_appid.txt`, `steam_emu.ini`, `OnlineFix.ini`, ...).
//! These files contain the real Steam AppID, which is then matched against the
//! library. Legit Steam installs are recognised through `steamapps/common/<dir>`
//! and the matching `appmanifest_<appid>.acf`.
//!
//! Results are cached per executable path, so the filesystem is only probed the
//! first time a given executable is seen.

use crate::achievements::models::{Achievement, Game};
use crate::achievements::steam::is_valid_game_name;
use crate::user_data::{PlaySession, MIN_SESSION_DURATION_SECS};
use crate::AppState;
use serde::Serialize;
use std::collections::{HashMap, HashSet};
use std::path::{Component, Path, PathBuf};
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};
use sysinfo::{ProcessRefreshKind, ProcessesToUpdate, System, UpdateKind};
use tauri::{Emitter, Manager};

const SCAN_INTERVAL: Duration = Duration::from_secs(5);
/// How many parent directories above the executable are probed for markers
/// (e.g. `Game/Binaries/Win64/Game-Win64-Shipping.exe` needs 3).
const MAX_PARENT_DEPTH: usize = 5;
/// Marker files larger than this are ignored (they are tiny in practice).
const MAX_MARKER_SIZE: u64 = 256 * 1024;
const EXE_CACHE_LIMIT: usize = 4096;
const MANIFEST_REFRESH_INTERVAL: Duration = Duration::from_secs(60);

pub const RUNNING_GAME_EVENT: &str = "running-game-changed";
pub const PLAYTIME_UPDATED_EVENT: &str = "playtime-updated";
pub const PLAY_SESSION_RECORDED_EVENT: &str = "play-session-recorded";

/// Processes that live next to a game (or in Steam) but are never the game itself.
const IGNORED_PROCESS_NAMES: &[&str] = &[
    "steam.exe",
    "steamwebhelper.exe",
    "steamservice.exe",
    "steamerrorreporter.exe",
    "steamerrorreporter64.exe",
    "gameoverlayui.exe",
    "gameoverlayui64.exe",
    "crashpad_handler.exe",
    "crashreportclient.exe",
    "unitycrashhandler32.exe",
    "unitycrashhandler64.exe",
    "easyanticheat.exe",
    "easyanticheat_eos.exe",
    "start_protected_game.exe",
    "wallpaper32.exe",
    "wallpaper64.exe",
    "wallpaperservice32.exe",
    "wallpaperservice64.exe",
    "webwallpaper32.exe",
    "ui32.exe",
    "borderlessgaming.exe",
    "losslessscaling.exe",
];

/// Non-game Steam AppIDs (utilities, runtimes, redistributables, tools).
/// 480 = Spacewar (emulator placeholder), 431960 = Wallpaper Engine,
/// 388080 = Borderless Gaming, 993090 = Lossless Scaling, 629520 = Soundpad,
/// 228980 = Steamworks Common Redistributables.
const IGNORED_APP_IDS: &[u32] = &[
    0, 480, 228980, 388080, 431960, 629520, 993090, 1007, 1391110, 1493710,
];

fn is_ignored_app_id(app_id: u32) -> bool {
    IGNORED_APP_IDS.contains(&app_id)
}

#[derive(Serialize, Clone, Debug, PartialEq)]
pub struct RunningGame {
    /// Steam AppID of the running game.
    pub app_id: u32,
    /// Library id (e.g. `steam_1245620`) when the game is part of the library.
    pub game_id: Option<String>,
    pub name: String,
    pub exe_path: String,
    pub pid: u32,
    /// Unix timestamp (seconds) at which the game process started.
    pub started_at: u64,
    /// Which marker identified the game (`goldberg`, `steam_emu.ini`, `steam`, ...).
    pub detected_via: String,
}

#[derive(Clone, Debug, PartialEq)]
struct Detection {
    app_id: u32,
    /// Directory considered to be the game root (used for name fallback).
    root_dir: PathBuf,
    via: &'static str,
}

#[derive(Clone, Debug)]
struct Candidate {
    detection: Detection,
    pid: u32,
    exe_path: PathBuf,
    start_time: u64,
}

// ---------------------------------------------------------------------------
// Marker parsing
// ---------------------------------------------------------------------------

/// INI-like marker files: (file name, keys in priority order, label).
const INI_MARKERS: &[(&str, &[&str], &str)] = &[
    ("OnlineFix.ini", &["RealAppId", "AppId"], "onlinefix"),
    ("steam_emu.ini", &["AppId"], "steam_emu.ini"),
    ("ColdClientLoader.ini", &["AppId"], "goldberg"),
    ("cream_api.ini", &["appid"], "creamapi"),
    ("SmartSteamEmu.ini", &["AppId"], "smartsteamemu"),
    ("ALI213.ini", &["AppID", "AppId"], "ali213"),
];

fn read_small_file(path: &Path) -> Option<String> {
    let meta = std::fs::metadata(path).ok()?;
    if !meta.is_file() || meta.len() > MAX_MARKER_SIZE {
        return None;
    }
    let bytes = std::fs::read(path).ok()?;
    Some(String::from_utf8_lossy(&bytes).into_owned())
}

fn leading_u32(raw: &str) -> Option<u32> {
    let digits: String = raw
        .trim()
        .trim_matches('"')
        .chars()
        .take_while(|c| c.is_ascii_digit())
        .collect();
    digits.parse::<u32>().ok()
}

/// Returns the first non-ignored numeric value for `keys` (in priority order).
fn find_ini_app_id(content: &str, keys: &[&str]) -> Option<u32> {
    keys.iter().find_map(|wanted| {
        content.lines().find_map(|line| {
            let line = line.trim();
            if line.starts_with(';') || line.starts_with('#') || line.starts_with('[') {
                return None;
            }
            let (key, value) = line.split_once('=')?;
            if !key.trim().eq_ignore_ascii_case(wanted) {
                return None;
            }
            leading_u32(value).filter(|id| !is_ignored_app_id(*id))
        })
    })
}

fn find_txt_app_id(content: &str) -> Option<u32> {
    content
        .split(|c: char| !c.is_ascii_digit())
        .find(|token| !token.is_empty())
        .and_then(|token| token.parse::<u32>().ok())
        .filter(|id| !is_ignored_app_id(*id))
}

/// True when `dir` (or the executable's own folder) also contains a Steam API
/// DLL. Third-party telemetry daemons (e.g. sim-racing overlays in WindowsApps)
/// sometimes drop a bare `steam_appid.txt` without shipping any game binaries;
/// requiring a sibling `steam_api(64).dll` for bare `steam_appid.txt` avoids
/// mistaking them for the game itself.
fn has_steam_api_dll(dir: &Path, exe_dir: &Path) -> bool {
    const DLLS: &[&str] = &[
        "steam_api64.dll",
        "steam_api.dll",
        "steamclient64.dll",
        "steamclient.dll",
    ];
    DLLS.iter()
        .any(|dll| dir.join(dll).is_file() || exe_dir.join(dll).is_file())
}

/// Looks for emulator marker files directly inside `dir`.
fn detect_in_dir(dir: &Path, exe_dir: &Path) -> Option<(u32, &'static str)> {
    if let Some(id) = read_small_file(&dir.join("steam_settings/steam_appid.txt"))
        .and_then(|c| find_txt_app_id(&c))
    {
        return Some((id, "goldberg"));
    }

    for (file, keys, via) in INI_MARKERS {
        if let Some(id) = read_small_file(&dir.join(file)).and_then(|c| find_ini_app_id(&c, keys)) {
            return Some((id, via));
        }
    }

    if let Some(id) =
        read_small_file(&dir.join("steam_appid.txt")).and_then(|c| find_txt_app_id(&c))
    {
        if has_steam_api_dll(dir, exe_dir) {
            return Some((id, "steam_appid.txt"));
        }
    }

    None
}

/// Walks up from the executable's directory looking for emulator markers.
fn detect_emulator_markers(exe: &Path) -> Option<Detection> {
    let exe_dir = exe.parent()?;
    let mut dir = Some(exe_dir);
    for _ in 0..MAX_PARENT_DEPTH {
        let current = dir?;
        // Never probe a drive root (`C:\`).
        current.parent()?;
        if let Some((app_id, via)) = detect_in_dir(current, exe_dir) {
            return Some(Detection {
                app_id,
                root_dir: current.to_path_buf(),
                via,
            });
        }
        dir = current.parent();
    }
    None
}

// ---------------------------------------------------------------------------
// Legit Steam installs (steamapps/common/<installdir>)
// ---------------------------------------------------------------------------

/// Splits `.../steamapps/common/<installdir>/...` into (steamapps dir, installdir).
fn split_steam_library_path(exe: &Path) -> Option<(PathBuf, String)> {
    let components: Vec<Component> = exe.components().collect();
    for i in 0..components.len().saturating_sub(2) {
        let is_steamapps = components[i]
            .as_os_str()
            .to_string_lossy()
            .eq_ignore_ascii_case("steamapps");
        let is_common = components[i + 1]
            .as_os_str()
            .to_string_lossy()
            .eq_ignore_ascii_case("common");
        if is_steamapps && is_common && i + 2 < components.len() - 1 {
            let steamapps: PathBuf = components[..=i].iter().collect();
            let installdir = components[i + 2].as_os_str().to_string_lossy().to_string();
            return Some((steamapps, installdir));
        }
    }
    None
}

/// Extracts `value` from a VDF line such as `"installdir"\t\t"Elden Ring"`.
fn acf_value<'a>(line: &'a str, key: &str) -> Option<&'a str> {
    let parts: Vec<&str> = line.trim().split('"').collect();
    if parts.len() >= 4 && parts[1].eq_ignore_ascii_case(key) {
        Some(parts[3])
    } else {
        None
    }
}

/// Parses an `appmanifest_*.acf` into (installdir, appid).
fn parse_app_manifest(content: &str) -> Option<(String, u32)> {
    let mut app_id = None;
    let mut install_dir = None;
    for line in content.lines() {
        if app_id.is_none() {
            app_id = acf_value(line, "appid").and_then(|v| v.parse::<u32>().ok());
        }
        if install_dir.is_none() {
            install_dir = acf_value(line, "installdir").map(|v| v.to_string());
        }
        if app_id.is_some() && install_dir.is_some() {
            break;
        }
    }
    Some((install_dir?, app_id?))
}

#[derive(Default)]
struct SteamManifestCache {
    /// steamapps dir -> (installdir lowercase -> appid, last read)
    libraries: HashMap<PathBuf, (HashMap<String, u32>, Instant)>,
}

impl SteamManifestCache {
    fn read_library(steamapps: &Path) -> HashMap<String, u32> {
        let mut map = HashMap::new();
        let Ok(entries) = std::fs::read_dir(steamapps) else {
            return map;
        };
        for entry in entries.flatten() {
            let name = entry.file_name().to_string_lossy().to_ascii_lowercase();
            if !(name.starts_with("appmanifest_") && name.ends_with(".acf")) {
                continue;
            }
            if let Some((dir, id)) =
                read_small_file(&entry.path()).and_then(|c| parse_app_manifest(&c))
            {
                map.insert(dir.to_lowercase(), id);
            }
        }
        map
    }

    fn lookup(&mut self, steamapps: &Path, installdir: &str) -> Option<u32> {
        let key = installdir.to_lowercase();
        let needs_refresh = match self.libraries.get(steamapps) {
            None => true,
            Some((map, read_at)) => {
                !map.contains_key(&key) && read_at.elapsed() > MANIFEST_REFRESH_INTERVAL
            }
        };
        if needs_refresh {
            let map = Self::read_library(steamapps);
            self.libraries
                .insert(steamapps.to_path_buf(), (map, Instant::now()));
        }
        self.libraries
            .get(steamapps)
            .and_then(|(map, _)| map.get(&key).copied())
            .filter(|id| !is_ignored_app_id(*id))
    }
}

fn detect_steam_install(exe: &Path, manifests: &mut SteamManifestCache) -> Option<Detection> {
    let (steamapps, installdir) = split_steam_library_path(exe)?;
    let app_id = manifests.lookup(&steamapps, &installdir)?;
    Some(Detection {
        app_id,
        root_dir: steamapps.join("common").join(&installdir),
        via: "steam",
    })
}

// ---------------------------------------------------------------------------
// Process scanning
// ---------------------------------------------------------------------------

fn is_ignored_exe(exe: &Path, windows_dir: &Option<String>) -> bool {
    let name = exe
        .file_name()
        .map(|n| n.to_string_lossy().to_ascii_lowercase())
        .unwrap_or_default();
    if !name.ends_with(".exe") || IGNORED_PROCESS_NAMES.contains(&name.as_str()) {
        return true;
    }
    let lower = exe.to_string_lossy().to_lowercase().replace('/', "\\");
    if let Some(win) = windows_dir {
        if lower.starts_with(win) {
            return true;
        }
    }
    // Microsoft Store apps and background companions (e.g. GridCard) live in
    // WindowsApps; non-Steam apps in Program Files are only valid when they are
    // inside a Steam library (`\steamapps\common\`).
    if lower.contains("\\windowsapps\\") {
        return true;
    }
    if (lower.contains("\\program files\\") || lower.contains("\\program files (x86)\\"))
        && !lower.contains("\\steamapps\\common\\")
    {
        return true;
    }
    false
}

fn is_technical_dir_name(name: &str) -> bool {
    matches!(
        name.trim().to_ascii_lowercase().as_str(),
        "bin"
            | "bin32"
            | "bin64"
            | "binaries"
            | "win32"
            | "win64"
            | "x86"
            | "x64"
            | "x86_64"
            | "game"
            | "games"
            | "retail"
            | "release"
            | "shipping"
            | "common"
            | "steam_settings"
    )
}

fn folder_name_fallback(root: &Path) -> Option<String> {
    let mut cursor = Some(root);
    for _ in 0..4 {
        let dir = cursor?;
        if let Some(name) = dir.file_name().map(|n| n.to_string_lossy().to_string()) {
            if !name.trim().is_empty() && !is_technical_dir_name(&name) {
                return Some(name);
            }
        }
        cursor = dir.parent();
    }
    None
}

struct Detector {
    system: System,
    exe_cache: HashMap<PathBuf, Option<Detection>>,
    manifests: SteamManifestCache,
    name_cache: HashMap<u32, String>,
    api_lookup_attempted: std::collections::HashSet<u32>,
    windows_dir: Option<String>,
    own_pid: u32,
    current: Option<RunningGame>,
}

impl Detector {
    fn new() -> Self {
        let windows_dir = std::env::var("WINDIR")
            .or_else(|_| std::env::var("SystemRoot"))
            .ok()
            .map(|w| format!("{}\\", w.trim_end_matches('\\').to_lowercase()));
        Self {
            system: System::new(),
            exe_cache: HashMap::new(),
            manifests: SteamManifestCache::default(),
            name_cache: HashMap::new(),
            api_lookup_attempted: Default::default(),
            windows_dir,
            own_pid: std::process::id(),
            current: None,
        }
    }

    fn detect_exe(&mut self, exe: &Path) -> Option<Detection> {
        if let Some(cached) = self.exe_cache.get(exe) {
            return cached.clone();
        }
        if self.exe_cache.len() >= EXE_CACHE_LIMIT {
            self.exe_cache.clear();
        }
        let detection =
            detect_emulator_markers(exe).or_else(|| detect_steam_install(exe, &mut self.manifests));
        self.exe_cache.insert(exe.to_path_buf(), detection.clone());
        detection
    }

    fn collect_candidates(&mut self) -> Vec<Candidate> {
        self.system.refresh_processes_specifics(
            ProcessesToUpdate::All,
            true,
            ProcessRefreshKind::nothing().with_exe(UpdateKind::OnlyIfNotSet),
        );

        let processes: Vec<(u32, PathBuf, u64)> = self
            .system
            .processes()
            .iter()
            .filter_map(|(pid, process)| {
                let exe = process.exe()?.to_path_buf();
                Some((pid.as_u32(), exe, process.start_time()))
            })
            .collect();

        let mut candidates = Vec::new();
        for (pid, exe, start_time) in processes {
            if pid == self.own_pid || is_ignored_exe(&exe, &self.windows_dir) {
                continue;
            }
            if let Some(detection) = self.detect_exe(&exe) {
                candidates.push(Candidate {
                    detection,
                    pid,
                    exe_path: exe,
                    start_time,
                });
            }
        }
        candidates
    }

    fn resolve_name(
        &mut self,
        app_id: u32,
        library_name: Option<String>,
        root_dir: &Path,
        language: &str,
        app_handle: &tauri::AppHandle,
    ) -> String {
        if let Some(name) = library_name.filter(|n| is_valid_game_name(n)) {
            self.name_cache.insert(app_id, name.clone());
            return name;
        }
        if let Some(name) = self.name_cache.get(&app_id) {
            if is_valid_game_name(name) {
                return name.clone();
            }
        }
        if let Some(disk_name) = crate::lookup_cached_game_name(app_handle, app_id) {
            self.name_cache.insert(app_id, disk_name.clone());
            return disk_name;
        }
        if self.api_lookup_attempted.insert(app_id) {
            let res = tauri::async_runtime::block_on(
                crate::achievements::steam::fetch_app_name_by_appid(app_id, language),
            );
            if let Ok(Some(name)) = res {
                if is_valid_game_name(&name) {
                    crate::cache_resolved_game_name(app_handle, app_id, &name);
                    self.name_cache.insert(app_id, name.clone());
                    return name;
                }
            }

            // Plan B: fallback to SteamGridDB if configured
            let sgdb_key = app_handle
                .state::<AppState>()
                .sgdb_api_key
                .lock()
                .map(|k| k.clone())
                .unwrap_or_default();
            if !sgdb_key.trim().is_empty() {
                let sgdb_res = tauri::async_runtime::block_on(async {
                    let client = reqwest::Client::builder()
                        .timeout(std::time::Duration::from_secs(10))
                        .build()
                        .ok()?;
                    crate::fetch_sgdb_game_name(&client, app_id, &sgdb_key).await
                });
                if let Some(sgdb_name) = sgdb_res {
                    if is_valid_game_name(&sgdb_name) {
                        crate::cache_resolved_game_name(app_handle, app_id, &sgdb_name);
                        self.name_cache.insert(app_id, sgdb_name.clone());
                        return sgdb_name;
                    }
                }
            }
        }
        folder_name_fallback(root_dir).unwrap_or_else(|| format!("AppID {}", app_id))
    }

    /// Runs one scan and returns `Some(new_state)` when the running game changed.
    fn scan(&mut self, app_handle: &tauri::AppHandle) -> Option<Option<RunningGame>> {
        let candidates = self.collect_candidates();

        // Library snapshot: appid -> (game id, name)
        let (library, language): (HashMap<u32, (String, String)>, String) = {
            let state = app_handle.state::<AppState>();
            let library = state
                .games
                .lock()
                .map(|games| {
                    games
                        .iter()
                        .filter_map(|g| g.steam_id.map(|sid| (sid, (g.id.clone(), g.name.clone()))))
                        .collect()
                })
                .unwrap_or_default();
            let language = state
                .language
                .lock()
                .map(|l| l.clone())
                .unwrap_or_else(|_| "fr".to_string());
            (library, language)
        };

        // Group processes by AppID; keep the earliest-started process of each group.
        let mut by_app: HashMap<u32, Candidate> = HashMap::new();
        for candidate in candidates {
            by_app
                .entry(candidate.detection.app_id)
                .and_modify(|existing| {
                    if candidate.start_time < existing.start_time {
                        *existing = candidate.clone();
                    }
                })
                .or_insert(candidate);
        }

        // Stickiness: keep the current game as long as one of its processes is alive.
        let chosen = self
            .current
            .as_ref()
            .and_then(|cur| by_app.get(&cur.app_id).cloned())
            .or_else(|| {
                by_app
                    .values()
                    .max_by_key(|c| (library.contains_key(&c.detection.app_id), c.start_time))
                    .cloned()
            });

        let next = chosen.map(|c| {
            let app_id = c.detection.app_id;
            let library_entry = library.get(&app_id).cloned();
            let name = self.resolve_name(
                app_id,
                library_entry.as_ref().map(|(_, n)| n.clone()),
                &c.detection.root_dir,
                &language,
                app_handle,
            );
            RunningGame {
                app_id,
                game_id: library_entry.map(|(id, _)| id),
                name,
                exe_path: c.exe_path.to_string_lossy().to_string(),
                pid: c.pid,
                started_at: c.start_time,
                detected_via: c.detection.via.to_string(),
            }
        });

        if next != self.current {
            self.current = next.clone();
            Some(next)
        } else {
            None
        }
    }
}

#[derive(Clone, Debug)]
struct ActiveSession {
    appid: u32,
    game_id: String,
    started_at: u64,
    started_instant: Instant,
    initial_unlocked_keys: HashSet<String>,
    snapshot_resolved: bool,
}

fn current_unix_secs() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0)
}

fn find_game_index(games: &[Game], game_id: &str, app_id: u32) -> Option<usize> {
    games
        .iter()
        .position(|g| g.id == game_id)
        .or_else(|| games.iter().position(|g| app_id > 0 && g.steam_id == Some(app_id)))
}

fn is_achievement_unlocked(ach: &Achievement) -> bool {
    ach.unlocked || ach.unlocked_time.is_some_and(|t| t > 0)
}

fn achievement_key(ach: &Achievement) -> &str {
    let key = ach.key.trim();
    if key.is_empty() {
        ach.name.trim()
    } else {
        key
    }
}

/// Returns deduplicated achievement keys unlocked during `[started_at, ended_at]`.
/// When `baseline_known` is `true`, compares against `initial_unlocked_keys` or matches
/// unlock timestamps inside the session window (with a 5-second clock-skew grace period);
/// when `false`, only timestamped unlocks inside the window are included.
fn collect_unlocked_during_session<'a>(
    achievements: impl IntoIterator<Item = &'a Achievement>,
    started_at: u64,
    ended_at: u64,
    initial_unlocked_keys: &HashSet<String>,
    baseline_known: bool,
) -> Vec<String> {
    let mut seen = HashSet::new();
    let mut unlocked = Vec::new();
    let window_end = ended_at.saturating_add(5);

    for ach in achievements {
        if !is_achievement_unlocked(ach) {
            continue;
        }
        let raw_key = achievement_key(ach);
        if raw_key.is_empty() {
            continue;
        }
        let normalized = raw_key.to_lowercase();
        let unlocked_in_window = ach
            .unlocked_time
            .is_some_and(|t| t > 0 && t >= started_at && t <= window_end);
        let is_new_unlock = if baseline_known {
            !initial_unlocked_keys.contains(&normalized) || unlocked_in_window
        } else {
            unlocked_in_window
        };
        if is_new_unlock && seen.insert(normalized) {
            unlocked.push(raw_key.to_string());
        }
    }

    unlocked
}

fn snapshot_unlocked_keys(
    app: &tauri::AppHandle,
    game_id: &str,
    appid: u32,
) -> Option<HashSet<String>> {
    let state = app.state::<AppState>();
    let games = state.games.lock().ok()?;
    let idx = find_game_index(&games, game_id, appid)?;
    let keys = games[idx]
        .achievements
        .iter()
        .filter(|ach| is_achievement_unlocked(ach))
        .map(|ach| achievement_key(ach).to_lowercase())
        .filter(|k| !k.is_empty())
        .collect();
    Some(keys)
}

fn finalize_active_session(app: &tauri::AppHandle, session: ActiveSession) {
    let duration_seconds = session.started_instant.elapsed().as_secs();
    if duration_seconds < MIN_SESSION_DURATION_SECS {
        return;
    }

    let ended_at = current_unix_secs();

    // Read achievements and drop the `state.games` lock before reading/writing `user_data`.
    let (resolved_game_id, unlocked_achievements) = app
        .state::<AppState>()
        .games
        .lock()
        .ok()
        .and_then(|games| {
            let idx = find_game_index(&games, &session.game_id, session.appid)?;
            let game = &games[idx];
            let unlocked = collect_unlocked_during_session(
                &game.achievements,
                session.started_at,
                ended_at,
                &session.initial_unlocked_keys,
                session.snapshot_resolved,
            );
            Some((game.id.clone(), unlocked))
        })
        .unwrap_or_else(|| (session.game_id.clone(), Vec::new()));

    let play_session = PlaySession {
        appid: session.appid,
        game_id: resolved_game_id,
        started_at: session.started_at,
        duration_seconds,
        unlocked_achievements,
    };

    if let Err(err) = crate::user_data::update(app, |user_data| {
        user_data.play_sessions.push(play_session.clone());
        Ok(())
    }) {
        log::warn!("[game-detector] Failed to save play session: {}", err);
    }

    if let Err(err) = app.emit(PLAY_SESSION_RECORDED_EVENT, &play_session) {
        log::warn!("[game-detector] Failed to emit play session event: {}", err);
    }
}

fn flush_playtime(app_handle: &tauri::AppHandle, game_id: &str, app_id: u32, delta_secs: u64) {
    if delta_secs == 0 {
        return;
    }

    let in_mem_total = app_handle
        .state::<AppState>()
        .games
        .lock()
        .ok()
        .and_then(|mut games| {
            let idx = find_game_index(&games, game_id, app_id)?;
            games[idx].playtime_seconds = games[idx].playtime_seconds.saturating_add(delta_secs);
            Some(games[idx].playtime_seconds)
        })
        .unwrap_or(0);

    let total = match crate::user_data::update(app_handle, |user_data| {
        let entry = user_data
            .playtime_seconds
            .entry(game_id.to_string())
            .or_insert(0);
        *entry = entry.saturating_add(delta_secs).max(in_mem_total);
        Ok(*entry)
    }) {
        Ok(total) => total,
        Err(err) => {
            log::warn!("[game-detector] Failed to flush playtime: {}", err);
            return;
        }
    };

    if let Ok(mut games) = app_handle.state::<AppState>().games.lock() {
        if let Some(idx) = find_game_index(&games, game_id, app_id) {
            games[idx].playtime_seconds = total;
        }
    }

    let _ = app_handle.emit(
        PLAYTIME_UPDATED_EVENT,
        serde_json::json!({
            "game_id": game_id,
            "playtime_seconds": total
        }),
    );
}

pub fn start(app_handle: tauri::AppHandle) {
    std::thread::spawn(move || {
        let mut detector = Detector::new();
        let mut last_tick = Instant::now();
        let mut unsaved_secs = 0u64;
        let mut active_game_key: Option<(String, u32)> = None;
        let mut active_session: Option<ActiveSession> = None;

        loop {
            let elapsed = last_tick.elapsed().as_secs();
            last_tick = Instant::now();

            if let Some((ref prev_id, prev_appid)) = active_game_key {
                unsaved_secs = unsaved_secs.saturating_add(elapsed);
                if unsaved_secs >= 15 {
                    flush_playtime(&app_handle, prev_id, prev_appid, unsaved_secs);
                    unsaved_secs = 0;
                }
            }

            if let Some(next) = detector.scan(&app_handle) {
                let next_key = next.as_ref().map(|game| {
                    let gid = game
                        .game_id
                        .clone()
                        .unwrap_or_else(|| format!("steam_{}", game.app_id));
                    (gid, game.app_id)
                });

                if next_key != active_game_key {
                    // Flush remaining seconds and finalize the previous session before switching.
                    if let Some((ref prev_id, prev_appid)) = active_game_key {
                        if unsaved_secs > 0 {
                            flush_playtime(&app_handle, prev_id, prev_appid, unsaved_secs);
                            unsaved_secs = 0;
                        }
                    }
                    if let Some(prev_session) = active_session.take() {
                        finalize_active_session(&app_handle, prev_session);
                    }

                    match (&next, next_key) {
                        (Some(game), Some((gid, app_id))) => {
                            log::info!(
                                "[game-detector] Running: '{}' (AppID {}, pid {}, via {})",
                                game.name,
                                game.app_id,
                                game.pid,
                                game.detected_via
                            );
                            let snapshot = snapshot_unlocked_keys(&app_handle, &gid, app_id);
                            let snapshot_resolved = snapshot.is_some();
                            active_session = Some(ActiveSession {
                                appid: app_id,
                                game_id: gid.clone(),
                                started_at: current_unix_secs(),
                                started_instant: Instant::now(),
                                initial_unlocked_keys: snapshot.unwrap_or_default(),
                                snapshot_resolved,
                            });
                            active_game_key = Some((gid, app_id));
                        }
                        _ => {
                            log::info!("[game-detector] No game running");
                            active_game_key = None;
                        }
                    }
                }
                if let Ok(mut current) = app_handle.state::<AppState>().running_game.lock() {
                    *current = next.clone();
                }
                let _ = app_handle.emit(RUNNING_GAME_EVENT, &next);
            }

            if let Some(ref mut session) = active_session {
                if !session.snapshot_resolved {
                    if let Some(keys) =
                        snapshot_unlocked_keys(&app_handle, &session.game_id, session.appid)
                    {
                        session.initial_unlocked_keys = keys;
                        session.snapshot_resolved = true;
                    }
                }
            }

            std::thread::sleep(SCAN_INTERVAL);
        }
    });
}

#[tauri::command]
pub fn get_running_game(state: tauri::State<'_, AppState>) -> Option<RunningGame> {
    state.running_game.lock().ok().and_then(|g| g.clone())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn temp_dir(name: &str) -> PathBuf {
        let dir =
            std::env::temp_dir().join(format!("accolade_detector_{}_{}", name, std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        dir
    }

    #[test]
    fn ini_app_id_respects_priority_and_skips_placeholders() {
        let onlinefix = "[Main]\nAppId=480\nRealAppId=1245620\n";
        assert_eq!(
            find_ini_app_id(onlinefix, &["RealAppId", "AppId"]),
            Some(1245620)
        );

        let codex = "[Settings]\n; comment\nAppId = 271590 ; trailing\n";
        assert_eq!(find_ini_app_id(codex, &["AppId"]), Some(271590));

        let only_spacewar = "AppId=480\n";
        assert_eq!(find_ini_app_id(only_spacewar, &["AppId"]), None);

        let creamapi = "[config]\nappid = 1091500\n";
        assert_eq!(find_ini_app_id(creamapi, &["appid"]), Some(1091500));
    }

    #[test]
    fn txt_app_id_parsing() {
        assert_eq!(find_txt_app_id("1245620\r\n"), Some(1245620));
        assert_eq!(find_txt_app_id("  \u{feff}892970"), Some(892970));
        assert_eq!(find_txt_app_id("480"), None);
        assert_eq!(find_txt_app_id("431960"), None); // Wallpaper Engine
        assert_eq!(find_txt_app_id(""), None);
    }

    #[test]
    fn bare_steam_appid_txt_requires_steam_api_dll() {
        let root = temp_dir("bare_txt");
        std::fs::write(root.join("steam_appid.txt"), "2399420").unwrap();
        let exe = root.join("daemon.exe");

        // No steam_api(64).dll -> rejected (like gridcard_agent.exe).
        assert!(detect_emulator_markers(&exe).is_none());

        // With steam_api64.dll next to it -> accepted.
        std::fs::write(root.join("steam_api64.dll"), b"").unwrap();
        let det = detect_emulator_markers(&exe).unwrap();
        assert_eq!(det.app_id, 2399420);
        assert_eq!(det.via, "steam_appid.txt");

        let _ = std::fs::remove_dir_all(&root);
    }

    #[test]
    fn detects_goldberg_marker_from_nested_exe() {
        let root = temp_dir("gse");
        let game = root.join("Elden Ring");
        let bin = game.join("Game");
        std::fs::create_dir_all(game.join("steam_settings")).unwrap();
        std::fs::create_dir_all(&bin).unwrap();
        std::fs::write(
            game.join("steam_settings").join("steam_appid.txt"),
            "1245620",
        )
        .unwrap();

        let det = detect_emulator_markers(&bin.join("eldenring.exe")).unwrap();
        assert_eq!(det.app_id, 1245620);
        assert_eq!(det.via, "goldberg");
        assert_eq!(det.root_dir, game);
        assert_eq!(
            folder_name_fallback(&det.root_dir).as_deref(),
            Some("Elden Ring")
        );

        let _ = std::fs::remove_dir_all(&root);
    }

    #[test]
    fn detects_steam_emu_ini() {
        let root = temp_dir("codex");
        let bin = root.join("MyGame").join("Binaries").join("Win64");
        std::fs::create_dir_all(&bin).unwrap();
        std::fs::write(bin.join("steam_emu.ini"), "[Settings]\nAppId=1174180\n").unwrap();

        let det = detect_emulator_markers(&bin.join("Game-Win64-Shipping.exe")).unwrap();
        assert_eq!(det.app_id, 1174180);
        assert_eq!(det.via, "steam_emu.ini");
        assert_eq!(
            folder_name_fallback(&det.root_dir).as_deref(),
            Some("MyGame")
        );

        let _ = std::fs::remove_dir_all(&root);
    }

    #[test]
    fn detects_legit_steam_install_via_manifest() {
        let root = temp_dir("steam");
        let steamapps = root.join("steamapps");
        let game_dir = steamapps.join("common").join("Hades");
        std::fs::create_dir_all(&game_dir).unwrap();
        std::fs::write(
            steamapps.join("appmanifest_1145360.acf"),
            "\"AppState\"\n{\n\t\"appid\"\t\t\"1145360\"\n\t\"name\"\t\t\"Hades\"\n\t\"installdir\"\t\t\"Hades\"\n}\n",
        )
        .unwrap();

        let mut cache = SteamManifestCache::default();
        let det =
            detect_steam_install(&game_dir.join("x64").join("Hades.exe"), &mut cache).unwrap();
        assert_eq!(det.app_id, 1145360);
        assert_eq!(det.via, "steam");

        // An exe directly in steamapps/common is not a game install.
        assert!(split_steam_library_path(&steamapps.join("common").join("x.exe")).is_none());

        let _ = std::fs::remove_dir_all(&root);
    }

    #[test]
    fn ignores_system_windowsapps_and_helper_processes() {
        let win = Some("c:\\windows\\".to_string());
        assert!(is_ignored_exe(Path::new("C:\\Windows\\explorer.exe"), &win));
        assert!(is_ignored_exe(
            Path::new("D:\\Games\\X\\UnityCrashHandler64.exe"),
            &win
        ));
        assert!(is_ignored_exe(
            Path::new("C:\\Program Files\\WindowsApps\\Pkg\\gridcard_agent.exe"),
            &win
        ));
        assert!(is_ignored_exe(
            Path::new("C:\\Program Files (x86)\\Borderless Gaming\\BorderlessGaming.exe"),
            &win
        ));
        assert!(!is_ignored_exe(
            Path::new("C:\\Program Files (x86)\\Steam\\steamapps\\common\\Hades\\Hades.exe"),
            &win
        ));
        assert!(!is_ignored_exe(Path::new("E:\\Jeux\\JWE3\\JWE3.exe"), &win));
    }

    fn make_test_ach(key: &str, name: &str, unlocked: bool, unlocked_time: Option<u64>) -> Achievement {
        Achievement {
            key: key.to_string(),
            name: name.to_string(),
            unlocked,
            unlocked_time,
            ..Default::default()
        }
    }

    #[test]
    fn collects_unlocked_achievements_by_key_diff_and_timestamp_window() {
        let initial: HashSet<String> = ["ach_old", "ach_reunlocked", "ach_after"]
            .into_iter()
            .map(str::to_string)
            .collect();

        let started_at = 1_000;
        let ended_at = 1_200;

        let achievements = vec![
            // Locked achievement (including Steam's Some(0) placeholder) -> ignored
            make_test_ach("ACH_LOCKED", "Locked", false, Some(0)),
            // Pre-existing unlock outside window -> ignored
            make_test_ach("ACH_OLD", "Old", true, Some(900)),
            // New unlock without timestamp (key diff) -> collected
            make_test_ach("ACH_NEW_NO_TS", "New No TS", true, None),
            // Pre-existing key whose unlock timestamp falls within [started_at, ended_at + 5] -> collected
            make_test_ach("ACH_REUNLOCKED", "Reunlocked", true, Some(1_204)),
            // Pre-existing key unlocked after grace window -> ignored
            make_test_ach("ACH_AFTER", "After", true, Some(1_206)),
            // Duplicate (different casing) -> deduplicated preserving first occurrence
            make_test_ach("ach_new_no_ts", "New No TS", true, Some(1_050)),
            // Fallback to name when key is empty
            make_test_ach("  ", "ACH_FALLBACK_NAME", true, Some(1_100)),
        ];

        let collected = collect_unlocked_during_session(
            &achievements,
            started_at,
            ended_at,
            &initial,
            true,
        );
        assert_eq!(
            collected,
            vec!["ACH_NEW_NO_TS", "ACH_REUNLOCKED", "ACH_FALLBACK_NAME"]
        );
    }

    #[test]
    fn unknown_baseline_ignores_untimestamped_and_catches_in_window_unlocks() {
        let initial = HashSet::new();
        let started_at = 1_000;
        let ended_at = 1_200;

        let achievements = vec![
            // Pre-existing unlock without timestamp or with Some(0) -> ignored
            make_test_ach("ACH_PRE_NONE", "Pre None", true, None),
            make_test_ach("ACH_PRE_ZERO", "Pre Zero", true, Some(0)),
            // Pre-existing unlock before session -> ignored
            make_test_ach("ACH_PRE_OLD", "Pre Old", true, Some(999)),
            // Unlocked during session -> collected
            make_test_ach("ACH_SESSION_START", "Start", true, Some(1_000)),
            make_test_ach("ACH_SESSION_GRACE", "Grace", true, Some(1_205)),
            // Unlocked after grace period -> ignored
            make_test_ach("ACH_AFTER_GRACE", "After", true, Some(1_206)),
        ];

        let collected = collect_unlocked_during_session(
            &achievements,
            started_at,
            ended_at,
            &initial,
            false,
        );
        assert_eq!(
            collected,
            vec!["ACH_SESSION_START", "ACH_SESSION_GRACE"]
        );
    }

    #[test]
    fn find_game_index_prefers_exact_game_id_over_steam_id_fallback() {
        let games = vec![
            Game {
                id: "goldberg_500".to_string(),
                steam_id: Some(500),
                ..Default::default()
            },
            Game {
                id: "steam_500".to_string(),
                steam_id: Some(500),
                ..Default::default()
            },
        ];

        assert_eq!(find_game_index(&games, "steam_500", 500), Some(1));
        assert_eq!(find_game_index(&games, "unknown_500", 500), Some(0));
        assert_eq!(find_game_index(&games, "unknown_999", 999), None);
    }
}
