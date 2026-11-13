<script lang="ts">
	// src/lib/MainContent.svelte
	import {
		games,
		getGameCompletionPct,
		getGameHeaderUrl,
		getRarityPalette,
		type Game,
		type Achievement
	} from '$lib/stores/Games.js';
	import { goto } from '$app/navigation';
	import { resolve } from '$app/paths';
	import { i18n, rarityLabelByIndex } from '$lib/stores/i18n.js';

	function rarityLabel(pct: string): string {
		const n = parseFloat(pct);
		if (isNaN(n)) return rarityLabelByIndex($i18n.language, 6);
		if (n <= 0.1) return rarityLabelByIndex($i18n.language, 0);
		if (n <= 1) return rarityLabelByIndex($i18n.language, 1);
		if (n <= 3) return rarityLabelByIndex($i18n.language, 2);
		if (n <= 7) return rarityLabelByIndex($i18n.language, 3);
		if (n <= 15) return rarityLabelByIndex($i18n.language, 4);
		if (n <= 35) return rarityLabelByIndex($i18n.language, 5);
		return rarityLabelByIndex($i18n.language, 6);
	}

	function formatDateTime(ts: number | null | undefined): string {
		if (!ts) return '';
		return new Date(ts * 1000).toLocaleString($i18n.locale, {
			day: '2-digit',
			month: '2-digit',
			year: 'numeric',
			hour: '2-digit',
			minute: '2-digit',
			hour12: false
		});
	}

	type RecentAchievement = Achievement & {
		gameName: string;
		gameId: string;
		gameHeaderImage: string;
	};

	let recentActivity = $derived.by((): RecentAchievement[] => {
		return $games
			.flatMap((g) => {
				const headerUrl = getGameHeaderUrl(g);
				return (g.achievements ?? [])
					.filter((a) => a.unlocked && a.unlocked_time)
					.map((a) => ({
						...a,
						gameName: g.name,
						gameId: g.id,
						gameHeaderImage: headerUrl
					}));
			})
			.sort((a, b) => (b.unlocked_time ?? 0) - (a.unlocked_time ?? 0))
			.slice(0, 6);
	});

	let recentGames = $derived.by((): Game[] => {
		return [...$games]
			.filter((g) => (g.achievements ?? []).some((a) => a.unlocked || a.unlocked_time))
			.sort((a, b) => {
				const lastUnlockA = Math.max(...(a.achievements ?? []).map((x) => x.unlocked_time ?? 0), 0);
				const lastUnlockB = Math.max(...(b.achievements ?? []).map((x) => x.unlocked_time ?? 0), 0);
				return lastUnlockB - lastUnlockA;
			})
			.slice(0, 12);
	});
</script>

