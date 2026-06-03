// src/lib/stores/selectedGame.ts
import { writable, derived, get } from 'svelte/store';
import { invoke } from '@tauri-apps/api/core';
import { games } from './Games.js';
import type { Game } from './Games.js';
import { steamUser } from './user.js';

const SELECTED_GAME_KEY = 'accolade:selected-game:v1';

function getInitialSelectedGameId(): string | null {
	if (typeof localStorage === 'undefined') return null;
	return localStorage.getItem(SELECTED_GAME_KEY);
}

export const selectedGameId = writable<string | null>(getInitialSelectedGameId());

selectedGameId.subscribe((id) => {
	if (typeof localStorage !== 'undefined' && id) {
		localStorage.setItem(SELECTED_GAME_KEY, id);
	}
});

export const selectedGame = derived(
	[games, selectedGameId],
	([$games, $id]) => $games.find((g: Game) => g.id === $id) ?? null
);

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

/**
 * Extrait la couleur dominante d'une image et l'ajuste pour la visibilité.
 */
export async function extractGameThemeColor(imageUrl: string): Promise<string | null> {
	if (!imageUrl) return null;

	return new Promise((resolve) => {
		const img = new Image();
		img.crossOrigin = 'Anonymous';
		img.src = imageUrl;

		img.onload = () => {
			const canvas = document.createElement('canvas');
			const ctx = canvas.getContext('2d');
			if (!ctx) return resolve(null);

			// On réduit l'image pour l'analyse de performance
			canvas.width = 40;
			canvas.height = 40;
			ctx.drawImage(img, 0, 0, 40, 40);

			const data = ctx.getImageData(0, 0, 40, 40).data;
			let r = 0,
				g = 0,
				b = 0,
				count = 0;

			// On calcule la moyenne des couleurs (plus simple et performant)
			for (let i = 0; i < data.length; i += 4) {
				const alpha = data[i + 3];
				if (alpha < 150) continue; // On ignore les pixels trop transparents

				r += data[i];
				g += data[i + 1];
				b += data[i + 2];
				count++;
			}

			if (count === 0) return resolve(null);

			r = Math.floor(r / count);
			g = Math.floor(g / count);
			b = Math.floor(b / count);

			// Ajustement de la couleur pour garantir qu'elle "pop"
			const [h, s, l] = rgbToHsl(r, g, b);

			// On booste la saturation si elle est trop faible
			const finalS = Math.max(s, 0.5);
			// On ajuste la luminosité pour qu'elle soit bien visible sur fond sombre
			const finalL = Math.max(Math.min(l, 0.65), 0.45);

			const [fR, fG, fB] = hslToRgb(h, finalS, finalL);
			resolve(`rgb(${fR}, ${fG}, ${fB})`);
		};

		img.onerror = () => resolve(null);
	});
}

function rgbToHsl(r: number, g: number, b: number): [number, number, number] {
	r /= 255;
	g /= 255;
	b /= 255;
	const max = Math.max(r, g, b),
		min = Math.min(r, g, b);
	let h = 0,
		s = 0;
	const l = (max + min) / 2;

	if (max !== min) {
		const d = max - min;
		s = l > 0.5 ? d / (2 - max - min) : d / (max + min);
		switch (max) {
			case r:
				h = (g - b) / d + (g < b ? 6 : 0);
				break;
			case g:
				h = (b - r) / d + 2;
				break;
			case b:
				h = (r - g) / d + 4;
				break;
		}
		h /= 6;
	}
	return [h, s, l];
}

function hslToRgb(h: number, s: number, l: number): [number, number, number] {
	let r, g, b;
	if (s === 0) {
		r = g = b = l;
	} else {
		const q = l < 0.5 ? l * (1 + s) : l + s - l * s;
		const p = 2 * l - q;
		r = hueToRgb(p, q, h + 1 / 3);
		g = hueToRgb(p, q, h);
		b = hueToRgb(p, q, h - 1 / 3);
	}
	return [Math.round(r * 255), Math.round(g * 255), Math.round(b * 255)];
}

