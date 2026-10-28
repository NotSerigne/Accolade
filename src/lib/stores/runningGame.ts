// src/lib/stores/runningGame.ts
import { readable } from 'svelte/store';
import { invoke } from '@tauri-apps/api/core';
import { listen } from '@tauri-apps/api/event';

/** Game currently running on the machine, as detected by the Rust `game_detector`. */
export interface RunningGame {
	/** Steam AppID of the running game. */
	app_id: number;
	/** Library id (e.g. `steam_1245620`) when the game is in the library. */
	game_id: string | null;
	name: string;
	exe_path: string;
	pid: number;
	/** Unix timestamp (seconds) at which the game process started. */
	started_at: number;
	/** Marker that identified the game (`goldberg`, `steam_emu.ini`, `steam`, ...). */
	detected_via: string;
}

/**
 * Live view of the running game. Starts listening on first subscription,
 * fetches the current state once, then follows `running-game-changed` events.
 */
export const runningGame = readable<RunningGame | null>(null, (set) => {
	let disposed = false;
	let unlisten: (() => void) | null = null;

	void listen<RunningGame | null>('running-game-changed', ({ payload }) => {
		set(payload ?? null);
	}).then((fn) => {
		if (disposed) fn();
		else unlisten = fn;
	});

	void invoke<RunningGame | null>('get_running_game')
		.then((game) => {
			if (!disposed) set(game ?? null);
		})
		.catch((e) => console.error('Failed to get running game:', e));

	return () => {
		disposed = true;
		unlisten?.();
	};
});
