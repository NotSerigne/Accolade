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
	is_favorite: boolean;
	tags: string[];
}

export interface OwnedGame {
	appid: number;
	name?: string;
	img_icon_url?: string;
}

type AchievementsUpdatedPayload = {
	game_id: string; // Mis à jour pour utiliser game_id au lieu de steam_id
	achievements: Achievement[];
	achievements_total?: number;
};

export const games = writable<Game[]>([]);
export const selectedGameId = writable<string | null>(null); // Changé en string | null
let achievementsSyncInitialized = false;

export const selectedGame = derived(
	[games, selectedGameId],
	([$games, $id]) => $games.find((g: Game) => g.id === $id) ?? null
);

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

export async function loadGames(): Promise<void> {
	try {
		const result = await invoke<Game[]>('get_all_games');
		if (result) {
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
	const effectiveApiKey = apiKey ?? s.steamApiKey;
	const effectiveSgdbApiKey = sgdbApiKey ?? s.steamGridDbApiKey;

	if (!effectiveApiKey) {
		console.warn('Cannot sync: No Steam API Key provided');
		return;
	}

	isSyncing.set(true);
	try {
		console.log('Syncing Steam metadata for ID:', s.steamId, 'Language:', s.language);

		const result = await invoke<Game[]>('sync_steam_metadata', {
			apiKey: effectiveApiKey,
			steamId: s.steamId,
			raUsername: s.raUsername,
			raApiKey: s.raApiKey,
			sgdbApiKey: effectiveSgdbApiKey,
			language: s.language
		});

		if (result) {
			games.set(result);
		}
	} catch (e) {
		console.error('Failed to sync Steam metadata:', e);
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
