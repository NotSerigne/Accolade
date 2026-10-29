// src-tauri/src/capture.rs
//! Shared Windows Graphics Capture (WGC) helpers used by screenshots and the replay buffer.
//!
//! When the display runs in HDR / Advanced Color mode, the desktop is composed in scRGB
//! (linear, Rec.709 primaries, 1.0 = 80 nits). Grabbing it as 8-bit RGBA lets Windows clip
//! that signal, which produces washed-out / blown-out colors. Instead we capture as
//! `Rgba16F` and tone-map to SDR ourselves, using the user's "SDR content brightness" as
//! the reference white so the result matches what is seen on screen.

use std::sync::{mpsc, OnceLock};
use std::time::Duration;

use windows_capture::{
    capture::{Context, GraphicsCaptureApiHandler},
    frame::Frame,
    graphics_capture_api::{GraphicsCaptureApi, InternalCaptureControl},
    monitor::Monitor,
    settings::{
        ColorFormat, CursorCaptureSettings, DirtyRegionSettings, DrawBorderSettings,
        MinimumUpdateIntervalSettings, SecondaryWindowSettings, Settings,
    },
};

/// Color state of the primary display.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct DisplayColorInfo {
    /// Desktop is composed in scRGB (HDR or Auto Color Management). Capture must use FP16.
    pub advanced_color: bool,
    /// True HDR output (as opposed to ACM on an SDR panel).
    pub hdr: bool,
    /// scRGB value of SDR reference white (1.0 = 80 nits).
    pub sdr_white_scale: f32,
}

impl Default for DisplayColorInfo {
    fn default() -> Self {
        Self {
            advanced_color: false,
            hdr: false,
            sdr_white_scale: 1.0,
        }
    }
}

impl DisplayColorInfo {
    pub fn capture_format(&self) -> ColorFormat {
        if self.advanced_color {
            ColorFormat::Rgba16F
        } else {
            ColorFormat::Rgba8
        }
    }
}

/// Queries the primary display's Advanced Color state and SDR white level.
pub fn primary_display_color_info() -> DisplayColorInfo {
    unsafe { query_primary_display_color_info() }.unwrap_or_default()
}

unsafe fn query_primary_display_color_info() -> Option<DisplayColorInfo> {
    use windows::Win32::Devices::Display::{
        DisplayConfigGetDeviceInfo, GetDisplayConfigBufferSizes, QueryDisplayConfig,
        DISPLAYCONFIG_DEVICE_INFO_GET_ADVANCED_COLOR_INFO,
        DISPLAYCONFIG_DEVICE_INFO_GET_SDR_WHITE_LEVEL, DISPLAYCONFIG_GET_ADVANCED_COLOR_INFO,
        DISPLAYCONFIG_MODE_INFO, DISPLAYCONFIG_MODE_INFO_TYPE_SOURCE, DISPLAYCONFIG_PATH_INFO,
        DISPLAYCONFIG_SDR_WHITE_LEVEL, QDC_ONLY_ACTIVE_PATHS,
    };
    use windows::Win32::Foundation::ERROR_SUCCESS;

    let mut path_count = 0u32;
    let mut mode_count = 0u32;
    if GetDisplayConfigBufferSizes(QDC_ONLY_ACTIVE_PATHS, &mut path_count, &mut mode_count)
        != ERROR_SUCCESS
    {
        return None;
    }

    let mut paths = vec![DISPLAYCONFIG_PATH_INFO::default(); path_count as usize];
    let mut modes = vec![DISPLAYCONFIG_MODE_INFO::default(); mode_count as usize];
    if QueryDisplayConfig(
        QDC_ONLY_ACTIVE_PATHS,
        &mut path_count,
        paths.as_mut_ptr(),
        &mut mode_count,
        modes.as_mut_ptr(),
        None,
    ) != ERROR_SUCCESS
    {
        return None;
    }
    paths.truncate(path_count as usize);
    modes.truncate(mode_count as usize);

    // The primary monitor is the one whose source mode sits at desktop origin (0, 0).
    let is_primary = |p: &DISPLAYCONFIG_PATH_INFO| {
        modes.iter().any(|m| {
            m.infoType == DISPLAYCONFIG_MODE_INFO_TYPE_SOURCE
                && m.id == p.sourceInfo.id
                && m.adapterId.LowPart == p.sourceInfo.adapterId.LowPart
                && m.adapterId.HighPart == p.sourceInfo.adapterId.HighPart
                && {
                    let pos = m.Anonymous.sourceMode.position;
                    pos.x == 0 && pos.y == 0
                }
        })
    };
    let path = paths.iter().find(|p| is_primary(p)).or(paths.first())?;

    let mut info = DisplayColorInfo::default();

    let mut color = DISPLAYCONFIG_GET_ADVANCED_COLOR_INFO::default();
    color.header.r#type = DISPLAYCONFIG_DEVICE_INFO_GET_ADVANCED_COLOR_INFO;
    color.header.size = std::mem::size_of::<DISPLAYCONFIG_GET_ADVANCED_COLOR_INFO>() as u32;
    color.header.adapterId = path.targetInfo.adapterId;
    color.header.id = path.targetInfo.id;
    if DisplayConfigGetDeviceInfo(&mut color.header) == 0 {
        // bit0 = supported, bit1 = enabled, bit2 = wideColorEnforced (ACM on SDR panel)
        let bits = color.Anonymous.value;
        let enabled = bits & 0b010 != 0;
        let wide_color_enforced = bits & 0b100 != 0;
        info.advanced_color = enabled;
        info.hdr = enabled && !wide_color_enforced;
    }

    let mut white = DISPLAYCONFIG_SDR_WHITE_LEVEL::default();
    white.header.r#type = DISPLAYCONFIG_DEVICE_INFO_GET_SDR_WHITE_LEVEL;
    white.header.size = std::mem::size_of::<DISPLAYCONFIG_SDR_WHITE_LEVEL>() as u32;
    white.header.adapterId = path.targetInfo.adapterId;
    white.header.id = path.targetInfo.id;
    if DisplayConfigGetDeviceInfo(&mut white.header) == 0 && white.SDRWhiteLevel > 0 {
        // Expressed in 1/1000 of 80 nits => directly the scRGB multiplier once divided by 1000.
        info.sdr_white_scale = (white.SDRWhiteLevel as f32 / 1000.0).clamp(1.0, 6.25);
    }

    Some(info)
}

