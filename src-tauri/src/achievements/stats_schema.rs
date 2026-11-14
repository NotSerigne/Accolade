// src-tauri/src/achievements/stats_schema.rs
//! Reads Steam's local `UserGameStatsSchema_<appid>.bin` (in `Steam/appcache/stats/`)
//! and Goldberg/GSE's `steam_settings/achievements.json` to map progress-based
//! achievements to the stat that drives them, then reads the live stat value from
//! the emulator's stats files (`stats/<name>` for Goldberg/GSE, `Stats.ini` for OnlineFix).

use crate::achievements::models::{filter_public_groups, Achievement, AchievementGroup};
use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};
use std::sync::{Mutex, OnceLock};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum StatKind {
    Int,
    Float,
}

#[derive(Clone, Debug, PartialEq)]
pub struct ProgressRule {
    /// Stat key (e.g. `STAT_BUCKETS_OF_BOLTS`).
    pub stat_name: String,
    pub min_val: f64,
    pub max_val: f64,
    pub kind: StatKind,
}

#[derive(Clone, Debug, PartialEq, Eq, Default)]
pub struct AchievementSchemaMeta {
    pub groupid: u32,
    pub archived: bool,
}

#[derive(Clone, Debug, Default)]
pub struct ParsedStatsSchema {
    pub progress_rules: SchemaProgressMap,
    pub achievement_meta: HashMap<String, AchievementSchemaMeta>,
    pub groups: Vec<AchievementGroup>,
}

/// Achievement key (lowercased) -> progress rule.
pub type SchemaProgressMap = HashMap<String, ProgressRule>;

static FULL_SCHEMA_CACHE: OnceLock<Mutex<HashMap<u32, ParsedStatsSchema>>> = OnceLock::new();

fn cache() -> &'static Mutex<HashMap<u32, ParsedStatsSchema>> {
    FULL_SCHEMA_CACHE.get_or_init(|| Mutex::new(HashMap::new()))
}

// ---------------------------------------------------------------------------
// Generic Binary VDF tree (type 0x00=map, 0x01=str, 0x02=i32, 0x03=f32, 0x07=u64, 0x08=end)
// ---------------------------------------------------------------------------

#[derive(Debug, Clone)]
enum VdfValue {
    Map(Vec<(String, VdfValue)>),
    Str(String),
    Int(i32),
    Float(f32),
    U64(u64),
}

impl VdfValue {
    fn as_map(&self) -> Option<&[(String, VdfValue)]> {
        match self {
            VdfValue::Map(m) => Some(m),
            _ => None,
        }
    }

    fn get(&self, key: &str) -> Option<&VdfValue> {
        self.as_map()?
            .iter()
            .find(|(k, _)| k.eq_ignore_ascii_case(key))
            .map(|(_, v)| v)
    }

    fn as_str(&self) -> Option<&str> {
        match self {
            VdfValue::Str(s) => Some(s),
            _ => None,
        }
    }

    fn as_f64(&self) -> Option<f64> {
        match self {
            VdfValue::Int(n) => Some(*n as f64),
            VdfValue::Float(f) => Some(*f as f64),
            VdfValue::U64(u) => Some(*u as f64),
            VdfValue::Str(s) => s.trim().parse::<f64>().ok(),
            VdfValue::Map(_) => None,
        }
    }

    fn as_u32(&self) -> Option<u32> {
        match self {
            VdfValue::Int(n) if *n >= 0 => Some(*n as u32),
            VdfValue::U64(u) => u32::try_from(*u).ok(),
            VdfValue::Float(f) if *f >= 0.0 && f.is_finite() => Some(*f as u32),
            VdfValue::Str(s) => s.trim().parse::<u32>().ok(),
            _ => None,
        }
    }

    fn as_i32(&self) -> Option<i32> {
        match self {
            VdfValue::Int(n) => Some(*n),
            VdfValue::U64(u) => i32::try_from(*u).ok(),
            VdfValue::Float(f) if f.is_finite() => Some(*f as i32),
            VdfValue::Str(s) => s.trim().parse::<i32>().ok(),
            _ => None,
        }
    }

