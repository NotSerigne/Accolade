// src/lib/stores/update.ts
import { writable } from 'svelte/store';
import { check, type Update } from '@tauri-apps/plugin-updater';
import { relaunch } from '@tauri-apps/plugin-process';

export type UpdateStatus =
	| 'idle'
	| 'checking'
	| 'up-to-date'
	| 'available'
	| 'downloading'
	| 'ready'
	| 'error';

export interface UpdateState {
	status: UpdateStatus;
	version: string | null;
	notes: string | null;
	progress: number | null;
	error: string | null;
	dismissed: boolean;
}

const initialState: UpdateState = {
	status: 'idle',
	version: null,
	notes: null,
	progress: null,
	error: null,
	dismissed: false
};

export const updateState = writable<UpdateState>(initialState);

let pendingUpdate: Update | null = null;
let checkInFlight: Promise<void> | null = null;

export async function checkForUpdates(silent = true): Promise<void> {
	if (checkInFlight) return checkInFlight;

	checkInFlight = (async () => {
		if (!silent) {
			updateState.update((s) => ({ ...s, status: 'checking', error: null }));
		}

		try {
			const update = await check();

			if (update) {
				pendingUpdate = update;
				updateState.set({
					status: 'available',
					version: update.version,
					notes: update.body ?? null,
					progress: null,
					error: null,
					dismissed: false
				});
			} else if (!silent) {
				updateState.update((s) => ({ ...s, status: 'up-to-date' }));
			} else {
				updateState.update((s) => ({ ...s, status: 'idle' }));
			}
		} catch (err) {
			console.error('[update] check failed:', err);
			if (silent) {
				// A background check failing (offline, endpoint unreachable, dev environment, etc.)
				// should never interrupt the user with an error toast - only surface errors
				// when the user explicitly requested the check.
				updateState.update((s) => ({ ...s, status: 'idle' }));
			} else {
				updateState.update((s) => ({
					...s,
					status: 'error',
					error: err instanceof Error ? err.message : String(err)
				}));
			}
		} finally {
			checkInFlight = null;
		}
	})();

	return checkInFlight;
}

export async function installUpdate(): Promise<void> {
	if (!pendingUpdate) return;

	updateState.update((s) => ({ ...s, status: 'downloading', progress: 0, error: null }));

	let totalBytes = 0;
	let downloadedBytes = 0;

	try {
		await pendingUpdate.downloadAndInstall((event) => {
			switch (event.event) {
				case 'Started':
					totalBytes = event.data.contentLength ?? 0;
					downloadedBytes = 0;
					break;
				case 'Progress':
					downloadedBytes += event.data.chunkLength;
					updateState.update((s) => ({
						...s,
						progress: totalBytes > 0 ? Math.min(100, (downloadedBytes / totalBytes) * 100) : null
					}));
					break;
				case 'Finished':
					updateState.update((s) => ({ ...s, progress: 100 }));
					break;
			}
		});

		updateState.update((s) => ({ ...s, status: 'ready' }));
		await relaunch();
	} catch (err) {
		console.error('[update] install failed:', err);
		updateState.update((s) => ({
			...s,
			status: 'error',
			error: err instanceof Error ? err.message : String(err)
		}));
	}
}

export function dismissUpdate(): void {
	updateState.update((s) => ({ ...s, dismissed: true }));
}
