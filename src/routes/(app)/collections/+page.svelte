<script lang="ts">
	import { games } from '$lib/stores/Games.js';
	import { i18n } from '$lib/stores/i18n.js';
	import { goto } from '$app/navigation';
	import { resolve } from '$app/paths';
	import type { Game } from '$lib/stores/Games.js';

	let collections = $derived.by(() => {
		const groups: Record<string, Game[]> = {};
		$games.forEach((game) => {
			if (game.tags && game.tags.length > 0) {
				game.tags.forEach((tag) => {
					if (!groups[tag]) groups[tag] = [];
					groups[tag].push(game);
				});
			} else {
				if (!groups['Sans collection']) groups['Sans collection'] = [];
				groups['Sans collection'].push(game);
			}
		});
		return groups;
	});

	function getCompletion(game: Game): number {
		const total = game.achievements_total || game.achievements?.length || 0;
		if (total === 0) return 0;
		const unlocked = game.achievements?.filter((a) => a.unlocked).length ?? 0;
		return Math.round((unlocked / total) * 100);
	}

	function openGame(gameId: string) {
		void goto(resolve('/(app)/games/[id]', { id: gameId }));
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
		<h1>📁 {$i18n.t('collections.title') || 'Mes Collections'}</h1>
		<p class="subtitle">
			{Object.keys(collections).filter((k) => k !== 'Sans collection').length}
			{$i18n.t('collections.count') || 'collections personnalisées'}
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
						<div
							class="game-item"
							onclick={() => openGame(game.id)}
							onkeydown={(e) => e.key === 'Enter' && openGame(game.id)}
							role="button"
							tabindex="0"
							title={game.name}
						>
							<div class="game-icon-container">
								{#if iconUrl(game)}
									<img
										src={iconUrl(game)}
										alt={game.name}
										onerror={onIconError(game.name, game.id)}
									/>
								{:else}
									<div class="game-fallback">{game.name[0]}</div>
								{/if}
								{#if game.is_favorite}
									<div class="fav-badge">♥️</div>
								{/if}
							</div>
							<div class="game-info">
								<div class="name">{game.name}</div>
								<div class="progress-mini">
									<div class="bar" style:width={getCompletion(game) + '%'}></div>
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

	.subtitle {
		color: var(--text-secondary);
		font-size: 14px;
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
	}

	.games-row {
		display: grid;
		grid-template-columns: repeat(auto-fill, minmax(150px, 1fr));
		gap: 16px;
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
