<script lang="ts">
	import { invoke } from '@tauri-apps/api/core';
	import { games, loadGames, getGameIconUrl, getGameCompletionPct } from '$lib/stores/Games.js';
	import { applyGameTheme, selectedGameId, toggleGameFavorite } from '$lib/stores/selectedGame.js';
	import { settings } from '$lib/stores/settings.js';
	import { i18n } from '$lib/stores/i18n.js';
	import { gameFilters } from '$lib/stores/uiPreferences.js';
	import { goto } from '$app/navigation';
	import { resolve } from '$app/paths';
	import type { Game } from '$lib/stores/Games.js';
	import { SvelteSet } from 'svelte/reactivity';

	async function autoGroup() {
		try {
			await invoke('auto_group_games');
			await loadGames();
		} catch (e) {
			console.error('Auto group failed:', e);
		}
	}

	let collections = $derived.by(() => {
		const tags = new SvelteSet<string>();
		$games.forEach((g) => g.tags?.forEach((t) => tags.add(t)));
		return Array.from(tags).sort();
	});

	function getExeName(game: Game): string {
		if (!game.path_buf) return '';
		const parts = game.path_buf.split(/[\\/]/);
		return parts[parts.length - 1] || '';
	}

	function formatDate(timestamp: number | null): string {
		if (!timestamp) return '---';
		return new Date(timestamp * 1000).toLocaleDateString($i18n.locale, {
			year: 'numeric',
			month: 'short',
			day: 'numeric'
		});
	}

	type GameStats = {
		completionPct: number;
		lastUnlockedTs: number;
	};

	let gameStatsMap = $derived.by(() => {
		const map = new Map<string, GameStats>();
		for (const g of $games) {
			const achs = g.achievements ?? [];
			let lastUnlockedTs = 0;
			for (const a of achs) {
				const t = a.unlocked_time ?? 0;
				if (t > lastUnlockedTs) lastUnlockedTs = t;
			}
			map.set(g.id, {
				completionPct: getGameCompletionPct(g),
				lastUnlockedTs
			});
		}
		return map;
	});

	let groupedGames = $derived.by(() => {
		const groups: { label: string; games: Game[] }[] = [];
		const map: Record<string, Game[]> = {};

		filteredGames.forEach((game) => {
			const lastUnlocked = gameStatsMap.get(game.id)?.lastUnlockedTs ?? 0;
			const date = lastUnlocked > 0 ? new Date(lastUnlocked * 1000) : null;
			const label = date
				? date.toLocaleDateString($i18n.locale, { month: 'long', year: 'numeric' })
				: $i18n.t('games.neverPlayed');

			if (!map[label]) {
				map[label] = [];
				groups.push({ label, games: map[label] });
			}
			map[label].push(game);
		});

		return groups;
	});

	let filteredGames = $derived.by((): Game[] => {
		let result = [...$games];
		const { status, favorite, collection, sortBy, sortOrder, searchQuery } = $gameFilters;

		// Filter by search (Name + Tags)
		if (searchQuery) {
			const q = searchQuery.toLowerCase();
			result = result.filter(
				(g) => g.name.toLowerCase().includes(q) || g.tags?.some((t) => t.toLowerCase().includes(q))
			);
		}

		// Filter by status
		if (status !== 'all') {
			result = result.filter((g) => {
				const total = g.achievements_total || g.achievements?.length || 0;
				const unlocked = g.achievements?.filter((a) => a.unlocked).length ?? 0;

				if (status === 'completed') return unlocked === total && total > 0;
				if (status === 'in-progress') return unlocked > 0 && unlocked < total;
				if (status === 'todo') return unlocked === 0;
				return true;
			});
		}

		// Filter by favorite
		if (favorite === 'yes') {
			result = result.filter((g) => g.is_favorite);
		} else if (favorite === 'no') {
			result = result.filter((g) => !g.is_favorite);
		}

		// Filter by collection (tag)
		if (collection !== 'all') {
			result = result.filter((g) => g.tags?.includes(collection));
		}

		// Sort
		result.sort((a, b) => {
			let cmp = 0;
			if (sortBy === 'name') {
				cmp = a.name.localeCompare(b.name);
			} else if (sortBy === 'completion') {
				cmp =
					(gameStatsMap.get(a.id)?.completionPct ?? 0) -
					(gameStatsMap.get(b.id)?.completionPct ?? 0);
			} else if (sortBy === 'exe_name') {
				cmp = getExeName(a).localeCompare(getExeName(b));
			} else if (sortBy === 'date') {
				const lastA = gameStatsMap.get(a.id)?.lastUnlockedTs ?? 0;
				const lastB = gameStatsMap.get(b.id)?.lastUnlockedTs ?? 0;
				cmp = lastA - lastB;
			}

			if (cmp === 0) cmp = a.name.localeCompare(b.name);

			return sortOrder === 'asc' ? cmp : -cmp;
		});

		return result;
	});

	function openGame(gameId: string) {
		void goto(resolve('/(app)/games/[id]', { id: gameId }));
	}

	function resetFilters() {
		gameFilters.reset();
	}

	function toggleSortOrder() {
		gameFilters.update((f) => ({
			...f,
			sortOrder: f.sortOrder === 'asc' ? 'desc' : 'asc'
		}));
	}
