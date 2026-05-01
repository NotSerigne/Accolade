<script lang="ts">
	// src/routes/(app)/objectives/+page.svelte
	import { games, type Game } from '$lib/stores/Games.js';
	import { i18n } from '$lib/stores/i18n.js';

	let sortBy = $state('progression');

	let sortedGames = $derived.by(() => {
		const mapped = $games
			.map((g) => {
				const total = g.achievements_total || g.achievements?.length || 0;
				const unlocked = g.achievements?.filter((a) => a.unlocked).length ?? 0;
				const remaining = total - unlocked;
				const pct = total > 0 ? (unlocked / total) * 100 : 0;
				return {
					...g,
					unlockedCount: unlocked,
					totalCount: total,
					remainingCount: remaining,
					progression: pct
				};
			})
			.filter((g) => g.totalCount > 0 && g.progression < 100);

		if (sortBy === 'progression') {
			return mapped.sort((a, b) => b.progression - a.progression);
		} else {
			return mapped.sort((a, b) => a.remainingCount - b.remainingCount);
		}
	});

	let easyAchievements = $derived.by(() => {
		const locked = $games.flatMap((g) =>
			(g.achievements ?? []).filter((a) => !a.unlocked).map((a) => ({ ...a, gameName: g.name }))
		);
		return locked
			.sort((a, b) => parseFloat(b.completionpercentage) - parseFloat(a.completionpercentage))
			.slice(0, 10);
	});

	function getGameIcon(game: Game): string {
		if (game.steamgrid_icon_url && game.steamgrid_icon_url.startsWith('http')) {
			return game.steamgrid_icon_url;
		}

		if (game.game_icon && game.game_icon.startsWith('http')) {
			return game.game_icon;
		}

		if (game.header_image_url) return game.header_image_url;

		return `https://cdn.cloudflare.steamstatic.com/steam/apps/${game.steam_id}/header.jpg`;
	}
</script>