    fn as_bool(&self) -> Option<bool> {
        match self {
            VdfValue::Int(n) => Some(*n != 0),
            VdfValue::U64(u) => Some(*u != 0),
            VdfValue::Float(f) => Some(*f != 0.0),
            VdfValue::Str(s) => match s.trim().to_ascii_lowercase().as_str() {
                "1" | "true" | "yes" => Some(true),
                "0" | "false" | "no" => Some(false),
                _ => None,
            },
            VdfValue::Map(_) => None,
        }
    }
}

fn read_cstring(data: &[u8], pos: &mut usize) -> Option<String> {
    let end = data.get(*pos..)?.iter().position(|&b| b == 0)?;
    let s = String::from_utf8_lossy(&data[*pos..*pos + end]).into_owned();
    *pos += end + 1;
    Some(s)
}

fn parse_vdf_map(data: &[u8], pos: &mut usize, depth: usize) -> Option<Vec<(String, VdfValue)>> {
    if depth > 16 {
        return None;
    }
    let mut entries = Vec::new();
    while *pos < data.len() {
        let tag = data[*pos];
        *pos += 1;
        if tag == 0x08 {
            break;
        }
        let key = read_cstring(data, pos)?;
        let val = match tag {
            0x00 => VdfValue::Map(parse_vdf_map(data, pos, depth + 1)?),
            0x01 => VdfValue::Str(read_cstring(data, pos)?),
            0x02 => {
                let bytes: [u8; 4] = data.get(*pos..*pos + 4)?.try_into().ok()?;
                *pos += 4;
                VdfValue::Int(i32::from_le_bytes(bytes))
            }
            0x03 => {
                let bytes: [u8; 4] = data.get(*pos..*pos + 4)?.try_into().ok()?;
                *pos += 4;
                VdfValue::Float(f32::from_le_bytes(bytes))
            }
            0x07 => {
                let bytes: [u8; 8] = data.get(*pos..*pos + 8)?.try_into().ok()?;
                *pos += 8;
                VdfValue::U64(u64::from_le_bytes(bytes))
            }
            _ => return None,
        };
        entries.push((key, val));
    }
    Some(entries)
}

fn is_achievements_stat_type(raw: &VdfValue) -> bool {
    match raw {
        VdfValue::Str(s) => {
            s.eq_ignore_ascii_case("ACHIEVEMENTS")
                || s.eq_ignore_ascii_case("achievement")
                || s == "4"
        }
        VdfValue::Int(n) => *n == 4,
        _ => false,
    }
}

fn parse_stat_kind(raw: Option<&VdfValue>) -> StatKind {
    match raw {
        Some(VdfValue::Str(s))
            if s.eq_ignore_ascii_case("FLOAT")
                || s.eq_ignore_ascii_case("AVGRATE")
                || s == "2"
                || s == "3" =>
        {
            StatKind::Float
        }
        Some(VdfValue::Int(2 | 3)) => StatKind::Float,
        _ => StatKind::Int,
    }
}

fn extract_vdf_group_name(entry: &VdfValue) -> String {
    if let Some(s) = entry.get("localized_name").and_then(|v| v.as_str()) {
        if !s.trim().is_empty() {
            return s.trim().to_string();
        }
    }
    for container in [entry.get("display"), Some(entry)].into_iter().flatten() {
        if let Some(name_node) = container.get("name") {
            if let Some(s) = name_node.as_str() {
                if !s.trim().is_empty() {
                    return s.trim().to_string();
                }
            }
            if let Some(s) = name_node.get("english").and_then(|v| v.as_str()) {
                if !s.trim().is_empty() {
                    return s.trim().to_string();
                }
            }
            if let Some(pairs) = name_node.as_map() {
                for (_, val) in pairs {
                    if let Some(s) = val.as_str() {
                        if !s.trim().is_empty() {
                            return s.trim().to_string();
                        }
                    }
                }
            }
        }
        if let Some(s) = container.get("english").and_then(|v| v.as_str()) {
            if !s.trim().is_empty() {
                return s.trim().to_string();
            }
        }
    }
    String::new()
}

