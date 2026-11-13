// src-tauri/src/hltb.rs
//! HowLongToBeat (HLTB) completion time integration with local disk caching
//! and manual override support.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::path::PathBuf;
use std::sync::{Mutex, OnceLock};
use std::time::{SystemTime, UNIX_EPOCH};
use tauri::{AppHandle, Manager};

const HLTB_USER_AGENT: &str = "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/134.0.0.0 Safari/537.36";
const HLTB_CANDIDATE_BASES: &[&str] = &[
    "/api/find",
    "/api/seek",
    "/api/locate",
    "/api/search",
    "/api/s",
    "/api/lookup",
];
const NEGATIVE_CACHE_TTL_SECS: u64 = 60;

static HLTB_HTTP_CLIENT: OnceLock<reqwest::Client> = OnceLock::new();
static DISCOVERED_HLTB_ENDPOINT: OnceLock<Mutex<Option<String>>> = OnceLock::new();
static NEGATIVE_HLTB_CACHE: OnceLock<Mutex<HashMap<String, HltbData>>> = OnceLock::new();

fn get_hltb_client() -> &'static reqwest::Client {
    HLTB_HTTP_CLIENT.get_or_init(|| {
        reqwest::Client::builder()
            .user_agent(HLTB_USER_AGENT)
            .timeout(std::time::Duration::from_secs(10))
            .pool_max_idle_per_host(8)
            .tcp_nodelay(true)
            .build()
            .unwrap_or_else(|_| reqwest::Client::new())
    })
}

#[derive(Serialize, Deserialize, Clone, Debug, Default)]
pub struct HltbData {
    pub game_id: String,
    pub title: String,
    /// Main Story completion time in hours (e.g. 24.5).
    pub main_hours: Option<f32>,
    /// Main + Sides completion time in hours (e.g. 42.0).
    pub main_extra_hours: Option<f32>,
    /// Completionist (100%) time in hours (e.g. 68.5).
    pub completionist_hours: Option<f32>,
    /// `"hltb"`, `"custom"`, or `"none"`
    pub source: String,
    pub fetched_at: u64,
}

impl HltbData {
    fn has_any_time(&self) -> bool {
        self.main_hours.is_some()
            || self.main_extra_hours.is_some()
            || self.completionist_hours.is_some()
    }
}

fn now_unix() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0)
}

fn now_millis() -> u128 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_millis())
        .unwrap_or(0)
}

fn get_cache_path(app_handle: &AppHandle) -> PathBuf {
    let mut path = app_handle
        .path()
        .app_data_dir()
        .unwrap_or_else(|_| PathBuf::from("."));
    if !path.exists() {
        let _ = fs::create_dir_all(&path);
    }
    path.push("hltb_cache.json");
    path
}

pub fn load_hltb_cache(app_handle: &AppHandle) -> HashMap<String, HltbData> {
    let path = get_cache_path(app_handle);
    if let Ok(content) = fs::read_to_string(path) {
        let mut parsed: HashMap<String, HltbData> =
            serde_json::from_str(&content).unwrap_or_default();
        // Discard inaccurate legacy SteamSpy entries so games are re-fetched from HLTB
        parsed.retain(|_, v| v.source != "steamspy");
        parsed
    } else {
        HashMap::new()
    }
}

pub fn save_hltb_cache(app_handle: &AppHandle, cache: &HashMap<String, HltbData>) {
    // Never persist failed ("none") entries lacking times so hltb_cache.json is never poisoned
    let filtered: HashMap<&String, &HltbData> = cache
        .iter()
        .filter(|(_, v)| v.source != "none" || v.has_any_time())
        .collect();
    let path = get_cache_path(app_handle);
    if let Ok(content) = serde_json::to_string_pretty(&filtered) {
        let _ = fs::write(path, content);
    }
}

fn clean_game_title(raw: &str) -> String {
    let cleaned: String = raw
        .chars()
        .filter(|&c| c != '™' && c != '®' && c != '©')
        .map(|c| {
            if c == ':' || c == '-' || c == '—' {
                ' '
            } else {
                c
            }
        })
        .collect();
    cleaned.split_whitespace().collect::<Vec<_>>().join(" ")
}

