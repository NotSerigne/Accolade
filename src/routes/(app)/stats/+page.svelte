<script lang="ts">
	// src/routes/(app)/stats/+page.svelte
	import { games, totalUnlockedAchievements, type Achievement } from '$lib/stores/Games.js';
	import { SvelteDate, SvelteMap } from 'svelte/reactivity';
	import { get } from 'svelte/store';
	import { i18n } from '$lib/stores/i18n.js';

	let activeTab = $state('overview');

	let totalPossibleAchievements = $derived(
		$games.reduce((acc, g) => acc + (g.achievements_total || g.achievements?.length || 0), 0)
	);

	let globalCompletionRate = $derived(
		totalPossibleAchievements > 0
			? Math.round(($totalUnlockedAchievements / totalPossibleAchievements) * 10000) / 100
			: 0
	);

	let remainingAchievements = $derived(totalPossibleAchievements - $totalUnlockedAchievements);

	let sortedByProgression = $derived.by(() => {
		return [...$games]
			.map((g) => {
				const total = g.achievements_total || g.achievements?.length || 0;
				const unlocked = g.achievements?.filter((a) => a.unlocked).length ?? 0;
				const pct = total > 0 ? (unlocked / total) * 100 : 0;
				return { ...g, unlockedCount: unlocked, totalCount: total, progression: pct };
			})
			.sort((a, b) => {
				if (Math.round(b.progression) !== Math.round(a.progression))
					return b.progression - a.progression;
				return b.unlockedCount - a.unlockedCount;
			});
	});

	let topGames = $derived(sortedByProgression.slice(0, 10));
	let podium = $derived(sortedByProgression.slice(0, 3));

	let allUnlockedAchievements = $derived.by(() => {
		return $games
			.flatMap((g) =>
				(g.achievements ?? []).filter((a) => a.unlocked).map((a) => ({ ...a, gameName: g.name }))
			)
			.sort((a, b) => (b.unlocked_time ?? 0) - (a.unlocked_time ?? 0));
	});

	let rarestAchievement = $derived.by(() => {
		const unlocked = allUnlockedAchievements;
		if (unlocked.length === 0) return null;
		return [...unlocked].sort(
			(a, b) => parseFloat(a.completionpercentage) - parseFloat(b.completionpercentage)
		)[0];
	});

	let today = new SvelteDate();
	let oneYearAgo = new SvelteDate();
	oneYearAgo.setFullYear(today.getFullYear() - 1);

	function formatDate(date: Date | SvelteDate) {
		return date.toLocaleDateString(get(i18n).locale, { day: 'numeric', month: 'short', year: 'numeric' });
	}

	let heatmapData = $derived.by(() => {
		const unlocked = allUnlockedAchievements;
		const grid = [];
		const now = new SvelteDate();

		// Apply offset based on chart navigation
		if (cumulativeTimeframe === '1y') {
			now.setFullYear(now.getFullYear() - currentWindowOffset);
		} else if (cumulativeTimeframe === '6m') {
			now.setMonth(now.getMonth() - currentWindowOffset * 6);
		} else if (cumulativeTimeframe === '1m') {
			now.setMonth(now.getMonth() - currentWindowOffset);
		} else if (cumulativeTimeframe === '1w') {
			now.setDate(now.getDate() - currentWindowOffset * 7);
		}

		now.setHours(23, 59, 59, 999);

		// Find the most recent Sunday to align the grid
		const lastSunday = new SvelteDate(now.getTime());
		lastSunday.setDate(now.getDate() - now.getDay());

		// Start 51 weeks before that Sunday (total 52 weeks)
		const startDate = new SvelteDate(lastSunday.getTime());
		startDate.setDate(lastSunday.getDate() - 51 * 7);
		startDate.setHours(0, 0, 0, 0);

		// Group achievements by day
		const dayMap = new SvelteMap<string, { count: number; achievements: Achievement[] }>();
		unlocked.forEach((a) => {
			if (!a.unlocked_time) return;
			try {
				const d = new Date(a.unlocked_time * 1000);
				if (isNaN(d.getTime())) return;
				const key = d.toISOString().split('T')[0];
				if (!dayMap.has(key)) {
					dayMap.set(key, { count: 0, achievements: [] });
				}
				const data = dayMap.get(key)!;
				data.count++;
				data.achievements.push(a);
			} catch {
				console.error('Error processing achievement date');
			}
		});

		// Build 52 columns
		for (let w = 0; w < 52; w++) {
			const col = [];
			for (let d = 0; d < 7; d++) {
				const date = new SvelteDate(startDate.getTime());
				date.setDate(startDate.getDate() + w * 7 + d);

				const key = (() => {
					try {
						return date.toISOString().split('T')[0];
					} catch {
						// Fallback for invalid dates
						return `${date.getFullYear()}-${String(date.getMonth() + 1).padStart(2, '0')}-${String(
							date.getDate()
						).padStart(2, '0')}`;
					}
				})();

				const dayData = dayMap.get(key) || { count: 0, achievements: [] };
				const isFuture = date.getTime() > new Date().getTime(); // Future relative to ACTUAL now

				col.push({
					date: new SvelteDate(date.getTime()),
					count: dayData.count,
					achievements: dayData.achievements,
					isFuture
				});
			}
			grid.push(col);
		}
		return grid;
	});

	let monthLabels = $derived([
		$i18n.t('stats.month.jan'),
		$i18n.t('stats.month.feb'),
		$i18n.t('stats.month.mar'),
		$i18n.t('stats.month.apr'),
		$i18n.t('stats.month.may'),
		$i18n.t('stats.month.jun'),
		$i18n.t('stats.month.jul'),
		$i18n.t('stats.month.aug'),
		$i18n.t('stats.month.sep'),
		$i18n.t('stats.month.oct'),
		$i18n.t('stats.month.nov'),
		$i18n.t('stats.month.dec')
	]);

	let heatmapMonths = $derived.by(() => {
		try {
			if (!heatmapData || heatmapData.length === 0) return [];
			const labels: { label: string; index: number }[] = [];
			let currentMonth = -1;
			heatmapData.forEach((col, i) => {
				if (!col || col.length === 0) return;
				const month = col[0].date.getMonth();
				if (month !== currentMonth) {
					labels.push({ label: monthLabels[month], index: i });
					currentMonth = month;
				}
			});
			return labels;
		} catch {
			console.error('Error calculating heatmap months');
			return [];
		}
	});

	let cumulativeTimeframe = $state('all');
	let currentWindowOffset = $state(0);
	let chartContainerWidth = $state(0);
	let chartPhysicalWidth = $derived(chartContainerWidth);

	let timeframeLabel = $derived.by(() => {
		switch (cumulativeTimeframe) {
			case '1y':
				return $i18n.t('stats.lastYear');
			case '6m':
				return $i18n.t('stats.last6Months');
			case '1m':
				return $i18n.t('stats.lastMonth');
			case '1w':
				return $i18n.t('stats.lastWeek');
			default:
				return $i18n.t('stats.fullHistory');
		}
	});

	let cumulativeData = $derived.by(() => {
		try {
			const allUnlocked = [...allUnlockedAchievements].sort(
				(a, b) => (a.unlocked_time ?? 0) - (b.unlocked_time ?? 0)
			);
			if (allUnlocked.length === 0) return [];

			let startTime = 0;
			let endTime = Math.floor(Date.now() / 1000);
			const now = new SvelteDate();

			if (cumulativeTimeframe === '1y') {
				const start = new SvelteDate(now.getTime());
				start.setFullYear(start.getFullYear() - (currentWindowOffset + 1));
				const end = new SvelteDate(now.getTime());
				end.setFullYear(end.getFullYear() - currentWindowOffset);
				startTime = Math.floor(start.getTime() / 1000);
				endTime = Math.floor(end.getTime() / 1000);
			} else if (cumulativeTimeframe === '6m') {
				const start = new SvelteDate(now.getTime());
				start.setMonth(start.getMonth() - (currentWindowOffset + 1) * 6);
				const end = new SvelteDate(now.getTime());
				end.setMonth(end.getMonth() - currentWindowOffset * 6);
				startTime = Math.floor(start.getTime() / 1000);
				endTime = Math.floor(end.getTime() / 1000);
			} else if (cumulativeTimeframe === '1m') {
				const start = new SvelteDate(now.getTime());
				start.setMonth(start.getMonth() - (currentWindowOffset + 1));
				const end = new SvelteDate(now.getTime());
				end.setMonth(end.getMonth() - currentWindowOffset);
				startTime = Math.floor(start.getTime() / 1000);
				endTime = Math.floor(end.getTime() / 1000);
			} else if (cumulativeTimeframe === '1w') {
				const start = new SvelteDate(now.getTime());
				start.setDate(start.getDate() - (currentWindowOffset + 1) * 7);
				const end = new SvelteDate(now.getTime());
				end.setDate(end.getDate() - currentWindowOffset * 7);
				startTime = Math.floor(start.getTime() / 1000);
				endTime = Math.floor(end.getTime() / 1000);
			}

			let runningCount = 0;
			const fullHistory: { x: number; y: number; date: Date | SvelteDate; count: number }[] = [];

			allUnlocked.forEach((a) => {
				runningCount++;
				fullHistory.push({
					x: a.unlocked_time ?? 0,
					y: runningCount,
					date: new SvelteDate((a.unlocked_time ?? 0) * 1000),
					count: runningCount
				});
			});

			if (cumulativeTimeframe === 'all') {
				const nowTime = Math.floor(Date.now() / 1000);
				fullHistory.push({
					x: nowTime,
					y: runningCount,
					date: new SvelteDate(nowTime * 1000),
					count: runningCount
				});
				return fullHistory;
			}

			const historyInWindow = fullHistory.filter((p) => p.x >= startTime && p.x <= endTime);
			const countAtStart = fullHistory.filter((p) => p.x < startTime).length;

			const startPoint = {
				x: startTime,
				y: countAtStart,
				date: new SvelteDate(startTime * 1000),
				count: countAtStart
			};

			const result = [startPoint, ...historyInWindow];

			result.push({
				x: endTime,
				y:
					historyInWindow.length > 0 ? historyInWindow[historyInWindow.length - 1].y : countAtStart,
				date: new SvelteDate(endTime * 1000),
				count:
					historyInWindow.length > 0 ? historyInWindow[historyInWindow.length - 1].y : countAtStart
			});

			return result;
		} catch {
			console.error('Error calculating cumulative progress data');
			return [];
		}
	});

	let cumulativeMeta = $derived.by(() => {
		const width = chartPhysicalWidth || 800;
		const height = 180;

		type Point = {
			x: number;
			y: number;
			px: number;
			py: number;
			date: Date | SvelteDate;
			count: number;
		};

		if (cumulativeData.length === 0) {
			return { minX: 0, maxX: 1, minY: 0, maxY: 1, path: '', points: [] as Point[], width, height };
		}

		const minX = cumulativeData[0].x;
		const maxX = cumulativeData[cumulativeData.length - 1].x;
		const xRange = Math.max(1, maxX - minX);

		const minY = Math.min(...cumulativeData.map((p) => p.y));
		const maxY = Math.max(...cumulativeData.map((p) => p.y), 1);

		const yRange = maxY - minY;
		const displayMinY = Math.max(0, minY - yRange * 0.1);
		const displayMaxY = maxY + (yRange * 0.1 || 10);
		const displayYRange = Math.max(1, displayMaxY - displayMinY);

		const points = cumulativeData.map((p) => {
			const px = ((p.x - minX) / xRange) * width;
			const py = height - ((p.y - displayMinY) / displayYRange) * height;
			return {
				...p,
				px: isNaN(px) ? 0 : px,
				py: isNaN(py) ? 0 : py
			};
		});

		const path = points.map((p) => `${p.px.toFixed(2)},${p.py.toFixed(2)}`).join(' ');

		return { minX, maxX, minY: displayMinY, maxY: displayMaxY, path, points, width, height };
	});

	let rhythmData = $derived.by(() => {
		const unlocked = allUnlockedAchievements;
		const data = [];
		const now = new SvelteDate();
		now.setHours(23, 59, 59, 999);

		const unit =
			cumulativeTimeframe === '1w' || cumulativeTimeframe === '1m'
				? 'day'
				: cumulativeTimeframe === '1y'
					? 'month'
					: 'week';

		const count =
			cumulativeTimeframe === '1w'
				? 7
				: cumulativeTimeframe === '1m'
					? 30
					: cumulativeTimeframe === '1y'
						? 12
						: 26;

		for (let i = count - 1; i >= 0; i--) {
			const start = new SvelteDate(now.getTime());
			const end = new SvelteDate(now.getTime());

			if (unit === 'day') {
				start.setDate(now.getDate() - i);
				start.setHours(0, 0, 0, 0);
				end.setDate(now.getDate() - i + 1);
				end.setHours(0, 0, 0, 0);
			} else if (unit === 'month') {
				start.setMonth(now.getMonth() - i, 1);
				start.setHours(0, 0, 0, 0);
				end.setMonth(now.getMonth() - i + 1, 1);
				end.setHours(0, 0, 0, 0);
			} else {
				// week
				const day = now.getDay();
				const diff = now.getDate() - day - i * 7;
				start.setDate(diff);
				start.setHours(0, 0, 0, 0);
				end.setDate(diff + 7);
				end.setHours(0, 0, 0, 0);
			}

			const periodUnlocked = unlocked.filter((a) => {
				const ut = (a.unlocked_time ?? 0) * 1000;
				return ut >= start.getTime() && ut < end.getTime();
			}).length;

			data.push({
				start: new SvelteDate(start.getTime()),
				count: periodUnlocked,
				label:
					unit === 'day'
						? start.toLocaleDateString($i18n.locale, { day: 'numeric', month: 'short' })
						: unit === 'month'
							? start.toLocaleDateString($i18n.locale, { month: 'short' })
							: $i18n.t('stats.weekAbbrev', {
									date: start.toLocaleDateString($i18n.locale, { day: 'numeric', month: 'short' })
								})
			});
		}

		const max = Math.max(...data.map((d) => d.count), 1);
		return data.map((d) => ({
			...d,
			height: (d.count / max) * 100,
			isMax: d.count === max && max > 0
		}));
	});

	let bestWeek = $derived.by(() => {
		const unlocked = allUnlockedAchievements;
		if (unlocked.length === 0) return { count: 0, date: '' };

		const weekMap = new SvelteMap<string, number>();
		unlocked.forEach((a) => {
			if (!a.unlocked_time) return;
			const d = new SvelteDate(a.unlocked_time * 1000);
			const startOfWeek = new SvelteDate(d.getTime());
			startOfWeek.setDate(d.getDate() - d.getDay());
			const key = startOfWeek.toISOString().split('T')[0];
			weekMap.set(key, (weekMap.get(key) || 0) + 1);
		});

		let maxCount = 0;
		let bestKey = '';
		for (const [key, count] of weekMap.entries()) {
			if (count > maxCount) {
				maxCount = count;
				bestKey = key;
			}
		}
		return { count: maxCount, date: bestKey };
	});

	let tooltip = $state({
		show: false,
		x: 0,
		y: 0,
		title: '',
		value: '',
		sub: '',
		list: [] as string[]
	});

	function showTooltip(
		e: MouseEvent,
		title: string,
		value: string,
		sub: string = '',
		list: string[] = []
	) {
		tooltip = {
			show: true,
			x: e.clientX,
			y: e.clientY,
			title,
			value,
			sub,
			list
		};
	}

	function hideTooltip() {
		tooltip.show = false;
	}
