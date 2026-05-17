<script lang="ts">
	// src/lib/Sidebar.svelte
	import {
		games,
		loadGames,
		totalUnlockedAchievements,
		totalCompletedGames,
		type Game
	} from '$lib/stores/Games.js';
	import { selectedGameId } from '$lib/stores/selectedGame.js';
	import { settingsOpen, watcherActive } from '$lib/stores/ui.js';
	import { steamUser } from '$lib/stores/user.js';
	import { i18n } from '$lib/stores/i18n.js';
	import { onMount } from 'svelte';
	import { goto } from '$app/navigation';
	import { resolve } from '$app/paths';
	import { page } from '$app/state';

	onMount(() => loadGames());

	function selectGame(id: string): void {
		selectedGameId.set(id);
		settingsOpen.set(false);
		void goto(resolve('/(app)/games/[id]', { id }));
	}

	function goHome(): void {
		selectedGameId.set(null);
		settingsOpen.set(false);
		void goto(resolve('/'));
	}

	function goStats(): void {
		selectedGameId.set(null);
		settingsOpen.set(false);
		void goto(resolve('/stats/'));
	}

	function goObjectives(): void {
		selectedGameId.set(null);
		settingsOpen.set(false);
		void goto(resolve('/objectives/'));
	}

	function goJournal(): void {
		selectedGameId.set(null);
		settingsOpen.set(false);
		void goto(resolve('/journal/'));
	}

	function goSettings(): void {
		settingsOpen.set(true);
	}

	function initials(name: string): string {
		return (name || '?')
			.split(' ')
			.slice(0, 2)
			.map((w: string) => w[0])
			.join('')
			.toUpperCase();
	}

	function progress(game: Game): string {
		const total = game.achievements_total || game.achievements?.length || 0;
		if (total === 0) return '';
		const unlocked = game.achievements?.filter((a) => a.unlocked).length ?? 0;
		return `${unlocked}/${total}`;
	}

	function iconUrl(game: Game): string {
		if (game.steamgrid_icon_url && game.steamgrid_icon_url.startsWith('http')) {
			return game.steamgrid_icon_url;
		}

		if (game.game_icon && game.game_icon.startsWith('http')) {
			return game.game_icon;
		}

		if (
			game.game_icon &&
			!game.game_icon.includes('/') &&
			!game.game_icon.includes('\\') &&
			game.steam_id
		) {
			return `https://media.steampowered.com/steamcommunity/public/images/apps/${game.steam_id}/${game.game_icon}.ico`;
		}

		if (game.header_image_url) return game.header_image_url;

		if (game.steam_id) {
			return `https://cdn.cloudflare.steamstatic.com/steam/apps/${game.steam_id}/header.jpg`;
		}

		return '';
	}

	function onIconError(name: string, id: string, steamId: number | null) {
		return (e: Event) => {
			const t = e.target as HTMLImageElement;
			const currentSrc = t.src;

			if (steamId) {
				if (currentSrc.includes('header.jpg')) {
					t.onerror = null;
					t.src = `https://ui-avatars.com/api/?name=${encodeURIComponent(name || String(id))}&background=1e1e1e&color=c8a96e&size=52&bold=true&length=2`;
				} else {
					t.src = `https://cdn.cloudflare.steamstatic.com/steam/apps/${steamId}/header.jpg`;
				}
			} else {
				t.onerror = null;
				t.src = `https://ui-avatars.com/api/?name=${encodeURIComponent(name || String(id))}&background=1e1e1e&color=c8a96e&size=52&bold=true&length=2`;
			}
		};
	}

	function goFavorites(): void {
		selectedGameId.set(null);
		settingsOpen.set(false);
		void goto(resolve('/favorites/'));
	}

	function goCollections(): void {
		selectedGameId.set(null);
		settingsOpen.set(false);
		void goto(resolve('/collections/'));
	}

	function goCompare(): void {
		selectedGameId.set(null);
		settingsOpen.set(false);
		void goto(resolve('/compare/'));
	}

	let pathname = $derived(String(page.url.pathname));
	let isHomeActive = $derived(pathname === '/');
	let isStatsActive = $derived(pathname.startsWith('/stats'));
	let isObjectivesActive = $derived(pathname.startsWith('/objectives'));
	let isJournalActive = $derived(pathname.startsWith('/journal'));
	let isFavoritesActive = $derived(pathname.startsWith('/favorites'));
	let isCollectionsActive = $derived(pathname.startsWith('/collections'));
	let isCompareActive = $derived(pathname.startsWith('/compare'));

	let isSettingsActive = $derived($settingsOpen);
	let sortedGames = $derived.by(() => {
		return [...$games].sort((a, b) =>
			(a.name || a.id).localeCompare(b.name || b.id, $i18n.locale, {
				sensitivity: 'base',
				numeric: true
			})
		);
	});
