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
		rarity?: number | string | null;
		completionpercentage?: string | number | null;
		pinned?: boolean;
	}

	interface HudData {
		game_id: string;
		game_name: string;
		achievements: HudAchievement[];
		pinned_keys: string[];
	}

	const NOTIF_WIDTH = 480;
	const NOTIF_HEIGHT = 460;
	const MARGIN = 24;
	const DEFAULT_POSITION: WindowPosition = 'top-right';
	const DEFAULT_SOUND = 'Steam.mp3';
	const DEFAULT_VOLUME = 0.7;
	const DEFAULT_ACCENT = '#c8a96e';

	let settingsStorePromise: Promise<Store> | null = null;

	function resolveThemeValue(themeSetting: string | null | undefined): 'dark' | 'light' {
		if (themeSetting === 'light' || themeSetting === 'dark') return themeSetting;
		if (typeof window !== 'undefined' && window.matchMedia) {
			return window.matchMedia('(prefers-color-scheme: dark)').matches ? 'dark' : 'light';
		}
		return 'dark';
	}

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
		accentColor: string;
		theme: 'dark' | 'light';
	}> {
		try {
			const store = await getSettingsStore();
			const [windowPosition, notificationSound, notificationVolume, accentColor, rawTheme] =
				await Promise.all([
					store.get<WindowPosition>('windowPosition'),
					store.get<string>('notificationSound'),
					store.get<number>('notificationVolume'),
					store.get<string>('accentColor'),
					store.get<string>('theme')
				]);
			return {
				windowPosition: windowPosition ?? DEFAULT_POSITION,
				notificationSound: notificationSound ?? DEFAULT_SOUND,
				notificationVolume: notificationVolume ?? DEFAULT_VOLUME,
				accentColor: accentColor || DEFAULT_ACCENT,
				theme: resolveThemeValue(rawTheme)
			};
		} catch {
			settingsStorePromise = null;
			return {
				windowPosition: DEFAULT_POSITION,
				notificationSound: DEFAULT_SOUND,
				notificationVolume: DEFAULT_VOLUME,
				accentColor: DEFAULT_ACCENT,
				theme: resolveThemeValue('system')
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
		const inner = await win.innerSize().catch(() => null);
		const winW = inner && inner.width > 0 ? inner.width / monitor.scaleFactor : NOTIF_WIDTH;
		const winH = inner && inner.height > 0 ? inner.height / monitor.scaleFactor : NOTIF_HEIGHT;

		let x: number;
		let y: number;

		switch (pos) {
			case 'top-left':
				x = ox + MARGIN;
				y = oy + MARGIN;
				break;
			case 'top-center':
				x = ox + (sw - winW) / 2;
				y = oy + MARGIN;
				break;
			case 'top-right':
				x = ox + sw - winW - MARGIN;
				y = oy + MARGIN;
				break;
			case 'bottom-left':
				x = ox + MARGIN;
				y = oy + sh - winH - MARGIN;
				break;
			case 'bottom-center':
				x = ox + (sw - winW) / 2;
				y = oy + sh - winH - MARGIN;
				break;
			case 'bottom-right':
				x = ox + sw - winW - MARGIN;
				y = oy + sh - winH - MARGIN;
				break;
			default:
				x = ox + sw - winW - MARGIN;
				y = oy + MARGIN;
		}

		x = Math.max(ox, Math.min(x, ox + sw - winW));
		y = Math.max(oy, Math.min(y, oy + sh - winH));

		await win.setPosition(new LogicalPosition(Math.round(x), Math.round(y)));
		// Re-assert HWND_TOPMOST after moving so fullscreen games don't cover the overlay
		await invoke('raise_overlay').catch(() => {});
		return pos;
	}

	let achievement = $state<Achievement | null>(null);
	let visible = $state(false);
	let position = $state<WindowPosition>('top-right');
	let accentColor = $state(DEFAULT_ACCENT);
	let overlayTheme = $state<'dark' | 'light'>('dark');

	let hudVisible = $state(false);
	let hudGameName = $state('');
	let hudItems = $state<HudAchievement[]>([]);
	let hudTotalCount = $state(0);
	let hudUnlockedCount = $state(0);
	let hudPinnedKeys = $state<string[]>([]);

	async function refreshAppearancePreferences(): Promise<void> {
		const prefs = await getOverlayPreferences();
		accentColor = prefs.accentColor;
		overlayTheme = prefs.theme;
	}

	async function refreshHudData(): Promise<void> {
		try {
			const data = await invoke<HudData | null>('get_hud_data', {
				fallbackGameId:
					typeof localStorage !== 'undefined'
						? localStorage.getItem('accolade:selected-game:v1')
						: null
			});
			if (!data) {
				hudGameName = '';
				hudItems = [];
				hudTotalCount = 0;
				hudUnlockedCount = 0;
				hudPinnedKeys = [];
				return;
			}
			hudGameName = data.game_name;
			hudTotalCount = data.achievements.length;
			hudUnlockedCount = data.achievements.filter(
				(a) => a.unlocked || Boolean(a.unlocked_time)
			).length;
			hudPinnedKeys = data.pinned_keys ?? [];

			const pinSet = new Set(hudPinnedKeys.map((k) => k.trim().toLowerCase()));
			if (pinSet.size > 0) {
				hudItems = data.achievements
					.filter((a) => pinSet.has(a.key.trim().toLowerCase()))
					.slice(0, 4)
					.map((a) => ({ ...a, pinned: true }));
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
			await refreshAppearancePreferences();
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

			const prefs = await getOverlayPreferences();
			accentColor = prefs.accentColor;
			overlayTheme = prefs.theme;
			const resolvedPosition = await positionOverlay(prefs.windowPosition);

			if (!payload.is_progress) {
				playNotificationSound(prefs.notificationSound, prefs.notificationVolume);
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

		void refreshAppearancePreferences();

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

		listen('settings-updated', () => {
			void refreshAppearancePreferences();
		}).then((fn) => cleanups.push(fn));

		return () => {
			for (const fn of cleanups) fn();
		};
	});
</script>

<div
	class="overlay-viewport"
	class:align-bottom={Boolean(visible && achievement && position.startsWith('bottom-'))}
	class:align-right={Boolean(visible && achievement && position.endsWith('-right'))}
	class:align-center={Boolean(visible && achievement && position.endsWith('-center'))}
	data-theme={overlayTheme}
	style:--accent={accentColor}
>
	{#if visible && achievement}
		<AchievementNotif {achievement} {visible} {position} />
	{:else if hudVisible}
		<HudTracker
			visible={hudVisible}
			gameName={hudGameName}
			achievements={hudItems}
			totalCount={hudTotalCount}
			unlockedCount={hudUnlockedCount}
			pinnedKeys={hudPinnedKeys}
		/>
	{/if}
</div>

<style>
	.overlay-viewport {
		width: 100%;
		height: 100%;
		background: transparent;
		pointer-events: none;
		overflow: visible;
		display: flex;
		flex-direction: column;
		justify-content: flex-start;
		align-items: flex-start;
	}

	.overlay-viewport.align-bottom {
		justify-content: flex-end;
	}

	.overlay-viewport.align-right {
		align-items: flex-end;
	}

	.overlay-viewport.align-center {
		align-items: center;
	}
</style>