const EDITION_SUFFIXES: &[&str] = &[
    "game of the year edition",
    "anniversary edition",
    "definitive edition",
    "complete edition",
    "ultimate edition",
    "enhanced edition",
    "director's cut",
    "goty edition",
    "remastered",
    "edition",
];

fn strip_edition_suffix(title: &str) -> String {
    let mut current = title.trim().to_string();
    loop {
        let lower = current.to_lowercase();
        let mut stripped_any = false;
        for suffix in EDITION_SUFFIXES {
            if lower.ends_with(suffix) && lower.len() > suffix.len() {
                let split_idx = current.len() - suffix.len();
                if current.as_bytes()[split_idx - 1].is_ascii_whitespace() {
                    let candidate = current[..split_idx].trim();
                    if !candidate.is_empty() {
                        current = candidate.to_string();
                        stripped_any = true;
                        break;
                    }
                }
            }
        }
        if !stripped_any {
            break;
        }
    }
    current
}

fn round_hours(seconds: u64) -> Option<f32> {
    if seconds == 0 {
        return None;
    }
    let raw = (seconds as f32) / 3600.0;
    let rounded = if raw >= 1.0 {
        (raw * 2.0).round() / 2.0
    } else {
        (raw * 10.0).round() / 10.0
    };
    if rounded <= 0.0 {
        None
    } else {
        Some(rounded)
    }
}

#[derive(Deserialize, Debug)]
struct InitTokenResponse {
    token: Option<String>,
    #[serde(rename = "hpKey")]
    hp_key: Option<String>,
    #[serde(rename = "hpVal")]
    hp_val: Option<String>,
}

#[derive(Deserialize)]
struct HltbSearchResponse {
    data: Option<Vec<HltbGameEntry>>,
}

#[derive(Deserialize, Clone, Debug, Default, PartialEq, Eq)]
struct HltbGameEntry {
    #[serde(default)]
    game_id: u64,
    #[serde(default)]
    game_name: String,
    #[serde(default)]
    comp_main: u64,
    #[serde(default)]
    comp_plus: u64,
    #[serde(default)]
    comp_100: u64,
}

fn extract_next_chunk_urls(html: &str) -> Vec<String> {
    let mut priority_chunks = Vec::new();
    let mut other_chunks = Vec::new();
    let needle = "/_next/static/chunks/";
    let mut offset = 0;

    while let Some(pos) = html[offset..].find(needle) {
        let start = offset + pos;
        let rest = &html[start..];
        let end = rest
            .find(|c: char| c == '"' || c == '\'' || c == '\\' || c.is_whitespace() || c == '<')
            .unwrap_or(rest.len());
        let path = &rest[..end];
        if path.ends_with(".js") {
            let owned = path.to_string();
            if path.contains("pages/index") || path.contains("pages/_app") {
                if !priority_chunks.contains(&owned) {
                    priority_chunks.push(owned);
                }
            } else if !other_chunks.contains(&owned) {
                other_chunks.push(owned);
            }
        }
        offset = start + end.max(1);
    }

    priority_chunks.extend(other_chunks);
    priority_chunks.truncate(24);
    priority_chunks
}

fn is_api_seg_byte(b: u8) -> bool {
    b.is_ascii_alphanumeric() || b == b'_' || b == b'-'
}