</script>

<aside class="sidebar">
	<div class="nav-wrap">
		<div class="pill" class:visible={isHomeActive}></div>
		<button
			class="game-slot home-slot"
			class:active={isHomeActive}
			onclick={goHome}
			title={$i18n.t('sidebar.home')}
		>
			<svg
				width="20"
				height="20"
				viewBox="0 0 24 24"
				fill="none"
				stroke="currentColor"
				stroke-width="2.2"
			>
				<path d="M3 9l9-7 9 7v11a2 2 0 0 1-2 2H5a2 2 0 0 1-2-2z" />
				<polyline points="9 22 9 12 15 12 15 22" />
			</svg>
		</button>
	</div>

	<div class="divider"></div>

	<div class="nav-wrap">
		<div class="pill" class:visible={isFavoritesActive}></div>
		<button
			class="game-slot nav-btn"
			class:active={isFavoritesActive}
			onclick={goFavorites}
			title={$i18n.t('sidebar.favorites') || 'Favoris'}
		>
			<svg
				width="20"
				height="20"
				viewBox="0 0 24 24"
				fill="none"
				stroke="currentColor"
				stroke-width="2.2"
			>
				<path
					d="M20.84 4.61a5.5 5.5 0 0 0-7.78 0L12 5.67l-1.06-1.06a5.5 5.5 0 0 0-7.78 7.78l1.06 1.06L12 21.23l8.78-8.78 1.06-1.06a5.5 5.5 0 0 0 0-7.78z"
				/>
			</svg>
		</button>
	</div>

	<div class="nav-wrap">
		<div class="pill" class:visible={isCollectionsActive}></div>
		<button
			class="game-slot nav-btn"
			class:active={isCollectionsActive}
			onclick={goCollections}
			title={$i18n.t('sidebar.collections') || 'Collections'}
		>
			<svg
				width="20"
				height="20"
				viewBox="0 0 24 24"
				fill="none"
				stroke="currentColor"
				stroke-width="2.2"
			>
				<path d="M22 19a2 2 0 0 1-2 2H4a2 2 0 0 1-2-2V5a2 2 0 0 1 2-2h5l2 3h9a2 2 0 0 1 2 2z" />
			</svg>
		</button>
	</div>

	<div class="divider"></div>

	<div class="nav-wrap">
		<div class="pill" class:visible={isStatsActive}></div>
		<button
			class="game-slot nav-btn"
			class:active={isStatsActive}
			onclick={goStats}
			title={$i18n.t('sidebar.stats')}
		>
			<svg
				width="20"
				height="20"
				viewBox="0 0 24 24"
				fill="none"
				stroke="currentColor"
				stroke-width="2.2"
			>
				<path d="M21.21 15.89A10 10 0 1 1 8 2.83" />
				<path d="M22 12A10 10 0 0 0 12 2v10z" />
			</svg>
		</button>
	</div>

	<div class="nav-wrap">
		<div class="pill" class:visible={isObjectivesActive}></div>
		<button
			class="game-slot nav-btn"
			class:active={isObjectivesActive}
			onclick={goObjectives}
			title={$i18n.t('sidebar.objectives')}
		>
			<svg
				width="20"
				height="20"
				viewBox="0 0 24 24"
				fill="none"
				stroke="currentColor"
				stroke-width="2.2"
			>
				<circle cx="12" cy="12" r="10" />
				<path d="M12 8v8M8 12h8" />
			</svg>
		</button>
	</div>

	<div class="nav-wrap">
		<div class="pill" class:visible={isJournalActive}></div>
		<button
			class="game-slot nav-btn"
			class:active={isJournalActive}
			onclick={goJournal}
			title={$i18n.t('sidebar.journal')}
		>
			<svg
				width="20"
				height="20"
				viewBox="0 0 24 24"
				fill="none"
				stroke="currentColor"
				stroke-width="2.2"
			>
				<path d="M4 19.5A2.5 2.5 0 0 1 6.5 17H20" />
				<path d="M6.5 2H20v20H6.5A2.5 2.5 0 0 1 4 19.5v-15A2.5 2.5 0 0 1 6.5 2z" />
			</svg>
		</button>
	</div>

	<div class="nav-wrap">
		<div class="pill" class:visible={isCompareActive}></div>
		<button
			class="game-slot nav-btn"
			class:active={isCompareActive}
			onclick={goCompare}
			title="Comparer"
		>
			<svg
				width="20"
				height="20"
				viewBox="0 0 24 24"
				fill="none"
				stroke="currentColor"
				stroke-width="2.2"
			>
				<circle cx="12" cy="12" r="10" />
				<path d="M12 2a14.5 14.5 0 0 0 0 20 14.5 14.5 0 0 0 0-20" />
				<path d="M2 12h20" />
			</svg>
		</button>
	</div>

	<div class="divider"></div>

	<div class="games-list">
		{#each sortedGames as game (game.id)}
			{@const isActive = $selectedGameId === game.id}
			<div class="nav-wrap">
				<div class="pill" class:visible={isActive}></div>
				<button
					class="game-slot"
					class:active={isActive}
					onclick={() => selectGame(game.id)}
					title="{game.name || game.id}{progress(game) ? ' · ' + progress(game) : ''}"
				>
					{#if iconUrl(game)}
						<img
							src={iconUrl(game)}
							alt={game.name}
							class="game-icon-img"
							onerror={onIconError(game.name, game.id, game.steam_id)}
						/>
						<span class="game-icon-fallback" style="display:none">
							{initials(game.name || game.id)}
						</span>
					{:else}
						<span class="game-icon-fallback">
							{initials(game.name || game.id)}
						</span>
					{/if}
				</button>
			</div>
		{/each}

		{#if sortedGames.length === 0}
			<div class="empty-hint">{$i18n.t('sidebar.emptyGames')}</div>
		{/if}
	</div>

	<div class="user-panel">
		{#if $steamUser}
			<img class="avatar" src={$steamUser.avatarfull} alt={$steamUser.personaname} />
			<div class="user-info">
				<div class="user-name">{$steamUser.personaname}</div>
				<div class="user-meta">
					{$i18n.t('sidebar.user.achievements', {
						count: $totalUnlockedAchievements,
						completed: $totalCompletedGames
					})}
				</div>
			</div>
		{:else}
			<div class="avatar">?</div>
			<div class="user-info">
				<div class="user-name">{$i18n.t('sidebar.user.disconnected')}</div>
				<div class="user-meta">{$i18n.t('sidebar.user.configureSteam')}</div>
			</div>
		{/if}
		<div class="user-actions">
			<div
				class="watcher-dot"
				class:active={$watcherActive}
				title={$watcherActive
					? $i18n.t('sidebar.watcher.active')
					: $i18n.t('sidebar.watcher.inactive')}
			></div>
			<button
				class="icon-btn"
				class:active={isSettingsActive}
				onclick={goSettings}
				title={$i18n.t('sidebar.advancedSettings')}
			>
				<svg
					width="15.75"
					height="15.75"
					viewBox="0 0 24 24"
					fill="none"
					stroke="currentColor"
					stroke-width="2"
				>
					<circle cx="12" cy="12" r="3" />
					<path
						d="M19.4 15a1.65 1.65 0 0 0 .33 1.82l.06.06a2 2 0 0 1-2.83 2.83l-.06-.06a1.65 1.65 0 0 0-1.82-.33 1.65 1.65 0 0 0-1 1.51V21a2 2 0 0 1-4 0v-.09A1.65 1.65 0 0 0 9 19.4a1.65 1.65 0 0 0-1.82.33l-.06.06a2 2 0 0 1-2.83-2.83l.06-.06A1.65 1.65 0 0 0 4.68 15a1.65 1.65 0 0 0-1.51-1H3a2 2 0 0 1 0-4h.09A1.65 1.65 0 0 0 4.6 9a1.65 1.65 0 0 0-.33-1.82l-.06-.06a2 2 0 0 1 2.83-2.83l.06.06A1.65 1.65 0 0 0 9 4.68a1.65 1.65 0 0 0 1-1.51V3a2 2 0 0 1 4 0v.09a1.65 1.65 0 0 0 1 1.51 1.65 1.65 0 0 0 1.82-.33l.06-.06a2 2 0 0 1 2.83 2.83l-.06.06A1.65 1.65 0 0 0 19.4 9a1.65 1.65 0 0 0 1.51 1H21a2 2 0 0 1 0 4h-.09a1.65 1.65 0 0 0-1.51 1z"
					/>
				</svg>
			</button>
		</div>
	</div>
</aside>

<style>
	.sidebar {
		grid-column: 1 / 2;
		grid-row: 1 / 3;
		height: 100%;
		background: var(--bg-sidebar);
		border-radius: 0;
		display: flex;
		flex-direction: column;
		align-items: center;
		padding: 12px 0 0;
		position: relative;
		overflow: visible;
		z-index: 1000;
	}

	.nav-wrap {
		position: relative;
		width: 100%;
		display: flex;
		align-items: center;
		justify-content: center;
		flex-shrink: 0;
	}

	.pill {
		position: absolute;
		left: 0;
		width: 3px;
		height: 0;
		background: var(--accent, #c8a96e);
		border-radius: 0 3px 3px 0;
		transition: height 0.18s cubic-bezier(0.4, 0, 0.2, 1);
		pointer-events: none;
	}
	.pill.visible {
		height: 22px;
	}

	.game-slot {
		width: 48px;
		height: 48px;
		border-radius: 12px;
		background: var(--surface-2);
		border: none;
		cursor: pointer;
		display: flex;
		align-items: center;
		justify-content: center;
		overflow: hidden;
		transition:
			border-radius 0.2s cubic-bezier(0.4, 0, 0.2, 1),
			background 0.15s,
			box-shadow 0.15s;
		flex-shrink: 0;
		margin: 3px 0;
		color: var(--text-muted);
	}

	.game-slot:hover {
		border-radius: 30%;
		background: var(--surface-hover);
		box-shadow: 0 0 0 2px color-mix(in srgb, var(--accent, #c8a96e) 30%, transparent);
	}

	.game-slot.active {
		border-radius: 30%;
		box-shadow: 0 0 0 2px var(--accent, #c8a96e);
	}

	.home-slot {
		background: var(--surface-1);
	}
	.home-slot.active,
	.home-slot:hover {
		color: var(--accent, #c8a96e);
	}

	.nav-btn {
		background: var(--surface-1);
	}
	.nav-btn.active,
	.nav-btn:hover {
		color: var(--accent, #c8a96e);
	}

	.game-icon-img {
		width: 100%;
		height: 100%;
		object-fit: cover;
	}

	.game-icon-fallback {
		font-size: 12px;
		font-weight: 700;
		color: var(--accent, #c8a96e);
		display: flex;
		align-items: center;
		justify-content: center;
		width: 100%;
		height: 100%;
	}

	.divider {
		width: 32px;
		height: 1px;
		background: var(--border-soft);
		margin: 6px 0;
		flex-shrink: 0;
	}

	.games-list {
		flex: 1;
		width: 100%;
		overflow-y: auto;
		overflow-x: visible;
		display: flex;
		flex-direction: column;
		align-items: center;
		scrollbar-width: none;
		padding-bottom: 72px;
	}
	.games-list::-webkit-scrollbar {
		display: none;
	}

	.empty-hint {
		font-size: 10px;
		color: var(--text-muted);
		text-align: center;
		margin-top: 12px;
		line-height: 1.5;
		white-space: pre-line;
	}

	.user-panel {
		position: absolute;
		bottom: 20px;
		left: 16px;
		width: max-content;
		min-width: 320px;
		max-width: 450px;
		background: var(--surface-1);
		border: 1px solid var(--border-soft);
		border-radius: 16px;
		padding: 16px 20px;
		display: flex;
		align-items: center;
		gap: 16px;
		z-index: 100;
		box-shadow: 0 20px 50px rgba(0, 0, 0, 0.7);
		backdrop-filter: blur(12px);
	}

	.avatar {
		width: 52px;
		height: 52px;
		border-radius: 50%;
		background: var(--accent, #c8a96e);
		flex-shrink: 0;
		display: flex;
		align-items: center;
		justify-content: center;
		font-size: 16px;
		font-weight: 700;
		color: var(--accent-text);
		object-fit: cover;
	}

	.user-info {
		flex: 1;
		min-width: 0;
	}

	.user-name {
		font-size: 16px;
		font-weight: 700;
		color: var(--text-primary);
		white-space: nowrap;
		overflow: hidden;
		text-overflow: ellipsis;
	}

	.user-meta {
		font-size: 11px;
		color: var(--text-muted);
		white-space: nowrap;
		overflow: hidden;
		text-overflow: ellipsis;
		margin-top: 2px;
	}

	.user-actions {
		display: flex;
		align-items: center;
		gap: 10px;
		flex-shrink: 0;
	}

	.watcher-dot {
		width: 10px;
		height: 10px;
		border-radius: 50%;
		background: #6a7080;
		transition: background 0.3s;
	}
	.watcher-dot.active {
		background: #3ddc84;
	}

	.icon-btn {
		width: 32px;
		height: 32px;
		border-radius: 8px;
		background: transparent;
		border: none;
		cursor: pointer;
		color: var(--text-muted);
		display: flex;
		align-items: center;
		justify-content: center;
		transition:
			background 0.12s,
			color 0.12s;
	}
	.icon-btn:hover {
		background: var(--surface-hover);
		color: var(--text-primary);
	}
	.icon-btn.active {
		color: var(--accent, #c8a96e);
	}
</style>
