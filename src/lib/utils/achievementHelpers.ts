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