</script>

<main class="games-page">
	<div class="controls">
		<div class="search-box">
			<div class="search-wrapper">
				<input
					type="text"
					placeholder={$i18n.t('games.searchPlaceholder')}
					bind:value={$gameFilters.searchQuery}
					class="search-input"
				/>
				{#if $gameFilters.searchQuery}
					<button class="clear-search" onclick={() => ($gameFilters.searchQuery = '')}>×</button>
				{/if}
			</div>
		</div>

		<div class="filter-group">
			<div class="filter-item">
				<label for="status-filter">{$i18n.t('games.filter.status')}</label>
				<select id="status-filter" bind:value={$gameFilters.status} class="filter-select">
					<option value="all">{$i18n.t('games.status.all')}</option>
					<option value="todo">{$i18n.t('games.status.todo')}</option>
					<option value="in-progress">{$i18n.t('games.status.inProgress')}</option>
					<option value="completed">{$i18n.t('games.status.completed')}</option>
				</select>
			</div>

			<div class="filter-item">
				<label for="fav-filter">{$i18n.t('games.filter.favorite')}</label>
				<select id="fav-filter" bind:value={$gameFilters.favorite} class="filter-select">
					<option value="all">{$i18n.t('games.favorite.all')}</option>
					<option value="yes">{$i18n.t('games.favorite.yes')}</option>
					<option value="no">{$i18n.t('games.favorite.no')}</option>
				</select>
			</div>

			<div class="filter-item">
				<label for="collection-filter">{$i18n.t('games.filter.collection')}</label>
				<select id="collection-filter" bind:value={$gameFilters.collection} class="filter-select">
					<option value="all">{$i18n.t('games.filter.all')}</option>
					{#each collections as tag (tag)}
						<option value={tag}>{tag}</option>
					{/each}
				</select>
			</div>

			<div class="filter-item">
				<label for="sort-by">{$i18n.t('games.sort.label')}</label>
				<div class="sort-controls">
					<select id="sort-by" bind:value={$gameFilters.sortBy} class="filter-select">
						<option value="name">{$i18n.t('games.sort.name')}</option>
						<option value="date">{$i18n.t('games.sort.date')}</option>
						<option value="completion">{$i18n.t('games.sort.completion')}</option>
						<option value="exe_name">{$i18n.t('games.sort.exe')}</option>
					</select>
					<button
						class="sort-order-btn"
						onclick={toggleSortOrder}
						title={$i18n.t('games.sort.changeOrder')}
					>
						{$gameFilters.sortOrder === 'asc' ? '↑' : '↓'}
					</button>
				</div>
			</div>

			<button class="reset-btn" onclick={resetFilters}>
				{$i18n.t('games.reset')}
			</button>

			<button
				class="auto-group-btn"
				onclick={autoGroup}
				title={$i18n.t('collections.autoGroupTitle')}
			>
				{$i18n.t('collections.autoGroup')}
			</button>
		</div>

		<div class="view-modes">
			<button
				class="view-btn"
				class:active={$gameFilters.displayMode === 'grid'}
				onclick={() => ($gameFilters.displayMode = 'grid')}
				title={$i18n.t('games.view.grid')}
			>
				<svg
					width="15"
					height="15"
					viewBox="0 0 24 24"
					fill="none"
					stroke="currentColor"
					stroke-width="2"
					stroke-linecap="round"
					stroke-linejoin="round"
					aria-hidden="true"
				>
					<rect x="3" y="3" width="7" height="7" rx="1" />
					<rect x="14" y="3" width="7" height="7" rx="1" />
					<rect x="3" y="14" width="7" height="7" rx="1" />
					<rect x="14" y="14" width="7" height="7" rx="1" />
				</svg>
			</button>
			<button
				class="view-btn"
				class:active={$gameFilters.displayMode === 'list'}
				onclick={() => ($gameFilters.displayMode = 'list')}
				title={$i18n.t('games.view.list')}
			>
				<svg
					width="15"
					height="15"
					viewBox="0 0 24 24"
					fill="none"
					stroke="currentColor"
					stroke-width="2"
					stroke-linecap="round"
					stroke-linejoin="round"
					aria-hidden="true"
				>
					<line x1="8" y1="6" x2="21" y2="6" />
					<line x1="8" y1="12" x2="21" y2="12" />
					<line x1="8" y1="18" x2="21" y2="18" />
					<line x1="3" y1="6" x2="3.01" y2="6" />
					<line x1="3" y1="12" x2="3.01" y2="12" />
					<line x1="3" y1="18" x2="3.01" y2="18" />
				</svg>
			</button>
			<button
				class="view-btn"
				class:active={$gameFilters.displayMode === 'timeline'}
				onclick={() => ($gameFilters.displayMode = 'timeline')}
				title={$i18n.t('games.view.timeline')}
			>
				<svg
					width="15"
					height="15"
					viewBox="0 0 24 24"
					fill="none"
					stroke="currentColor"
					stroke-width="2"
					stroke-linecap="round"
					stroke-linejoin="round"
					aria-hidden="true"
				>
					<circle cx="12" cy="12" r="9" />
					<polyline points="12 7 12 12 15 14" />
				</svg>
			</button>
		</div>
	</div>

	<div class="results-info">
		{$i18n.t('games.results', { count: filteredGames.length })}
	</div>

	{#if $gameFilters.displayMode === 'grid'}
		<div class="games-grid">
			{#each filteredGames as game (game.id)}
				{@const pct = gameStatsMap.get(game.id)?.completionPct ?? 0}
				<div class="game-card-container">
					<div
						class="game-card"
						class:selected={$selectedGameId === game.id}
						role="button"
						tabindex="0"
						onclick={() => openGame(game.id)}
						onkeydown={(e) => {
							if (e.target !== e.currentTarget) return;
							if (e.key === 'Enter' || e.key === ' ') {
								e.preventDefault();
								openGame(game.id);
							}
						}}
					>
						<div class="card-image">
							{#if game.header_image_url}
								<img src={game.header_image_url} alt={game.name} loading="lazy" decoding="async" />
							{:else if game.steam_id}
								<img
									src="https://cdn.cloudflare.steamstatic.com/steam/apps/{game.steam_id}/header.jpg"
									alt={game.name}
									loading="lazy"
									decoding="async"
								/>
							{:else}
								<div class="no-image">{$i18n.t('games.noImage')}</div>
							{/if}
						</div>
						<div class="card-content">
							<h3 class="card-title">{game.name}</h3>
							<div class="card-progress">
								<div class="progress-bar">
									<div
										class="progress-fill"
										style:width={pct + '%'}
										style:background={pct === 100 ? 'var(--completed)' : 'var(--accent)'}
									></div>
								</div>
								<span
									class="progress-text"
									style:color={pct === 100 ? 'var(--completed)' : 'var(--accent)'}>{pct}%</span
								>
							</div>
							<div class="card-actions">
								<button
									class="action-btn theme-btn"
									class:active={$settings.dynamicTheme && $selectedGameId === game.id}
									onclick={(e) => {
										e.stopPropagation();
										applyGameTheme(game.id);
									}}
									title={$i18n.t('games.action.useAsTheme')}
									aria-label={$i18n.t('games.action.useAsTheme')}
								>
									<svg
										width="14"
										height="14"
										viewBox="0 0 24 24"
										fill="none"
										stroke="currentColor"
										stroke-width="2"
										stroke-linecap="round"
										stroke-linejoin="round"
										aria-hidden="true"
									>
										<circle cx="13.5" cy="6.5" r=".5" fill="currentColor" />
										<circle cx="17.5" cy="10.5" r=".5" fill="currentColor" />
										<circle cx="8.5" cy="7.5" r=".5" fill="currentColor" />
										<circle cx="6.5" cy="12.5" r=".5" fill="currentColor" />
										<path
											d="M12 2C6.5 2 2 6.5 2 12s4.5 10 10 10c.926 0 1.648-.746 1.648-1.688 0-.437-.18-.835-.437-1.125-.29-.289-.438-.652-.438-1.125a1.64 1.64 0 0 1 1.668-1.668h1.996c3.051 0 5.555-2.503 5.555-5.554C21.965 6.012 17.461 2 12 2z"
										/>
									</svg>
								</button>
								<button
									class="action-btn fav-btn"
									class:favorited={game.is_favorite}
									onclick={(e) => {
										e.stopPropagation();
										toggleGameFavorite(game.id);
									}}
									title={$i18n.t(
										game.is_favorite ? 'games.action.removeFavorite' : 'games.action.addFavorite'
									)}
									aria-label={$i18n.t(
										game.is_favorite ? 'games.action.removeFavorite' : 'games.action.addFavorite'
									)}
								>
									<svg
										width="14"
										height="14"
										viewBox="0 0 24 24"
										fill={game.is_favorite ? 'currentColor' : 'none'}
										stroke="currentColor"
										stroke-width="2"
										stroke-linecap="round"
										stroke-linejoin="round"
										aria-hidden="true"
									>
										<path
											d="M20.84 4.61a5.5 5.5 0 0 0-7.78 0L12 5.67l-1.06-1.06a5.5 5.5 0 0 0-7.78 7.78l1.06 1.06L12 21.23l7.78-7.78 1.06-1.06a5.5 5.5 0 0 0 0-7.78z"
										/>
									</svg>
								</button>
								<button
									class="action-btn open-btn"
									onclick={(e) => {
										e.stopPropagation();
										openGame(game.id);
									}}
									title={$i18n.t('games.action.openGame')}
									aria-label={$i18n.t('games.action.openGame')}
								>
									<svg
										width="14"
										height="14"
										viewBox="0 0 24 24"
										fill="none"
										stroke="currentColor"
										stroke-width="2"
										stroke-linecap="round"
										stroke-linejoin="round"
										aria-hidden="true"
									>
										<line x1="5" y1="12" x2="19" y2="12" />
										<polyline points="12 5 19 12 12 19" />
									</svg>
								</button>
							</div>
						</div>
					</div>
				</div>
			{/each}
		</div>
	{:else if $gameFilters.displayMode === 'list'}
		<div class="games-list">
			<div class="list-header">
				<span class="col-cover"></span>
				<span class="col-name">{$i18n.t('games.sort.name')}</span>
				<span class="col-exe">{$i18n.t('games.sort.exe')}</span>
				<span class="col-completion">{$i18n.t('games.sort.completion')}</span>
				<span class="col-date">{$i18n.t('games.sort.date')}</span>
				<span class="col-actions"></span>
			</div>
			{#each filteredGames as game (game.id)}
				{@const stats = gameStatsMap.get(game.id)}
				{@const pct = stats?.completionPct ?? 0}
				{@const lastTs = stats?.lastUnlockedTs ?? 0}
				<div
					class="list-item"
					class:selected={$selectedGameId === game.id}
					role="button"
					tabindex="0"
					onclick={() => openGame(game.id)}
					onkeydown={(e) => {
						if (e.target !== e.currentTarget) return;
						if (e.key === 'Enter' || e.key === ' ') {
							e.preventDefault();
							openGame(game.id);
						}
					}}
				>
					<div class="col-cover">
						<img
							src={getGameIconUrl(game)}
							alt={game.name}
							class="list-icon"
							loading="lazy"
							decoding="async"
						/>
					</div>
					<div class="col-name">
						<h3 class="list-title">{game.name}</h3>
					</div>
					<div class="col-exe">
						<span class="exe-name">{getExeName(game)}</span>
					</div>
					<div class="col-completion">
						<div class="list-progress">
							<div class="progress-bar">
								<div
									class="progress-fill"
									style:width={pct + '%'}
									style:background={pct === 100 ? 'var(--completed)' : 'var(--accent)'}
								></div>
							</div>
							<span
								class="list-pct"
								style:color={pct === 100 ? 'var(--completed)' : 'inherit'}>{pct}%</span
							>
						</div>
					</div>
					<div class="col-date">
						<span class="date-text">{formatDate(lastTs)}</span>
					</div>
					<div class="col-actions">
						<div class="list-actions">
							<button
								class="action-btn theme-btn"
								class:active={$settings.dynamicTheme && $selectedGameId === game.id}
								onclick={(e) => {
									e.stopPropagation();
									applyGameTheme(game.id);
								}}
								title={$i18n.t('games.action.useAsTheme')}
								aria-label={$i18n.t('games.action.useAsTheme')}
							>
								<svg
									width="14"
									height="14"
									viewBox="0 0 24 24"
									fill="none"
									stroke="currentColor"
									stroke-width="2"
									stroke-linecap="round"
									stroke-linejoin="round"
									aria-hidden="true"
								>
									<circle cx="13.5" cy="6.5" r=".5" fill="currentColor" />
									<circle cx="17.5" cy="10.5" r=".5" fill="currentColor" />
									<circle cx="8.5" cy="7.5" r=".5" fill="currentColor" />
									<circle cx="6.5" cy="12.5" r=".5" fill="currentColor" />
									<path
										d="M12 2C6.5 2 2 6.5 2 12s4.5 10 10 10c.926 0 1.648-.746 1.648-1.688 0-.437-.18-.835-.437-1.125-.29-.289-.438-.652-.438-1.125a1.64 1.64 0 0 1 1.668-1.668h1.996c3.051 0 5.555-2.503 5.555-5.554C21.965 6.012 17.461 2 12 2z"
									/>
								</svg>
							</button>
							<button
								class="action-btn fav-btn"
								class:favorited={game.is_favorite}
								onclick={(e) => {
									e.stopPropagation();
									toggleGameFavorite(game.id);
								}}
								title={$i18n.t(
									game.is_favorite ? 'games.action.removeFavorite' : 'games.action.addFavorite'
								)}
								aria-label={$i18n.t(
									game.is_favorite ? 'games.action.removeFavorite' : 'games.action.addFavorite'
								)}
							>
								<svg
									width="14"
									height="14"
									viewBox="0 0 24 24"
									fill={game.is_favorite ? 'currentColor' : 'none'}
									stroke="currentColor"
									stroke-width="2"
									stroke-linecap="round"
									stroke-linejoin="round"
									aria-hidden="true"
								>
									<path
										d="M20.84 4.61a5.5 5.5 0 0 0-7.78 0L12 5.67l-1.06-1.06a5.5 5.5 0 0 0-7.78 7.78l1.06 1.06L12 21.23l7.78-7.78 1.06-1.06a5.5 5.5 0 0 0 0-7.78z"
									/>
								</svg>
							</button>
							<button
								class="action-btn open-btn"
								onclick={(e) => {
									e.stopPropagation();
									openGame(game.id);
								}}
								title={$i18n.t('games.action.openGame')}
								aria-label={$i18n.t('games.action.openGame')}
							>
								<svg
									width="14"
									height="14"
									viewBox="0 0 24 24"
									fill="none"
									stroke="currentColor"
									stroke-width="2"
									stroke-linecap="round"
									stroke-linejoin="round"
									aria-hidden="true"
								>
									<line x1="5" y1="12" x2="19" y2="12" />
									<polyline points="12 5 19 12 12 19" />
								</svg>
							</button>
						</div>
					</div>
				</div>
			{/each}
		</div>
	{:else}
		<div class="games-timeline">
			{#each groupedGames as group (group.label)}
				<div class="timeline-group">
					<div class="timeline-group-header">
						<span class="group-label">{group.label}</span>
					</div>
					<div class="timeline-items">
						{#each group.games as game (game.id)}
							{@const stats = gameStatsMap.get(game.id)}
							{@const pct = stats?.completionPct ?? 0}
							{@const lastTs = stats?.lastUnlockedTs ?? 0}
							<div class="timeline-item" class:selected={$selectedGameId === game.id}>
								<div class="timeline-dot"></div>
								<div
									class="timeline-content"
									role="button"
									tabindex="0"
									onclick={() => openGame(game.id)}
									onkeydown={(e) => {
										if (e.target !== e.currentTarget) return;
										if (e.key === 'Enter' || e.key === ' ') {
											e.preventDefault();
											openGame(game.id);
										}
									}}
								>
									<img
										src={getGameIconUrl(game)}
										alt={game.name}
										class="timeline-icon"
										loading="lazy"
										decoding="async"
									/>
									<div class="timeline-info">
										<div class="timeline-header">
											<h3 class="timeline-title">{game.name}</h3>
											<span class="timeline-date">{formatDate(lastTs)}</span>
										</div>
										<div class="timeline-progress-row">
											<div class="progress-bar">
												<div
													class="progress-fill"
													style:width={pct + '%'}
													style:background={pct === 100 ? 'var(--completed)' : 'var(--accent)'}
												></div>
											</div>
											<span
												class="timeline-completion"
												style:color={pct === 100 ? 'var(--completed)' : 'var(--accent)'}
												>{pct}%</span
											>
										</div>
									</div>
									<div class="timeline-actions">
										<button
											class="action-btn theme-btn"
											class:active={$settings.dynamicTheme && $selectedGameId === game.id}
											onclick={(e) => {
												e.stopPropagation();
												applyGameTheme(game.id);
											}}
											title={$i18n.t('games.action.useAsTheme')}
											aria-label={$i18n.t('games.action.useAsTheme')}
										>
											<svg
												width="14"
												height="14"
												viewBox="0 0 24 24"
												fill="none"
												stroke="currentColor"
												stroke-width="2"
												stroke-linecap="round"
												stroke-linejoin="round"
												aria-hidden="true"
											>
												<circle cx="13.5" cy="6.5" r=".5" fill="currentColor" />
												<circle cx="17.5" cy="10.5" r=".5" fill="currentColor" />
												<circle cx="8.5" cy="7.5" r=".5" fill="currentColor" />
												<circle cx="6.5" cy="12.5" r=".5" fill="currentColor" />
												<path
													d="M12 2C6.5 2 2 6.5 2 12s4.5 10 10 10c.926 0 1.648-.746 1.648-1.688 0-.437-.18-.835-.437-1.125-.29-.289-.438-.652-.438-1.125a1.64 1.64 0 0 1 1.668-1.668h1.996c3.051 0 5.555-2.503 5.555-5.554C21.965 6.012 17.461 2 12 2z"
												/>
											</svg>
										</button>
										<button
											class="action-btn fav-btn"
											class:favorited={game.is_favorite}
											onclick={(e) => {
												e.stopPropagation();
												toggleGameFavorite(game.id);
											}}
											title={$i18n.t(
												game.is_favorite
													? 'games.action.removeFavorite'
													: 'games.action.addFavorite'
											)}
											aria-label={$i18n.t(
												game.is_favorite
													? 'games.action.removeFavorite'
													: 'games.action.addFavorite'
											)}
										>
											<svg
												width="14"
												height="14"
												viewBox="0 0 24 24"
												fill={game.is_favorite ? 'currentColor' : 'none'}
												stroke="currentColor"
												stroke-width="2"
												stroke-linecap="round"
												stroke-linejoin="round"
												aria-hidden="true"
											>
												<path
													d="M20.84 4.61a5.5 5.5 0 0 0-7.78 0L12 5.67l-1.06-1.06a5.5 5.5 0 0 0-7.78 7.78l1.06 1.06L12 21.23l7.78-7.78 1.06-1.06a5.5 5.5 0 0 0 0-7.78z"
												/>
											</svg>
										</button>
										<button
											class="action-btn open-btn"
											onclick={(e) => {
												e.stopPropagation();
												openGame(game.id);
											}}
											title={$i18n.t('games.action.openGame')}
											aria-label={$i18n.t('games.action.openGame')}
										>
											<svg
												width="14"
												height="14"
												viewBox="0 0 24 24"
												fill="none"
												stroke="currentColor"
												stroke-width="2"
												stroke-linecap="round"
												stroke-linejoin="round"
												aria-hidden="true"
											>
												<line x1="5" y1="12" x2="19" y2="12" />
												<polyline points="12 5 19 12 12 19" />
											</svg>
										</button>
									</div>
								</div>
							</div>
						{/each}
					</div>
				</div>
			{/each}
		</div>
	{/if}
</main>

<style>
	.games-page {
		display: flex;
		flex-direction: column;
		height: 100%;
		overflow: hidden;
		padding: 16px;
	}

	.controls {
		display: flex;
		gap: 12px;
		margin-bottom: 16px;
		flex-wrap: wrap;
		align-items: center;
	}

	.search-box {
		flex: 1;
		min-width: 250px;
	}

	.search-wrapper {
		position: relative;
		display: flex;
		align-items: center;
	}

	.search-input {
		width: 100%;
		padding: 8px 12px;
		padding-right: 32px;
		background: var(--surface-1);
		border: 1px solid var(--border-soft);
		border-radius: 8px;
		color: var(--text-primary);
		font-size: 14px;
	}

	.clear-search {
		position: absolute;
		right: 8px;
		background: none;
		border: none;
		color: var(--text-muted);
		cursor: pointer;
		font-size: 18px;
		padding: 0 4px;
	}

	.clear-search:hover {
		color: var(--text-primary);
	}

	.filter-group {
		display: flex;
		gap: 12px;
		flex-wrap: wrap;
		align-items: flex-end;
	}

	.filter-item {
		display: flex;
		flex-direction: column;
		gap: 4px;
	}

	.filter-item label {
		font-size: 11px;
		font-weight: 600;
		color: var(--text-muted);
		text-transform: uppercase;
		letter-spacing: 0.05em;
	}

	.filter-select {
		padding: 8px 12px;
		background: var(--surface-1);
		border: 1px solid var(--border-soft);
		border-radius: 8px;
		color: var(--text-primary);
		font-size: 13px;
		cursor: pointer;
		min-width: 120px;
	}

	.sort-controls {
		display: flex;
		gap: 4px;
	}

	.sort-order-btn {
		width: 36px;
		height: 36px;
		background: var(--surface-1);
		border: 1px solid var(--border-soft);
		border-radius: 8px;
		color: var(--text-primary);
		cursor: pointer;
		display: flex;
		align-items: center;
		justify-content: center;
		font-weight: bold;
	}

	.reset-btn {
		padding: 8px 16px;
		background: var(--surface-2);
		border: 1px solid var(--border-soft);
		border-radius: 8px;
		color: var(--text-secondary);
		font-size: 13px;
		cursor: pointer;
		transition: all 0.2s;
		height: 36px;
	}

	.reset-btn:hover {
		background: var(--surface-3);
		color: var(--accent);
		border-color: var(--accent);
	}

	.auto-group-btn {
		padding: 8px 14px;
		background: color-mix(in srgb, var(--accent) 12%, transparent);
		border: 1px solid color-mix(in srgb, var(--accent) 25%, transparent);
		border-radius: 8px;
		color: var(--accent);
		font-size: 13px;
		font-weight: 600;
		cursor: pointer;
		transition: all 0.2s;
		display: flex;
		align-items: center;
		gap: 6px;
		height: 36px;
	}

	.results-info {
		font-size: 12px;
		color: var(--text-muted);
		margin-bottom: 12px;
		font-variant-numeric: tabular-nums;
	}

	.auto-group-btn:hover {
		background: color-mix(in srgb, var(--accent) 20%, transparent);
		border-color: var(--accent);
		transform: translateY(-1px);
	}

	.view-modes {
		display: flex;
		gap: 4px;
		background: var(--surface-2);
		padding: 4px;
		border-radius: 8px;
	}

	.view-btn {
		width: 32px;
		height: 32px;
		border: none;
		background: transparent;
		color: var(--text-secondary);
		cursor: pointer;
		border-radius: 7px;
		display: inline-flex;
		align-items: center;
		justify-content: center;
		transition: all 0.2s;
	}

	.view-btn.active {
		background: var(--surface-1);
		color: var(--accent);
	}

	.games-grid {
		display: grid;
		grid-template-columns: repeat(auto-fill, minmax(280px, 1fr));
		gap: 20px;
		overflow-y: auto;
		flex: 1;
		padding: 4px;
		align-content: start;
		align-items: start;
	}

	.game-card-container {
		display: flex;
		content-visibility: auto;
		contain-intrinsic-size: 200px 260px;
	}

	.game-card {
		border: 1px solid var(--border-soft);
		border-radius: 12px;
		overflow: hidden;
		background: var(--surface-1);
		transition: all 0.2s cubic-bezier(0.4, 0, 0.2, 1);
		cursor: pointer;
		display: flex;
		flex-direction: column;
		justify-content: space-between;
		width: 100%;
		position: relative;
	}

	.game-card:hover,
	.game-card.selected {
		border-color: var(--accent);
		background: var(--surface-2);
		transform: translateY(-4px);
		box-shadow: var(--shadow-card, 0 8px 24px rgba(0, 0, 0, 0.2));
	}

	.card-image {
		width: 100%;
		aspect-ratio: 16/9;
		overflow: hidden;
		background: var(--surface-3);
		position: relative;
	}

	.card-image img {
		width: 100%;
		height: 100%;
		object-fit: cover;
	}

	.no-image {
		width: 100%;
		height: 100%;
		display: flex;
		align-items: center;
		justify-content: center;
		color: var(--text-muted);
		font-size: 12px;
	}

	.card-content {
		padding: 16px;
		display: flex;
		flex-direction: column;
		flex: 1;
		gap: 12px;
	}

	.card-title {
		margin: 0;
		font-size: 15px;
		font-weight: 600;
		color: var(--text-primary);
		line-height: 1.4;
		display: -webkit-box;
		-webkit-line-clamp: 2;
		line-clamp: 2;
		-webkit-box-orient: vertical;
		overflow: hidden;
		min-height: 42px;
	}

	.card-progress {
		display: flex;
		align-items: center;
		gap: 12px;
		font-size: 12px;
	}

	.progress-bar {
		flex: 1;
		height: 6px;
		background: var(--surface-3);
		border-radius: 3px;
		overflow: hidden;
	}

	.progress-fill {
		height: 100%;
		background: var(--accent);
		transition: width 0.6s cubic-bezier(0.4, 0, 0.2, 1);
	}

	.progress-text {
		color: var(--accent);
		font-weight: 700;
		font-size: 13px;
		min-width: 35px;
		text-align: right;
		font-variant-numeric: tabular-nums;
	}

	.card-actions {
		display: flex;
		gap: 8px;
		margin-top: auto;
	}

	.action-btn {
		flex: 1;
		padding: 8px;
		background: var(--surface-2);
		border: 1px solid var(--border-soft);
		border-radius: 6px;
		color: var(--text-secondary);
		cursor: pointer;
		font-size: 14px;
		transition: all 0.2s;
		display: flex;
		align-items: center;
		justify-content: center;
	}

	.action-btn:hover {
		background: var(--surface-3);
		color: var(--accent);
		border-color: var(--accent);
	}

	.action-btn.theme-btn.active {
		color: var(--accent);
		border-color: color-mix(in srgb, var(--accent) 55%, transparent);
		background: color-mix(in srgb, var(--accent) 14%, transparent);
	}

	.action-btn.fav-btn.favorited {
		color: #ff4d4d;
		border-color: rgba(255, 77, 77, 0.3);
		background: rgba(255, 77, 77, 0.05);
	}

	/* List View Styles */
	.games-list {
		display: flex;
		flex-direction: column;
		overflow-y: auto;
		flex: 1;
		background: var(--surface-1);
		border-radius: 12px;
		border: 1px solid var(--border-soft);
	}

	.list-header {
		display: flex;
		align-items: center;
		padding: 12px 20px;
		background: var(--surface-2);
		border-bottom: 1px solid var(--border-soft);
		font-size: 11px;
		font-weight: 700;
		color: var(--text-muted);
		text-transform: uppercase;
		letter-spacing: 0.05em;
		position: sticky;
		top: 0;
		z-index: 10;
	}

	.list-item {
		display: flex;
		align-items: center;
		padding: 12px 20px;
		border-bottom: 1px solid var(--border-soft);
		transition: all 0.2s;
		cursor: pointer;
		content-visibility: auto;
		contain-intrinsic-size: 0 72px;
	}

	.list-item:hover,
	.list-item.selected {
		background: var(--surface-2);
	}

	.list-item.selected {
		border-left: 3px solid var(--accent);
		padding-left: 17px;
	}

	.col-cover {
		width: 48px;
		flex-shrink: 0;
	}
	.col-name {
		flex: 3;
		min-width: 200px;
		padding: 0 16px;
	}
	.col-exe {
		flex: 2;
		min-width: 150px;
		padding: 0 16px;
		color: var(--text-muted);
		font-family: monospace;
		font-size: 12px;
	}
	.col-completion {
		flex: 2;
		min-width: 150px;
		padding: 0 16px;
	}
	.col-date {
		flex: 1.5;
		min-width: 120px;
		padding: 0 16px;
		color: var(--text-secondary);
		font-size: 13px;
		font-variant-numeric: tabular-nums;
	}
	.col-actions {
		width: 120px;
		flex-shrink: 0;
		display: flex;
		justify-content: flex-end;
	}

	.list-icon {
		width: 40px;
		height: 40px;
		border-radius: 6px;
		object-fit: cover;
		background: var(--surface-3);
	}

	.list-title {
		margin: 0;
		font-size: 14px;
		font-weight: 600;
		color: var(--text-primary);
	}

	.exe-name {
		white-space: nowrap;
		overflow: hidden;
		text-overflow: ellipsis;
		display: block;
	}

	.list-progress {
		display: flex;
		align-items: center;
		gap: 8px;
		font-size: 12px;
		color: var(--text-secondary);
		font-variant-numeric: tabular-nums;
	}

	.list-actions {
		display: flex;
		gap: 6px;
	}

	.list-actions .action-btn {
		width: 32px;
		height: 32px;
		padding: 0;
		flex: none;
	}

	/* Timeline View Styles */
	.games-timeline {
		display: flex;
		flex-direction: column;
		gap: 32px;
		overflow-y: auto;
		flex: 1;
		padding: 20px;
		padding-left: 40px;
		position: relative;
	}

	.games-timeline::before {
		content: '';
		position: absolute;
		left: 27px;
		top: 0;
		bottom: 0;
		width: 2px;
		background: linear-gradient(
			to bottom,
			transparent,
			var(--border-soft) 5%,
			var(--border-soft) 95%,
			transparent
		);
	}

	.timeline-group {
		display: flex;
		flex-direction: column;
		gap: 16px;
	}

	.timeline-group-header {
		position: relative;
		margin-bottom: 8px;
	}

	.group-label {
		background: var(--surface-3);
		color: var(--accent);
		padding: 4px 12px;
		border-radius: 20px;
		font-size: 12px;
		font-weight: 700;
		text-transform: uppercase;
		letter-spacing: 0.05em;
		border: 1px solid color-mix(in srgb, var(--accent) 30%, transparent);
	}

	.timeline-items {
		display: flex;
		flex-direction: column;
		gap: 12px;
	}

	.timeline-item {
		display: flex;
		align-items: flex-start;
		gap: 20px;
		position: relative;
	}

	.timeline-dot {
		width: 12px;
		height: 12px;
		background: var(--accent);
		border-radius: 50%;
		flex-shrink: 0;
		margin-top: 24px;
		z-index: 1;
		box-shadow: 0 0 0 3px color-mix(in srgb, var(--accent) 18%, transparent);
		position: absolute;
		left: -20px;
	}

	.timeline-content {
		flex: 1;
		display: flex;
		align-items: center;
		gap: 16px;
		padding: 12px 16px;
		background: var(--surface-1);
		border: 1px solid var(--border-soft);
		border-radius: 12px;
		transition: all 0.2s;
		cursor: pointer;
	}

	.timeline-content:hover,
	.timeline-item.selected .timeline-content {
		border-color: var(--accent);
		background: var(--surface-2);
		transform: translateX(4px);
	}

	.timeline-icon {
		width: 48px;
		height: 48px;
		border-radius: 8px;
		object-fit: cover;
	}

	.timeline-info {
		flex: 1;
		display: flex;
		flex-direction: column;
		gap: 8px;
	}

	.timeline-header {
		display: flex;
		justify-content: space-between;
		align-items: baseline;
	}

	.timeline-title {
		margin: 0;
		font-size: 15px;
		font-weight: 600;
		color: var(--text-primary);
	}

	.timeline-date {
		font-size: 12px;
		color: var(--text-muted);
		font-variant-numeric: tabular-nums;
	}

	.timeline-progress-row {
		display: flex;
		align-items: center;
		gap: 12px;
	}

	.timeline-completion {
		font-size: 13px;
		font-weight: 600;
		color: var(--accent);
		min-width: 40px;
		font-variant-numeric: tabular-nums;
	}

	.timeline-actions {
		display: flex;
		gap: 6px;
	}

	.timeline-actions .action-btn {
		width: 36px;
		height: 36px;
		padding: 0;
		flex: none;
	}
</style>