function hueToRgb(p: number, q: number, t: number) {
	if (t < 0) t += 1;
	if (t > 1) t -= 1;
	if (t < 1 / 6) return p + (q - p) * 6 * t;
	if (t < 1 / 2) return q;
	if (t < 2 / 3) return p + (q - p) * (2 / 3 - t) * 6;
	return p;
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

export function calculateAdvancedStats(gamesList: Game[]) {
	const allUnlocked = gamesList.flatMap((g) => g.achievements?.filter((a) => a.unlocked) || []);

	// Rarest achievement
	const rarest = allUnlocked
		.filter((a) => a.completionpercentage && !isNaN(parseFloat(a.completionpercentage)))
		.sort((a, b) => parseFloat(a.completionpercentage) - parseFloat(b.completionpercentage))[0];

	// Success by day/week
	const dailyMap: Record<string, number> = {};
	const weeklyMap: Record<string, number> = {};

	allUnlocked.forEach((a) => {
		if (!a.unlocked_time) return;
		const d = new Date(a.unlocked_time * 1000);
		const dayKey = d.toISOString().split('T')[0];
		dailyMap[dayKey] = (dailyMap[dayKey] || 0) + 1;

		// Week calculation aligned with Sunday (like the stats page)
		const startOfWeek = new Date(d.getTime());
		startOfWeek.setDate(d.getDate() - d.getDay());
		startOfWeek.setHours(0, 0, 0, 0);
		const weekKey = startOfWeek.toISOString().split('T')[0];
		weeklyMap[weekKey] = (weeklyMap[weekKey] || 0) + 1;
	});

	const maxDayCount = Math.max(0, ...Object.values(dailyMap));
	const maxWeekCount = Math.max(0, ...Object.values(weeklyMap));
	const totalRemaining =
		gamesList.reduce((acc, g) => acc + (g.achievements_total || 0), 0) - allUnlocked.length;

	return {
		rarestAchievement: rarest ? { name: rarest.name, pct: rarest.completionpercentage } : null,
		maxAchievementsInDay: maxDayCount,
		maxAchievementsInWeek: maxWeekCount,
		totalRemaining,
		totalUnlocked: allUnlocked.length
	};
}

export async function exportProfile() {
	try {
		const $games = get(games);
		const $user = get(steamUser);
		const advancedStats = calculateAdvancedStats($games);

		const exportData = {
			user: $user,
			games: $games,
			stats: advancedStats,
			exported_at: new Date().toISOString()
		};
		const data = JSON.stringify(exportData, null, 2);
		const timestamp = new Date().toISOString().replace(/[:.]/g, '-');
		const filename = `profile-export-${timestamp}.json`;
		const path = await invoke<string>('export_to_json', { data, filename });
		return path;
	} catch (err) {
		console.error('Export failed:', err);
		throw err;
	}
}

export async function exportGame(game: Game) {
	try {
		const data = JSON.stringify(game, null, 2);
		const safeName = game.name.replace(/[^a-z0-9]/gi, '_').toLowerCase();
		const filename = `game-export-${safeName}.json`;
		const path = await invoke<string>('export_to_json', { data, filename });
		return path;
	} catch (err) {
		console.error('Game export failed:', err);
		throw err;
	}
}

export async function openProfilesFolder() {
	try {
		await invoke('open_profiles_dir');
	} catch (err) {
		console.error('Failed to open profiles folder:', err);
	}
}

export async function exportSummaryPdf(title: string, content: string) {
	try {
		const timestamp = new Date().toISOString().replace(/[:.]/g, '-');
		const filename = `summary-${timestamp}.pdf`;
		const path = await invoke<string>('export_to_pdf', { title, content, filename });
		return path;
	} catch (err) {
		console.error('PDF export failed:', err);
		throw err;
	}
}
