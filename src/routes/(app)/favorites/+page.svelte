<script lang="ts">
	import { games, getGameIconUrl } from '$lib/stores/Games.js';
	import { toggleGameFavorite } from '$lib/stores/selectedGame.js';
	import { i18n } from '$lib/stores/i18n.js';
	import { goto } from '$app/navigation';
	import { resolve } from '$app/paths';
	import type { Game } from '$lib/stores/Games.js';

	let favoriteGames = $derived($games.filter((g) => g.is_favorite));

	function getCompletion(game: Game): number {
		const total = game.achievements_total || game.achievements?.length || 0;
		if (total === 0) return 0;
		const unlocked = game.achievements?.filter((a) => a.unlocked).length ?? 0;
		return Math.round((unlocked / total) * 100);
	}

	function openGame(gameId: string) {
		void goto(resolve('/(app)/games/[id]', { id: gameId }));
	}

	function onIconError(name: string, id: string) {
		return (e: Event) => {
			const t = e.target as HTMLImageElement;
			t.onerror = null;
			t.src = `https://ui-avatars.com/api/?name=${encodeURIComponent(name || String(id))}&background=1e1e1e&color=c8a96e&size=150&bold=true&length=2`;
		};
	}
</script>

<main class="favorites-page">
	<header class="page-header">
		<h1 class="page-title-row">
			<svg
				width="26"
				height="26"
				viewBox="0 0 24 24"
				fill="currentColor"
				class="title-heart"
				aria-hidden="true"
			>
				<path
					d="M20.84 4.61a5.5 5.5 0 0 0-7.78 0L12 5.67l-1.06-1.06a5.5 5.5 0 0 0-7.78 7.78l1.06 1.06L12 21.23l7.78-7.78 1.06-1.06a5.5 5.5 0 0 0 0-7.78z"
				/>
			</svg>
			{$i18n.t('favorites.title')}
		</h1>
		<p class="subtitle">
			{favoriteGames.length}
			{$i18n.t('favorites.count')}
		</p>
	</header>

	{#if favoriteGames.length > 0}
		<div class="games-grid">
			{#each favoriteGames as game (game.id)}
				<div
					class="game-item"
					onclick={() => openGame(game.id)}
					onkeydown={(e) => e.key === 'Enter' && openGame(game.id)}
					role="button"
					tabindex="0"
					title={game.name}
				>
					<div class="game-icon-container">
						{#if getGameIconUrl(game)}
							<img
								src={getGameIconUrl(game)}
								alt={game.name}
								loading="lazy"
								decoding="async"
								onerror={onIconError(game.name, game.id)}
							/>
						{:else}
							<div class="game-fallback">{game.name[0]}</div>
						{/if}
						<button
							type="button"
							class="fav-badge"
							onclick={(e) => {
								e.stopPropagation();
								toggleGameFavorite(game.id);
							}}
							title={$i18n.t('games.action.removeFavorite')}
							aria-label={$i18n.t('games.action.removeFavorite')}
						>
							<svg
								width="12"
								height="12"
								viewBox="0 0 24 24"
								fill="currentColor"
								aria-hidden="true"
							>
								<path
									d="M20.84 4.61a5.5 5.5 0 0 0-7.78 0L12 5.67l-1.06-1.06a5.5 5.5 0 0 0-7.78 7.78l1.06 1.06L12 21.23l7.78-7.78 1.06-1.06a5.5 5.5 0 0 0 0-7.78z"
								/>
							</svg>
						</button>
					</div>
					<div class="game-info">
						<div class="name">{game.name}</div>
						<div class="progress-mini">
							<div
								class="bar"
								style:width={getCompletion(game) + '%'}
								style:background={getCompletion(game) === 100
									? 'var(--completed)'
									: 'var(--accent)'}
							></div>
						</div>
					</div>
				</div>
			{/each}
		</div>
	{:else}
		<div class="empty-state">
			<div class="empty-icon">
				<svg
					width="64"
					height="64"
					viewBox="0 0 24 24"
					fill="currentColor"
					aria-hidden="true"
				>
					<path
						d="M20.84 4.61a5.5 5.5 0 0 0-7.78 0L12 5.67l-1.06-1.06a5.5 5.5 0 0 0-7.78 7.78l1.06 1.06L12 21.23l7.78-7.78 1.06-1.06a5.5 5.5 0 0 0 0-7.78z"
					/>
				</svg>
			</div>
			<h2>{$i18n.t('favorites.emptyTitle')}</h2>
			<p>{$i18n.t('favorites.emptyText')}</p>
			<button class="browse-btn" onclick={() => goto(resolve('/'))}>
				{$i18n.t('favorites.browseGames')}
			</button>
		</div>
	{/if}
</main>

<style>
	.favorites-page {
		display: flex;
		flex-direction: column;
		height: 100%;
		overflow-y: auto;
		padding: 32px;
	}

	.page-header {
		margin-bottom: 32px;
	}

	.page-header h1 {
		margin: 0 0 8px;
		font-size: 28px;
		color: var(--text-primary);
	}

	.page-title-row {
		display: flex;
		align-items: center;
		gap: 12px;
	}

	.title-heart {
		color: #ff4d4d;
		flex-shrink: 0;
	}

	.subtitle {
		color: var(--text-secondary);
		font-size: 14px;
		font-variant-numeric: tabular-nums;
	}

	.games-grid {
		display: grid;
		grid-template-columns: repeat(auto-fill, minmax(150px, 1fr));
		gap: 16px;
		align-content: start;
		align-items: start;
	}

	.game-item {
		display: flex;
		flex-direction: column;
		gap: 8px;
		padding: 10px;
		background: var(--surface-1);
		border: 1px solid var(--border-soft);
		border-radius: 12px;
		cursor: pointer;
		transition: all 0.2s;
		text-align: left;
		position: relative;
	}

	.game-item:hover {
		background: var(--surface-2);
		border-color: var(--accent);
		transform: translateY(-2px);
	}

	.game-icon-container {
		position: relative;
		width: 100%;
		aspect-ratio: 1;
	}

	.game-icon-container img {
		width: 100%;
		height: 100%;
		object-fit: cover;
		border-radius: 8px;
		background: var(--surface-3);
	}

	.game-fallback {
		width: 100%;
		height: 100%;
		display: flex;
		align-items: center;
		justify-content: center;
		background: var(--surface-3);
		border-radius: 8px;
		font-size: 24px;
		font-weight: 700;
		color: var(--accent);
	}

	.fav-badge {
		position: absolute;
		top: -8px;
		right: -8px;
		width: 24px;
		height: 24px;
		background: var(--surface-2);
		border: 1px solid var(--border-soft);
		border-radius: 50%;
		display: flex;
		align-items: center;
		justify-content: center;
		font-size: 12px;
		color: #ff4d4d;
		cursor: pointer;
		box-shadow: 0 2px 8px rgba(0, 0, 0, 0.3);
		transition: all 0.2s;
	}

	.fav-badge:hover {
		transform: scale(1.1);
		background: var(--surface-3);
	}

	.game-info {
		display: flex;
		flex-direction: column;
		gap: 4px;
	}

	.name {
		font-size: 12px;
		font-weight: 600;
		color: var(--text-primary);
		white-space: nowrap;
		overflow: hidden;
		text-overflow: ellipsis;
	}

	.progress-mini {
		height: 3px;
		background: var(--surface-3);
		border-radius: 1.5px;
		overflow: hidden;
	}

	.progress-mini .bar {
		height: 100%;
		background: var(--accent);
	}

	.empty-state {
		flex: 1;
		display: flex;
		flex-direction: column;
		align-items: center;
		justify-content: center;
		text-align: center;
		color: var(--text-muted);
	}

	.empty-icon {
		font-size: 64px;
		margin-bottom: 24px;
		opacity: 0.2;
	}

	.empty-state h2 {
		margin: 0 0 12px;
		color: var(--text-primary);
	}

	.empty-state p {
		margin: 0 0 24px;
		max-width: 300px;
	}

	.browse-btn {
		padding: 10px 24px;
		background: var(--accent);
		color: var(--accent-text);
		border: none;
		border-radius: 8px;
		font-weight: 600;
		cursor: pointer;
		transition: transform 0.2s;
	}

	.browse-btn:hover {
		transform: scale(1.05);
	}
</style>
