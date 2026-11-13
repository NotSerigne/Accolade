use image::imageops::FilterType;
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::{Mutex, OnceLock};

static THEME_COLOR_CACHE: OnceLock<Mutex<HashMap<String, String>>> = OnceLock::new();

const FALLBACK_PALETTE: &[&str] = &[
    "#e11d48", "#db2777", "#c026d3", "#9333ea", "#7c3aed", "#4f46e5", "#2563eb", "#0284c7",
    "#0891b2", "#0d9488", "#059669", "#16a34a", "#ca8a04", "#d97706", "#ea580c", "#dc2626",
];

fn fallback_palette_color(source: &str) -> String {
    let mut hash: u64 = 0xcbf29ce484222325;
    for &b in source.as_bytes() {
        hash ^= b as u64;
        hash = hash.wrapping_mul(0x100000001b3);
    }
    let idx = (hash as usize) % FALLBACK_PALETTE.len();
    FALLBACK_PALETTE[idx].to_string()
}

fn percent_decode_path(input: &str) -> String {
    let bytes = input.as_bytes();
    let mut out = Vec::with_capacity(bytes.len());
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i] == b'%' && i + 2 < bytes.len() {
            if let (Some(hi), Some(lo)) = (
                (bytes[i + 1] as char).to_digit(16),
                (bytes[i + 2] as char).to_digit(16),
            ) {
                out.push(((hi << 4) | lo) as u8);
                i += 3;
                continue;
            }
        }
        out.push(bytes[i]);
        i += 1;
    }
    String::from_utf8_lossy(&out).into_owned()
}

fn resolve_local_path(source: &str) -> PathBuf {
    if let Some(rest) = source.strip_prefix("file://") {
        let rest = rest.strip_prefix("localhost/").unwrap_or(rest);
        let decoded = percent_decode_path(rest);
        #[cfg(windows)]
        let normalized = {
            let b = decoded.as_bytes();
            if b.first() == Some(&b'/') && b.get(2) == Some(&b':') {
                &decoded[1..]
            } else {
                decoded.as_str()
            }
        };
        #[cfg(not(windows))]
        let normalized = decoded.as_str();
        PathBuf::from(normalized)
    } else {
        PathBuf::from(source)
    }
}

async fn load_image_bytes(source: &str) -> Option<Vec<u8>> {
    if source.starts_with("http://") || source.starts_with("https://") {
        let resp = crate::achievements::steam::steam_http_client()
            .get(source)
            .send()
            .await
            .ok()?;
        if !resp.status().is_success() {
            return None;
        }
        resp.bytes().await.ok().map(|b| b.to_vec())
    } else {
        let path = resolve_local_path(source);
        tauri::async_runtime::spawn_blocking(move || std::fs::read(path).ok())
            .await
            .ok()
            .flatten()
    }
}

fn rgb_to_hsl(r: f64, g: f64, b: f64) -> (f64, f64, f64) {
    let max = r.max(g).max(b);
    let min = r.min(g).min(b);
    let l = (max + min) / 2.0;

    if (max - min).abs() < f64::EPSILON {
        return (0.0, 0.0, l);
    }

    let d = max - min;
    let s = if l > 0.5 {
        d / (2.0 - max - min)
    } else {
        d / (max + min)
    };
    let h = if (max - r).abs() < f64::EPSILON {
        (g - b) / d + if g < b { 6.0 } else { 0.0 }
    } else if (max - g).abs() < f64::EPSILON {
        (b - r) / d + 2.0
    } else {
        (r - g) / d + 4.0
    };

    (h / 6.0, s, l)
}

fn hue_to_rgb(p: f64, q: f64, mut t: f64) -> f64 {
    if t < 0.0 {
        t += 1.0;
    }
    if t > 1.0 {
        t -= 1.0;
    }
    if t < 1.0 / 6.0 {
        return p + (q - p) * 6.0 * t;
    }
    if t < 0.5 {
        return q;
    }
    if t < 2.0 / 3.0 {
        return p + (q - p) * (2.0 / 3.0 - t) * 6.0;
    }
    p
}

fn hsl_to_rgb(h: f64, s: f64, l: f64) -> (u8, u8, u8) {
    if s <= 0.0 {
        let v = (l * 255.0).round().clamp(0.0, 255.0) as u8;
        return (v, v, v);
    }
    let q = if l < 0.5 {
        l * (1.0 + s)
    } else {
        l + s - l * s
    };
    let p = 2.0 * l - q;
    let r = hue_to_rgb(p, q, h + 1.0 / 3.0);
    let g = hue_to_rgb(p, q, h);
    let b = hue_to_rgb(p, q, h - 1.0 / 3.0);
    (
        (r * 255.0).round().clamp(0.0, 255.0) as u8,
        (g * 255.0).round().clamp(0.0, 255.0) as u8,
        (b * 255.0).round().clamp(0.0, 255.0) as u8,
    )
}

