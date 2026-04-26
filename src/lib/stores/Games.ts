import { writable, derived } from 'svelte/store';
import { invoke } from '@tauri-apps/api/core';

export interface Achievement {
    key: string;
    name: string;
    unlocked: boolean;
    icon: string;
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
    game_icon_url: string;
    header_image_url: string;
    background_image_url: string;
    achievements_total: number;
    achievements: Achievement[];
    path_buf: string;
    emulator: string;
}

export const games = writable<Game[]>([]);
export const selectedGameId = writable<number | null>(null);

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
        games.set(result);
    } catch (e) {
        console.error('Failed to load games:', e);
    }
}

export async function syncSteamMetadata(apiKey: string): Promise<void> {
    try {
        const result = await invoke<Game[]>('sync_steam_metadata', { apiKey });
        games.set(result);
    } catch (e) {
        console.error('Failed to sync Steam metadata:', e);
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