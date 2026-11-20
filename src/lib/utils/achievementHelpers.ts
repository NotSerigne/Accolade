/** Canonical achievement marker tag keys stored by the backend and UI. */
export const ACHIEVEMENT_TAG_KEYS = ['missable', 'multi', 'ngplus', 'buggy'] as const;

/** Canonical achievement marker tag union. */
export type AchievementTagKey = (typeof ACHIEVEMENT_TAG_KEYS)[number];

const TAG_ALIASES: Record<string, AchievementTagKey> = {
	manquable: 'missable',
	missable: 'missable',
	multi: 'multi',
	multiplayer: 'multi',
	coop: 'multi',
	'ng+': 'ngplus',
	ngplus: 'ngplus',
	'new game+': 'ngplus',
	'newgame+': 'ngplus',
	buggué: 'buggy',
	buggue: 'buggy',
	buggy: 'buggy',
	glitched: 'buggy'
};

/**
 * Normalizes a raw achievement marker tag (FR/EN aliases) to its canonical key,
 * returning `null` for empty or unrecognized values.
 */
export function normalizeAchievementTagKey(raw: string): AchievementTagKey | null {
	const normalized = raw.trim().toLowerCase();
	if (!normalized) return null;
	return TAG_ALIASES[normalized] ?? null;
}

/**
 * Normalizes and deduplicates an array of raw achievement marker tags,
 * preserving first-seen canonical order.
 */
export function normalizeAchievementTags(tags?: readonly string[] | null): AchievementTagKey[] {
	if (!tags || tags.length === 0) return [];
	const seen = new Set<AchievementTagKey>();
	const result: AchievementTagKey[] = [];
	for (const raw of tags) {
		const canonical = normalizeAchievementTagKey(raw);
		if (canonical && !seen.has(canonical)) {
			seen.add(canonical);
			result.push(canonical);
		}
	}
	return result;
}

/**
 * Computes estimated remaining hours to reach 100% completion from an HLTB
 * completionist estimate and accumulated playtime in seconds, clamped at `0`.
 * Returns `null` when no valid positive completionist estimate is available.
 */
export function computeRemainingCompletionistHours(
	completionistHours: number | null | undefined,
	playtimeSeconds: number
): number | null {
	if (
		typeof completionistHours !== 'number' ||
		!Number.isFinite(completionistHours) ||
		completionistHours <= 0
	) {
		return null;
	}
	const safePlaytimeSeconds =
		typeof playtimeSeconds === 'number' && Number.isFinite(playtimeSeconds) && playtimeSeconds > 0
			? playtimeSeconds
			: 0;
	return Math.max(0, completionistHours - safePlaytimeSeconds / 3600);
}

/**
 * Formats a remaining duration in hours using compact French/UI notation:
 * `'0h'`, `'45min'`, `'3h'`, or `'2h15'`.
 */
export function formatRemainingTimeHours(hours: number): string {
	if (typeof hours !== 'number' || !Number.isFinite(hours)) return '0h';
	const totalMinutes = Math.max(0, Math.round(hours * 60));
	if (totalMinutes === 0) return '0h';
	const h = Math.floor(totalMinutes / 60);
	const m = totalMinutes % 60;
	if (h === 0) return `${m}min`;
	if (m === 0) return `${h}h`;
	return `${h}h${String(m).padStart(2, '0')}`;
}

/** 7-tier rarity taxonomy ordered from rarest (`0`) to most common (`6`). */
export const RARITY_TIERS_7 = [
	'mythic',
	'legendary',
	'epic',
	'very-rare',
	'rare',
	'uncommon',
	'common'
] as const;

/** Canonical 7-tier rarity union matching `rarityLabelByIndex` and `--rarity-*` CSS tokens. */
export type RarityTier7 = (typeof RARITY_TIERS_7)[number];

/**
 * Classifies a global completion percentage into one of 7 rarity tiers.
 * Null, undefined, empty, non-finite, or negative inputs fall back to `'common'`.
 */
export function getRarityTier7(percent?: number | string | null): RarityTier7 {
	if (percent === undefined || percent === null) return 'common';
	let val: number;
	if (typeof percent === 'number') {
		val = percent;
	} else {
		const cleaned = percent.trim().replace(/%$/, '').trim().replace(',', '.');
		if (!cleaned) return 'common';
		val = Number(cleaned);
	}
	if (!Number.isFinite(val) || val < 0) return 'common';
	if (val <= 0.1) return 'mythic';
	if (val <= 1) return 'legendary';
	if (val <= 3) return 'epic';
	if (val <= 7) return 'very-rare';
	if (val <= 15) return 'rare';
	if (val <= 35) return 'uncommon';
	return 'common';
}

/**
 * Returns the `0..6` index of a 7-tier rarity matching `RARITY_TIERS_7` and `rarityLabelByIndex`.
 */
