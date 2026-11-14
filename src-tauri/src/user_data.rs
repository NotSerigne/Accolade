// src-tauri/src/user_data.rs
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::{Mutex, MutexGuard, OnceLock};
use tauri::{AppHandle, Manager};

pub const MIN_SESSION_DURATION_SECS: u64 = 10;
const MAX_SHOWCASE_ITEMS: usize = 3;

#[derive(Serialize, Deserialize, Default, Clone, Debug)]
pub struct ManualAchievementOverride {
    pub unlocked: bool,
    pub unlocked_time: Option<u64>,
}

#[derive(Serialize, Deserialize, Default, Clone, Debug, PartialEq, Eq)]
pub struct ShowcaseAchievementRef {
    pub game_id: String,
    pub ach_key: String,
}

#[derive(Serialize, Deserialize, Default, Clone, Debug, PartialEq, Eq)]
pub struct ProfileShowcase {
    #[serde(default)]
    pub games: Vec<String>,
    #[serde(default)]
    pub achievements: Vec<ShowcaseAchievementRef>,
}

#[derive(Serialize, Deserialize, Default, Clone, Debug, PartialEq, Eq)]
pub struct PlaySession {
    pub appid: u32,
    #[serde(default)]
    pub game_id: String,
    pub started_at: u64,
    pub duration_seconds: u64,
    #[serde(default)]
    pub unlocked_achievements: Vec<String>,
}

#[derive(Serialize, Deserialize, Default, Clone, Debug, PartialEq, Eq)]
pub struct AchievementUserMeta {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub note: Option<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub tags: Vec<String>,
}

impl AchievementUserMeta {
    pub fn is_empty(&self) -> bool {
        self.note.is_none() && self.tags.is_empty()
    }
}

#[derive(Serialize, Deserialize, Default, Clone)]
pub struct UserData {
    pub favorites: HashSet<String>,
    pub tags: HashMap<String, Vec<String>>,
    /// Per-game ordered list of achievement keys pinned to the in-game HUD tracker.
    #[serde(default)]
    pub pinned_achievements: HashMap<String, Vec<String>>,
    /// Per-game tracked playtime in seconds.
    #[serde(default)]
    pub playtime_seconds: HashMap<String, u64>,
    /// Per-game manual achievement unlock/date overrides (game_id -> ach_key_lowercase -> override).
    #[serde(default)]
    pub manual_achievements: HashMap<String, HashMap<String, ManualAchievementOverride>>,
    /// Per-game personal notes and marker tags per achievement (game_id -> ach_key_lowercase -> meta).
    #[serde(default)]
    pub achievement_meta: HashMap<String, HashMap<String, AchievementUserMeta>>,
    /// User-curated games and achievements showcased on the profile view.
    #[serde(default)]
    pub profile_showcase: ProfileShowcase,
    /// Recorded play sessions detected during gameplay.
    #[serde(default)]
    pub play_sessions: Vec<PlaySession>,
}

/// Trims, deduplicates (preserving order), drops empty entries, and caps showcase games and
/// achievements at `MAX_SHOWCASE_ITEMS` (3).
pub fn sanitize_profile_showcase(showcase: ProfileShowcase) -> ProfileShowcase {
    let mut seen_games = HashSet::new();
    let mut games = Vec::with_capacity(showcase.games.len().min(MAX_SHOWCASE_ITEMS));
    for raw_id in showcase.games {
        let game_id = raw_id.trim();
        if game_id.is_empty() {
            continue;
        }
        let owned = game_id.to_string();
        if seen_games.insert(owned.clone()) {
            games.push(owned);
            if games.len() >= MAX_SHOWCASE_ITEMS {
                break;
            }
        }
    }

    let mut seen_achs = HashSet::new();
    let mut achievements = Vec::with_capacity(showcase.achievements.len().min(MAX_SHOWCASE_ITEMS));
    for item in showcase.achievements {
        let game_id = item.game_id.trim();
        let ach_key = item.ach_key.trim();
        if game_id.is_empty() || ach_key.is_empty() {
            continue;
        }
        let dedup_key = (game_id.to_string(), ach_key.to_lowercase());
        if seen_achs.insert(dedup_key) {
            achievements.push(ShowcaseAchievementRef {
                game_id: game_id.to_string(),
                ach_key: ach_key.to_string(),
            });
            if achievements.len() >= MAX_SHOWCASE_ITEMS {
                break;
            }
        }
    }

    ProfileShowcase {
        games,
        achievements,
    }
}

