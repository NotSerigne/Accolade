// src/lib/stores/Games.ts
import { writable, derived, get } from 'svelte/store';
import { invoke } from '@tauri-apps/api/core';
import { listen } from '@tauri-apps/api/event';
import { settings } from './settings';
import { isSyncing } from './ui';

export interface Achievement {
	key: string;
	name: string;
	unlocked: boolean;
	icon: string;
	icon_gray: string;
	unlocked_time: number | null;
	rarity: string;
	completionpercentage: string;
	desc: string;
	hidden?: boolean;
	/** Current value of a progress-based achievement (null when not progress-based). */
	progress?: number | null;
	/** Target value of a progress-based achievement (always > 0 when set). */
	max_progress?: number | null;
}

export type SourceType =
	| { type: 'Emulator'; value: string }
	| { type: 'RetroAchievements' }
	| { type: 'Custom' };

export interface Game {
	name: string;
	id: string;
	steam_id: number | null;
	game_icon: string;
	steamgrid_icon_url: string;
	header_image_url: string;
	background_image_url: string;
	achievements_total: number;
	achievements: Achievement[];
	path_buf: string | null;
	source: SourceType;
	genres: string[];
	is_favorite: boolean;
	tags: string[];
}

export interface OwnedGame {
	appid: number;
	name?: string;
	img_icon_url?: string;
}

type AchievementsUpdatedPayload = {
	game_id: string;
	achievements: Achievement[];
	achievements_total?: number;
};

/**
 * Computes the Gamerscore (XP) value of a single achievement based on its global completion rate.
 */
export function getAchievementPoints(ach: Achievement): number {
	const pct = parseFloat(ach.completionpercentage || '');
	if (!Number.isFinite(pct) || pct <= 0) return 15;
	if (pct <= 0.1) return 200;
	if (pct <= 1.0) return 100;
	if (pct <= 3.0) return 60;
	if (pct <= 7.0) return 40;
	if (pct <= 15.0) return 25;
	if (pct <= 35.0) return 15;
	return 10;
}

export interface ProfileRankInfo {
	score: number;
	level: number;
	currentLevelXp: number;
	nextLevelXp: number;
	progressPct: number;
}

export function calculateProfileRank(gamesList: Game[]): ProfileRankInfo {
	let score = 0;
	for (const g of gamesList) {
		const achs = g.achievements ?? [];
		const total = g.achievements_total || achs.length || 0;
		let unlockedCount = 0;
		for (const a of achs) {
			if (a.unlocked) {
				unlockedCount++;
				score += getAchievementPoints(a);
			}
		}
		if (total > 0 && unlockedCount === total) {
			score += 250; // Platinum / 100% completion bonus
		}
	}

	// Level curve: L = floor(sqrt(score / 100)) + 1
	// Level 1 starts at 0 XP, Level 2 at 100 XP, Level 3 at 400 XP, Level 4 at 900 XP, etc.
	const rawLevel = Math.floor(Math.sqrt(score / 100)) + 1;
	const levelFloorXp = (rawLevel - 1) * (rawLevel - 1) * 100;
	const levelCeilXp = rawLevel * rawLevel * 100;
	const span = Math.max(1, levelCeilXp - levelFloorXp);
	const currentLevelXp = score - levelFloorXp;
	const progressPct = Math.min(100, Math.max(0, Math.round((currentLevelXp / span) * 100)));

	return {
		score,
		level: rawLevel,
		currentLevelXp,
		nextLevelXp: span,
		progressPct
	};
}

export const games = writable<Game[]>([]);
let achievementsSyncInitialized = false;

export const totalUnlockedAchievements = derived(games, ($games) =>
	$games.reduce((acc, g) => acc + (g.achievements?.filter((a) => a.unlocked).length ?? 0), 0)
);

export const totalCompletedGames = derived(
	games,
	($games) =>
		$games.filter((g) => {
			const total = g.achievements_total || g.achievements?.length || 0;
			if (total === 0) return false;
			const unlocked = g.achievements?.filter((a) => a.unlocked).length ?? 0;
			return unlocked === total;
		}).length
);

export const profileRank = derived(games, ($games) => calculateProfileRank($games));

export async function loadGames(): Promise<void> {
	try {
		const result = await invoke<Game[]>('get_all_games');
		if (result) {
			result.sort((a, b) => (a.name || '').localeCompare(b.name || ''));
			games.set(result);
		} else {
			console.log('No games returned from backend');
		}
	} catch (e) {
		console.error('Failed to load games:', e);
	}
}

export async function syncSteamMetadata(apiKey?: string, sgdbApiKey?: string): Promise<void> {
	const s = get(settings);
	const effectiveApiKey = apiKey || s.steamApiKey;
	const effectiveSgdbApiKey = sgdbApiKey || s.steamGridDbApiKey;

	console.log('[DEBUG][syncSteamMetadata] Keys:', {
		effectiveApiKeyLen: effectiveApiKey?.length,
		effectiveSgdbApiKeyLen: effectiveSgdbApiKey?.length,
		raUsername: s.raUsername
	});

	isSyncing.set(true);
	try {
		const result = await invoke<Game[]>('sync_steam_metadata', {
			apiKey: effectiveApiKey || '',
			steamId: s.steamId || '',
			raUsername: s.raUsername || '',
			raApiKey: s.raApiKey || '',
			sgdbApiKey: effectiveSgdbApiKey || '',
			language: s.language
		});

		if (result) {
			result.sort((a, b) => (a.name || '').localeCompare(b.name || ''));
			games.set(result);
		}
	} catch (e) {
		console.error('Failed to sync metadata:', e);
	} finally {
		isSyncing.set(false);
	}
}

export async function loadAchievements(game: Game): Promise<Achievement[]> {
	try {
		return await invoke<Achievement[]>('get_achievements', { game });
	} catch (e) {
		console.error('Failed to load achievements:', e);
		return [];
	}
}

export function setupAchievementsRealtimeSync(): void {
	if (achievementsSyncInitialized) return;
	achievementsSyncInitialized = true;

	void listen<AchievementsUpdatedPayload>('achievements-updated', ({ payload }) => {
		if (!payload || !Array.isArray(payload.achievements)) return;

		games.update((currentGames) =>
			currentGames.map((game) => {
				if (game.id !== payload.game_id) return game;

				const total = payload.achievements_total ?? payload.achievements.length;
				return {
					...game,
					achievements: payload.achievements,
					achievements_total: total
				};
			})
		);
	});
}