fn extract_color_from_bytes(bytes: &[u8]) -> Option<String> {
    let img = image::load_from_memory(bytes).ok()?;
    let rgb = img.resize_exact(32, 32, FilterType::Triangle).to_rgb8();

    let mut sum_r = 0.0_f64;
    let mut sum_g = 0.0_f64;
    let mut sum_b = 0.0_f64;
    let mut total_weight = 0.0_f64;

    for pixel in rgb.pixels() {
        let [r, g, b] = pixel.0;
        if (r < 28 && g < 28 && b < 28) || (r > 235 && g > 235 && b > 235) {
            continue;
        }
        let max = r.max(g).max(b);
        let min = r.min(g).min(b);
        let sat = (max - min) as f64 + 1.0;
        let weight = sat * sat;
        sum_r += (r as f64) * weight;
        sum_g += (g as f64) * weight;
        sum_b += (b as f64) * weight;
        total_weight += weight;
    }

    if total_weight <= 0.0 {
        return None;
    }

    let avg_r = (sum_r / total_weight) / 255.0;
    let avg_g = (sum_g / total_weight) / 255.0;
    let avg_b = (sum_b / total_weight) / 255.0;

    let (h, s, l) = rgb_to_hsl(avg_r, avg_g, avg_b);
    let clamped_s = s.clamp(0.55, 0.85);
    let clamped_l = l.clamp(0.55, 0.68);
    let (out_r, out_g, out_b) = hsl_to_rgb(h, clamped_s, clamped_l);

    Some(format!("#{:02x}{:02x}{:02x}", out_r, out_g, out_b))
}

/// Extract a legible vibrant theme color from an image URL or local file path.
pub async fn extract_dominant_color(image_source: &str) -> Option<String> {
    let key = image_source.trim();
    if key.is_empty() {
        return Some(fallback_palette_color(""));
    }

    let cache = THEME_COLOR_CACHE.get_or_init(|| Mutex::new(HashMap::new()));
    if let Ok(guard) = cache.lock() {
        if let Some(cached) = guard.get(key) {
            return Some(cached.clone());
        }
    }

    let extracted = match load_image_bytes(key).await {
        Some(bytes) => tauri::async_runtime::spawn_blocking(move || extract_color_from_bytes(&bytes))
            .await
            .ok()
            .flatten(),
        None => None,
    };

    let color = extracted.unwrap_or_else(|| fallback_palette_color(key));
    if let Ok(mut guard) = cache.lock() {
        guard.insert(key.to_string(), color.clone());
    }
    Some(color)
}

#[cfg(test)]
mod tests {
    use super::*;
    use image::{ImageFormat, Rgb, RgbImage};
    use std::io::Cursor;

    #[test]
    fn extracts_vibrant_color_over_neutral_background() {
        let mut img = RgbImage::new(32, 32);
        for (x, y, pixel) in img.enumerate_pixels_mut() {
            if x < 4 && y < 4 {
                *pixel = Rgb([10, 10, 10]); // near-black skipped
            } else if x < 8 && y < 8 {
                *pixel = Rgb([250, 250, 250]); // near-white skipped
            } else if x >= 16 {
                *pixel = Rgb([220, 40, 40]); // vibrant red
            } else {
                *pixel = Rgb([120, 120, 120]); // neutral gray
            }
        }
        let mut buf = Vec::new();
        img.write_to(&mut Cursor::new(&mut buf), ImageFormat::Png)
            .unwrap();

        let hex = extract_color_from_bytes(&buf).expect("color should be extracted");
        assert!(hex.starts_with('#') && hex.len() == 7);
        let r = u8::from_str_radix(&hex[1..3], 16).unwrap();
        let g = u8::from_str_radix(&hex[3..5], 16).unwrap();
        let b = u8::from_str_radix(&hex[5..7], 16).unwrap();
        assert!(r > g && r > b, "expected red-dominant color, got {}", hex);
    }

    #[test]
    fn fallback_palette_is_deterministic_and_hex() {
        let a = fallback_palette_color("https://example.com/cover.jpg");
        let b = fallback_palette_color("https://example.com/cover.jpg");
        assert_eq!(a, b);
        assert!(FALLBACK_PALETTE.contains(&a.as_str()));
    }
}