/// Normalizes a single achievement marker tag, mapping known FR/EN aliases to canonical keys.
pub fn normalize_achievement_tag(raw: &str) -> Option<String> {
    let trimmed = raw.trim();
    if trimmed.is_empty() {
        return None;
    }
    match trimmed.to_lowercase().as_str() {
        "manquable" | "missable" => Some("missable".to_string()),
        "multi" | "multiplayer" | "coop" => Some("multi".to_string()),
        "ng+" | "ngplus" | "new game+" | "newgame+" => Some("ngplus".to_string()),
        "buggué" | "buggue" | "buggy" | "glitched" => Some("buggy".to_string()),
        _ => Some(trimmed.to_string()),
    }
}

/// Normalizes and deduplicates a list of achievement tags while preserving order.
pub fn normalize_achievement_tags(tags: &[String]) -> Vec<String> {
    let mut seen = HashSet::new();
    let mut out = Vec::new();
    for raw in tags {
        if let Some(tag) = normalize_achievement_tag(raw) {
            if seen.insert(tag.clone()) {
                out.push(tag);
            }
        }
    }
    out
}

/// Trims an optional personal achievement note, returning `None` when empty or whitespace-only.
pub fn normalize_achievement_note(note: Option<&str>) -> Option<String> {
    note.map(str::trim)
        .filter(|s| !s.is_empty())
        .map(str::to_string)
}

impl UserData {
    fn mutate_achievement_meta(
        &mut self,
        game_id: &str,
        ach_key: &str,
        create_if_missing: bool,
        mutate: impl FnOnce(&mut AchievementUserMeta),
    ) {
        let key_lower = ach_key.trim().to_lowercase();
        if game_id.trim().is_empty() || key_lower.is_empty() {
            return;
        }

        if create_if_missing {
            let game_meta = self.achievement_meta.entry(game_id.to_string()).or_default();
            let entry = game_meta.entry(key_lower).or_default();
            mutate(entry);
        } else if let Some(game_meta) = self.achievement_meta.get_mut(game_id) {
            if let Some(entry) = game_meta.get_mut(&key_lower) {
                mutate(entry);
                if entry.is_empty() {
                    game_meta.remove(&key_lower);
                }
            }
            if game_meta.is_empty() {
                self.achievement_meta.remove(game_id);
            }
        }
    }

    /// Sets or clears the personal note for `(game_id, ach_key)` and prunes empty metadata entries.
    pub fn set_achievement_note(
        &mut self,
        game_id: &str,
        ach_key: &str,
        note: Option<&str>,
    ) -> Option<String> {
        if game_id.trim().is_empty() || ach_key.trim().is_empty() {
            return None;
        }
        let normalized = normalize_achievement_note(note);
        self.mutate_achievement_meta(game_id, ach_key, normalized.is_some(), |entry| {
            entry.note = normalized.clone();
        });
        normalized
    }

    /// Sets or clears the marker tags for `(game_id, ach_key)` and prunes empty metadata entries.
    pub fn set_achievement_tags(
        &mut self,
        game_id: &str,
        ach_key: &str,
        tags: &[String],
    ) -> Vec<String> {
        if game_id.trim().is_empty() || ach_key.trim().is_empty() {
            return Vec::new();
        }
        let normalized = normalize_achievement_tags(tags);
        self.mutate_achievement_meta(game_id, ach_key, !normalized.is_empty(), |entry| {
            entry.tags = normalized.clone();
        });
        normalized
    }
}

static USER_DATA_IO_LOCK: OnceLock<Mutex<()>> = OnceLock::new();

fn io_lock() -> MutexGuard<'static, ()> {
    USER_DATA_IO_LOCK
        .get_or_init(|| Mutex::new(()))
        .lock()
        .unwrap_or_else(|e| e.into_inner())
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

fn load_unlocked(path: &Path) -> UserData {
    if let Ok(content) = fs::read_to_string(path) {
        serde_json::from_str(&content).unwrap_or_default()
    } else {
        UserData::default()
    }
}