</script>

<main class="stats-page">
	{#if tooltip.show}
		<div class="custom-tooltip" style="left: {tooltip.x + 10}px; top: {tooltip.y + 10}px">
			<div class="tooltip-title">{tooltip.title}</div>
			<div class="tooltip-value">{tooltip.value}</div>
			{#if tooltip.sub}
				<div class="tooltip-sub">{tooltip.sub}</div>
			{/if}
			{#if tooltip.list.length > 0}
				<ul class="tooltip-list">
					{#each tooltip.list.slice(0, 5) as item, i (i)}
						<li>• {item}</li>
					{/each}
					{#if tooltip.list.length > 5}
						<li class="more">{$i18n.t('stats.andMore', { count: tooltip.list.length - 5 })}</li>
					{/if}
				</ul>
			{/if}
		</div>
	{/if}

	<header class="stats-header">
		<div class="header-left">
			<h1>{$i18n.t('stats.title')}</h1>
			<div class="tabs">
				<button
					type="button"
					class:active={activeTab === 'overview'}
					onclick={() => {
						console.log('Switching to overview');
						activeTab = 'overview';
					}}
				>
					{$i18n.t('stats.tabOverview')}
				</button>
				<button
					type="button"
					class:active={activeTab === 'activity'}
					onclick={() => {
						console.log('Switching to activity');
						activeTab = 'activity';
					}}
				>
					{$i18n.t('stats.tabActivity')}
				</button>
			</div>
		</div>
		<div class="header-right">
			<span class="date-now">{formatDate(today)}</span>
		</div>
	</header>

	<div class="stats-content scrollable">
		{#if activeTab === 'overview'}
			<section class="grid-overview">
				<!-- Stat Cards -->
				<div class="card stat-summary">
					<div class="icon-circle games">
						<svg
							width="16"
							height="16"
							viewBox="0 0 24 24"
							fill="none"
							stroke="currentColor"
							stroke-width="2"><path d="M6 12L3 17V19H21V17L18 12M6 12L12 3L18 12M6 12H18" /></svg
						>
					</div>
					<div class="card-info">
						<span class="label">{$i18n.t('stats.gamesDetected')}</span>
						<span class="value">{$games.length}</span>
					</div>
				</div>

				<div class="card stat-summary">
					<div class="icon-circle trophy">
						<svg
							width="16"
							height="16"
							viewBox="0 0 24 24"
							fill="none"
							stroke="currentColor"
							stroke-width="2"
							><path
								d="M6 9H4.5a2.5 2.5 0 0 1 0-5H6M18 9h1.5a2.5 2.5 0 0 0 0-5H18M4 22h16M10 14.66V17c0 .55.47.98.97 1.21C11.47 18.44 12 19 12 19s.53-.56 1.03-.79c.5-.23.97-.66.97-1.21v-2.34M7 2h10a1 1 0 0 1 1 1v12a1 1 0 0 1-1 1H7a1 1 0 0 1-1-1V3a1 1 0 0 1 1-1Z"
							/></svg
						>
					</div>
					<div class="card-info">
						<span class="label">{$i18n.t('stats.achievementsUnlocked')}</span>
						<span class="value">{$totalUnlockedAchievements}</span>
						<span class="sub">{$i18n.t('stats.outOf', { count: totalPossibleAchievements })}</span>
					</div>
				</div>

				<div class="card stat-summary">
					<div class="icon-circle rate">
						<svg
							width="16"
							height="16"
							viewBox="0 0 24 24"
							fill="none"
							stroke="currentColor"
							stroke-width="2"
							><circle cx="12" cy="12" r="10" /><circle cx="12" cy="12" r="3" /></svg
						>
					</div>
					<div class="card-info">
						<span class="label">{$i18n.t('stats.completionRate')}</span>
						<span class="value">{globalCompletionRate}%</span>
						<span class="sub">{$i18n.t('stats.globalAverage')}</span>
					</div>
				</div>

				<div class="card stat-summary">
					<div class="icon-circle remaining">
						<svg
							width="16"
							height="16"
							viewBox="0 0 24 24"
							fill="none"
							stroke="currentColor"
							stroke-width="2"
							><path
								d="M12 2v4M12 18v4M4.93 4.93l2.83 2.83M16.24 16.24l2.83 2.83M2 12h4M18 12h4M4.93 19.07l2.83-2.83M16.24 7.76l2.83-2.83"
							/></svg
						>
					</div>
					<div class="card-info">
						<span class="label">{$i18n.t('stats.achievementsRemaining')}</span>
						<span class="value">{remainingAchievements}</span>
						<span class="sub">{$i18n.t('stats.toUnlock')}</span>
					</div>
				</div>

				<!-- Main Grid Row -->
				<div class="card top-games">
					<div class="card-header">
						<h2>{$i18n.t('stats.topGamesByProgress')}</h2>
						<span class="count">{$i18n.t('stats.gamesCount', { count: $games.length })}</span>
					</div>
					<div class="games-progress-list">
						{#each topGames as game (game.steam_id)}
							<div class="game-progress-row">
								<div class="game-info">
									<span class="name">{game.name}</span>
									{#if game.progression === 100}
										<span class="completed-tag">Remastered</span>
									{/if}
								</div>
								<div class="progress-bar-wrap">
									<div class="progress-bar">
										<div class="progress-fill" style="width: {game.progression}%"></div>
									</div>
									<span class="progress-pct">{Math.round(game.progression)}%</span>
								</div>
							</div>
						{/each}
					</div>
				</div>

				<div class="side-column">
					<div class="card global-progression">
						<h2>{$i18n.t('stats.globalProgression')}</h2>
						<div class="donut-wrap">
							<div class="donut">
								<svg class="donut-ring" viewBox="0 0 140 140" aria-hidden="true">
									<circle class="donut-track" cx="70" cy="70" r="58" />
									<circle
										class="donut-progress"
										cx="70"
										cy="70"
										r="58"
										pathLength="100"
										stroke-dasharray="{globalCompletionRate} 100"
									/>
								</svg>
								<div class="donut-inner">
									<span class="donut-val">{globalCompletionRate}%</span>
									<span class="donut-label">{$i18n.t('stats.completionLabel')}</span>
								</div>
							</div>
						</div>
						<div class="unlocked-count">
							<span class="val">{$totalUnlockedAchievements} / {totalPossibleAchievements}</span>
							<span class="lbl">{$i18n.t('stats.unlockedAchievementsLabel')}</span>
						</div>
					</div>

					<div class="card podium">
						<h2>{$i18n.t('stats.podium')}</h2>
						<div class="podium-list">
							{#each podium as game, i (game.steam_id)}
								<div class="podium-item">
									<div class="rank">{i + 1}</div>
									<span class="name">{game.name}</span>
									<span class="pct">{Math.round(game.progression)}%</span>
								</div>
							{/each}
						</div>
					</div>

					<div class="card rarest">
						<span class="card-subtitle"
							><svg
								width="12"
								height="12"
								viewBox="0 0 24 24"
								fill="none"
								stroke="currentColor"
								stroke-width="2"><circle cx="12" cy="12" r="10" /><path d="M12 8v8M8 12h8" /></svg
							> {$i18n.t('stats.rarestAchievement')}</span
						>
						{#if rarestAchievement}
							<div class="rarity-item">
								{#if rarestAchievement.icon}
									<img
										src={rarestAchievement.icon}
										alt={rarestAchievement.name}
										onerror={(e) => {
											const t = e.target as HTMLImageElement;
											t.style.display = 'none';
										}}
									/>
								{:else}
									<div class="rarity-icon-placeholder">🏆</div>
								{/if}
								<div class="info">
									<span class="name">{rarestAchievement.name}</span>
									<span class="game">{rarestAchievement.gameName}</span>
								</div>
								<span class="rarity-badge">{rarestAchievement.completionpercentage}%</span>
							</div>
						{:else}
							<p class="empty">{$i18n.t('stats.noAchievement')}</p>
						{/if}
					</div>

					<div class="card best-week">
						<span class="card-subtitle"
							><svg
								width="12"
								height="12"
								viewBox="0 0 24 24"
								fill="none"
								stroke="currentColor"
								stroke-width="2"><path d="M13 2L3 14h9l-1 8 10-12h-9l1-8z" /></svg
							> {$i18n.t('stats.bestWeek')}</span
						>
						<div class="best-week-val">
							<span class="val">{bestWeek.count}</span>
							<span class="lbl">{$i18n.t('stats.achievementsInAWeek')}</span>
						</div>
					</div>
				</div>
			</section>
		{:else}
			<section class="activity-view">
				<div class="card heatmap-card">
					<div class="card-header">
						<h2>{$i18n.t('stats.tabActivity')}</h2>
						<span class="sub"
							>{$i18n.t('stats.achievementsOver52Weeks', { count: $totalUnlockedAchievements })}</span
						>
					</div>
					<div class="heatmap-container">
						<div class="heatmap-wrapper">
							<div class="heatmap-y-labels">
								<span class="day-label">{$i18n.t('stats.dayMon')}</span>
								<span class="day-label">{$i18n.t('stats.dayWed')}</span>
								<span class="day-label">{$i18n.t('stats.dayFri')}</span>
								<span class="day-label">{$i18n.t('stats.daySun')}</span>
							</div>
							<div class="heatmap-scroll-area">
								<div class="heatmap-months">
									{#each heatmapMonths as { label, index }, i (label + i)}
										<span class="month-label" style="left: {index * 13}px">{label}</span>
									{/each}
								</div>
								<div class="heatmap-grid">
									{#each heatmapData as col, i (i)}
										<div class="heatmap-col">
											{#each col as cell (cell.date.getTime())}
												<div
													class="heatmap-cell"
													role="gridcell"
													tabindex="-1"
													aria-label={$i18n.t('stats.activityCellAriaLabel')}
													class:lvl1={cell.count > 0 && cell.count <= 1}
													class:lvl2={cell.count > 1 && cell.count <= 3}
													class:lvl3={cell.count > 3 && cell.count <= 6}
													class:lvl4={cell.count > 6 && cell.count <= 10}
													class:lvl5={cell.count > 10}
													class:is-future={cell.isFuture}
													onmouseenter={(e) =>
														showTooltip(
															e,
															cell.date.toLocaleDateString($i18n.locale, {
																day: 'numeric',
																month: 'long',
																year: 'numeric'
															}),
															$i18n.t('stats.achievementsCount', { count: cell.count }),
															'',
															cell.achievements.map((a) => a.name)
														)}
													onmouseleave={hideTooltip}
												></div>
											{/each}
										</div>
									{/each}
								</div>
							</div>
						</div>
						<div class="heatmap-legend">
							<span>{$i18n.t('stats.less')}</span>
							<div class="heatmap-cell"></div>
							<div class="heatmap-cell lvl1"></div>
							<div class="heatmap-cell lvl2"></div>
							<div class="heatmap-cell lvl3"></div>
							<div class="heatmap-cell lvl4"></div>
							<div class="heatmap-cell lvl5"></div>
							<span>{$i18n.t('stats.more')}</span>
						</div>
					</div>
				</div>

				<div class="charts-row">
					<div class="card chart-card">
						<div class="chart-header-row">
							<div class="title-group">
								<h2>{$i18n.t('stats.cumulativeProgress')}</h2>
								<p class="sub">{timeframeLabel}</p>
							</div>
							<div class="chart-nav-group">
								{#if cumulativeTimeframe !== 'all'}
									<div class="window-nav">
										<button onclick={() => currentWindowOffset++} aria-label={$i18n.t('stats.previous')}>
											<svg
												width="14"
												height="14"
												viewBox="0 0 24 24"
												fill="none"
												stroke="currentColor"
												stroke-width="2.5"><path d="M15 18l-6-6 6-6" /></svg
											>
										</button>
										<button
											onclick={() => (currentWindowOffset = Math.max(0, currentWindowOffset - 1))}
											disabled={currentWindowOffset === 0}
											aria-label={$i18n.t('stats.next')}
										>
											<svg
												width="14"
												height="14"
												viewBox="0 0 24 24"
												fill="none"
												stroke="currentColor"
												stroke-width="2.5"><path d="M9 18l6-6-6-6" /></svg
											>
										</button>
									</div>
								{/if}
								<div class="timeframe-selector">
									<button
										class:active={cumulativeTimeframe === 'all'}
										onclick={() => {
											cumulativeTimeframe = 'all';
											currentWindowOffset = 0;
									}}>{$i18n.t('stats.all')}</button
									>
									<button
										class:active={cumulativeTimeframe === '1y'}
										onclick={() => {
											cumulativeTimeframe = '1y';
											currentWindowOffset = 0;
										}}>{$i18n.t('stats.year')}</button
									>
									<button
										class:active={cumulativeTimeframe === '1m'}
										onclick={() => {
											cumulativeTimeframe = '1m';
											currentWindowOffset = 0;
										}}>{$i18n.t('stats.month')}</button
									>
									<button
										class:active={cumulativeTimeframe === '1w'}
										onclick={() => {
											cumulativeTimeframe = '1w';
											currentWindowOffset = 0;
										}}>{$i18n.t('stats.week')}</button
									>
								</div>
							</div>
						</div>
						<div class="chart-container" bind:clientWidth={chartContainerWidth}>
							{#if cumulativeData.length > 0 && cumulativeMeta.path}
								<svg
									width="100%"
									height="100%"
									viewBox="0 0 {cumulativeMeta.width} {cumulativeMeta.height}"
									preserveAspectRatio="none"
								>
									<defs>
										<linearGradient id="line-gradient" x1="0%" y1="0%" x2="0%" y2="100%">
											<stop offset="0%" stop-color="var(--accent, #c8a96e)" stop-opacity="0.2" />
											<stop offset="100%" stop-color="var(--accent, #c8a96e)" stop-opacity="0" />
										</linearGradient>
									</defs>

									<path
										d="M 0 {cumulativeMeta.height} L {cumulativeMeta.path} L {cumulativeMeta.width} {cumulativeMeta.height} Z"
										fill="url(#line-gradient)"
									/>
									<polyline
										fill="none"
										stroke="var(--accent, #c8a96e)"
										stroke-width="2"
										vector-effect="non-scaling-stroke"
										points={cumulativeMeta.path}
									/>

									{#each cumulativeMeta.points as p, i (p.x + '-' + i)}
										<circle
											cx={p.px}
											cy={p.py}
											r="2.5"
											fill="var(--accent, #c8a96e)"
											class="chart-point"
											role="img"
											aria-label={$i18n.t('stats.dataPoint')}
											onmouseenter={(e) =>
												showTooltip(
													e,
													p.date.toLocaleDateString($i18n.locale, {
														day: 'numeric',
														month: 'short',
														year: 'numeric',
														hour: '2-digit',
														minute: '2-digit'
													}),
													$i18n.t('stats.cumulativeAchievements', { count: p.count })
												)}
											onmouseleave={hideTooltip}
										/>
									{/each}
								</svg>
							{:else if activeTab === 'activity'}
								<div class="empty-chart">{$i18n.t('stats.notEnoughData')}</div>
							{/if}
						</div>
					</div>
					<div class="card chart-card">
						<h2>{$i18n.t('stats.unlockRhythm')}</h2>
						<p class="sub">
							{#if cumulativeTimeframe === '1w' || cumulativeTimeframe === '1m'}
								{$i18n.t('stats.achievementsPerDay')}
							{:else if cumulativeTimeframe === '1y'}
								{$i18n.t('stats.achievementsPerMonth')}
							{:else}
								{$i18n.t('stats.achievementsPerWeek')}
							{/if}
						</p>
						<div class="chart-container">
							<div class="bar-chart">
								{#each rhythmData as item, i (item.start.getTime() + '-' + i)}
									<div
										class="bar"
										role="img"
										aria-label={$i18n.t('stats.weeklyActivityBar')}
										class:max={item.isMax}
										class:empty={item.count === 0}
										style="height: {Math.max(item.height, 2)}%"
										onmouseenter={(e) =>
											showTooltip(
												e,
												item.label,
												$i18n.t('stats.achievementsUnlockedCount', { count: item.count })
											)}
										onmouseleave={hideTooltip}
									></div>
								{/each}
							</div>
						</div>
					</div>
				</div>

				<div class="card recent-list">
					<div class="card-header">
						<h2>{$i18n.t('stats.recentAchievements')}</h2>
						<span class="count">{$i18n.t('stats.last50Achievements')}</span>
					</div>
					<div class="recent-scroll">
						{#each allUnlockedAchievements.slice(0, 50) as a (a.gameName + ':' + a.key)}
							<div class="recent-item">
								<img src={a.icon} alt={a.name} />
								<div class="info">
									<div class="name-row">
										<span class="name">{a.name}</span>
										<span class="rarity-badge small">{a.completionpercentage}%</span>
									</div>
									<span class="game">{a.gameName}</span>
									<p class="desc">{a.desc}</p>
								</div>
								<span class="time"
									>🕒 {new Date((a.unlocked_time ?? 0) * 1000).toLocaleDateString($i18n.locale, {
										day: 'numeric',
										month: 'short'
									})}</span
								>
							</div>
						{/each}
					</div>
				</div>
			</section>
		{/if}
	</div>
</main>

<style>
	.stats-page {
		display: flex;
		flex-direction: column;
		background: #161616;
		color: #fff;
		height: 100%;
		overflow: hidden;
	}

	.stats-header {
		padding: 24px 32px;
		display: flex;
		justify-content: space-between;
		align-items: flex-start;
	}

	.stats-header h1 {
		font-size: 28px;
		font-weight: 700;
		margin: 0 0 16px 0;
	}

	.tabs {
		display: flex;
		gap: 8px;
		background: rgba(255, 255, 255, 0.05);
		padding: 4px;
		border-radius: 8px;
		width: fit-content;
	}

	.tabs button {
		background: transparent;
		border: none;
		color: rgba(255, 255, 255, 0.5);
		padding: 6px 16px;
		border-radius: 6px;
		font-size: 13px;
		cursor: pointer;
		transition: all 0.2s;
	}

	.tabs button.active {
		background: rgba(200, 169, 110, 0.1);
		color: var(--accent, #c8a96e);
	}

	.date-now {
		color: rgba(255, 255, 255, 0.3);
		font-size: 12px;
	}

	.scrollable {
		flex: 1;
		overflow-y: auto;
		padding: 0 32px 32px;
	}

	.grid-overview {
		display: grid;
		grid-template-columns: repeat(4, 1fr);
		grid-template-rows: auto 1fr;
		gap: 16px;
	}

	.card {
		background: #1a1a1a;
		border: 1px solid rgba(255, 255, 255, 0.05);
		border-radius: 12px;
		padding: 20px;
	}

	.stat-summary {
		display: flex;
		align-items: center;
		gap: 16px;
	}

	.icon-circle {
		width: 40px;
		height: 40px;
		border-radius: 10px;
		display: flex;
		align-items: center;
		justify-content: center;
		background: rgba(255, 255, 255, 0.05);
	}

	.icon-circle.games {
		color: var(--accent, #c8a96e);
		background: rgba(200, 169, 110, 0.1);
	}
	.icon-circle.trophy {
		color: #3ddc84;
		background: rgba(61, 220, 132, 0.1);
	}
	.icon-circle.rate {
		color: #a855f7;
		background: rgba(168, 85, 247, 0.1);
	}
	.icon-circle.remaining {
		color: #f4b860;
		background: rgba(244, 184, 96, 0.1);
	}

	.card-info {
		display: flex;
		flex-direction: column;
	}
	.card-info .label {
		font-size: 10px;
		font-weight: 600;
		color: rgba(255, 255, 255, 0.3);
		letter-spacing: 0.05em;
	}
	.card-info .value {
		font-size: 24px;
		font-weight: 700;
		margin: 2px 0;
		color: #fff;
	}
	.card-info .sub {
		font-size: 11px;
		color: rgba(255, 255, 255, 0.2);
	}

	.top-games {
		grid-column: 1 / 4;
	}
	.card-header {
		display: flex;
		justify-content: space-between;
		align-items: baseline;
		margin-bottom: 20px;
	}
	.card-header h2 {
		font-size: 16px;
		font-weight: 600;
		color: #fff;
	}
	.card-header .count {
		font-size: 11px;
		color: rgba(255, 255, 255, 0.2);
	}

	.game-progress-row {
		margin-bottom: 16px;
	}
	.game-progress-row .game-info {
		display: flex;
		align-items: center;
		gap: 8px;
		margin-bottom: 8px;
	}
	.game-progress-row .name {
		font-size: 14px;
		font-weight: 500;
		color: #e0e0e0;
	}
	.game-progress-row .completed-tag {
		font-size: 10px;
		color: rgba(255, 255, 255, 0.4);
	}

	.progress-bar-wrap {
		display: flex;
		align-items: center;
		gap: 12px;
	}
	.progress-bar {
		flex: 1;
		height: 12px;
		background: rgba(255, 255, 255, 0.05);
		border-radius: 6px;
		overflow: hidden;
	}
	.progress-fill {
		height: 100%;
		background: var(--accent, #c8a96e);
		border-radius: 6px;
	}
	.progress-pct {
		font-size: 12px;
		color: var(--accent, #c8a96e);
		font-weight: 600;
		width: 40px;
		text-align: right;
	}

	.side-column {
		display: flex;
		flex-direction: column;
		gap: 16px;
	}

	.global-progression {
		display: flex;
		flex-direction: column;
		align-items: center;
		text-align: center;
	}
	.global-progression h2 {
		font-size: 11px;
		font-weight: 600;
		color: rgba(255, 255, 255, 0.3);
		margin-bottom: 20px;
		align-self: flex-start;
	}

	.donut-wrap {
		position: relative;
		width: 140px;
		height: 140px;
		margin-bottom: 16px;
	}
	.donut {
		position: relative;
		width: 100%;
		height: 100%;
		border-radius: 50%;
		display: flex;
		align-items: center;
		justify-content: center;
		box-shadow:
			0 0 0 1px rgba(255, 255, 255, 0.025),
			0 0 22px color-mix(in srgb, var(--accent, #c8a96e) 12%, transparent);
	}
	.donut-ring {
		position: absolute;
		inset: 0;
		width: 100%;
		height: 100%;
		transform: rotate(-90deg);
		overflow: visible;
	}
	.donut-track,
	.donut-progress {
		fill: none;
		stroke-width: 12;
	}
	.donut-track {
		stroke: rgba(255, 255, 255, 0.055);
	}
	.donut-progress {
		stroke: color-mix(in srgb, var(--accent, #c8a96e) 82%, #ffffff);
		stroke-linecap: round;
		filter: drop-shadow(0 0 5px color-mix(in srgb, var(--accent, #c8a96e) 24%, transparent));
		transition: stroke-dasharray 0.6s ease;
	}
	.donut-inner {
		width: 82%;
		height: 82%;
		background: #1a1a1a;
		box-shadow: inset 0 0 18px rgba(0, 0, 0, 0.22);
		border-radius: 50%;
		display: flex;
		flex-direction: column;
		align-items: center;
		justify-content: center;
	}
	.donut-val {
		font-size: 24px;
		font-weight: 700;
		color: #fff;
	}
	.donut-label {
		font-size: 10px;
		color: rgba(255, 255, 255, 0.3);
	}

	.unlocked-count {
		text-align: center;
	}
	.unlocked-count .val {
		font-size: 18px;
		font-weight: 600;
		display: block;
		color: #fff;
	}
	.unlocked-count .lbl {
		font-size: 10px;
		color: rgba(255, 255, 255, 0.3);
	}

	.podium h2 {
		font-size: 11px;
		font-weight: 600;
		color: rgba(255, 255, 255, 0.3);
		margin-bottom: 16px;
	}
	.podium-list {
		display: flex;
		flex-direction: column;
		gap: 12px;
	}
	.podium-item {
		display: flex;
		align-items: center;
		gap: 12px;
	}
	.podium-item .rank {
		width: 24px;
		height: 24px;
		border-radius: 12px;
		display: flex;
		align-items: center;
		justify-content: center;
		font-size: 10px;
		font-weight: 700;
		background: rgba(200, 169, 110, 0.1);
		color: var(--accent, #c8a96e);
	}
	.podium-item .name {
		flex: 1;
		font-size: 13px;
		color: #e0e0e0;
		white-space: nowrap;
		overflow: hidden;
		text-overflow: ellipsis;
	}
	.podium-item .pct {
		font-size: 12px;
		font-weight: 600;
		color: #fff;
		border-bottom: 2px solid var(--accent, #c8a96e);
	}

	.card-subtitle {
		font-size: 11px;
		font-weight: 600;
		color: rgba(255, 255, 255, 0.3);
		display: flex;
		align-items: center;
		gap: 6px;
		margin-bottom: 12px;
	}
	.rarity-item {
		display: flex;
		align-items: center;
		gap: 12px;
	}
	.rarity-item img {
		width: 40px;
		height: 40px;
		border-radius: 8px;
	}
	.rarity-icon-placeholder {
		width: 40px;
		height: 40px;
		border-radius: 8px;
		background: rgba(200, 169, 110, 0.1);
		display: flex;
		align-items: center;
		justify-content: center;
		font-size: 18px;
		flex-shrink: 0;
	}
	.rarity-item .info {
		flex: 1;
		display: flex;
		flex-direction: column;
		min-width: 0;
	}
	.rarity-item .name {
		font-size: 14px;
		font-weight: 600;
		color: #fff;
		white-space: nowrap;
		overflow: hidden;
		text-overflow: ellipsis;
	}
	.rarity-item .game {
		font-size: 11px;
		color: rgba(255, 255, 255, 0.3);
		white-space: nowrap;
		overflow: hidden;
		text-overflow: ellipsis;
	}
	.rarity-badge {
		font-size: 11px;
		font-weight: 700;
		color: #f4b860;
		background: rgba(244, 184, 96, 0.1);
		padding: 2px 6px;
		border-radius: 4px;
	}

	.best-week-val .val {
		font-size: 32px;
		font-weight: 700;
		color: #fff;
		display: block;
	}
	.best-week-val .lbl {
		font-size: 11px;
		color: rgba(255, 255, 255, 0.3);
	}

	.activity-view {
		display: flex;
		flex-direction: column;
		gap: 16px;
	}
	.heatmap-card .sub {
		font-size: 12px;
		color: rgba(255, 255, 255, 0.3);
	}
	.heatmap-container {
		margin-top: 20px;
	}

	.heatmap-wrapper {
		display: flex;
		gap: 8px;
		position: relative;
	}

	.heatmap-y-labels {
		display: flex;
		flex-direction: column;
		justify-content: space-between;
		padding: 18px 0 2px;
		font-size: 9px;
		color: rgba(255, 255, 255, 0.2);
		height: 77px;
		margin-top: 18px;
	}

	.heatmap-scroll-area {
		flex: 1;
		display: flex;
		flex-direction: column;
		gap: 4px;
		overflow-x: auto;
		padding-bottom: 4px;
	}

	.heatmap-months {
		position: relative;
		height: 18px;
		font-size: 10px;
		color: rgba(255, 255, 255, 0.2);
	}

	.month-label {
		position: absolute;
		white-space: nowrap;
	}

	.heatmap-grid {
		display: flex;
		gap: 3px;
	}
	.heatmap-col {
		display: flex;
		flex-direction: column;
		gap: 3px;
	}
	.heatmap-cell {
		width: 10px;
		height: 10px;
		background: rgba(255, 255, 255, 0.05);
		border-radius: 2px;
		transition: transform 0.1s;
	}
	.heatmap-cell:hover:not(.is-future) {
		transform: scale(1.2);
		z-index: 10;
		outline: 1px solid rgba(255, 255, 255, 0.2);
	}
	.heatmap-cell.lvl1 {
		background: color-mix(in srgb, var(--accent, #c8a96e) 20%, transparent);
	}
	.heatmap-cell.lvl2 {
		background: color-mix(in srgb, var(--accent, #c8a96e) 40%, transparent);
	}
	.heatmap-cell.lvl3 {
		background: color-mix(in srgb, var(--accent, #c8a96e) 60%, transparent);
	}
	.heatmap-cell.lvl4 {
		background: color-mix(in srgb, var(--accent, #c8a96e) 80%, transparent);
	}
	.heatmap-cell.lvl5 {
		background: var(--accent, #c8a96e);
	}
	.heatmap-cell.is-future {
		opacity: 0.1;
		cursor: default;
	}

	.heatmap-legend {
		display: flex;
		align-items: center;
		gap: 6px;
		font-size: 10px;
		color: rgba(255, 255, 255, 0.3);
		margin-top: 12px;
		justify-content: flex-end;
	}

	.charts-row {
		display: grid;
		grid-template-columns: 1fr 1fr;
		gap: 16px;
	}
	.chart-card .sub {
		font-size: 11px;
		color: rgba(255, 255, 255, 0.3);
		margin: 4px 0 20px;
	}

	.chart-header-row {
		display: flex;
		justify-content: space-between;
		align-items: flex-start;
		margin-bottom: 20px;
	}

	.chart-nav-group {
		display: flex;
		align-items: center;
		gap: 12px;
	}

	.window-nav {
		display: flex;
		gap: 2px;
		background: rgba(255, 255, 255, 0.03);
		padding: 2px;
		border-radius: 6px;
	}

	.window-nav button {
		background: transparent;
		border: none;
		color: rgba(255, 255, 255, 0.3);
		width: 24px;
		height: 24px;
		display: flex;
		align-items: center;
		justify-content: center;
		border-radius: 4px;
		cursor: pointer;
		transition: all 0.2s;
	}

	.window-nav button:hover:not(:disabled) {
		background: rgba(255, 255, 255, 0.05);
		color: #fff;
	}

	.window-nav button:disabled {
		opacity: 0.1;
		cursor: default;
	}

	.timeframe-selector {
		display: flex;
		gap: 4px;
		background: rgba(255, 255, 255, 0.03);
		padding: 2px;
		border-radius: 6px;
	}

	.timeframe-selector button {
		background: transparent;
		border: none;
		color: rgba(255, 255, 255, 0.3);
		padding: 4px 8px;
		border-radius: 4px;
		font-size: 10px;
		font-weight: 600;
		cursor: pointer;
		transition: all 0.2s;
		text-transform: uppercase;
	}

	.timeframe-selector button:hover {
		color: rgba(255, 255, 255, 0.6);
	}

	.timeframe-selector button.active {
		background: rgba(255, 255, 255, 0.07);
		color: var(--accent, #c8a96e);
	}

	.chart-container {
		height: 180px;
		position: relative;
		padding-bottom: 20px;
		border-bottom: 1px solid rgba(255, 255, 255, 0.05);
	}

	.chart-point {
		cursor: pointer;
		transition:
			r 0.2s,
			opacity 0.2s;
		opacity: 0.4;
	}
	.chart-point:hover {
		r: 4;
		opacity: 1;
	}
	.bar-chart {
		display: flex;
		align-items: flex-end;
		gap: 4px;
		width: 100%;
		height: 100%;
		justify-content: space-between;
	}
	.bar {
		flex: 1;
		border-radius: 2px 2px 0 0;
		background: color-mix(in srgb, var(--accent, #c8a96e) 25%, transparent);
		transition: all 0.2s;
		cursor: pointer;
	}
	.bar:hover {
		background: color-mix(in srgb, var(--accent, #c8a96e) 50%, transparent);
	}
	.bar.empty {
		background: rgba(255, 255, 255, 0.05);
	}
	.bar.max {
		background: #3ddc84;
	}
	.bar.max:hover {
		background: #4ef095;
	}

	.recent-list {
		flex: 1;
	}
	.recent-scroll {
		display: flex;
		flex-direction: column;
		gap: 16px;
		margin-top: 12px;
	}
	.recent-item {
		display: flex;
		align-items: flex-start;
		gap: 16px;
		padding-bottom: 16px;
		border-bottom: 1px solid rgba(255, 255, 255, 0.03);
	}
	.recent-item img {
		width: 48px;
		height: 48px;
		border-radius: 8px;
		flex-shrink: 0;
	}
	.recent-item .info {
		flex: 1;
		display: flex;
		flex-direction: column;
		min-width: 0;
	}

	.name-row {
		display: flex;
		align-items: center;
		gap: 8px;
		margin-bottom: 2px;
	}
	.recent-item .name {
		font-size: 15px;
		font-weight: 600;
		color: #fff;
	}
	.rarity-badge.small {
		font-size: 9px;
		padding: 1px 4px;
	}

	.recent-item .game {
		font-size: 12px;
		color: var(--accent, #c8a96e);
		font-weight: 500;
		margin-bottom: 4px;
	}
	.recent-item .desc {
		font-size: 13px;
		color: rgba(255, 255, 255, 0.4);
		line-height: 1.4;
		display: -webkit-box;
		-webkit-line-clamp: 2;
		line-clamp: 2;
		-webkit-box-orient: vertical;
		overflow: hidden;
	}
	.recent-item .time {
		font-size: 11px;
		color: rgba(255, 255, 255, 0.2);
		white-space: nowrap;
		margin-top: 4px;
	}

	.custom-tooltip {
		position: fixed;
		background: #1e2329;
		border: 1px solid rgba(255, 255, 255, 0.1);
		padding: 12px;
		border-radius: 8px;
		z-index: 1000;
		pointer-events: none;
		box-shadow: 0 8px 24px rgba(0, 0, 0, 0.5);
		min-width: 150px;
		max-width: 250px;
	}

	.tooltip-title {
		font-size: 11px;
		color: rgba(255, 255, 255, 0.4);
		margin-bottom: 4px;
	}
	.tooltip-value {
		font-size: 16px;
		font-weight: 700;
		color: #fff;
		margin-bottom: 8px;
	}
	.tooltip-sub {
		font-size: 12px;
		color: rgba(255, 255, 255, 0.6);
		margin-bottom: 8px;
	}

	.tooltip-list {
		list-style: none;
		padding: 0;
		margin: 0;
		border-top: 1px solid rgba(255, 255, 255, 0.05);
		padding-top: 8px;
	}

	.tooltip-list li {
		font-size: 12px;
		color: rgba(255, 255, 255, 0.7);
		margin-bottom: 4px;
		white-space: nowrap;
		overflow: hidden;
		text-overflow: ellipsis;
	}

	.tooltip-list li.more {
		color: rgba(255, 255, 255, 0.3);
		font-style: italic;
	}

	.empty {
		font-size: 12px;
		color: rgba(255, 255, 255, 0.2);
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