export function getRarityTierIndex(tier: RarityTier7): number {
	const index = RARITY_TIERS_7.indexOf(tier);
	return index >= 0 ? index : RARITY_TIERS_7.length - 1;
}

/**
 * Returns the CSS custom property reference for a 7-tier rarity (`var(--rarity-*)`).
 */
export function getRarityCssVar(tier: RarityTier7): string {
	return `var(--rarity-${tier})`;
}

type RgbTuple = [number, number, number];

const DARK_TEXT_RGB: RgbTuple = [17, 17, 17];
const LIGHT_TEXT_RGB: RgbTuple = [255, 255, 255];
const DARK_SURFACE_RGB: RgbTuple = [26, 27, 32];
const LIGHT_SURFACE_RGB: RgbTuple = [237, 241, 246];
const MIN_AA_CONTRAST = 4.6;

function clampByte(value: number): number {
	return Math.min(255, Math.max(0, Math.round(value)));
}

/**
 * Parses `#rgb`, `#rrggbb`, and `rgb(...)` / `rgba(...)` color strings into clamped
 * `[0..255]` integer RGB channels, returning `null` if invalid.
 */
export function parseCssRgb(color: string): RgbTuple | null {
	if (typeof color !== 'string') return null;
	const trimmed = color.trim();
	if (!trimmed) return null;

	const hexMatch = trimmed.match(/^#([0-9a-f]{3}|[0-9a-f]{6})$/i);
	if (hexMatch) {
		const hex = hexMatch[1];
		if (hex.length === 3) {
			return [
				parseInt(hex[0] + hex[0], 16),
				parseInt(hex[1] + hex[1], 16),
				parseInt(hex[2] + hex[2], 16)
			];
		}
		return [
			parseInt(hex.slice(0, 2), 16),
			parseInt(hex.slice(2, 4), 16),
			parseInt(hex.slice(4, 6), 16)
		];
	}

	const num = '[+-]?(?:\\d+(?:\\.\\d+)?|\\.\\d+)';
	const alpha = '[+-]?(?:\\d+(?:\\.\\d+)?|\\.\\d+)%?';
	const commaMatch = trimmed.match(
		new RegExp(
			`^rgba?\\(\\s*(${num})\\s*,\\s*(${num})\\s*,\\s*(${num})(?:\\s*,\\s*${alpha})?\\s*\\)$`,
			'i'
		)
	);
	if (commaMatch) {
		const r = Number(commaMatch[1]);
		const g = Number(commaMatch[2]);
		const b = Number(commaMatch[3]);
		if (Number.isFinite(r) && Number.isFinite(g) && Number.isFinite(b)) {
			return [clampByte(r), clampByte(g), clampByte(b)];
		}
		return null;
	}

	const spaceMatch = trimmed.match(
		new RegExp(`^rgba?\\(\\s*(${num})\\s+(${num})\\s+(${num})(?:\\s*\\/\\s*${alpha})?\\s*\\)$`, 'i')
	);
	if (spaceMatch) {
		const r = Number(spaceMatch[1]);
		const g = Number(spaceMatch[2]);
		const b = Number(spaceMatch[3]);
		if (Number.isFinite(r) && Number.isFinite(g) && Number.isFinite(b)) {
			return [clampByte(r), clampByte(g), clampByte(b)];
		}
	}

	return null;
}

function linearizeChannel(channel: number): number {
	const srgb = channel / 255;
	return srgb <= 0.03928 ? srgb / 12.92 : Math.pow((srgb + 0.055) / 1.055, 2.4);
}

function getRelativeLuminance([r, g, b]: RgbTuple): number {
	return 0.2126 * linearizeChannel(r) + 0.7152 * linearizeChannel(g) + 0.0722 * linearizeChannel(b);
}

function getContrastRatioFromLuminance(l1: number, l2: number): number {
	const lighter = Math.max(l1, l2);
	const darker = Math.min(l1, l2);
	return (lighter + 0.05) / (darker + 0.05);
}

const DARK_TEXT_LUM = getRelativeLuminance(DARK_TEXT_RGB);
const LIGHT_TEXT_LUM = getRelativeLuminance(LIGHT_TEXT_RGB);
const DARK_SURFACE_LUM = getRelativeLuminance(DARK_SURFACE_RGB);
const LIGHT_SURFACE_LUM = getRelativeLuminance(LIGHT_SURFACE_RGB);

function rgbToHex([r, g, b]: RgbTuple): string {
	return (
		'#' + [r, g, b].map((channel) => clampByte(channel).toString(16).padStart(2, '0')).join('')
	);
}

function getContrastTextFromRgb(rgb: RgbTuple): '#111111' | '#ffffff' {
	const bgLum = getRelativeLuminance(rgb);
	const darkContrast = getContrastRatioFromLuminance(bgLum, DARK_TEXT_LUM);
	const lightContrast = getContrastRatioFromLuminance(bgLum, LIGHT_TEXT_LUM);
	return darkContrast >= lightContrast ? '#111111' : '#ffffff';
}

/**
 * Returns `#111111` or `#ffffff` depending on which provides higher WCAG contrast
 * against the supplied background color (defaults to `'#ffffff'` if unparseable).
 */
export function getContrastTextForBg(color: string): '#111111' | '#ffffff' {
	const rgb = parseCssRgb(color);
	if (!rgb) return '#ffffff';
	return getContrastTextFromRgb(rgb);
}

function rgbToHsl([r, g, b]: RgbTuple): [number, number, number] {
	const rn = r / 255;
	const gn = g / 255;
	const bn = b / 255;
	const max = Math.max(rn, gn, bn);
	const min = Math.min(rn, gn, bn);
	const l = (max + min) / 2;

	if (max === min) {
		return [0, 0, l];
	}

	const d = max - min;
	const s = l > 0.5 ? d / (2 - max - min) : d / (max + min);
	let h: number;
	if (max === rn) {
		h = (gn - bn) / d + (gn < bn ? 6 : 0);
	} else if (max === gn) {
		h = (bn - rn) / d + 2;
	} else {
		h = (rn - gn) / d + 4;
	}
	return [h / 6, s, l];
}

function hueToChannel(p: number, q: number, t: number): number {
	let wrapped = t;
	if (wrapped < 0) wrapped += 1;
	if (wrapped > 1) wrapped -= 1;
	if (wrapped < 1 / 6) return p + (q - p) * 6 * wrapped;
	if (wrapped < 1 / 2) return q;
	if (wrapped < 2 / 3) return p + (q - p) * (2 / 3 - wrapped) * 6;
	return p;
}

function hslToRgb(h: number, s: number, l: number): RgbTuple {
	if (s === 0) {
		const gray = clampByte(l * 255);
		return [gray, gray, gray];
	}
	const q = l < 0.5 ? l * (1 + s) : l + s - l * s;
	const p = 2 * l - q;
	return [
		clampByte(hueToChannel(p, q, h + 1 / 3) * 255),
		clampByte(hueToChannel(p, q, h) * 255),
		clampByte(hueToChannel(p, q, h - 1 / 3) * 255)
	];
}

function isAccentCompliant(rgb: RgbTuple, theme: 'dark' | 'light'): boolean {
	const lum = getRelativeLuminance(rgb);
	const surfaceLum = theme === 'dark' ? DARK_SURFACE_LUM : LIGHT_SURFACE_LUM;
	if (getContrastRatioFromLuminance(lum, surfaceLum) < MIN_AA_CONTRAST) {
		return false;
	}
	const textLum = getContrastTextFromRgb(rgb) === '#111111' ? DARK_TEXT_LUM : LIGHT_TEXT_LUM;
	return getContrastRatioFromLuminance(lum, textLum) >= MIN_AA_CONTRAST;
}

/**
 * Calibrates an accent color for `'dark'` (`#1a1b20`) or `'light'` (`#edf1f6`)
 * surfaces so both the accent against the surface and its paired foreground text
 * achieve >= 4.6:1 WCAG contrast while preserving hue and saturation.
 */
export function calibrateAccentForTheme(
	color: string,
	theme: 'dark' | 'light'
): { accent: string; text: '#111111' | '#ffffff' } {
	const parsed = parseCssRgb(color);
	if (!parsed) {
		return theme === 'dark'
			? { accent: '#c8a96e', text: '#111111' }
			: { accent: '#785612', text: '#ffffff' };
	}

	if (isAccentCompliant(parsed, theme)) {
		return {
			accent: rgbToHex(parsed),
			text: getContrastTextFromRgb(parsed)
		};
	}

	const [h, s, l] = rgbToHsl(parsed);
	let calibratedRgb: RgbTuple;

	if (theme === 'dark') {
		let lo = l;
		let hi = 1;
		for (let i = 0; i < 24; i++) {
			const mid = (lo + hi) / 2;
			if (isAccentCompliant(hslToRgb(h, s, mid), 'dark')) {
				hi = mid;
			} else {
				lo = mid;
			}
		}
		calibratedRgb = hslToRgb(h, s, hi);
	} else {
		let lo = 0;
		let hi = l;
		for (let i = 0; i < 24; i++) {
			const mid = (lo + hi) / 2;
			if (isAccentCompliant(hslToRgb(h, s, mid), 'light')) {
				lo = mid;
			} else {
				hi = mid;
			}
		}
		calibratedRgb = hslToRgb(h, s, lo);
	}

	return {
		accent: rgbToHex(calibratedRgb),
		text: getContrastTextFromRgb(calibratedRgb)
	};
}