fn save_unlocked(path: &Path, data: &UserData) -> Result<(), String> {
    if let Some(parent) = path.parent().filter(|p| !p.as_os_str().is_empty()) {
        fs::create_dir_all(parent).map_err(|e| e.to_string())?;
    }
    let content = serde_json::to_string_pretty(data).map_err(|e| e.to_string())?;
    let file_name = path
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or("user_data.json");
    let tmp_path = path.with_file_name(format!("{}.tmp.{}", file_name, std::process::id()));
    if let Err(err) = fs::write(&tmp_path, content) {
        let _ = fs::remove_file(&tmp_path);
        return Err(err.to_string());
    }
    if let Err(err) = fs::rename(&tmp_path, path) {
        let _ = fs::remove_file(&tmp_path);
        return Err(err.to_string());
    }
    Ok(())
}

fn update_at_path<F, R>(path: &Path, f: F) -> Result<R, String>
where
    F: FnOnce(&mut UserData) -> Result<R, String>,
{
    let _guard = io_lock();
    let mut data = load_unlocked(path);
    let res = f(&mut data)?;
    save_unlocked(path, &data)?;
    Ok(res)
}

pub fn load_user_data(app_handle: &AppHandle) -> UserData {
    let _guard = io_lock();
    let path = get_user_data_path(app_handle);
    load_unlocked(&path)
}

pub fn save_user_data(app_handle: &AppHandle, data: &UserData) -> Result<(), String> {
    let _guard = io_lock();
    let path = get_user_data_path(app_handle);
    save_unlocked(&path, data)
}

