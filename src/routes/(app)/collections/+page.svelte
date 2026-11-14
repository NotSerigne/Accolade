<script lang="ts">
	import { games, getGameCompletionPct, getGameIconUrl } from '$lib/stores/Games.js';
	import { i18n } from '$lib/stores/i18n.js';
	import { goto } from '$app/navigation';
	import { resolve } from '$app/paths';
	import type { Game } from '$lib/stores/Games.js';

	let collections = $derived.by(() => {
		const groups: Record<string, Game[]> = {};
		const uncategorized = $i18n.t('collections.uncategorized');
		$games.forEach((game) => {
			if (game.tags && game.tags.length > 0) {
				game.tags.forEach((tag) => {
					if (!groups[tag]) groups[tag] = [];
					groups[tag].push(game);
				});
			} else {
				if (!groups[uncategorized]) groups[uncategorized] = [];
				groups[uncategorized].push(game);
			}
		});
		return groups;
	});

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

<main class="collections-page">
	<header class="page-header">
		<h1 class="page-title-row">
			<svg
				width="26"
				height="26"
				viewBox="0 0 24 24"
				fill="none"
				stroke="currentColor"
				stroke-width="2"
				stroke-linecap="round"
				stroke-linejoin="round"
				aria-hidden="true"
			>
				<path d="M22 19a2 2 0 0 1-2 2H4a2 2 0 0 1-2-2V5a2 2 0 0 1 2-2h5l2 3h9a2 2 0 0 1 2 2z" />
			</svg>
			{$i18n.t('collections.title')}
		</h1>
		<p class="subtitle">
			{Object.keys(collections).filter((k) => k !== $i18n.t('collections.uncategorized')).length}
			{$i18n.t('collections.count')}
		</p>
	</header>

	<div class="collections-list">
		{#each Object.entries(collections) as [tag, games] (tag)}
			<section class="collection-section">
				<h2 class="collection-title">
					{tag} <span class="count">({games.length})</span>
				</h2>
				<div class="games-row">
					{#each games as game (game.id)}
						{@const completion = getGameCompletionPct(game)}
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
								{#if game.is_favorite}
									<div class="fav-badge">
										<svg
											width="10"
											height="10"
											viewBox="0 0 24 24"
											fill="currentColor"
											aria-hidden="true"
										>
											<path
												d="M20.84 4.61a5.5 5.5 0 0 0-7.78 0L12 5.67l-1.06-1.06a5.5 5.5 0 0 0-7.78 7.78l1.06 1.06L12 21.23l7.78-7.78 1.06-1.06a5.5 5.5 0 0 0 0-7.78z"
											/>
										</svg>
									</div>
								{/if}
							</div>
							<div class="game-info">
								<div class="name">{game.name}</div>
								<div class="progress-mini">
									<div
										class="bar"
										style:width={completion + '%'}
										style:background={completion === 100
											? 'var(--completed)'
											: 'var(--accent)'}
									></div>
								</div>
							</div>
						</div>
					{/each}
				</div>
			</section>
		{/each}
	</div>
</main>

<style>
	.collections-page {
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

	.subtitle {
		color: var(--text-secondary);
		font-size: 14px;
		font-variant-numeric: tabular-nums;
	}

	.collections-list {
		display: flex;
		flex-direction: column;
		gap: 40px;
	}

	.collection-section {
		display: flex;
		flex-direction: column;
		gap: 16px;
	}

	.collection-title {
		font-size: 18px;
		font-weight: 600;
		color: var(--accent);
		margin: 0;
		display: flex;
		align-items: center;
		gap: 12px;
	}

	.collection-title .count {
		font-size: 14px;
		color: var(--text-muted);
		font-weight: 400;
		font-variant-numeric: tabular-nums;
	}

	.games-row {
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
		top: -4px;
		right: -4px;
		width: 18px;
		height: 18px;
		background: var(--surface-2);
		border: 1px solid var(--border-soft);
		border-radius: 50%;
		display: flex;
		align-items: center;
		justify-content: center;
		font-size: 10px;
		color: #ff4d4d;
		box-shadow: 0 2px 4px rgba(0, 0, 0, 0.2);
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
</style>