fn extract_api_base_from_js(js: &str) -> Option<String> {
    let ignored = [
        "/api/user",
        "/api/login",
        "/api/logout",
        "/api/auth",
        "/api/stats",
        "/api/image",
        "/api/submit",
    ];
    let mut best: Option<(i32, String)> = None;
    let needle = "/api/";
    let mut offset = 0;

    while let Some(pos) = js[offset..].find(needle) {
        let abs_pos = offset + pos;
        let after_prefix = &js[abs_pos + needle.len()..];
        let seg1_len = after_prefix.bytes().take_while(|&b| is_api_seg_byte(b)).count();

        if seg1_len > 0 {
            let seg1 = &after_prefix[..seg1_len];
            let after_seg1 = &after_prefix[seg1_len..];

            let mut candidate = format!("/api/{}", seg1);
            let mut after_base = after_seg1;
            let mut total_len = needle.len() + seg1_len;

            if let Some(rest_after_slash) = after_seg1.strip_prefix('/') {
                let seg2_len = rest_after_slash
                    .bytes()
                    .take_while(|&b| is_api_seg_byte(b))
                    .count();
                if seg2_len > 0 {
                    let seg2 = &rest_after_slash[..seg2_len];
                    if seg2 != "init" {
                        candidate = format!("/api/{}/{}", seg1, seg2);
                        after_base = &rest_after_slash[seg2_len..];
                        total_len += 1 + seg2_len;
                    }
                }
            }

            let is_ignored = ignored
                .iter()
                .any(|ig| candidate == *ig || candidate.starts_with(&format!("{}/", ig)));

            if !is_ignored {
                let mut win_start = abs_pos.saturating_sub(250);
                while !js.is_char_boundary(win_start) {
                    win_start = win_start.saturating_sub(1);
                }
                let mut win_end = (abs_pos + total_len + 250).min(js.len());
                while win_end < js.len() && !js.is_char_boundary(win_end) {
                    win_end += 1;
                }
                let context = &js[win_start..win_end];

                let mut score = 0i32;
                if after_base.starts_with("/init") {
                    score += 5;
                }
                if context.contains("init") {
                    score += 3;
                }
                if context.contains("searchTerms") {
                    score += 4;
                }
                if context.contains("x-auth-token") {
                    score += 4;
                }
                if context.contains("hpKey") || context.contains("x-hp-key") {
                    score += 4;
                }
                if context.contains("fetch(") {
                    score += 2;
                }
                if score > 0 {
                    match &best {
                        Some((best_score, _)) if *best_score >= score => {}
                        _ => best = Some((score, candidate)),
                    }
                }
            }
        }
        offset = abs_pos + needle.len();
    }

    best.map(|(_, path)| path)
}

async fn discover_hltb_endpoint(client: &reqwest::Client) -> Option<String> {
    let lock = DISCOVERED_HLTB_ENDPOINT.get_or_init(|| Mutex::new(None));
    if let Ok(guard) = lock.lock() {
        if let Some(ref cached) = *guard {
            return Some(cached.clone());
        }
    }

    let html = client
        .get("https://howlongtobeat.com/")
        .header("Referer", "https://howlongtobeat.com/")
        .header("Origin", "https://howlongtobeat.com")
        .header(
            "Accept",
            "text/html,application/xhtml+xml,application/xml;q=0.9,*/*;q=0.8",
        )
        .send()
        .await
        .ok()?
        .text()
        .await
        .ok()?;

    let chunk_urls = extract_next_chunk_urls(&html);
    for chunk_path in chunk_urls {
        let full_url = format!("https://howlongtobeat.com{}", chunk_path);
        if let Ok(resp) = client
            .get(&full_url)
            .header("Referer", "https://howlongtobeat.com/")
            .header("Accept", "*/*")
            .send()
            .await
        {
            if resp.status().is_success() {
                if let Ok(js) = resp.text().await {
                    if let Some(base) = extract_api_base_from_js(&js) {
                        log::info!("[hltb] Discovered API base endpoint: {}", base);
                        if let Ok(mut guard) = lock.lock() {
                            *guard = Some(base.clone());
                        }
                        return Some(base);
                    }
                }
            }
        }
    }

    None
}

async fn fetch_init_token(client: &reqwest::Client, base: &str) -> Option<InitTokenResponse> {
    let init_url = format!("https://howlongtobeat.com{}/init?t={}", base, now_millis());
    let resp = client
        .get(&init_url)
        .header("Referer", "https://howlongtobeat.com/")
        .header("Origin", "https://howlongtobeat.com")
        .header("Accept", "*/*")
        .send()
        .await
        .ok()?;

    if !resp.status().is_success() {
        return None;
    }

    let parsed = resp.json::<InitTokenResponse>().await.ok()?;
    if parsed.token.as_deref().is_some_and(|t| !t.is_empty()) {
        Some(parsed)
    } else {
        None
    }
}