/// Atomically loads `UserData`, applies `f`, and persists changes under `USER_DATA_IO_LOCK`.
pub fn update<F, R>(app: &AppHandle, f: F) -> Result<R, String>
where
    F: FnOnce(&mut UserData) -> Result<R, String>,
{
    let path = get_user_data_path(app);
    update_at_path(&path, f)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn normalizes_achievement_tags_aliases_whitespace_and_custom() {
        assert_eq!(normalize_achievement_tag(""), None);
        assert_eq!(normalize_achievement_tag("   \t\n "), None);

        assert_eq!(
            normalize_achievement_tag("  Manquable "),
            Some("missable".to_string())
        );
        assert_eq!(
            normalize_achievement_tag("MISSABLE"),
            Some("missable".to_string())
        );
        assert_eq!(
            normalize_achievement_tag("MultiPlayer"),
            Some("multi".to_string())
        );
        assert_eq!(
            normalize_achievement_tag("coop"),
            Some("multi".to_string())
        );
        assert_eq!(
            normalize_achievement_tag("  NG+ "),
            Some("ngplus".to_string())
        );
        assert_eq!(
            normalize_achievement_tag("New Game+"),
            Some("ngplus".to_string())
        );
        assert_eq!(
            normalize_achievement_tag("newgame+"),
            Some("ngplus".to_string())
        );
        assert_eq!(
            normalize_achievement_tag("BUGGUÉ"),
            Some("buggy".to_string())
        );
        assert_eq!(
            normalize_achievement_tag("buggue"),
            Some("buggy".to_string())
        );
        assert_eq!(
            normalize_achievement_tag("Glitched"),
            Some("buggy".to_string())
        );
        assert_eq!(
            normalize_achievement_tag("  SpeedRun  "),
            Some("SpeedRun".to_string())
        );

        let deduped = normalize_achievement_tags(&[
            "  ".to_string(),
            "Manquable".to_string(),
            "missable".to_string(),
            "Custom".to_string(),
            "coop".to_string(),
            "Custom".to_string(),
            "multiplayer".to_string(),
            "New Game+".to_string(),
        ]);
        assert_eq!(deduped, vec!["missable", "Custom", "multi", "ngplus"]);
    }

    #[test]
    fn sets_updates_and_clears_notes_with_case_insensitive_keys_and_pruning() {
        let mut data = UserData::default();

        assert_eq!(data.set_achievement_note("  ", "ACH_1", Some("x")), None);
        assert_eq!(data.set_achievement_note("steam_1", "   ", Some("x")), None);
        assert!(data.achievement_meta.is_empty());

        let saved = data.set_achievement_note(
            "steam_1",
            "  ACH_Boss ",
            Some("  Save before Chapter 4  "),
        );
        assert_eq!(saved.as_deref(), Some("Save before Chapter 4"));
        assert_eq!(
            data.achievement_meta["steam_1"]["ach_boss"].note.as_deref(),
            Some("Save before Chapter 4")
        );

        let updated = data.set_achievement_note("steam_1", "ach_boss", Some("Updated note"));
        assert_eq!(updated.as_deref(), Some("Updated note"));
        assert_eq!(
            data.achievement_meta["steam_1"]["ach_boss"].note.as_deref(),
            Some("Updated note")
        );

        assert_eq!(
            data.set_achievement_note("steam_1", "ACH_BOSS", Some("   ")),
            None
        );
        assert!(data.achievement_meta.is_empty());
    }

    #[test]
    fn sets_dedups_and_clears_tags_while_preserving_note_until_both_empty() {
        let mut data = UserData::default();

        assert!(data
            .set_achievement_tags("", "ACH_1", &["missable".to_string()])
            .is_empty());
        assert!(data
            .set_achievement_tags("steam_1", " ", &["missable".to_string()])
            .is_empty());
        assert!(data.achievement_meta.is_empty());

        data.set_achievement_note("steam_1", "ACH_1", Some("Requires co-op partner"));
        let tags = data.set_achievement_tags(
            "steam_1",
            " ach_1 ",
            &[
                "Manquable".to_string(),
                "missable".to_string(),
                "coop".to_string(),
                "".to_string(),
            ],
        );
        assert_eq!(tags, vec!["missable", "multi"]);
        assert_eq!(
            data.achievement_meta["steam_1"]["ach_1"].note.as_deref(),
            Some("Requires co-op partner")
        );

        let cleared_tags = data.set_achievement_tags("steam_1", "ACH_1", &["  ".to_string()]);
        assert!(cleared_tags.is_empty());
        assert!(data.achievement_meta["steam_1"]["ach_1"].tags.is_empty());
        assert_eq!(
            data.achievement_meta["steam_1"]["ach_1"].note.as_deref(),
            Some("Requires co-op partner")
        );

        assert_eq!(data.set_achievement_note("steam_1", "ACH_1", None), None);
        assert!(data.achievement_meta.is_empty());
    }

    #[test]
    fn deserializes_legacy_user_data_without_achievement_meta() {
        let legacy_json = r#"{
            "favorites": ["steam_420"],
            "tags": { "steam_420": ["RPG"] },
            "pinned_achievements": { "steam_420": ["ACH_START"] },
            "playtime_seconds": { "steam_420": 3600 },
            "manual_achievements": {}
        }"#;
        let parsed: UserData = serde_json::from_str(legacy_json).expect("legacy json should parse");
        assert!(parsed.favorites.contains("steam_420"));
        assert!(parsed.achievement_meta.is_empty());
    }

    #[test]
    fn profile_showcase_serialization_round_trip() {
        let showcase = ProfileShowcase {
            games: vec!["steam_420".to_string(), "steam_1245620".to_string()],
            achievements: vec![ShowcaseAchievementRef {
                game_id: "steam_1245620".to_string(),
                ach_key: "ACH_ELDEN_LORD".to_string(),
            }],
        };
        let json = serde_json::to_string(&showcase).expect("showcase should serialize");
        let parsed: ProfileShowcase =
            serde_json::from_str(&json).expect("showcase should deserialize");
        assert_eq!(parsed, showcase);
    }

    #[test]
    fn sanitizes_profile_showcase_trim_filter_dedup_and_truncate() {
        let raw = ProfileShowcase {
            games: vec![
                "   ".to_string(),
                " steam_1 ".to_string(),
                "steam_2".to_string(),
                "steam_1".to_string(),
                "".to_string(),
                " steam_3 ".to_string(),
                "steam_4".to_string(),
            ],
            achievements: vec![
                ShowcaseAchievementRef {
                    game_id: "  ".to_string(),
                    ach_key: "ACH_1".to_string(),
                },
                ShowcaseAchievementRef {
                    game_id: "steam_1".to_string(),
                    ach_key: "\t ".to_string(),
                },
                ShowcaseAchievementRef {
                    game_id: " steam_1 ".to_string(),
                    ach_key: " ACH_First ".to_string(),
                },
                ShowcaseAchievementRef {
                    game_id: "steam_1".to_string(),
                    ach_key: "ach_first".to_string(),
                },
                ShowcaseAchievementRef {
                    game_id: "steam_2".to_string(),
                    ach_key: "ACH_First".to_string(),
                },
                ShowcaseAchievementRef {
                    game_id: "steam_3".to_string(),
                    ach_key: "ACH_Three".to_string(),
                },
                ShowcaseAchievementRef {
                    game_id: "steam_4".to_string(),
                    ach_key: "ACH_Four".to_string(),
                },
            ],
        };

        let sanitized = sanitize_profile_showcase(raw);
        assert_eq!(sanitized.games, vec!["steam_1", "steam_2", "steam_3"]);
        assert_eq!(
            sanitized.achievements,
            vec![
                ShowcaseAchievementRef {
                    game_id: "steam_1".to_string(),
                    ach_key: "ACH_First".to_string(),
                },
                ShowcaseAchievementRef {
                    game_id: "steam_2".to_string(),
                    ach_key: "ACH_First".to_string(),
                },
                ShowcaseAchievementRef {
                    game_id: "steam_3".to_string(),
                    ach_key: "ACH_Three".to_string(),
                },
            ]
        );
    }

    #[test]
    fn play_session_serialization_round_trip() {
        let session = PlaySession {
            appid: 1245620,
            game_id: "steam_1245620".to_string(),
            started_at: 1_700_000_000,
            duration_seconds: 1842,
            unlocked_achievements: vec!["ACH_01".to_string(), "ACH_02".to_string()],
        };
        let json = serde_json::to_string(&session).expect("play session should serialize");
        let parsed: PlaySession =
            serde_json::from_str(&json).expect("play session should deserialize");
        assert_eq!(parsed, session);
    }

    #[test]
    fn deserializes_legacy_user_data_without_showcase_and_sessions() {
        let legacy_json = r#"{
            "favorites": ["steam_420"],
            "tags": { "steam_420": ["RPG"] },
            "pinned_achievements": {},
            "playtime_seconds": { "steam_420": 3600 },
            "manual_achievements": {},
            "achievement_meta": {}
        }"#;
        let parsed: UserData = serde_json::from_str(legacy_json).expect("legacy json should parse");
        assert!(parsed.favorites.contains("steam_420"));
        assert_eq!(parsed.profile_showcase, ProfileShowcase::default());
        assert!(parsed.play_sessions.is_empty());
    }

    #[test]
    fn save_unlocked_writes_atomically_without_leaving_temp_file() {
        let dir = std::env::temp_dir().join(format!("accolade_ud_atomic_{}", std::process::id()));
        let _ = fs::remove_dir_all(&dir);
        let path = dir.join("nested").join("user_data.json");

        let mut initial = UserData::default();
        initial.favorites.insert("steam_100".to_string());
        save_unlocked(&path, &initial).expect("initial save should succeed");

        let mut updated = UserData::default();
        updated.favorites.insert("steam_200".to_string());
        save_unlocked(&path, &updated).expect("overwrite save should succeed");

        let loaded = load_unlocked(&path);
        assert!(!loaded.favorites.contains("steam_100"));
        assert!(loaded.favorites.contains("steam_200"));

        let tmp_path = path.with_file_name(format!("user_data.json.tmp.{}", std::process::id()));
        assert!(!tmp_path.exists());

        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn update_at_path_serializes_concurrent_rmw_and_skips_save_on_err() {
        let dir = std::env::temp_dir().join(format!("accolade_ud_rmw_{}", std::process::id()));
        let _ = fs::remove_dir_all(&dir);
        let path = std::sync::Arc::new(dir.join("user_data.json"));

        std::thread::scope(|s| {
            for i in 0..8u64 {
                let p = std::sync::Arc::clone(&path);
                s.spawn(move || {
                    update_at_path(&p, |data| {
                        data.play_sessions.push(PlaySession {
                            appid: 100 + i as u32,
                            game_id: format!("steam_{}", 100 + i),
                            started_at: 1_700_000_000 + i,
                            duration_seconds: 60,
                            unlocked_achievements: Vec::new(),
                        });
                        Ok(())
                    })
                    .expect("concurrent update should succeed");
                });
            }
        });

        let aborted = update_at_path(&path, |data| {
            data.favorites.insert("should_not_persist".to_string());
            Err::<(), String>("aborted".to_string())
        });
        assert_eq!(aborted, Err("aborted".to_string()));

        let loaded = load_unlocked(&path);
        assert_eq!(loaded.play_sessions.len(), 8);
        assert!(!loaded.favorites.contains("should_not_persist"));

        let _ = fs::remove_dir_all(&dir);
    }
}
