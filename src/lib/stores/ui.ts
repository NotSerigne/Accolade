// src/lib/stores/ui.ts
import { writable } from 'svelte/store';

export const settingsOpen = writable(false);

export const searchQuery = writable('');
export const searchDraft = writable('');

export type AchievementJumpIntent = {
	gameId: string;
	achievementKey: string;
	token: number;
};

export const achievementJumpIntent = writable<AchievementJumpIntent | null>(null);

export const watcherActive = writable(true);
export const isSyncing = writable(false);