fn build_search_payload(terms: &[String], init: &InitTokenResponse) -> serde_json::Value {
    let mut payload = serde_json::json!({
        "searchType": "games",
        "searchTerms": terms,
        "searchPage": 1,
        "size": 20,
        "searchOptions": {
            "games": {
                "userId": 0,
                "platform": "",
                "sortCategory": "popular",
                "rangeCategory": "main",
                "rangeTime": { "min": null, "max": null },
                "gameplay": { "perspective": "", "flow": "", "genre": "", "difficulty": "" },
                "rangeYear": { "min": "", "max": "" },
                "modifier": ""
            },
            "users": { "sortCategory": "postcount" },
            "lists": { "sortCategory": "follows" },
            "filter": "",
            "sort": 0,
            "randomizer": 0
        },
        "useCache": false
    });

    if let (Some(ref hp_key), Some(ref hp_val)) = (&init.hp_key, &init.hp_val) {
        if !hp_key.is_empty() && !hp_val.is_empty() {
            if let Some(map) = payload.as_object_mut() {
                map.insert(
                    hp_key.clone(),
                    serde_json::Value::String(hp_val.clone()),
                );
            }
        }
    }

    payload
}

async fn post_hltb_search(
    client: &reqwest::Client,
    base: &str,
    init: &InitTokenResponse,
    terms: &[String],
) -> Option<Vec<HltbGameEntry>> {
    let token = init.token.as_deref()?;
    let payload = build_search_payload(terms, init);
    let post_urls = [
        format!("https://howlongtobeat.com{}", base),
        format!("https://howlongtobeat.com{}/{}", base, token),
    ];

    for post_url in post_urls {
        let mut req = client
            .post(&post_url)
            .header("Content-Type", "application/json")
            .header("Accept", "*/*")
            .header("Referer", "https://howlongtobeat.com/")
            .header("Origin", "https://howlongtobeat.com")
            .header("x-auth-token", token);

        if let (Some(ref hp_key), Some(ref hp_val)) = (&init.hp_key, &init.hp_val) {
            if !hp_key.is_empty() && !hp_val.is_empty() {
                req = req.header("x-hp-key", hp_key).header("x-hp-val", hp_val);
            }
        }

        if let Ok(resp) = req.json(&payload).send().await {
            if resp.status().is_success() {
                if let Ok(parsed) = resp.json::<HltbSearchResponse>().await {
                    if let Some(entries) = parsed.data {
                        return Some(entries);
                    }
                }
            }
        }
    }

    None
}

fn pick_best_hltb_entry(entries: &[HltbGameEntry], wanted_title: &str) -> Option<HltbGameEntry> {
    let lower_wanted = wanted_title.to_lowercase();
    entries
        .iter()
        .filter(|e| e.comp_main > 0 || e.comp_plus > 0 || e.comp_100 > 0)
        .min_by_key(|e| {
            let entry_clean = clean_game_title(&e.game_name).to_lowercase();
            if entry_clean == lower_wanted {
                0usize
            } else if entry_clean.contains(&lower_wanted) || lower_wanted.contains(&entry_clean) {
                1usize
            } else {
                2usize
            }
        })
        .cloned()
}

async fn search_with_candidate_bases(
    client: &reqwest::Client,
    title: &str,
    preferred_base: Option<&str>,
) -> (Option<Vec<HltbGameEntry>>, Option<String>) {
    let terms: Vec<String> = title.split_whitespace().map(|s| s.to_string()).collect();
    if terms.is_empty() {
        return (None, None);
    }

    let mut bases: Vec<String> = Vec::new();
    if let Some(pref) = preferred_base {
        bases.push(pref.to_string());
    }
    if let Ok(guard) = DISCOVERED_HLTB_ENDPOINT
        .get_or_init(|| Mutex::new(None))
        .lock()
    {
        if let Some(ref discovered) = *guard {
            if !bases.contains(discovered) {
                bases.push(discovered.clone());
            }
        }
    }
    for &b in HLTB_CANDIDATE_BASES {
        let s = b.to_string();
        if !bases.contains(&s) {
            bases.push(s);
        }
    }

    for base in &bases {
        if let Some(init) = fetch_init_token(client, base).await {
            if let Some(entries) = post_hltb_search(client, base, &init, &terms).await {
                return (Some(entries), Some(base.clone()));
            }
        }
    }

    // Dynamic endpoint discovery fallback if all static candidates failed
    if let Some(discovered) = discover_hltb_endpoint(client).await {
        if !bases.contains(&discovered) {
            if let Some(init) = fetch_init_token(client, &discovered).await {
                if let Some(entries) = post_hltb_search(client, &discovered, &init, &terms).await {
                    return (Some(entries), Some(discovered));
                }
            }
        }
    }

    (None, None)
}