// ---------------------------------------------------------------------------
// Pixel conversion
// ---------------------------------------------------------------------------

fn half_to_f32(h: u16) -> f32 {
    let sign = ((h >> 15) & 1) as u32;
    let exp = ((h >> 10) & 0x1f) as i32;
    let mant = (h & 0x3ff) as u32;
    let bits = if exp == 0 {
        if mant == 0 {
            sign << 31
        } else {
            // Subnormal: normalise the mantissa.
            let mut e: i32 = 127 - 15 + 1;
            let mut m = mant;
            while m & 0x400 == 0 {
                m <<= 1;
                e -= 1;
            }
            (sign << 31) | ((e as u32) << 23) | ((m & 0x3ff) << 13)
        }
    } else if exp == 31 {
        (sign << 31) | (0xff << 23) | (mant << 13)
    } else {
        (sign << 31) | (((exp + 127 - 15) as u32) << 23) | (mant << 13)
    };
    f32::from_bits(bits)
}

fn half_lut() -> &'static [f32] {
    static LUT: OnceLock<Vec<f32>> = OnceLock::new();
    LUT.get_or_init(|| {
        (0..=u16::MAX)
            .map(|h| {
                let v = half_to_f32(h);
                if v.is_finite() && v > 0.0 {
                    v
                } else {
                    0.0
                }
            })
            .collect()
    })
}

const SRGB_LUT_SIZE: usize = 16384;

fn srgb_lut() -> &'static [u8] {
    static LUT: OnceLock<Vec<u8>> = OnceLock::new();
    LUT.get_or_init(|| {
        (0..SRGB_LUT_SIZE)
            .map(|i| {
                let c = i as f32 / (SRGB_LUT_SIZE - 1) as f32;
                let s = if c <= 0.003_130_8 {
                    12.92 * c
                } else {
                    1.055 * c.powf(1.0 / 2.4) - 0.055
                };
                (s * 255.0).round().clamp(0.0, 255.0) as u8
            })
            .collect()
    })
}