/// Parses a `UserGameStatsSchema_<appid>.bin` byte slice into progress rules,
/// achievement group/archived metadata, and achievement groups.
pub fn parse_full_stats_schema_bytes(data: &[u8]) -> ParsedStatsSchema {
    let mut pos = 0usize;
    let Some(root_entries) = parse_vdf_map(data, &mut pos, 0) else {
        return ParsedStatsSchema::default();
    };
    let root = VdfValue::Map(root_entries);

    // Root is `{ "<appid>": { "stats": { ... }, "groups": { ... } } }`.
    let app_node = root
        .as_map()
        .and_then(|entries| entries.first().map(|(_, v)| v));
    let stats_node = app_node
        .and_then(|app| app.get("stats"))
        .or_else(|| root.get("stats"));
    let groups_node = app_node
        .and_then(|app| app.get("groups").or_else(|| app.get("achievement_groups")))
        .or_else(|| root.get("groups").or_else(|| root.get("achievement_groups")))
        .or_else(|| {
            stats_node.and_then(|s| s.get("groups").or_else(|| s.get("achievement_groups")))
        });

    let mut groups = Vec::new();
    if let Some(group_entries) = groups_node.and_then(|n| n.as_map()) {
        for (raw_key, entry) in group_entries {
            if entry.as_map().is_none() {
                continue;
            }
            let groupid = entry
                .get("groupid")
                .or_else(|| entry.get("id"))
                .and_then(|v| v.as_u32())
                .or_else(|| raw_key.trim().parse::<u32>().ok())
                .unwrap_or(0);
            let localized_name = extract_vdf_group_name(entry);
            let dlcappid = entry.get("dlcappid").and_then(|v| v.as_u32()).unwrap_or(0);
            let archived = entry
                .get("archived")
                .and_then(|v| v.as_bool())
                .unwrap_or(false);
            let developeronly = entry
                .get("developeronly")
                .and_then(|v| v.as_bool())
                .unwrap_or(false);
            let ispublic = entry
                .get("ispublic")
                .and_then(|v| v.as_bool())
                .unwrap_or(true);
            let order = entry.get("order").and_then(|v| v.as_i32()).unwrap_or(0);

            groups.push(AchievementGroup {
                groupid,
                localized_name,
                dlcappid,
                archived,
                developeronly,
                ispublic,
                order,
            });
        }
    }

    let Some(stat_entries) = stats_node.and_then(|n| n.as_map()) else {
        return ParsedStatsSchema {
            groups,
            ..Default::default()
        };
    };

    // Pass 1: collect stat types (INT vs FLOAT) keyed by lowercase stat name.
    let mut stat_kinds: HashMap<String, StatKind> = HashMap::new();
    for (_, entry) in stat_entries {
        if entry.get("type").is_some_and(is_achievements_stat_type) {
            continue;
        }
        if let Some(name) = entry.get("name").and_then(|v| v.as_str()) {
            let kind = parse_stat_kind(entry.get("type"));
            stat_kinds.insert(name.trim().to_lowercase(), kind);
        }
    }

    // Pass 2: collect achievement group/archived metadata and `progress` rules.
    let mut progress_rules = HashMap::new();
    let mut achievement_meta = HashMap::new();
    for (_, entry) in stat_entries {
        let is_ach =
            entry.get("type").is_some_and(is_achievements_stat_type) || entry.get("bits").is_some();
        if !is_ach {
            continue;
        }
        let Some(bits) = entry.get("bits").and_then(|b| b.as_map()) else {
            continue;
        };
        for (_, bit) in bits {
            let Some(ach_key) = bit.get("name").and_then(|v| v.as_str()) else {
                continue;
            };
            let normalized_key = ach_key.trim().to_lowercase();
            let groupid = bit
                .get("groupid")
                .or_else(|| bit.get("display").and_then(|d| d.get("groupid")))
                .and_then(|v| v.as_u32())
                .unwrap_or(0);
            let archived = bit
                .get("archived")
                .or_else(|| bit.get("display").and_then(|d| d.get("archived")))
                .and_then(|v| v.as_bool())
                .unwrap_or(false);
            achievement_meta.insert(
                normalized_key.clone(),
                AchievementSchemaMeta { groupid, archived },
            );

            let Some(prog) = bit.get("progress") else {
                continue;
            };
            let max_val = prog.get("max_val").and_then(|v| v.as_f64()).unwrap_or(0.0);
            if !(max_val.is_finite() && max_val > 0.0) {
                continue;
            }
            let min_val = prog.get("min_val").and_then(|v| v.as_f64()).unwrap_or(0.0);
            let stat_name = prog
                .get("value")
                .and_then(|v| v.get("operand1").or_else(|| v.get("operand")))
                .and_then(|v| v.as_str())
                .map(|s| s.trim().to_string())
                .unwrap_or_default();
            if stat_name.is_empty() {
                continue;
            }
            let kind = stat_kinds
                .get(&stat_name.to_lowercase())
                .copied()
                .unwrap_or(StatKind::Int);
            progress_rules.insert(
                normalized_key,
                ProgressRule {
                    stat_name,
                    min_val,
                    max_val,
                    kind,
                },
            );
        }
    }

    ParsedStatsSchema {
        progress_rules,
        achievement_meta,
        groups,
    }
}

