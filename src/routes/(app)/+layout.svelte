<script lang="ts">
	// src/routes/(app)/+layout.svelte
	import './layout.css';
	import Settings from '$lib/Settings.svelte';
	import { loadSettings, settings } from '$lib/stores/settings.js';
	import { settingsOpen, watcherActive } from '$lib/stores/ui.js';
	import {
		getGameHeaderUrl,
		loadGames,
		setupAchievementsRealtimeSync,
		syncSteamMetadata
	} from '$lib/stores/Games.js';
	import { refreshSteamUser } from '$lib/stores/user.js';
	import { onMount } from 'svelte';
	import { get } from 'svelte/store';
	import { listen } from '@tauri-apps/api/event';
	import { invoke } from '@tauri-apps/api/core';
	import { goto } from '$app/navigation';
	import { resolve } from '$app/paths';
	import { page } from '$app/state';
	import Sidebar from '$lib/Sidebar.svelte';
	import Topbar from '$lib/Topbar.svelte';
	import { selectedGame, extractGameThemeColor } from '$lib/stores/selectedGame.js';
	import { i18n } from '$lib/stores/i18n.js';
	import { getCurrentWindow } from '@tauri-apps/api/window';
	import { checkForUpdates } from '$lib/stores/update.js';
	import UpdateNotice from '$lib/UpdateNotice.svelte';

	let { children } = $props();
	let isSetupRoute = $derived(page.url.pathname.startsWith('/setup'));
	let activeThemeColor = $state<string | null>(null);
	let activeThemeContrast = $state<string>('#111111');
	let selectedBannerUrl = $derived(
		$selectedGame
			? $selectedGame.header_image_url ||
				($selectedGame.steam_id
					? getGameHeaderUrl($selectedGame)
					: $selectedGame.background_image_url ||
						$selectedGame.steamgrid_icon_url ||
						getGameHeaderUrl($selectedGame))
			: ''
	);

	function parseColorRgb(color: string): [number, number, number] | null {
		const hexMatch = color.trim().match(/^#([0-9a-f]{3}|[0-9a-f]{6})$/i);
		if (hexMatch) {
			const hex = hexMatch[1];
			if (hex.length === 3) {
				return [
					parseInt(hex[0] + hex[0], 16),
					parseInt(hex[1] + hex[1], 16),
					parseInt(hex[2] + hex[2], 16)
				];
			}
			return [
				parseInt(hex.slice(0, 2), 16),
				parseInt(hex.slice(2, 4), 16),
				parseInt(hex.slice(4, 6), 16)
			];
		}
		const rgb = color.match(/\d+/g);
		if (rgb && rgb.length >= 3) {
			return [Number(rgb[0]), Number(rgb[1]), Number(rgb[2])];
		}
		return null;
	}

	$effect(() => {
		let cancelled = false;
		const game = $selectedGame;
		const s = $settings;

		if (!s.dynamicTheme || !game) {
			activeThemeColor = null;
			activeThemeContrast = '#111111';
			return;
		}

		const imageUrl =
			game.header_image_url ||
			(game.steam_id
				? getGameHeaderUrl(game)
				: game.background_image_url || game.steamgrid_icon_url || getGameHeaderUrl(game));
		if (imageUrl) {
			extractGameThemeColor(imageUrl).then((color) => {
				if (cancelled) return;
				activeThemeColor = color;
				if (color) {
					const parsed = parseColorRgb(color);
					if (parsed) {
						const [r, g, b] = parsed;
						const luminance = (0.299 * r + 0.587 * g + 0.114 * b) / 255;
						activeThemeContrast = luminance > 0.6 ? '#111111' : '#ffffff';
					}
				}
			});
		} else {
			activeThemeColor = null;
			activeThemeContrast = '#111111';
		}

		return () => {
			cancelled = true;
		};
	});

	function hasEmptyApiKeys(): boolean {
		const s = get(settings);
		return !s.steamApiKey.trim() && !s.steamGridDbApiKey.trim();
	}

	onMount(() => {
		setupAchievementsRealtimeSync();

		const unlisten = listen<boolean>('watcher-status', (event) => {
			watcherActive.set(event.payload);
		});

		let unlistenClose: (() => void) | undefined;

		const appWindow = getCurrentWindow();
		appWindow
			.onCloseRequested(async (event) => {
				const s = get(settings);
				event.preventDefault();
				if (s.minimizeToTray) {
					await invoke('hide_app');
				} else {
					await invoke('exit_app');
				}
			})
			.then((unlisten) => {
				unlistenClose = unlisten;
			});

		void (async () => {
			await loadSettings();
			const s = get(settings);

			try {
				await invoke('update_screenshot_shortcut', {
					shortcutStr: s.screenshotShortcut || 'F12'
				});
			} catch (err) {
				console.error('Failed to register screenshot shortcut:', err);
			}

			try {
				await invoke('update_hud_shortcut', {
					shortcutStr: s.hudShortcut || 'Alt+A'
				});
			} catch (err) {
				console.error('Failed to register HUD shortcut:', err);
			}

			// Hide the window only when launched via autostart (--minimized flag from registry).
			// startMinimized is the user preference that controls whether autostart hides the window;
			// it must NOT hide the window on a normal launch.
			const launchedMinimized = await invoke<boolean>('is_launched_minimized');
			if (launchedMinimized && s.startMinimized) {
				await invoke('hide_app');
			}

			const needsSetup = !s.setupCompleted || hasEmptyApiKeys();

			if (needsSetup && !isSetupRoute) {
				await goto(resolve('/setup/'));
				return;
			}

			if (!needsSetup && isSetupRoute) {
				await goto(resolve('/'));
				return;
			}

			await loadGames();
			void refreshSteamUser();
			console.log('[DEBUG][layout] Starting sync with keys:', {
				steamKeyLen: s.steamApiKey?.length,
				sgdbKeyLen: s.steamGridDbApiKey?.length
			});
			void syncSteamMetadata(s.steamApiKey, s.steamGridDbApiKey);

			void checkForUpdates(true);
		})();

		return () => {
			unlisten.then((u) => u());
			if (unlistenClose) unlistenClose();
		};
	});

	function closeSettings(): void {
		settingsOpen.set(false);
	}
</script>

<svelte:head>
	<link rel="icon" href="/favicon.svg" />
</svelte:head>

<div
	class="app-container"
	class:setup={isSetupRoute}
	style:--accent={$settings.dynamicTheme && activeThemeColor
		? activeThemeColor
		: 'var(--accent-default)'}
	style:--accent-text={$settings.dynamicTheme && activeThemeColor ? activeThemeContrast : undefined}
>
	{#if $settings.dynamicTheme && $selectedGame && !isSetupRoute}
		<div
			class="dynamic-bg"
			style:background-image="url({$selectedGame.background_image_url || selectedBannerUrl})"
		></div>
	{/if}
	{#if !isSetupRoute}
		<div class="sidebar-slot">
			<Sidebar />
		</div>
		<div class="topbar-slot">
			<Topbar />
		</div>
	{/if}
	<div class="content-slot">
		{@render children()}
	</div>
</div>

{#if $settingsOpen}
	<div class="settings-overlay">
		<button
			type="button"
			class="settings-backdrop"
			aria-label={$i18n.t('button.close')}
			onclick={closeSettings}
		></button>
		<div class="settings-modal" role="dialog" aria-modal="true" aria-labelledby="settings-title">
			<Settings />
			</div>
		</div>
	{/if}

	<UpdateNotice />

<style>
	.app-container {
		display: grid;
		grid-template-columns: 76px 1fr;
		grid-template-rows: 64px 1fr;
		height: 100vh;
		gap: 10px;
		padding: 10px;
		overflow: hidden;
		background: var(--bg-app);
		position: relative;
	}

	@media (max-width: 980px) {
		.app-container:not(.setup) {
			grid-template-columns: 66px 1fr;
		}
	}

	.dynamic-bg {
		position: absolute;
		inset: 0;
		background-size: cover;
		background-position: center;
		filter: blur(80px) saturate(1.8) brightness(0.4);
		opacity: 0.45;
		z-index: 0;
		pointer-events: none;
		transition:
			background-image 0.8s cubic-bezier(0.4, 0, 0.2, 1),
			opacity 0.8s ease;
	}

	:global([data-theme='light']) .dynamic-bg {
		filter: blur(80px) saturate(1.4) brightness(1.2);
		opacity: 0.25;
	}

	.sidebar-slot,
	.topbar-slot {
		position: relative;
		z-index: 1;
	}

	.sidebar-slot {
		grid-column: 1 / 2;
		grid-row: 1 / 3;
		z-index: 20;
	}

	.topbar-slot {
		grid-column: 2 / 3;
		grid-row: 1 / 2;
		padding: 0;
		min-width: 0;
		z-index: 15;
	}

	.content-slot {
		grid-column: 2 / 3;
		grid-row: 2 / 3;
		padding: 0;
		min-width: 0;
		min-height: 0;
		overflow: hidden;
		display: flex;
		flex-direction: column;
		position: relative;
		z-index: 1;
	}

	.app-container.setup {
		grid-template-columns: 1fr;
		grid-template-rows: 1fr;
		padding: 10px;
	}

	.app-container.setup .content-slot {
		grid-column: 1 / 2;
		grid-row: 1 / 2;
		padding: 0;
	}

	.settings-overlay {
		position: fixed;
		inset: 0;
		background: rgba(0, 0, 0, 0.54);
		backdrop-filter: blur(12px);
		-webkit-backdrop-filter: blur(12px);
		display: flex;
		align-items: center;
		justify-content: center;
		padding: 24px;
		z-index: 1000;
		animation: settings-fade-in 0.22s var(--spring-out) both;
	}

	.settings-backdrop {
		position: absolute;
		inset: 0;
		border: 0;
		padding: 0;
		margin: 0;
		background: transparent;
		cursor: default;
	}

	.settings-modal {
		position: relative;
		width: min(1100px, 100%);
		height: min(88vh, 860px);
		background: var(--glass-bg, var(--bg-panel));
		backdrop-filter: blur(20px) saturate(180%);
		-webkit-backdrop-filter: blur(20px) saturate(180%);
		border: 1px solid var(--border-specular);
		border-radius: 20px;
		overflow: hidden;
		box-shadow: var(--shadow-elevated);
		z-index: 1;
		animation: settings-spring-in 0.34s var(--spring-out) both;
	}

	@keyframes settings-fade-in {
		from {
			opacity: 0;
		}
		to {
			opacity: 1;
		}
	}

	@keyframes settings-spring-in {
		from {
			opacity: 0;
			transform: scale(0.96) translateY(8px);
		}
		to {
			opacity: 1;
			transform: scale(1) translateY(0);
		}
	}
</style>
