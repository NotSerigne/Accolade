<script lang="ts">
	// src/routes/overlay/+page.svelte
	import { onMount } from 'svelte';
	import { listen } from '@tauri-apps/api/event';
	import { getCurrentWindow, currentMonitor } from '@tauri-apps/api/window';
	import { LogicalPosition } from '@tauri-apps/api/dpi';
	import { Store } from '@tauri-apps/plugin-store';
	import { invoke } from '@tauri-apps/api/core';
	import AchievementNotif from '$lib/AchievementNotif.svelte';

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
		const audio = new Audio(`/sounds/${encodeURIComponent(filename)}`);
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
		return pos;
	}

	let achievement = $state<Achievement | null>(null);
	let visible = $state(false);
	let position = $state<WindowPosition>('top-right');

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

			playNotificationSound(notificationSound, notificationVolume);

			await new Promise((resolve) => requestAnimationFrame(resolve));

			position = resolvedPosition;
			achievement = payload;
			visible = true;

			// Wait for animation to finish before taking screenshot
			await new Promise((resolve) => setTimeout(resolve, 500));

			if (!achievement.test && achievement.game_id && achievement.ach_key) {
				try {
					await invoke('capture_automatic_screenshot', {
						gameId: achievement.game_id,
						achKey: achievement.ach_key
					});
				} catch (e) {
					console.error('Failed to take automatic screenshot:', e);
				}
			}

			// Wait for notification to be visible (5s total, 500ms already passed) + fade out (0.6s)
			await new Promise((resolve) => setTimeout(resolve, 5100));

			visible = false;
			await new Promise((resolve) => setTimeout(resolve, 600));
		}

		processingQueue = false;
	}

	onMount(() => {
		let unlisten: (() => void) | undefined;

		listen<Achievement>('achievement-notif', (event) => {
			queue.push(event.payload);
			processQueue();
		}).then((cleanup) => {
			unlisten = cleanup;
		});

		return () => {
			unlisten?.();
		};
	});
</script>

<AchievementNotif {achievement} {visible} {position} />