/// Parses a `UserGameStatsSchema_<appid>.bin` byte slice into achievement progress rules.
pub fn parse_stats_schema_bytes(data: &[u8]) -> SchemaProgressMap {
    parse_full_stats_schema_bytes(data).progress_rules
}

fn load_steam_stats_schema(app_id: u32) -> ParsedStatsSchema {
    let Some(steam_path) = crate::achievements::appinfo::get_steam_path() else {
        return ParsedStatsSchema::default();
    };
    let bin_path = steam_path
        .join("appcache")
        .join("stats")
        .join(format!("UserGameStatsSchema_{}.bin", app_id));
    std::fs::read(&bin_path)
        .map(|bytes| parse_full_stats_schema_bytes(&bytes))
        .unwrap_or_default()
}

/// Cached lookup of the parsed Steam `UserGameStatsSchema_<appid>.bin`.
pub fn full_schema_for_app(app_id: u32) -> ParsedStatsSchema {
    if let Ok(guard) = cache().lock() {
        if let Some(found) = guard.get(&app_id) {
            return found.clone();
        }
    }
    let loaded = load_steam_stats_schema(app_id);
    if let Ok(mut guard) = cache().lock() {
        guard.insert(app_id, loaded.clone());
    }
    loaded
}

/// Cached lookup of the Steam `UserGameStatsSchema_<appid>.bin` progress rules.
pub fn schema_for_app(app_id: u32) -> SchemaProgressMap {
    full_schema_for_app(app_id).progress_rules
}

/// Cached lookup of public, order-sorted achievement groups from `UserGameStatsSchema_<appid>.bin`.
pub fn schema_groups_for_app(app_id: u32) -> Vec<AchievementGroup> {
    filter_public_groups(&full_schema_for_app(app_id).groups)
}

pub(crate) fn apply_schema_meta(schema: &ParsedStatsSchema, achievements: &mut [Achievement]) {
    if schema.achievement_meta.is_empty() && schema.groups.is_empty() {
        return;
    }
    let archived_groups: HashSet<u32> = schema
        .groups
        .iter()
        .filter(|g| g.archived)
        .map(|g| g.groupid)
        .collect();
    for ach in achievements.iter_mut() {
        let key = ach.key.trim().to_lowercase();
        if let Some(meta) = schema.achievement_meta.get(&key) {
            if ach.groupid == 0 && meta.groupid != 0 {
                ach.groupid = meta.groupid;
            }
            if meta.archived {
                ach.archived = true;
            }
        }
        if ach.groupid != 0 && archived_groups.contains(&ach.groupid) {
            ach.archived = true;
        }
    }
}