/// Maps linear scRGB to display-referred SDR sRGB.
///
/// Everything below `KNEE` (relative to SDR white) is left untouched so SDR content and
/// mid-tones look exactly like on screen; only highlights above it are smoothly rolled off
/// (hue-preserving extended Reinhard) so that the frame's peak lands on pure white.
#[derive(Clone, Copy)]
struct ToneMapper {
    inv_white: f32,
    /// Peak brightness of the frame, expressed in "knee space" units (>= 1).
    white_e_sq: f32,
}

const KNEE: f32 = 0.75;

impl ToneMapper {
    fn new(sdr_white_scale: f32, peak_normalized: f32) -> Self {
        let peak = peak_normalized.clamp(1.0, 64.0);
        let white_e = (peak - KNEE) / (1.0 - KNEE);
        Self {
            inv_white: 1.0 / sdr_white_scale.max(1.0),
            white_e_sq: white_e * white_e,
        }
    }

    #[inline]
    fn map(&self, lut: &[u8], r: f32, g: f32, b: f32) -> [u8; 3] {
        let mut r = r * self.inv_white;
        let mut g = g * self.inv_white;
        let mut b = b * self.inv_white;
        let m = r.max(g).max(b);
        if m > KNEE {
            let e = (m - KNEE) / (1.0 - KNEE);
            let e2 = e * (1.0 + e / self.white_e_sq) / (1.0 + e);
            let target = (KNEE + (1.0 - KNEE) * e2).min(1.0);
            let s = target / m;
            r *= s;
            g *= s;
            b *= s;
        }
        let max_idx = (SRGB_LUT_SIZE - 1) as f32;
        [
            lut[(r.clamp(0.0, 1.0) * max_idx + 0.5) as usize],
            lut[(g.clamp(0.0, 1.0) * max_idx + 0.5) as usize],
            lut[(b.clamp(0.0, 1.0) * max_idx + 0.5) as usize],
        ]
    }
}

#[inline]
fn read_f16_rgb(data: &[u8], offset: usize, half: &[f32]) -> (f32, f32, f32) {
    let px = &data[offset..offset + 6];
    (
        half[u16::from_le_bytes([px[0], px[1]]) as usize],
        half[u16::from_le_bytes([px[2], px[3]]) as usize],
        half[u16::from_le_bytes([px[4], px[5]]) as usize],
    )
}

/// Estimates the frame's highlight peak (99.7th percentile of max(R,G,B)), relative to SDR white.
fn estimate_peak(data: &[u8], width: u32, height: u32, pitch: usize, sdr_white: f32) -> f32 {
    let half = half_lut();
    let step_x = (width / 320).max(1) as usize;
    let step_y = (height / 180).max(1) as usize;
    let inv_white = 1.0 / sdr_white.max(1.0);
    let mut samples: Vec<f32> = Vec::with_capacity(320 * 180 + 1024);
    for y in (0..height as usize).step_by(step_y) {
        let row = y * pitch;
        for x in (0..width as usize).step_by(step_x) {
            let (r, g, b) = read_f16_rgb(data, row + x * 8, half);
            samples.push(r.max(g).max(b) * inv_white);
        }
    }
    if samples.is_empty() {
        return 1.0;
    }
    let idx = ((samples.len() as f32 * 0.997) as usize).min(samples.len() - 1);
    let (_, p, _) = samples.select_nth_unstable_by(idx, |a, b| a.total_cmp(b));
    p.max(1.0)
}

/// A raw frame copied out of the capture pool (row padding included).
pub struct RawFrame {
    pub width: u32,
    pub height: u32,
    pub row_pitch: usize,
    pub format: ColorFormat,
    pub data: Vec<u8>,
}