fn parse_hltb_next_data(html: &str) -> Option<HltbGameEntry> {
    let marker = html
        .find(r#"id="__NEXT_DATA__""#)
        .or_else(|| html.find("id='__NEXT_DATA__'"))?;
    let after_marker = &html[marker..];
    let content_start_rel = after_marker.find('>')? + 1;
    let content = &after_marker[content_start_rel..];
    let content_end = content.find("</script>")?;
    let json_str = content[..content_end].trim();

    let val: serde_json::Value = serde_json::from_str(json_str).ok()?;
    let game = val.pointer("/props/pageProps/game/data/game/0")?;

    let game_id = game.get("game_id").and_then(|v| v.as_u64()).unwrap_or(0);
    let game_name = game
        .get("game_name")
        .and_then(|v| v.as_str())
        .unwrap_or_default()
        .trim()
        .to_string();
    let comp_main = game.get("comp_main").and_then(|v| v.as_u64()).unwrap_or(0);
    let comp_plus = game.get("comp_plus").and_then(|v| v.as_u64()).unwrap_or(0);
    let comp_100 = game.get("comp_100").and_then(|v| v.as_u64()).unwrap_or(0);

    if game_id == 0 && game_name.is_empty() {
        return None;
    }

    Some(HltbGameEntry {
        game_id,
        game_name,
        comp_main,
        comp_plus,
        comp_100,
    })
}

async fn fetch_hltb_game_page_by_id(
    client: &reqwest::Client,
    game_id: u64,
) -> Option<HltbGameEntry> {
    if game_id == 0 {
        return None;
    }
    let url = format!("https://howlongtobeat.com/game/{}", game_id);
    let resp = client
        .get(&url)
        .header("Referer", "https://howlongtobeat.com/")
        .header("Origin", "https://howlongtobeat.com")
        .header(
            "Accept",
            "text/html,application/xhtml+xml,application/xml;q=0.9,*/*;q=0.8",
        )
        .send()
        .await
        .ok()?;
    if !resp.status().is_success() {
        return None;
    }
    let html = resp.text().await.ok()?;
    parse_hltb_next_data(&html)
}

async fn enrich_entry_from_live_page(
    client: &reqwest::Client,
    mut entry: HltbGameEntry,
) -> HltbGameEntry {
    if entry.game_id > 0 {
        if let Some(live) = fetch_hltb_game_page_by_id(client, entry.game_id).await {
            if live.comp_main > 0 {
                entry.comp_main = live.comp_main;
            }
            if live.comp_plus > 0 {
                entry.comp_plus = live.comp_plus;
            }
            if live.comp_100 > 0 {
                entry.comp_100 = live.comp_100;
            }
            if !live.game_name.is_empty() {
                entry.game_name = live.game_name;
            }
        }
    }
    entry
}

fn url_encode_query(input: &str) -> String {
    let mut out = String::with_capacity(input.len() * 2);
    for &b in input.as_bytes() {
        match b {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' => {
                out.push(b as char);
            }
            b' ' => out.push('+'),
            _ => out.push_str(&format!("%{:02X}", b)),
        }
    }
    out
}

fn extract_hltb_game_ids_from_html(html: &str) -> Vec<u64> {
    let lower = html.to_ascii_lowercase();
    let patterns = ["howlongtobeat.com/game/", "howlongtobeat.com%2fgame%2f"];
    let mut hits: Vec<(usize, u64)> = Vec::new();

    for pat in patterns {
        let mut offset = 0;
        while let Some(pos) = lower[offset..].find(pat) {
            let abs = offset + pos + pat.len();
            let digits: String = lower[abs..]
                .bytes()
                .take_while(|b| b.is_ascii_digit())
                .map(|b| b as char)
                .collect();
            if let Ok(id) = digits.parse::<u64>() {
                if id > 0 {
                    hits.push((abs, id));
                }
            }
            offset = abs;
        }
    }

    hits.sort_by_key(|(pos, _)| *pos);
    let mut ids = Vec::new();
    for (_, id) in hits {
        if !ids.contains(&id) {
            ids.push(id);
            if ids.len() >= 5 {
                break;
            }
        }
    }
    ids
}

