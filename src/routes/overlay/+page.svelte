<script lang="ts">
	// src/routes/overlay/+page.svelte
	import { onMount } from 'svelte';
	import { listen } from '@tauri-apps/api/event';
	import { getCurrentWindow, currentMonitor } from '@tauri-apps/api/window';
	import { LogicalPosition } from '@tauri-apps/api/dpi';
	import { Store } from '@tauri-apps/plugin-store';
	import { invoke, convertFileSrc } from '@tauri-apps/api/core';
	import AchievementNotif from '$lib/AchievementNotif.svelte';
	import HudTracker from '$lib/HudTracker.svelte';

	type WindowPosition =
		| 'top-left'
		| 'top-center'
		| 'top-right'
		| 'bottom-left'
		| 'bottom-center'
		| 'bottom-right';

	interface Achievement {
		name?: string;
		desc?: string;
		icon?: string;
		rarity?: string;
		completionpercentage?: number;
		unlocked?: number;
		total?: number;
		is_platinum?: boolean;
		test?: boolean;
		game_id?: string;
		ach_key?: string;
		is_progress?: boolean;
		progress_current?: number;
		progress_max?: number;
	}

	interface HudAchievement {
		key: string;
		name: string;
		desc: string;
		icon: string;
		icon_gray: string;
		unlocked: boolean;
		unlocked_time: number | null;
		progress?: number | null;
		max_progress?: number | null;
	}

	interface HudData {
		game_id: string;
		game_name: string;
		achievements: HudAchievement[];
		pinned_keys: string[];
	}

	const NOTIF_WIDTH = 460;
	const NOTIF_HEIGHT = 260;
	const MARGIN = 24;
	const DEFAULT_POSITION: WindowPosition = 'top-right';
	const DEFAULT_SOUND = 'Steam.mp3';
	const DEFAULT_VOLUME = 0.7;

	let settingsStorePromise: Promise<Store> | null = null;

	async function getSettingsStore(): Promise<Store> {
		if (!settingsStorePromise) {
			settingsStorePromise = Store.load('settings.json');
		}
		return settingsStorePromise;
	}

	async function getOverlayPreferences(): Promise<{
		windowPosition: WindowPosition;
		notificationSound: string;
		notificationVolume: number;
	}> {
		try {
			const store = await getSettingsStore();
			const [windowPosition, notificationSound, notificationVolume] = await Promise.all([
				store.get<WindowPosition>('windowPosition'),
				store.get<string>('notificationSound'),
				store.get<number>('notificationVolume')
			]);
			return {
				windowPosition: windowPosition ?? DEFAULT_POSITION,
				notificationSound: notificationSound ?? DEFAULT_SOUND,
				notificationVolume: notificationVolume ?? DEFAULT_VOLUME
			};
		} catch {
			settingsStorePromise = null;
			return {
				windowPosition: DEFAULT_POSITION,
				notificationSound: DEFAULT_SOUND,
				notificationVolume: DEFAULT_VOLUME
			};
		}
	}

	function playNotificationSound(filename: string, volume: number): void {
		if (!filename || filename === 'none') return;
		const src = filename.startsWith('custom:')
			? convertFileSrc(filename.slice('custom:'.length))
			: '/sounds/' + encodeURIComponent(filename);
		const audio = new Audio(src);
		audio.volume = volume;
		audio.play().catch(() => {});
	}

	async function positionOverlay(pos: WindowPosition): Promise<WindowPosition> {
		const win = getCurrentWindow();
		const monitor = await currentMonitor();
		if (!monitor) return DEFAULT_POSITION;

		const sw = monitor.size.width / monitor.scaleFactor;
		const sh = monitor.size.height / monitor.scaleFactor;
		const ox = monitor.position.x / monitor.scaleFactor;
		const oy = monitor.position.y / monitor.scaleFactor;

		let x: number;
		let y: number;

		switch (pos) {
			case 'top-left':
				x = ox + MARGIN;
				y = oy + MARGIN;
				break;
			case 'top-center':
				x = ox + (sw - NOTIF_WIDTH) / 2;
				y = oy + MARGIN;
				break;
			case 'top-right':
				x = ox + sw - NOTIF_WIDTH - MARGIN;
				y = oy + MARGIN;
				break;
			case 'bottom-left':
				x = ox + MARGIN;
				y = oy + sh - NOTIF_HEIGHT - MARGIN;
				break;
			case 'bottom-center':
				x = ox + (sw - NOTIF_WIDTH) / 2;
				y = oy + sh - NOTIF_HEIGHT - MARGIN;
				break;
			case 'bottom-right':
				x = ox + sw - NOTIF_WIDTH - MARGIN;
				y = oy + sh - NOTIF_HEIGHT - MARGIN;
				break;
			default:
				x = ox + sw - NOTIF_WIDTH - MARGIN;
				y = oy + MARGIN;
		}

		await win.setPosition(new LogicalPosition(Math.round(x), Math.round(y)));
		// Re-assert HWND_TOPMOST after moving so fullscreen games don't cover the overlay
		await invoke('raise_overlay').catch(() => {});
		return pos;
	}

	let achievement = $state<Achievement | null>(null);
	let visible = $state(false);
	let position = $state<WindowPosition>('top-right');

	let hudVisible = $state(false);
	let hudGameName = $state('');
	let hudItems = $state<HudAchievement[]>([]);

	async function refreshHudData(): Promise<void> {
		try {
			const data = await invoke<HudData | null>('get_hud_data', { fallbackGameId: typeof localStorage !== 'undefined' ? localStorage.getItem('accolade:selected-game:v1') : null });
			if (!data) {
				hudGameName = '';
				hudItems = [];
				return;
			}
			hudGameName = data.game_name;
			const pinSet = new Set(data.pinned_keys.map((k) => k.trim().toLowerCase()));
			if (pinSet.size > 0) {
				hudItems = data.achievements
					.filter((a) => pinSet.has(a.key.trim().toLowerCase()))
					.slice(0, 4);
			} else {
				hudItems = data.achievements
					.filter((a) => !a.unlocked && !a.unlocked_time)
					.slice(0, 3);
			}
		} catch (e) {
			console.error('Failed to refresh HUD data:', e);
		}
	}

	async function toggleHud(): Promise<void> {
		if (!hudVisible) {
			await positionOverlay('top-left');
			await refreshHudData();
			hudVisible = true;
		} else {
			hudVisible = false;
		}
	}

	let queue: Achievement[] = [];
	let processingQueue = false;

	async function processQueue() {
		if (processingQueue || queue.length === 0) return;
		processingQueue = true;

		while (queue.length > 0) {
			const payload = queue.shift()!;

			visible = false;
			achievement = null;

			const { windowPosition, notificationSound, notificationVolume } =
				await getOverlayPreferences();
			const resolvedPosition = await positionOverlay(windowPosition);

			if (!payload.is_progress) {
				playNotificationSound(notificationSound, notificationVolume);
			}

			await new Promise((resolve) => requestAnimationFrame(resolve));

			position = resolvedPosition;
			achievement = payload;
			visible = true;

			// Wait for animation to finish before taking screenshot
			await new Promise((resolve) => setTimeout(resolve, 500));

			if (
				!achievement.test &&
				!achievement.is_progress &&
				achievement.game_id &&
				achievement.ach_key
			) {
				try {
					await invoke('capture_automatic_screenshot', {
						gameId: achievement.game_id,
						achKey: achievement.ach_key
					});
				} catch (e) {
					console.error('Failed to take automatic screenshot:', e);
				}
			}

			const holdMs = payload.is_progress ? 3000 : 5100;
			await new Promise((resolve) => setTimeout(resolve, holdMs));

			visible = false;
			await new Promise((resolve) => setTimeout(resolve, 600));
		}

		if (hudVisible) {
			await positionOverlay('top-left');
		}
		processingQueue = false;
	}

	onMount(() => {
		const cleanups: Array<() => void> = [];

		listen<Achievement>('achievement-notif', (event) => {
			queue.push(event.payload);
			processQueue();
		}).then((fn) => cleanups.push(fn));

		listen('toggle-hud', () => {
			void toggleHud();
		}).then((fn) => cleanups.push(fn));

		listen('achievements-updated', () => {
			if (hudVisible) void refreshHudData();
		}).then((fn) => cleanups.push(fn));

		listen('pinned-achievements-updated', () => {
			if (hudVisible) void refreshHudData();
		}).then((fn) => cleanups.push(fn));

		listen('running-game-changed', () => {
			if (hudVisible) void refreshHudData();
		}).then((fn) => cleanups.push(fn));

		return () => {
			for (const fn of cleanups) fn();
		};
	});
</script>

{#if visible && achievement}
	<AchievementNotif {achievement} {visible} {position} />
{:else if hudVisible}
	<HudTracker visible={hudVisible} gameName={hudGameName} achievements={hudItems} />
{/if}
