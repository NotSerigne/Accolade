// src-tauri/src/achievements/models.rs
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

const fn default_true() -> bool {
    true
}

pub(crate) fn deserialize_bool_or_int<'de, D>(deserializer: D) -> Result<bool, D::Error>
where
    D: serde::Deserializer<'de>,
{
    let val = serde_json::Value::deserialize(deserializer)?;
    Ok(match val {
        serde_json::Value::Bool(b) => b,
        serde_json::Value::Number(n) => {
            n.as_i64().is_some_and(|i| i != 0) || n.as_u64().is_some_and(|u| u != 0)
        }
        serde_json::Value::String(s) => {
            let lower = s.trim().to_ascii_lowercase();
            matches!(lower.as_str(), "true" | "1" | "yes")
        }
        _ => false,
    })
}

pub(crate) fn deserialize_bool_or_int_default_true<'de, D>(
    deserializer: D,
) -> Result<bool, D::Error>
where
    D: serde::Deserializer<'de>,
{
    let val = serde_json::Value::deserialize(deserializer)?;
    Ok(match val {
        serde_json::Value::Null => true,
        serde_json::Value::Bool(b) => b,
        serde_json::Value::Number(n) => {
            n.as_i64().is_some_and(|i| i != 0) || n.as_u64().is_some_and(|u| u != 0)
        }
        serde_json::Value::String(s) => {
            let lower = s.trim().to_ascii_lowercase();
            matches!(lower.as_str(), "true" | "1" | "yes")
        }
        _ => true,
    })
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct AchievementGroup {
    #[serde(default)]
    pub groupid: u32,
    #[serde(default)]
    pub localized_name: String,
    #[serde(default)]
    pub dlcappid: u32,
    #[serde(default, deserialize_with = "deserialize_bool_or_int")]
    pub archived: bool,
    #[serde(default, deserialize_with = "deserialize_bool_or_int")]
    pub developeronly: bool,
    #[serde(
        default = "default_true",
        deserialize_with = "deserialize_bool_or_int_default_true"
    )]
    pub ispublic: bool,
    #[serde(default)]
    pub order: i32,
}

impl Default for AchievementGroup {
    fn default() -> Self {
        Self {
            groupid: 0,
            localized_name: String::new(),
            dlcappid: 0,
            archived: false,
            developeronly: false,
            ispublic: true,
            order: 0,
        }
    }
}

#[derive(Debug, Serialize, Deserialize, Clone, Default)]
pub struct Achievement {
    pub key: String,
    pub name: String,
    pub unlocked: bool,
    pub icon: String,
    pub icon_gray: String,
    pub unlocked_time: Option<u64>,
    pub rarity: String,
    pub completionpercentage: String,
    pub desc: String,
    pub hidden: bool,
    /// Current value of a progress-based achievement (e.g. 37 for "Collect 100 items").
    /// `None` when the emulator does not expose any progress for this achievement.
    #[serde(default)]
    pub progress: Option<f64>,
    /// Target value of a progress-based achievement (e.g. 100). Always > 0 when set.
    #[serde(default)]
    pub max_progress: Option<f64>,
    /// Steam achievement set group ID (`0` = implicit Base Game set).
    #[serde(default)]
    pub groupid: u32,
    /// True when this achievement or its group is marked archived by the developer.
    #[serde(default)]
    pub archived: bool,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Default)]
pub enum Emulator {
    #[default]
    Goldberg,
    Empress,
    Codex,
    OnlineFix,
    Rune,
    Tenoke,
    Flt,
    Ali213,
    SmartSteamEmu,
    CreamApi,
    Steam,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Default)]
#[serde(tag = "type", content = "value")]
pub enum SourceType {
    Emulator(Emulator),
    RetroAchievements,
    #[default]
    Custom,
}

#[derive(Debug, Serialize, Deserialize, Clone, Default)]
pub struct Game {
    pub name: String,
    pub id: String,
    pub steam_id: Option<u32>,
    pub game_icon: String,
    pub steamgrid_icon_url: String,
    pub header_image_url: String,
    pub background_image_url: String,
    pub achievements_total: u32,
    pub achievements: Vec<Achievement>,
    pub path_buf: Option<String>,
    pub source: SourceType,
    #[serde(default)]
    pub genres: Vec<String>,
    #[serde(default)]
    pub is_favorite: bool,
    #[serde(default)]
    pub tags: Vec<String>,
    /// Total tracked playtime in seconds (local detector + Steam playtime).
    #[serde(default)]
    pub playtime_seconds: u64,
    /// Achievement sets / groups returned by Steam (base game, DLCs, etc.).
    #[serde(default)]
    pub achievement_groups: Vec<AchievementGroup>,
}