fn title_similarity(wanted: &str, candidate: &str) -> f32 {
    let a = clean_game_title(wanted).to_lowercase();
    let b = clean_game_title(candidate).to_lowercase();
    if a.is_empty() || b.is_empty() {
        return 0.0;
    }
    if a == b {
        return 1.0;
    }

    let tokens_a: std::collections::HashSet<&str> = a
        .split(|c: char| !c.is_alphanumeric())
        .filter(|s| !s.is_empty())
        .collect();
    let tokens_b: std::collections::HashSet<&str> = b
        .split(|c: char| !c.is_alphanumeric())
        .filter(|s| !s.is_empty())
        .collect();

    if tokens_a.is_empty() || tokens_b.is_empty() {
        return 0.0;
    }

    let intersection = tokens_a.intersection(&tokens_b).count() as f32;
    (2.0 * intersection) / ((tokens_a.len() + tokens_b.len()) as f32)
}

async fn discover_hltb_game_via_web(
    client: &reqwest::Client,
    title: &str,
) -> Option<HltbGameEntry> {
    let encoded = url_encode_query(title);
    if encoded.is_empty() {
        return None;
    }

    let search_url = format!(
        "https://html.duckduckgo.com/html/?q=site%3Ahowlongtobeat.com%2Fgame+{}",
        encoded
    );
    let resp = client
        .get(&search_url)
        .header(
            "Accept",
            "text/html,application/xhtml+xml,application/xml;q=0.9,*/*;q=0.8",
        )
        .send()
        .await
        .ok()?;
    if !resp.status().is_success() {
        return None;
    }
    let html = resp.text().await.ok()?;
    let candidate_ids = extract_hltb_game_ids_from_html(&html);
    if candidate_ids.is_empty() {
        return None;
    }

    let mut best: Option<(f32, HltbGameEntry)> = None;
    for id in candidate_ids {
        if let Some(entry) = fetch_hltb_game_page_by_id(client, id).await {
            if entry.comp_main == 0 && entry.comp_plus == 0 && entry.comp_100 == 0 {
                continue;
            }
            let sim_full = title_similarity(title, &entry.game_name);
            let stripped = strip_edition_suffix(&clean_game_title(&entry.game_name));
            let sim_stripped = title_similarity(title, &stripped);
            let sim = sim_full.max(sim_stripped);
            if sim >= 0.45 {
                if sim >= 0.95 {
                    return Some(entry);
                }
                match &best {
                    Some((best_sim, _)) if *best_sim >= sim => {}
                    _ => best = Some((sim, entry)),
                }
            }
        }
    }

    best.map(|(_, entry)| entry)
}

async fn query_hltb_remote(game_name: &str) -> Option<HltbGameEntry> {
    let cleaned = clean_game_title(game_name);
    if cleaned.is_empty() || cleaned.starts_with("AppID ") {
        return None;
    }

    let client = get_hltb_client();
    let (entries_opt, working_base) = search_with_candidate_bases(client, &cleaned, None).await;

    if let Some(entries) = entries_opt {
        if let Some(best) = pick_best_hltb_entry(&entries, &cleaned) {
            return Some(enrich_entry_from_live_page(client, best).await);
        }
    }

    // Smart Title Retry: strip edition suffix (e.g. "GOTY Edition", "Complete Edition")
    let base_title = strip_edition_suffix(&cleaned);
    if !base_title.is_empty() && base_title != cleaned {
        let (retry_entries_opt, _) =
            search_with_candidate_bases(client, &base_title, working_base.as_deref()).await;
        if let Some(entries) = retry_entries_opt {
            if let Some(best) = pick_best_hltb_entry(&entries, &base_title) {
                return Some(enrich_entry_from_live_page(client, best).await);
            }
        }
    }

    // Web search fallback if HLTB API search failed or returned no match
    if let Some(entry) = discover_hltb_game_via_web(client, &cleaned).await {
        return Some(entry);
    }
    if !base_title.is_empty() && base_title != cleaned {
        if let Some(entry) = discover_hltb_game_via_web(client, &base_title).await {
            return Some(entry);
        }
    }

    None
}

