<script lang="ts">
	// src/routes/(app)/stats/+page.svelte
	import { onDestroy, onMount, tick } from 'svelte';
	import { invoke } from '@tauri-apps/api/core';
	import {
		games,
		computeGameProgress,
		getGameCompletionPct,
		getGameIconUrl,
		getRarityPalette,
		totalUnlockedAchievements,
		totalCompletedGames,
		totalPlaytimeSeconds,
		formatPlaytime,
		profileRank,
		type Achievement,
		type Game,
		type GameProgressSummary
	} from '$lib/stores/Games.js';
	import { steamUser } from '$lib/stores/user.js';
	import { settings } from '$lib/stores/settings.js';
	import { settingsOpen } from '$lib/stores/ui.js';
	import { SvelteDate, SvelteMap, SvelteSet } from 'svelte/reactivity';
	import { i18n } from '$lib/stores/i18n.js';

	interface ShowcaseAchievementRef {
		game_id: string;
		ach_key: string;
	}

	interface ProfileShowcase {
		games: string[];
		achievements: ShowcaseAchievementRef[];
	}

	interface ShowcaseGameItem {
		game: Game;
		progress: GameProgressSummary;
	}

	interface ShowcaseAchievementItem {
		game: Game;
		ach: Achievement;
		achKey: string;
		rarityValue: number;
	}

	type ShowcasePickerState = {
		kind: 'game' | 'achievement';
		slotIndex: number;
	} | null;

	const SHOWCASE_SLOTS = [0, 1, 2] as const;
	const MAX_PICKER_ACHIEVEMENTS = 150;

	let activeTab = $state('overview');

	let gameProgressList = $derived(
		$games.map((game) => ({
			game,
			progress: computeGameProgress(game.achievements ?? [], game.achievement_groups)
		}))
	);

	let totalPossibleAchievements = $derived(
		gameProgressList.reduce(
			(acc, { game, progress }) =>
				acc + (game.achievements?.length ? progress.totalAll : game.achievements_total || 0),
			0
		)
	);

	let globalCompletionRate = $derived(
		totalPossibleAchievements > 0
			? Math.round(($totalUnlockedAchievements / totalPossibleAchievements) * 10000) / 100
			: 0
	);

	let remainingAchievements = $derived(
		Math.max(0, totalPossibleAchievements - $totalUnlockedAchievements)
	);

	let sortedByProgression = $derived.by(() => {
		return gameProgressList
			.map(({ game, progress }) => {
				const progression = getGameCompletionPct(game);
				return {
					...game,
					unlockedCount: progress.unlockedBase,
					progression
				};
			})
			.sort((a, b) => {
				if (b.progression !== a.progression) return b.progression - a.progression;
				return b.unlockedCount - a.unlockedCount;
			});
	});

	let topGames = $derived(sortedByProgression.slice(0, 10));
	let podium = $derived(sortedByProgression.slice(0, 3));

	let allUnlockedAchievements = $derived.by(() => {
		return gameProgressList
			.flatMap(({ game, progress }) =>
				progress.sections
					.flatMap((s) => s.achievements)
					.filter((a) => a.unlocked)
					.map((a) => ({ ...a, gameName: game.name }))
			)
			.sort((a, b) => (b.unlocked_time ?? 0) - (a.unlocked_time ?? 0));
	});

	let rarestAchievement = $derived.by(() => {
		const withPct = allUnlockedAchievements.filter((a) => {
			const n = parseFloat(a.completionpercentage);
			return Number.isFinite(n) && n > 0;
		});
		if (withPct.length === 0) return null;
		return [...withPct].sort(
			(a, b) => parseFloat(a.completionpercentage) - parseFloat(b.completionpercentage)
		)[0];
	});

	function formatDate(date: Date | SvelteDate, locale: string) {
		return date.toLocaleDateString(locale, { day: 'numeric', month: 'short', year: 'numeric' });
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

	let profileShowcase = $state<ProfileShowcase>({ games: [], achievements: [] });
	let hasUserMutatedShowcase = false;
	let showcaseSaving = $state(false);
	let isExportingPng = $state(false);
	let showcaseToast = $state('');
	let showcaseToastTimer: ReturnType<typeof setTimeout> | null = null;

	let pickerState = $state<ShowcasePickerState>(null);
	let pickerSearch = $state('');
	let pickerModalEl: HTMLDivElement | undefined = $state();
	let pickerSearchInputEl: HTMLInputElement | undefined = $state();
	let lastPickerTriggerEl: HTMLElement | null = null;
	const failedImages = new SvelteSet<string>();

	function normalizeShowcase(raw: Partial<ProfileShowcase> | null | undefined): ProfileShowcase {
		const gamesList: string[] = [];
		const seenGames = new SvelteSet<string>();
		for (const rawId of raw?.games ?? []) {
			const id = typeof rawId === 'string' ? rawId.trim() : '';
			if (!id || seenGames.has(id)) continue;
			seenGames.add(id);
			gamesList.push(id);
			if (gamesList.length >= 3) break;
		}

		const achList: ShowcaseAchievementRef[] = [];
		const seenAchs = new SvelteSet<string>();
		for (const item of raw?.achievements ?? []) {
			const gameId = typeof item?.game_id === 'string' ? item.game_id.trim() : '';
			const achKey = typeof item?.ach_key === 'string' ? item.ach_key.trim() : '';
			if (!gameId || !achKey) continue;
			const dedupKey = `${gameId}:${achKey.toLowerCase()}`;
			if (seenAchs.has(dedupKey)) continue;
			seenAchs.add(dedupKey);
			achList.push({ game_id: gameId, ach_key: achKey });
			if (achList.length >= 3) break;
		}

		return { games: gamesList, achievements: achList };
	}

	onMount(() => {
		let cancelled = false;
		void (async () => {
			try {
				const loaded = await invoke<ProfileShowcase>('get_profile_showcase');
				if (cancelled || hasUserMutatedShowcase) return;
				profileShowcase = normalizeShowcase(loaded);
			} catch (err) {
				console.error('Failed to load profile showcase:', err);
			}
		})();
		return () => {
			cancelled = true;
		};
	});

	onDestroy(() => {
		if (showcaseToastTimer !== null) {
			clearTimeout(showcaseToastTimer);
			showcaseToastTimer = null;
		}
	});

	function showShowcaseToast(message: string) {
		showcaseToast = message;
		if (showcaseToastTimer !== null) {
			clearTimeout(showcaseToastTimer);
		}
		showcaseToastTimer = setTimeout(() => {
			showcaseToast = '';
			showcaseToastTimer = null;
		}, 3500);
	}

	function parseRarityValue(raw?: string | null): number {
		const n = parseFloat(raw ?? '');
		return Number.isFinite(n) && n > 0 ? n : 100;
	}

	function formatRarityPct(raw?: string | null): string {
		const n = parseFloat(raw ?? '');
		if (!Number.isFinite(n) || n <= 0) return '—';
		return n < 0.1 ? `${n.toFixed(2)}%` : `${n.toFixed(1)}%`;
	}

	function getInitials(label: string): string {
		const words = label
			.trim()
			.split(/\s+/)
			.filter(Boolean);
		if (words.length === 0) return '?';
		if (words.length === 1) return words[0].slice(0, 2).toUpperCase();
		return `${words[0][0]}${words[1][0]}`.toUpperCase();
	}

	function getGameThumbCandidates(game: Game): string[] {
		const raw = [
			game.steamgrid_icon_url?.trim(),
			game.game_icon?.trim() ? getGameIconUrl(game) : '',
			game.header_image_url?.trim(),
			game.steam_id
				? `https://cdn.cloudflare.steamstatic.com/steam/apps/${game.steam_id}/header.jpg`
				: ''
		].filter((u): u is string => Boolean(u));
		return [...new Set(raw)];
	}

	function getGameCardCandidates(game: Game): string[] {
		const raw = [
			game.header_image_url?.trim(),
			game.steam_id
				? `https://cdn.cloudflare.steamstatic.com/steam/apps/${game.steam_id}/header.jpg`
				: '',
			getGameIconUrl(game)
		].filter((u): u is string => Boolean(u));
		return [...new Set(raw)];
	}

	function getBestGameImage(game: Game): string | null {
		for (const url of getGameThumbCandidates(game)) {
			if (!failedImages.has(url)) return url;
		}
		return null;
	}

	let platinumGameItems = $derived.by<ShowcaseGameItem[]>(() =>
		gameProgressList
			.filter((item) => item.progress.isPlatinum)
			.sort((a, b) => a.game.name.localeCompare(b.game.name))
	);

	let unlockedAchievementsByRarity = $derived.by<ShowcaseAchievementItem[]>(() => {
		const seen = new SvelteSet<string>();
		const items: ShowcaseAchievementItem[] = [];
		for (const { game, progress } of gameProgressList) {
			for (const section of progress.sections) {
				for (const ach of section.achievements) {
					if (!ach.unlocked) continue;
					const achKey = (ach.key || ach.name).trim();
					if (!achKey) continue;
					const dedupId = `${game.id}:${achKey.toLowerCase()}`;
					if (seen.has(dedupId)) continue;
					seen.add(dedupId);
					items.push({
						game,
						ach,
						achKey,
						rarityValue: parseRarityValue(ach.completionpercentage)
					});
				}
			}
		}
		return items.sort((a, b) => {
			if (a.rarityValue !== b.rarityValue) return a.rarityValue - b.rarityValue;
			return (b.ach.unlocked_time ?? 0) - (a.ach.unlocked_time ?? 0);
		});
	});

	let validShowcaseGames = $derived.by<ShowcaseGameItem[]>(() => {
		const byId = new Map(platinumGameItems.map((item) => [item.game.id, item]));
		const resolved: ShowcaseGameItem[] = [];
		for (const gameId of profileShowcase.games) {
			const match = byId.get(gameId);
			if (match) resolved.push(match);
			if (resolved.length >= 3) break;
		}
		return resolved;
	});

	let validShowcaseAchievements = $derived.by<ShowcaseAchievementItem[]>(() => {
		const byGameId = new Map(gameProgressList.map((entry) => [entry.game.id, entry]));
		const resolved: ShowcaseAchievementItem[] = [];
		const seen = new SvelteSet<string>();

		for (const ref of profileShowcase.achievements) {
			const entry = byGameId.get(ref.game_id);
			if (!entry) continue;
			const targetKey = ref.ach_key.trim().toLowerCase();
			if (!targetKey) continue;

			const sectionPool = entry.progress.sections.flatMap((s) => s.achievements);
			const matchAch =
				sectionPool.find(
					(a) =>
						a.unlocked &&
						((a.key || a.name).toLowerCase() === targetKey || a.name.toLowerCase() === targetKey)
				) ??
				(entry.game.achievements ?? []).find(
					(a) =>
						a.unlocked &&
						((a.key || a.name).toLowerCase() === targetKey || a.name.toLowerCase() === targetKey)
				);

			if (!matchAch) continue;
			const achKey = matchAch.key || matchAch.name;
			const dedupKey = `${entry.game.id}:${achKey.toLowerCase()}`;
			if (seen.has(dedupKey)) continue;
			seen.add(dedupKey);

			resolved.push({
				game: entry.game,
				ach: matchAch,
				achKey,
				rarityValue: parseRarityValue(matchAch.completionpercentage)
			});
			if (resolved.length >= 3) break;
		}

		return resolved;
	});

	let filteredPickerGames = $derived.by(() => {
		const query = pickerSearch.trim().toLowerCase();
		if (!query) return platinumGameItems;
		return platinumGameItems.filter((item) => item.game.name.toLowerCase().includes(query));
	});

	let filteredPickerAchievementsAll = $derived.by(() => {
		const query = pickerSearch.trim().toLowerCase();
		if (!query) return unlockedAchievementsByRarity;
		return unlockedAchievementsByRarity.filter(
			(item) =>
				item.ach.name.toLowerCase().includes(query) ||
				item.game.name.toLowerCase().includes(query) ||
				(item.ach.desc && item.ach.desc.toLowerCase().includes(query))
		);
	});

	let filteredPickerAchievements = $derived(
		filteredPickerAchievementsAll.slice(0, MAX_PICKER_ACHIEVEMENTS)
	);

	function isSameAchRef(a: ShowcaseAchievementRef, gameId: string, achKey: string): boolean {
		return a.game_id === gameId && a.ach_key.toLowerCase() === achKey.toLowerCase();
	}

	async function persistShowcase(next: ProfileShowcase) {
		hasUserMutatedShowcase = true;
		const normalized = normalizeShowcase(next);
		profileShowcase = normalized;
		showcaseSaving = true;
		try {
			const saved = await invoke<ProfileShowcase>('set_profile_showcase', { showcase: normalized });
			profileShowcase = normalizeShowcase(saved);
		} catch (err) {
			console.error('Failed to save profile showcase:', err);
		} finally {
			showcaseSaving = false;
		}
	}

	function openGamePicker(slotIndex: number) {
		lastPickerTriggerEl =
			document.activeElement instanceof HTMLElement ? document.activeElement : null;
		pickerSearch = '';
		pickerState = { kind: 'game', slotIndex };
		void tick().then(() => {
			if (pickerSearchInputEl) pickerSearchInputEl.focus();
			else pickerModalEl?.focus();
		});
	}

	function openAchPicker(slotIndex: number) {
		lastPickerTriggerEl =
			document.activeElement instanceof HTMLElement ? document.activeElement : null;
		pickerSearch = '';
		pickerState = { kind: 'achievement', slotIndex };
		void tick().then(() => {
			if (pickerSearchInputEl) pickerSearchInputEl.focus();
			else pickerModalEl?.focus();
		});
	}

	function closePicker() {
		pickerState = null;
		pickerSearch = '';
		const trigger = lastPickerTriggerEl;
		lastPickerTriggerEl = null;
		void tick().then(() => {
			trigger?.focus();
		});
	}

	function handleWindowKeydown(event: KeyboardEvent) {
		if (event.key === 'Escape' && pickerState && !$settingsOpen) {
			event.preventDefault();
			closePicker();
		}
	}

	async function selectGameForSlot(gameId: string) {
		if (!pickerState || pickerState.kind !== 'game') return;
		const slotIndex = pickerState.slotIndex;
		const currentIds = validShowcaseGames.map((item) => item.game.id);
		const filteredOther = currentIds.filter((id, idx) => idx === slotIndex || id !== gameId);
		const nextIds = [...filteredOther];
		if (slotIndex < nextIds.length) {
			nextIds[slotIndex] = gameId;
		} else {
			nextIds.push(gameId);
		}
		const currentAchs: ShowcaseAchievementRef[] = validShowcaseAchievements.map((item) => ({
			game_id: item.game.id,
			ach_key: item.achKey
		}));
		closePicker();
		await persistShowcase({ games: nextIds, achievements: currentAchs });
	}

	async function removeGameSlot(slotIndex: number) {
		const nextIds = validShowcaseGames
			.filter((_, idx) => idx !== slotIndex)
			.map((item) => item.game.id);
		const currentAchs: ShowcaseAchievementRef[] = validShowcaseAchievements.map((item) => ({
			game_id: item.game.id,
			ach_key: item.achKey
		}));
		await persistShowcase({ games: nextIds, achievements: currentAchs });
	}

	async function selectAchForSlot(item: ShowcaseAchievementItem) {
		if (!pickerState || pickerState.kind !== 'achievement') return;
		const slotIndex = pickerState.slotIndex;
		const currentIds = validShowcaseGames.map((g) => g.game.id);
		const newRef: ShowcaseAchievementRef = {
			game_id: item.game.id,
			ach_key: item.achKey
		};
		const currentAchs: ShowcaseAchievementRef[] = validShowcaseAchievements.map((entry) => ({
			game_id: entry.game.id,
			ach_key: entry.achKey
		}));
		const nextAchs = currentAchs.filter(
			(ref, idx) => idx === slotIndex || !isSameAchRef(ref, newRef.game_id, newRef.ach_key)
		);
		if (slotIndex < nextAchs.length) {
			nextAchs[slotIndex] = newRef;
		} else {
			nextAchs.push(newRef);
		}
		closePicker();
		await persistShowcase({ games: currentIds, achievements: nextAchs });
	}

	async function removeAchSlot(slotIndex: number) {
		const currentIds = validShowcaseGames.map((g) => g.game.id);
		const nextAchs: ShowcaseAchievementRef[] = validShowcaseAchievements
			.filter((_, idx) => idx !== slotIndex)
			.map((item) => ({ game_id: item.game.id, ach_key: item.achKey }));
		await persistShowcase({ games: currentIds, achievements: nextAchs });
	}

	async function autoFillShowcase() {
		const sortedPlatinums = [...platinumGameItems].sort(
			(a, b) =>
				b.progress.totalBase - a.progress.totalBase ||
				b.progress.unlockedAll - a.progress.unlockedAll ||
				(b.game.playtime_seconds ?? 0) - (a.game.playtime_seconds ?? 0)
		);

		const bothFull = validShowcaseGames.length >= 3 && validShowcaseAchievements.length >= 3;

		let nextGameIds: string[];
		if (bothFull || validShowcaseGames.length === 0) {
			nextGameIds = sortedPlatinums.slice(0, 3).map((item) => item.game.id);
		} else {
			nextGameIds = validShowcaseGames.map((item) => item.game.id);
			const existingIds = new SvelteSet(nextGameIds);
			for (const item of sortedPlatinums) {
				if (nextGameIds.length >= 3) break;
				if (!existingIds.has(item.game.id)) {
					nextGameIds.push(item.game.id);
					existingIds.add(item.game.id);
				}
			}
		}

		let nextAchs: ShowcaseAchievementRef[];
		if (bothFull || validShowcaseAchievements.length === 0) {
			nextAchs = unlockedAchievementsByRarity.slice(0, 3).map((item) => ({
				game_id: item.game.id,
				ach_key: item.achKey
			}));
		} else {
			nextAchs = validShowcaseAchievements.map((item) => ({
				game_id: item.game.id,
				ach_key: item.achKey
			}));
			for (const item of unlockedAchievementsByRarity) {
				if (nextAchs.length >= 3) break;
				if (!nextAchs.some((ref) => isSameAchRef(ref, item.game.id, item.achKey))) {
					nextAchs.push({ game_id: item.game.id, ach_key: item.achKey });
				}
			}
		}

		await persistShowcase({ games: nextGameIds, achievements: nextAchs });
	}

	function loadImg(src: string): Promise<HTMLImageElement | null> {
		return new Promise((resolve) => {
			if (!src) return resolve(null);
			let settled = false;
			const img = new Image();
			const timer = setTimeout(() => {
				if (settled) return;
				settled = true;
				resolve(null);
			}, 5000);
			img.crossOrigin = 'anonymous';
			img.onload = () => {
				if (settled) return;
				settled = true;
				clearTimeout(timer);
				resolve(img);
			};
			img.onerror = () => {
				if (settled) return;
				settled = true;
				clearTimeout(timer);
				resolve(null);
			};
			img.src = src;
		});
	}

	async function loadFirstAvailableImg(urls: readonly string[]): Promise<HTMLImageElement | null> {
		for (const url of urls) {
			const img = await loadImg(url);
			if (img) return img;
		}
		return null;
	}

	function drawRoundedRect(
		ctx: CanvasRenderingContext2D,
		x: number,
		y: number,
		w: number,
		h: number,
		r: number
	) {
		ctx.beginPath();
		ctx.roundRect(x, y, w, h, r);
	}

	function drawImageCover(
		ctx: CanvasRenderingContext2D,
		img: HTMLImageElement,
		x: number,
		y: number,
		w: number,
		h: number
	) {
		const scale = Math.max(w / img.width, h / img.height);
		const dw = img.width * scale;
		const dh = img.height * scale;
		ctx.drawImage(img, x + (w - dw) / 2, y + (h - dh) / 2, dw, dh);
	}

	function truncateCanvasText(
		ctx: CanvasRenderingContext2D,
		text: string,
		maxWidth: number
	): string {
		if (!text) return '';
		if (ctx.measureText(text).width <= maxWidth) return text;
		const ellipsis = '…';
		let low = 0;
		let high = text.length;
		while (low < high) {
			const mid = Math.ceil((low + high) / 2);
			const candidate = text.slice(0, mid).trimEnd() + ellipsis;
			if (ctx.measureText(candidate).width <= maxWidth) {
				low = mid;
			} else {
				high = mid - 1;
			}
		}
		return text.slice(0, low).trimEnd() + ellipsis;
	}

	async function exportGlobalProfileCard() {
		if (isExportingPng) return;
		isExportingPng = true;

		try {
			const W = 1280;
			const H = 720;
			const canvas = document.createElement('canvas');
			canvas.width = W;
			canvas.height = H;
			const ctx = canvas.getContext('2d');
			if (!ctx) return;

			const showcaseGames = validShowcaseGames.slice(0, 3);
			const showcaseAchs = validShowcaseAchievements.slice(0, 3);

			const [avatarImg, gameImgs, achImgs] = await Promise.all([
				$steamUser?.avatarfull ? loadImg($steamUser.avatarfull) : Promise.resolve(null),
				Promise.all(showcaseGames.map((item) => loadFirstAvailableImg(getGameCardCandidates(item.game)))),
				Promise.all(showcaseAchs.map((item) => loadImg(item.ach.icon)))
			]);

			// Background base gradient
			const bgGrad = ctx.createLinearGradient(0, 0, W, H);
			bgGrad.addColorStop(0, '#0b0d12');
			bgGrad.addColorStop(0.5, '#12151f');
			bgGrad.addColorStop(1, '#0c0e14');
			ctx.fillStyle = bgGrad;
			ctx.fillRect(0, 0, W, H);

			// Ambient radial glows
			const goldGlow = ctx.createRadialGradient(180, 110, 10, 180, 110, 520);
			goldGlow.addColorStop(0, 'rgba(200, 169, 110, 0.16)');
			goldGlow.addColorStop(1, 'rgba(200, 169, 110, 0)');
			ctx.fillStyle = goldGlow;
			ctx.fillRect(0, 0, W, H);

			const coolGlow = ctx.createRadialGradient(1120, 620, 10, 1120, 620, 460);
			coolGlow.addColorStop(0, 'rgba(78, 168, 222, 0.1)');
			coolGlow.addColorStop(1, 'rgba(78, 168, 222, 0)');
			ctx.fillStyle = coolGlow;
			ctx.fillRect(0, 0, W, H);

			// Outer Accolade glassmorphic gold-bordered card
			ctx.save();
			ctx.fillStyle = 'rgba(18, 20, 27, 0.84)';
			ctx.strokeStyle = 'rgba(200, 169, 110, 0.42)';
			ctx.lineWidth = 2;
			drawRoundedRect(ctx, 24, 24, W - 48, H - 48, 24);
			ctx.fill();
			ctx.stroke();

			ctx.strokeStyle = 'rgba(255, 255, 255, 0.06)';
			ctx.lineWidth = 1;
			drawRoundedRect(ctx, 27, 27, W - 54, H - 54, 21);
			ctx.stroke();
			ctx.restore();

			// Player Header Panel
			const hx = 52;
			const hy = 48;
			const hw = W - 104;
			const hh = 132;
			ctx.save();
			ctx.fillStyle = 'rgba(255, 255, 255, 0.035)';
			ctx.strokeStyle = 'rgba(200, 169, 110, 0.28)';
			ctx.lineWidth = 1.5;
			drawRoundedRect(ctx, hx, hy, hw, hh, 18);
			ctx.fill();
			ctx.stroke();
			ctx.restore();

			const username =
				$steamUser?.personaname || $settings.raUsername || $i18n.t('settings.unknownPlayer');
			const avatarCx = hx + 66;
			const avatarCy = hy + hh / 2;
			const avatarR = 42;

			// Avatar circular clip + gold ring
			ctx.save();
			ctx.beginPath();
			ctx.arc(avatarCx, avatarCy, avatarR + 4, 0, Math.PI * 2);
			ctx.strokeStyle = '#c8a96e';
			ctx.lineWidth = 3;
			ctx.stroke();

			ctx.beginPath();
			ctx.arc(avatarCx, avatarCy, avatarR, 0, Math.PI * 2);
			ctx.clip();
			if (avatarImg) {
				drawImageCover(
					ctx,
					avatarImg,
					avatarCx - avatarR,
					avatarCy - avatarR,
					avatarR * 2,
					avatarR * 2
				);
			} else {
				const fallbackGrad = ctx.createLinearGradient(
					avatarCx - avatarR,
					avatarCy - avatarR,
					avatarCx + avatarR,
					avatarCy + avatarR
				);
				fallbackGrad.addColorStop(0, '#2b2418');
				fallbackGrad.addColorStop(1, '#181a22');
				ctx.fillStyle = fallbackGrad;
				ctx.fillRect(avatarCx - avatarR, avatarCy - avatarR, avatarR * 2, avatarR * 2);
				ctx.fillStyle = '#e7c98a';
				ctx.font = '800 30px "Segoe UI", system-ui, sans-serif';
				ctx.textAlign = 'center';
				ctx.textBaseline = 'middle';
				ctx.fillText(getInitials(username).slice(0, 1), avatarCx, avatarCy + 1);
			}
			ctx.restore();

			// Username & Level badge
			const infoX = hx + 132;
			ctx.textAlign = 'left';
			ctx.textBaseline = 'alphabetic';
			ctx.font = '800 28px "Segoe UI", system-ui, sans-serif';
			const displayName = truncateCanvasText(ctx, username, 460);
			ctx.fillStyle = '#ffffff';
			ctx.fillText(displayName, infoX, hy + 46);

			const nameW = ctx.measureText(displayName).width;
			const levelLabel = $i18n.t('stats.showcase.cardLevel', { level: $profileRank.level });
			ctx.font = '800 13px "Segoe UI", system-ui, sans-serif';
			const lvlTextW = ctx.measureText(levelLabel).width;
			const lvlPillX = infoX + nameW + 16;
			const lvlPillY = hy + 24;
			const lvlPillW = lvlTextW + 22;
			const lvlPillH = 26;

			ctx.save();
			ctx.fillStyle = 'rgba(200, 169, 110, 0.16)';
			ctx.strokeStyle = 'rgba(200, 169, 110, 0.52)';
			ctx.lineWidth = 1.25;
			drawRoundedRect(ctx, lvlPillX, lvlPillY, lvlPillW, lvlPillH, 13);
			ctx.fill();
			ctx.stroke();
			ctx.fillStyle = '#f3d38b';
			ctx.textAlign = 'center';
			ctx.textBaseline = 'middle';
			ctx.fillText(levelLabel, lvlPillX + lvlPillW / 2, lvlPillY + lvlPillH / 2 + 0.5);
			ctx.restore();

			// Gamerscore + Unlocked achievements line
			ctx.textAlign = 'left';
			ctx.textBaseline = 'alphabetic';
			const scoreStr = `${$profileRank.score.toLocaleString($i18n.locale)} G`;
			ctx.font = '800 16px "Segoe UI", system-ui, sans-serif';
			ctx.fillStyle = '#c8a96e';
			ctx.fillText(scoreStr, infoX, hy + 76);
			const scoreW = ctx.measureText(scoreStr).width;

			const unlockedSummary = `  ·  ${$totalUnlockedAchievements.toLocaleString($i18n.locale)} ${$i18n.t('stats.unlockedAchievementsLabel')}`;
			ctx.font = '600 14px "Segoe UI", system-ui, sans-serif';
			ctx.fillStyle = 'rgba(255, 255, 255, 0.65)';
			ctx.fillText(truncateCanvasText(ctx, unlockedSummary, 430), infoX + scoreW, hy + 76);

			// XP Progress Bar
			const xpBarX = infoX;
			const xpBarY = hy + 94;
			const xpBarW = 360;
			const xpBarH = 10;
			ctx.save();
			ctx.fillStyle = 'rgba(255, 255, 255, 0.08)';
			drawRoundedRect(ctx, xpBarX, xpBarY, xpBarW, xpBarH, 5);
			ctx.fill();

			const fillPct = Math.max(0, Math.min(100, $profileRank.progressPct));
			if (fillPct > 0) {
				const fillW = Math.max(10, (xpBarW * fillPct) / 100);
				const xpGrad = ctx.createLinearGradient(xpBarX, 0, xpBarX + fillW, 0);
				xpGrad.addColorStop(0, '#c8a96e');
				xpGrad.addColorStop(1, '#f3d38b');
				ctx.fillStyle = xpGrad;
				drawRoundedRect(ctx, xpBarX, xpBarY, fillW, xpBarH, 5);
				ctx.fill();
			}
			ctx.restore();

			ctx.fillStyle = 'rgba(255, 255, 255, 0.56)';
			ctx.font = '600 12px "Segoe UI", system-ui, sans-serif';
			const xpCaption = `${$profileRank.currentLevelXp.toLocaleString($i18n.locale)} / ${$profileRank.nextLevelXp.toLocaleString($i18n.locale)} XP (${$profileRank.progressPct}%)`;
			ctx.fillText(xpCaption, xpBarX + xpBarW + 14, xpBarY + 9);

			// Right side: Completion % badge + Total Platinums badge
			const platBadgeW = 176;
			const platBadgeH = 92;
			const platBadgeX = hx + hw - platBadgeW - 22;
			const platBadgeY = hy + 20;

			const rateBadgeW = 148;
			const rateBadgeX = platBadgeX - rateBadgeW - 14;
			const rateBadgeY = hy + 20;

			ctx.save();
			ctx.fillStyle = 'rgba(255, 255, 255, 0.04)';
			ctx.strokeStyle = 'rgba(255, 255, 255, 0.12)';
			ctx.lineWidth = 1.25;
			drawRoundedRect(ctx, rateBadgeX, rateBadgeY, rateBadgeW, platBadgeH, 14);
			ctx.fill();
			ctx.stroke();

			ctx.textAlign = 'center';
			ctx.fillStyle = '#ffffff';
			ctx.font = '800 26px "Segoe UI", system-ui, sans-serif';
			ctx.fillText(`${globalCompletionRate}%`, rateBadgeX + rateBadgeW / 2, rateBadgeY + 48);
			ctx.fillStyle = 'rgba(255, 255, 255, 0.55)';
			ctx.font = '700 11px "Segoe UI", system-ui, sans-serif';
			ctx.fillText(
				truncateCanvasText(ctx, $i18n.t('stats.completionLabel').toUpperCase(), rateBadgeW - 16),
				rateBadgeX + rateBadgeW / 2,
				rateBadgeY + 72
			);

			// Total Platinums badge
			ctx.fillStyle = 'rgba(200, 169, 110, 0.12)';
			ctx.strokeStyle = 'rgba(200, 169, 110, 0.48)';
			ctx.lineWidth = 1.5;
			drawRoundedRect(ctx, platBadgeX, platBadgeY, platBadgeW, platBadgeH, 14);
			ctx.fill();
			ctx.stroke();

			ctx.fillStyle = '#f3d38b';
			ctx.font = '800 30px "Segoe UI", system-ui, sans-serif';
			ctx.fillText(String($totalCompletedGames), platBadgeX + platBadgeW / 2, platBadgeY + 48);
			ctx.fillStyle = 'rgba(243, 211, 139, 0.85)';
			ctx.font = '700 11px "Segoe UI", system-ui, sans-serif';
			ctx.fillText(
				truncateCanvasText(
					ctx,
					$i18n.t('stats.showcase.cardPlatinums').toUpperCase(),
					platBadgeW - 16
				),
				platBadgeX + platBadgeW / 2,
				platBadgeY + 72
			);
			ctx.restore();

			// Two Showcase Panels
			const panelY = 202;
			const panelH = 446;
			const panelW = 576;
			const leftX = 52;
			const rightX = 652;

			const drawPanelFrame = (px: number, title: string, count: number) => {
				ctx.save();
				ctx.fillStyle = 'rgba(255, 255, 255, 0.028)';
				ctx.strokeStyle = 'rgba(255, 255, 255, 0.09)';
				ctx.lineWidth = 1.25;
				drawRoundedRect(ctx, px, panelY, panelW, panelH, 18);
				ctx.fill();
				ctx.stroke();

				// Accent marker
				ctx.fillStyle = '#c8a96e';
				ctx.beginPath();
				ctx.arc(px + 26, panelY + 29, 4, 0, Math.PI * 2);
				ctx.fill();

				ctx.textAlign = 'left';
				ctx.textBaseline = 'middle';
				ctx.fillStyle = '#c8a96e';
				ctx.font = '800 13px "Segoe UI", system-ui, sans-serif';
				ctx.fillText(truncateCanvasText(ctx, title.toUpperCase(), panelW - 110), px + 38, panelY + 30);

				ctx.textAlign = 'right';
				ctx.fillStyle = 'rgba(255, 255, 255, 0.48)';
				ctx.font = '700 12px "Segoe UI", system-ui, sans-serif';
				ctx.fillText(`${count} / 3`, px + panelW - 22, panelY + 30);

				// Header divider
				ctx.strokeStyle = 'rgba(255, 255, 255, 0.07)';
				ctx.beginPath();
				ctx.moveTo(px + 20, panelY + 52);
				ctx.lineTo(px + panelW - 20, panelY + 52);
				ctx.stroke();
				ctx.restore();
			};

			// Left Panel: Platinum Games
			drawPanelFrame(leftX, $i18n.t('stats.showcase.platinumGamesTitle'), showcaseGames.length);
			for (let idx = 0; idx < 3; idx++) {
				const rowX = leftX + 20;
				const rowY = panelY + 68 + idx * 124;
				const rowW = panelW - 40;
				const rowH = 110;
				const slot = showcaseGames[idx];

				if (slot) {
					ctx.save();
					ctx.fillStyle = 'rgba(255, 255, 255, 0.045)';
					ctx.strokeStyle = 'rgba(200, 169, 110, 0.34)';
					ctx.lineWidth = 1.25;
					drawRoundedRect(ctx, rowX, rowY, rowW, rowH, 14);
					ctx.fill();
					ctx.stroke();
					ctx.restore();

					const thumbX = rowX + 14;
					const thumbY = rowY + 14;
					const thumbW = 136;
					const thumbH = 82;
					const gameImg = gameImgs[idx];

					ctx.save();
					drawRoundedRect(ctx, thumbX, thumbY, thumbW, thumbH, 10);
					ctx.clip();
					if (gameImg) {
						drawImageCover(ctx, gameImg, thumbX, thumbY, thumbW, thumbH);
					} else {
						ctx.fillStyle = 'rgba(200, 169, 110, 0.12)';
						ctx.fillRect(thumbX, thumbY, thumbW, thumbH);
						ctx.fillStyle = '#e7c98a';
						ctx.font = '800 22px "Segoe UI", system-ui, sans-serif';
						ctx.textAlign = 'center';
						ctx.textBaseline = 'middle';
						ctx.fillText(getInitials(slot.game.name), thumbX + thumbW / 2, thumbY + thumbH / 2);
					}
					ctx.restore();

					const tx = thumbX + thumbW + 18;
					const maxTextW = rowX + rowW - tx - 16;

					ctx.textAlign = 'left';
					ctx.textBaseline = 'alphabetic';
					ctx.fillStyle = '#ffffff';
					ctx.font = '700 18px "Segoe UI", system-ui, sans-serif';
					ctx.fillText(truncateCanvasText(ctx, slot.game.name, maxTextW), tx, rowY + 38);

					ctx.fillStyle = 'rgba(255, 255, 255, 0.68)';
					ctx.font = '600 14px "Segoe UI", system-ui, sans-serif';
					ctx.fillText(`${slot.progress.unlockedBase} / ${slot.progress.totalBase}`, tx, rowY + 62);

					// 100% PLATINUM badge
					ctx.save();
					const badgeText = $i18n.t('stats.showcase.platinumBadge');
					ctx.font = '800 11px "Segoe UI", system-ui, sans-serif';
					const badgeW = ctx.measureText(badgeText).width + 20;
					const badgeY = rowY + 73;
					ctx.fillStyle = 'rgba(228, 180, 65, 0.16)';
					ctx.strokeStyle = 'rgba(228, 180, 65, 0.48)';
					ctx.lineWidth = 1;
					drawRoundedRect(ctx, tx, badgeY, badgeW, 23, 999);
					ctx.fill();
					ctx.stroke();
					ctx.fillStyle = '#f3d38b';
					ctx.textBaseline = 'middle';
					ctx.fillText(badgeText, tx + 10, badgeY + 11.5);
					ctx.restore();
				} else {
					ctx.save();
					ctx.fillStyle = 'rgba(255, 255, 255, 0.015)';
					ctx.strokeStyle = 'rgba(255, 255, 255, 0.14)';
					ctx.lineWidth = 1.25;
					ctx.setLineDash([6, 6]);
					drawRoundedRect(ctx, rowX, rowY, rowW, rowH, 14);
					ctx.fill();
					ctx.stroke();
					ctx.setLineDash([]);
					ctx.fillStyle = 'rgba(255, 255, 255, 0.28)';
					ctx.font = '600 14px "Segoe UI", system-ui, sans-serif';
					ctx.textAlign = 'center';
					ctx.textBaseline = 'middle';
					ctx.fillText($i18n.t('stats.showcase.emptyGameSlot'), rowX + rowW / 2, rowY + rowH / 2);
					ctx.restore();
				}
			}

			// Right Panel: Rarest Achievements
			drawPanelFrame(rightX, $i18n.t('stats.showcase.rareAchievementsTitle'), showcaseAchs.length);
			for (let idx = 0; idx < 3; idx++) {
				const rowX = rightX + 20;
				const rowY = panelY + 68 + idx * 124;
				const rowW = panelW - 40;
				const rowH = 110;
				const slot = showcaseAchs[idx];

				if (slot) {
					const palette = getRarityPalette(slot.ach.completionpercentage);
					ctx.save();
					ctx.fillStyle = 'rgba(255, 255, 255, 0.045)';
					ctx.strokeStyle = palette.border;
					ctx.lineWidth = 1.25;
					drawRoundedRect(ctx, rowX, rowY, rowW, rowH, 14);
					ctx.fill();
					ctx.stroke();
					ctx.restore();

					const iconX = rowX + 16;
					const iconY = rowY + 17;
					const iconSize = 76;
					const achImg = achImgs[idx];

					ctx.save();
					drawRoundedRect(ctx, iconX, iconY, iconSize, iconSize, 12);
					ctx.clip();
					if (achImg) {
						drawImageCover(ctx, achImg, iconX, iconY, iconSize, iconSize);
					} else {
						ctx.fillStyle = palette.bg;
						ctx.fillRect(iconX, iconY, iconSize, iconSize);
						ctx.fillStyle = palette.color;
						ctx.font = '800 22px "Segoe UI", system-ui, sans-serif';
						ctx.textAlign = 'center';
						ctx.textBaseline = 'middle';
						ctx.fillText(
							getInitials(slot.ach.name || slot.ach.key),
							iconX + iconSize / 2,
							iconY + iconSize / 2
						);
					}
					ctx.restore();

					// Rarity % pill on right
					const rarityLabel = formatRarityPct(slot.ach.completionpercentage);
					ctx.save();
					ctx.font = '800 13px "Segoe UI", system-ui, sans-serif';
					const pillW = Math.max(66, ctx.measureText(rarityLabel).width + 22);
					const pillH = 30;
					const pillX = rowX + rowW - pillW - 16;
					const pillY = rowY + (rowH - pillH) / 2;
					ctx.fillStyle = palette.bg;
					ctx.strokeStyle = palette.border;
					ctx.lineWidth = 1.25;
					drawRoundedRect(ctx, pillX, pillY, pillW, pillH, 999);
					ctx.fill();
					ctx.stroke();
					ctx.fillStyle = palette.color;
					ctx.textAlign = 'center';
					ctx.textBaseline = 'middle';
					ctx.fillText(rarityLabel, pillX + pillW / 2, pillY + pillH / 2 + 0.5);
					ctx.restore();

					const tx = iconX + iconSize + 16;
					const maxTextW = rowX + rowW - tx - pillW - 30;

					ctx.textAlign = 'left';
					ctx.textBaseline = 'alphabetic';
					ctx.fillStyle = '#ffffff';
					ctx.font = '700 17px "Segoe UI", system-ui, sans-serif';
					ctx.fillText(
						truncateCanvasText(ctx, slot.ach.name || slot.ach.key, maxTextW),
						tx,
						rowY + 40
					);

					ctx.fillStyle = '#c8a96e';
					ctx.font = '600 13px "Segoe UI", system-ui, sans-serif';
					ctx.fillText(truncateCanvasText(ctx, slot.game.name, maxTextW), tx, rowY + 63);

					if (slot.ach.desc) {
						ctx.fillStyle = 'rgba(255, 255, 255, 0.54)';
						ctx.font = '500 12px "Segoe UI", system-ui, sans-serif';
						ctx.fillText(truncateCanvasText(ctx, slot.ach.desc, maxTextW), tx, rowY + 84);
					}
				} else {
					ctx.save();
					ctx.fillStyle = 'rgba(255, 255, 255, 0.015)';
					ctx.strokeStyle = 'rgba(255, 255, 255, 0.14)';
					ctx.lineWidth = 1.25;
					ctx.setLineDash([6, 6]);
					drawRoundedRect(ctx, rowX, rowY, rowW, rowH, 14);
					ctx.fill();
					ctx.stroke();
					ctx.setLineDash([]);
					ctx.fillStyle = 'rgba(255, 255, 255, 0.28)';
					ctx.font = '600 14px "Segoe UI", system-ui, sans-serif';
					ctx.textAlign = 'center';
					ctx.textBaseline = 'middle';
					ctx.fillText($i18n.t('stats.showcase.emptyAchSlot'), rowX + rowW / 2, rowY + rowH / 2);
					ctx.restore();
				}
			}

			// Footer
			ctx.textAlign = 'left';
			ctx.textBaseline = 'middle';
			ctx.fillStyle = 'rgba(200, 169, 110, 0.88)';
			ctx.font = '800 12px "Segoe UI", system-ui, sans-serif';
			ctx.fillText(`ACCOLADE  •  ${$i18n.t('stats.showcase.cardTitle')}`, 52, H - 46);

			ctx.textAlign = 'right';
			ctx.fillStyle = 'rgba(255, 255, 255, 0.45)';
			ctx.font = '600 12px "Segoe UI", system-ui, sans-serif';
			ctx.fillText(formatDate(new Date(), $i18n.locale), W - 52, H - 46);

			const blob = await new Promise<Blob | null>((resolve) => {
				try {
					canvas.toBlob((b) => resolve(b), 'image/png');
				} catch (err) {
					console.error('Failed to serialize profile card canvas:', err);
					resolve(null);
				}
			});
			if (!blob) return;

			try {
				if (navigator.clipboard && 'ClipboardItem' in window) {
					await navigator.clipboard.write([new ClipboardItem({ 'image/png': blob })]);
				}
			} catch {
				// Clipboard write may fail without focus/permissions; download still proceeds
			}

			const url = URL.createObjectURL(blob);
			const a = document.createElement('a');
			a.href = url;
			a.download = 'accolade-profile-card.png';
			a.click();
			setTimeout(() => URL.revokeObjectURL(url), 1000);

			showShowcaseToast($i18n.t('stats.showcase.exportedToast'));
		} catch (err) {
			console.error('Failed to generate profile showcase PNG:', err);
		} finally {
			isExportingPng = false;
		}
	}
</script>

<svelte:window onkeydown={handleWindowKeydown} />

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
						activeTab = 'overview';
					}}
				>
					{$i18n.t('stats.tabOverview')}
				</button>
				<button
					type="button"
					class:active={activeTab === 'activity'}
					onclick={() => {
						activeTab = 'activity';
					}}
				>
					{$i18n.t('stats.tabActivity')}
				</button>
			</div>
		</div>
		<div class="header-right">
			<span class="date-now">{formatDate(new Date(), $i18n.locale)}</span>
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

				<div class="card stat-summary gamerscore-card">
					<div class="icon-circle gamerscore">
						<svg
							width="16"
							height="16"
							viewBox="0 0 24 24"
							fill="none"
							stroke="currentColor"
							stroke-width="2"
						>
							<polygon
								points="12 2 15.09 8.26 22 9.27 17 14.14 18.18 21.02 12 17.77 5.82 21.02 7 14.14 2 9.27 8.91 8.26 12 2"
							/>
						</svg>
					</div>
					<div class="card-info">
						<span class="label">{$i18n.t('stats.gamerscore')}</span>
						<span class="value">{$profileRank.score.toLocaleString()} G</span>
						<span class="sub">
							{$i18n.t('stats.levelSub', {
								level: $profileRank.level,
								xp: $profileRank.currentLevelXp,
								next: $profileRank.nextLevelXp
							})}
						</span>
					</div>
				</div>

				<div class="card stat-summary playtime-card">
					<div class="icon-circle playtime">
						<svg
							width="16"
							height="16"
							viewBox="0 0 24 24"
							fill="none"
							stroke="currentColor"
							stroke-width="2"
						>
							<circle cx="12" cy="12" r="10" />
							<polyline points="12 6 12 12 16 14" />
						</svg>
					</div>
					<div class="card-info">
						<span class="label">{$i18n.t('stats.playtime')}</span>
						<span class="value">{formatPlaytime($totalPlaytimeSeconds)}</span>
						<span class="sub">{$i18n.t('stats.playtimeSub')}</span>
					</div>
				</div>

				<!-- Profile Showcase / Trophy Cabinet -->
				<section class="card showcase-section" aria-labelledby="showcase-heading">
					<div class="showcase-header">
						<div class="showcase-title-group">
							<div class="showcase-icon-badge" aria-hidden="true">
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
									<path
										d="M6 9H4.5a2.5 2.5 0 0 1 0-5H6M18 9h1.5a2.5 2.5 0 0 0 0-5H18M4 22h16M10 14.66V17c0 .55.47.98.97 1.21C11.47 18.44 12 19 12 19s.53-.56 1.03-.79c.5-.23.97-.66.97-1.21v-2.34M7 2h10a1 1 0 0 1 1 1v12a1 1 0 0 1-1 1H7a1 1 0 0 1-1-1V3a1 1 0 0 1 1-1Z"
									/>
								</svg>
							</div>
							<div class="showcase-heading-text">
								<h2 id="showcase-heading">{$i18n.t('stats.showcase.title')}</h2>
								<p class="showcase-subtitle">{$i18n.t('stats.showcase.subtitle')}</p>
							</div>
						</div>

						<div class="showcase-actions">
							{#if showcaseToast}
								<span class="showcase-toast" role="status">{showcaseToast}</span>
							{/if}
							<button
								type="button"
								class="showcase-btn secondary"
								disabled={showcaseSaving ||
									(platinumGameItems.length === 0 && unlockedAchievementsByRarity.length === 0)}
								onclick={() => void autoFillShowcase()}
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
									<path
										d="m12 3-1.912 5.813a2 2 0 0 1-1.275 1.275L3 12l5.813 1.912a2 2 0 0 1 1.275 1.275L12 21l1.912-5.813a2 2 0 0 1 1.275-1.275L21 12l-5.813-1.912a2 2 0 0 1-1.275-1.275L12 3Z"
									/>
								</svg>
								<span>{$i18n.t('stats.showcase.autoFill')}</span>
							</button>
							<button
								type="button"
								class="showcase-btn primary"
								disabled={isExportingPng}
								onclick={() => void exportGlobalProfileCard()}
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
									<path d="M21 15v4a2 2 0 0 1-2 2H5a2 2 0 0 1-2-2v-4" />
									<polyline points="7 10 12 15 17 10" />
									<line x1="12" x2="12" y1="15" y2="3" />
								</svg>
								<span>
									{isExportingPng
										? $i18n.t('stats.showcase.exportingPng')
										: $i18n.t('stats.showcase.exportPng')}
								</span>
							</button>
						</div>
					</div>

					<div class="showcase-columns">
						<!-- Platinum / 100% Games Column -->
						<div class="showcase-group">
							<div class="showcase-group-header">
								<h3>
									<svg
										width="14"
										height="14"
										viewBox="0 0 24 24"
										fill="none"
										stroke="currentColor"
										stroke-width="2"
										aria-hidden="true"
									>
										<polygon
											points="12 2 15.09 8.26 22 9.27 17 14.14 18.18 21.02 12 17.77 5.82 21.02 7 14.14 2 9.27 8.91 8.26 12 2"
										/>
									</svg>
									{$i18n.t('stats.showcase.platinumGamesTitle')}
								</h3>
								<span class="showcase-count-pill tabular-nums">{validShowcaseGames.length}/3</span>
							</div>
							<div class="showcase-slots">
								{#each SHOWCASE_SLOTS as slotIdx (slotIdx)}
									{@const slotGame = validShowcaseGames[slotIdx] ?? null}
									{#if slotGame}
										{@const thumbUrl = getBestGameImage(slotGame.game)}
										<div class="showcase-slot filled">
											<button
												type="button"
												class="showcase-slot-main"
												onclick={() => openGamePicker(slotIdx)}
											>
												{#if thumbUrl}
													<img
														class="slot-game-thumb"
														src={thumbUrl}
														alt={slotGame.game.name}
														loading="lazy"
														decoding="async"
														onerror={() => failedImages.add(thumbUrl)}
													/>
												{:else}
													<div class="slot-game-fallback" aria-hidden="true">
														{getInitials(slotGame.game.name)}
													</div>
												{/if}
												<div class="slot-info">
													<span class="slot-title">{slotGame.game.name}</span>
													<span class="slot-meta tabular-nums">
														{slotGame.progress.unlockedBase}/{slotGame.progress.totalBase}
													</span>
												</div>
												<span class="slot-platinum-badge tabular-nums">100%</span>
											</button>
											<button
												type="button"
												class="slot-remove-btn"
												title={$i18n.t('stats.showcase.removeSlot')}
												aria-label={$i18n.t('stats.showcase.removeSlot')}
												disabled={showcaseSaving}
												onclick={() => void removeGameSlot(slotIdx)}
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
													<line x1="18" y1="6" x2="6" y2="18" />
													<line x1="6" y1="6" x2="18" y2="18" />
												</svg>
											</button>
										</div>
									{:else}
										<button
											type="button"
											class="showcase-slot empty"
											onclick={() => openGamePicker(slotIdx)}
										>
											<span class="slot-plus" aria-hidden="true">+</span>
											<span class="slot-empty-label">{$i18n.t('stats.showcase.emptyGameSlot')}</span>
										</button>
									{/if}
								{/each}
							</div>
						</div>

						<!-- Rarest Achievements Column -->
						<div class="showcase-group">
							<div class="showcase-group-header">
								<h3>
									<svg
										width="14"
										height="14"
										viewBox="0 0 24 24"
										fill="none"
										stroke="currentColor"
										stroke-width="2"
										aria-hidden="true"
									>
										<path d="M6 3h12l4 6-10 13L2 9Z" />
										<path d="M11 3 8 9l4 13 4-13-3-6" />
										<path d="M2 9h20" />
									</svg>
									{$i18n.t('stats.showcase.rareAchievementsTitle')}
								</h3>
								<span class="showcase-count-pill tabular-nums"
									>{validShowcaseAchievements.length}/3</span
								>
							</div>
							<div class="showcase-slots">
								{#each SHOWCASE_SLOTS as slotIdx (slotIdx)}
									{@const slotAch = validShowcaseAchievements[slotIdx] ?? null}
									{#if slotAch}
										{@const palette = getRarityPalette(slotAch.ach.completionpercentage)}
										{@const achIcon =
											slotAch.ach.icon && !failedImages.has(slotAch.ach.icon)
												? slotAch.ach.icon
												: null}
										<div class="showcase-slot filled">
											<button
												type="button"
												class="showcase-slot-main"
												onclick={() => openAchPicker(slotIdx)}
											>
												{#if achIcon}
													<img
														class="slot-ach-icon"
														src={achIcon}
														alt={slotAch.ach.name || slotAch.ach.key}
														loading="lazy"
														decoding="async"
														onerror={() => failedImages.add(achIcon)}
													/>
												{:else}
													<div
														class="slot-ach-fallback"
														style="color: {palette.color}; background: {palette.bg};"
														aria-hidden="true"
													>
														<svg
															width="18"
															height="18"
															viewBox="0 0 24 24"
															fill="none"
															stroke="currentColor"
															stroke-width="2"
														>
															<path
																d="M6 9H4.5a2.5 2.5 0 0 1 0-5H6M18 9h1.5a2.5 2.5 0 0 0 0-5H18M4 22h16M10 14.66V17c0 .55.47.98.97 1.21C11.47 18.44 12 19 12 19s.53-.56 1.03-.79c.5-.23.97-.66.97-1.21v-2.34M7 2h10a1 1 0 0 1 1 1v12a1 1 0 0 1-1 1H7a1 1 0 0 1-1-1V3a1 1 0 0 1 1-1Z"
															/>
														</svg>
													</div>
												{/if}
												<div class="slot-info">
													<span class="slot-title">{slotAch.ach.name || slotAch.ach.key}</span>
													<span class="slot-meta">{slotAch.game.name}</span>
												</div>
												<span
													class="slot-rarity-badge tabular-nums"
													style="color: {palette.color}; background: {palette.bg}; border-color: {palette.border};"
												>
													{formatRarityPct(slotAch.ach.completionpercentage)}
												</span>
											</button>
											<button
												type="button"
												class="slot-remove-btn"
												title={$i18n.t('stats.showcase.removeSlot')}
												aria-label={$i18n.t('stats.showcase.removeSlot')}
												disabled={showcaseSaving}
												onclick={() => void removeAchSlot(slotIdx)}
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
													<line x1="18" y1="6" x2="6" y2="18" />
													<line x1="6" y1="6" x2="18" y2="18" />
												</svg>
											</button>
										</div>
									{:else}
										<button
											type="button"
											class="showcase-slot empty"
											onclick={() => openAchPicker(slotIdx)}
										>
											<span class="slot-plus" aria-hidden="true">+</span>
											<span class="slot-empty-label">{$i18n.t('stats.showcase.emptyAchSlot')}</span>
										</button>
									{/if}
								{/each}
							</div>
						</div>
					</div>
				</section>

				<!-- Main Grid Row -->
				<div class="card top-games">
					<div class="card-header">
						<h2>{$i18n.t('stats.topGamesByProgress')}</h2>
						<span class="count">{$i18n.t('stats.gamesCount', { count: $games.length })}</span>
					</div>
					<div class="games-progress-list">
						{#each topGames as game (game.id)}
							<div class="game-progress-row">
								<div class="game-info">
									<span class="name">{game.name}</span>
									{#if game.progression === 100}
										<span class="completed-tag tabular-nums">100%</span>
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
							{#each podium as game, i (game.id)}
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
							>
							{$i18n.t('stats.rarestAchievement')}</span
						>
						{#if rarestAchievement}
							<div class="rarity-item">
								{#if rarestAchievement.icon}
									<img
										src={rarestAchievement.icon}
										alt={rarestAchievement.name}
										loading="lazy"
										decoding="async"
										onerror={(e) => {
											const t = e.target as HTMLImageElement;
											t.style.display = 'none';
										}}
									/>
								{:else}
									<div class="rarity-icon-placeholder">
										<svg
											width="18"
											height="18"
											viewBox="0 0 24 24"
											fill="none"
											stroke="currentColor"
											stroke-width="2"
											stroke-linecap="round"
											stroke-linejoin="round"
											aria-hidden="true"
										>
											<path
												d="M6 9H4.5a2.5 2.5 0 0 1 0-5H6M18 9h1.5a2.5 2.5 0 0 0 0-5H18M4 22h16M10 14.66V17c0 .55.47.98.97 1.21C11.47 18.44 12 19 12 19s.53-.56 1.03-.79c.5-.23.97-.66.97-1.21v-2.34M7 2h10a1 1 0 0 1 1 1v12a1 1 0 0 1-1 1H7a1 1 0 0 1-1-1V3a1 1 0 0 1 1-1Z"
											/>
										</svg>
									</div>
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
							>
							{$i18n.t('stats.bestWeek')}</span
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
							>{$i18n.t('stats.achievementsOver52Weeks', {
								count: $totalUnlockedAchievements
							})}</span
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
										<button
											onclick={() => currentWindowOffset++}
											aria-label={$i18n.t('stats.previous')}
										>
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
								<img src={a.icon} alt={a.name} loading="lazy" decoding="async" />
								<div class="info">
									<div class="name-row">
										<span class="name">{a.name}</span>
										{#if a.completionpercentage && Number.isFinite(parseFloat(a.completionpercentage))}
											<span class="rarity-badge small">{a.completionpercentage}%</span>
										{/if}
									</div>
									<span class="game">{a.gameName}</span>
									<p class="desc">{a.desc}</p>
								</div>
								<span class="time"
									><svg
										width="12"
										height="12"
										viewBox="0 0 24 24"
										fill="none"
										stroke="currentColor"
										stroke-width="2"
										stroke-linecap="round"
										stroke-linejoin="round"
										aria-hidden="true"
									>
										<circle cx="12" cy="12" r="10" />
										<polyline points="12 6 12 12 16 14" />
									</svg>{new Date((a.unlocked_time ?? 0) * 1000).toLocaleDateString($i18n.locale, {
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

	{#if pickerState}
		<div class="showcase-modal-overlay">
			<button
				type="button"
				class="showcase-modal-backdrop"
				aria-label={$i18n.t('button.close')}
				tabindex="-1"
				onclick={closePicker}
			></button>
			<div
				class="showcase-modal"
				role="dialog"
				aria-modal="true"
				aria-labelledby="showcase-picker-title"
				tabindex="-1"
				bind:this={pickerModalEl}
			>
				<div class="showcase-modal-header">
					<h2 id="showcase-picker-title">
						{pickerState.kind === 'game'
							? $i18n.t('stats.showcase.selectGameModalTitle')
							: $i18n.t('stats.showcase.selectAchModalTitle')}
					</h2>
					<button
						type="button"
						class="showcase-modal-close"
						aria-label={$i18n.t('button.close')}
						onclick={closePicker}
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
							<line x1="18" y1="6" x2="6" y2="18" />
							<line x1="6" y1="6" x2="18" y2="18" />
						</svg>
					</button>
				</div>

				{#if pickerState.kind === 'game'}
					{#if platinumGameItems.length === 0}
						<div class="showcase-modal-empty">
							<p>{$i18n.t('stats.showcase.noPlatinumYet')}</p>
						</div>
					{:else}
						<div class="showcase-modal-search">
							<svg
								width="15"
								height="15"
								viewBox="0 0 24 24"
								fill="none"
								stroke="currentColor"
								stroke-width="2"
								aria-hidden="true"
							>
								<circle cx="11" cy="11" r="8" />
								<path d="m21 21-4.3-4.3" />
							</svg>
							<input
								type="search"
								bind:this={pickerSearchInputEl}
								bind:value={pickerSearch}
								aria-label={$i18n.t('stats.showcase.searchPlaceholder')}
								placeholder={$i18n.t('stats.showcase.searchPlaceholder')}
							/>
						</div>

						<div class="showcase-modal-list">
							{#if filteredPickerGames.length === 0}
								<div class="showcase-modal-empty">
									<p>{$i18n.t('games.results', { count: 0 })}</p>
								</div>
							{:else}
								{#each filteredPickerGames as item (item.game.id)}
									{@const isCurrent =
										validShowcaseGames[pickerState.slotIndex]?.game.id === item.game.id}
									{@const isUsedElsewhere = validShowcaseGames.some(
										(g, idx) => idx !== pickerState?.slotIndex && g.game.id === item.game.id
									)}
									{@const thumbUrl = getBestGameImage(item.game)}
									<button
										type="button"
										class="picker-item"
										class:current={isCurrent}
										disabled={isUsedElsewhere || showcaseSaving}
										onclick={() => void selectGameForSlot(item.game.id)}
									>
										{#if thumbUrl}
											<img
												class="picker-game-thumb"
												src={thumbUrl}
												alt={item.game.name}
												loading="lazy"
												decoding="async"
												onerror={() => {
													if (thumbUrl) failedImages.add(thumbUrl);
												}}
											/>
										{:else}
											<div class="picker-game-fallback" aria-hidden="true">
												{getInitials(item.game.name)}
											</div>
										{/if}
										<div class="picker-item-info">
											<span class="picker-item-title">{item.game.name}</span>
											<span class="picker-item-sub tabular-nums">
												{item.progress.unlockedBase}/{item.progress.totalBase}
											</span>
										</div>
										<span class="slot-platinum-badge tabular-nums">100%</span>
									</button>
								{/each}
							{/if}
						</div>
					{/if}
				{:else if unlockedAchievementsByRarity.length === 0}
					<div class="showcase-modal-empty">
						<p>{$i18n.t('stats.showcase.noUnlockedAchYet')}</p>
					</div>
				{:else}
					<div class="showcase-modal-search">
						<svg
							width="15"
							height="15"
							viewBox="0 0 24 24"
							fill="none"
							stroke="currentColor"
							stroke-width="2"
							aria-hidden="true"
						>
							<circle cx="11" cy="11" r="8" />
							<path d="m21 21-4.3-4.3" />
						</svg>
						<input
							type="search"
							bind:this={pickerSearchInputEl}
							bind:value={pickerSearch}
							aria-label={$i18n.t('stats.showcase.searchPlaceholder')}
							placeholder={$i18n.t('stats.showcase.searchPlaceholder')}
						/>
					</div>

					<div class="showcase-modal-list">
						{#if filteredPickerAchievements.length === 0}
							<div class="showcase-modal-empty">
								<p>{$i18n.t('games.results', { count: 0 })}</p>
							</div>
						{:else}
							{#each filteredPickerAchievements as item (item.game.id + ':' + item.achKey.toLowerCase())}
								{@const currentEntry = validShowcaseAchievements[pickerState.slotIndex]}
								{@const isCurrent = Boolean(
									currentEntry &&
										currentEntry.game.id === item.game.id &&
										currentEntry.achKey.toLowerCase() === item.achKey.toLowerCase()
								)}
								{@const isUsedElsewhere = validShowcaseAchievements.some(
									(entry, idx) =>
										idx !== pickerState?.slotIndex &&
										entry.game.id === item.game.id &&
										entry.achKey.toLowerCase() === item.achKey.toLowerCase()
								)}
								{@const palette = getRarityPalette(item.ach.completionpercentage)}
								{@const achIcon =
									item.ach.icon && !failedImages.has(item.ach.icon) ? item.ach.icon : null}
								<button
									type="button"
									class="picker-item"
									class:current={isCurrent}
									disabled={isUsedElsewhere || showcaseSaving}
									onclick={() => void selectAchForSlot(item)}
								>
									{#if achIcon}
										<img
											class="picker-ach-icon"
											src={achIcon}
											alt={item.ach.name || item.ach.key}
											loading="lazy"
											decoding="async"
											onerror={() => {
												if (achIcon) failedImages.add(achIcon);
											}}
										/>
									{:else}
										<div
											class="picker-ach-fallback"
											style="color: {palette.color}; background: {palette.bg};"
											aria-hidden="true"
										>
											{getInitials(item.ach.name || item.ach.key)}
										</div>
									{/if}
									<div class="picker-item-info">
										<span class="picker-item-title">{item.ach.name || item.ach.key}</span>
										<span class="picker-item-sub">{item.game.name}</span>
									</div>
									<span
										class="slot-rarity-badge tabular-nums"
										style="color: {palette.color}; background: {palette.bg}; border-color: {palette.border};"
									>
										{formatRarityPct(item.ach.completionpercentage)}
									</span>
								</button>
							{/each}
						{/if}
					</div>
				{/if}
			</div>
		</div>
	{/if}
</main>

<style>
	.stats-page {
		display: flex;
		flex-direction: column;
		background: var(--bg-panel);
		color: var(--text-primary);
		height: 100%;
		min-width: 0;
		max-width: 100%;
		overflow: hidden;
		overflow-x: hidden;
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
		color: var(--text-primary);
	}

	.tabs {
		display: flex;
		gap: 8px;
		background: var(--surface-2);
		border: 1px solid var(--border-soft);
		padding: 4px;
		border-radius: 8px;
		width: fit-content;
	}

	.tabs button {
		background: transparent;
		border: none;
		color: var(--text-muted);
		padding: 6px 16px;
		border-radius: 6px;
		font-size: 13px;
		cursor: pointer;
		transition: all 0.2s;
	}

	.tabs button:hover {
		color: var(--text-primary);
	}

	.tabs button.active {
		background: color-mix(in srgb, var(--accent, #c8a96e) 14%, var(--surface-1));
		color: var(--accent, #c8a96e);
	}

	.date-now {
		color: var(--text-muted);
		font-size: 12px;
		font-variant-numeric: tabular-nums;
	}

	.scrollable {
		flex: 1;
		min-width: 0;
		overflow-x: hidden;
		overflow-y: auto;
		padding: 0 32px 32px;
	}

	.grid-overview {
		display: grid;
		grid-template-columns: repeat(6, minmax(0, 1fr));
		grid-auto-rows: auto;
		gap: 12px;
		width: 100%;
		min-width: 0;
	}

	.card {
		background: var(--surface-1);
		border: 1px solid var(--border-soft);
		border-radius: 12px;
		padding: 20px;
		min-width: 0;
		box-sizing: border-box;
		box-shadow: var(--shadow-card, none);
	}

	.stat-summary {
		display: flex;
		align-items: center;
		gap: 12px;
		padding: 14px 16px;
		min-width: 0;
		box-sizing: border-box;
	}

	.icon-circle {
		width: 40px;
		height: 40px;
		border-radius: 10px;
		display: flex;
		align-items: center;
		justify-content: center;
		flex-shrink: 0;
		background: var(--surface-2);
	}

	.icon-circle.games {
		color: var(--accent, #c8a96e);
		background: color-mix(in srgb, var(--accent, #c8a96e) 12%, transparent);
	}
	.icon-circle.trophy {
		color: #3ddc84;
		background: rgba(61, 220, 132, 0.12);
	}
	.icon-circle.rate {
		color: #a855f7;
		background: rgba(168, 85, 247, 0.12);
	}
	.icon-circle.gamerscore {
		color: var(--accent, #c8a96e);
		background: color-mix(in srgb, var(--accent, #c8a96e) 14%, transparent);
	}
	.icon-circle.playtime {
		color: #4ac8ff;
		background: rgba(74, 200, 255, 0.12);
	}
	.icon-circle.remaining {
		color: #f4b860;
		background: rgba(244, 184, 96, 0.12);
	}

	.card-info {
		display: flex;
		flex-direction: column;
		min-width: 0;
		overflow: hidden;
	}
	.card-info .label {
		font-size: 10px;
		font-weight: 600;
		color: var(--text-muted);
		letter-spacing: 0.05em;
		white-space: nowrap;
		overflow: hidden;
		text-overflow: ellipsis;
	}
	.card-info .value {
		font-size: clamp(18px, 1.8vw, 24px);
		font-weight: 700;
		margin: 2px 0;
		color: var(--text-primary);
		font-variant-numeric: tabular-nums;
	}
	.card-info .sub {
		font-size: 11px;
		color: var(--text-muted);
		font-variant-numeric: tabular-nums;
		white-space: nowrap;
		overflow: hidden;
		text-overflow: ellipsis;
	}

	.top-games {
		grid-column: 1 / 5;
		min-width: 0;
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
		color: var(--text-primary);
	}
	.card-header .count {
		font-size: 11px;
		color: var(--text-muted);
		font-variant-numeric: tabular-nums;
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
		color: var(--text-secondary);
	}
	.game-progress-row .completed-tag {
		font-size: 10px;
		font-weight: 700;
		color: var(--completed, #e4b441);
		background: color-mix(in srgb, var(--completed, #e4b441) 14%, transparent);
		padding: 1px 6px;
		border-radius: 999px;
		font-variant-numeric: tabular-nums;
	}

	.progress-bar-wrap {
		display: flex;
		align-items: center;
		gap: 12px;
	}
	.progress-bar {
		flex: 1;
		height: 12px;
		background: var(--surface-3);
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
		font-variant-numeric: tabular-nums;
	}

	.side-column {
		grid-column: 5 / 7;
		min-width: 0;
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
		color: var(--text-muted);
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
		box-shadow: 0 0 0 1px var(--border-soft);
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
		stroke: var(--surface-3);
	}
	.donut-progress {
		stroke: var(--accent, #c8a96e);
		stroke-linecap: round;
		transition: stroke-dasharray 0.6s ease;
	}
	.donut-inner {
		width: 82%;
		height: 82%;
		background: var(--surface-1);
		border-radius: 50%;
		display: flex;
		flex-direction: column;
		align-items: center;
		justify-content: center;
	}
	.donut-val {
		font-size: 24px;
		font-weight: 700;
		color: var(--text-primary);
		font-variant-numeric: tabular-nums;
	}
	.donut-label {
		font-size: 10px;
		color: var(--text-muted);
	}

	.unlocked-count {
		text-align: center;
	}
	.unlocked-count .val {
		font-size: 18px;
		font-weight: 600;
		display: block;
		color: var(--text-primary);
		font-variant-numeric: tabular-nums;
	}
	.unlocked-count .lbl {
		font-size: 10px;
		color: var(--text-muted);
	}

	.podium h2 {
		font-size: 11px;
		font-weight: 600;
		color: var(--text-muted);
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
		background: color-mix(in srgb, var(--accent, #c8a96e) 14%, transparent);
		color: var(--accent, #c8a96e);
		font-variant-numeric: tabular-nums;
	}
	.podium-item .name {
		flex: 1;
		font-size: 13px;
		color: var(--text-secondary);
		white-space: nowrap;
		overflow: hidden;
		text-overflow: ellipsis;
	}
	.podium-item .pct {
		font-size: 12px;
		font-weight: 600;
		color: var(--text-primary);
		border-bottom: 2px solid var(--accent, #c8a96e);
		font-variant-numeric: tabular-nums;
	}

	.card-subtitle {
		font-size: 11px;
		font-weight: 600;
		color: var(--text-muted);
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
		background: color-mix(in srgb, var(--accent, #c8a96e) 14%, transparent);
		color: var(--accent, #c8a96e);
		display: flex;
		align-items: center;
		justify-content: center;
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
		color: var(--text-primary);
		white-space: nowrap;
		overflow: hidden;
		text-overflow: ellipsis;
	}
	.rarity-item .game {
		font-size: 11px;
		color: var(--text-muted);
		white-space: nowrap;
		overflow: hidden;
		text-overflow: ellipsis;
	}
	.rarity-badge {
		font-size: 11px;
		font-weight: 700;
		color: color-mix(in srgb, #e59424 80%, var(--text-primary));
		background: rgba(244, 184, 96, 0.14);
		padding: 2px 6px;
		border-radius: 4px;
		font-variant-numeric: tabular-nums;
	}

	.best-week-val .val {
		font-size: 32px;
		font-weight: 700;
		color: var(--text-primary);
		display: block;
		font-variant-numeric: tabular-nums;
	}
	.best-week-val .lbl {
		font-size: 11px;
		color: var(--text-muted);
	}

	.activity-view {
		display: flex;
		flex-direction: column;
		gap: 16px;
		min-width: 0;
		max-width: 100%;
	}
	.heatmap-card .sub {
		font-size: 12px;
		color: var(--text-muted);
		font-variant-numeric: tabular-nums;
	}
	.heatmap-container {
		margin-top: 20px;
		min-width: 0;
		max-width: 100%;
	}

	.heatmap-wrapper {
		display: flex;
		gap: 8px;
		position: relative;
		min-width: 0;
		max-width: 100%;
	}

	.heatmap-y-labels {
		display: flex;
		flex-direction: column;
		justify-content: space-between;
		flex-shrink: 0;
		padding: 18px 0 2px;
		font-size: 9px;
		color: var(--text-muted);
		height: 77px;
		margin-top: 18px;
	}

	.heatmap-scroll-area {
		flex: 1;
		min-width: 0;
		max-width: 100%;
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
		color: var(--text-muted);
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
		background: var(--surface-3);
		border-radius: 2px;
		transition: transform 0.1s;
	}
	.heatmap-cell:hover:not(.is-future) {
		transform: scale(1.2);
		z-index: 10;
		outline: 1px solid var(--border-soft);
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
		color: var(--text-muted);
		margin-top: 12px;
		justify-content: flex-end;
	}

	.charts-row {
		display: grid;
		grid-template-columns: repeat(2, minmax(0, 1fr));
		gap: 16px;
		min-width: 0;
		max-width: 100%;
	}
	.chart-card {
		min-width: 0;
		max-width: 100%;
	}
	.chart-card .sub {
		font-size: 11px;
		color: var(--text-muted);
		margin: 4px 0 20px;
	}

	.chart-header-row {
		display: flex;
		justify-content: space-between;
		align-items: flex-start;
		flex-wrap: wrap;
		gap: 10px;
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
		background: var(--surface-2);
		padding: 2px;
		border-radius: 6px;
	}

	.window-nav button {
		background: transparent;
		border: none;
		color: var(--text-muted);
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
		background: var(--surface-hover);
		color: var(--text-primary);
	}

	.window-nav button:disabled {
		opacity: 0.2;
		cursor: default;
	}

	.timeframe-selector {
		display: flex;
		gap: 4px;
		background: var(--surface-2);
		padding: 2px;
		border-radius: 6px;
	}

	.timeframe-selector button {
		background: transparent;
		border: none;
		color: var(--text-muted);
		padding: 4px 8px;
		border-radius: 4px;
		font-size: 10px;
		font-weight: 600;
		cursor: pointer;
		transition: all 0.2s;
		text-transform: uppercase;
	}

	.timeframe-selector button:hover {
		color: var(--text-primary);
	}

	.timeframe-selector button.active {
		background: var(--surface-1);
		color: var(--accent, #c8a96e);
	}

	.chart-container {
		height: 180px;
		position: relative;
		padding-bottom: 20px;
		border-bottom: 1px solid var(--border-soft);
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
		background: var(--surface-3);
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
		border-bottom: 1px solid var(--border-soft);
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
		color: var(--text-primary);
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
		color: var(--text-muted);
		line-height: 1.4;
		display: -webkit-box;
		-webkit-line-clamp: 2;
		line-clamp: 2;
		-webkit-box-orient: vertical;
		overflow: hidden;
	}
	.recent-item .time {
		font-size: 11px;
		color: var(--text-muted);
		white-space: nowrap;
		margin-top: 4px;
		display: inline-flex;
		align-items: center;
		gap: 5px;
		font-variant-numeric: tabular-nums;
	}

	.custom-tooltip {
		position: fixed;
		background: var(--surface-1);
		border: 1px solid var(--border-soft);
		padding: 12px;
		border-radius: 8px;
		z-index: 1000;
		pointer-events: none;
		box-shadow: var(--shadow-elevated, 0 8px 24px rgba(0, 0, 0, 0.35));
		min-width: 150px;
		max-width: 250px;
	}

	.tooltip-title {
		font-size: 11px;
		color: var(--text-muted);
		margin-bottom: 4px;
		font-variant-numeric: tabular-nums;
	}
	.tooltip-value {
		font-size: 16px;
		font-weight: 700;
		color: var(--text-primary);
		margin-bottom: 8px;
		font-variant-numeric: tabular-nums;
	}
	.tooltip-sub {
		font-size: 12px;
		color: var(--text-secondary);
		margin-bottom: 8px;
	}

	.tooltip-list {
		list-style: none;
		padding: 0;
		margin: 0;
		border-top: 1px solid var(--border-soft);
		padding-top: 8px;
	}

	.tooltip-list li {
		font-size: 12px;
		color: var(--text-secondary);
		margin-bottom: 4px;
		white-space: nowrap;
		overflow: hidden;
		text-overflow: ellipsis;
	}

	.tooltip-list li.more {
		color: var(--text-muted);
		font-style: italic;
	}

	.empty {
		font-size: 12px;
		color: var(--text-muted);
	}

	/* Profile Showcase / Trophy Cabinet */
	.showcase-section {
		grid-column: 1 / -1;
		display: flex;
		flex-direction: column;
		gap: 18px;
		border-color: color-mix(in srgb, var(--accent, #c8a96e) 24%, var(--border-soft));
		background:
			radial-gradient(
				circle at 0% 0%,
				color-mix(in srgb, var(--accent, #c8a96e) 7%, transparent),
				transparent 52%
			),
			var(--surface-1);
	}

	.showcase-header {
		display: flex;
		align-items: center;
		justify-content: space-between;
		flex-wrap: wrap;
		gap: 14px;
	}

	.showcase-title-group {
		display: flex;
		align-items: center;
		gap: 12px;
		min-width: 0;
	}

	.showcase-icon-badge {
		width: 38px;
		height: 38px;
		border-radius: 10px;
		display: flex;
		align-items: center;
		justify-content: center;
		flex-shrink: 0;
		color: var(--accent, #c8a96e);
		background: color-mix(in srgb, var(--accent, #c8a96e) 14%, transparent);
		border: 1px solid color-mix(in srgb, var(--accent, #c8a96e) 28%, transparent);
	}

	.showcase-heading-text {
		min-width: 0;
	}

	.showcase-heading-text h2 {
		font-size: 16px;
		font-weight: 700;
		color: var(--text-primary);
		margin: 0 0 2px 0;
	}

	.showcase-subtitle {
		font-size: 12px;
		color: var(--text-muted);
		margin: 0;
	}

	.showcase-actions {
		display: flex;
		align-items: center;
		flex-wrap: wrap;
		gap: 8px;
	}

	.showcase-toast {
		font-size: 12px;
		font-weight: 600;
		color: #3ddc84;
		background: rgba(61, 220, 132, 0.12);
		border: 1px solid rgba(61, 220, 132, 0.28);
		padding: 5px 10px;
		border-radius: 8px;
	}

	.showcase-btn {
		display: inline-flex;
		align-items: center;
		gap: 7px;
		padding: 7px 13px;
		border-radius: 8px;
		font-size: 12px;
		font-weight: 600;
		cursor: pointer;
		transition: all 0.2s;
	}

	.showcase-btn.secondary {
		background: var(--surface-2);
		border: 1px solid var(--border-soft);
		color: var(--text-primary);
	}

	.showcase-btn.secondary:hover:not(:disabled) {
		background: var(--surface-hover);
		border-color: color-mix(in srgb, var(--accent, #c8a96e) 40%, var(--border-soft));
	}

	.showcase-btn.primary {
		background: color-mix(in srgb, var(--accent, #c8a96e) 18%, var(--surface-2));
		border: 1px solid color-mix(in srgb, var(--accent, #c8a96e) 48%, transparent);
		color: var(--accent, #c8a96e);
	}

	.showcase-btn.primary:hover:not(:disabled) {
		background: var(--accent, #c8a96e);
		color: var(--accent-text, #111111);
	}

	.showcase-btn:disabled {
		opacity: 0.45;
		cursor: not-allowed;
	}

	.showcase-columns {
		display: grid;
		grid-template-columns: repeat(2, minmax(0, 1fr));
		gap: 16px;
		min-width: 0;
	}

	.showcase-group {
		display: flex;
		flex-direction: column;
		gap: 10px;
		min-width: 0;
		background: var(--surface-2);
		border: 1px solid var(--border-soft);
		border-radius: 10px;
		padding: 14px;
	}

	.showcase-group-header {
		display: flex;
		align-items: center;
		justify-content: space-between;
		gap: 8px;
	}

	.showcase-group-header h3 {
		display: inline-flex;
		align-items: center;
		gap: 7px;
		font-size: 12px;
		font-weight: 700;
		letter-spacing: 0.03em;
		text-transform: uppercase;
		color: var(--text-secondary);
		margin: 0;
	}

	.showcase-group-header h3 svg {
		color: var(--accent, #c8a96e);
		flex-shrink: 0;
	}

	.showcase-count-pill {
		font-size: 11px;
		font-weight: 700;
		color: var(--text-muted);
		background: var(--surface-1);
		border: 1px solid var(--border-soft);
		padding: 2px 8px;
		border-radius: 999px;
	}

	.showcase-slots {
		display: flex;
		flex-direction: column;
		gap: 8px;
	}

	.showcase-slot.filled {
		display: flex;
		align-items: stretch;
		gap: 6px;
		min-width: 0;
	}

	.showcase-slot-main {
		flex: 1;
		min-width: 0;
		display: flex;
		align-items: center;
		gap: 12px;
		padding: 10px 12px;
		border-radius: 10px;
		background: var(--surface-1);
		border: 1px solid var(--border-soft);
		color: inherit;
		text-align: left;
		cursor: pointer;
		transition:
			border-color 0.2s,
			background 0.2s,
			transform 0.15s;
	}

	.showcase-slot-main:hover {
		background: var(--surface-hover);
		border-color: color-mix(in srgb, var(--accent, #c8a96e) 45%, var(--border-soft));
	}

	.slot-game-thumb,
	.slot-game-fallback {
		width: 56px;
		height: 38px;
		border-radius: 7px;
		flex-shrink: 0;
		object-fit: cover;
		background: var(--surface-3);
	}

	.slot-game-fallback,
	.slot-ach-fallback,
	.picker-game-fallback,
	.picker-ach-fallback {
		display: flex;
		align-items: center;
		justify-content: center;
		font-size: 12px;
		font-weight: 800;
		color: var(--accent, #c8a96e);
		background: color-mix(in srgb, var(--accent, #c8a96e) 14%, transparent);
	}

	.slot-ach-icon,
	.slot-ach-fallback {
		width: 40px;
		height: 40px;
		border-radius: 8px;
		flex-shrink: 0;
		object-fit: cover;
	}

	.slot-info {
		flex: 1;
		min-width: 0;
		display: flex;
		flex-direction: column;
		gap: 2px;
	}

	.slot-title {
		font-size: 13px;
		font-weight: 600;
		color: var(--text-primary);
		white-space: nowrap;
		overflow: hidden;
		text-overflow: ellipsis;
	}

	.slot-meta {
		font-size: 11px;
		color: var(--text-muted);
		white-space: nowrap;
		overflow: hidden;
		text-overflow: ellipsis;
	}

	.slot-platinum-badge {
		font-size: 11px;
		font-weight: 700;
		color: var(--completed, #e4b441);
		background: color-mix(in srgb, var(--completed, #e4b441) 14%, transparent);
		border: 1px solid color-mix(in srgb, var(--completed, #e4b441) 36%, transparent);
		padding: 2px 8px;
		border-radius: 999px;
		flex-shrink: 0;
	}

	.slot-rarity-badge {
		font-size: 11px;
		font-weight: 700;
		border: 1px solid transparent;
		padding: 2px 8px;
		border-radius: 999px;
		flex-shrink: 0;
	}

	.slot-remove-btn {
		width: 34px;
		border-radius: 10px;
		background: var(--surface-1);
		border: 1px solid var(--border-soft);
		color: var(--text-muted);
		display: flex;
		align-items: center;
		justify-content: center;
		cursor: pointer;
		flex-shrink: 0;
		transition: all 0.2s;
	}

	.slot-remove-btn:hover:not(:disabled) {
		color: #ef4444;
		border-color: rgba(239, 68, 68, 0.4);
		background: rgba(239, 68, 68, 0.1);
	}

	.slot-remove-btn:disabled {
		opacity: 0.45;
		cursor: not-allowed;
	}

	.showcase-slot.empty {
		width: 100%;
		min-height: 60px;
		display: flex;
		align-items: center;
		justify-content: center;
		gap: 8px;
		padding: 10px 14px;
		border-radius: 10px;
		background: transparent;
		border: 1px dashed var(--border-soft);
		color: var(--text-muted);
		font-size: 12px;
		font-weight: 600;
		cursor: pointer;
		transition: all 0.2s;
	}

	.showcase-slot.empty:hover {
		color: var(--accent, #c8a96e);
		border-color: color-mix(in srgb, var(--accent, #c8a96e) 55%, transparent);
		background: color-mix(in srgb, var(--accent, #c8a96e) 6%, transparent);
	}

	.slot-plus {
		width: 22px;
		height: 22px;
		border-radius: 999px;
		background: var(--surface-1);
		border: 1px solid var(--border-soft);
		display: inline-flex;
		align-items: center;
		justify-content: center;
		font-size: 14px;
		font-weight: 700;
		line-height: 1;
	}

	/* Showcase Picker Modal */
	.showcase-modal-overlay {
		position: fixed;
		inset: 0;
		z-index: 1200;
		display: flex;
		align-items: center;
		justify-content: center;
		padding: 24px;
	}

	.showcase-modal-backdrop {
		position: absolute;
		inset: 0;
		background: rgba(0, 0, 0, 0.72);
		backdrop-filter: blur(8px);
		border: none;
		padding: 0;
		cursor: default;
	}

	.showcase-modal-backdrop:active {
		transform: none;
	}

	.showcase-modal {
		position: relative;
		z-index: 1;
		width: min(560px, 100%);
		max-height: min(640px, 84vh);
		background: var(--surface-1);
		border: 1px solid color-mix(in srgb, var(--accent, #c8a96e) 32%, var(--border-soft));
		border-radius: 16px;
		box-shadow: var(--shadow-elevated, 0 24px 60px rgba(0, 0, 0, 0.65));
		display: flex;
		flex-direction: column;
		overflow: hidden;
	}

	.showcase-modal-header {
		display: flex;
		align-items: center;
		justify-content: space-between;
		gap: 12px;
		padding: 18px 20px;
		border-bottom: 1px solid var(--border-soft);
	}

	.showcase-modal-header h2 {
		font-size: 16px;
		font-weight: 700;
		color: var(--text-primary);
		margin: 0;
	}

	.showcase-modal-close {
		width: 32px;
		height: 32px;
		border-radius: 8px;
		background: var(--surface-2);
		border: 1px solid var(--border-soft);
		color: var(--text-muted);
		display: flex;
		align-items: center;
		justify-content: center;
		cursor: pointer;
		transition: all 0.2s;
	}

	.showcase-modal-close:hover {
		background: var(--surface-hover);
		color: var(--text-primary);
	}

	.showcase-modal-search {
		display: flex;
		align-items: center;
		gap: 8px;
		margin: 14px 20px 6px;
		padding: 8px 12px;
		border-radius: 10px;
		background: var(--surface-2);
		border: 1px solid var(--border-soft);
		color: var(--text-muted);
	}

	.showcase-modal-search:focus-within {
		border-color: var(--accent, #c8a96e);
	}

	.showcase-modal-search input {
		flex: 1;
		min-width: 0;
		background: transparent;
		border: none;
		color: var(--text-primary);
		font-size: 13px;
		outline: none;
	}

	.showcase-modal-list {
		flex: 1;
		overflow-y: auto;
		padding: 10px 20px 20px;
		display: flex;
		flex-direction: column;
		gap: 8px;
	}

	.showcase-modal-empty {
		padding: 36px 20px;
		text-align: center;
		color: var(--text-muted);
		font-size: 13px;
	}

	.picker-item {
		width: 100%;
		display: flex;
		align-items: center;
		gap: 12px;
		padding: 10px 12px;
		border-radius: 10px;
		background: var(--surface-2);
		border: 1px solid var(--border-soft);
		color: inherit;
		text-align: left;
		cursor: pointer;
		transition: all 0.15s;
	}

	.picker-item:hover:not(:disabled) {
		background: var(--surface-hover);
		border-color: color-mix(in srgb, var(--accent, #c8a96e) 45%, var(--border-soft));
	}

	.picker-item.current {
		border-color: var(--accent, #c8a96e);
		background: color-mix(in srgb, var(--accent, #c8a96e) 12%, var(--surface-2));
	}

	.picker-item:disabled {
		opacity: 0.42;
		cursor: not-allowed;
	}

	.picker-game-thumb,
	.picker-game-fallback {
		width: 56px;
		height: 36px;
		border-radius: 6px;
		flex-shrink: 0;
		object-fit: cover;
	}

	.picker-ach-icon,
	.picker-ach-fallback {
		width: 40px;
		height: 40px;
		border-radius: 8px;
		flex-shrink: 0;
		object-fit: cover;
	}

	.picker-item-info {
		flex: 1;
		min-width: 0;
		display: flex;
		flex-direction: column;
		gap: 2px;
	}

	.picker-item-title {
		font-size: 13px;
		font-weight: 600;
		color: var(--text-primary);
		white-space: nowrap;
		overflow: hidden;
		text-overflow: ellipsis;
	}

	.picker-item-sub {
		font-size: 11px;
		color: var(--text-muted);
		white-space: nowrap;
		overflow: hidden;
		text-overflow: ellipsis;
	}

	.scrollable::-webkit-scrollbar {
		width: 6px;
	}
	.scrollable::-webkit-scrollbar-track {
		background: transparent;
	}
	.scrollable::-webkit-scrollbar-thumb {
		background: var(--border-soft);
		border-radius: 3px;
	}

	@media (max-width: 1280px) {
		.grid-overview {
			grid-template-columns: repeat(3, minmax(0, 1fr));
		}
		.top-games {
			grid-column: 1 / 3;
		}
		.side-column {
			grid-column: 3 / 4;
		}
	}

	@media (max-width: 1180px) {
		.charts-row {
			grid-template-columns: 1fr;
		}
	}

	@media (max-width: 960px) {
		.showcase-columns {
			grid-template-columns: 1fr;
		}
	}

	@media (max-width: 820px) {
		.stats-header {
			padding: 20px 20px 16px;
		}
		.scrollable {
			padding: 0 20px 24px;
		}
		.grid-overview {
			grid-template-columns: repeat(2, minmax(0, 1fr));
		}
		.top-games,
		.side-column {
			grid-column: 1 / -1;
		}
	}
</style>