/// Filters out developer-only and non-public achievement groups and sorts them by `(order, groupid)`.
pub fn filter_public_groups(groups: &[AchievementGroup]) -> Vec<AchievementGroup> {
    let mut public: Vec<AchievementGroup> = groups
        .iter()
        .filter(|g| !g.developeronly && g.ispublic)
        .cloned()
        .collect();
    public.sort_by_key(|g| (g.order, g.groupid));
    public
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq)]
pub struct GameCompletionStats {
    pub unlocked_base: u32,
    pub total_base: u32,
    pub platinum_pct: f32,
    pub is_platinum: bool,
    pub unlocked_all: u32,
    pub total_all: u32,
    pub global_pct: f32,
    pub has_dlc: bool,
}

/// Computes Base Game (Platinum) and Global (Base + DLC) completion metrics,
/// excluding developer-only/private groups and treating archived achievements
/// as excluded from totals unless already unlocked.
pub fn compute_game_completion(
    achievements: &[Achievement],
    groups: &[AchievementGroup],
) -> GameCompletionStats {
    let groups_map: HashMap<u32, &AchievementGroup> =
        groups.iter().map(|g| (g.groupid, g)).collect();

    let mut unlocked_base: u32 = 0;
    let mut total_base: u32 = 0;
    let mut unlocked_all: u32 = 0;
    let mut total_all: u32 = 0;

    for ach in achievements {
        let group = groups_map.get(&ach.groupid).copied();
        if ach.groupid != 0 && group.is_some_and(|g| g.developeronly || !g.ispublic) {
            continue;
        }

        let is_dlc = group.is_some_and(|g| g.dlcappid != 0);
        let is_archived = ach.archived || group.is_some_and(|g| g.archived);

        if is_archived && !ach.unlocked {
            continue;
        }

        total_all += 1;
        if !is_dlc {
            total_base += 1;
        }
        if ach.unlocked {
            unlocked_all += 1;
            if !is_dlc {
                unlocked_base += 1;
            }
        }
    }

    let platinum_pct = if total_base > 0 {
        (unlocked_base as f32 / total_base as f32) * 100.0
    } else {
        0.0
    };
    let is_platinum = total_base > 0 && unlocked_base == total_base;
    let global_pct = if total_all > 0 {
        (unlocked_all as f32 / total_all as f32) * 100.0
    } else {
        0.0
    };
    let has_dlc = groups
        .iter()
        .any(|g| !g.developeronly && g.ispublic && g.dlcappid != 0);

    GameCompletionStats {
        unlocked_base,
        total_base,
        platinum_pct,
        is_platinum,
        unlocked_all,
        total_all,
        global_pct,
        has_dlc,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_ach(key: &str, unlocked: bool, groupid: u32, archived: bool) -> Achievement {
        Achievement {
            key: key.to_string(),
            name: key.to_string(),
            unlocked,
            groupid,
            archived,
            ..Default::default()
        }
    }

    #[test]
    fn filter_public_groups_excludes_private_and_sorts() {
        let groups = vec![
            AchievementGroup {
                groupid: 20,
                localized_name: "DLC 2".to_string(),
                dlcappid: 200,
                order: 2,
                ..Default::default()
            },
            AchievementGroup {
                groupid: 5,
                localized_name: "Dev".to_string(),
                developeronly: true,
                order: 0,
                ..Default::default()
            },
            AchievementGroup {
                groupid: 6,
                localized_name: "Private".to_string(),
                ispublic: false,
                order: 0,
                ..Default::default()
            },
            AchievementGroup {
                groupid: 10,
                localized_name: "DLC 1".to_string(),
                dlcappid: 100,
                order: 1,
                ..Default::default()
            },
            AchievementGroup {
                groupid: 8,
                localized_name: "DLC 0".to_string(),
                dlcappid: 90,
                order: 1,
                ..Default::default()
            },
        ];

        let filtered = filter_public_groups(&groups);
        let ids: Vec<u32> = filtered.iter().map(|g| g.groupid).collect();
        assert_eq!(ids, vec![8, 10, 20]);
    }

    #[test]
    fn achievement_group_ispublic_defaults_to_true_in_serde() {
        let parsed: AchievementGroup =
            serde_json::from_str(r#"{"groupid": 1, "localized_name": "Expansion"}"#).unwrap();
        assert!(parsed.ispublic);
        assert!(!parsed.developeronly);
        assert!(!parsed.archived);
        assert_eq!(parsed.dlcappid, 0);
    }

    #[test]
    fn compute_completion_pure_base_game_without_groups() {
        let achs = vec![
            make_ach("A1", true, 0, false),
            make_ach("A2", true, 0, false),
            make_ach("A3", false, 0, false),
        ];
        let stats = compute_game_completion(&achs, &[]);
        assert_eq!(stats.unlocked_base, 2);
        assert_eq!(stats.total_base, 3);
        assert_eq!(stats.unlocked_all, 2);
        assert_eq!(stats.total_all, 3);
        assert!(!stats.has_dlc);
        assert!(!stats.is_platinum);
        assert!((stats.platinum_pct - (200.0 / 3.0)).abs() < 0.01);
        assert!((stats.global_pct - stats.platinum_pct).abs() < 0.01);
    }

    #[test]
    fn compute_completion_base_platinum_even_with_locked_dlc() {
        let groups = vec![AchievementGroup {
            groupid: 1,
            localized_name: "Shadow of the Erdtree".to_string(),
            dlcappid: 2778580,
            order: 1,
            ..Default::default()
        }];
        let achs = vec![
            make_ach("BASE_1", true, 0, false),
            make_ach("BASE_2", true, 0, false),
            make_ach("DLC_1", true, 1, false),
            make_ach("DLC_2", false, 1, false),
        ];

        let stats = compute_game_completion(&achs, &groups);
        assert!(stats.has_dlc);
        assert_eq!(stats.unlocked_base, 2);
        assert_eq!(stats.total_base, 2);
        assert!(stats.is_platinum);
        assert_eq!(stats.platinum_pct, 100.0);
        assert_eq!(stats.unlocked_all, 3);
        assert_eq!(stats.total_all, 4);
        assert_eq!(stats.global_pct, 75.0);
    }

    #[test]
    fn compute_completion_non_dlc_group_counts_toward_base() {
        let groups = vec![AchievementGroup {
            groupid: 1,
            localized_name: "Free Challenge Mode".to_string(),
            dlcappid: 0,
            order: 1,
            ..Default::default()
        }];
        let achs = vec![
            make_ach("BASE_1", true, 0, false),
            make_ach("MODE_1", false, 1, false),
        ];

        let stats = compute_game_completion(&achs, &groups);
        assert!(!stats.has_dlc);
        assert_eq!(stats.unlocked_base, 1);
        assert_eq!(stats.total_base, 2);
        assert!(!stats.is_platinum);
        assert_eq!(stats.platinum_pct, 50.0);
    }

    #[test]
    fn compute_completion_handles_archived_achievements_without_exceeding_100() {
        let groups = vec![AchievementGroup {
            groupid: 2,
            localized_name: "Legacy Season".to_string(),
            dlcappid: 0,
            archived: true,
            ..Default::default()
        }];
        let achs = vec![
            make_ach("BASE_1", true, 0, false),
            // Locked archived by achievement flag -> excluded from total
            make_ach("ARCH_LOCKED", false, 0, true),
            // Locked archived by group flag -> excluded from total
            make_ach("GROUP_ARCH_LOCKED", false, 2, false),
            // Unlocked archived -> +1 to both unlocked and total
            make_ach("ARCH_UNLOCKED", true, 2, false),
        ];

        let stats = compute_game_completion(&achs, &groups);
        assert_eq!(stats.unlocked_base, 2);
        assert_eq!(stats.total_base, 2);
        assert!(stats.is_platinum);
        assert_eq!(stats.platinum_pct, 100.0);
        assert_eq!(stats.unlocked_all, 2);
        assert_eq!(stats.total_all, 2);
        assert_eq!(stats.global_pct, 100.0);
    }

    #[test]
    fn compute_completion_excludes_developeronly_and_private_groups() {
        let groups = vec![
            AchievementGroup {
                groupid: 1,
                localized_name: "DLC".to_string(),
                dlcappid: 999,
                ..Default::default()
            },
            AchievementGroup {
                groupid: 2,
                localized_name: "Dev Test".to_string(),
                developeronly: true,
                dlcappid: 888,
                ..Default::default()
            },
            AchievementGroup {
                groupid: 3,
                localized_name: "Secret".to_string(),
                ispublic: false,
                ..Default::default()
            },
        ];
        let achs = vec![
            make_ach("BASE_1", true, 0, false),
            make_ach("DLC_1", true, 1, false),
            make_ach("DEV_1", false, 2, false),
            make_ach("PRIV_1", true, 3, false),
        ];

        let stats = compute_game_completion(&achs, &groups);
        assert!(stats.has_dlc);
        assert_eq!(stats.unlocked_base, 1);
        assert_eq!(stats.total_base, 1);
        assert!(stats.is_platinum);
        assert_eq!(stats.unlocked_all, 2);
        assert_eq!(stats.total_all, 2);
        assert_eq!(stats.global_pct, 100.0);
    }

    #[test]
    fn compute_completion_empty_returns_zero() {
        let stats = compute_game_completion(&[], &[]);
        assert_eq!(stats.unlocked_base, 0);
        assert_eq!(stats.total_base, 0);
        assert_eq!(stats.platinum_pct, 0.0);
        assert!(!stats.is_platinum);
        assert_eq!(stats.global_pct, 0.0);
        assert!(!stats.has_dlc);
    }
}