#[tauri::command]
pub async fn get_hltb_data(
    app_handle: AppHandle,
    game_id: String,
    game_name: String,
    force_refresh: Option<bool>,
) -> Result<HltbData, String> {
    let force = force_refresh.unwrap_or(false);
    let mut cache = load_hltb_cache(&app_handle);

    if !force {
        if let Some(existing) = cache.get(&game_id) {
            if existing.source == "custom" {
                return Ok(existing.clone());
            }
            if existing.source == "hltb" && existing.has_any_time() {
                return Ok(existing.clone());
            }
            // Do not return stale "none" entries lacking times if older than 60s
            if existing.source == "none"
                && !existing.has_any_time()
                && now_unix().saturating_sub(existing.fetched_at) < NEGATIVE_CACHE_TTL_SECS
            {
                return Ok(existing.clone());
            }
        }

        if let Ok(neg) = NEGATIVE_HLTB_CACHE
            .get_or_init(|| Mutex::new(HashMap::new()))
            .lock()
        {
            if let Some(existing) = neg.get(&game_id) {
                if now_unix().saturating_sub(existing.fetched_at) < NEGATIVE_CACHE_TTL_SECS {
                    return Ok(existing.clone());
                }
            }
        }
    }

    if let Some(entry) = query_hltb_remote(&game_name).await {
        let data = HltbData {
            game_id: game_id.clone(),
            title: entry.game_name,
            main_hours: round_hours(entry.comp_main),
            main_extra_hours: round_hours(entry.comp_plus),
            completionist_hours: round_hours(entry.comp_100),
            source: "hltb".to_string(),
            fetched_at: now_unix(),
        };
        if let Ok(mut neg) = NEGATIVE_HLTB_CACHE
            .get_or_init(|| Mutex::new(HashMap::new()))
            .lock()
        {
            neg.remove(&game_id);
        }
        cache.insert(game_id, data.clone());
        save_hltb_cache(&app_handle, &cache);
        return Ok(data);
    }

    // Preserve previously cached custom or HLTB data if remote lookup fails
    if let Some(existing) = cache.get(&game_id).cloned() {
        if existing.source == "custom"
            || (existing.source == "hltb"
                && (existing.main_hours.is_some()
                    || existing.main_extra_hours.is_some()
                    || existing.completionist_hours.is_some()))
        {
            return Ok(existing);
        }
        // Remove any stale "none" or legacy "steamspy" entry from disk cache
        if existing.source == "none" || existing.source == "steamspy" {
            cache.remove(&game_id);
            save_hltb_cache(&app_handle, &cache);
        }
    }

    let empty = HltbData {
        game_id: game_id.clone(),
        title: game_name,
        main_hours: None,
        main_extra_hours: None,
        completionist_hours: None,
        source: "none".to_string(),
        fetched_at: now_unix(),
    };
    if let Ok(mut neg) = NEGATIVE_HLTB_CACHE
        .get_or_init(|| Mutex::new(HashMap::new()))
        .lock()
    {
        neg.insert(game_id, empty.clone());
    }
    Ok(empty)
}

