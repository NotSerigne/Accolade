// stores/games.js
import { writable, derived } from 'svelte/store';
import { invoke } from '@tauri-apps/api/core';

export const games = writable([]);
export const selectedGameId = writable(null);

export const selectedGame = derived(
    [games, selectedGameId],
    ([$games, $id]) => $games.find(g => g.steam_id === $id) ?? null
);

export async function loadGames() {
    try {
        const result = await invoke('get_all_games');
        games.set(result);
    } catch (e) {
        console.error('Failed to load games:', e);
    }
}

export async function loadAchievements(game) {
    try {
        const result = await invoke('get_achievements', { game });
        return result;
    } catch (e) {
        console.error('Failed to load achievements:', e);
        return [];
    }
}