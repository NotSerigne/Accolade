<script lang="ts">
	// src/lib/StatsPanel.svelte
	import {
		games,
		computeGameProgress,
		formatPlaytime,
		getGameHeaderUrl,
		getGameIconUrl,
		getRarityCssVar,
		getRarityTier7,
		RARITY_TIERS_7,
		totalUnlockedAchievements,
		type RarityTier7
	} from '$lib/stores/Games.js';
	import { selectedGameId } from '$lib/stores/selectedGame.js';
	import { goto } from '$app/navigation';
	import { resolve } from '$app/paths';
	import { i18n, rarityLabelByIndex } from '$lib/stores/i18n.js';
	import { SvelteSet } from 'svelte/reactivity';

	let failedImageUrls = new SvelteSet<string>();

	function pickImageUrl(...candidates: Array<string | null | undefined>): string | null {
		for (const candidate of candidates) {
			const url = candidate?.trim();
			if (url && !failedImageUrls.has(url)) return url;
		}
		return null;
	}

	function openGame(id: string): void {
		selectedGameId.set(id);
		void goto(resolve('/(app)/games/[id]', { id }));
	}

	let gameProgressList = $derived(
		$games.map((game) => ({
			game,
			progress: computeGameProgress(game.achievements ?? [], game.achievement_groups)
		}))
	);

	let rarityCounts = $derived.by(() => {
		const counts: Record<RarityTier7, number> = {
			mythic: 0,
			legendary: 0,
			epic: 0,
			'very-rare': 0,
			rare: 0,
			uncommon: 0,
			common: 0
		};

		for (const { progress } of gameProgressList) {
			for (const section of progress.sections) {
				for (const ach of section.achievements) {
					if (ach.unlocked) {
						counts[getRarityTier7(ach.completionpercentage)] += 1;
					}
				}
			}
		}

		return counts;
	});

	let highRarityPct = $derived.by(() => {
		if ($totalUnlockedAchievements === 0) return 0;
		const highRarity =
			rarityCounts.mythic + rarityCounts.legendary + rarityCounts.epic + rarityCounts['very-rare'];
		return Math.round((highRarity / $totalUnlockedAchievements) * 100);
	});

	let recentlyCompleted = $derived.by(() => {
		return gameProgressList
			.filter(({ progress }) => progress.isPlatinum)
			.sort((a, b) => {
				const latestA = Math.max(
					0,
					...a.progress.sections
						.filter((s) => !s.isDlc)
						.flatMap((s) => s.achievements.map((x) => x.unlocked_time ?? 0))
				);
				const latestB = Math.max(
					0,
					...b.progress.sections
						.filter((s) => !s.isDlc)
						.flatMap((s) => s.achievements.map((x) => x.unlocked_time ?? 0))
				);
				return latestB - latestA;
			})
			.slice(0, 3);
	});
</script>

