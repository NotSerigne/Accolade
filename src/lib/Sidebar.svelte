<script lang="ts">
	// src/lib/Sidebar.svelte
	import {
		games,
		loadGames,
		getGameIconUrl,
		totalUnlockedAchievements,
		totalCompletedGames,
		profileRank,
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

	function goLibrary(): void {
		selectedGameId.set(null);
		settingsOpen.set(false);
		void goto(resolve('/(app)/games'));
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

	function goScreenshots(): void {
		selectedGameId.set(null);
		settingsOpen.set(false);
		void goto(resolve('/screenshots/'));
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
	let isLibraryActive = $derived(pathname === '/games' || pathname === '/games/');
	let isStatsActive = $derived(pathname.startsWith('/stats'));
	let isObjectivesActive = $derived(pathname.startsWith('/objectives'));
	let isJournalActive = $derived(pathname.startsWith('/journal'));
	let isScreenshotsActive = $derived(pathname.startsWith('/screenshots'));
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
			type="button"
			class="game-slot home-slot"
			class:active={isHomeActive}
			onclick={goHome}
			title={$i18n.t('sidebar.home')}
			aria-label={$i18n.t('sidebar.home')}
		>
			<svg
				width="20"
				height="20"
				viewBox="0 0 24 24"
				fill="none"
				stroke="currentColor"
				stroke-width="2.1"
				stroke-linecap="round"
				stroke-linejoin="round"
				aria-hidden="true"
			>
				<path d="M3 9l9-7 9 7v11a2 2 0 0 1-2 2H5a2 2 0 0 1-2-2z" />
				<polyline points="9 22 9 12 15 12 15 22" />
			</svg>
		</button>
	</div>

	<div class="nav-wrap">
		<div class="pill" class:visible={isLibraryActive}></div>
		<button
			type="button"
			class="game-slot nav-btn"
			class:active={isLibraryActive}
			onclick={goLibrary}
			title={$i18n.t('topbar.library')}
			aria-label={$i18n.t('topbar.library')}
		>
			<svg
				width="20"
				height="20"
				viewBox="0 0 24 24"
				fill="none"
				stroke="currentColor"
				stroke-width="2.1"
				stroke-linecap="round"
				stroke-linejoin="round"
				aria-hidden="true"
			>
				<rect x="3" y="3" width="7" height="7" rx="1.5" />
				<rect x="14" y="3" width="7" height="7" rx="1.5" />
				<rect x="3" y="14" width="7" height="7" rx="1.5" />
				<rect x="14" y="14" width="7" height="7" rx="1.5" />
			</svg>
		</button>
	</div>

	<div class="divider"></div>

	<div class="nav-wrap">
		<div class="pill" class:visible={isFavoritesActive}></div>
		<button
			type="button"
			class="game-slot nav-btn"
			class:active={isFavoritesActive}
			onclick={goFavorites}
			title={$i18n.t('sidebar.favorites')}
			aria-label={$i18n.t('sidebar.favorites')}
		>
			<svg
				width="20"
				height="20"
				viewBox="0 0 24 24"
				fill="none"
				stroke="currentColor"
				stroke-width="2.1"
				stroke-linecap="round"
				stroke-linejoin="round"
				aria-hidden="true"
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
			type="button"
			class="game-slot nav-btn"
			class:active={isCollectionsActive}
			onclick={goCollections}
			title={$i18n.t('sidebar.collections')}
			aria-label={$i18n.t('sidebar.collections')}
		>
			<svg
				width="20"
				height="20"
				viewBox="0 0 24 24"
				fill="none"
				stroke="currentColor"
				stroke-width="2.1"
				stroke-linecap="round"
				stroke-linejoin="round"
				aria-hidden="true"
			>
				<path d="M22 19a2 2 0 0 1-2 2H4a2 2 0 0 1-2-2V5a2 2 0 0 1 2-2h5l2 3h9a2 2 0 0 1 2 2z" />
			</svg>
		</button>
	</div>

	<div class="divider"></div>

	<div class="nav-wrap">
		<div class="pill" class:visible={isStatsActive}></div>
		<button
			type="button"
			class="game-slot nav-btn"
			class:active={isStatsActive}
			onclick={goStats}
			title={$i18n.t('sidebar.stats')}
			aria-label={$i18n.t('sidebar.stats')}
		>
			<svg
				width="20"
				height="20"
				viewBox="0 0 24 24"
				fill="none"
				stroke="currentColor"
				stroke-width="2.1"
				stroke-linecap="round"
				stroke-linejoin="round"
				aria-hidden="true"
			>
				<path d="M21.21 15.89A10 10 0 1 1 8 2.83" />
				<path d="M22 12A10 10 0 0 0 12 2v10z" />
			</svg>
		</button>
	</div>

	<div class="nav-wrap">
		<div class="pill" class:visible={isObjectivesActive}></div>
		<button
			type="button"
			class="game-slot nav-btn"
			class:active={isObjectivesActive}
			onclick={goObjectives}
			title={$i18n.t('sidebar.objectives')}
			aria-label={$i18n.t('sidebar.objectives')}
		>
			<svg
				width="20"
				height="20"
				viewBox="0 0 24 24"
				fill="none"
				stroke="currentColor"
				stroke-width="2.1"
				stroke-linecap="round"
				stroke-linejoin="round"
				aria-hidden="true"
			>
				<circle cx="12" cy="12" r="10" />
				<path d="M12 8v8M8 12h8" />
			</svg>
		</button>
	</div>

	<div class="nav-wrap">
		<div class="pill" class:visible={isJournalActive}></div>
		<button
			type="button"
			class="game-slot nav-btn"
			class:active={isJournalActive}
			onclick={goJournal}
			title={$i18n.t('sidebar.journal')}
			aria-label={$i18n.t('sidebar.journal')}
		>
			<svg
				width="20"
				height="20"
				viewBox="0 0 24 24"
				fill="none"
				stroke="currentColor"
				stroke-width="2.1"
				stroke-linecap="round"
				stroke-linejoin="round"
				aria-hidden="true"
			>
				<path d="M4 19.5A2.5 2.5 0 0 1 6.5 17H20" />
				<path d="M6.5 2H20v20H6.5A2.5 2.5 0 0 1 4 19.5v-15A2.5 2.5 0 0 1 6.5 2z" />
			</svg>
		</button>
	</div>

	<div class="nav-wrap">
		<div class="pill" class:visible={isScreenshotsActive}></div>
		<button
			type="button"
			class="game-slot nav-btn"
			class:active={isScreenshotsActive}
			onclick={goScreenshots}
			title={$i18n.t('sidebar.screenshots')}
			aria-label={$i18n.t('sidebar.screenshots')}
		>
			<svg
				width="20"
				height="20"
				viewBox="0 0 24 24"
				fill="none"
				stroke="currentColor"
				stroke-width="2.1"
				stroke-linecap="round"
				stroke-linejoin="round"
				aria-hidden="true"
			>
				<rect x="3" y="3" width="18" height="18" rx="2" ry="2" />
				<circle cx="8.5" cy="8.5" r="1.5" />
				<polyline points="21 15 16 10 5 21" />
			</svg>
		</button>
	</div>

	<div class="nav-wrap">
		<div class="pill" class:visible={isCompareActive}></div>
		<button
			type="button"
			class="game-slot nav-btn"
			class:active={isCompareActive}
			onclick={goCompare}
			title={$i18n.t('sidebar.compare')}
			aria-label={$i18n.t('sidebar.compare')}
		>
			<svg
				width="20"
				height="20"
				viewBox="0 0 24 24"
				fill="none"
				stroke="currentColor"
				stroke-width="2.1"
				stroke-linecap="round"
				stroke-linejoin="round"
				aria-hidden="true"
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
			{@const icon = getGameIconUrl(game)}
			<div class="nav-wrap">
				<div class="pill" class:visible={isActive}></div>
				<button
					type="button"
					class="game-slot"
					class:active={isActive}
					onclick={() => selectGame(game.id)}
					title="{game.name || game.id}{progress(game) ? ' · ' + progress(game) : ''}"
					aria-label={game.name || game.id}
				>
					{#if icon}
						<img
							src={icon}
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

	<div class="divider"></div>

	<div class="user-dock">
		<button
			type="button"
			class="user-trigger"
			onclick={goSettings}
			title={$steamUser
				? `${$steamUser.personaname} · LVL ${$profileRank.level}`
				: $i18n.t('sidebar.user.disconnected')}
			aria-label={$steamUser
				? `${$steamUser.personaname} · LVL ${$profileRank.level}`
				: $i18n.t('sidebar.user.disconnected')}
		>
			{#if $steamUser}
				<img class="dock-avatar" src={$steamUser.avatarfull} alt={$steamUser.personaname} />
			{:else}
				<span class="dock-avatar dock-avatar-fallback" aria-hidden="true">?</span>
			{/if}
			<span
				class="watcher-dot dock-watcher-dot"
				class:active={$watcherActive}
				aria-hidden="true"
			></span>
		</button>

		<span class="dock-lvl-badge tabular-nums" aria-hidden="true">LVL {$profileRank.level}</span>

		<button
			type="button"
			class="dock-settings-btn"
			class:active={isSettingsActive}
			onclick={goSettings}
			title={$i18n.t('sidebar.advancedSettings')}
			aria-label={$i18n.t('sidebar.advancedSettings')}
		>
			<svg
				width="16"
				height="16"
				viewBox="0 0 24 24"
				fill="none"
				stroke="currentColor"
				stroke-width="2"
				stroke-linecap="round"
				stroke-linejoin="round"
				aria-hidden="true"
			>
				<circle cx="12" cy="12" r="3" />
				<path
					d="M19.4 15a1.65 1.65 0 0 0 .33 1.82l.06.06a2 2 0 0 1-2.83 2.83l-.06-.06a1.65 1.65 0 0 0-1.82-.33 1.65 1.65 0 0 0-1 1.51V21a2 2 0 0 1-4 0v-.09A1.65 1.65 0 0 0 9 19.4a1.65 1.65 0 0 0-1.82.33l-.06.06a2 2 0 0 1-2.83-2.83l.06-.06A1.65 1.65 0 0 0 4.68 15a1.65 1.65 0 0 0-1.51-1H3a2 2 0 0 1 0-4h.09A1.65 1.65 0 0 0 4.6 9a1.65 1.65 0 0 0-.33-1.82l-.06-.06a2 2 0 0 1 2.83-2.83l.06.06A1.65 1.65 0 0 0 9 4.68a1.65 1.65 0 0 0 1-1.51V3a2 2 0 0 1 4 0v.09a1.65 1.65 0 0 0 1 1.51 1.65 1.65 0 0 0 1.82-.33l.06-.06a2 2 0 0 1 2.83 2.83l-.06.06A1.65 1.65 0 0 0 19.4 9a1.65 1.65 0 0 0 1.51 1H21a2 2 0 0 1 0 4h-.09a1.65 1.65 0 0 0-1.51 1z"
				/>
			</svg>
		</button>

		<div
			class="user-popover"
			role="region"
			aria-label={$steamUser ? $steamUser.personaname : $i18n.t('sidebar.user.disconnected')}
		>
			<div class="popover-header">
				{#if $steamUser}
					<img class="popover-avatar" src={$steamUser.avatarfull} alt={$steamUser.personaname} />
					<div class="user-info">
						<div class="user-name-row">
							<span class="user-name">{$steamUser.personaname}</span>
							<span
								class="user-lvl-badge tabular-nums"
								title="{$profileRank.currentLevelXp}/{$profileRank.nextLevelXp} XP"
							>
								LVL {$profileRank.level}
							</span>
						</div>
						<div class="user-meta tabular-nums">
							{$i18n.t('sidebar.user.achievements', {
								count: $totalUnlockedAchievements,
								completed: $totalCompletedGames
							})} · <span class="user-gs tabular-nums">{$profileRank.score.toLocaleString()} G</span>
						</div>
						<div class="user-xp-track">
							<div class="user-xp-fill" style="width: {$profileRank.progressPct}%"></div>
						</div>
						<div class="user-xp-caption tabular-nums">
							<span>{$profileRank.currentLevelXp.toLocaleString()} / {$profileRank.nextLevelXp.toLocaleString()} XP</span>
							<span>{$profileRank.progressPct}%</span>
						</div>
					</div>
				{:else}
					<div class="popover-avatar popover-avatar-fallback" aria-hidden="true">?</div>
					<div class="user-info">
						<div class="user-name">{$i18n.t('sidebar.user.disconnected')}</div>
						<div class="user-meta">{$i18n.t('sidebar.user.configureSteam')}</div>
					</div>
				{/if}
			</div>

			<div class="popover-footer">
				<span class="popover-watcher-status">
					<span class="watcher-dot" class:active={$watcherActive} aria-hidden="true"></span>
					<span>
						{$watcherActive
							? $i18n.t('sidebar.watcher.active')
							: $i18n.t('sidebar.watcher.inactive')}
					</span>
				</span>
			</div>
		</div>
	</div>
</aside>

<style>
	.sidebar {
		grid-column: 1 / 2;
		grid-row: 1 / 3;
		width: 100%;
		height: 100%;
		background: var(--bg-sidebar);
		backdrop-filter: blur(20px) saturate(180%);
		-webkit-backdrop-filter: blur(20px) saturate(180%);
		border: 1px solid var(--border-specular);
		border-radius: 16px;
		box-shadow: var(--shadow-card);
		display: flex;
		flex-direction: column;
		align-items: center;
		padding: 10px 0;
		position: relative;
		overflow: visible;
		z-index: 50;
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
		width: 3.5px;
		height: 0;
		opacity: 0;
		transform: scaleY(0.4);
		background: var(--accent, #c8a96e);
		border-radius: 0 4px 4px 0;
		transition:
			height 0.3s var(--spring-out),
			opacity 0.2s ease,
			transform 0.3s var(--spring-out);
		pointer-events: none;
	}

	.pill.visible {
		height: 22px;
		opacity: 1;
		transform: scaleY(1);
	}

	.game-slot {
		width: 46px;
		height: 46px;
		border-radius: 14px;
		background: var(--surface-2);
		border: 1px solid transparent;
		cursor: pointer;
		display: flex;
		align-items: center;
		justify-content: center;
		overflow: hidden;
		transition:
			border-radius 0.28s var(--spring-out),
			transform 0.24s var(--spring-out),
			background 0.18s ease,
			color 0.18s ease,
			border-color 0.18s ease,
			box-shadow 0.24s var(--spring-out);
		flex-shrink: 0;
		margin: 2.5px 0;
		color: var(--text-muted);
	}

	.game-slot:hover {
		border-radius: 16px;
		background: var(--surface-hover);
		box-shadow: 0 0 0 1.5px color-mix(in srgb, var(--accent, #c8a96e) 35%, transparent);
	}

	.game-slot.active {
		border-radius: 16px;
		box-shadow: 0 0 0 1.5px var(--accent, #c8a96e);
	}

	.home-slot,
	.nav-btn {
		background: var(--surface-1);
		border-color: var(--border-soft);
	}

	.home-slot.active,
	.home-slot:hover,
	.nav-btn.active,
	.nav-btn:hover {
		color: var(--accent, #c8a96e);
		background: color-mix(in srgb, var(--accent, #c8a96e) 12%, var(--surface-1));
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
		width: 30px;
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
		padding: 2px 0 4px;
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

	.user-dock {
		position: relative;
		width: 100%;
		display: flex;
		flex-direction: column;
		align-items: center;
		gap: 6px;
		padding: 2px 0 0;
		flex-shrink: 0;
	}

	.user-trigger {
		position: relative;
		width: 44px;
		height: 44px;
		padding: 0;
		border-radius: 14px;
		border: 1px solid var(--border-specular);
		background: var(--surface-1);
		cursor: pointer;
		display: flex;
		align-items: center;
		justify-content: center;
		transition:
			transform 0.24s var(--spring-out),
			border-color 0.18s ease,
			box-shadow 0.24s var(--spring-out);
	}

	.user-trigger:hover,
	.user-dock:focus-within .user-trigger {
		border-color: color-mix(in srgb, var(--accent, #c8a96e) 55%, var(--border-specular));
		box-shadow: 0 0 0 2px color-mix(in srgb, var(--accent, #c8a96e) 22%, transparent);
	}

	.dock-avatar {
		width: 100%;
		height: 100%;
		border-radius: 13px;
		object-fit: cover;
		display: block;
	}

	.dock-avatar-fallback {
		display: flex;
		align-items: center;
		justify-content: center;
		background: var(--accent, #c8a96e);
		color: var(--accent-text);
		font-size: 15px;
		font-weight: 700;
	}

	.dock-watcher-dot {
		position: absolute;
		top: -3px;
		right: -3px;
		border: 2px solid var(--bg-app);
	}

	.dock-lvl-badge {
		font-size: 9px;
		font-weight: 800;
		line-height: 1.2;
		padding: 2px 6px;
		border-radius: 999px;
		background: color-mix(in srgb, var(--accent, #c8a96e) 16%, var(--surface-1));
		border: 1px solid color-mix(in srgb, var(--accent, #c8a96e) 42%, transparent);
		color: var(--accent, #c8a96e);
		letter-spacing: 0.04em;
	}

	.dock-settings-btn {
		width: 34px;
		height: 30px;
		border-radius: 10px;
		background: var(--surface-2);
		border: 1px solid var(--border-soft);
		cursor: pointer;
		color: var(--text-muted);
		display: flex;
		align-items: center;
		justify-content: center;
		transition:
			background 0.15s ease,
			color 0.15s ease,
			border-color 0.15s ease;
	}

	.dock-settings-btn:hover {
		background: var(--surface-hover);
		color: var(--text-primary);
		border-color: var(--border-specular);
	}

	.dock-settings-btn.active {
		color: var(--accent, #c8a96e);
		border-color: color-mix(in srgb, var(--accent, #c8a96e) 45%, transparent);
		background: color-mix(in srgb, var(--accent, #c8a96e) 14%, var(--surface-1));
	}

	.user-popover {
		position: absolute;
		left: calc(100% + 12px);
		bottom: 0;
		width: 304px;
		background: var(--glass-bg, var(--surface-1));
		backdrop-filter: blur(20px) saturate(180%);
		-webkit-backdrop-filter: blur(20px) saturate(180%);
		border: 1px solid var(--border-specular);
		border-radius: 16px;
		padding: 14px 16px;
		box-shadow: var(--shadow-elevated);
		z-index: 250;
		opacity: 0;
		visibility: hidden;
		pointer-events: none;
		transform: translateX(-6px) scale(0.97);
		transform-origin: left bottom;
		transition:
			opacity 0.18s ease,
			transform 0.28s var(--spring-out);
	}

	/* Invisible hover bridge so moving pointer from dock into popover stays hovered */
	.user-popover::before {
		content: '';
		position: absolute;
		left: -16px;
		top: 0;
		bottom: 0;
		width: 16px;
	}

	.user-dock:hover .user-popover,
	.user-dock:focus-within .user-popover {
		opacity: 1;
		visibility: visible;
		pointer-events: auto;
		transform: translateX(0) scale(1);
	}

	.popover-header {
		display: flex;
		align-items: center;
		gap: 12px;
	}

	.popover-avatar {
		width: 46px;
		height: 46px;
		border-radius: 14px;
		background: var(--accent, #c8a96e);
		flex-shrink: 0;
		display: flex;
		align-items: center;
		justify-content: center;
		font-size: 16px;
		font-weight: 700;
		color: var(--accent-text);
		object-fit: cover;
		border: 1px solid var(--border-soft);
	}

	.user-info {
		flex: 1;
		min-width: 0;
	}

	.user-name-row {
		display: flex;
		align-items: center;
		gap: 8px;
		min-width: 0;
	}

	.user-name {
		font-size: 14px;
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

	.watcher-dot {
		width: 9px;
		height: 9px;
		border-radius: 50%;
		background: #6a7080;
		flex-shrink: 0;
		transition: background 0.3s ease;
	}

	.watcher-dot.active {
		background: #3ddc84;
	}

	.user-lvl-badge {
		font-size: 10px;
		font-weight: 800;
		padding: 1px 6px;
		border-radius: 99px;
		background: color-mix(in srgb, var(--accent, #c8a96e) 18%, transparent);
		border: 1px solid color-mix(in srgb, var(--accent, #c8a96e) 45%, transparent);
		color: var(--accent, #c8a96e);
		letter-spacing: 0.04em;
		flex-shrink: 0;
	}

	.user-gs {
		color: var(--accent, #c8a96e);
		font-weight: 700;
	}

	.user-xp-track {
		width: 100%;
		height: 4px;
		background: var(--surface-2);
		border-radius: 99px;
		margin-top: 6px;
		overflow: hidden;
	}

	.user-xp-fill {
		height: 100%;
		background: var(--accent, #c8a96e);
		border-radius: 99px;
		transition: width 0.3s var(--spring-out);
	}

	.user-xp-caption {
		display: flex;
		justify-content: space-between;
		font-size: 10px;
		color: var(--text-muted);
		margin-top: 4px;
	}

	.popover-footer {
		margin-top: 10px;
		padding-top: 10px;
		border-top: 1px solid var(--border-soft);
		display: flex;
		align-items: center;
		justify-content: space-between;
	}

	.popover-watcher-status {
		display: flex;
		align-items: center;
		gap: 8px;
		font-size: 11px;
		font-weight: 500;
		color: var(--text-secondary);
	}

	@media (max-width: 980px) {
		.game-slot {
			width: 42px;
			height: 42px;
			border-radius: 13px;
		}

		.user-trigger {
			width: 40px;
			height: 40px;
			border-radius: 13px;
		}
	}
</style>
