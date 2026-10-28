// src-tauri/src/achievements/stats_schema.rs
//! Reads Steam's local `UserGameStatsSchema_<appid>.bin` (in `Steam/appcache/stats/`)
//! and Goldberg/GSE's `steam_settings/achievements.json` to map progress-based
//! achievements to the stat that drives them, then reads the live stat value from
//! the emulator's stats files (`stats/<name>` for Goldberg/GSE, `Stats.ini` for OnlineFix).

use crate::achievements::models::Achievement;
use std::collections::HashMap;
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

/// Achievement key (lowercased) -> progress rule.
pub type SchemaProgressMap = HashMap<String, ProgressRule>;

static SCHEMA_CACHE: OnceLock<Mutex<HashMap<u32, SchemaProgressMap>>> = OnceLock::new();

fn cache() -> &'static Mutex<HashMap<u32, SchemaProgressMap>> {
    SCHEMA_CACHE.get_or_init(|| Mutex::new(HashMap::new()))
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

/// Parses a `UserGameStatsSchema_<appid>.bin` byte slice into achievement progress rules.
pub fn parse_stats_schema_bytes(data: &[u8]) -> SchemaProgressMap {
    let mut pos = 0usize;
    let Some(root_entries) = parse_vdf_map(data, &mut pos, 0) else {
        return HashMap::new();
    };
    let root = VdfValue::Map(root_entries);

    // Root is `{ "<appid>": { "stats": { "1": { ... }, "2": { ... } } } }`.
    let stats_node = root
        .as_map()
        .and_then(|entries| entries.first().map(|(_, v)| v))
        .and_then(|app| app.get("stats"))
        .or_else(|| root.get("stats"));

    let Some(stat_entries) = stats_node.and_then(|n| n.as_map()) else {
        return HashMap::new();
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

    // Pass 2: collect achievements that declare a `progress` block.
    let mut map = HashMap::new();
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
            map.insert(
                ach_key.trim().to_lowercase(),
                ProgressRule {
                    stat_name,
                    min_val,
                    max_val,
                    kind,
                },
            );
        }
    }

    map
}

fn load_steam_stats_schema(app_id: u32) -> SchemaProgressMap {
    let Some(steam_path) = crate::achievements::appinfo::get_steam_path() else {
        return HashMap::new();
    };
    let bin_path = steam_path
        .join("appcache")
        .join("stats")
        .join(format!("UserGameStatsSchema_{}.bin", app_id));
    std::fs::read(&bin_path)
        .map(|bytes| parse_stats_schema_bytes(&bytes))
        .unwrap_or_default()
}

/// Cached lookup of the Steam `UserGameStatsSchema_<appid>.bin` progress rules.
pub fn schema_for_app(app_id: u32) -> SchemaProgressMap {
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

/// Fills missing `(progress, max_progress)` on `achievements` by combining the
/// Steam `UserGameStatsSchema_<appid>.bin` with any sibling `stats/` or `Stats.ini`.
pub fn enrich_from_sibling_stats(
    achievements: &mut [Achievement],
    ach_file_path: &str,
    steam_id: Option<u32>,
) {
    let Some(app_id) = steam_id.filter(|&id| id > 0) else {
        return;
    };
    let ach_path = Path::new(ach_file_path);
    if !ach_path.exists() {
        return;
    }

    let rules = schema_for_app(app_id);
    if rules.is_empty() {
        return;
    }

    let mut stat_values = read_goldberg_stats_dir(ach_path, &rules);
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
}
