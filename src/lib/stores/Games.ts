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
	playtime_seconds?: number;
}

export interface OwnedGame {
	appid: number;
	name?: string;
	img_icon_url?: string;
	playtime_forever?: number;
}

export interface HltbData {
	game_id: string;
	title: string;
	main_hours: number | null;
	main_extra_hours: number | null;
	completionist_hours: number | null;
	source: string;
	fetched_at: number;
}

type AchievementsUpdatedPayload = {
	game_id?: string;
	achievements?: Achievement[];
	achievements_total?: number;
};

type PlaytimeUpdatedPayload = {
	game_id: string;
	playtime_seconds: number;
};

export type RarityTier = 'legendary' | 'epic' | 'rare' | 'common';

export interface RarityPalette {
	tier: RarityTier;
	color: string;
	bg: string;
	border: string;
	glow: string;
}

const RARITY_PALETTES: Record<RarityTier, RarityPalette> = {
	legendary: {
		tier: 'legendary',
		color: '#f5b841',
		bg: 'rgba(245, 184, 65, 0.14)',
		border: 'rgba(245, 184, 65, 0.38)',
		glow: 'none'
	},
	epic: {
		tier: 'epic',
		color: '#b57cff',
		bg: 'rgba(181, 124, 255, 0.14)',
		border: 'rgba(181, 124, 255, 0.36)',
		glow: 'none'
	},
	rare: {
		tier: 'rare',
		color: '#4ea8de',
		bg: 'rgba(78, 168, 222, 0.14)',
		border: 'rgba(78, 168, 222, 0.34)',
		glow: 'none'
	},
	common: {
		tier: 'common',
		color: '#8b93a6',
		bg: 'rgba(139, 147, 166, 0.12)',
		border: 'rgba(139, 147, 166, 0.26)',
		glow: 'none'
	}
};

export function getGameHeaderUrl(game: Game): string {
	if (game.header_image_url?.trim()) {
		return game.header_image_url;
	}
	return `https://cdn.cloudflare.steamstatic.com/steam/apps/${game.steam_id || game.id}/header.jpg`;
}

export function getGameIconUrl(game: Game): string {
	if (game.steamgrid_icon_url?.trim()) {
		return game.steamgrid_icon_url;
	}
	if (game.game_icon?.trim()) {
		if (
			game.game_icon.startsWith('http') ||
			game.game_icon.startsWith('data:') ||
			game.game_icon.startsWith('asset:') ||
			game.game_icon.startsWith('/')
		) {
			return game.game_icon;
		}
		if (!game.game_icon.includes('/') && !game.game_icon.includes('\\') && game.steam_id) {
			return `https://media.steampowered.com/steamcommunity/public/images/apps/${game.steam_id}/${game.game_icon}.ico`;
		}
	}
	return getGameHeaderUrl(game);
}

export function getGameCompletionPct(game: Game): number {
	const achs = game.achievements ?? [];
	const total = game.achievements_total || achs.length || 0;
	if (total <= 0) return 0;
	let unlocked = 0;
	for (let i = 0; i < achs.length; i++) {
		if (achs[i].unlocked) unlocked++;
	}
	return Math.min(100, Math.max(0, Math.round((unlocked / total) * 100)));
}

export function getRarityTier(percent?: number | string): RarityTier {
	if (percent === undefined || percent === null || percent === '') return 'common';
	const val = typeof percent === 'number' ? percent : parseFloat(percent);
	if (!Number.isFinite(val)) return 'common';
	if (val < 5) return 'legendary';
	if (val < 15) return 'epic';
	if (val < 40) return 'rare';
	return 'common';
}

export function getRarityPalette(percent?: number | string): RarityPalette {
	return RARITY_PALETTES[getRarityTier(percent)];
}

export function formatPlaytime(seconds?: number): string {
	const s = Math.max(0, Math.floor(seconds || 0));
	if (s === 0) return '0m';
	if (s < 60) return '< 1m';
	const hours = Math.floor(s / 3600);
	const mins = Math.floor((s % 3600) / 60);
	if (hours === 0) return `${mins}m`;
	if (mins === 0) return `${hours}h`;
	return `${hours}h ${mins}m`;
}

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
let activeSyncPromise: Promise<void> | null = null;

export const totalUnlockedAchievements = derived(games, ($games) => {
	let count = 0;
	for (let i = 0; i < $games.length; i++) {
		const achs = $games[i].achievements;
		if (!achs) continue;
		for (let j = 0; j < achs.length; j++) {
			if (achs[j].unlocked) count++;
		}
	}
	return count;
});

export const totalCompletedGames = derived(games, ($games) => {
	let completed = 0;
	for (let i = 0; i < $games.length; i++) {
		const g = $games[i];
		const achs = g.achievements;
		const total = g.achievements_total || achs?.length || 0;
		if (total === 0 || !achs) continue;
		let unlocked = 0;
		for (let j = 0; j < achs.length; j++) {
			if (achs[j].unlocked) unlocked++;
		}
		if (unlocked === total) completed++;
	}
	return completed;
});

export const totalPlaytimeSeconds = derived(games, ($games) =>
	$games.reduce((acc, g) => acc + (g.playtime_seconds || 0), 0)
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
	if (activeSyncPromise) {
		console.log('[syncSteamMetadata] Sync already in flight — reusing active promise');
		return activeSyncPromise;
	}

	const run = async () => {
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
			activeSyncPromise = null;
		}
	};

	activeSyncPromise = run();
	return activeSyncPromise;
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
		if (!payload || !payload.game_id || !Array.isArray(payload.achievements)) {
			void loadGames();
			return;
		}

		games.update((currentGames) =>
			currentGames.map((game) => {
				if (game.id !== payload.game_id) return game;

				const total = payload.achievements_total ?? payload.achievements!.length;
				return {
					...game,
					achievements: payload.achievements!,
					achievements_total: total
				};
			})
		);
	});

	void listen<PlaytimeUpdatedPayload>('playtime-updated', ({ payload }) => {
		if (!payload || !payload.game_id) return;
		games.update((currentGames) =>
			currentGames.map((game) =>
				game.id === payload.game_id
					? { ...game, playtime_seconds: payload.playtime_seconds }
					: game
			)
		);
	});
}
