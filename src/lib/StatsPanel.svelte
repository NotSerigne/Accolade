<script lang="ts">
	// src/lib/StatsPanel.svelte
	import {
		games,
		computeGameProgress,
		getGameCompletionPct,
		getGameHeaderUrl,
		getGameIconUrl,
		totalUnlockedAchievements
	} from '$lib/stores/Games.js';
	import { goto } from '$app/navigation';
	import { resolve } from '$app/paths';
	import { i18n, rarityLabelByIndex } from '$lib/stores/i18n.js';

	type RarityBreakdown = {
		mythic: number;
		legendary: number;
		epic: number;
		veryRare: number;
		rare: number;
		uncommon: number;
		common: number;
	};

	function openGame(id: string) {
		void goto(resolve('/(app)/games/[id]', { id }));
	}

	let gameProgressList = $derived(
		$games.map((game) => ({
			game,
			progress: computeGameProgress(game.achievements ?? [], game.achievement_groups)
		}))
	);

	let totalAchievements = $derived(
		gameProgressList.reduce(
			(acc, { game, progress }) =>
				acc + (game.achievements?.length ? progress.totalAll : game.achievements_total || 0),
			0
		)
	);

	let avgProgression = $derived.by(() => {
		if ($games.length === 0) return 0;
		const sum = $games.reduce((acc, g) => acc + getGameCompletionPct(g), 0);
		return Math.round(sum / $games.length);
	});

	let rarityBreakdown = $derived.by(() => {
		const all = gameProgressList
			.flatMap(({ progress }) => progress.sections.flatMap((s) => s.achievements))
			.filter((a) => a.unlocked);
		const breakdown: RarityBreakdown = {
			mythic: 0,
			legendary: 0,
			epic: 0,
			veryRare: 0,
			rare: 0,
			uncommon: 0,
			common: 0
		};

		for (const a of all) {
			const n = parseFloat(a.completionpercentage);
			if (isNaN(n)) {
				breakdown.common += 1;
			} else if (n <= 0.1) {
				breakdown.mythic += 1;
			} else if (n <= 1) {
				breakdown.legendary += 1;
			} else if (n <= 3) {
				breakdown.epic += 1;
			} else if (n <= 7) {
				breakdown.veryRare += 1;
			} else if (n <= 15) {
				breakdown.rare += 1;
			} else if (n <= 35) {
				breakdown.uncommon += 1;
			} else {
				breakdown.common += 1;
			}
		}

		return breakdown;
	});

	let highRarityPct = $derived.by(() => {
		if ($totalUnlockedAchievements === 0) return 0;
		const highRarity =
			rarityBreakdown.mythic +
			rarityBreakdown.legendary +
			rarityBreakdown.epic +
			rarityBreakdown.veryRare;
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

	let rarityRows = $derived.by(() => [
		{
			label: rarityLabelByIndex($i18n.language, 0),
			count: rarityBreakdown.mythic,
			className: 'mythic'
		},
		{
			label: rarityLabelByIndex($i18n.language, 1),
			count: rarityBreakdown.legendary,
			className: 'legendary'
		},
		{
			label: rarityLabelByIndex($i18n.language, 2),
			count: rarityBreakdown.epic,
			className: 'epic'
		},
		{
			label: rarityLabelByIndex($i18n.language, 3),
			count: rarityBreakdown.veryRare,
			className: 'very-rare'
		},
		{
			label: rarityLabelByIndex($i18n.language, 4),
			count: rarityBreakdown.rare,
			className: 'rare'
		},
		{
			label: rarityLabelByIndex($i18n.language, 5),
			count: rarityBreakdown.uncommon,
			className: 'uncommon'
		},
		{
			label: rarityLabelByIndex($i18n.language, 6),
			count: rarityBreakdown.common,
			className: 'common'
		}
	]);
</script>

<div class="stats-panel">
	<h2 class="section-title">{$i18n.t('statsPanel.stats')}</h2>

	<div class="stat-card">
		<span class="stat-label">{$i18n.t('statsPanel.unlocked')}</span>
		<span class="stat-value tabular-nums">{$totalUnlockedAchievements}</span>
		<span class="stat-sub tabular-nums">{$i18n.t('statsPanel.outOf', { total: totalAchievements })}</span>
	</div>

	<div class="stat-card">
		<span class="stat-label">{$i18n.t('statsPanel.avgProgress')}</span>
		<span class="stat-value tabular-nums">{avgProgression}%</span>
	</div>

	<div class="stat-card">
		<span class="stat-label">{$i18n.t('statsPanel.avgRarity')}</span>
		<span class="stat-value tabular-nums">{highRarityPct}%</span>
	</div>

	<h2 class="section-title">{$i18n.t('statsPanel.rarityBreakdown')}</h2>

	<ul class="rarity-list">
		{#each rarityRows as row (row.className)}
			{@const sharePct =
				$totalUnlockedAchievements > 0 ? (row.count / $totalUnlockedAchievements) * 100 : 0}
			<li class="rarity-row">
				<div class="rarity-row-top">
					<span class={'dot ' + row.className}></span>
					<span class="rarity-label">{row.label}</span>
					<span class="rarity-count tabular-nums">{row.count}</span>
				</div>
				<div class="rarity-bar-track" aria-hidden="true">
					<div class={'rarity-bar-fill ' + row.className} style:width="{sharePct}%"></div>
				</div>
			</li>
		{/each}
	</ul>

	<h2 class="section-title">{$i18n.t('statsPanel.recentCompleted')}</h2>

	<ul class="completed-list">
		{#each recentlyCompleted as { game, progress } (game.id)}
			<li class="completed-item">
				<button type="button" class="completed-btn" onclick={() => openGame(game.id)}>
					<img
						src={getGameIconUrl(game)}
						alt={game.name}
						class="game-icon"
						onerror={(e) => {
							const t = e.target as HTMLImageElement;
							const headerUrl = getGameHeaderUrl(game);
							if (headerUrl && t.src !== headerUrl) {
								t.src = headerUrl;
							}
						}}
					/>
					<div class="completed-info">
						<span class="completed-name">{game.name}</span>
						<span class="completed-sub tabular-nums">
							100% · {progress.unlockedBase}/{progress.totalBase}
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
</div>

<style>
	.stats-panel {
		background: var(--bg-panel);
		border: 1px solid var(--border-soft);
		border-radius: 14px;
		padding: 24px;
		overflow-y: auto;
		display: flex;
		flex-direction: column;
		gap: 8px;
		height: 100%;
		box-shadow: var(--shadow-card);
	}

	.section-title {
		font-size: 11px;
		font-weight: 600;
		letter-spacing: 0.08em;
		text-transform: uppercase;
		color: var(--text-muted);
		margin-top: 16px;
		margin-bottom: 4px;
	}

	.section-title:first-child {
		margin-top: 0;
	}

	.stat-card {
		background: var(--surface-1);
		border-radius: 10px;
		border: 1px solid var(--border-soft);
		padding: 14px 16px;
		display: flex;
		flex-direction: column;
		gap: 4px;
		box-shadow: var(--shadow-card);
	}

	.stat-label {
		font-size: 11px;
		font-weight: 600;
		letter-spacing: 0.08em;
		text-transform: uppercase;
		color: var(--text-muted);
	}

	.stat-value {
		font-size: 30px;
		font-weight: 700;
		color: var(--accent, #c8a96e);
		line-height: 1.05;
		font-variant-numeric: tabular-nums;
	}

	.stat-sub {
		font-size: 12px;
		color: var(--text-muted);
		margin-top: 2px;
		font-variant-numeric: tabular-nums;
	}

	.rarity-list {
		list-style: none;
		padding: 0;
		margin: 0;
		display: flex;
		flex-direction: column;
		gap: 10px;
	}

	.rarity-row {
		display: flex;
		flex-direction: column;
		gap: 5px;
	}

	.rarity-row-top {
		display: flex;
		align-items: center;
		gap: 10px;
		font-size: 13px;
	}

	.dot {
		width: 8px;
		height: 8px;
		border-radius: 50%;
		flex-shrink: 0;
	}

	.dot.mythic,
	.rarity-bar-fill.mythic {
		background: #ff3b5c;
	}
	.dot.legendary,
	.rarity-bar-fill.legendary {
		background: #ffd85a;
	}
	.dot.epic,
	.rarity-bar-fill.epic {
		background: #a855f7;
	}
	.dot.very-rare,
	.rarity-bar-fill.very-rare {
		background: #f4b860;
	}
	.dot.rare,
	.rarity-bar-fill.rare {
		background: #4ac8ff;
	}
	.dot.uncommon,
	.rarity-bar-fill.uncommon {
		background: #3ddc84;
	}
	.dot.common,
	.rarity-bar-fill.common {
		background: var(--text-muted);
	}

	.rarity-label {
		flex: 1;
		color: var(--text-secondary);
	}
	.rarity-count {
		color: var(--text-primary);
		font-weight: 600;
		font-variant-numeric: tabular-nums;
	}

	.rarity-bar-track {
		height: 4px;
		border-radius: 999px;
		background: var(--surface-2);
		overflow: hidden;
	}

	.rarity-bar-fill {
		height: 100%;
		border-radius: 999px;
		transition: width 0.35s var(--spring-out);
	}

	.completed-list {
		list-style: none;
		padding: 0;
		margin: 0;
		display: flex;
		flex-direction: column;
		gap: 4px;
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
		gap: 12px;
		width: 100%;
		padding: 8px 6px;
		border-radius: 8px;
		border: none;
		background: transparent;
		text-align: left;
		font-family: inherit;
		cursor: pointer;
		transition:
			background 0.18s ease,
			transform 0.2s var(--spring-out);
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
		outline-offset: 1px;
	}

	.game-icon {
		width: 36px;
		height: 36px;
		border-radius: 6px;
		object-fit: cover;
		background: var(--surface-2);
		flex-shrink: 0;
	}

	.completed-info {
		flex: 1;
		min-width: 0;
		display: flex;
		flex-direction: column;
		gap: 2px;
		overflow: hidden;
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
		font-size: 11px;
		color: var(--completed, #4caf6e);
		font-weight: 600;
		font-variant-numeric: tabular-nums;
	}

	.completed-chevron {
		color: var(--text-muted);
		flex-shrink: 0;
		transition:
			color 0.18s ease,
			transform 0.2s var(--spring-out);
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