#[tauri::command]
pub fn set_custom_hltb_data(
    app_handle: AppHandle,
    game_id: String,
    game_name: String,
    main_hours: Option<f32>,
    main_extra_hours: Option<f32>,
    completionist_hours: Option<f32>,
) -> Result<HltbData, String> {
    let mut cache = load_hltb_cache(&app_handle);
    let data = HltbData {
        game_id: game_id.clone(),
        title: game_name,
        main_hours: main_hours.filter(|&v| v > 0.0),
        main_extra_hours: main_extra_hours.filter(|&v| v > 0.0),
        completionist_hours: completionist_hours.filter(|&v| v > 0.0),
        source: "custom".to_string(),
        fetched_at: now_unix(),
    };
    if let Ok(mut neg) = NEGATIVE_HLTB_CACHE
        .get_or_init(|| Mutex::new(HashMap::new()))
        .lock()
    {
        neg.remove(&game_id);
    }
    cache.insert(game_id, data.clone());
    save_hltb_cache(&app_handle, &cache);
    Ok(data)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cleans_game_titles_and_rounds_hours() {
        assert_eq!(
            clean_game_title("The Witcher® 3: Wild Hunt™ - Complete Edition"),
            "The Witcher 3 Wild Hunt Complete Edition"
        );
        assert_eq!(round_hours(0), None);
        assert_eq!(round_hours(3600 * 12 + 1800), Some(12.5));
        assert_eq!(round_hours(3600 * 16 + 400), Some(16.0));
        assert_eq!(round_hours(3600 * 17 + 1500), Some(17.5));
        assert_eq!(round_hours(1800), Some(0.5));
        assert_eq!(round_hours(900), Some(0.3));
    }

    #[test]
    fn strips_edition_suffixes_cleanly() {
        assert_eq!(
            strip_edition_suffix("The Witcher 3 Wild Hunt Complete Edition"),
            "The Witcher 3 Wild Hunt"
        );
        assert_eq!(
            strip_edition_suffix("Fallout 4 Game of the Year Edition"),
            "Fallout 4"
        );
        assert_eq!(
            strip_edition_suffix("Death Stranding Director's Cut"),
            "Death Stranding"
        );
        assert_eq!(
            strip_edition_suffix("Dark Souls Remastered"),
            "Dark Souls"
        );
        assert_eq!(strip_edition_suffix("Hades"), "Hades");
        assert_eq!(strip_edition_suffix("Edition"), "Edition");
    }

    #[test]
    fn deserializes_init_token_response_with_hp_fields() {
        let raw = r#"{"token":"abc123","hpKey":"k_xyz","hpVal":"v_789"}"#;
        let parsed: InitTokenResponse = serde_json::from_str(raw).unwrap();
        assert_eq!(parsed.token.as_deref(), Some("abc123"));
        assert_eq!(parsed.hp_key.as_deref(), Some("k_xyz"));
        assert_eq!(parsed.hp_val.as_deref(), Some("v_789"));

        let legacy = r#"{"token":"only_tok"}"#;
        let parsed_legacy: InitTokenResponse = serde_json::from_str(legacy).unwrap();
        assert_eq!(parsed_legacy.token.as_deref(), Some("only_tok"));
        assert_eq!(parsed_legacy.hp_key, None);
        assert_eq!(parsed_legacy.hp_val, None);
    }

    #[test]
    fn extracts_api_base_single_and_two_segment() {
        let js = r#"const url="/api/locate/init?t="+Date.now();fetch("/api/locate",{headers:{"x-auth-token":t},body:JSON.stringify({searchTerms:q})})"#;
        assert_eq!(extract_api_base_from_js(js).as_deref(), Some("/api/locate"));

        let js_two_seg = r#"fetch("/api/v2/seek",{headers:{"x-hp-key":k},body:JSON.stringify({searchTerms:q})})"#;
        assert_eq!(
            extract_api_base_from_js(js_two_seg).as_deref(),
            Some("/api/v2/seek")
        );
    }

    #[test]
    fn parses_hltb_next_data_game_entry() {
        let html = r#"<!DOCTYPE html><html><head><script id="__NEXT_DATA__" type="application/json">{"props":{"pageProps":{"game":{"data":{"game":[{"game_id":141414,"game_name":"007 First Light","comp_main":57600,"comp_plus":68400,"comp_100":90000}]}}}}}</script></head><body></body></html>"#;
        let parsed = parse_hltb_next_data(html).expect("should parse __NEXT_DATA__");
        assert_eq!(parsed.game_id, 141414);
        assert_eq!(parsed.game_name, "007 First Light");
        assert_eq!(round_hours(parsed.comp_main), Some(16.0));
        assert_eq!(round_hours(parsed.comp_plus), Some(19.0));
        assert_eq!(round_hours(parsed.comp_100), Some(25.0));
    }

    #[test]
    fn extracts_hltb_game_ids_from_ddg_links_and_scores_similarity() {
        let html = r#"
            <a href="https://duckduckgo.com/l/?uddg=https%3A%2F%2Fhowlongtobeat.com%2Fgame%2F141414&rut=1">007</a>
            <a href="https://howlongtobeat.com/game/99999">Other</a>
        "#;
        assert_eq!(extract_hltb_game_ids_from_html(html), vec![141414, 99999]);
        assert!(title_similarity("007 First Light", "007: First Light") >= 0.45);
    }
}