/// Converts a captured frame to a tightly packed RGBA8 buffer (full resolution).
pub fn raw_to_rgba8(raw: &RawFrame, info: &DisplayColorInfo) -> Vec<u8> {
    let w = raw.width as usize;
    let h = raw.height as usize;
    let mut out = vec![0u8; w * h * 4];
    if w == 0 || h == 0 {
        return out;
    }

    match raw.format {
        ColorFormat::Rgba8 | ColorFormat::Bgra8 => {
            let bgra = raw.format == ColorFormat::Bgra8;
            for (y, dst_row) in out.chunks_exact_mut(w * 4).enumerate() {
                let src = &raw.data[y * raw.row_pitch..y * raw.row_pitch + w * 4];
                for (d, s) in dst_row.chunks_exact_mut(4).zip(src.chunks_exact(4)) {
                    if bgra {
                        d[0] = s[2];
                        d[1] = s[1];
                        d[2] = s[0];
                    } else {
                        d[..3].copy_from_slice(&s[..3]);
                    }
                    d[3] = 255;
                }
            }
        }
        ColorFormat::Rgba16F => {
            let peak = estimate_peak(
                &raw.data,
                raw.width,
                raw.height,
                raw.row_pitch,
                info.sdr_white_scale,
            );
            let tm = ToneMapper::new(info.sdr_white_scale, peak);
            let half = half_lut();
            let lut = srgb_lut();
            let threads = std::thread::available_parallelism()
                .map(|n| n.get())
                .unwrap_or(4)
                .clamp(1, 8);
            let rows_per_chunk = h.div_ceil(threads);
            std::thread::scope(|scope| {
                for (chunk_idx, chunk) in out.chunks_mut(rows_per_chunk * w * 4).enumerate() {
                    let data = &raw.data;
                    let pitch = raw.row_pitch;
                    scope.spawn(move || {
                        let first_row = chunk_idx * rows_per_chunk;
                        for (i, dst_row) in chunk.chunks_exact_mut(w * 4).enumerate() {
                            let row = (first_row + i) * pitch;
                            for (x, d) in dst_row.chunks_exact_mut(4).enumerate() {
                                let (r, g, b) = read_f16_rgb(data, row + x * 8, half);
                                let [r8, g8, b8] = tm.map(lut, r, g, b);
                                d[0] = r8;
                                d[1] = g8;
                                d[2] = b8;
                                d[3] = 255;
                            }
                        }
                    });
                }
            });
        }
    }
    out
}

/// Downscales a mapped frame buffer to `target_width` (2x2 supersampled) and converts it to RGBA8.
/// Reads straight from the mapped GPU buffer so no full-resolution copy is made.
pub fn downscale_to_rgba8(
    data: &[u8],
    width: u32,
    height: u32,
    pitch: usize,
    format: ColorFormat,
    info: &DisplayColorInfo,
    target_width: u32,
) -> (u32, u32, Vec<u8>) {
    let src_w = width.max(1);
    let src_h = height.max(1);
    let dst_w = target_width.min(src_w).max(1);
    let dst_h = ((src_h as u64 * dst_w as u64) / src_w as u64).max(1) as u32;
    let mut out = vec![0u8; (dst_w * dst_h * 4) as usize];

    let sx = src_w as f32 / dst_w as f32;
    let sy = src_h as f32 / dst_h as f32;
    // 2x2 sample points inside each destination cell.
    let offsets = [(0.25f32, 0.25f32), (0.75, 0.25), (0.25, 0.75), (0.75, 0.75)];

    let sample_pos = |dx: u32, dy: u32, o: (f32, f32)| -> (usize, usize) {
        let x = (((dx as f32 + o.0) * sx) as u32).min(src_w - 1) as usize;
        let y = (((dy as f32 + o.1) * sy) as u32).min(src_h - 1) as usize;
        (x, y)
    };

    match format {
        ColorFormat::Rgba8 | ColorFormat::Bgra8 => {
            let bgra = format == ColorFormat::Bgra8;
            for dy in 0..dst_h {
                for dx in 0..dst_w {
                    let mut acc = [0u32; 3];
                    for o in offsets {
                        let (x, y) = sample_pos(dx, dy, o);
                        let off = y * pitch + x * 4;
                        let px = &data[off..off + 4];
                        if bgra {
                            acc[0] += px[2] as u32;
                            acc[1] += px[1] as u32;
                            acc[2] += px[0] as u32;
                        } else {
                            acc[0] += px[0] as u32;
                            acc[1] += px[1] as u32;
                            acc[2] += px[2] as u32;
                        }
                    }
                    let o = ((dy * dst_w + dx) * 4) as usize;
                    out[o] = (acc[0] / 4) as u8;
                    out[o + 1] = (acc[1] / 4) as u8;
                    out[o + 2] = (acc[2] / 4) as u8;
                    out[o + 3] = 255;
                }
            }
        }
        ColorFormat::Rgba16F => {
            let half = half_lut();
            let lut = srgb_lut();
            let peak = estimate_peak(data, width, height, pitch, info.sdr_white_scale);
            let tm = ToneMapper::new(info.sdr_white_scale, peak);
            for dy in 0..dst_h {
                for dx in 0..dst_w {
                    // Average in linear light, then tone-map once.
                    let mut acc = [0f32; 3];
                    for o in offsets {
                        let (x, y) = sample_pos(dx, dy, o);
                        let (r, g, b) = read_f16_rgb(data, y * pitch + x * 8, half);
                        acc[0] += r;
                        acc[1] += g;
                        acc[2] += b;
                    }
                    let [r8, g8, b8] = tm.map(lut, acc[0] * 0.25, acc[1] * 0.25, acc[2] * 0.25);
                    let o = ((dy * dst_w + dx) * 4) as usize;
                    out[o] = r8;
                    out[o + 1] = g8;
                    out[o + 2] = b8;
                    out[o + 3] = 255;
                }
            }
        }
    }

    (dst_w, dst_h, out)
}

