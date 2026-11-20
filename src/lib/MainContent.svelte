<script lang="ts">
	// src/lib/MainContent.svelte
	import {
		games,
		computeGameProgress,
		formatPlaytime,
		getGameCompletionPct,
		getGameHeaderUrl,
		getGameIconUrl,
		getRarityCssVar,
		getRarityTier7,
		getRarityTierIndex,
		profileRank,
		totalCompletedGames,
		totalUnlockedAchievements,
		type Achievement,
		type Game
	} from '$lib/stores/Games.js';
	import { steamUser } from '$lib/stores/user.js';
	import { achievementJumpIntent } from '$lib/stores/ui.js';
	import { selectedGameId } from '$lib/stores/selectedGame.js';
	import { i18n, rarityLabelByIndex } from '$lib/stores/i18n.js';
	import { goto } from '$app/navigation';
	import { resolve } from '$app/paths';
	import { SvelteSet } from 'svelte/reactivity';

	let failedImageUrls = new SvelteSet<string>();

	function pickImageUrl(...candidates: Array<string | null | undefined>): string | null {
		for (const candidate of candidates) {
			const url = candidate?.trim();
			if (url && !failedImageUrls.has(url)) return url;
		}
		return null;
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

	function formatRarityPct(raw?: string | null): string {
		const val = parseFloat((raw ?? '').replace(',', '.'));
		if (!Number.isFinite(val)) return '—';
		return `${val.toFixed(1)}%`;
	}

	function formatCompletionPct(unlocked: number, total: number, rawPct: number): number {
		if (total <= 0) return 0;
		if (unlocked === total) return 100;
		return Math.min(99, Math.max(0, Math.round(rawPct)));
	}

	function getSourceLabel(game: Game): string {
		if (typeof game.source === 'object' && game.source?.type === 'Emulator') {
			return game.source.value || game.source.type;
		}
		return game.source?.type ?? '';
	}

	function openGame(gameId: string): void {
		selectedGameId.set(gameId);
		void goto(resolve('/(app)/games/[id]', { id: gameId }));
	}

	function openAchievement(item: RecentAchievement): void {
		achievementJumpIntent.set({
			gameId: item.gameId,
			achievementKey: item.key,
			token: Date.now()
		});
		selectedGameId.set(item.gameId);
		void goto(resolve('/(app)/games/[id]', { id: item.gameId }));
	}

	type RecentAchievement = Achievement & {
		gameName: string;
		gameId: string;
		gameHeaderImage: string;
	};

	let totalAllAchievements = $derived.by(() => {
		let total = 0;
		for (const game of $games) {
			if (game.achievements?.length) {
				total += computeGameProgress(game.achievements, game.achievement_groups).totalAll;
			} else {
				total += game.achievements_total || 0;
			}
		}
		return total;
	});

	let avgProgression = $derived.by(() => {
		if ($games.length === 0) return 0;
		const sum = $games.reduce((acc, g) => acc + getGameCompletionPct(g), 0);
		return Math.round(sum / $games.length);
	});

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

{#snippet trophySvg(size: number)}
	<svg
		width={size}
		height={size}
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
{/snippet}

<div class="main-content">
	<header class="home-header">
		<h1 class="home-title">{$i18n.t('sidebar.home')}</h1>
		{#if $steamUser || $games.length > 0}
			<div class="home-profile">
				{#if $steamUser?.personaname}
					<span class="profile-name">{$steamUser.personaname}</span>
					<span class="profile-sep" aria-hidden="true">·</span>
				{/if}
				<span class="profile-level">{$i18n.t('main.level', { level: $profileRank.level })}</span>
				<span class="profile-sep" aria-hidden="true">·</span>
				<span class="profile-xp tabular-nums">
					{$profileRank.currentLevelXp} / {$profileRank.nextLevelXp} XP
				</span>
			</div>
		{/if}
	</header>

	<section class="kpi-strip" aria-label={$i18n.t('statsPanel.stats')}>
		<div class="kpi-cell">
			<span class="kpi-label">{$i18n.t('statsPanel.unlocked')}</span>
			<span class="kpi-value tabular-nums">
				{$totalUnlockedAchievements}<span class="kpi-sub"> / {totalAllAchievements}</span>
			</span>
		</div>
		<div class="kpi-cell">
			<span class="kpi-label">{$i18n.t('main.platinums')}</span>
			<span class="kpi-value is-completed tabular-nums">{$totalCompletedGames}</span>
		</div>
		<div class="kpi-cell">
			<span class="kpi-label">{$i18n.t('statsPanel.avgProgress')}</span>
			<span class="kpi-value tabular-nums">{avgProgression}%</span>
		</div>
		<div class="kpi-cell">
			<span class="kpi-label">{$i18n.t('main.gamerscore')}</span>
			<span class="kpi-value tabular-nums">{$profileRank.score} G</span>
		</div>
	</section>

	<section class="home-section">
		<div class="section-header">
			<h2 class="section-title">{$i18n.t('main.recentActivity')}</h2>
		</div>

		{#if recentActivity.length > 0}
			<ul class="activity-list">
				{#each recentActivity as item (item.gameId + ':' + item.key)}
					{@const tier = getRarityTier7(item.completionpercentage)}
					{@const rarityLabel = rarityLabelByIndex($i18n.language, getRarityTierIndex(tier))}
					{@const iconUrl = pickImageUrl(item.icon, item.gameHeaderImage)}
					<li class="activity-item">
						<button type="button" class="activity-row" onclick={() => openAchievement(item)}>
							<div class="activity-icon-wrap">
								{#if iconUrl}
									<img
										src={iconUrl}
										alt={item.name}
										class="activity-icon"
										onerror={() => failedImageUrls.add(iconUrl)}
									/>
								{:else}
									{@render trophySvg(18)}
								{/if}
							</div>
							<div class="activity-info">
								<span class="activity-name">{item.name}</span>
								<span class="activity-meta tabular-nums">
									{item.unlocked_time
										? `${item.gameName} · ${formatDateTime(item.unlocked_time)}`
										: item.gameName}
								</span>
							</div>
							<span class="activity-rarity tabular-nums" title={rarityLabel}>
								<span class="rarity-dot" style:background={getRarityCssVar(tier)} aria-hidden="true"
								></span>
								<span>{formatRarityPct(item.completionpercentage)}</span>
							</span>
						</button>
					</li>
				{/each}
			</ul>
		{:else}
			<div class="empty-hint">{$i18n.t('main.noRecentActivity')}</div>
		{/if}
	</section>

	<section class="home-section library-section">
		<div class="section-header">
			<h2 class="section-title">{$i18n.t('main.recentLibrary')}</h2>
			<button
				type="button"
				class="library-link-btn"
				onclick={() => void goto(resolve('/(app)/games'))}
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
					<rect x="3" y="3" width="7" height="7" rx="1.5" />
					<rect x="14" y="3" width="7" height="7" rx="1.5" />
					<rect x="3" y="14" width="7" height="7" rx="1.5" />
					<rect x="14" y="14" width="7" height="7" rx="1.5" />
				</svg>
				<span>{$i18n.t('topbar.library')}</span>
				<svg
					width="13"
					height="13"
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
		</div>

		<div class="game-grid">
			{#each recentGames as game (game.id)}
				{@const iconUrl = pickImageUrl(getGameIconUrl(game), getGameHeaderUrl(game))}
				{@const progress = computeGameProgress(
					game.achievements ?? [],
					game.achievement_groups,
					$i18n.t('game.baseGame')
				)}
				{@const basePct = formatCompletionPct(
					progress.unlockedBase,
					progress.totalBase,
					progress.platinumPct
				)}
				{@const totalPct = formatCompletionPct(
					progress.unlockedAll,
					progress.totalAll,
					progress.globalPct
				)}
				{@const sourceLabel = getSourceLabel(game)}
				<button type="button" class="game-card" onclick={() => openGame(game.id)}>
					<div class="game-card-top">
						<div class="game-card-thumb">
							{#if iconUrl}
								<img
									src={iconUrl}
									alt={game.name || game.id}
									class="game-card-img"
									onerror={() => failedImageUrls.add(iconUrl)}
								/>
							{:else}
								{@render trophySvg(20)}
							{/if}
						</div>
						<div class="game-card-head">
							<div class="game-card-title-row">
								<span class="game-card-title">{game.name || game.id}</span>
								{#if progress.isPlatinum}
									<span
										class="game-platinum-icon"
										title={$i18n.t('game.completedLabel')}
										aria-label={$i18n.t('game.completedLabel')}
									>
										{@render trophySvg(13)}
									</span>
								{/if}
							</div>
							<span class="game-card-sub tabular-nums">
								{#if sourceLabel}{sourceLabel} ·
								{/if}{progress.unlockedAll}/{progress.totalAll}
							</span>
						</div>
					</div>

					<div class="game-card-gauges">
						{#if progress.hasDlc}
							<div class="gauge-row">
								<span class="gauge-label">{$i18n.t('main.platinumShort')}</span>
								<div class="gauge-track" aria-hidden="true">
									<div class="gauge-fill is-completed" style:width="{progress.platinumPct}%"></div>
								</div>
								<span class="gauge-pct tabular-nums">{basePct}%</span>
							</div>
							<div class="gauge-row">
								<span class="gauge-label">{$i18n.t('main.totalShort')}</span>
								<div class="gauge-track" aria-hidden="true">
									<div class="gauge-fill is-accent" style:width="{progress.globalPct}%"></div>
								</div>
								<span class="gauge-pct tabular-nums">{totalPct}%</span>
							</div>
						{:else}
							<div class="gauge-row">
								<div class="gauge-track" aria-hidden="true">
									<div
										class="gauge-fill"
										class:is-completed={progress.isPlatinum}
										class:is-accent={!progress.isPlatinum}
										style:width="{progress.platinumPct}%"
									></div>
								</div>
								<span class="gauge-pct tabular-nums" class:is-completed={progress.isPlatinum}>
									{basePct}%
								</span>
							</div>
						{/if}
					</div>

					<div class="game-card-footer tabular-nums">
						{game.playtime_seconds && game.playtime_seconds > 0
							? formatPlaytime(game.playtime_seconds)
							: $i18n.t('games.neverPlayed')}
					</div>
				</button>
			{/each}
		</div>
	</section>
</div>

<style>
	.main-content {
		padding: 24px 28px;
		overflow-y: auto;
		flex: 1;
		min-height: 0;
		display: flex;
		flex-direction: column;
		gap: 24px;
	}

	.home-header {
		display: flex;
		align-items: baseline;
		justify-content: space-between;
		gap: 12px;
		flex-wrap: wrap;
	}

	.home-title {
		font-size: 22px;
		font-weight: 700;
		letter-spacing: -0.02em;
		line-height: 1.2;
		color: var(--text-primary);
	}

	.home-profile {
		display: inline-flex;
		align-items: center;
		gap: 7px;
		flex-wrap: wrap;
		font-size: 12px;
		color: var(--text-secondary);
	}

	.profile-name {
		font-weight: 600;
		color: var(--text-primary);
	}

	.profile-sep,
	.profile-xp {
		color: var(--text-muted);
	}

	.kpi-strip {
		display: grid;
		grid-template-columns: repeat(4, minmax(0, 1fr));
		background: var(--surface-1);
		border: 1px solid var(--border-soft);
		border-radius: 10px;
	}

	.kpi-cell {
		display: flex;
		flex-direction: column;
		gap: 4px;
		padding: 13px 16px;
		min-width: 0;
	}

	.kpi-cell + .kpi-cell {
		border-left: 1px solid var(--border-soft);
	}

	.kpi-label {
		font-size: 11px;
		font-weight: 600;
		color: var(--text-muted);
		white-space: nowrap;
		overflow: hidden;
		text-overflow: ellipsis;
	}

	.kpi-value {
		font-size: 20px;
		font-weight: 700;
		letter-spacing: -0.02em;
		line-height: 1.15;
		color: var(--text-primary);
	}

	.kpi-value.is-completed {
		color: var(--completed);
	}

	.kpi-sub {
		font-size: 13px;
		font-weight: 500;
		color: var(--text-muted);
	}

	.home-section {
		display: flex;
		flex-direction: column;
	}

	.library-section {
		flex: 1;
	}

	.section-header {
		display: flex;
		align-items: center;
		justify-content: space-between;
		gap: 12px;
		margin-bottom: 10px;
	}

	.section-title {
		font-size: 11px;
		font-weight: 600;
		letter-spacing: 0.08em;
		text-transform: uppercase;
		color: var(--text-muted);
	}

	.library-link-btn {
		display: inline-flex;
		align-items: center;
		gap: 6px;
		padding: 5px 11px;
		border-radius: 999px;
		background: var(--surface-1);
		border: 1px solid var(--border-soft);
		color: var(--text-secondary);
		font-size: 12px;
		font-weight: 600;
		font-family: inherit;
		cursor: pointer;
		transition:
			background 0.16s ease,
			border-color 0.16s ease,
			color 0.16s ease;
	}

	.library-link-btn:hover {
		background: var(--surface-hover);
		border-color: var(--border-specular);
		color: var(--text-primary);
	}

	.activity-list {
		list-style: none;
		padding: 0;
		margin: 0;
		display: flex;
		flex-direction: column;
	}

	.activity-item {
		border-bottom: 1px solid var(--border-soft);
	}

	.activity-item:last-child {
		border-bottom: none;
	}

	.activity-row {
		display: flex;
		align-items: center;
		gap: 12px;
		width: 100%;
		padding: 9px 8px;
		border: none;
		border-radius: 8px;
		background: transparent;
		text-align: left;
		font-family: inherit;
		cursor: pointer;
		transition: background 0.15s ease;
	}

	.activity-row:hover {
		background: var(--surface-hover);
	}

	.activity-row:focus-visible {
		outline: 2px solid var(--accent);
		outline-offset: -2px;
	}

	.activity-icon-wrap {
		flex-shrink: 0;
		width: 36px;
		height: 36px;
		border-radius: 7px;
		overflow: hidden;
		background: var(--surface-2);
		color: var(--text-muted);
		display: flex;
		align-items: center;
		justify-content: center;
	}

	.activity-icon {
		width: 100%;
		height: 100%;
		object-fit: cover;
		display: block;
	}

	.activity-info {
		flex: 1;
		min-width: 0;
		display: flex;
		flex-direction: column;
		gap: 2px;
	}

	.activity-name {
		font-size: 13px;
		font-weight: 600;
		color: var(--text-primary);
		white-space: nowrap;
		overflow: hidden;
		text-overflow: ellipsis;
	}

	.activity-meta {
		font-size: 12px;
		color: var(--text-secondary);
		white-space: nowrap;
		overflow: hidden;
		text-overflow: ellipsis;
	}

	.activity-rarity {
		display: inline-flex;
		align-items: center;
		gap: 6px;
		flex-shrink: 0;
		font-size: 12px;
		font-weight: 500;
		color: var(--text-secondary);
	}

	.rarity-dot {
		width: 6px;
		height: 6px;
		border-radius: 50%;
		flex-shrink: 0;
	}

	.empty-hint {
		font-size: 13px;
		color: var(--text-muted);
		padding: 12px 0;
	}

	.game-grid {
		display: grid;
		grid-template-columns: repeat(auto-fill, minmax(220px, 1fr));
		gap: 10px;
	}

	.game-card {
		display: flex;
		flex-direction: column;
		gap: 10px;
		padding: 12px;
		border-radius: 10px;
		background: var(--surface-1);
		border: 1px solid var(--border-soft);
		text-align: left;
		font-family: inherit;
		cursor: pointer;
		min-width: 0;
		transition:
			background 0.16s ease,
			border-color 0.16s ease;
	}

	.game-card:hover {
		background: var(--surface-hover);
		border-color: var(--border-specular);
	}

	.game-card-top {
		display: flex;
		align-items: center;
		gap: 10px;
		min-width: 0;
	}

	.game-card-thumb {
		width: 44px;
		height: 44px;
		border-radius: 8px;
		overflow: hidden;
		background: var(--surface-2);
		color: var(--text-muted);
		flex-shrink: 0;
		display: flex;
		align-items: center;
		justify-content: center;
	}

	.game-card-img {
		width: 100%;
		height: 100%;
		object-fit: cover;
		display: block;
	}

	.game-card-head {
		flex: 1;
		min-width: 0;
		display: flex;
		flex-direction: column;
		gap: 2px;
	}

	.game-card-title-row {
		display: flex;
		align-items: center;
		gap: 6px;
		min-width: 0;
	}

	.game-card-title {
		flex: 1;
		min-width: 0;
		font-size: 13px;
		font-weight: 600;
		color: var(--text-primary);
		white-space: nowrap;
		overflow: hidden;
		text-overflow: ellipsis;
	}

	.game-platinum-icon {
		color: var(--completed);
		flex-shrink: 0;
		display: inline-flex;
		align-items: center;
	}

	.game-card-sub {
		font-size: 11px;
		color: var(--text-muted);
		white-space: nowrap;
		overflow: hidden;
		text-overflow: ellipsis;
	}

	.game-card-gauges {
		display: flex;
		flex-direction: column;
		gap: 5px;
	}

	.gauge-row {
		display: flex;
		align-items: center;
		gap: 8px;
	}

	.gauge-label {
		font-size: 11px;
		font-weight: 500;
		color: var(--text-muted);
		min-width: 34px;
		flex-shrink: 0;
	}

	.gauge-track {
		flex: 1;
		height: 3px;
		border-radius: 999px;
		background: var(--surface-2);
		overflow: hidden;
	}

	.gauge-fill {
		height: 100%;
		border-radius: 999px;
	}

	.gauge-fill.is-completed {
		background: var(--completed);
	}

	.gauge-fill.is-accent {
		background: var(--accent);
	}

	.gauge-pct {
		font-size: 11px;
		font-weight: 600;
		color: var(--text-secondary);
		flex-shrink: 0;
	}

	.gauge-pct.is-completed {
		color: var(--completed);
	}

	.game-card-footer {
		font-size: 11px;
		color: var(--text-muted);
	}

	@media (max-width: 720px) {
		.main-content {
			padding: 18px 16px;
			gap: 20px;
		}

		.kpi-strip {
			grid-template-columns: repeat(2, minmax(0, 1fr));
		}

		.kpi-cell:nth-child(3) {
			border-left: none;
		}

		.kpi-cell:nth-child(n + 3) {
			border-top: 1px solid var(--border-soft);
		}
	}
</style>