<main class="main-content">
	<section class="activity-section">
		<h2 class="section-title">{$i18n.t('main.recentActivity')}</h2>
		<div class="activity-grid">
			{#each recentActivity as item (item.gameId + ':' + item.key)}
				{@const palette = getRarityPalette(item.completionpercentage)}
				<button
					type="button"
					class="activity-card"
					onclick={() => void goto(resolve('/(app)/games/[id]', { id: item.gameId }))}
				>
					<div class="activity-icon-wrap">
						{#if item.icon}
							<img
								src={item.icon}
								alt={item.name}
								class="activity-icon"
								onerror={(e) => {
									const t = e.target as HTMLImageElement;
									if (item.gameHeaderImage && t.src !== item.gameHeaderImage) {
										t.src = item.gameHeaderImage;
									} else {
										t.style.display = 'none';
										t.parentElement?.classList.add('icon-error');
									}
								}}
							/>
							<span class="activity-icon-fallback">
								<svg width="22" height="22" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.8" stroke-linecap="round" stroke-linejoin="round" aria-hidden="true">
									<path d="M6 9H4.5a2.5 2.5 0 0 1 0-5H6" />
									<path d="M18 9h1.5a2.5 2.5 0 0 0 0-5H18" />
									<path d="M4 22h16" />
									<path d="M10 14.66V17c0 .55-.47.98-.97 1.21C7.85 18.75 7 20.24 7 22" />
									<path d="M14 14.66V17c0 .55.47.98.97 1.21C16.15 18.75 17 20.24 17 22" />
									<path d="M18 2H6v7a6 6 0 0 0 12 0V2Z" />
								</svg>
							</span>
						{:else if item.gameHeaderImage}
							<img
								src={item.gameHeaderImage}
								alt={item.name}
								class="activity-icon"
								onerror={(e) => {
									const t = e.target as HTMLImageElement;
									t.style.display = 'none';
									t.parentElement?.classList.add('icon-error');
								}}
							/>
							<span class="activity-icon-fallback">
								<svg width="22" height="22" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.8" stroke-linecap="round" stroke-linejoin="round" aria-hidden="true">
									<path d="M6 9H4.5a2.5 2.5 0 0 1 0-5H6" />
									<path d="M18 9h1.5a2.5 2.5 0 0 0 0-5H18" />
									<path d="M4 22h16" />
									<path d="M10 14.66V17c0 .55-.47.98-.97 1.21C7.85 18.75 7 20.24 7 22" />
									<path d="M14 14.66V17c0 .55.47.98.97 1.21C16.15 18.75 17 20.24 17 22" />
									<path d="M18 2H6v7a6 6 0 0 0 12 0V2Z" />
								</svg>
							</span>
						{:else}
							<span class="activity-icon activity-icon-fallback always-visible">
								<svg width="22" height="22" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.8" stroke-linecap="round" stroke-linejoin="round" aria-hidden="true">
									<path d="M6 9H4.5a2.5 2.5 0 0 1 0-5H6" />
									<path d="M18 9h1.5a2.5 2.5 0 0 0 0-5H18" />
									<path d="M4 22h16" />
									<path d="M10 14.66V17c0 .55-.47.98-.97 1.21C7.85 18.75 7 20.24 7 22" />
									<path d="M14 14.66V17c0 .55.47.98.97 1.21C16.15 18.75 17 20.24 17 22" />
									<path d="M18 2H6v7a6 6 0 0 0 12 0V2Z" />
								</svg>
							</span>
						{/if}
					</div>
					<div class="activity-info">
						<div class="activity-header">
							<span class="activity-name">{item.name}</span>
						</div>
						<div class="activity-game">{item.gameName}</div>
						<div class="activity-footer">
							<span
								class="activity-rarity-badge tabular-nums"
								style:color={palette.color}
								style:background={palette.bg}
								style:border-color={palette.border}
							>
								{rarityLabel(item.completionpercentage)} · {parseFloat(
									item.completionpercentage
								).toFixed(1)}%
							</span>
							<span class="activity-time tabular-nums">{formatDateTime(item.unlocked_time)}</span>
						</div>
					</div>
				</button>
			{/each}
		</div>

		{#if recentActivity.length === 0}
			<div class="empty-hint">{$i18n.t('main.noRecentActivity')}</div>
		{/if}
	</section>

	<section class="activity-section library-section">
		<div class="section-header">
			<h2 class="section-title">{$i18n.t('main.recentLibrary')}</h2>
			<button
				type="button"
				class="library-link-btn"
				onclick={() => void goto(resolve('/(app)/games'))}
			>
				<svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round" aria-hidden="true">
					<rect x="3" y="3" width="7" height="7" rx="1.5" />
					<rect x="14" y="3" width="7" height="7" rx="1.5" />
					<rect x="3" y="14" width="7" height="7" rx="1.5" />
					<rect x="14" y="14" width="7" height="7" rx="1.5" />
				</svg>
				<span>{$i18n.t('topbar.library')}</span>
				<svg width="13" height="13" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2.2" stroke-linecap="round" stroke-linejoin="round" aria-hidden="true">
					<path d="M9 18l6-6-6-6" />
				</svg>
			</button>
		</div>
		<div class="game-grid">
			{#each recentGames as game (game.id)}
				{@const headerUrl = getGameHeaderUrl(game)}
				{@const pct = getGameCompletionPct(game)}
				<button
					type="button"
					class="game-card"
					onclick={() => void goto(resolve('/(app)/games/[id]', { id: game.id }))}
				>
					<div class="game-card-bg">
						{#if headerUrl}
							<img src={headerUrl} alt={game.name} />
						{:else}
							<svg width="28" height="28" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.6" stroke-linecap="round" stroke-linejoin="round" class="game-card-fallback-icon" aria-hidden="true">
								<path d="M6 9H4.5a2.5 2.5 0 0 1 0-5H6" />
								<path d="M18 9h1.5a2.5 2.5 0 0 0 0-5H18" />
								<path d="M4 22h16" />
								<path d="M10 14.66V17c0 .55-.47.98-.97 1.21C7.85 18.75 7 20.24 7 22" />
								<path d="M14 14.66V17c0 .55.47.98.97 1.21C16.15 18.75 17 20.24 17 22" />
								<path d="M18 2H6v7a6 6 0 0 0 12 0V2Z" />
							</svg>
						{/if}
					</div>
					<div class="game-card-overlay"></div>
					<div class="game-card-info">
						<div class="game-card-title">{game.name || game.id}</div>
						<div class="game-card-bar">
							<div
								class="game-card-bar-fill"
								style="width: {pct}%; background: {pct === 100
									? 'var(--completed)'
									: 'var(--accent)'}"
							></div>
						</div>
						<div
							class="game-card-pct tabular-nums"
							class:complete={pct === 100}
						>
							{pct}%
						</div>
					</div>
				</button>
			{/each}
		</div>
	</section>
</main>

<style>
	.main-content {
		background: var(--bg-panel);
		padding: 28px 32px;
		overflow-y: auto;
		height: 100%;
		display: flex;
		flex-direction: column;
	}

	.activity-section {
		display: flex;
		flex-direction: column;
	}

	.library-section {
		margin-top: 32px;
		flex: 1;
	}

	.section-header {
		display: flex;
		align-items: center;
		justify-content: space-between;
		gap: 12px;
		margin-bottom: 18px;
	}

	.section-header .section-title {
		margin-bottom: 0;
	}

	.section-title {
		font-size: 20px;
		font-weight: 800;
		margin-bottom: 18px;
		letter-spacing: -0.5px;
		color: var(--text-primary);
	}

	.library-link-btn {
		display: inline-flex;
		align-items: center;
		gap: 6px;
		padding: 6px 12px;
		border-radius: 999px;
		background: var(--surface-1);
		border: 1px solid var(--border-soft);
		color: var(--text-secondary);
		font-size: 12px;
		font-weight: 600;
		font-family: inherit;
		cursor: pointer;
		box-shadow: var(--shadow-card);
		transition:
			transform 0.2s var(--spring-out),
			background 0.18s ease,
			border-color 0.18s ease,
			color 0.18s ease;
	}

	.library-link-btn:hover {
		background: var(--surface-hover);
		border-color: var(--border-specular);
		color: var(--text-primary);
		transform: translateY(-1px);
	}

	.library-link-btn:focus-visible {
		outline: 2px solid var(--accent);
		outline-offset: 2px;
	}

	.activity-grid {
		display: grid;
		grid-template-columns: repeat(auto-fill, minmax(340px, 1fr));
		gap: 12px;
	}

	.activity-card {
		display: flex;
		align-items: center;
		gap: 14px;
		padding: 12px;
		background: var(--surface-1);
		border: 1px solid var(--border-soft);
		border-radius: 12px;
		cursor: pointer;
		box-shadow: var(--shadow-card);
		transition:
			transform 0.22s var(--spring-out),
			background 0.18s ease,
			border-color 0.18s ease,
			box-shadow 0.22s ease;
		text-align: left;
		width: 100%;
		font-family: inherit;
	}

	.activity-card:hover {
		background: var(--surface-hover);
		border-color: var(--border-specular);
		box-shadow: var(--shadow-elevated);
		transform: translateY(-2px);
	}

	.activity-card:focus-visible {
		outline: 2px solid var(--accent);
		outline-offset: 2px;
	}

	.activity-icon-wrap {
		flex-shrink: 0;
		display: flex;
		align-items: center;
		justify-content: center;
		width: 56px;
		height: 56px;
		position: relative;
	}

	.activity-icon {
		width: 56px;
		height: 56px;
		border-radius: 8px;
		object-fit: cover;
		background: var(--surface-3);
	}

	.activity-icon-fallback {
		display: none;
		align-items: center;
		justify-content: center;
		width: 56px;
		height: 56px;
		border-radius: 8px;
		background: var(--surface-3);
		color: var(--text-muted);
	}

	:global(.activity-icon-wrap.icon-error .activity-icon-fallback) {
		display: flex;
	}

	.activity-icon-fallback.always-visible {
		display: flex;
	}

	.activity-info {
		flex: 1;
		min-width: 0;
		display: flex;
		flex-direction: column;
		justify-content: space-between;
	}

	.activity-header {
		display: flex;
		justify-content: space-between;
		align-items: flex-start;
		margin-bottom: 2px;
	}

	.activity-name {
		font-size: 14px;
		font-weight: 700;
		color: var(--text-primary);
		white-space: nowrap;
		overflow: hidden;
		text-overflow: ellipsis;
	}

	.activity-game {
		font-size: 12px;
		font-weight: 500;
		color: var(--text-secondary);
		white-space: nowrap;
		overflow: hidden;
		text-overflow: ellipsis;
		margin-bottom: 6px;
	}

	.activity-footer {
		display: flex;
		justify-content: space-between;
		align-items: center;
		gap: 8px;
	}

	.activity-rarity-badge {
		font-size: 10px;
		font-weight: 800;
		padding: 2px 8px;
		border-radius: 5px;
		border: 1px solid transparent;
		text-transform: uppercase;
		letter-spacing: 0.4px;
		font-variant-numeric: tabular-nums;
	}

	.activity-time {
		font-size: 10px;
		font-weight: 600;
		color: var(--text-muted);
		font-variant-numeric: tabular-nums;
	}

	.empty-hint {
		font-size: 13px;
		color: var(--text-muted);
		padding: 16px 0;
	}

	.game-grid {
		display: grid;
		grid-template-columns: repeat(auto-fill, minmax(200px, 1fr));
		gap: 12px;
	}

	.game-card {
		position: relative;
		height: 120px;
		border-radius: 12px;
		overflow: hidden;
		border: 1px solid var(--border-soft);
		cursor: pointer;
		background: var(--surface-3);
		padding: 0;
		box-shadow: var(--shadow-card);
		transition:
			transform 0.22s var(--spring-out),
			border-color 0.18s ease,
			box-shadow 0.22s ease;
		text-align: left;
	}

	.game-card:hover {
		border-color: var(--accent, #c8a96e);
		box-shadow: var(--shadow-elevated);
		transform: translateY(-2px);
	}

	.game-card:focus-visible {
		outline: 2px solid var(--accent);
		outline-offset: 2px;
	}

	.game-card-bg {
		position: absolute;
		inset: 0;
		display: flex;
		align-items: center;
		justify-content: center;
		background: var(--surface-2);
	}
	.game-card-bg img {
		width: 100%;
		height: 100%;
		object-fit: cover;
	}
	.game-card-fallback-icon {
		color: var(--text-muted);
		opacity: 0.55;
	}

	.game-card-overlay {
		position: absolute;
		inset: 0;
		background: linear-gradient(
			to top,
			rgba(10, 12, 18, 0.92) 0%,
			rgba(10, 12, 18, 0.45) 58%,
			rgba(10, 12, 18, 0.08) 100%
		);
	}

	.game-card-info {
		position: absolute;
		bottom: 0;
		left: 0;
		right: 0;
		padding: 10px 12px;
	}

	.game-card-title {
		font-size: 12px;
		font-weight: 700;
		color: #ffffff;
		text-shadow: 0 1px 2px rgba(0, 0, 0, 0.45);
		white-space: nowrap;
		overflow: hidden;
		text-overflow: ellipsis;
		margin-bottom: 6px;
	}

	.game-card-bar {
		height: 3px;
		background: rgba(255, 255, 255, 0.22);
		border-radius: 2px;
		overflow: hidden;
		margin-bottom: 4px;
	}

	.game-card-bar-fill {
		height: 100%;
		background: var(--accent, #c8a96e);
		border-radius: 2px;
	}

	.game-card-pct {
		font-size: 10px;
		font-weight: 800;
		color: rgba(255, 255, 255, 0.8);
		font-variant-numeric: tabular-nums;
	}
	.game-card-pct.complete {
		color: var(--completed, #e4b441);
	}

	::-webkit-scrollbar {
		width: 4px;
	}
	::-webkit-scrollbar-track {
		background: transparent;
	}
	::-webkit-scrollbar-thumb {
		background: var(--border-soft);
		border-radius: 2px;
	}
	::-webkit-scrollbar-thumb:hover {
		background: var(--text-muted);
	}
</style>