// ---------------------------------------------------------------------------
// Capture sessions
// ---------------------------------------------------------------------------

pub fn cursor_settings() -> CursorCaptureSettings {
    if GraphicsCaptureApi::is_cursor_settings_supported().unwrap_or(false) {
        CursorCaptureSettings::WithoutCursor
    } else {
        CursorCaptureSettings::Default
    }
}

pub fn border_settings() -> DrawBorderSettings {
    if GraphicsCaptureApi::is_border_settings_supported().unwrap_or(false) {
        DrawBorderSettings::WithoutBorder
    } else {
        DrawBorderSettings::Default
    }
}

pub fn min_update_interval(interval: Duration) -> MinimumUpdateIntervalSettings {
    if GraphicsCaptureApi::is_minimum_update_interval_supported().unwrap_or(false) {
        MinimumUpdateIntervalSettings::Custom(interval)
    } else {
        MinimumUpdateIntervalSettings::Default
    }
}

/// Builds monitor capture settings for the primary display.
pub fn primary_monitor_settings<F>(
    info: &DisplayColorInfo,
    interval: Option<Duration>,
    flags: F,
) -> Result<Settings<F, Monitor>, String> {
    let monitor = Monitor::primary().map_err(|e| e.to_string())?;
    Ok(Settings::new(
        monitor,
        cursor_settings(),
        border_settings(),
        SecondaryWindowSettings::Default,
        interval
            .map(min_update_interval)
            .unwrap_or(MinimumUpdateIntervalSettings::Default),
        DirtyRegionSettings::Default,
        info.capture_format(),
        flags,
    ))
}

type FrameResult = Result<RawFrame, String>;

struct OneShotCapture {
    tx: Option<mpsc::Sender<FrameResult>>,
}

impl GraphicsCaptureApiHandler for OneShotCapture {
    type Flags = mpsc::Sender<FrameResult>;
    type Error = Box<dyn std::error::Error + Send + Sync>;

    fn new(ctx: Context<Self::Flags>) -> Result<Self, Self::Error> {
        Ok(Self {
            tx: Some(ctx.flags),
        })
    }

    fn on_frame_arrived(
        &mut self,
        frame: &mut Frame,
        capture_control: InternalCaptureControl,
    ) -> Result<(), Self::Error> {
        let format = frame.color_format();
        let result = match frame.buffer() {
            Ok(mut buffer) => {
                let width = buffer.width();
                let height = buffer.height();
                let row_pitch = buffer.row_pitch() as usize;
                Ok(RawFrame {
                    width,
                    height,
                    row_pitch,
                    format,
                    data: buffer.as_raw_buffer().to_vec(),
                })
            }
            Err(e) => Err(e.to_string()),
        };
        if let Some(tx) = self.tx.take() {
            let _ = tx.send(result);
        }
        capture_control.stop();
        Ok(())
    }
}

/// Grabs a single frame of the primary monitor through WGC (HDR-aware).
pub fn capture_primary_monitor(info: &DisplayColorInfo) -> Result<RawFrame, String> {
    let (tx, rx) = mpsc::channel();
    let settings = primary_monitor_settings(info, None, tx)?;
    let control = OneShotCapture::start_free_threaded(settings).map_err(|e| e.to_string())?;
    let result = rx.recv_timeout(Duration::from_secs(3));
    let _ = control.stop();
    match result {
        Ok(frame) => frame,
        Err(_) => Err("Timed out waiting for a captured frame".to_string()),
    }
}