/// Populates missing `groupid` and `archived` flags on `achievements` from the local
/// `UserGameStatsSchema_<appid>.bin` (offline fallback).
pub fn enrich_achievements_with_schema(achievements: &mut [Achievement], app_id: u32) {
    if app_id == 0 || achievements.is_empty() {
        return;
    }
    let schema = full_schema_for_app(app_id);
    apply_schema_meta(&schema, achievements);
}

// ---------------------------------------------------------------------------
// Live stat readers (Goldberg/GSE `stats/` dir & OnlineFix `Stats.ini`)
// ---------------------------------------------------------------------------

/// Reads a single Goldberg/GSE stat file (`<save_dir>/stats/<stat_name>`).
/// Goldberg writes raw 4-byte little-endian `i32` or `f32` values (or ASCII text in some tools).
fn read_goldberg_stat_file(path: &Path, kind: StatKind) -> Option<f64> {
    let bytes = std::fs::read(path).ok()?;
    if bytes.len() == 4 {
        let raw: [u8; 4] = bytes.as_slice().try_into().ok()?;
        let val = match kind {
            StatKind::Int => i32::from_le_bytes(raw) as f64,
            StatKind::Float => f32::from_le_bytes(raw) as f64,
        };
        if val.is_finite() {
            return Some(val);
        }
    }
    std::str::from_utf8(&bytes)
        .ok()?
        .trim()
        .parse::<f64>()
        .ok()
        .filter(|v| v.is_finite())
}

/// Reads all stats from a sibling `stats/` directory (case-insensitive on stat name).
fn read_goldberg_stats_dir(ach_file: &Path, rules: &SchemaProgressMap) -> HashMap<String, f64> {
    let mut out = HashMap::new();
    let Some(stats_dir) = ach_file.parent().map(|p| p.join("stats")) else {
        return out;
    };
    let Ok(entries) = std::fs::read_dir(&stats_dir) else {
        return out;
    };

    // Map lowercase stat name -> preferred StatKind from the schema.
    let kind_by_stat: HashMap<String, StatKind> = rules
        .values()
        .map(|r| (r.stat_name.to_lowercase(), r.kind))
        .collect();

    for entry in entries.flatten() {
        let path = entry.path();
        if !path.is_file() {
            continue;
        }
        let Some(name) = path.file_name().and_then(|n| n.to_str()) else {
            continue;
        };
        let key = name.trim().to_lowercase();
        let kind = kind_by_stat.get(&key).copied().unwrap_or(StatKind::Int);
        if let Some(val) = read_goldberg_stat_file(&path, kind) {
            out.insert(key, val);
        }
    }
    out
}

/// Reads `Stats.ini` (or `stats.ini`) sitting next to an OnlineFix `Achievements.ini`.
fn read_sibling_stats_ini(ach_file: &Path) -> HashMap<String, f64> {
    let mut out = HashMap::new();
    let Some(parent) = ach_file.parent() else {
        return out;
    };
    let candidates = [parent.join("Stats.ini"), parent.join("stats.ini")];
    let Some(ini_path) = candidates.iter().find(|p| p.is_file()) else {
        return out;
    };
    let Ok(ini) = ini::Ini::load_from_file(ini_path) else {
        return out;
    };
    for (_, prop) in &ini {
        for (k, v) in prop.iter() {
            if let Ok(n) = v.trim().trim_matches('"').parse::<f64>() {
                if n.is_finite() {
                    out.insert(k.trim().to_lowercase(), n);
                }
            }
        }
    }
    out
}