<main class="objectives-page">
	<header class="page-header">
		<div class="header-left">
			<div class="title-row">
				<div class="icon-circle">
					<svg
						width="20"
						height="20"
						viewBox="0 0 24 24"
						fill="none"
						stroke="currentColor"
						stroke-width="2.2"><circle cx="12" cy="12" r="10" /><path d="M12 8v8M8 12h8" /></svg
					>
				</div>
				<h1>{$i18n.t('objectives.title')}</h1>
			</div>
			<p class="subtitle">{$i18n.t('objectives.subtitle')}</p>
		</div>
		<div class="header-right">
			<div class="sort-toggle">
				<button class:active={sortBy === 'progression'} onclick={() => (sortBy = 'progression')}
					>{$i18n.t('objectives.sort.progression')}</button
				>
				<button class:active={sortBy === 'remaining'} onclick={() => (sortBy = 'remaining')}
					>{$i18n.t('objectives.sort.remaining')}</button
				>
			</div>
		</div>
	</header>

	<div class="content scrollable">
		<section class="easy-section">
			<div class="section-header">
				<h2>{$i18n.t('objectives.easy')}</h2>
				<span class="sub">{$i18n.t('objectives.easySubtitle')}</span>
			</div>
			<div class="easy-grid">
				{#each easyAchievements as a (a.gameName + ':' + a.key)}
					<div class="easy-card">
						<img src={a.icon_gray || a.icon} alt={a.name} />
						<div class="info">
							<span class="name">{a.name}</span>
							<span class="game">{a.gameName}</span>
						</div>
						<span class="pct">{parseFloat(a.completionpercentage).toFixed(1)}%</span>
					</div>
				{/each}
			</div>
		</section>

		<section class="games-section">
			<div class="games-list">
				{#each sortedGames as game, i (game.steam_id)}
					<div class="game-objective-card">
						<span class="rank">{i + 1}</span>
						<img
							src={getGameIcon(game)}
							alt={game.name}
							class="game-thumb"
							onerror={(e) => {
								const t = e.target as HTMLImageElement;
								if (!t.src.includes('header.jpg')) {
									t.src = `https://cdn.cloudflare.steamstatic.com/steam/apps/${game.steam_id}/header.jpg`;
								}
							}}
						/>
						<div class="game-info">
							<h3>{game.name}</h3>
							<div class="progress-details">
								<span class="counts"
									>{$i18n.t('objectives.remaining', {
										unlocked: game.unlockedCount,
										total: game.totalCount,
										remaining: game.remainingCount
									})}</span
								>
							</div>
						</div>
						<div class="progress-bar-container">
							<div class="progress-bar">
								<div class="progress-fill" style="width: {game.progression}%"></div>
							</div>
							<span class="pct">{game.progression.toFixed(2)}%</span>
						</div>
						<button class="arrow-btn" aria-label={$i18n.t('objectives.details')}>
							<svg
								width="16"
								height="16"
								viewBox="0 0 24 24"
								fill="none"
								stroke="currentColor"
								stroke-width="2.5"><path d="M9 18l6-6-6-6" /></svg
							>
						</button>
					</div>
				{/each}
			</div>
		</section>
	</div>
</main>

<style>
	.objectives-page {
		grid-column: 2 / -1;
		grid-row: 2 / -1;
		display: flex;
		flex-direction: column;
		background: var(--bg-panel);
		color: var(--text-primary);
		height: 100%;
		overflow: hidden;
	}

	.page-header {
		padding: 32px;
		display: flex;
		justify-content: space-between;
		align-items: center;
	}

	.title-row {
		display: flex;
		align-items: center;
		gap: 12px;
		margin-bottom: 8px;
	}
	.icon-circle {
		width: 36px;
		height: 36px;
		border-radius: 50%;
		background: rgba(200, 169, 110, 0.1);
		color: var(--accent, #c8a96e);
		display: flex;
		align-items: center;
		justify-content: center;
	}
	.page-header h1 {
		font-size: 28px;
		font-weight: 700;
		margin: 0;
		color: var(--text-primary);
	}
	.subtitle {
		font-size: 14px;
		color: var(--text-muted);
		margin: 0;
	}

	.sort-toggle {
		display: flex;
		background: var(--surface-2);
		padding: 4px;
		border-radius: 8px;
	}
	.sort-toggle button {
		background: transparent;
		border: none;
		color: var(--text-muted);
		padding: 6px 16px;
		border-radius: 6px;
		font-size: 13px;
		cursor: pointer;
		transition: all 0.2s;
	}
	.sort-toggle button.active {
		background: rgba(200, 169, 110, 0.1);
		color: var(--accent, #c8a96e);
	}

	.content {
		flex: 1;
		overflow-y: auto;
		padding: 0 32px 32px;
	}

	.section-header {
		display: flex;
		align-items: center;
		gap: 12px;
		margin-bottom: 24px;
	}
	.section-header h2 {
		font-size: 18px;
		font-weight: 600;
		margin: 0;
		color: var(--text-primary);
	}
	.section-header .sub {
		font-size: 12px;
		color: var(--text-muted);
		margin-left: auto;
	}

	.easy-grid {
		display: grid;
		grid-template-columns: 1fr 1fr;
		gap: 12px;
		margin-bottom: 40px;
	}
	.easy-card {
		background: var(--surface-1);
		border: 1px solid var(--border-soft);
		border-radius: 12px;
		padding: 12px;
		display: flex;
		align-items: center;
		gap: 12px;
	}
	.easy-card img {
		width: 44px;
		height: 44px;
		border-radius: 8px;
		filter: grayscale(1);
		opacity: 0.6;
	}
	.easy-card .info {
		flex: 1;
		display: flex;
		flex-direction: column;
		min-width: 0;
	}
	.easy-card .name {
		font-size: 14px;
		font-weight: 600;
		color: var(--text-primary);
		white-space: nowrap;
		overflow: hidden;
		text-overflow: ellipsis;
	}
	.easy-card .game {
		font-size: 11px;
		color: var(--text-secondary);
		white-space: nowrap;
		overflow: hidden;
		text-overflow: ellipsis;
	}
	.easy-card .pct {
		font-size: 12px;
		color: var(--text-secondary);
		background: var(--surface-2);
		padding: 2px 8px;
		border-radius: 4px;
	}

	.games-list {
		display: flex;
		flex-direction: column;
		gap: 12px;
	}
	.game-objective-card {
		background: var(--surface-1);
		border: 1px solid var(--border-soft);
		border-radius: 16px;
		padding: 16px 24px;
		display: flex;
		align-items: center;
		gap: 20px;
		transition: background 0.2s;
	}
	.game-objective-card:hover {
		background: var(--surface-hover);
	}

	.rank {
		font-size: 12px;
		color: var(--text-muted);
		width: 20px;
	}
	.game-thumb {
		width: 40px;
		height: 40px;
		border-radius: 8px;
		object-fit: cover;
	}
	.game-info {
		flex: 1;
		min-width: 0;
	}
	.game-info h3 {
		font-size: 16px;
		font-weight: 600;
		margin: 0 0 4px 0;
		color: var(--text-primary);
		white-space: nowrap;
		overflow: hidden;
		text-overflow: ellipsis;
	}
	.progress-details {
		font-size: 12px;
		color: var(--text-secondary);
	}

	.progress-bar-container {
		width: 300px;
		display: flex;
		flex-direction: column;
		gap: 8px;
	}
	.progress-bar {
		height: 6px;
		background: var(--surface-2);
		border-radius: 3px;
		overflow: hidden;
	}
	.progress-fill {
		height: 100%;
		background: var(--accent, #c8a96e);
		border-radius: 3px;
	}
	.progress-bar-container .pct {
		font-size: 12px;
		color: var(--accent, #c8a96e);
		font-weight: 700;
		align-self: flex-end;
	}

	.arrow-btn {
		background: var(--surface-2);
		border: none;
		color: var(--text-muted);
		width: 32px;
		height: 32px;
		border-radius: 8px;
		display: flex;
		align-items: center;
		justify-content: center;
		cursor: pointer;
		transition: all 0.2s;
	}
	.arrow-btn:hover {
		background: var(--surface-hover);
		color: var(--text-primary);
	}

	.scrollable::-webkit-scrollbar {
		width: 6px;
	}
	.scrollable::-webkit-scrollbar-track {
		background: transparent;
	}
	.scrollable::-webkit-scrollbar-thumb {
		background: rgba(255, 255, 255, 0.1);
		border-radius: 3px;
	}
</style>
