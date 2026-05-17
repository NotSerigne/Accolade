<script lang="ts">
	import { games } from '$lib/stores/Games.js';
	import { selectedGameId, toggleGameFavorite, addGameTag } from '$lib/stores/selectedGame.js';
	import { i18n } from '$lib/stores/i18n.js';
	import { goto } from '$app/navigation';
	import { resolve } from '$app/paths';
	import type { Game } from '$lib/stores/Games.js';
	import { SvelteSet } from 'svelte/reactivity';

	let displayMode = $state<'grid' | 'list' | 'timeline'>('grid');
	let filterMode = $state<'all' | 'completed' | 'in-progress' | 'favorites' | string>('all');
	let searchQuery = $state('');
	let sortBy = $state<'name' | 'completion' | 'date'>('name');

	function autoGroup() {
		$games.forEach((game) => {
			const type =
				typeof game.source === 'object' && game.source.type === 'Emulator'
					? game.source.value
					: game.source.type;

			if (!game.tags?.includes(type)) {
				addGameTag(game.id, type);
			}

			// Similar name grouping (e.g. "God of War")
			const mainName = game.name.split(':')[0].split('-')[0].trim();
			const similarGames = $games.filter((g) => g.name.startsWith(mainName) && g.id !== game.id);
			if (similarGames.length > 0 && !game.tags?.includes(mainName)) {
				addGameTag(game.id, mainName);
			}
		});
	}

	let collections = $derived.by(() => {
		const tags = new SvelteSet<string>();
		$games.forEach((g) => g.tags?.forEach((t) => tags.add(t)));
		return Array.from(tags).sort();
	});

	let filteredGames = $derived.by((): Game[] => {
		let result = $games;

		// Filter by search
		if (searchQuery) {
			const q = searchQuery.toLowerCase();
			result = result.filter((g) => g.name.toLowerCase().includes(q));
		}

		// Filter by status/collection
		if (filterMode === 'favorites') {
			result = result.filter((g) => g.is_favorite);
		} else if (filterMode === 'completed' || filterMode === 'in-progress') {
			result = result.filter((g) => {
				const total = g.achievements_total || g.achievements?.length || 0;
				if (total === 0) return false;
				const unlocked = g.achievements?.filter((a) => a.unlocked).length ?? 0;
				const completion = unlocked === total;

				return filterMode === 'completed' ? completion : !completion;
			});
		} else if (filterMode !== 'all') {
			// Assume it's a collection (tag)
			result = result.filter((g) => g.tags?.includes(filterMode));
		}

		// Sort
		result.sort((a, b) => {
			if (sortBy === 'name') {
				return a.name.localeCompare(b.name);
			} else if (sortBy === 'completion') {
				const totalA = a.achievements_total || a.achievements?.length || 0;
				const totalB = b.achievements_total || b.achievements?.length || 0;
				const unlockedA = a.achievements?.filter((x) => x.unlocked).length ?? 0;
				const unlockedB = b.achievements?.filter((x) => x.unlocked).length ?? 0;
				const pctA = totalA ? unlockedA / totalA : 0;
				const pctB = totalB ? unlockedB / totalB : 0;
				return pctB - pctA;
			} else {
				const lastA = Math.max(...(a.achievements ?? []).map((x) => x.unlocked_time ?? 0), 0);
				const lastB = Math.max(...(b.achievements ?? []).map((x) => x.unlocked_time ?? 0), 0);
				return lastB - lastA;
			}
		});

		return result;
	});

	function getCompletion(game: Game): number {
		const total = game.achievements_total || game.achievements?.length || 0;
		if (total === 0) return 0;
		const unlocked = game.achievements?.filter((a) => a.unlocked).length ?? 0;
		return Math.round((unlocked / total) * 100);
	}

	function selectGameTheme(gameId: string) {
		selectedGameId.set(gameId);
	}

	function openGame(gameId: string) {
		void goto(resolve('/(app)/games/[id]', { id: gameId }));
	}