/// Fills missing `(progress, max_progress)`, `groupid`, and `archived` on `achievements`
/// by combining Steam's `UserGameStatsSchema_<appid>.bin` with any sibling `stats/` or `Stats.ini`.
pub fn enrich_from_sibling_stats(
    achievements: &mut [Achievement],
    ach_file_path: &str,
    steam_id: Option<u32>,
) {
    let Some(app_id) = steam_id.filter(|&id| id > 0) else {
        return;
    };

    let schema = full_schema_for_app(app_id);
    apply_schema_meta(&schema, achievements);

    let ach_path = Path::new(ach_file_path);
    if !ach_path.exists() {
        return;
    }

    let rules = &schema.progress_rules;
    if rules.is_empty() {
        return;
    }

    let mut stat_values = read_goldberg_stats_dir(ach_path, rules);
    for (k, v) in read_sibling_stats_ini(ach_path) {
        stat_values.entry(k).or_insert(v);
    }
    if stat_values.is_empty() {
        return;
    }

    for ach in achievements.iter_mut() {
        if ach.max_progress.is_some() {
            continue;
        }
        let Some(rule) = rules.get(&ach.key.trim().to_lowercase()) else {
            continue;
        };
        if let Some(&raw) = stat_values.get(&rule.stat_name.to_lowercase()) {
            let cur = raw.clamp(rule.min_val.min(0.0), rule.max_val);
            ach.progress = Some(cur);
            ach.max_progress = Some(rule.max_val);
        }
    }
}

