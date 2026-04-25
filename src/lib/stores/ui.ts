import { writable } from 'svelte/store';

export const settingsOpen = writable(false);

export const searchQuery = writable('');
export const searchDraft = writable('');

export type AchievementJumpIntent = {
	gameId: number;
	achievementKey: string;
	token: number;
};

export const achievementJumpIntent = writable<AchievementJumpIntent | null>(null);

