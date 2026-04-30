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

export interface Game {
    name: string;
    steam_id: number;
    game_icon: string;
    steamgrid_icon_url: string;
    header_image_url: string;
    background_image_url: string;
    achievements_total: number;
    achievements: Achievement[];
    path_buf: string;
    emulator: string;
}

export interface OwnedGame {
    appid: number;
    name?: string;
    img_icon_url?: string;
}

type AchievementsUpdatedPayload = {
    steam_id: number;
    achievements: Achievement[];
    achievements_total?: number;
};

export const games = writable<Game[]>([]);
export const selectedGameId = writable<number | null>(null);
let achievementsSyncInitialized = false;

export const selectedGame = derived(
    [games, selectedGameId],
    ([$games, $id]) => $games.find((g: Game) => g.steam_id === $id) ?? null
);

export const totalUnlockedAchievements = derived(games, ($games) =>
    $games.reduce((acc, g) => acc + (g.achievements?.filter((a) => a.unlocked).length ?? 0), 0)
);

export async function loadGames(): Promise<void> {
    try {
        const result = await invoke<Game[]>('get_all_games');
        if (result && result.length > 0) {
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
        console.log('Syncing Steam metadata for ID:', s.steamId);

        const result = await invoke<Game[]>('sync_steam_metadata', {
            apiKey: effectiveApiKey,
            steamId: s.steamId,
            sgdbApiKey: effectiveSgdbApiKey
        });

        if (result && result.length > 0) {
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
                if (game.steam_id !== payload.steam_id) return game;

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