<aside class="stats-panel" aria-label={$i18n.t('statsPanel.stats')}>
	<section class="panel-section">
		<div class="rarity-header">
			<h2 class="section-title">{$i18n.t('statsPanel.rarityBreakdown')}</h2>
			<span class="rarity-share tabular-nums">
				<span class="rarity-share-pct">{highRarityPct}%</span>
				<span class="rarity-share-label">{$i18n.t('statsPanel.rareShare')}</span>
			</span>
		</div>

		<ul class="rarity-list">
			{#each RARITY_TIERS_7 as tier, idx (tier)}
				{@const count = rarityCounts[tier]}
				{@const sharePct =
					$totalUnlockedAchievements > 0 ? (count / $totalUnlockedAchievements) * 100 : 0}
				{@const colorVar = getRarityCssVar(tier)}
				<li class="rarity-row">
					<div class="rarity-row-top">
						<span class="rarity-dot" style:background={colorVar} aria-hidden="true"></span>
						<span class="rarity-label">{rarityLabelByIndex($i18n.language, idx)}</span>
						<span class="rarity-count tabular-nums">{count}</span>
					</div>
					<div class="rarity-bar-track" aria-hidden="true">
						<div
							class="rarity-bar-fill"
							style:width="{sharePct}%"
							style:background={colorVar}
						></div>
					</div>
				</li>
			{/each}
		</ul>

		<div class="rarity-summary">
			<span class="rarity-summary-label">{$i18n.t('statsPanel.cumulativeTotal')}</span>
			<span class="rarity-summary-value tabular-nums">
				{$i18n.t('statsPanel.achievementsCount', { count: $totalUnlockedAchievements })}
			</span>
		</div>
	</section>

	{#if recentlyCompleted.length > 0}
		<section class="panel-section">
			<h2 class="section-title">{$i18n.t('statsPanel.recentCompleted')}</h2>

			<ul class="completed-list">
				{#each recentlyCompleted as { game, progress } (game.id)}
					{@const iconUrl = pickImageUrl(getGameIconUrl(game), getGameHeaderUrl(game))}
					<li class="completed-item">
						<button type="button" class="completed-btn" onclick={() => openGame(game.id)}>
							<div class="game-icon-wrap">
								{#if iconUrl}
									<img
										src={iconUrl}
										alt={game.name || game.id}
										class="game-icon"
										onerror={() => failedImageUrls.add(iconUrl)}
									/>
								{:else}
									<svg
										width="16"
										height="16"
										viewBox="0 0 24 24"
										fill="none"
										stroke="currentColor"
										stroke-width="1.8"
										stroke-linecap="round"
										stroke-linejoin="round"
										aria-hidden="true"
									>
										<path d="M6 9H4.5a2.5 2.5 0 0 1 0-5H6" />
										<path d="M18 9h1.5a2.5 2.5 0 0 0 0-5H18" />
										<path d="M4 22h16" />
										<path d="M10 14.66V17c0 .55-.47.98-.97 1.21C7.85 18.75 7 20.24 7 22" />
										<path d="M14 14.66V17c0 .55.47.98.97 1.21C16.15 18.75 17 20.24 17 22" />
										<path d="M18 2H6v7a6 6 0 0 0 12 0V2Z" />
									</svg>
								{/if}
							</div>
							<div class="completed-info">
								<span class="completed-name">{game.name || game.id}</span>
								<span class="completed-sub tabular-nums">
									<span class="completed-pct"
										>100% · {progress.unlockedBase}/{progress.totalBase}</span
									>
									{#if game.playtime_seconds && game.playtime_seconds > 0}
										<span class="completed-playtime">· {formatPlaytime(game.playtime_seconds)}</span
										>
									{/if}
								</span>
							</div>
							<svg
								class="completed-chevron"
								width="14"
								height="14"
								viewBox="0 0 24 24"
								fill="none"
								stroke="currentColor"
								stroke-width="2.2"
								stroke-linecap="round"
								stroke-linejoin="round"
								aria-hidden="true"
							>
								<path d="M9 18l6-6-6-6" />
							</svg>
						</button>
					</li>
				{/each}
			</ul>
		</section>
	{/if}
</aside>

<style>
	.stats-panel {
		background: var(--bg-panel);
		border: 1px solid var(--border-soft);
		border-radius: 14px;
		box-shadow: var(--shadow-card);
		padding: 20px;
		overflow-y: auto;
		display: flex;
		flex-direction: column;
		gap: 20px;
		height: 100%;
		min-height: 0;
	}

	.panel-section {
		display: flex;
		flex-direction: column;
		gap: 10px;
	}

	.rarity-header {
		display: flex;
		align-items: baseline;
		justify-content: space-between;
		gap: 8px;
		flex-wrap: wrap;
	}

	.section-title {
		font-size: 11px;
		font-weight: 600;
		letter-spacing: 0.08em;
		text-transform: uppercase;
		color: var(--text-muted);
	}

	.rarity-share {
		display: inline-flex;
		align-items: baseline;
		gap: 4px;
		font-size: 11px;
	}

	.rarity-share-pct {
		font-weight: 700;
		color: var(--text-primary);
	}

	.rarity-share-label {
		color: var(--text-muted);
	}

	.rarity-list {
		list-style: none;
		padding: 0;
		margin: 0;
		display: flex;
		flex-direction: column;
		gap: 9px;
	}

	.rarity-row {
		display: flex;
		flex-direction: column;
		gap: 4px;
	}

	.rarity-row-top {
		display: flex;
		align-items: center;
		gap: 8px;
		font-size: 12px;
	}

	.rarity-dot {
		width: 6px;
		height: 6px;
		border-radius: 50%;
		flex-shrink: 0;
	}

	.rarity-label {
		flex: 1;
		color: var(--text-secondary);
		min-width: 0;
		white-space: nowrap;
		overflow: hidden;
		text-overflow: ellipsis;
	}

	.rarity-count {
		color: var(--text-primary);
		font-weight: 600;
	}

	.rarity-bar-track {
		height: 3px;
		border-radius: 999px;
		background: var(--surface-2);
		overflow: hidden;
	}

	.rarity-bar-fill {
		height: 100%;
		border-radius: 999px;
		transition: width 0.35s var(--spring-out);
	}

	.rarity-summary {
		display: flex;
		align-items: center;
		justify-content: space-between;
		gap: 8px;
		padding-top: 10px;
		border-top: 1px solid var(--border-soft);
		font-size: 12px;
	}

	.rarity-summary-label {
		color: var(--text-muted);
	}

	.rarity-summary-value {
		font-weight: 600;
		color: var(--text-primary);
	}

	.completed-list {
		list-style: none;
		padding: 0;
		margin: 0;
		display: flex;
		flex-direction: column;
	}

	.completed-item {
		border-bottom: 1px solid var(--border-soft);
	}

	.completed-item:last-child {
		border-bottom: none;
	}

	.completed-btn {
		display: flex;
		align-items: center;
		gap: 10px;
		width: 100%;
		padding: 8px 6px;
		border-radius: 8px;
		border: none;
		background: transparent;
		text-align: left;
		font-family: inherit;
		cursor: pointer;
		transition: background 0.16s ease;
	}

	.completed-btn:hover {
		background: var(--surface-hover);
	}

	.completed-btn:hover .completed-chevron {
		color: var(--text-primary);
		transform: translateX(2px);
	}

	.completed-btn:focus-visible {
		outline: 2px solid var(--accent);
		outline-offset: -2px;
	}

	.game-icon-wrap {
		width: 32px;
		height: 32px;
		border-radius: 6px;
		overflow: hidden;
		background: var(--surface-2);
		color: var(--text-muted);
		flex-shrink: 0;
		display: flex;
		align-items: center;
		justify-content: center;
	}

	.game-icon {
		width: 100%;
		height: 100%;
		object-fit: cover;
		display: block;
	}

	.completed-info {
		flex: 1;
		min-width: 0;
		display: flex;
		flex-direction: column;
		gap: 2px;
	}

	.completed-name {
		font-size: 13px;
		font-weight: 600;
		color: var(--text-primary);
		white-space: nowrap;
		overflow: hidden;
		text-overflow: ellipsis;
	}

	.completed-sub {
		display: flex;
		align-items: center;
		gap: 4px;
		font-size: 11px;
		white-space: nowrap;
		overflow: hidden;
		text-overflow: ellipsis;
	}

	.completed-pct {
		color: var(--completed);
		font-weight: 600;
	}

	.completed-playtime {
		color: var(--text-muted);
	}

	.completed-chevron {
		color: var(--text-muted);
		flex-shrink: 0;
		transition:
			color 0.16s ease,
			transform 0.2s var(--spring-out);
	}
</style>