pub fn temp_path_helper(p: &Path) -> PathBuf {
    p.to_path_buf()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_real_steam_stats_schema_if_present() {
        let p = Path::new(
            r"C:\Program Files (x86)\Steam\appcache\stats\UserGameStatsSchema_105600.bin",
        );
        if !p.is_file() {
            return;
        }
        let bytes = std::fs::read(p).unwrap();
        let rules = parse_stats_schema_bytes(&bytes);
        let rule = rules
            .get("mech_slayer")
            .or_else(|| rules.values().next())
            .unwrap();
        assert!(rule.max_val > 0.0);
        assert!(!rule.stat_name.is_empty());
    }

    #[test]
    fn reads_goldberg_binary_int_and_float_stats() {
        let dir = std::env::temp_dir().join(format!("accolade_stat_bin_{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();

        let int_file = dir.join("kills");
        std::fs::write(&int_file, 42i32.to_le_bytes()).unwrap();
        assert_eq!(
            read_goldberg_stat_file(&int_file, StatKind::Int),
            Some(42.0)
        );

        let flt_file = dir.join("distance");
        std::fs::write(&flt_file, 12.5f32.to_le_bytes()).unwrap();
        assert_eq!(
            read_goldberg_stat_file(&flt_file, StatKind::Float),
            Some(12.5)
        );

        let _ = std::fs::remove_dir_all(&dir);
    }

    fn vdf_str(buf: &mut Vec<u8>, key: &str, val: &str) {
        buf.push(0x01);
        buf.extend_from_slice(key.as_bytes());
        buf.push(0);
        buf.extend_from_slice(val.as_bytes());
        buf.push(0);
    }

    fn vdf_i32(buf: &mut Vec<u8>, key: &str, val: i32) {
        buf.push(0x02);
        buf.extend_from_slice(key.as_bytes());
        buf.push(0);
        buf.extend_from_slice(&val.to_le_bytes());
    }

    fn vdf_map(buf: &mut Vec<u8>, key: &str, build: impl FnOnce(&mut Vec<u8>)) {
        buf.push(0x00);
        buf.extend_from_slice(key.as_bytes());
        buf.push(0);
        build(buf);
        buf.push(0x08);
    }

    #[test]
    fn parses_binary_vdf_groups_dlcappid_groupid_and_archived() {
        let mut bin = Vec::new();
        vdf_map(&mut bin, "123456", |app| {
            vdf_map(app, "groups", |groups| {
                vdf_map(groups, "0", |g| {
                    vdf_i32(g, "groupid", 1);
                    vdf_map(g, "display", |d| {
                        vdf_map(d, "name", |n| vdf_str(n, "english", "Free Update"));
                    });
                    vdf_i32(g, "dlcappid", 0);
                    vdf_i32(g, "order", 1);
                });
                vdf_map(groups, "1", |g| {
                    vdf_i32(g, "groupid", 2);
                    vdf_str(g, "localized_name", "Story Expansion");
                    vdf_i32(g, "dlcappid", 654321);
                    vdf_i32(g, "order", 2);
                    vdf_i32(g, "ispublic", 1);
                });
                vdf_map(groups, "2", |g| {
                    vdf_i32(g, "groupid", 3);
                    vdf_str(g, "name", "Seasonal Event");
                    vdf_i32(g, "archived", 1);
                    vdf_i32(g, "order", 3);
                });
                vdf_map(groups, "3", |g| {
                    vdf_i32(g, "groupid", 4);
                    vdf_str(g, "name", "Dev Only");
                    vdf_i32(g, "developeronly", 1);
                });
                vdf_map(groups, "4", |g| {
                    vdf_i32(g, "groupid", 5);
                    vdf_str(g, "name", "Private");
                    vdf_i32(g, "ispublic", 0);
                });
            });
            vdf_map(app, "stats", |stats| {
                vdf_map(stats, "1", |s| {
                    vdf_i32(s, "type", 1);
                    vdf_str(s, "name", "kills");
                });
                vdf_map(stats, "2", |s| {
                    vdf_i32(s, "type", 4);
                    vdf_map(s, "bits", |bits| {
                        vdf_map(bits, "0", |b| {
                            vdf_str(b, "name", "ACH_BASE");
                        });
                        vdf_map(bits, "1", |b| {
                            vdf_str(b, "name", "ACH_DLC");
                            vdf_i32(b, "groupid", 2);
                            vdf_map(b, "progress", |p| {
                                vdf_i32(p, "min_val", 0);
                                vdf_i32(p, "max_val", 10);
                                vdf_map(p, "value", |v| vdf_str(v, "operand1", "kills"));
                            });
                        });
                        vdf_map(bits, "2", |b| {
                            vdf_str(b, "name", "ACH_BIT_ARCHIVED");
                            vdf_i32(b, "archived", 1);
                        });
                        vdf_map(bits, "3", |b| {
                            vdf_str(b, "name", "ACH_GRP_ARCHIVED");
                            vdf_i32(b, "groupid", 3);
                        });
                    });
                });
            });
        });

        let parsed = parse_full_stats_schema_bytes(&bin);
        assert_eq!(parsed.progress_rules.get("ach_dlc").unwrap().max_val, 10.0);
        assert_eq!(
            parse_stats_schema_bytes(&bin).get("ach_dlc").unwrap().stat_name,
            "kills"
        );

        let public_groups = filter_public_groups(&parsed.groups);
        let group_ids: Vec<u32> = public_groups.iter().map(|g| g.groupid).collect();
        assert_eq!(group_ids, vec![1, 2, 3]);
        assert_eq!(public_groups[0].localized_name, "Free Update");
        assert_eq!(public_groups[1].dlcappid, 654321);
        assert!(public_groups[2].archived);

        let mut achs = vec![
            Achievement {
                key: "ACH_BASE".to_string(),
                ..Default::default()
            },
            Achievement {
                key: "ACH_DLC".to_string(),
                ..Default::default()
            },
            Achievement {
                key: "ACH_BIT_ARCHIVED".to_string(),
                ..Default::default()
            },
            Achievement {
                key: "ACH_GRP_ARCHIVED".to_string(),
                ..Default::default()
            },
        ];
        apply_schema_meta(&parsed, &mut achs);

        assert_eq!(achs[0].groupid, 0);
        assert!(!achs[0].archived);
        assert_eq!(achs[1].groupid, 2);
        assert!(!achs[1].archived);
        assert_eq!(achs[2].groupid, 0);
        assert!(achs[2].archived);
        assert_eq!(achs[3].groupid, 3);
        assert!(achs[3].archived);
    }
}
