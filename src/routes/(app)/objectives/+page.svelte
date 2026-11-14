<script lang="ts">
	// src/routes/(app)/objectives/+page.svelte
	import { onDestroy, onMount, tick } from 'svelte';
	import { get } from 'svelte/store';
	import { invoke } from '@tauri-apps/api/core';
	import {
		games,
		computeGameProgress,
		computeRemainingCompletionistHours,
		formatRemainingTimeHours,
		getGameIconUrl,
		type Achievement,
		type Game,
		type HltbData
	} from '$lib/stores/Games.js';
	import { goto } from '$app/navigation';
	import { resolve } from '$app/paths';
	import { i18n } from '$lib/stores/i18n.js';
	import { achievementJumpIntent, searchQuery, settingsOpen } from '$lib/stores/ui.js';
	import { selectedGameId } from '$lib/stores/selectedGame.js';

	type SortMode = 'progression' | 'remaining' | 'remaining_time';
	type RouletteMode = 'all' | 'game' | 'achievement';

	type RouletteGameCandidate = {
		id: string;
		kind: 'game';
		game: Game;
		unlockedCount: number;
		totalCount: number;
		pct: number;
		remainingHours: number | null;
	};

	type RouletteAchievementCandidate = {
		id: string;
		kind: 'achievement';
		game: Game;
		achievement: Achievement;
		reasonType: 'progress' | 'accessible' | 'game';
		reasonPct: number;
		progressCurrent: number | null;
		progressMax: number | null;
		unlockedCount: number;
		totalCount: number;
		gamePct: number;
		completionPct: number;
	};

	type RouletteCandidate = RouletteGameCandidate | RouletteAchievementCandidate;

	const SPIN_STEP_DELAYS_MS = [55, 62, 70, 79, 89, 101, 114, 129, 146, 165, 186, 208];

	let sortBy = $state<SortMode>('progression');
	let hltbMap = $state<Record<string, HltbData>>({});

	let rouletteOpen = $state(false);
	let rouletteMode = $state<RouletteMode>('all');
	let isSpinning = $state(false);
	let spinJustLanded = $state(false);
	let displayedCandidate = $state<RouletteCandidate | null>(null);
	let selectedCandidate = $state<RouletteCandidate | null>(null);
	let failedCandidateId = $state<string | null>(null);

	let triggerBtnEl = $state<HTMLButtonElement | null>(null);
	let modalEl = $state<HTMLDivElement | null>(null);
	let lastPickedId: string | null = null;
	let spinTimerId: ReturnType<typeof setTimeout> | null = null;
	let spinGeneration = 0;

	onMount(() => {
		let cancelled = false;

		void (async () => {
			let initialMap: Record<string, HltbData> = {};
			try {
				const cached = await invoke<Record<string, HltbData>>('get_cached_hltb_data');
				if (cancelled) return;
				if (cached && typeof cached === 'object') {
					initialMap = cached;
					hltbMap = cached;
				}
			} catch {
				// Ignore cache load failure and still allow page to render
			}

			const missingCandidates = get(games)
				.map((g) => {
					const progress = computeGameProgress(g.achievements ?? [], g.achievement_groups);
					return {
						game: g,
						totalCount: progress.totalBase,
						isPlatinum: progress.isPlatinum,
						progression: progress.platinumPct
					};
				})
				.filter((item) => item.totalCount > 0 && !item.isPlatinum && !initialMap[item.game.id])
				.sort((a, b) => b.progression - a.progression)
				.slice(0, 25);

			for (const item of missingCandidates) {
				if (cancelled) return;
				try {
					const fetched = await invoke<HltbData>('get_hltb_data', {
						gameId: item.game.id,
						gameName: item.game.name,
						forceRefresh: false
					});
					if (cancelled) return;
					if (fetched) {
						hltbMap = { ...hltbMap, [item.game.id]: fetched };
					}
				} catch {
					// Skip individual HLTB lookup errors cleanly
				}
			}
		})();

		return () => {
			cancelled = true;
		};
	});

	let incompleteGames = $derived.by(() =>
		$games
			.map((g) => {
				const progress = computeGameProgress(g.achievements ?? [], g.achievement_groups);
				const remaining = progress.totalBase - progress.unlockedBase;
				const remainingHours = computeRemainingCompletionistHours(
					hltbMap[g.id]?.completionist_hours,
					g.playtime_seconds ?? 0
				);
				return {
					...g,
					unlockedCount: progress.unlockedBase,
					totalCount: progress.totalBase,
					remainingCount: remaining,
					progression: progress.platinumPct,
					isPlatinum: progress.isPlatinum,
					remainingHours
				};
			})
			.filter((g) => g.totalCount > 0 && !g.isPlatinum)
	);

	let sortedGames = $derived.by(() => {
		const mapped = [...incompleteGames];
		if (sortBy === 'progression') {
			return mapped.sort((a, b) => b.progression - a.progression);
		}
		if (sortBy === 'remaining_time') {
			return mapped.sort((a, b) => {
				if (a.remainingHours !== null && b.remainingHours !== null) {
					return a.remainingHours - b.remainingHours || a.remainingCount - b.remainingCount;
				}
				if (a.remainingHours !== null) return -1;
				if (b.remainingHours !== null) return 1;
				return b.progression - a.progression;
			});
		}
		return mapped.sort((a, b) => a.remainingCount - b.remainingCount);
	});

	let fastestGames = $derived.by(() =>
		incompleteGames
			.filter((g): g is typeof g & { remainingHours: number } => g.remainingHours !== null)
			.sort((a, b) => a.remainingHours - b.remainingHours || a.remainingCount - b.remainingCount)
			.slice(0, 6)
	);

	let easyAchievements = $derived.by(() => {
		const locked = $games.flatMap((g) => {
			const progress = computeGameProgress(g.achievements ?? [], g.achievement_groups);
			return progress.sections
				.filter((s) => !s.archived)
				.flatMap((s) =>
					s.achievements
						.filter((a) => !a.unlocked && !a.archived)
						.map((a) => ({ ...a, gameName: g.name }))
				);
		});
		return locked
			.sort(
				(a, b) =>
					(parseFloat(b.completionpercentage) || 0) - (parseFloat(a.completionpercentage) || 0)
			)
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

		if (game.steam_id) {
			return `https://cdn.cloudflare.steamstatic.com/steam/apps/${game.steam_id}/header.jpg`;
		}

		return '';
	}

	let roulettePools = $derived.by(() => {
		const startedGames: RouletteGameCandidate[] = [];
		const unstartedGames: RouletteGameCandidate[] = [];
		const primaryAchievements: RouletteAchievementCandidate[] = [];
		const startedFallbackAchievements: RouletteAchievementCandidate[] = [];
		const bestPerIncompleteGame = Object.create(null) as Record<
			string,
			RouletteAchievementCandidate
		>;
		const seenAchievementIds = Object.create(null) as Record<string, true>;

		for (const game of $games) {
			const progress = computeGameProgress(game.achievements ?? [], game.achievement_groups);
			if (progress.totalBase <= 0 || progress.isPlatinum) continue;

			const isStarted = progress.unlockedBase > 0;
			const remainingHours = computeRemainingCompletionistHours(
				hltbMap[game.id]?.completionist_hours,
				game.playtime_seconds ?? 0
			);
			const gameCandidate: RouletteGameCandidate = {
				id: `game:${game.id}`,
				kind: 'game',
				game,
				unlockedCount: progress.unlockedBase,
				totalCount: progress.totalBase,
				pct: progress.platinumPct,
				remainingHours
			};

			if (isStarted) {
				startedGames.push(gameCandidate);
			} else {
				unstartedGames.push(gameCandidate);
			}

			for (const section of progress.sections) {
				if (section.archived) continue;
				for (const ach of section.achievements) {
					if (ach.unlocked || ach.archived) continue;
					const candidateId = `ach:${game.id}:${ach.key}`;
					if (seenAchievementIds[candidateId]) continue;
					seenAchievementIds[candidateId] = true;

					const maxProg = ach.max_progress;
					const hasTracked =
						typeof maxProg === 'number' && Number.isFinite(maxProg) && maxProg > 0;
					const rawCurProg =
						typeof ach.progress === 'number' && Number.isFinite(ach.progress) ? ach.progress : 0;
					const ratio = hasTracked ? rawCurProg / maxProg : 0;
					const meetsProgress = hasTracked && ratio >= 0.5;

					const parsedCompletion = parseFloat(ach.completionpercentage);
					const completionPct = Number.isFinite(parsedCompletion) ? parsedCompletion : 0;
					const meetsAccessible = isStarted && completionPct >= 25;

					const commonFields = {
						id: candidateId,
						kind: 'achievement' as const,
						game,
						achievement: ach,
						progressCurrent: hasTracked ? Math.max(0, rawCurProg) : null,
						progressMax: hasTracked ? maxProg : null,
						unlockedCount: progress.unlockedBase,
						totalCount: progress.totalBase,
						gamePct: Math.round(progress.platinumPct),
						completionPct
					};

					if (meetsProgress || meetsAccessible) {
						primaryAchievements.push({
							...commonFields,
							reasonType: meetsProgress ? 'progress' : 'accessible',
							reasonPct: meetsProgress
								? Math.min(100, Math.round(ratio * 100))
								: Number(completionPct.toFixed(1))
						});
						continue;
					}

					const fallbackCandidate: RouletteAchievementCandidate = {
						...commonFields,
						reasonType: completionPct > 0 ? 'accessible' : 'game',
						reasonPct:
							completionPct > 0 ? Number(completionPct.toFixed(1)) : Math.round(progress.platinumPct)
					};

					if (isStarted) {
						startedFallbackAchievements.push(fallbackCandidate);
					}

					const existingBest = bestPerIncompleteGame[game.id];
					if (!existingBest || fallbackCandidate.completionPct > existingBest.completionPct) {
						bestPerIncompleteGame[game.id] = fallbackCandidate;
					}
				}
			}
		}

		const gameCandidates = startedGames.length > 0 ? startedGames : unstartedGames;

		let achievementCandidates: RouletteAchievementCandidate[];
		if (primaryAchievements.length > 0) {
			achievementCandidates = primaryAchievements;
		} else if (startedFallbackAchievements.length > 0) {
			achievementCandidates = [...startedFallbackAchievements]
				.sort((a, b) => b.completionPct - a.completionPct)
				.slice(0, 12);
		} else {
			achievementCandidates = Object.values(bestPerIncompleteGame)
				.sort((a, b) => b.completionPct - a.completionPct)
				.slice(0, 12);
		}

		return { gameCandidates, achievementCandidates };
	});

	let currentPoolCount = $derived.by(() => {
		const { gameCandidates, achievementCandidates } = roulettePools;
		if (rouletteMode === 'game') return gameCandidates.length;
		if (rouletteMode === 'achievement') return achievementCandidates.length;
		return gameCandidates.length + achievementCandidates.length;
	});

	function clearSpinTimer(): void {
		if (spinTimerId !== null) {
			clearTimeout(spinTimerId);
			spinTimerId = null;
		}
	}

	function cancelSpin(): void {
		clearSpinTimer();
		spinGeneration += 1;
		isSpinning = false;
	}

	onDestroy(() => {
		cancelSpin();
	});

	function pickRandomItem<T>(items: readonly T[]): T {
		return items[Math.floor(Math.random() * items.length)];
	}

	function pickWinner(
		mode: RouletteMode,
		gamePool: readonly RouletteGameCandidate[],
		achPool: readonly RouletteAchievementCandidate[],
		prevId: string | null
	): RouletteCandidate | null {
		if (mode === 'game') {
			if (gamePool.length === 0) return null;
			const eligible =
				gamePool.length > 1 && prevId ? gamePool.filter((c) => c.id !== prevId) : gamePool;
			return pickRandomItem(eligible.length > 0 ? eligible : gamePool);
		}

		if (mode === 'achievement') {
			if (achPool.length === 0) return null;
			const eligible =
				achPool.length > 1 && prevId ? achPool.filter((c) => c.id !== prevId) : achPool;
			return pickRandomItem(eligible.length > 0 ? eligible : achPool);
		}

		const total = gamePool.length + achPool.length;
		if (total === 0) return null;
		if (total === 1) return gamePool[0] ?? achPool[0] ?? null;

		const eligibleGames = prevId ? gamePool.filter((c) => c.id !== prevId) : gamePool;
		const eligibleAchs = prevId ? achPool.filter((c) => c.id !== prevId) : achPool;

		if (eligibleGames.length > 0 && eligibleAchs.length > 0) {
			return Math.random() < 0.5
				? pickRandomItem(eligibleGames)
				: pickRandomItem(eligibleAchs);
		}
		if (eligibleGames.length > 0) return pickRandomItem(eligibleGames);
		if (eligibleAchs.length > 0) return pickRandomItem(eligibleAchs);

		return pickRandomItem([...gamePool, ...achPool]);
	}

	function buildPreviewPool(
		mode: RouletteMode,
		gamePool: readonly RouletteGameCandidate[],
		achPool: readonly RouletteAchievementCandidate[]
	): RouletteCandidate[] {
		if (mode === 'game') return [...gamePool];
		if (mode === 'achievement') return [...achPool];

		const mixed: RouletteCandidate[] = [];
		const maxLen = Math.max(gamePool.length, achPool.length);
		for (let i = 0; i < maxLen; i += 1) {
			if (i < gamePool.length) mixed.push(gamePool[i]);
			if (i < achPool.length) mixed.push(achPool[i]);
		}
		return mixed;
	}

	function buildPreviewSequence(
		pool: readonly RouletteCandidate[],
		winner: RouletteCandidate,
		steps: number
	): RouletteCandidate[] {
		const sequence: RouletteCandidate[] = [];
		let prevId: string | null = null;

		for (let i = 0; i < steps; i += 1) {
			const isLastPreview = i === steps - 1;
			let choices = pool.filter(
				(c) => c.id !== prevId && (!isLastPreview || c.id !== winner.id)
			);
			if (choices.length === 0 && isLastPreview) {
				choices = pool.filter((c) => c.id !== winner.id);
			}
			if (choices.length === 0) {
				choices = pool.filter((c) => c.id !== prevId);
			}
			const chosen = pickRandomItem(choices.length > 0 ? choices : pool);
			sequence.push(chosen);
			prevId = chosen.id;
		}

		return sequence;
	}

	function startSpin(mode: RouletteMode = rouletteMode): void {
		clearSpinTimer();
		spinGeneration += 1;
		const currentGen = spinGeneration;

		spinJustLanded = false;
		failedCandidateId = null;

		const { gameCandidates, achievementCandidates } = roulettePools;
		const winner = pickWinner(mode, gameCandidates, achievementCandidates, lastPickedId);

		if (!winner) {
			isSpinning = false;
			displayedCandidate = null;
			selectedCandidate = null;
			return;
		}

		const prefersReducedMotion =
			typeof window !== 'undefined' &&
			window.matchMedia?.('(prefers-reduced-motion: reduce)').matches === true;

		if (prefersReducedMotion) {
			isSpinning = false;
			displayedCandidate = winner;
			selectedCandidate = winner;
			lastPickedId = winner.id;
			spinJustLanded = true;
			return;
		}

		const previewPool = buildPreviewPool(mode, gameCandidates, achievementCandidates);
		if (previewPool.length <= 1) {
			isSpinning = true;
			selectedCandidate = null;
			displayedCandidate = winner;
			spinTimerId = setTimeout(() => {
				if (currentGen !== spinGeneration) return;
				spinTimerId = null;
				displayedCandidate = winner;
				selectedCandidate = winner;
				lastPickedId = winner.id;
				isSpinning = false;
				spinJustLanded = true;
			}, 260);
			return;
		}

		const previews = buildPreviewSequence(previewPool, winner, SPIN_STEP_DELAYS_MS.length);
		isSpinning = true;
		selectedCandidate = null;
		displayedCandidate = previews[0];

		const scheduleStep = (stepIndex: number): void => {
			const delayMs = SPIN_STEP_DELAYS_MS[stepIndex];
			spinTimerId = setTimeout(() => {
				if (currentGen !== spinGeneration) return;
				const nextIndex = stepIndex + 1;
				if (nextIndex < SPIN_STEP_DELAYS_MS.length) {
					failedCandidateId = null;
					displayedCandidate = previews[nextIndex];
					scheduleStep(nextIndex);
					return;
				}
				spinTimerId = null;
				failedCandidateId = null;
				displayedCandidate = winner;
				selectedCandidate = winner;
				lastPickedId = winner.id;
				isSpinning = false;
				spinJustLanded = true;
			}, delayMs);
		};

		scheduleStep(0);
	}

	function openRoulette(): void {
		rouletteOpen = true;
		startSpin(rouletteMode);
		void tick().then(() => {
			if (rouletteOpen) modalEl?.focus();
		});
	}

	function closeRoulette(): void {
		cancelSpin();
		rouletteOpen = false;
		spinJustLanded = false;
		void tick().then(() => {
			triggerBtnEl?.focus();
		});
	}

	function switchRouletteMode(mode: RouletteMode): void {
		if (rouletteMode === mode) return;
		rouletteMode = mode;
		startSpin(mode);
	}

	function openGame(gameId: string): void {
		searchQuery.set('');
		achievementJumpIntent.set(null);
		selectedGameId.set(gameId);
		void goto(resolve('/(app)/games/[id]', { id: gameId }));
	}

	function openRouletteTarget(candidate: RouletteCandidate): void {
		cancelSpin();
		rouletteOpen = false;
		spinJustLanded = false;
		if (candidate.kind === 'game') {
			openGame(candidate.game.id);
			return;
		}

		searchQuery.set('');
		selectedGameId.set(candidate.game.id);
		achievementJumpIntent.set({
			gameId: candidate.game.id,
			achievementKey: candidate.achievement.key,
			token: Date.now()
		});
		void goto(resolve('/(app)/games/[id]', { id: candidate.game.id }));
	}

	function formatProgressVal(n: number): string {
		return Number.isInteger(n) ? String(n) : n.toFixed(1);
	}

	function getCandidateArt(candidate: RouletteCandidate): string {
		if (candidate.kind === 'game') {
			return getGameIcon(candidate.game) || getGameIconUrl(candidate.game);
		}
		return (
			candidate.achievement.icon ||
			candidate.achievement.icon_gray ||
			getGameIcon(candidate.game) ||
			getGameIconUrl(candidate.game)
		);
	}

	function handleCandidateArtError(event: Event, candidate: RouletteCandidate): void {
		const target = event.currentTarget as HTMLImageElement;
		const step = Number(target.dataset.fallbackStep ?? '0');

		if (candidate.kind === 'game') {
			if (step < 1 && candidate.game.steam_id && !target.src.includes('header.jpg')) {
				target.dataset.fallbackStep = '1';
				target.src = `https://cdn.cloudflare.steamstatic.com/steam/apps/${candidate.game.steam_id}/header.jpg`;
				return;
			}
			failedCandidateId = candidate.id;
			return;
		}

		if (step < 1 && candidate.achievement.icon_gray && target.src !== candidate.achievement.icon_gray) {
			target.dataset.fallbackStep = '1';
			target.src = candidate.achievement.icon_gray;
			return;
		}

		if (step < 2) {
			const gameArt = getGameIcon(candidate.game);
			if (gameArt && target.src !== gameArt) {
				target.dataset.fallbackStep = '2';
				target.src = gameArt;
				return;
			}
		}

		failedCandidateId = candidate.id;
	}

	function handleWindowKeydown(event: KeyboardEvent): void {
		if (event.key === 'Escape' && rouletteOpen && !$settingsOpen) {
			event.preventDefault();
			closeRoulette();
		}
	}
</script>

<svelte:window onkeydown={handleWindowKeydown} />

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
			<button
				type="button"
				class="roulette-trigger-btn"
				bind:this={triggerBtnEl}
				onclick={openRoulette}
			>
				<svg
					width="16"
					height="16"
					viewBox="0 0 24 24"
					fill="none"
					stroke="currentColor"
					stroke-width="2"
					stroke-linecap="round"
					stroke-linejoin="round"
					aria-hidden="true"
				>
					<rect width="12" height="12" x="2" y="10" rx="2" ry="2" />
					<path d="m17.92 14 3.5-3.5a2.24 2.24 0 0 0 0-3l-5-4.92a2.24 2.24 0 0 0-3 0L10 6" />
					<path d="M6 18h.01" />
					<path d="M10 14h.01" />
					<path d="M15 6h.01" />
					<path d="M18 9h.01" />
				</svg>
				<span>{$i18n.t('objectives.roulette.button')}</span>
			</button>
			<div class="sort-toggle">
				<button
					type="button"
					class:active={sortBy === 'progression'}
					onclick={() => (sortBy = 'progression')}
					>{$i18n.t('objectives.sort.progression')}</button
				>
				<button
					type="button"
					class:active={sortBy === 'remaining'}
					onclick={() => (sortBy = 'remaining')}
					>{$i18n.t('objectives.sort.remaining')}</button
				>
				<button
					type="button"
					class:active={sortBy === 'remaining_time'}
					onclick={() => (sortBy = 'remaining_time')}
					>{$i18n.t('objectives.sort.remainingTime')}</button
				>
			</div>
		</div>
	</header>

	<div class="content scrollable">
		{#if fastestGames.length > 0}
			<section class="fastest-section">
				<div class="section-header">
					<h2>{$i18n.t('objectives.fastestSection')}</h2>
					<span class="sub">{$i18n.t('objectives.fastestSubtitle')}</span>
				</div>
				<div class="fastest-grid">
					{#each fastestGames as game (game.id)}
						<div
							class="fastest-card"
							role="button"
							tabindex="0"
							onclick={() => openGame(game.id)}
							onkeydown={(event) => {
								if (event.key === 'Enter' || event.key === ' ') {
									event.preventDefault();
									openGame(game.id);
								}
							}}
						>
							<img
								src={getGameIcon(game)}
								alt={game.name}
								class="game-thumb"
								loading="lazy"
								decoding="async"
								onerror={(e) => {
									const t = e.target as HTMLImageElement;
									if (game.steam_id && !t.src.includes('header.jpg')) {
										t.src = `https://cdn.cloudflare.steamstatic.com/steam/apps/${game.steam_id}/header.jpg`;
									}
								}}
							/>
							<div class="fastest-info">
								<span class="fastest-name">{game.name}</span>
								<div class="fastest-meta">
									<span class="time-badge">
										{$i18n.t('objectives.remainingTimeBadge', {
											time: formatRemainingTimeHours(game.remainingHours)
										})}
									</span>
									<span class="fastest-pct tabular-nums">{game.progression.toFixed(0)}%</span>
								</div>
							</div>
						</div>
					{/each}
				</div>
			</section>
		{/if}

		<section class="easy-section">
			<div class="section-header">
				<h2>{$i18n.t('objectives.easy')}</h2>
				<span class="sub">{$i18n.t('objectives.easySubtitle')}</span>
			</div>
			<div class="easy-grid">
				{#each easyAchievements as a (a.gameName + ':' + a.key)}
					<div class="easy-card">
						<img src={a.icon_gray || a.icon} alt={a.name} loading="lazy" decoding="async" />
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
				{#each sortedGames as game, i (game.id)}
					<div
						class="game-objective-card"
						role="button"
						tabindex="0"
						onclick={() => openGame(game.id)}
						onkeydown={(event) => {
							if (event.key === 'Enter' || event.key === ' ') {
								event.preventDefault();
								openGame(game.id);
							}
						}}
					>
						<span class="rank">{i + 1}</span>
						<img
							src={getGameIcon(game)}
							alt={game.name}
							class="game-thumb"
							loading="lazy"
							decoding="async"
							onerror={(e) => {
								const t = e.target as HTMLImageElement;
								if (game.steam_id && !t.src.includes('header.jpg')) {
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
								{#if game.remainingHours !== null}
									<span class="time-badge">
										{$i18n.t('objectives.remainingTimeBadge', {
											time: formatRemainingTimeHours(game.remainingHours)
										})}
									</span>
								{/if}
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

{#if rouletteOpen}
	<div class="roulette-overlay">
		<button
			type="button"
			class="roulette-backdrop"
			aria-label={$i18n.t('objectives.roulette.close')}
			onclick={closeRoulette}
		></button>
		<div
			class="roulette-modal"
			role="dialog"
			aria-modal="true"
			aria-labelledby="roulette-modal-title"
			tabindex="-1"
			bind:this={modalEl}
		>
			<header class="roulette-header">
				<div class="roulette-header-copy">
					<div class="roulette-title-row">
						<span class="roulette-badge-icon" aria-hidden="true">
							<svg
								width="18"
								height="18"
								viewBox="0 0 24 24"
								fill="none"
								stroke="currentColor"
								stroke-width="2"
								stroke-linecap="round"
								stroke-linejoin="round"
							>
								<rect width="12" height="12" x="2" y="10" rx="2" ry="2" />
								<path d="m17.92 14 3.5-3.5a2.24 2.24 0 0 0 0-3l-5-4.92a2.24 2.24 0 0 0-3 0L10 6" />
								<path d="M6 18h.01" />
								<path d="M10 14h.01" />
								<path d="M15 6h.01" />
								<path d="M18 9h.01" />
							</svg>
						</span>
						<h2 id="roulette-modal-title">{$i18n.t('objectives.roulette.title')}</h2>
					</div>
					<p class="roulette-subtitle">{$i18n.t('objectives.roulette.subtitle')}</p>
				</div>
				<button
					type="button"
					class="roulette-close-btn"
					aria-label={$i18n.t('objectives.roulette.close')}
					onclick={closeRoulette}
				>
					<svg
						width="16"
						height="16"
						viewBox="0 0 24 24"
						fill="none"
						stroke="currentColor"
						stroke-width="2.2"
						stroke-linecap="round"
						stroke-linejoin="round"
						aria-hidden="true"
					>
						<path d="M18 6 6 18" />
						<path d="m6 6 12 12" />
					</svg>
				</button>
			</header>

			<div class="roulette-modes" role="group" aria-label={$i18n.t('objectives.roulette.title')}>
				<button
					type="button"
					class="roulette-mode-btn"
					class:active={rouletteMode === 'all'}
					aria-pressed={rouletteMode === 'all'}
					onclick={() => switchRouletteMode('all')}
				>
					{$i18n.t('objectives.roulette.modeAll')}
				</button>
				<button
					type="button"
					class="roulette-mode-btn"
					class:active={rouletteMode === 'game'}
					aria-pressed={rouletteMode === 'game'}
					onclick={() => switchRouletteMode('game')}
				>
					{$i18n.t('objectives.roulette.modeGame')}
				</button>
				<button
					type="button"
					class="roulette-mode-btn"
					class:active={rouletteMode === 'achievement'}
					aria-pressed={rouletteMode === 'achievement'}
					onclick={() => switchRouletteMode('achievement')}
				>
					{$i18n.t('objectives.roulette.modeAchievement')}
				</button>
			</div>

			<p class="sr-only" aria-live="polite">
				{#if isSpinning}
					{$i18n.t('objectives.roulette.spinning')}
				{:else if selectedCandidate}
					{selectedCandidate.kind === 'game'
						? selectedCandidate.game.name
						: `${selectedCandidate.achievement.name} — ${selectedCandidate.game.name}`}
				{:else}
					{$i18n.t('objectives.roulette.noCandidates')}
				{/if}
			</p>

			<div
				class="roulette-stage"
				class:is-spinning={isSpinning}
				class:is-landed={!isSpinning && spinJustLanded && selectedCandidate !== null}
			>
				{#if displayedCandidate}
					{@const candidate = displayedCandidate}
					{@const artUrl = getCandidateArt(candidate)}
					<div class="roulette-stage-top">
						<span class="roulette-status-pill" class:spinning={isSpinning}>
							{#if isSpinning}
								<span class="spin-dot" aria-hidden="true"></span>
								{$i18n.t('objectives.roulette.spinning')}
							{:else if candidate.kind === 'game'}
								{$i18n.t('objectives.roulette.typeGame')}
							{:else}
								{$i18n.t('objectives.roulette.typeAchievement')}
							{/if}
						</span>
						{#if candidate.kind === 'game' && candidate.remainingHours !== null}
							<span class="time-badge">
								{$i18n.t('objectives.remainingTimeBadge', {
									time: formatRemainingTimeHours(candidate.remainingHours)
								})}
							</span>
						{:else if candidate.kind === 'achievement'}
							<span class="roulette-game-chip">
								{candidate.game.name}
							</span>
						{/if}
					</div>

					<div class="roulette-card" class:tick-cycling={isSpinning}>
						<div class="roulette-art-frame" class:achievement-art={candidate.kind === 'achievement'}>
							{#key candidate.id}
								{#if artUrl && failedCandidateId !== candidate.id}
									<img
										src={artUrl}
										alt={candidate.kind === 'game'
											? candidate.game.name
											: candidate.achievement.name}
										decoding="async"
										data-fallback-step="0"
										onerror={(event) => handleCandidateArtError(event, candidate)}
									/>
								{:else}
									<div class="roulette-art-fallback" aria-hidden="true">
										<svg
											width="26"
											height="26"
											viewBox="0 0 24 24"
											fill="none"
											stroke="currentColor"
											stroke-width="1.8"
											stroke-linecap="round"
											stroke-linejoin="round"
										>
											{#if candidate.kind === 'game'}
												<line x1="6" x2="10" y1="11" y2="11" />
												<line x1="8" x2="8" y1="9" y2="13" />
												<line x1="15" x2="15.01" y1="12" y2="12" />
												<line x1="18" x2="18.01" y1="10" y2="10" />
												<path
													d="M17.32 5H6.68a4 4 0 0 0-3.978 3.59c-.006.052-.01.101-.017.152C2.604 9.416 2 14.456 2 16a3 3 0 0 0 3 3c1 0 1.5-.5 2-1l1.414-1.414A2 2 0 0 1 9.828 16h4.344a2 2 0 0 1 1.414.586L17 18c.5.5 1 1 2 1a3 3 0 0 0 3-3c0-1.545-.604-6.584-.685-7.258-.007-.05-.011-.1-.017-.151A4 4 0 0 0 17.32 5z"
												/>
											{:else}
												<path d="M6 9H4.5a2.5 2.5 0 0 1 0-5H6" />
												<path d="M18 9h1.5a2.5 2.5 0 0 0 0-5H18" />
												<path d="M4 22h16" />
												<path d="M10 14.66V17c0 .55-.47.98-.97 1.21C7.85 18.75 7 20.24 7 22" />
												<path d="M14 14.66V17c0 .55.47.98.97 1.21C16.15 18.75 17 20.24 17 22" />
												<path d="M18 2H6v7a6 6 0 0 0 12 0V2Z" />
											{/if}
										</svg>
									</div>
								{/if}
							{/key}
						</div>

						<div class="roulette-card-body">
							<h3 class="roulette-target-title">
								{candidate.kind === 'game' ? candidate.game.name : candidate.achievement.name}
							</h3>
							{#if candidate.kind === 'achievement' && candidate.achievement.desc}
								<p class="roulette-target-desc">{candidate.achievement.desc}</p>
							{/if}

							<div class="roulette-meta-row">
								<span class="roulette-reason-badge">
									{#if candidate.kind === 'game'}
										{$i18n.t('objectives.roulette.reasonGameStarted', {
											unlocked: candidate.unlockedCount,
											total: candidate.totalCount,
											pct: Math.round(candidate.pct)
										})}
									{:else if candidate.reasonType === 'progress'}
										{$i18n.t('objectives.roulette.reasonProgress', {
											pct: candidate.reasonPct
										})}
									{:else if candidate.reasonType === 'accessible'}
										{$i18n.t('objectives.roulette.reasonAccessible', {
											pct: candidate.reasonPct
										})}
									{:else}
										{$i18n.t('objectives.roulette.reasonGameStarted', {
											unlocked: candidate.unlockedCount,
											total: candidate.totalCount,
											pct: candidate.gamePct
										})}
									{/if}
								</span>

								{#if candidate.kind === 'achievement' && candidate.progressMax !== null && candidate.progressCurrent !== null}
									<span class="roulette-progress-counter tabular-nums">
										{formatProgressVal(candidate.progressCurrent)} / {formatProgressVal(
											candidate.progressMax
										)}
									</span>
								{/if}
							</div>

							{#if candidate.kind === 'game'}
								<div class="roulette-bar-track">
									<div
										class="roulette-bar-fill"
										style="width: {Math.min(100, Math.max(0, candidate.pct))}%"
									></div>
								</div>
							{:else if candidate.progressMax !== null && candidate.progressCurrent !== null}
								<div class="roulette-bar-track">
									<div
										class="roulette-bar-fill"
										style="width: {Math.min(
											100,
											Math.max(0, (candidate.progressCurrent / candidate.progressMax) * 100)
										)}%"
									></div>
								</div>
							{/if}
						</div>
					</div>
				{:else}
					<div class="roulette-empty">
						<svg
							width="28"
							height="28"
							viewBox="0 0 24 24"
							fill="none"
							stroke="currentColor"
							stroke-width="1.8"
							stroke-linecap="round"
							stroke-linejoin="round"
							aria-hidden="true"
						>
							<circle cx="12" cy="12" r="10" />
							<path d="M8 12h8" />
						</svg>
						<p>{$i18n.t('objectives.roulette.noCandidates')}</p>
					</div>
				{/if}
			</div>

			<footer class="roulette-actions">
				<button
					type="button"
					class="roulette-btn-secondary"
					disabled={currentPoolCount === 0}
					onclick={() => startSpin(rouletteMode)}
				>
					<svg
						width="15"
						height="15"
						viewBox="0 0 24 24"
						fill="none"
						stroke="currentColor"
						stroke-width="2"
						stroke-linecap="round"
						stroke-linejoin="round"
						class:spinning-icon={isSpinning}
						aria-hidden="true"
					>
						<path d="M21 12a9 9 0 0 0-9-9 9.75 9.75 0 0 0-6.74 2.74L3 8" />
						<path d="M3 3v5h5" />
						<path d="M3 12a9 9 0 0 0 9 9 9.75 9.75 0 0 0 6.74-2.74L21 16" />
						<path d="M16 16h5v5" />
					</svg>
					<span>{$i18n.t('objectives.roulette.respin')}</span>
				</button>

				<button
					type="button"
					class="roulette-btn-primary"
					disabled={isSpinning || !selectedCandidate}
					onclick={() => {
						if (selectedCandidate) openRouletteTarget(selectedCandidate);
					}}
				>
					<span>{$i18n.t('objectives.roulette.openTarget')}</span>
					<svg
						width="15"
						height="15"
						viewBox="0 0 24 24"
						fill="none"
						stroke="currentColor"
						stroke-width="2.2"
						stroke-linecap="round"
						stroke-linejoin="round"
						aria-hidden="true"
					>
						<path d="M5 12h14" />
						<path d="m12 5 7 7-7 7" />
					</svg>
				</button>
			</footer>
		</div>
	</div>
{/if}

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
		gap: 16px;
		flex-wrap: wrap;
	}

	.header-right {
		display: flex;
		align-items: center;
		gap: 12px;
		flex-wrap: wrap;
	}

	.roulette-trigger-btn {
		display: inline-flex;
		align-items: center;
		gap: 8px;
		padding: 8px 14px;
		border-radius: 10px;
		border: 1px solid color-mix(in srgb, var(--accent, #c8a96e) 38%, transparent);
		background: color-mix(in srgb, var(--accent, #c8a96e) 14%, var(--surface-2));
		color: var(--accent, #c8a96e);
		font-size: 13px;
		font-weight: 600;
		cursor: pointer;
		transition:
			background 0.2s,
			border-color 0.2s,
			transform 0.15s,
			box-shadow 0.2s;
	}

	.roulette-trigger-btn:hover {
		background: color-mix(in srgb, var(--accent, #c8a96e) 22%, var(--surface-2));
		border-color: color-mix(in srgb, var(--accent, #c8a96e) 65%, transparent);
		box-shadow: 0 6px 18px color-mix(in srgb, var(--accent, #c8a96e) 18%, transparent);
		transform: translateY(-1px);
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
		align-content: start;
		align-items: start;
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
		cursor: pointer;
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
		display: flex;
		align-items: center;
		flex-wrap: wrap;
		gap: 8px;
	}
	.time-badge {
		display: inline-flex;
		align-items: center;
		font-size: 11px;
		font-weight: 700;
		color: var(--accent, #c8a96e);
		background: color-mix(in srgb, var(--accent, #c8a96e) 14%, transparent);
		border: 1px solid color-mix(in srgb, var(--accent, #c8a96e) 32%, transparent);
		padding: 2px 8px;
		border-radius: 999px;
		white-space: nowrap;
	}

	.fastest-section {
		margin-bottom: 36px;
	}
	.fastest-grid {
		display: grid;
		grid-template-columns: repeat(auto-fill, minmax(290px, 1fr));
		gap: 12px;
	}
	.fastest-card {
		background: var(--surface-1);
		border: 1px solid var(--border-soft);
		border-radius: 14px;
		padding: 12px 14px;
		display: flex;
		align-items: center;
		gap: 12px;
		cursor: pointer;
		transition:
			background 0.2s,
			border-color 0.2s,
			transform 0.15s;
	}
	.fastest-card:hover {
		background: var(--surface-hover);
		border-color: color-mix(in srgb, var(--accent, #c8a96e) 40%, var(--border-soft));
		transform: translateY(-1px);
	}
	.fastest-info {
		flex: 1;
		min-width: 0;
		display: flex;
		flex-direction: column;
		gap: 5px;
	}
	.fastest-name {
		font-size: 14px;
		font-weight: 600;
		color: var(--text-primary);
		white-space: nowrap;
		overflow: hidden;
		text-overflow: ellipsis;
	}
	.fastest-meta {
		display: flex;
		align-items: center;
		justify-content: space-between;
		gap: 8px;
	}
	.fastest-pct {
		font-size: 11px;
		font-weight: 700;
		color: var(--text-muted);
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

	.sr-only {
		position: absolute;
		width: 1px;
		height: 1px;
		padding: 0;
		margin: -1px;
		overflow: hidden;
		clip: rect(0, 0, 0, 0);
		white-space: nowrap;
		border: 0;
	}

	.roulette-overlay {
		position: fixed;
		inset: 0;
		background: rgba(0, 0, 0, 0.58);
		backdrop-filter: blur(12px);
		-webkit-backdrop-filter: blur(12px);
		display: flex;
		align-items: center;
		justify-content: center;
		padding: 24px;
		z-index: 900;
		animation: roulette-fade-in 0.2s var(--spring-out, ease-out) both;
	}

	.roulette-backdrop {
		position: absolute;
		inset: 0;
		border: 0;
		padding: 0;
		margin: 0;
		background: transparent;
		cursor: default;
	}

	.roulette-modal {
		position: relative;
		z-index: 1;
		width: min(560px, 100%);
		max-height: min(88vh, 680px);
		overflow-y: auto;
		background: var(--glass-bg, var(--bg-panel));
		backdrop-filter: blur(22px) saturate(180%);
		-webkit-backdrop-filter: blur(22px) saturate(180%);
		border: 1px solid var(--border-specular, rgba(255, 255, 255, 0.16));
		border-radius: 20px;
		padding: 24px;
		box-shadow: var(--shadow-elevated);
		display: flex;
		flex-direction: column;
		gap: 18px;
		animation: roulette-spring-in 0.3s var(--spring-out, ease-out) both;
	}

	.roulette-modal:focus {
		outline: none;
	}

	.roulette-header {
		display: flex;
		align-items: flex-start;
		justify-content: space-between;
		gap: 12px;
	}

	.roulette-header-copy {
		min-width: 0;
	}

	.roulette-title-row {
		display: flex;
		align-items: center;
		gap: 10px;
		margin-bottom: 4px;
	}

	.roulette-badge-icon {
		width: 32px;
		height: 32px;
		border-radius: 10px;
		background: color-mix(in srgb, var(--accent, #c8a96e) 16%, transparent);
		border: 1px solid color-mix(in srgb, var(--accent, #c8a96e) 35%, transparent);
		color: var(--accent, #c8a96e);
		display: inline-flex;
		align-items: center;
		justify-content: center;
		flex-shrink: 0;
	}

	.roulette-header h2 {
		margin: 0;
		font-size: 20px;
		font-weight: 700;
		color: var(--text-primary);
	}

	.roulette-subtitle {
		margin: 0;
		font-size: 13px;
		color: var(--text-muted);
		line-height: 1.4;
	}

	.roulette-close-btn {
		width: 32px;
		height: 32px;
		border-radius: 9px;
		border: 1px solid var(--border-soft);
		background: var(--surface-2);
		color: var(--text-muted);
		display: inline-flex;
		align-items: center;
		justify-content: center;
		cursor: pointer;
		flex-shrink: 0;
		transition:
			background 0.2s,
			color 0.2s,
			border-color 0.2s;
	}

	.roulette-close-btn:hover {
		background: var(--surface-hover);
		color: var(--text-primary);
		border-color: var(--border-specular);
	}

	.roulette-modes {
		display: grid;
		grid-template-columns: repeat(3, minmax(0, 1fr));
		gap: 4px;
		padding: 4px;
		background: var(--surface-2);
		border: 1px solid var(--border-soft);
		border-radius: 10px;
	}

	.roulette-mode-btn {
		border: none;
		background: transparent;
		color: var(--text-muted);
		padding: 7px 10px;
		border-radius: 7px;
		font-size: 12.5px;
		font-weight: 600;
		cursor: pointer;
		transition:
			background 0.2s,
			color 0.2s;
		white-space: nowrap;
		overflow: hidden;
		text-overflow: ellipsis;
	}

	.roulette-mode-btn:hover:not(.active) {
		color: var(--text-primary);
		background: var(--surface-hover);
	}

	.roulette-mode-btn.active {
		background: color-mix(in srgb, var(--accent, #c8a96e) 18%, transparent);
		color: var(--accent, #c8a96e);
	}

	.roulette-stage {
		min-height: 184px;
		background: var(--surface-1);
		border: 1px solid var(--border-soft);
		border-radius: 16px;
		padding: 16px 18px;
		display: flex;
		flex-direction: column;
		justify-content: space-between;
		gap: 14px;
		transition:
			border-color 0.25s,
			box-shadow 0.25s,
			background 0.25s;
	}

	.roulette-stage.is-spinning {
		border-color: color-mix(in srgb, var(--accent, #c8a96e) 45%, var(--border-soft));
		box-shadow: 0 0 0 1px color-mix(in srgb, var(--accent, #c8a96e) 16%, transparent);
	}

	.roulette-stage.is-landed {
		border-color: color-mix(in srgb, var(--accent, #c8a96e) 70%, transparent);
		box-shadow:
			0 12px 28px rgba(0, 0, 0, 0.28),
			0 0 0 1px color-mix(in srgb, var(--accent, #c8a96e) 32%, transparent);
		animation: roulette-land-pop 0.36s var(--spring-out, ease-out) both;
	}

	.roulette-stage-top {
		display: flex;
		align-items: center;
		justify-content: space-between;
		gap: 8px;
		flex-wrap: wrap;
	}

	.roulette-status-pill {
		display: inline-flex;
		align-items: center;
		gap: 6px;
		font-size: 11px;
		font-weight: 700;
		text-transform: uppercase;
		letter-spacing: 0.04em;
		color: var(--accent, #c8a96e);
		background: color-mix(in srgb, var(--accent, #c8a96e) 14%, transparent);
		border: 1px solid color-mix(in srgb, var(--accent, #c8a96e) 32%, transparent);
		padding: 3px 9px;
		border-radius: 999px;
	}

	.roulette-status-pill.spinning {
		color: var(--text-primary);
		background: var(--surface-2);
		border-color: var(--border-specular);
	}

	.spin-dot {
		width: 7px;
		height: 7px;
		border-radius: 50%;
		background: var(--accent, #c8a96e);
		animation: roulette-dot-pulse 0.65s ease-in-out infinite alternate;
	}

	.roulette-game-chip {
		font-size: 11.5px;
		font-weight: 600;
		color: var(--text-secondary);
		background: var(--surface-2);
		border: 1px solid var(--border-soft);
		padding: 3px 10px;
		border-radius: 999px;
		max-width: 240px;
		white-space: nowrap;
		overflow: hidden;
		text-overflow: ellipsis;
	}

	.roulette-card {
		display: flex;
		align-items: center;
		gap: 16px;
		min-width: 0;
		transition: opacity 0.12s ease;
	}

	.roulette-card.tick-cycling {
		opacity: 0.88;
	}

	.roulette-art-frame {
		width: 68px;
		height: 68px;
		border-radius: 14px;
		overflow: hidden;
		background: var(--surface-2);
		border: 1px solid var(--border-soft);
		flex-shrink: 0;
		display: flex;
		align-items: center;
		justify-content: center;
	}

	.roulette-art-frame.achievement-art {
		border-color: color-mix(in srgb, var(--accent, #c8a96e) 32%, var(--border-soft));
	}

	.roulette-art-frame img {
		width: 100%;
		height: 100%;
		object-fit: cover;
		display: block;
	}

	.roulette-art-fallback {
		color: var(--accent, #c8a96e);
		display: flex;
		align-items: center;
		justify-content: center;
	}

	.roulette-card-body {
		flex: 1;
		min-width: 0;
		display: flex;
		flex-direction: column;
		gap: 8px;
	}

	.roulette-target-title {
		margin: 0;
		font-size: 17px;
		font-weight: 700;
		color: var(--text-primary);
		line-height: 1.25;
		display: -webkit-box;
		line-clamp: 2;
		-webkit-line-clamp: 2;
		-webkit-box-orient: vertical;
		overflow: hidden;
	}

	.roulette-target-desc {
		margin: 0;
		font-size: 12.5px;
		color: var(--text-secondary);
		line-height: 1.4;
		display: -webkit-box;
		line-clamp: 2;
		-webkit-line-clamp: 2;
		-webkit-box-orient: vertical;
		overflow: hidden;
	}

	.roulette-meta-row {
		display: flex;
		align-items: center;
		justify-content: space-between;
		gap: 10px;
		flex-wrap: wrap;
	}

	.roulette-reason-badge {
		font-size: 12px;
		font-weight: 600;
		color: var(--text-secondary);
	}

	.roulette-progress-counter {
		font-size: 12px;
		font-weight: 700;
		color: var(--accent, #c8a96e);
	}

	.roulette-bar-track {
		height: 6px;
		width: 100%;
		background: var(--surface-2);
		border-radius: 999px;
		overflow: hidden;
	}

	.roulette-bar-fill {
		height: 100%;
		background: var(--accent, #c8a96e);
		border-radius: 999px;
		transition: width 0.25s ease;
	}

	.roulette-empty {
		flex: 1;
		display: flex;
		flex-direction: column;
		align-items: center;
		justify-content: center;
		gap: 10px;
		padding: 24px 12px;
		text-align: center;
		color: var(--text-muted);
	}

	.roulette-empty p {
		margin: 0;
		font-size: 13.5px;
	}

	.roulette-actions {
		display: flex;
		align-items: center;
		justify-content: flex-end;
		gap: 10px;
	}

	.roulette-btn-secondary,
	.roulette-btn-primary {
		display: inline-flex;
		align-items: center;
		justify-content: center;
		gap: 8px;
		padding: 10px 16px;
		border-radius: 10px;
		font-size: 13.5px;
		font-weight: 600;
		cursor: pointer;
		transition:
			background 0.2s,
			border-color 0.2s,
			opacity 0.2s,
			transform 0.15s;
	}

	.roulette-btn-secondary {
		background: var(--surface-2);
		border: 1px solid var(--border-soft);
		color: var(--text-primary);
	}

	.roulette-btn-secondary:hover:not(:disabled) {
		background: var(--surface-hover);
		border-color: var(--border-specular);
	}

	.roulette-btn-primary {
		background: var(--accent, #c8a96e);
		border: 1px solid transparent;
		color: var(--accent-text, #111111);
	}

	.roulette-btn-primary:hover:not(:disabled) {
		filter: brightness(1.06);
		transform: translateY(-1px);
	}

	.roulette-btn-secondary:disabled,
	.roulette-btn-primary:disabled {
		opacity: 0.45;
		cursor: not-allowed;
		transform: none;
	}

	.spinning-icon {
		animation: roulette-spin 0.85s linear infinite;
	}

	@keyframes roulette-fade-in {
		from {
			opacity: 0;
		}
		to {
			opacity: 1;
		}
	}

	@keyframes roulette-spring-in {
		from {
			opacity: 0;
			transform: scale(0.96) translateY(8px);
		}
		to {
			opacity: 1;
			transform: scale(1) translateY(0);
		}
	}

	@keyframes roulette-land-pop {
		0% {
			transform: scale(0.985);
		}
		55% {
			transform: scale(1.01);
		}
		100% {
			transform: scale(1);
		}
	}

	@keyframes roulette-dot-pulse {
		from {
			opacity: 0.45;
			transform: scale(0.85);
		}
		to {
			opacity: 1;
			transform: scale(1.15);
		}
	}

	@keyframes roulette-spin {
		from {
			transform: rotate(0deg);
		}
		to {
			transform: rotate(360deg);
		}
	}

	@media (max-width: 600px) {
		.page-header {
			padding: 20px;
		}

		.content {
			padding: 0 20px 24px;
		}

		.roulette-overlay {
			padding: 12px;
		}

		.roulette-modal {
			padding: 18px;
			border-radius: 16px;
		}

		.roulette-card {
			flex-direction: column;
			align-items: flex-start;
		}

		.roulette-actions {
			flex-direction: column-reverse;
			align-items: stretch;
		}
	}
</style>