</script>

<main class="games-page">
	<div class="controls">
		<div class="search-box">
			<input
				type="text"
				placeholder="Search games..."
				bind:value={searchQuery}
				class="search-input"
			/>
		</div>

		<div class="filter-group">
			<button
				class="auto-group-btn"
				onclick={autoGroup}
				title={$i18n.t('collections.autoGroupTitle')}
			>
				{$i18n.t('collections.autoGroup')}
			</button>

			<select bind:value={filterMode} class="filter-select">
				<option value="all">All Games</option>
				<option value="favorites">♥️ Favorites</option>
				<option value="in-progress">In Progress</option>
				<option value="completed">Completed</option>
				{#if collections.length > 0}
					<optgroup label="Collections">
						{#each collections as tag (tag)}
							<option value={tag}>{tag}</option>
						{/each}
					</optgroup>
				{/if}
			</select>

			<select bind:value={sortBy} class="filter-select">
				<option value="name">Sort by Name</option>
				<option value="completion">Sort by Completion</option>
				<option value="date">Sort by Date</option>
			</select>
		</div>

		<div class="view-modes">
			<button
				class="view-btn"
				class:active={displayMode === 'grid'}
				onclick={() => (displayMode = 'grid')}
				title="Grid view"
			>
				⊞
			</button>
			<button
				class="view-btn"
				class:active={displayMode === 'list'}
				onclick={() => (displayMode = 'list')}
				title="List view"
			>
				≡
			</button>
			<button
				class="view-btn"
				class:active={displayMode === 'timeline'}
				onclick={() => (displayMode = 'timeline')}
				title="Timeline view"
			>
				◆
			</button>
		</div>
	</div>

	{#if displayMode === 'grid'}
		<div class="games-grid">
			{#each filteredGames as game (game.id)}
				<div class="game-card-container">
					<div class="game-card" class:selected={$selectedGameId === game.id}>
						<div class="card-image">
							{#if game.header_image_url}
								<img src={game.header_image_url} alt={game.name} />
							{:else if game.steam_id}
								<img
									src="https://cdn.cloudflare.steamstatic.com/steam/apps/{game.steam_id}/header.jpg"
									alt={game.name}
								/>
							{:else}
								<div class="no-image">No image</div>
							{/if}
						</div>
						<div class="card-content">
							<h3 class="card-title">{game.name}</h3>
							<div class="card-progress">
								<div class="progress-bar">
									<div class="progress-fill" style:width={getCompletion(game) + '%'}></div>
								</div>
								<span class="progress-text">{getCompletion(game)}%</span>
							</div>
							<div class="card-actions">
								<button
									class="action-btn theme-btn"
									onclick={() => selectGameTheme(game.id)}
									title="Use as theme"
								>
									🎨
								</button>
								<button
									class="action-btn fav-btn"
									class:favorited={game.is_favorite}
									onclick={(e) => {
										e.stopPropagation();
										toggleGameFavorite(game.id);
									}}
									title="Toggle favorite"
								>
									{game.is_favorite ? '♥️' : '♡'}
								</button>
								<button
									class="action-btn open-btn"
									onclick={() => openGame(game.id)}
									title="Open game"
								>
									→
								</button>
							</div>
						</div>
					</div>
				</div>
			{/each}
		</div>
	{:else if displayMode === 'list'}
		<div class="games-list">
			{#each filteredGames as game (game.id)}
				<div class="list-item" class:selected={$selectedGameId === game.id}>
					<img src={game.game_icon} alt={game.name} class="list-icon" />
					<div class="list-info">
						<h3 class="list-title">{game.name}</h3>
						<div class="list-progress">
							<div class="progress-bar">
								<div class="progress-fill" style:width={getCompletion(game) + '%'}></div>
							</div>
							<span>{getCompletion(game)}%</span>
						</div>
					</div>
					<div class="list-actions">
						<button
							class="action-btn theme-btn"
							onclick={() => selectGameTheme(game.id)}
							title="Use as theme"
						>
							🎨
						</button>
						<button
							class="action-btn fav-btn"
							class:favorited={game.is_favorite}
							onclick={(e) => {
								e.stopPropagation();
								toggleGameFavorite(game.id);
							}}
							title="Toggle favorite"
						>
							{game.is_favorite ? '♥️' : '♡'}
						</button>
						<button class="action-btn open-btn" onclick={() => openGame(game.id)} title="Open game">
							→
						</button>
					</div>
				</div>
			{/each}
		</div>
	{:else}
		<div class="games-timeline">
			{#each filteredGames as game (game.id)}
				<div class="timeline-item" class:selected={$selectedGameId === game.id}>
					<div class="timeline-dot"></div>
					<div class="timeline-content">
						<div class="timeline-header">
							<h3 class="timeline-title">{game.name}</h3>
							<span class="timeline-completion">{getCompletion(game)}%</span>
						</div>
						<div class="timeline-actions">
							<button
								class="action-btn theme-btn"
								onclick={() => selectGameTheme(game.id)}
								title="Use as theme"
							>
								🎨
							</button>
							<button
								class="action-btn fav-btn"
								class:favorited={game.is_favorite}
								onclick={(e) => {
									e.stopPropagation();
									toggleGameFavorite(game.id);
								}}
								title="Toggle favorite"
							>
								{game.is_favorite ? '♥️' : '♡'}
							</button>
							<button
								class="action-btn open-btn"
								onclick={() => openGame(game.id)}
								title="Open game"
							>
								→
							</button>
						</div>
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
		min-width: 200px;
	}

	.search-input {
		width: 100%;
		padding: 8px 12px;
		background: var(--surface-1);
		border: 1px solid var(--border-soft);
		border-radius: 8px;
		color: var(--text-primary);
		font-size: 14px;
	}

	.filter-group {
		display: flex;
		gap: 8px;
	}

	.filter-select {
		padding: 8px 12px;
		background: var(--surface-1);
		border: 1px solid var(--border-soft);
		border-radius: 8px;
		color: var(--text-primary);
		font-size: 14px;
		cursor: pointer;
	}

	.auto-group-btn {
		padding: 8px 14px;
		background: rgba(200, 169, 110, 0.1);
		border: 1px solid rgba(200, 169, 110, 0.2);
		border-radius: 8px;
		color: var(--accent);
		font-size: 13px;
		font-weight: 600;
		cursor: pointer;
		transition: all 0.2s;
		display: flex;
		align-items: center;
		gap: 6px;
	}

	.auto-group-btn:hover {
		background: rgba(200, 169, 110, 0.2);
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
		border-radius: 4px;
		font-size: 16px;
		transition: all 0.2s;
	}

	.view-btn.active {
		background: var(--surface-1);
		color: var(--accent);
	}

	.games-grid {
		display: grid;
		grid-template-columns: repeat(auto-fill, minmax(180px, 1fr));
		gap: 16px;
		overflow-y: auto;
		flex: 1;
	}

	.game-card-container {
		display: flex;
	}

	.game-card {
		border: 2px solid var(--border-soft);
		border-radius: 8px;
		overflow: hidden;
		background: var(--surface-1);
		transition: all 0.2s;
		cursor: pointer;
		display: flex;
		flex-direction: column;
		height: 100%;
	}

	.game-card:hover,
	.game-card.selected {
		border-color: var(--accent);
		background: var(--surface-2);
	}

	.card-image {
		width: 100%;
		aspect-ratio: 16/9;
		overflow: hidden;
		background: var(--surface-3);
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
		padding: 12px;
		display: flex;
		flex-direction: column;
		flex: 1;
	}

	.card-title {
		margin: 0 0 8px;
		font-size: 14px;
		font-weight: 600;
		color: var(--text-primary);
		display: -webkit-box;
		-webkit-line-clamp: 2;
		line-clamp: 2;
		-webkit-box-orient: vertical;
		overflow: hidden;
	}

	.card-progress {
		display: flex;
		align-items: center;
		gap: 8px;
		margin-bottom: 12px;
		font-size: 12px;
	}

	.progress-bar {
		flex: 1;
		height: 4px;
		background: var(--surface-3);
		border-radius: 2px;
		overflow: hidden;
	}

	.progress-fill {
		height: 100%;
		background: var(--accent);
		transition: width 0.3s;
	}

	.progress-text {
		color: var(--text-secondary);
		font-size: 11px;
		min-width: 30px;
		text-align: right;
	}

	.card-actions {
		display: flex;
		gap: 4px;
	}

	.action-btn {
		flex: 1;
		padding: 6px;
		background: var(--surface-2);
		border: 1px solid var(--border-soft);
		border-radius: 4px;
		color: var(--text-secondary);
		cursor: pointer;
		font-size: 12px;
		transition: all 0.2s;
	}

	.action-btn:hover {
		background: var(--surface-3);
		color: var(--accent);
	}

	.action-btn.fav-btn.favorited {
		color: #ff4d4d;
	}

	.games-list {
		display: flex;
		flex-direction: column;
		gap: 8px;
		overflow-y: auto;
		flex: 1;
	}

	.list-item {
		display: flex;
		align-items: center;
		gap: 12px;
		padding: 12px;
		background: var(--surface-1);
		border: 2px solid var(--border-soft);
		border-radius: 8px;
		transition: all 0.2s;
	}

	.list-item:hover,
	.list-item.selected {
		border-color: var(--accent);
		background: var(--surface-2);
	}

	.list-icon {
		width: 48px;
		height: 48px;
		border-radius: 4px;
		object-fit: cover;
	}

	.list-info {
		flex: 1;
	}

	.list-title {
		margin: 0 0 6px;
		font-size: 14px;
		font-weight: 600;
		color: var(--text-primary);
	}

	.list-progress {
		display: flex;
		align-items: center;
		gap: 8px;
		font-size: 12px;
		color: var(--text-secondary);
	}

	.list-progress .progress-bar {
		width: 100px;
		height: 3px;
	}

	.list-actions {
		display: flex;
		gap: 8px;
	}

	.games-timeline {
		display: flex;
		flex-direction: column;
		gap: 16px;
		overflow-y: auto;
		flex: 1;
		padding-left: 20px;
		position: relative;
	}

	.games-timeline::before {
		content: '';
		position: absolute;
		left: 8px;
		top: 0;
		bottom: 0;
		width: 2px;
		background: var(--border-soft);
	}

	.timeline-item {
		display: flex;
		gap: 16px;
		padding: 12px;
		background: var(--surface-1);
		border: 2px solid var(--border-soft);
		border-radius: 8px;
		transition: all 0.2s;
	}

	.timeline-item:hover,
	.timeline-item.selected {
		border-color: var(--accent);
		background: var(--surface-2);
	}

	.timeline-dot {
		width: 16px;
		height: 16px;
		background: var(--accent);
		border-radius: 50%;
		flex-shrink: 0;
		margin-top: 4px;
		box-shadow: 0 0 12px rgba(200, 169, 110, 0.3);
	}

	.timeline-content {
		flex: 1;
	}

	.timeline-header {
		display: flex;
		justify-content: space-between;
		align-items: center;
		margin-bottom: 8px;
	}

	.timeline-title {
		margin: 0;
		font-size: 14px;
		font-weight: 600;
		color: var(--text-primary);
	}

	.timeline-completion {
		font-size: 12px;
		color: var(--text-secondary);
	}

	.timeline-actions {
		display: flex;
		gap: 8px;
	}
</style>
