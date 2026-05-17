import { writable, derived } from 'svelte/store';
import { invoke } from '@tauri-apps/api/core';
import { games } from './Games';
import type { Game } from './Games';

export const selectedGameId = writable<string | null>(null);

export const selectedGame = derived(
	[games, selectedGameId],
	([$games, $id]) => $games.find((g: Game) => g.id === $id) ?? null
);

export async function updateGameTheme(gameId: string, themeColor: string) {
	try {
		await invoke('update_game_theme', { gameId, themeColor });
	} catch (err) {
		console.error('Failed to update game theme:', err);
	}
}

export async function toggleGameFavorite(gameId: string) {
	try {
		const isFavorite = await invoke<boolean>('toggle_game_favorite', { gameId });
		games.update(($games) =>
			$games.map((g) => (g.id === gameId ? { ...g, is_favorite: isFavorite } : g))
		);
	} catch (err) {
		console.error('Failed to toggle favorite:', err);
	}
}

export async function extractGameThemeColor(imageUrl: string): Promise<string | null> {
	try {
		const color = await invoke<string>('extract_game_theme_color', { imageUrl });
		return color;
	} catch (err) {
		console.error('Failed to extract theme color:', err);
		return null;
	}
}

export async function addGameTag(gameId: string, tag: string) {
	try {
		await invoke('add_game_tag', { gameId, tag });
		games.update(($games) =>
			$games.map((g) => {
				if (g.id === gameId) {
					const tags = [...g.tags];
					if (!tags.includes(tag)) tags.push(tag);
					return { ...g, tags };
				}
				return g;
			})
		);
	} catch (err) {
		console.error('Failed to add tag:', err);
	}
}

export async function removeGameTag(gameId: string, tag: string) {
	try {
		await invoke('remove_game_tag', { gameId, tag });
		games.update(($games) =>
			$games.map((g) => {
				if (g.id === gameId) {
					return { ...g, tags: g.tags.filter((t) => t !== tag) };
				}
				return g;
			})
		);
	} catch (err) {
		console.error('Failed to remove tag:', err);
	}
}
