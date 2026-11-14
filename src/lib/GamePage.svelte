<script lang="ts">
	// src/lib/GamePage.svelte
	import { onMount, tick } from 'svelte';
	import { get } from 'svelte/store';
	import { listen } from '@tauri-apps/api/event';
	import { invoke } from '@tauri-apps/api/core';
	import {
		loadAchievements,
		getAchievementPoints,
		formatPlaytime,
		computeGameProgress,
		type Game,
		type Achievement,
		type HltbData,
		games
	} from '$lib/stores/Games.js';
	import { searchQuery, achievementJumpIntent, settingsOpen } from '$lib/stores/ui.js';
	import { goto } from '$app/navigation';
	import { resolve } from '$app/paths';
	import { open } from '@tauri-apps/plugin-shell';
	import { SvelteMap, SvelteSet } from 'svelte/reactivity';
	import { i18n, rarityLabelByIndex } from '$lib/stores/i18n.js';

	import {
		toggleGameFavorite,
		addGameTag,
		removeGameTag,
		exportGame,
		openProfilesFolder,
		exportSummaryPdf,
		selectedGameId
	} from '$lib/stores/selectedGame.js';

	let { game }: { game: Game | null } = $props();

	$effect(() => {
		if (game?.id) {
			selectedGameId.set(game.id);
		}
	});

	let newTag = $state('');

	let allTags = $derived.by(() => {
		const tags = new SvelteSet<string>();
		$games.forEach((g) => g.tags?.forEach((t) => tags.add(t)));
		return Array.from(tags).sort();
	});

	function handleAddTag() {
		if (game && newTag.trim()) {
			addGameTag(game.id, newTag.trim());
			newTag = '';
		}
	}

	let currentIndex = $derived($games.findIndex((g) => g.id === game?.id));
	let prevGame = $derived(currentIndex > 0 ? $games[currentIndex - 1] : null);
	let nextGame = $derived(currentIndex < $games.length - 1 ? $games[currentIndex + 1] : null);

	// Live playtime reactive to store updates
	let gamePlaytimeSeconds = $derived(
		$games.find((g) => g.id === game?.id)?.playtime_seconds ?? game?.playtime_seconds ?? 0
	);
	let editingPlaytime = $state(false);
	let editPlaytimeHours = $state(0);
	let editPlaytimeMins = $state(0);

	function openPlaytimeEditor() {
		const secs = gamePlaytimeSeconds;
		editPlaytimeHours = Math.floor(secs / 3600);
		editPlaytimeMins = Math.floor((secs % 3600) / 60);
		editingPlaytime = true;
	}

	async function saveManualPlaytime() {
		if (!game) return;
		const totalSecs =
			Math.max(0, Math.floor(Number(editPlaytimeHours) || 0)) * 3600 +
			Math.max(0, Math.floor(Number(editPlaytimeMins) || 0)) * 60;
		try {
			await invoke('set_manual_playtime', {
				gameId: game.id,
				playtimeSeconds: totalSecs
			});
			games.update((list) =>
				list.map((g) => (g.id === game.id ? { ...g, playtime_seconds: totalSecs } : g))
			);
			editingPlaytime = false;
		} catch (e) {
			console.error('Failed to save playtime:', e);
		}
	}

	// HowLongToBeat (HLTB) state
	let hltbData = $state<HltbData | null>(null);
	let hltbLoading = $state(false);
	let editingHltb = $state(false);
	let customHltbMain = $state('');
	let customHltbExtra = $state('');
	let customHltb100 = $state('');

	$effect(() => {
		const gid = game?.id;
		const gname = game?.name;
		editingHltb = false;
		editingPlaytime = false;
		editingAchKey = null;
		if (!gid || !gname) {
			hltbData = null;
			return;
		}
		hltbLoading = true;
		void invoke<HltbData>('get_hltb_data', {
			gameId: gid,
			gameName: gname,
			forceRefresh: false
		})
			.then((res) => {
				if (game?.id === gid) {
					hltbData = res;
				}
			})
			.catch((e) => console.error('HLTB error:', e))
			.finally(() => {
				if (game?.id === gid) hltbLoading = false;
			});
	});

	async function refreshHltb() {
		if (!game) return;
		hltbLoading = true;
		try {
			hltbData = await invoke<HltbData>('get_hltb_data', {
				gameId: game.id,
				gameName: game.name,
				forceRefresh: true
			});
		} catch (e) {
			console.error('HLTB refresh error:', e);
		} finally {
			hltbLoading = false;
		}
	}

	function openHltbEditor() {
		customHltbMain = hltbData?.main_hours ? String(hltbData.main_hours) : '';
		customHltbExtra = hltbData?.main_extra_hours ? String(hltbData.main_extra_hours) : '';
		customHltb100 = hltbData?.completionist_hours ? String(hltbData.completionist_hours) : '';
		editingHltb = true;
	}

	async function saveCustomHltb() {
		if (!game) return;
		const parseOpt = (val: string): number | null => {
			const n = parseFloat(String(val).replace(',', '.'));
			return Number.isFinite(n) && n > 0 ? Math.round(n * 10) / 10 : null;
		};
		try {
			hltbData = await invoke<HltbData>('set_custom_hltb_data', {
				gameId: game.id,
				gameName: game.name,
				mainHours: parseOpt(customHltbMain),
				mainExtraHours: parseOpt(customHltbExtra),
				completionistHours: parseOpt(customHltb100)
			});
			editingHltb = false;
		} catch (e) {
			console.error('Failed to save custom HLTB:', e);
		}
	}

	function hltbProgressPct(targetHours: number | null | undefined): number {
		if (!targetHours || targetHours <= 0) return 0;
		const playedHours = gamePlaytimeSeconds / 3600;
		return Math.min(100, Math.max(0, Math.round((playedHours / targetHours) * 100)));
	}

	// Manual Achievement Editor state
	let editingAchKey = $state<string | null>(null);
	let editAchUnlocked = $state(true);
	let editAchDateTime = $state('');
	let savingAchEdit = $state(false);

	function unixToLocalDatetimeInput(unixSecs: number | null | undefined): string {
		const d = unixSecs && unixSecs > 0 ? new Date(unixSecs * 1000) : new Date();
		const pad = (n: number) => String(n).padStart(2, '0');
		return `${d.getFullYear()}-${pad(d.getMonth() + 1)}-${pad(d.getDate())}T${pad(d.getHours())}:${pad(d.getMinutes())}`;
	}

	function openAchievementEditor(ach: Achievement) {
		if (editingAchKey === ach.key) {
			editingAchKey = null;
			return;
		}
		editingAchKey = ach.key;
		editAchUnlocked = ach.unlocked || Boolean(ach.unlocked_time);
		editAchDateTime = unixToLocalDatetimeInput(ach.unlocked_time);
	}

	function setEditDateTimeNow() {
		editAchDateTime = unixToLocalDatetimeInput(Math.floor(Date.now() / 1000));
	}

	async function saveAchievementEdit(achKey: string) {
		if (!game || savingAchEdit) return;
		savingAchEdit = true;
		try {
			let unlockedTime: number | null = null;
			if (editAchUnlocked) {
				const parsedMs = editAchDateTime ? new Date(editAchDateTime).getTime() : Date.now();
				unlockedTime = Number.isFinite(parsedMs)
					? Math.floor(parsedMs / 1000)
					: Math.floor(Date.now() / 1000);
			}

			const updated = await invoke<Achievement[]>('set_achievement_state', {
				gameId: game.id,
				achKey,
				unlocked: editAchUnlocked,
				unlockedTime
			});

			if (updated && updated.length > 0) {
				achievements = updated;
				games.update((list) =>
					list.map((g) => {
						if (g.id !== game.id) return g;
						const prevByKey = new SvelteMap<string, Achievement>();
						for (const prev of g.achievements ?? []) {
							prevByKey.set(prev.key.trim().toLowerCase(), prev);
						}
						const preserved = updated.map((u) => {
							const prev = prevByKey.get(u.key.trim().toLowerCase());
							if (!prev) return u;
							return {
								...u,
								groupid: u.groupid ?? prev.groupid,
								archived: u.archived ?? prev.archived
							};
						});
						return { ...g, achievements: preserved };
					})
				);
			} else {
				// Update locally in case game wasn't found in state
				const norm = achKey.trim().toLowerCase();
				achievements = achievements.map((a) =>
					a.key.trim().toLowerCase() === norm
						? { ...a, unlocked: editAchUnlocked, unlocked_time: unlockedTime }
						: a
				);
			}
			editingAchKey = null;
		} catch (e) {
			console.error('Failed to update achievement state:', e);
		} finally {
			savingAchEdit = false;
		}
	}

	function goToGame(id: string) {
		void goto(resolve('/(app)/games/[id]', { id }));
	}

	let copiedKey = $state<string | null>(null);
	let pinnedKeys = $state<string[]>([]);

	$effect(() => {
		const gid = game?.id;
		if (!gid) {
			pinnedKeys = [];
			return;
		}
		void invoke<string[]>('get_pinned_achievements', { gameId: gid })
			.then((pins) => {
				pinnedKeys = pins ?? [];
			})
			.catch(() => {});
	});

	let pinnedSet = $derived(new Set(pinnedKeys.map((k) => k.trim().toLowerCase())));

	async function togglePin(key: string): Promise<void> {
		if (!game) return;
		try {
			pinnedKeys = await invoke<string[]>('toggle_pinned_achievement', {
				gameId: game.id,
				achKey: key
			});
		} catch (e) {
			console.error('Failed to toggle pin:', e);
		}
	}

	function copyToClipboard(text: string, key: string) {
		navigator.clipboard.writeText(text);
		copiedKey = key;
		setTimeout(() => {
			if (copiedKey === key) copiedKey = null;
		}, 2000);
	}

	function searchGuide(gameName: string, achName: string) {
		const query = `${gameName} ${achName} achievement guide`;
		void open(`https://www.google.com/search?q=${encodeURIComponent(query)}`);
	}

	type ExportNotice = {
		message: string;
		type: 'success' | 'error';
	};

	let exportNotice = $state<ExportNotice | null>(null);
	let exportNoticeTimer: ReturnType<typeof setTimeout> | null = null;

	function showExportNotice(message: string, type: 'success' | 'error' = 'success') {
		exportNotice = { message, type };
		if (exportNoticeTimer) clearTimeout(exportNoticeTimer);
		exportNoticeTimer = setTimeout(() => {
			exportNotice = null;
			exportNoticeTimer = null;
		}, 4000);
	}

	async function handleExportGame() {
		if (game) {
			try {
				const path = await exportGame(game);
				showExportNotice(path, 'success');
			} catch {
				showExportNotice('Export failed', 'error');
			}
		}
	}

	async function handleExportGamePdf() {
		if (game) {
			try {
				const title = game.name;
				const unlocked = progress.unlockedAll;
				const total = progress.totalAll;
				let content = `Jeu: ${game.name}\n`;
				content += `Identifiant: ${game.id}\n`;
				content += `Source: ${typeof game.source === 'object' ? game.source.type : game.source}\n`;
				content += `Progression: ${unlocked}/${total} (${total > 0 ? globalPctDisplay : 0}%)\n\n`;
				content += 'Unlocked achievements:\n';
				content += `--------------------------\n`;

				publicAchievements
					.filter((a) => a.unlocked)
					.forEach((a) => {
						content += `[X] ${a.name}\n`;
					});

				const path = await exportSummaryPdf(title, content);
				showExportNotice(path, 'success');
			} catch (err) {
				console.error('PDF Export Error:', err);
				showExportNotice(String(err), 'error');
			}
		}
	}

	type AchievementsUpdatedPayload = {
		game_id: string;
		achievements: Achievement[];
	};

	let achievements = $state<Achievement[]>([]);
	let activeView = $state<'achievements' | 'map'>('achievements');
	let activeSetId = $state<string>('all');
	let filter = $state<'all' | 'unlocked' | 'locked'>('all');
	let search = $state('');
	let sort = $state<'date' | 'rarity' | 'name'>('date');
	let sortOrder = $state<'asc' | 'desc'>('desc');
	let revealed = $state(true);
	let loading = $state(true);
	let loadedGameId = $state<string | null>(null);
	let loadRequestToken = 0;

	let jumpedToken = $state<number | null>(null);
	let flashAchievementKey = $state('');

	const rowRefs = new SvelteMap<string, HTMLDivElement>();
	let flashTimer: ReturnType<typeof setTimeout> | null = null;

	$effect(() => {
		if (!game) {
			loadRequestToken += 1;
			loadedGameId = null;
			achievements = [];
			loading = false;
			return;
		}

		if (loadedGameId === game.id) return;

		loadedGameId = game.id;
		loading = true;
		const requestToken = ++loadRequestToken;

		void loadAchievements(game).then((result) => {
			if (requestToken !== loadRequestToken) return;
			achievements = result;
			loading = false;
		});
	});

	onMount(() => {
		let unlisten: (() => void) | undefined;

		const handleKeydown = (e: KeyboardEvent) => {
			if (get(settingsOpen)) return;
			if (e.target instanceof HTMLInputElement || e.target instanceof HTMLTextAreaElement) return;

			if (e.key === 'ArrowLeft' && prevGame) {
				goToGame(prevGame.id);
			} else if (e.key === 'ArrowRight' && nextGame) {
				goToGame(nextGame.id);
			}
		};
		window.addEventListener('keydown', handleKeydown);

		listen<AchievementsUpdatedPayload>('achievements-updated', (event) => {
			const payload = event.payload;
			if (game && payload?.game_id === game.id && Array.isArray(payload.achievements)) {
				achievements = payload.achievements;
			}
		}).then((fn) => {
			unlisten = fn;
		});

		return () => {
			unlisten?.();
			window.removeEventListener('keydown', handleKeydown);
			if (flashTimer) clearTimeout(flashTimer);
			if (exportNoticeTimer) clearTimeout(exportNoticeTimer);
		};
	});

	let merged = $derived.by((): Achievement[] => {
		if (!game) return [];
		const schema = game.achievements ?? [];
		const stateMap = new Map<string, Achievement>(
			achievements.map((a) => [a.key.trim().toLowerCase(), a])
		);
		const seenKeys = new SvelteSet<string>();

		const fromSchema: Achievement[] = schema.map((s: Achievement) => {
			const normKey = s.key.trim().toLowerCase();
			seenKeys.add(normKey);
			const live = stateMap.get(normKey);
			return {
				...s,
				name: live?.name || s.name || s.key,
				unlocked: live?.unlocked ?? s.unlocked ?? false,
				unlocked_time:
					live?.unlocked_time !== undefined ? live.unlocked_time : (s.unlocked_time ?? null),
				rarity: live?.rarity || s.rarity || '',
				completionpercentage: live?.completionpercentage || s.completionpercentage || '',
				desc: live?.desc || s.desc || '',
				icon: live?.icon || s.icon || '',
				icon_gray: live?.icon_gray || s.icon_gray || '',
				progress: live?.progress ?? s.progress ?? null,
				max_progress: live?.max_progress ?? s.max_progress ?? null,
				groupid: s.groupid || live?.groupid || 0,
				archived: Boolean(s.archived || live?.archived)
			};
		});

		for (const live of achievements) {
			const normKey = live.key.trim().toLowerCase();
			if (!seenKeys.has(normKey)) {
				seenKeys.add(normKey);
				fromSchema.push({
					...live,
					groupid: live.groupid ?? 0,
					archived: live.archived ?? false
				});
			}
		}

		return fromSchema;
	});

	let progress = $derived(
		computeGameProgress(merged, game?.achievement_groups, $i18n.t('game.baseGame'))
	);
	let isPlatinum = $derived(progress.isPlatinum);
	let hasSets = $derived(progress.sections.length > 1 || progress.hasDlc);

	let prevSetGameId: string | null = null;
	$effect(() => {
		const gid = game?.id ?? null;
		if (gid !== prevSetGameId) {
			prevSetGameId = gid;
			activeSetId = 'all';
			return;
		}
		if (activeSetId !== 'all' && !progress.sections.some((s) => s.id === activeSetId)) {
			activeSetId = 'all';
		}
	});

	function formatCompletionPct(unlocked: number, total: number, rawPct: number): number {
		if (total <= 0) return 0;
		if (unlocked === total) return 100;
		return Math.min(99, Math.max(0, Math.round(rawPct)));
	}

	let platinumPctDisplay = $derived(
		formatCompletionPct(progress.unlockedBase, progress.totalBase, progress.platinumPct)
	);
	let globalPctDisplay = $derived(
		formatCompletionPct(progress.unlockedAll, progress.totalAll, progress.globalPct)
	);

	let globalAchievementHighlight = $derived($searchQuery.trim().toLowerCase());

	function filterAndSortAchievements(
		list: Achievement[],
		filterMode: 'all' | 'unlocked' | 'locked',
		query: string,
		sortBy: 'date' | 'rarity' | 'name',
		order: 'asc' | 'desc'
	): Achievement[] {
		let result = list;

		if (filterMode === 'unlocked') result = result.filter((a) => a.unlocked);
		else if (filterMode === 'locked') result = result.filter((a) => !a.unlocked);

		const localQuery = query.trim().toLowerCase();
		if (localQuery) {
			result = result.filter(
				(a) =>
					a.name.toLowerCase().includes(localQuery) ||
					(a.desc ?? '').toLowerCase().includes(localQuery)
			);
		}

		const sorted = [...result];
		const orderMultiplier = order === 'asc' ? 1 : -1;

		if (sortBy === 'date') {
			sorted.sort((a: Achievement, b: Achievement) => {
				if (a.unlocked && b.unlocked) {
					return ((a.unlocked_time ?? 0) - (b.unlocked_time ?? 0)) * orderMultiplier;
				}
				if (a.unlocked) return -1;
				if (b.unlocked) return 1;
				return 0;
			});
		} else if (sortBy === 'rarity') {
			sorted.sort(
				(a: Achievement, b: Achievement) =>
					((parseFloat(a.completionpercentage) || 100) -
						(parseFloat(b.completionpercentage) || 100)) *
					orderMultiplier
			);
		} else if (sortBy === 'name') {
			sorted.sort(
				(a: Achievement, b: Achievement) => a.name.localeCompare(b.name) * orderMultiplier
			);
		}

		return sorted;
	}

	let activeSections = $derived(
		activeSetId === 'all'
			? progress.sections
			: progress.sections.filter((s) => s.id === activeSetId)
	);

	let visibleSections = $derived(
		activeSections.map((section) => ({
			...section,
			displayPct: formatCompletionPct(section.unlocked, section.total, section.pct),
			filteredAchievements: filterAndSortAchievements(
				section.achievements,
				filter,
				search,
				sort,
				sortOrder
			)
		}))
	);

	let flatFiltered = $derived(
		visibleSections.flatMap((section) =>
			section.filteredAchievements.map((ach) => ({
				ach,
				isArchived: Boolean(ach.archived || section.archived)
			}))
		)
	);

	let filteredCount = $derived(flatFiltered.length);
	let visibleTotalCount = $derived(
		activeSections.reduce((acc, s) => acc + s.achievements.length, 0)
	);
	let publicAchievements = $derived(progress.sections.flatMap((s) => s.achievements));
	let unlockedCount = $derived(progress.hasDlc ? progress.unlockedAll : progress.unlockedBase);
	let totalCount = $derived(progress.hasDlc ? progress.totalAll : progress.totalBase);

	let mapAchievements = $derived.by(() => {
		const unlocked = publicAchievements
			.filter((achievement) => achievement.unlocked || Boolean(achievement.unlocked_time))
			.sort((a, b) => (a.unlocked_time ?? 0) - (b.unlocked_time ?? 0));
		const locked = publicAchievements
			.filter((achievement) => !achievement.unlocked && !achievement.unlocked_time)
			.sort((a, b) => {
				const rarityA = parseFloat(a.completionpercentage);
				const rarityB = parseFloat(b.completionpercentage);
				const normalizedA = Number.isFinite(rarityA) ? rarityA : Number.POSITIVE_INFINITY;
				const normalizedB = Number.isFinite(rarityB) ? rarityB : Number.POSITIVE_INFINITY;

				return normalizedB - normalizedA || a.name.localeCompare(b.name);
			});
		return [...unlocked, ...locked];
	});

	function normalizeAchievementKey(key: string): string {
		return (key || '').trim().toLowerCase();
	}

	function trackAchievementRow(node: HTMLDivElement, key: string) {
		const normalized = normalizeAchievementKey(key);
		rowRefs.set(normalized, node);

		return {
			destroy() {
				rowRefs.delete(normalized);
			}
		};
	}

	function triggerAchievementFlash(key: string) {
		flashAchievementKey = key;
		if (flashTimer) clearTimeout(flashTimer);
		flashTimer = setTimeout(() => {
			flashAchievementKey = '';
			flashTimer = null;
		}, 1800);
	}

	async function openAchievementFromMap(key: string) {
		activeView = 'achievements';
		activeSetId = 'all';
		filter = 'all';
		search = '';

		const normalizedKey = normalizeAchievementKey(key);
		let target: HTMLDivElement | undefined;

		for (let i = 0; i < 20; i += 1) {
			await tick();
			target = rowRefs.get(normalizedKey);
			if (target) break;
			await delay(50);
		}

		if (target) {
			target.scrollIntoView({ behavior: 'smooth', block: 'center' });
			triggerAchievementFlash(normalizedKey);
		}
	}

	function delay(ms: number): Promise<void> {
		return new Promise((resolve) => setTimeout(resolve, ms));
	}

	$effect(() => {
		const intent = $achievementJumpIntent;
		if (!intent || !game) return;
		if (intent.gameId !== game.id) return;
		if (jumpedToken === intent.token) return;

		jumpedToken = intent.token;
		const normalizedKey = normalizeAchievementKey(intent.achievementKey);

		activeView = 'achievements';
		activeSetId = 'all';
		filter = 'all';
		search = '';

		void (async () => {
			let target: HTMLDivElement | undefined;

			for (let i = 0; i < 20; i += 1) {
				await tick();
				target = rowRefs.get(normalizedKey);
				if (target) break;
				await delay(50);
			}

			if (target) {
				target.scrollIntoView({ behavior: 'smooth', block: 'center' });
				triggerAchievementFlash(normalizedKey);
			}

			achievementJumpIntent.set(null);
		})();
	});

	function formatProgressVal(n: number): string {
		return Number.isInteger(n) ? String(n) : n.toFixed(1);
	}

	function formatDate(ts: number | null): string {
		if (!ts) return '';
		return new Date(ts * 1000).toLocaleDateString($i18n.locale, {
			day: '2-digit',
			month: '2-digit',
			year: 'numeric',
			hour: '2-digit',
			minute: '2-digit'
		});
	}

	function rarityLabel(pct: string): string {
		const n = parseFloat(pct);
		if (isNaN(n)) return '';
		if (n <= 0.1) return rarityLabelByIndex(get(i18n).language, 0);
		if (n <= 1) return rarityLabelByIndex(get(i18n).language, 1);
		if (n <= 3) return rarityLabelByIndex(get(i18n).language, 2);
		if (n <= 7) return rarityLabelByIndex(get(i18n).language, 3);
		if (n <= 15) return rarityLabelByIndex(get(i18n).language, 4);
		if (n <= 35) return rarityLabelByIndex(get(i18n).language, 5);
		return rarityLabelByIndex(get(i18n).language, 6);
	}

	type RarityPalette = {
		border: string;
		badgeBg: string;
		badgeColor: string;
		fill: string;
	};

	function rarityPalette(pct: string): RarityPalette {
		const n = parseFloat(pct);
		if (isNaN(n)) {
			return {
				border: 'var(--border-soft)',
				badgeBg: 'var(--surface-2)',
				badgeColor: 'var(--text-muted)',
				fill: 'var(--text-muted)'
			};
		}

		if (n <= 0.1) {
			return {
				border: '#ff3b5c',
				badgeBg: 'rgba(255,59,92,0.14)',
				badgeColor: '#ff3b5c',
				fill: '#ff3b5c'
			};
		}
		if (n <= 1) {
			return {
				border: '#ffd85a',
				badgeBg: 'rgba(255,216,90,0.14)',
				badgeColor: '#ffd85a',
				fill: '#ffd85a'
			};
		}
		if (n <= 3) {
			return {
				border: '#a855f7',
				badgeBg: 'rgba(168,85,247,0.14)',
				badgeColor: '#a855f7',
				fill: '#a855f7'
			};
		}
		if (n <= 7) {
			return {
				border: '#f4b860',
				badgeBg: 'rgba(244,184,96,0.14)',
				badgeColor: '#f4b860',
				fill: '#f4b860'
			};
		}
		if (n <= 15) {
			return {
				border: '#4ac8ff',
				badgeBg: 'rgba(74,200,255,0.1)',
				badgeColor: '#4ac8ff',
				fill: '#4ac8ff'
			};
		}
		if (n <= 35) {
			return {
				border: '#3ddc84',
				badgeBg: 'rgba(61,220,132,0.12)',
				badgeColor: '#3ddc84',
				fill: '#3ddc84'
			};
		}

		return {
			border: 'var(--border-soft)',
			badgeBg: 'var(--surface-2)',
			badgeColor: 'var(--text-muted)',
			fill: 'var(--text-muted)'
		};
	}

	function gameTitle(current: Game): string {
		return current.name?.trim() || `ID ${current.id}`;
	}

	function heroBackground(current: Game): string {
		if (current.steam_id) {
			return `https://cdn.cloudflare.steamstatic.com/steam/apps/${current.steam_id}/library_hero.jpg`;
		}
		return current.header_image_url || '';
	}
</script>

{#if !game}
	<div class="not-found">{$i18n.t('game.notFound')}</div>
{:else}
	<div class="game-page" style:--game-bg={`url('${heroBackground(game)}')`}>
		<div class="bg-overlay"></div>

		<div class="game-nav">
			<button
				class="nav-btn"
				disabled={!prevGame}
				onclick={() => prevGame && goToGame(prevGame.id)}
			>
				<svg
					width="12"
					height="12"
					viewBox="0 0 24 24"
					fill="none"
					stroke="currentColor"
					stroke-width="3"
				>
					<path d="M15 18l-6-6 6-6" />
				</svg>
				<span class="nav-label">{prevGame ? prevGame.name || prevGame.id : ''}</span>
			</button>

			<span class="nav-index tabular-nums">{currentIndex + 1} / {$games.length}</span>

			<button
				class="nav-btn"
				disabled={!nextGame}
				onclick={() => nextGame && goToGame(nextGame.id)}
			>
				<span class="nav-label">{nextGame ? nextGame.name || nextGame.id : ''}</span>
				<svg
					width="12"
					height="12"
					viewBox="0 0 24 24"
					fill="none"
					stroke="currentColor"
					stroke-width="3"
				>
					<path d="M9 18l6-6-6-6" />
				</svg>
			</button>
		</div>

		<div class="game-scroll-area">
			<div class="content-wrapper">
				<div class="game-header">
					<div class="game-title-row">
						<div class="title-with-fav">
							<h1 class="game-title">{gameTitle(game)}</h1>
							<button
								class="big-fav-btn"
								class:favorited={game.is_favorite}
								onclick={() => toggleGameFavorite(game.id)}
								title={$i18n.t(
									game.is_favorite ? 'games.action.removeFavorite' : 'games.action.addFavorite'
								)}
								aria-label={$i18n.t(
									game.is_favorite ? 'games.action.removeFavorite' : 'games.action.addFavorite'
								)}
							>
								{#if game.is_favorite}
									<svg
										width="28"
										height="28"
										viewBox="0 0 24 24"
										fill="currentColor"
										stroke="currentColor"
										stroke-width="1.8"
										stroke-linecap="round"
										stroke-linejoin="round"
										aria-hidden="true"
									>
										<path
											d="M20.84 4.61a5.5 5.5 0 0 0-7.78 0L12 5.67l-1.06-1.06a5.5 5.5 0 0 0-7.78 7.78l1.06 1.06L12 21.23l7.78-7.78 1.06-1.06a5.5 5.5 0 0 0 0-7.78z"
										/>
									</svg>
								{:else}
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
										<path
											d="M20.84 4.61a5.5 5.5 0 0 0-7.78 0L12 5.67l-1.06-1.06a5.5 5.5 0 0 0-7.78 7.78l1.06 1.06L12 21.23l7.78-7.78 1.06-1.06a5.5 5.5 0 0 0 0-7.78z"
										/>
									</svg>
								{/if}
							</button>
						</div>
						<span class="emulator-badge"
							>{typeof game.source === 'object' && game.source.type === 'Emulator'
								? game.source.value
								: game.source.type}</span
						>
					</div>

					<div class="progress-card-container">
						<div class="progress-card" class:has-dual-gauges={progress.hasDlc}>
							<div class="progress-left" class:dual-gauges={progress.hasDlc}>
								{#if progress.hasDlc}
									<div class="gauge-block" class:is-platinum={isPlatinum}>
										<div class="gauge-header">
											<span class="gauge-title">
												{$i18n.t('game.platinumProgress')}
												{#if isPlatinum}
													<span class="platinum-pill">100%</span>
												{/if}
											</span>
											<div class="gauge-numbers tabular-nums">
												<span class="gauge-count">
													<strong>{progress.unlockedBase}</strong>
													<span class="gauge-slash">/ {progress.totalBase}</span>
												</span>
												<span
													class="gauge-pct"
													style:color={isPlatinum ? 'var(--completed)' : '#3ddc84'}
													>{platinumPctDisplay}%</span
												>
											</div>
										</div>
										<div class="progress-bar-track">
											<div
												class="progress-bar-fill"
												style="width: {progress.platinumPct}%; background: {isPlatinum
													? 'var(--completed)'
													: '#3ddc84'}"
											></div>
										</div>
									</div>

									<div class="gauge-block">
										<div class="gauge-header">
											<span class="gauge-title">{$i18n.t('game.totalProgressWithDlc')}</span>
											<div class="gauge-numbers tabular-nums">
												<span class="gauge-count">
													<strong>{progress.unlockedAll}</strong>
													<span class="gauge-slash">/ {progress.totalAll}</span>
												</span>
												<span
													class="gauge-pct"
													style:color={globalPctDisplay === 100
														? 'var(--completed)'
														: 'var(--accent)'}>{globalPctDisplay}%</span
												>
											</div>
										</div>
										<div class="progress-bar-track secondary">
											<div
												class="progress-bar-fill"
												style="width: {progress.globalPct}%; background: {globalPctDisplay === 100
													? 'var(--completed)'
													: 'var(--accent, #c8a96e)'}"
											></div>
										</div>
									</div>
								{:else}
									<div class="progress-stats">
										<span class="main-count tabular-nums"
											><strong>{progress.unlockedBase}</strong>
											<span class="slash">/ {progress.totalBase}</span></span
										>
										<span
											class="main-pct tabular-nums"
											style:color={isPlatinum ? 'var(--completed)' : 'var(--accent)'}
											>{platinumPctDisplay}%</span
										>
									</div>
									<div class="progress-bar-track">
										<div
											class="progress-bar-fill"
											style="width: {progress.platinumPct}%; background: {isPlatinum
												? 'var(--completed)'
												: '#3ddc84'}"
										></div>
									</div>
								{/if}
							</div>
							<div class="progress-right">
								<div class="stat-item">
									<span class="stat-label">{$i18n.t('game.total')}</span>
									<span class="stat-value tabular-nums">{totalCount}</span>
								</div>
								<div class="stat-item">
									<span class="stat-label">{$i18n.t('game.unlocked')}</span>
									<span class="stat-value highlight tabular-nums">{unlockedCount}</span>
								</div>
								<div class="stat-item playtime-stat">
									<div class="stat-label-row">
										<span class="stat-label">{$i18n.t('game.playtime')}</span>
										<button
											class="mini-edit-icon"
											onclick={() =>
												editingPlaytime ? (editingPlaytime = false) : openPlaytimeEditor()}
											title={$i18n.t('game.editPlaytime')}
											aria-label={$i18n.t('game.editPlaytime')}
										>
											<svg
												width="11"
												height="11"
												viewBox="0 0 24 24"
												fill="none"
												stroke="currentColor"
												stroke-width="2.2"
												stroke-linecap="round"
												stroke-linejoin="round"
												aria-hidden="true"
											>
												<path d="M12 20h9" />
												<path d="M16.5 3.5a2.121 2.121 0 0 1 3 3L7 19l-4 1 1-4L16.5 3.5z" />
											</svg>
										</button>
									</div>
									{#if editingPlaytime}
										<div class="playtime-inline-edit">
											<input
												type="number"
												min="0"
												bind:value={editPlaytimeHours}
												class="pt-input tabular-nums"
												title={$i18n.t('game.hours')}
											/>
											<span>h</span>
											<input
												type="number"
												min="0"
												max="59"
												bind:value={editPlaytimeMins}
												class="pt-input tabular-nums"
												title={$i18n.t('game.minutes')}
											/>
											<span>m</span>
											<button
												class="pt-save-btn"
												onclick={saveManualPlaytime}
												aria-label={$i18n.t('game.save')}
											>
												<svg
													width="12"
													height="12"
													viewBox="0 0 24 24"
													fill="none"
													stroke="currentColor"
													stroke-width="3"
													stroke-linecap="round"
													stroke-linejoin="round"
													aria-hidden="true"
												>
													<polyline points="20 6 9 17 4 12" />
												</svg>
											</button>
										</div>
									{:else}
										<span class="stat-value playtime-val tabular-nums"
											>{formatPlaytime(gamePlaytimeSeconds)}</span
										>
									{/if}
								</div>
							</div>
						</div>

						<!-- HowLongToBeat Bar -->
						<div class="hltb-bar">
							<div class="hltb-brand">
								<span class="hltb-badge">HowLongToBeat</span>
							</div>

							{#if editingHltb}
								<div class="hltb-edit-row">
									<label>
										<span>{$i18n.t('game.hltb.mainShort')}</span>
										<input
											type="number"
											step="0.5"
											min="0"
											bind:value={customHltbMain}
											placeholder="ex: 15"
										/>
									</label>
									<label>
										<span>{$i18n.t('game.hltb.extraShort')}</span>
										<input
											type="number"
											step="0.5"
											min="0"
											bind:value={customHltbExtra}
											placeholder="ex: 28"
										/>
									</label>
									<label>
										<span>{$i18n.t('game.hltb.compShort')}</span>
										<input
											type="number"
											step="0.5"
											min="0"
											bind:value={customHltb100}
											placeholder="ex: 45"
										/>
									</label>
									<button class="hltb-btn primary" onclick={saveCustomHltb}
										>{$i18n.t('game.save')}</button
									>
									<button class="hltb-btn" onclick={() => (editingHltb = false)}
										>{$i18n.t('game.cancel')}</button
									>
								</div>
							{:else}
								<div class="hltb-metrics">
									<div class="hltb-metric">
										<div class="hltb-metric-top">
											<span class="hltb-label">{$i18n.t('game.hltb.main')}</span>
											<strong class="hltb-hours tabular-nums">
												{hltbLoading
													? '...'
													: hltbData?.main_hours
														? `${hltbData.main_hours}h`
														: '—'}
											</strong>
										</div>
										<div class="hltb-track">
											<div
												class="hltb-fill"
												style:width="{hltbProgressPct(hltbData?.main_hours)}%"
											></div>
										</div>
									</div>

									<div class="hltb-metric">
										<div class="hltb-metric-top">
											<span class="hltb-label">{$i18n.t('game.hltb.extra')}</span>
											<strong class="hltb-hours tabular-nums">
												{hltbLoading
													? '...'
													: hltbData?.main_extra_hours
														? `${hltbData.main_extra_hours}h`
														: '—'}
											</strong>
										</div>
										<div class="hltb-track">
											<div
												class="hltb-fill extra"
												style:width="{hltbProgressPct(hltbData?.main_extra_hours)}%"
											></div>
										</div>
									</div>

									<div class="hltb-metric">
										<div class="hltb-metric-top">
											<span class="hltb-label">{$i18n.t('game.hltb.completionist')}</span>
											<strong class="hltb-hours tabular-nums">
												{hltbLoading
													? '...'
													: hltbData?.completionist_hours
														? `${hltbData.completionist_hours}h`
														: '—'}
											</strong>
										</div>
										<div class="hltb-track">
											<div
												class="hltb-fill comp"
												style:width="{hltbProgressPct(hltbData?.completionist_hours)}%"
											></div>
										</div>
									</div>
								</div>

								<div class="hltb-actions">
									<button
										class="hltb-icon-btn"
										onclick={openHltbEditor}
										title={$i18n.t('game.hltb.editManual')}
										aria-label={$i18n.t('game.hltb.editManual')}
									>
										<svg
											width="12"
											height="12"
											viewBox="0 0 24 24"
											fill="none"
											stroke="currentColor"
											stroke-width="2.2"
											stroke-linecap="round"
											stroke-linejoin="round"
											aria-hidden="true"
										>
											<path d="M12 20h9" />
											<path d="M16.5 3.5a2.121 2.121 0 0 1 3 3L7 19l-4 1 1-4L16.5 3.5z" />
										</svg>
									</button>
									<button
										class="hltb-icon-btn"
										disabled={hltbLoading}
										onclick={refreshHltb}
										title={$i18n.t('game.hltb.refresh')}
										aria-label={$i18n.t('game.hltb.refresh')}
									>
										<svg
											width="12"
											height="12"
											viewBox="0 0 24 24"
											fill="none"
											stroke="currentColor"
											stroke-width="2.2"
											stroke-linecap="round"
											stroke-linejoin="round"
											aria-hidden="true"
										>
											<polyline points="23 4 23 10 17 10" />
											<path d="M20.49 15a9 9 0 1 1-2.12-9.36L23 10" />
										</svg>
									</button>
								</div>
							{/if}
						</div>

						<div class="tags-section">
							<div class="tags-list">
								{#each game.tags || [] as tag (tag)}
									<span class="tag">
										{tag}
										<button
											class="remove-tag"
											onclick={() => removeGameTag(game.id, tag)}
											title={$i18n.t('game.removeTag')}
											aria-label={$i18n.t('game.removeTag')}>×</button
										>
									</span>
								{/each}
								<div class="add-tag-form">
									<input
										type="text"
										placeholder={$i18n.t('game.addTag')}
										bind:value={newTag}
										list="existing-tags"
										onkeydown={(e) => e.key === 'Enter' && handleAddTag()}
									/>
									<datalist id="existing-tags">
										{#each allTags as tag (tag)}
											<option value={tag}></option>
										{/each}
									</datalist>
									<button onclick={handleAddTag}>+</button>
								</div>
							</div>
						</div>
					</div>
				</div>

				<div class="main-section">
					<div class="controls-bar">
						<div class="view-tabs">
							<button
								class="tab"
								class:active={activeView === 'achievements'}
								onclick={() => (activeView = 'achievements')}
								>{$i18n.t('game.achievementsTitle')}</button
							>
							<button
								class="tab"
								class:active={activeView === 'map'}
								onclick={() => (activeView = 'map')}>{$i18n.t('game.shareCard')}</button
							>
						</div>

						{#if activeView === 'achievements' && hasSets}
							<div class="set-tabs" role="tablist" aria-label={$i18n.t('game.allSets')}>
								<button
									type="button"
									role="tab"
									aria-selected={activeSetId === 'all'}
									class="set-tab"
									class:active={activeSetId === 'all'}
									onclick={() => (activeSetId = 'all')}
								>
									<span class="set-tab-name">{$i18n.t('game.allSets')}</span>
									<span class="set-tab-count tabular-nums"
										>{progress.unlockedAll}/{progress.totalAll}</span
									>
								</button>
								{#each progress.sections as section (section.id)}
									<button
										type="button"
										role="tab"
										aria-selected={activeSetId === section.id}
										class="set-tab"
										class:active={activeSetId === section.id}
										onclick={() => (activeSetId = section.id)}
									>
										{#if section.isDlc}
											<span class="set-tab-dlc">{$i18n.t('game.dlcBadge')}</span>
										{/if}
										<span class="set-tab-name">{section.title}</span>
										<span class="set-tab-count tabular-nums"
											>{section.unlocked}/{section.total}</span
										>
									</button>
								{/each}
							</div>
						{/if}

						<div class="controls-bottom">
							{#if activeView === 'achievements'}
								<div class="filter-tabs">
									<button
										class="tab"
										class:active={filter === 'all'}
										onclick={() => (filter = 'all')}>{$i18n.t('game.filterAll')}</button
									>
									<button
										class="tab"
										class:active={filter === 'unlocked'}
										onclick={() => (filter = 'unlocked')}>{$i18n.t('game.unlocked')}</button
									>
									<button
										class="tab"
										class:active={filter === 'locked'}
										onclick={() => (filter = 'locked')}>{$i18n.t('game.locked')}</button
									>
								</div>

								<div class="search-wrap">
									<svg
										width="14"
										height="14"
										viewBox="0 0 24 24"
										fill="none"
										stroke="currentColor"
										stroke-width="2.5"
									>
										<circle cx="11" cy="11" r="8" /><line x1="21" y1="21" x2="16.65" y2="16.65" />
									</svg>
									<input
										class="search-input"
										type="text"
										placeholder={$i18n.t('game.searchAchievement')}
										bind:value={search}
									/>
									<span class="search-count tabular-nums"
										>{filteredCount} / {visibleTotalCount}</span
									>
								</div>

								<select class="sort-select" bind:value={sort}>
									<option value="date">{$i18n.t('game.sort.date')}</option>
									<option value="rarity">{$i18n.t('game.sort.rarity')}</option>
									<option value="name">{$i18n.t('game.sort.name')}</option>
								</select>

								<button
									class="order-btn"
									onclick={() => (sortOrder = sortOrder === 'asc' ? 'desc' : 'asc')}
									title={sortOrder === 'asc' ? $i18n.t('game.sort.asc') : $i18n.t('game.sort.desc')}
								>
									{#if sortOrder === 'asc'}
										<svg
											width="14"
											height="14"
											viewBox="0 0 24 24"
											fill="none"
											stroke="currentColor"
											stroke-width="2.5"><path d="M12 19V5M5 12l7-7 7 7" /></svg
										>
									{:else}
										<svg
											width="14"
											height="14"
											viewBox="0 0 24 24"
											fill="none"
											stroke="currentColor"
											stroke-width="2.5"><path d="M12 5v14M5 12l7 7 7-7" /></svg
										>
									{/if}
								</button>

								<button
									class="reveal-btn"
									class:active={revealed}
									onclick={() => (revealed = !revealed)}
								>
									{#if revealed}
										<svg
											width="14"
											height="14"
											viewBox="0 0 24 24"
											fill="none"
											stroke="currentColor"
											stroke-width="2.5"
										>
											<path d="M1 12s4-8 11-8 11 8 11 8-4 8-11 8-11-8-11-8z" /><circle
												cx="12"
												cy="12"
												r="3"
											/>
										</svg>
										<span>{$i18n.t('game.revealed')}</span>
									{:else}
										<svg
											width="14"
											height="14"
											viewBox="0 0 24 24"
											fill="none"
											stroke="currentColor"
											stroke-width="2.5"
										>
											<path
												d="M17.94 17.94A10.07 10.07 0 0 1 12 20c-7 0-11-8-11-8a18.45 18.45 0 0 1 5.06-5.94M9.9 4.24A9.12 9.12 0 0 1 12 4c7 0 11 8 11 8a18.5 18.5 0 0 1-2.16 3.19m-6.72-1.07a3 3 0 1 1-4.24-4.24"
											></path>
											<line x1="1" y1="1" x2="23" y2="23"></line>
										</svg>
										<span>{$i18n.t('game.hidden')}</span>
									{/if}
								</button>
							{:else}
								<div class="map-summary">
									<span class="map-summary-title">{$i18n.t('game.progressLabel')}</span>
									<span class="map-summary-value tabular-nums"
										>{$i18n.t('game.stepsCount', {
											unlocked: unlockedCount,
											total: totalCount
										})}</span
									>
								</div>
							{/if}

							<div class="export-actions">
								<button
									type="button"
									class="profile-btn-mini"
									onclick={handleExportGame}
									title={$i18n.t('game.exportJson')}
									aria-label={$i18n.t('game.exportJson')}
								>
									<svg
										width="14"
										height="14"
										viewBox="0 0 24 24"
										fill="none"
										stroke="currentColor"
										stroke-width="2.5"
										><path d="M21 15v4a2 2 0 0 1-2 2H5a2 2 0 0 1-2-2v-4" /><polyline
											points="7 10 12 15 17 10"
										/><line x1="12" y1="15" x2="12" y2="3" /></svg
									>
								</button>

								<button
									type="button"
									class="profile-btn-mini"
									onclick={handleExportGamePdf}
									title={$i18n.t('game.exportPdf')}
									aria-label={$i18n.t('game.exportPdf')}
								>
									<svg
										width="14"
										height="14"
										viewBox="0 0 24 24"
										fill="none"
										stroke="currentColor"
										stroke-width="2.5"
										><path
											d="M14 2H6a2 2 0 0 0-2 2v16a2 2 0 0 0 2 2h12a2 2 0 0 0 2-2V8z"
										/><polyline points="14 2 14 8 20 8" /><line
											x1="16"
											y1="13"
											x2="8"
											y2="13"
										/><line x1="16" y1="17" x2="8" y2="17" /><polyline points="10 9 9 9 8 9" /></svg
									>
								</button>

								<button
									type="button"
									class="profile-btn-mini"
									onclick={openProfilesFolder}
									title={$i18n.t('game.exportFolder')}
									aria-label={$i18n.t('game.exportFolder')}
								>
									<svg
										width="14"
										height="14"
										viewBox="0 0 24 24"
										fill="none"
										stroke="currentColor"
										stroke-width="2.5"
										><path
											d="M22 19a2 2 0 0 1-2 2H4a2 2 0 0 1-2-2V5a2 2 0 0 1 2-2h5l2 3h9a2 2 0 0 1 2 2z"
										/></svg
									>
								</button>
							</div>
						</div>
					</div>

					{#if activeView === 'achievements'}
						<!-- Liste -->
						<div class="achievements-list">
							{#snippet achievementCard(ach: Achievement, isArchived: boolean)}
								{@const isUnlocked = ach.unlocked || Boolean(ach.unlocked_time)}
								{@const isLocked = !isUnlocked}
								{@const isSecret = ach.hidden ?? false}
								{@const descText = (ach.desc || '').trim()}
								{@const showDescription =
									Boolean(descText) && (isUnlocked || !isSecret || revealed)}
								{@const showMaskedDescription = isLocked && isSecret && !revealed}
								{@const palette = rarityPalette(ach.completionpercentage)}
								{@const displayIcon = isLocked && ach.icon_gray ? ach.icon_gray : ach.icon}
								{@const achNormalizedKey = normalizeAchievementKey(ach.key)}
								{@const pinned = pinnedSet.has(achNormalizedKey)}
								{@const isTopbarMatch =
									globalAchievementHighlight &&
									(ach.name.toLowerCase().includes(globalAchievementHighlight) ||
										(ach.desc ?? '').toLowerCase().includes(globalAchievementHighlight))}

								<div
									class="ach-card"
									class:is-unlocked={isUnlocked}
									class:is-archived={isArchived}
									class:is-secret-hidden={showMaskedDescription}
									class:highlighted={Boolean(isTopbarMatch)}
									class:jump-flash={flashAchievementKey === achNormalizedKey}
									style:border-color={palette.border}
									use:trackAchievementRow={ach.key}
								>
									<div class="ach-card-inner">
										<div class="ach-icon-box">
											{#if displayIcon}
												<img
													src={displayIcon}
													alt={ach.name}
													class:muted-icon={isLocked}
													loading="lazy"
													decoding="async"
													onerror={(event) => {
														const target = event.currentTarget as HTMLImageElement;
														target.style.display = 'none';
														target.parentElement?.classList.add('icon-error');
													}}
												/>
												<span class="ach-placeholder icon-fallback">
													<svg
														width="24"
														height="24"
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
												</span>
											{:else}
												<span class="ach-placeholder">
													<svg
														width="24"
														height="24"
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
												</span>
											{/if}
										</div>

										<div class="ach-details">
											<div class="ach-header-row">
												<div class="ach-title-wrap">
													<div class="ach-name" class:muted={isLocked}>
														{isLocked && isSecret && !revealed
															? $i18n.t('game.secretAchievement')
															: ach.name || ach.key}
													</div>
													{#if isArchived}
														<span class="ach-archived-badge">{$i18n.t('game.archived')}</span>
													{/if}
												</div>
												{#if isUnlocked && ach.unlocked_time}
													<div class="ach-date tabular-nums">{formatDate(ach.unlocked_time)}</div>
												{/if}
											</div>

											<div class="ach-desc-row">
												{#if showDescription}
													<div class="ach-desc">{descText}</div>
												{:else if showMaskedDescription}
													<div class="ach-desc italic">{$i18n.t('game.descHidden')}</div>
												{/if}
											</div>

											<div class="ach-footer-row">
												{#if isLocked && ach.max_progress && ach.max_progress > 0}
													{@const cur = ach.progress ?? 0}
													{@const pct = Math.min(Math.max((cur / ach.max_progress) * 100, 0), 100)}
													<div class="ach-progress-info">
														<span class="ach-progress-tag tabular-nums">
															{formatProgressVal(cur)} / {formatProgressVal(ach.max_progress)}
														</span>
														<div class="rarity-progress-track">
															<div class="ach-progress-fill" style:width="{pct}%"></div>
														</div>
													</div>
												{/if}

												{#if ach.completionpercentage}
													<div class="rarity-info">
														<span
															class="rarity-tag tabular-nums"
															style:background={palette.badgeBg}
															style:color={palette.badgeColor}
														>
															{rarityLabel(ach.completionpercentage)} | {parseFloat(
																ach.completionpercentage
															).toFixed(1)}% · +{getAchievementPoints(ach)} G
														</span>
														<div class="rarity-progress-track">
															<div
																class="rarity-progress-fill"
																style:background={palette.fill}
																style:width={`${Math.min(parseFloat(ach.completionpercentage), 100)}%`}
															></div>
														</div>
													</div>
												{/if}

												<div class="ach-actions">
													<button
														class="action-btn"
														class:is-copied={editingAchKey === ach.key}
														onclick={() => openAchievementEditor(ach)}
														title={$i18n.t('game.editAchievementDate')}
														aria-label={$i18n.t('game.editAchievementDate')}
													>
														<svg
															width="11"
															height="11"
															viewBox="0 0 24 24"
															fill="none"
															stroke="currentColor"
															stroke-width="2.5"
															stroke-linecap="round"
															stroke-linejoin="round"
														>
															<path d="M12 20h9" />
															<path d="M16.5 3.5a2.121 2.121 0 0 1 3 3L7 19l-4 1 1-4L16.5 3.5z" />
														</svg>
														{$i18n.t('game.edit')}
													</button>
													<button
														class="action-btn"
														class:is-copied={pinned}
														onclick={() => togglePin(ach.key)}
													>
														<svg
															width="11"
															height="11"
															viewBox="0 0 24 24"
															fill={pinned ? 'currentColor' : 'none'}
															stroke="currentColor"
															stroke-width="2.5"
															stroke-linecap="round"
															stroke-linejoin="round"
														>
															<line x1="12" y1="17" x2="12" y2="22" />
															<path
																d="M5 17h14v-1.76a2 2 0 0 0-1.11-1.79l-1.78-.9A2 2 0 0 1 15 10.76V6h1a2 2 0 0 0 0-4H8a2 2 0 0 0 0 4h1v4.76a2 2 0 0 1-1.11 1.79l-1.78.9A2 2 0 0 0 5 15.24Z"
															/>
														</svg>
														{pinned ? $i18n.t('hud.unpin') : $i18n.t('hud.pin')}
													</button>
													<button
														class="action-btn"
														class:is-copied={copiedKey === ach.key}
														onclick={() => copyToClipboard(ach.name || ach.key, ach.key)}
													>
														{#if copiedKey === ach.key}
															<svg
																width="11"
																height="11"
																viewBox="0 0 24 24"
																fill="none"
																stroke="currentColor"
																stroke-width="3"
															>
																<polyline points="20 6 9 17 4 12" />
															</svg>
															{$i18n.t('game.copied')}
														{:else}
															<svg
																width="11"
																height="11"
																viewBox="0 0 24 24"
																fill="none"
																stroke="currentColor"
																stroke-width="3"
															>
																<rect x="9" y="9" width="13" height="13" rx="2" ry="2" /><path
																	d="M5 15H4a2 2 0 0 1-2-2V4a2 2 0 0 1 2-2h9a2 2 0 0 1 2 2v1"
																/>
															</svg>
															{$i18n.t('game.copyName')}
														{/if}
													</button>
													<button
														class="action-btn"
														onclick={() => searchGuide(game.name, ach.name || ach.key)}
													>
														<svg
															width="11"
															height="11"
															viewBox="0 0 24 24"
															fill="none"
															stroke="currentColor"
															stroke-width="3"
														>
															<path
																d="M18 13v6a2 2 0 0 1-2 2H5a2 2 0 0 1-2-2V8a2 2 0 0 1 2-2h6"
															/><polyline points="15 3 21 3 21 9" /><line
																x1="10"
																y1="14"
																x2="21"
																y2="3"
															/>
														</svg>
														{$i18n.t('game.findGuide')}
													</button>
												</div>
											</div>
										</div>
									</div>

									{#if editingAchKey === ach.key}
										<div class="ach-editor-panel">
											<div class="ach-editor-left">
												<button
													type="button"
													class="ach-state-toggle"
													class:unlocked={editAchUnlocked}
													onclick={() => (editAchUnlocked = !editAchUnlocked)}
												>
													{#if editAchUnlocked}
														<svg
															width="12"
															height="12"
															viewBox="0 0 24 24"
															fill="none"
															stroke="currentColor"
															stroke-width="2.8"
															stroke-linecap="round"
															stroke-linejoin="round"
															aria-hidden="true"
														>
															<polyline points="20 6 9 17 4 12" />
														</svg>
														<span>{$i18n.t('game.stateUnlocked')}</span>
													{:else}
														<svg
															width="12"
															height="12"
															viewBox="0 0 24 24"
															fill="none"
															stroke="currentColor"
															stroke-width="2.2"
															stroke-linecap="round"
															stroke-linejoin="round"
															aria-hidden="true"
														>
															<rect x="3" y="11" width="18" height="11" rx="2" ry="2" />
															<path d="M7 11V7a5 5 0 0 1 10 0v4" />
														</svg>
														<span>{$i18n.t('game.stateLocked')}</span>
													{/if}
												</button>

												{#if editAchUnlocked}
													<div class="ach-dt-group">
														<span class="ach-dt-label">{$i18n.t('game.dateTimeLabel')}</span>
														<input
															type="datetime-local"
															class="ach-dt-input"
															bind:value={editAchDateTime}
														/>
														<button type="button" class="ach-dt-now" onclick={setEditDateTimeNow}>
															{$i18n.t('game.now')}
														</button>
													</div>
												{/if}
											</div>

											<div class="ach-editor-right">
												<button
													type="button"
													class="ach-save-btn"
													disabled={savingAchEdit}
													onclick={() => saveAchievementEdit(ach.key)}
												>
													{savingAchEdit ? '...' : $i18n.t('game.save')}
												</button>
												<button
													type="button"
													class="ach-cancel-btn"
													onclick={() => (editingAchKey = null)}
												>
													{$i18n.t('game.cancel')}
												</button>
											</div>
										</div>
									{/if}
								</div>
							{/snippet}

							{#if loading}
								<div class="loading-state">{$i18n.t('game.loading')}</div>
							{:else if hasSets}
								<div class="ach-sections">
									{#each visibleSections as section (section.id)}
										{#if section.filteredAchievements.length > 0}
											<section class="ach-set-section">
												<header class="ach-set-header">
													<div class="ach-set-title-group">
														<h3 class="ach-set-title">{section.title}</h3>
														{#if section.isDlc}
															<span class="set-badge dlc">{$i18n.t('game.dlcBadge')}</span>
														{/if}
														{#if section.archived}
															<span class="set-badge archived">{$i18n.t('game.archived')}</span>
														{/if}
													</div>
													<div class="ach-set-progress">
														<span class="ach-set-stats tabular-nums">
															{section.unlocked} / {section.total} ({section.displayPct}%)
														</span>
														<div class="ach-set-bar-track">
															<div
																class="ach-set-bar-fill"
																class:complete={section.total > 0 &&
																	section.unlocked === section.total}
																style:width="{section.pct}%"
															></div>
														</div>
													</div>
												</header>
												<div class="ach-grid">
													{#each section.filteredAchievements as ach (ach.key)}
														{@render achievementCard(
															ach,
															Boolean(ach.archived || section.archived)
														)}
													{/each}
												</div>
											</section>
										{/if}
									{/each}
								</div>

								{#if filteredCount === 0}
									<div class="empty-state">{$i18n.t('game.noAchievementFound')}</div>
								{/if}
							{:else}
								<div class="ach-grid">
									{#each flatFiltered as item (item.ach.key)}
										{@render achievementCard(item.ach, item.isArchived)}
									{/each}
								</div>

								{#if filteredCount === 0}
									<div class="empty-state">{$i18n.t('game.noAchievementFound')}</div>
								{/if}
							{/if}
						</div>
					{:else}
						<div class="achievements-list">
							<div class="progress-map">
								<div class="map-intro">
									<div class="map-intro-copy">
										<h2>{$i18n.t('game.shareCard')}</h2>
										<p>{$i18n.t('game.timelineStepHint')}</p>
									</div>
									<strong class="map-overall-progress">{unlockedCount}/{totalCount}</strong>
								</div>
								<div class="map-track">
									{#each mapAchievements as ach, index (ach.key)}
										{@const palette = rarityPalette(ach.completionpercentage)}
										{@const isUnlocked = ach.unlocked || Boolean(ach.unlocked_time)}
										<div
											class="map-achievement-card"
											class:unlocked={isUnlocked}
											role="button"
											tabindex="0"
											onclick={() => void openAchievementFromMap(ach.key)}
											onkeydown={(event) => {
												if (event.key === 'Enter' || event.key === ' ') {
													event.preventDefault();
													void openAchievementFromMap(ach.key);
												}
											}}
											aria-label={$i18n.t('game.viewDescriptionAria', {
												name: ach.name || ach.key
											})}
										>
											<div
												class="map-achievement-index"
												class:complete={isUnlocked}
												aria-label={$i18n.t('game.stepIndexAria', {
													index: index + 1,
													total: mapAchievements.length
												})}
											>
												<span>{index + 1}</span>
												<small>/{mapAchievements.length}</small>
											</div>
											<div class="map-achievement-icon">
												{#if ach.icon || ach.icon_gray}
													<img
														src={isUnlocked ? ach.icon : ach.icon_gray || ach.icon}
														alt=""
														loading="lazy"
														decoding="async"
													/>
												{:else}
													<svg
														width="20"
														height="20"
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
											<div class="map-achievement-content">
												<div class="map-achievement-title">{ach.name || ach.key}</div>
												<div class="map-achievement-description">
													{ach.desc || $i18n.t('game.achievementToUnlock')}
												</div>
												<div class="map-achievement-meta">
													<span class="map-rarity tabular-nums" style:color={palette.badgeColor}
														>{rarityLabel(ach.completionpercentage) || $i18n.t('game.unknown')} · {parseFloat(
															ach.completionpercentage || '0'
														).toFixed(1)}%</span
													>
													<div class="map-rarity-track">
														<div
															class="map-rarity-fill"
															style:width={`${Math.min(parseFloat(ach.completionpercentage) || 0, 100)}%`}
															style:background={palette.fill}
														></div>
													</div>
												</div>
											</div>
											<div class="map-achievement-status" class:complete={isUnlocked}>
												{#if isUnlocked}
													<svg
														width="13"
														height="13"
														viewBox="0 0 24 24"
														fill="none"
														stroke="currentColor"
														stroke-width="3"
														stroke-linecap="round"
														stroke-linejoin="round"
														aria-hidden="true"
													>
														<polyline points="20 6 9 17 4 12" />
													</svg>
												{:else}
													<svg
														width="6"
														height="6"
														viewBox="0 0 8 8"
														fill="currentColor"
														aria-hidden="true"
													>
														<circle cx="4" cy="4" r="3" />
													</svg>
												{/if}
											</div>
										</div>
									{/each}
								</div>
							</div>
						</div>
					{/if}
				</div>
			</div>
		</div>

		{#if exportNotice}
			<div
				class="export-toast"
				class:is-error={exportNotice.type === 'error'}
				role="status"
				aria-live="polite"
			>
				{#if exportNotice.type === 'error'}
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
						<circle cx="12" cy="12" r="10" /><line x1="12" y1="8" x2="12" y2="12" /><line
							x1="12"
							y1="16"
							x2="12.01"
							y2="16"
						/>
					</svg>
				{:else}
					<svg
						width="15"
						height="15"
						viewBox="0 0 24 24"
						fill="none"
						stroke="currentColor"
						stroke-width="2.4"
						stroke-linecap="round"
						stroke-linejoin="round"
						aria-hidden="true"
					>
						<polyline points="20 6 9 17 4 12" />
					</svg>
				{/if}
				<span class="export-toast-msg">{exportNotice.message}</span>
				<button
					type="button"
					class="export-toast-close"
					aria-label={$i18n.t('button.close')}
					onclick={() => (exportNotice = null)}
				>
					<svg
						width="12"
						height="12"
						viewBox="0 0 24 24"
						fill="none"
						stroke="currentColor"
						stroke-width="2.5"
						stroke-linecap="round"
						aria-hidden="true"
					>
						<line x1="18" y1="6" x2="6" y2="18" /><line x1="6" y1="6" x2="18" y2="18" />
					</svg>
				</button>
			</div>
		{/if}
	</div>
{/if}

<style>
	.game-page {
		height: 100%;
		display: flex;
		flex-direction: column;
		overflow: hidden;
		background-color: var(--bg-app);
		background-image: var(--game-bg);
		background-size: cover;
		background-position: center top;
		background-attachment: fixed;
		position: relative;
		isolation: isolate;
	}

	.game-scroll-area {
		flex: 1;
		overflow-y: auto;
		overflow-y: overlay;
		overflow-x: hidden;
		scrollbar-width: thin;
		scrollbar-color: var(--border-soft) transparent;
	}

	.game-scroll-area::-webkit-scrollbar {
		width: 6px;
	}
	.game-scroll-area::-webkit-scrollbar-thumb {
		background: var(--border-soft);
		border-radius: 10px;
	}
	.game-scroll-area::-webkit-scrollbar-track {
		background: transparent;
	}

	.bg-overlay {
		position: fixed;
		inset: 0;
		background: linear-gradient(
			180deg,
			rgba(8, 10, 15, 0.34) 0%,
			color-mix(in srgb, var(--bg-app) 86%, transparent) 38%,
			var(--bg-app) 100%
		);
		z-index: -1;
		pointer-events: none;
	}

	.content-wrapper {
		position: relative;
		z-index: 1;
		display: flex;
		flex-direction: column;
		padding: 68px 32px 40px;
	}

	.not-found,
	.loading-state,
	.empty-state {
		display: flex;
		align-items: center;
		justify-content: center;
		color: var(--text-muted);
		font-size: 14px;
	}
	.not-found {
		height: 100%;
	}
	.loading-state,
	.empty-state {
		height: 200px;
	}

	.game-nav {
		position: absolute;
		top: 0;
		left: 0;
		right: 0;
		z-index: 100;
		display: flex;
		align-items: center;
		justify-content: space-between;
		padding: 16px 32px;
		flex-shrink: 0;
		background: color-mix(in srgb, var(--bg-panel) 78%, transparent);
		backdrop-filter: blur(16px);
		border-bottom: 1px solid var(--border-soft);
	}

	.nav-btn {
		display: flex;
		align-items: center;
		gap: 8px;
		background: var(--surface-2);
		border: 1px solid var(--border-soft);
		border-radius: 8px;
		padding: 6px 14px;
		color: var(--text-secondary);
		font-size: 11px;
		font-weight: 600;
		cursor: pointer;
		transition: all 0.2s;
		max-width: 180px;
	}
	.nav-btn:hover:not(:disabled) {
		background: var(--surface-hover);
		color: var(--text-primary);
		border-color: var(--border-specular);
	}
	.nav-btn:disabled {
		opacity: 0.28;
		cursor: default;
	}
	.nav-label {
		white-space: nowrap;
		overflow: hidden;
		text-overflow: ellipsis;
	}
	.nav-index {
		font-size: 12px;
		color: var(--text-muted);
		font-weight: 700;
		letter-spacing: 0.5px;
		font-variant-numeric: tabular-nums;
	}

	.game-header {
		padding: 8px 0 32px;
		flex-shrink: 0;
	}
	.game-title-row {
		display: flex;
		align-items: center;
		justify-content: space-between;
		margin-bottom: 24px;
	}
	.game-title {
		font-size: 42px;
		font-weight: 800;
		color: #fff;
		text-shadow: 0 2px 14px rgba(0, 0, 0, 0.45);
		letter-spacing: -1px;
	}
	.title-with-fav {
		display: flex;
		align-items: center;
		gap: 20px;
	}
	.big-fav-btn {
		background: transparent;
		border: none;
		cursor: pointer;
		color: rgba(255, 255, 255, 0.45);
		transition: all 0.2s var(--spring-out);
		display: flex;
		align-items: center;
		justify-content: center;
		padding: 4px;
	}
	.big-fav-btn:hover {
		transform: scale(1.1);
		color: rgba(255, 255, 255, 0.8);
	}
	.big-fav-btn.favorited {
		color: #ff4d4d;
	}
	.emulator-badge {
		font-size: 10px;
		font-weight: 700;
		color: #fff;
		background: rgba(0, 0, 0, 0.42);
		border: 1px solid rgba(255, 255, 255, 0.2);
		border-radius: 20px;
		padding: 4px 14px;
		text-transform: uppercase;
		letter-spacing: 1.2px;
		backdrop-filter: blur(8px);
	}

	.progress-card-container {
		display: flex;
		flex-direction: column;
		gap: 14px;
	}

	.progress-card {
		background: color-mix(in srgb, var(--surface-1) 90%, transparent);
		border: 1px solid var(--border-soft);
		border-radius: 16px;
		padding: 24px 32px;
		display: flex;
		gap: 48px;
		backdrop-filter: blur(16px);
		box-shadow: var(--shadow-elevated);
	}

	.stat-label-row {
		display: flex;
		align-items: center;
		gap: 6px;
	}
	.mini-edit-icon {
		background: transparent;
		border: none;
		color: var(--text-muted);
		cursor: pointer;
		padding: 2px;
		line-height: 1;
		display: inline-flex;
		align-items: center;
		justify-content: center;
		transition: color 0.15s;
	}
	.mini-edit-icon:hover {
		color: var(--accent, #c8a96e);
	}
	.playtime-val {
		color: #4ac8ff !important;
	}
	.playtime-inline-edit {
		display: flex;
		align-items: center;
		gap: 4px;
		font-size: 12px;
		color: var(--text-secondary);
	}
	.pt-input {
		width: 48px;
		background: var(--surface-2);
		border: 1px solid var(--border-soft);
		border-radius: 6px;
		color: var(--text-primary);
		font-size: 12px;
		font-weight: 700;
		padding: 3px 6px;
		outline: none;
		font-variant-numeric: tabular-nums;
	}
	.pt-save-btn {
		background: var(--accent, #c8a96e);
		color: var(--accent-text);
		border: none;
		border-radius: 6px;
		padding: 4px 8px;
		display: inline-flex;
		align-items: center;
		justify-content: center;
		cursor: pointer;
	}

	/* HowLongToBeat Bar */
	.hltb-bar {
		background: color-mix(in srgb, var(--surface-1) 88%, transparent);
		border: 1px solid var(--border-soft);
		border-radius: 14px;
		padding: 12px 20px;
		display: flex;
		align-items: center;
		gap: 24px;
		backdrop-filter: blur(12px);
		box-shadow: var(--shadow-card);
	}
	.hltb-brand {
		display: flex;
		align-items: center;
		gap: 8px;
		flex-shrink: 0;
	}
	.hltb-badge {
		background: rgba(74, 200, 255, 0.15);
		border: 1px solid rgba(74, 200, 255, 0.35);
		color: #4ac8ff;
		font-size: 10px;
		font-weight: 900;
		padding: 3px 8px;
		border-radius: 6px;
		letter-spacing: 0.08em;
	}
	.hltb-metrics {
		flex: 1;
		display: grid;
		grid-template-columns: repeat(3, 1fr);
		gap: 20px;
	}
	.hltb-metric {
		display: flex;
		flex-direction: column;
		gap: 5px;
	}
	.hltb-metric-top {
		display: flex;
		justify-content: space-between;
		align-items: baseline;
	}
	.hltb-label {
		font-size: 11px;
		color: var(--text-secondary);
		font-weight: 600;
	}
	.hltb-hours {
		font-size: 13px;
		font-weight: 800;
		color: var(--text-primary);
		font-variant-numeric: tabular-nums;
	}
	.hltb-track {
		height: 4px;
		background: var(--surface-2);
		border-radius: 3px;
		overflow: hidden;
	}
	.hltb-fill {
		height: 100%;
		background: #4ac8ff;
		border-radius: 3px;
		transition: width 0.4s ease;
	}
	.hltb-fill.extra {
		background: #a855f7;
	}
	.hltb-fill.comp {
		background: var(--accent, #c8a96e);
	}
	.hltb-actions {
		display: flex;
		align-items: center;
		gap: 6px;
		flex-shrink: 0;
	}
	.hltb-icon-btn {
		background: var(--surface-2);
		border: 1px solid var(--border-soft);
		color: var(--text-secondary);
		width: 26px;
		height: 26px;
		border-radius: 6px;
		cursor: pointer;
		display: flex;
		align-items: center;
		justify-content: center;
		transition: all 0.15s;
	}
	.hltb-icon-btn:hover:not(:disabled) {
		background: var(--surface-hover);
		color: var(--text-primary);
	}
	.hltb-edit-row {
		flex: 1;
		display: flex;
		align-items: center;
		gap: 12px;
		flex-wrap: wrap;
	}
	.hltb-edit-row label {
		display: flex;
		align-items: center;
		gap: 6px;
		font-size: 11px;
		color: var(--text-secondary);
	}
	.hltb-edit-row input {
		width: 68px;
		background: var(--surface-2);
		border: 1px solid var(--border-soft);
		border-radius: 6px;
		padding: 4px 8px;
		color: var(--text-primary);
		font-size: 12px;
		outline: none;
		font-variant-numeric: tabular-nums;
	}
	.hltb-btn {
		background: var(--surface-2);
		border: 1px solid var(--border-soft);
		color: var(--text-secondary);
		border-radius: 6px;
		padding: 5px 10px;
		font-size: 11px;
		font-weight: 700;
		cursor: pointer;
	}
	.hltb-btn.primary {
		background: var(--accent, #c8a96e);
		border-color: var(--accent, #c8a96e);
		color: var(--accent-text);
	}

	/* Manual Achievement Editor Panel */
	.ach-editor-panel {
		display: flex;
		align-items: center;
		justify-content: space-between;
		gap: 12px;
		padding: 10px 16px;
		border-top: 1px solid var(--border-soft);
		background: var(--surface-2);
		border-radius: 0 0 14px 14px;
		flex-wrap: wrap;
	}
	.ach-editor-left {
		display: flex;
		align-items: center;
		gap: 14px;
		flex-wrap: wrap;
	}
	.ach-state-toggle {
		display: inline-flex;
		align-items: center;
		gap: 6px;
		background: var(--surface-1);
		border: 1px solid var(--border-soft);
		color: var(--text-secondary);
		border-radius: 8px;
		padding: 5px 12px;
		font-size: 11px;
		font-weight: 800;
		cursor: pointer;
		transition: all 0.15s;
	}
	.ach-state-toggle.unlocked {
		background: rgba(61, 220, 132, 0.15);
		border-color: rgba(61, 220, 132, 0.4);
		color: #3ddc84;
	}
	.ach-dt-group {
		display: flex;
		align-items: center;
		gap: 8px;
	}
	.ach-dt-label {
		font-size: 11px;
		color: var(--text-secondary);
		font-weight: 600;
	}
	.ach-dt-input {
		background: var(--surface-1);
		border: 1px solid var(--border-soft);
		border-radius: 6px;
		padding: 4px 8px;
		color: var(--text-primary);
		font-size: 11px;
		color-scheme: dark;
		outline: none;
		font-variant-numeric: tabular-nums;
	}
	:global([data-theme='light']) .ach-dt-input {
		color-scheme: light;
	}
	.ach-dt-now {
		background: var(--surface-1);
		border: 1px solid var(--border-soft);
		color: var(--text-secondary);
		border-radius: 6px;
		padding: 4px 8px;
		font-size: 10px;
		font-weight: 700;
		cursor: pointer;
	}
	.ach-dt-now:hover {
		background: var(--surface-hover);
		color: var(--text-primary);
	}
	.ach-editor-right {
		display: flex;
		align-items: center;
		gap: 8px;
	}
	.ach-save-btn {
		background: var(--accent, #c8a96e);
		border: none;
		color: var(--accent-text);
		border-radius: 6px;
		padding: 5px 12px;
		font-size: 11px;
		font-weight: 800;
		cursor: pointer;
	}
	.ach-cancel-btn {
		background: transparent;
		border: 1px solid var(--border-soft);
		color: var(--text-secondary);
		border-radius: 6px;
		padding: 5px 10px;
		font-size: 11px;
		font-weight: 600;
		cursor: pointer;
	}

	.tags-section {
		display: flex;
		align-items: center;
		padding: 0 4px;
	}

	.tags-list {
		display: flex;
		flex-wrap: wrap;
		gap: 8px;
		align-items: center;
	}

	.tag {
		background: color-mix(in srgb, var(--accent, #c8a96e) 12%, var(--surface-1));
		border: 1px solid color-mix(in srgb, var(--accent, #c8a96e) 28%, transparent);
		color: var(--accent);
		padding: 4px 10px;
		border-radius: 6px;
		font-size: 11px;
		font-weight: 600;
		display: flex;
		align-items: center;
		gap: 6px;
	}

	.remove-tag {
		background: transparent;
		border: none;
		color: inherit;
		cursor: pointer;
		font-size: 14px;
		padding: 0;
		line-height: 1;
		opacity: 0.6;
	}

	.remove-tag:hover {
		opacity: 1;
	}

	.add-tag-form {
		display: flex;
		align-items: center;
		background: var(--surface-1);
		border: 1px solid var(--border-soft);
		border-radius: 6px;
		padding: 2px;
	}

	.add-tag-form input {
		background: transparent;
		border: none;
		color: var(--text-primary);
		font-size: 11px;
		padding: 2px 8px;
		outline: none;
		width: 110px;
	}
	.add-tag-form input::placeholder {
		color: var(--text-muted);
	}

	.add-tag-form button {
		background: var(--accent);
		color: var(--accent-text);
		border: none;
		width: 18px;
		height: 18px;
		border-radius: 4px;
		display: flex;
		align-items: center;
		justify-content: center;
		cursor: pointer;
		font-weight: bold;
	}
	.progress-left {
		flex: 1;
		min-width: 0;
		display: flex;
		flex-direction: column;
		gap: 16px;
	}
	.progress-left.dual-gauges {
		gap: 14px;
	}
	.gauge-block {
		display: flex;
		flex-direction: column;
		gap: 8px;
	}
	.gauge-block.is-platinum .gauge-title {
		color: var(--completed, #4caf6e);
	}
	.gauge-header {
		display: flex;
		align-items: baseline;
		justify-content: space-between;
		gap: 12px;
	}
	.gauge-title {
		display: inline-flex;
		align-items: center;
		gap: 8px;
		font-size: 12px;
		font-weight: 700;
		color: var(--text-secondary);
		letter-spacing: 0.3px;
	}
	.platinum-pill {
		font-size: 10px;
		font-weight: 800;
		padding: 1px 7px;
		border-radius: 999px;
		background: color-mix(in srgb, var(--completed, #4caf6e) 18%, transparent);
		border: 1px solid color-mix(in srgb, var(--completed, #4caf6e) 45%, transparent);
		color: var(--completed, #4caf6e);
	}
	.gauge-numbers {
		display: inline-flex;
		align-items: baseline;
		gap: 10px;
		flex-shrink: 0;
	}
	.gauge-count {
		font-size: 16px;
		font-weight: 800;
		color: var(--text-primary);
	}
	.gauge-slash {
		color: var(--text-muted);
		font-size: 13px;
		font-weight: 600;
		margin-left: 2px;
	}
	.gauge-pct {
		font-size: 18px;
		font-weight: 900;
	}
	.progress-stats {
		display: flex;
		align-items: baseline;
		justify-content: space-between;
	}
	.main-count {
		font-size: 32px;
		font-weight: 800;
		color: var(--text-primary);
		font-variant-numeric: tabular-nums;
	}
	.slash {
		color: var(--text-muted);
		font-size: 20px;
		font-weight: 600;
		margin-left: 6px;
	}
	.main-pct {
		font-size: 32px;
		font-weight: 900;
		color: var(--accent, #c8a96e);
		font-variant-numeric: tabular-nums;
	}
	.progress-bar-track {
		height: 10px;
		background: var(--surface-2);
		border-radius: 5px;
		overflow: hidden;
	}
	.progress-bar-track.secondary {
		height: 7px;
	}
	.progress-bar-fill {
		height: 100%;
		background: #3ddc84;
		border-radius: 5px;
		transition: width 1s cubic-bezier(0.19, 1, 0.22, 1);
	}

	.progress-right {
		display: flex;
		gap: 40px;
		border-left: 1px solid var(--border-soft);
		padding-left: 48px;
		align-items: center;
	}
	.stat-item {
		display: flex;
		flex-direction: column;
		gap: 6px;
	}
	.stat-label {
		font-size: 11px;
		font-weight: 700;
		color: var(--text-muted);
		letter-spacing: 1.5px;
	}
	.stat-value {
		font-size: 24px;
		font-weight: 800;
		color: var(--text-primary);
		font-variant-numeric: tabular-nums;
	}
	.stat-value.highlight {
		color: #3ddc84;
	}

	.main-section {
		flex: 1;
		display: flex;
		flex-direction: column;
		overflow: hidden;
		background: color-mix(in srgb, var(--bg-panel) 88%, transparent);
		border: 1px solid var(--border-soft);
		border-radius: 20px 20px 0 0;
		backdrop-filter: blur(12px);
		box-shadow: var(--shadow-card);
		min-width: 0;
	}

	.controls-bar {
		display: flex;
		flex-direction: column;
		align-items: stretch;
		gap: 12px;
		padding: 20px 28px;
		border-bottom: 1px solid var(--border-soft);
		flex-shrink: 0;
		overflow: visible;
	}

	.view-tabs,
	.controls-bottom {
		display: flex;
		align-items: center;
		gap: 10px;
		min-width: 0;
	}

	.view-tabs {
		align-self: flex-start;
	}

	.controls-bottom {
		width: 100%;
		flex-wrap: wrap;
		gap: 8px;
	}

	.filter-tabs {
		display: flex;
		flex: 0 1 auto;
		min-width: 0;
		background: var(--surface-2);
		border-radius: 10px;
		padding: 4px;
		gap: 4px;
	}
	.tab {
		padding: 8px 14px;
		border-radius: 8px;
		border: none;
		background: transparent;
		cursor: pointer;
		font-size: 12px;
		color: var(--text-muted);
		font-weight: 600;
		white-space: nowrap;
		transition: all 0.25s;
	}
	.tab.active {
		background: var(--accent, #0066cc);
		color: var(--accent-text);
	}
	.tab:hover:not(.active) {
		color: var(--text-primary);
		background: var(--surface-hover);
	}

	.profile-btn-mini {
		background: var(--surface-2);
		border: 1px solid var(--border-soft);
		border-radius: 8px;
		color: var(--text-secondary);
		cursor: pointer;
		display: flex;
		align-items: center;
		justify-content: center;
		padding: 8px;
		transition: all 0.2s var(--spring-out);
		flex-shrink: 0;
	}

	.profile-btn-mini:hover {
		background: var(--surface-hover);
		border-color: var(--accent);
		color: var(--accent);
		transform: translateY(-1px);
	}

	.profile-btn-mini:active {
		transform: scale(0.95);
	}

	.search-wrap {
		flex: 1 1 180px;
		min-width: 150px;
		max-width: none;
		height: 38px;
		background: var(--surface-2);
		border: 1px solid var(--border-soft);
		border-radius: 10px;
		display: flex;
		align-items: center;
		gap: 12px;
		padding: 0 14px;
		color: var(--text-muted);
	}
	.search-input {
		flex: 1;
		background: transparent;
		border: none;
		outline: none;
		font-size: 13px;
		color: var(--text-primary);
	}
	.search-input::placeholder {
		color: var(--text-muted);
	}
	.search-count {
		font-size: 12px;
		color: var(--text-muted);
		font-weight: 700;
		white-space: nowrap;
		flex-shrink: 0;
		font-variant-numeric: tabular-nums;
	}

	.export-actions {
		display: flex;
		align-items: center;
		gap: 6px;
		flex-shrink: 0;
		margin-left: auto;
	}

	.sort-select {
		flex: 0 1 160px;
		min-width: 125px;
		height: 38px;
		background: var(--surface-2);
		border: 1px solid var(--border-soft);
		border-radius: 10px;
		padding: 0 14px;
		font-size: 13px;
		color: var(--text-primary);
		outline: none;
		cursor: pointer;
		color-scheme: dark;
		transition:
			border-color 0.2s,
			background 0.2s;
	}
	:global([data-theme='light']) .sort-select {
		color-scheme: light;
	}
	.sort-select:hover {
		background: var(--surface-hover);
	}
	.sort-select:focus {
		border-color: var(--accent, #0066cc);
	}
	.sort-select option {
		background: var(--surface-1);
		color: var(--text-primary);
	}

	.order-btn {
		height: 38px;
		width: 38px;
		display: flex;
		align-items: center;
		justify-content: center;
		background: var(--surface-2);
		border: 1px solid var(--border-soft);
		border-radius: 10px;
		color: var(--text-secondary);
		cursor: pointer;
		transition: all 0.2s;
	}
	.order-btn:hover {
		background: var(--surface-hover);
		color: var(--text-primary);
	}

	.reveal-btn {
		height: 38px;
		padding: 0 12px;
		background: var(--surface-2);
		border: 1px solid var(--border-soft);
		border-radius: 10px;
		font-size: 13px;
		color: var(--text-secondary);
		cursor: pointer;
		display: flex;
		align-items: center;
		gap: 8px;
		transition: all 0.2s;
		flex: 0 1 auto;
		white-space: nowrap;
	}
	.reveal-btn.active {
		background: color-mix(in srgb, var(--accent, #0066cc) 14%, transparent);
		border-color: color-mix(in srgb, var(--accent, #0066cc) 35%, transparent);
		color: var(--accent, #c8a96e);
	}
	.reveal-btn span {
		font-weight: 600;
	}

	.map-summary {
		display: flex;
		align-items: center;
		gap: 10px;
		margin-right: auto;
		color: var(--text-muted);
		font-size: 12px;
	}
	.map-summary-title {
		color: var(--text-primary);
		font-weight: 700;
	}
	.map-summary-value {
		color: var(--accent);
		font-weight: 700;
		font-variant-numeric: tabular-nums;
	}

	.achievements-list {
		flex: 1;
		padding: 24px 28px;
	}
	.progress-map {
		min-height: 100%;
		padding: 8px 0 32px;
	}
	.map-intro {
		display: flex;
		align-items: center;
		justify-content: space-between;
		gap: 20px;
		margin-bottom: 28px;
	}
	.map-intro-copy {
		min-width: 0;
	}
	.map-intro h2 {
		margin: 0 0 6px;
		color: var(--text-primary);
		font-size: 20px;
	}
	.map-intro p {
		margin: 0;
		color: var(--text-muted);
		font-size: 13px;
	}
	.map-track {
		display: grid;
		grid-template-columns: repeat(auto-fit, minmax(260px, 1fr));
		gap: 10px;
		padding: 0 2px;
	}
	.map-overall-progress {
		color: var(--accent);
		font-size: 16px;
		font-variant-numeric: tabular-nums;
		white-space: nowrap;
	}
	.map-achievement-index {
		width: 42px;
		height: 42px;
		border: 2px solid var(--border-soft);
		border-radius: 50%;
		flex-shrink: 0;
		display: flex;
		align-items: center;
		justify-content: center;
		gap: 1px;
		color: var(--text-secondary);
		background: var(--surface-2);
		font-size: 13px;
		font-weight: 800;
		font-variant-numeric: tabular-nums;
	}
	.map-achievement-index span,
	.map-achievement-index small {
		font: inherit;
		line-height: 1;
	}
	.map-achievement-index small {
		font-size: 9px;
		font-weight: 700;
		color: var(--text-muted);
	}
	.map-achievement-index.complete {
		border-color: var(--accent);
		color: var(--accent);
	}
	.map-achievement-card {
		min-width: 0;
		min-height: 78px;
		display: flex;
		align-items: center;
		gap: 12px;
		padding: 10px;
		background: var(--surface-1);
		border: 1px solid var(--border-soft);
		border-radius: 10px;
		cursor: pointer;
		box-shadow: var(--shadow-card);
		transition:
			transform 0.2s var(--spring-out),
			border-color 0.2s ease,
			box-shadow 0.2s ease;
	}
	.map-achievement-card:focus-visible {
		outline: 2px solid var(--accent);
		outline-offset: 2px;
	}
	.map-achievement-card:hover {
		transform: translateY(-2px);
		border-color: color-mix(in srgb, var(--accent) 60%, transparent);
		box-shadow: var(--shadow-elevated);
	}
	.map-achievement-card.unlocked {
		border-color: color-mix(in srgb, var(--accent) 35%, var(--border-soft));
	}
	.map-achievement-icon {
		width: 48px;
		height: 48px;
		border-radius: 8px;
		overflow: hidden;
		flex-shrink: 0;
		background: var(--surface-3);
		color: var(--text-muted);
		display: flex;
		align-items: center;
		justify-content: center;
	}
	.map-achievement-icon img {
		width: 100%;
		height: 100%;
		object-fit: cover;
	}
	.map-achievement-content {
		flex: 1;
		min-width: 0;
	}
	.map-achievement-title {
		color: var(--text-primary);
		font-size: 13px;
		font-weight: 700;
		white-space: nowrap;
		overflow: hidden;
		text-overflow: ellipsis;
	}
	.map-achievement-description {
		color: var(--text-muted);
		font-size: 11px;
		white-space: nowrap;
		overflow: hidden;
		text-overflow: ellipsis;
		margin-top: 3px;
	}
	.map-achievement-meta {
		display: flex;
		align-items: center;
		gap: 8px;
		margin-top: 7px;
	}
	.map-rarity {
		font-size: 9px;
		font-weight: 800;
		white-space: nowrap;
		text-transform: uppercase;
		font-variant-numeric: tabular-nums;
	}
	.map-rarity-track {
		height: 3px;
		flex: 1;
		min-width: 24px;
		border-radius: 3px;
		background: var(--surface-2);
		overflow: hidden;
	}
	.map-rarity-fill {
		height: 100%;
		border-radius: inherit;
	}
	.map-achievement-status {
		width: 24px;
		height: 24px;
		border-radius: 50%;
		flex-shrink: 0;
		display: flex;
		align-items: center;
		justify-content: center;
		color: var(--text-muted);
		border: 1px solid var(--border-soft);
	}
	.map-achievement-status.complete {
		background: var(--accent);
		color: var(--accent-text);
		border-color: var(--accent);
	}
	.ach-grid {
		display: flex;
		flex-direction: column;
		gap: 14px;
	}

	.ach-card {
		background: var(--surface-1);
		border: 1px solid var(--border-soft);
		border-radius: 14px;
		box-shadow: var(--shadow-card);
		transition:
			transform 0.22s var(--spring-out),
			border-color 0.2s ease,
			box-shadow 0.22s ease;
		position: relative;
		content-visibility: auto;
		contain-intrinsic-size: 0 88px;
	}
	.ach-card:hover {
		transform: translateY(-1px);
		border-color: var(--border-specular);
		box-shadow: var(--shadow-elevated);
	}
	.ach-card.is-unlocked {
		border-color: var(--border-soft);
	}
	.ach-card-inner {
		display: flex;
		gap: 16px;
		padding: 10px 16px;
		height: 88px;
		box-sizing: border-box;
		position: relative;
	}

	.ach-icon-box {
		width: 68px;
		height: 68px;
		border-radius: 10px;
		overflow: hidden;
		flex-shrink: 0;
		background: var(--surface-3);
		border: 1px solid var(--border-soft);
	}
	.ach-icon-box img {
		width: 100%;
		height: 100%;
		object-fit: cover;
	}
	.ach-icon-box img.muted-icon {
		opacity: 0.35;
		filter: grayscale(0.8);
	}
	.ach-placeholder {
		color: var(--text-muted);
		display: flex;
		align-items: center;
		justify-content: center;
		height: 100%;
	}
	.ach-placeholder.icon-fallback {
		display: none;
	}
	:global(.ach-icon-box.icon-error .icon-fallback) {
		display: flex;
	}

	.ach-details {
		flex: 1;
		min-width: 0;
		display: flex;
		flex-direction: column;
		justify-content: space-between;
		height: 68px;
	}
	.ach-header-row {
		display: flex;
		justify-content: space-between;
		align-items: center;
		gap: 12px;
		line-height: 1;
	}
	.ach-title-wrap {
		display: flex;
		align-items: center;
		gap: 8px;
		min-width: 0;
		flex: 1;
	}
	.ach-archived-badge {
		font-size: 9px;
		font-weight: 800;
		text-transform: uppercase;
		letter-spacing: 0.6px;
		padding: 2px 7px;
		border-radius: 999px;
		background: var(--surface-2);
		border: 1px solid var(--border-soft);
		color: var(--text-muted);
		flex-shrink: 0;
	}
	.ach-card.is-archived:not(.is-unlocked) {
		opacity: 0.78;
	}
	.ach-name {
		font-size: 16px;
		font-weight: 800;
		color: var(--text-primary);
		letter-spacing: -0.2px;
		white-space: nowrap;
		overflow: hidden;
		text-overflow: ellipsis;
	}
	.ach-name.muted {
		color: var(--text-muted);
	}
	.ach-date {
		font-size: 10px;
		color: var(--text-muted);
		font-weight: 700;
		letter-spacing: 0.5px;
		font-variant-numeric: tabular-nums;
	}

	.ach-desc-row {
		overflow: hidden;
	}
	.ach-desc {
		font-size: 12px;
		color: var(--text-secondary);
		line-height: 1.3;
		font-weight: 500;
		display: -webkit-box;
		-webkit-line-clamp: 2;
		line-clamp: 2;
		-webkit-box-orient: vertical;
		overflow: hidden;
	}
	.ach-desc.italic {
		font-style: italic;
		opacity: 0.55;
	}

	.ach-footer-row {
		display: flex;
		align-items: center;
		justify-content: space-between;
		min-height: 20px;
	}
	.rarity-info {
		display: flex;
		align-items: center;
		gap: 12px;
	}
	.rarity-tag {
		font-size: 9px;
		font-weight: 800;
		padding: 2px 8px;
		border-radius: 4px;
		text-transform: uppercase;
		letter-spacing: 0.8px;
		font-variant-numeric: tabular-nums;
	}
	.rarity-progress-track {
		width: 100px;
		height: 4px;
		background: var(--surface-2);
		border-radius: 3px;
		overflow: hidden;
	}
	.rarity-progress-fill {
		height: 100%;
		border-radius: 3px;
	}
	.ach-progress-info {
		display: flex;
		align-items: center;
		gap: 8px;
		margin-right: 12px;
	}
	.ach-progress-tag {
		font-size: 9px;
		font-weight: 800;
		padding: 2px 7px;
		border-radius: 4px;
		background: color-mix(in srgb, var(--accent, #c8a96e) 16%, transparent);
		color: var(--accent, #c8a96e);
		letter-spacing: 0.4px;
		font-variant-numeric: tabular-nums;
	}
	.ach-progress-fill {
		height: 100%;
		border-radius: 3px;
		background: var(--accent, #c8a96e);
	}

	.ach-actions {
		display: flex;
		gap: 8px;
		opacity: 0;
		transition: opacity 0.2s;
		position: absolute;
		right: 16px;
		bottom: 10px;
	}
	.ach-card:hover .ach-actions,
	.ach-card:focus-within .ach-actions {
		opacity: 1;
	}
	.action-btn {
		background: var(--surface-3);
		border: 1px solid var(--border-soft);
		border-radius: 6px;
		padding: 4px 10px;
		font-size: 10px;
		color: var(--text-secondary);
		font-weight: 700;
		cursor: pointer;
		display: flex;
		align-items: center;
		gap: 6px;
		transition: all 0.2s;
		backdrop-filter: blur(4px);
	}
	.action-btn:hover {
		background: var(--surface-hover);
		color: var(--text-primary);
		border-color: var(--border-specular);
	}
	.action-btn.is-copied {
		background: rgba(61, 220, 132, 0.2);
		color: #3ddc84;
		border-color: rgba(61, 220, 132, 0.4);
	}

	.ach-card.jump-flash {
		animation: achJumpFlash 1.8s ease;
	}
	@keyframes achJumpFlash {
		0% {
			transform: scale(1.015);
			background: color-mix(in srgb, var(--accent) 18%, var(--surface-1));
			border-color: var(--accent);
			box-shadow: 0 0 0 2px color-mix(in srgb, var(--accent) 35%, transparent);
		}
		100% {
			transform: scale(1);
			background: var(--surface-1);
			box-shadow: var(--shadow-card);
		}
	}

	.is-secret-hidden {
		opacity: 0.3;
		filter: blur(2px);
	}

	.export-toast {
		position: fixed;
		right: 24px;
		bottom: 24px;
		z-index: 250;
		max-width: 420px;
		display: flex;
		align-items: center;
		gap: 10px;
		padding: 12px 14px;
		border-radius: 12px;
		background: var(--surface-1);
		border: 1px solid rgba(61, 220, 132, 0.4);
		color: var(--text-primary);
		box-shadow: var(--shadow-elevated);
		backdrop-filter: blur(14px);
		font-size: 12px;
		font-weight: 600;
	}

	.export-toast svg:first-child {
		color: #3ddc84;
		flex-shrink: 0;
	}

	.export-toast.is-error {
		border-color: rgba(255, 77, 77, 0.45);
	}

	.export-toast.is-error svg:first-child {
		color: #ff4d4d;
	}

	.export-toast-msg {
		flex: 1;
		min-width: 0;
		overflow: hidden;
		text-overflow: ellipsis;
		white-space: nowrap;
	}

	.export-toast-close {
		display: inline-flex;
		align-items: center;
		justify-content: center;
		background: transparent;
		border: none;
		color: var(--text-muted);
		cursor: pointer;
		padding: 2px;
		border-radius: 6px;
		transition:
			background 0.15s ease,
			color 0.15s ease;
	}

	.export-toast-close:hover {
		background: var(--surface-hover);
		color: var(--text-primary);
	}

	.progress-card.has-dual-gauges {
		align-items: center;
	}

	.set-tabs {
		display: flex;
		align-items: center;
		gap: 6px;
		flex-wrap: wrap;
		max-height: 112px;
		overflow-y: auto;
		padding: 2px 0;
		scrollbar-width: thin;
	}

	.set-tab {
		display: inline-flex;
		align-items: center;
		gap: 7px;
		padding: 6px 12px;
		border-radius: 9px;
		border: 1px solid var(--border-soft);
		background: var(--surface-2);
		color: var(--text-secondary);
		font-size: 12px;
		font-weight: 600;
		cursor: pointer;
		max-width: 260px;
		transition:
			background 0.18s ease,
			border-color 0.18s ease,
			color 0.18s ease;
	}

	.set-tab:hover:not(.active) {
		background: var(--surface-hover);
		border-color: var(--border-specular);
		color: var(--text-primary);
	}

	.set-tab.active {
		background: var(--accent, #0066cc);
		border-color: var(--accent, #0066cc);
		color: var(--accent-text);
	}

	.set-tab-name {
		white-space: nowrap;
		overflow: hidden;
		text-overflow: ellipsis;
	}

	.set-tab-dlc {
		font-size: 9px;
		font-weight: 800;
		text-transform: uppercase;
		letter-spacing: 0.6px;
		padding: 1px 6px;
		border-radius: 999px;
		background: color-mix(in srgb, var(--accent, #c8a96e) 18%, transparent);
		color: var(--accent, #c8a96e);
		flex-shrink: 0;
	}

	.set-tab.active .set-tab-dlc {
		background: color-mix(in srgb, var(--accent-text) 18%, transparent);
		color: var(--accent-text);
	}

	.set-tab-count {
		font-size: 11px;
		font-weight: 700;
		padding: 1px 7px;
		border-radius: 999px;
		background: color-mix(in srgb, var(--surface-1) 75%, transparent);
		color: var(--text-muted);
		flex-shrink: 0;
	}

	.set-tab.active .set-tab-count {
		background: color-mix(in srgb, var(--accent-text) 18%, transparent);
		color: var(--accent-text);
	}

	.ach-sections {
		display: flex;
		flex-direction: column;
		gap: 28px;
	}

	.ach-set-section {
		display: flex;
		flex-direction: column;
		gap: 14px;
	}

	.ach-set-header {
		display: flex;
		align-items: center;
		justify-content: space-between;
		gap: 16px;
		flex-wrap: wrap;
		padding: 10px 14px;
		border-radius: 12px;
		background: color-mix(in srgb, var(--surface-1) 82%, transparent);
		border: 1px solid var(--border-soft);
	}

	.ach-set-title-group {
		display: flex;
		align-items: center;
		gap: 8px;
		min-width: 0;
	}

	.ach-set-title {
		margin: 0;
		font-size: 14px;
		font-weight: 800;
		color: var(--text-primary);
		letter-spacing: -0.1px;
	}

	.set-badge {
		font-size: 9px;
		font-weight: 800;
		text-transform: uppercase;
		letter-spacing: 0.7px;
		padding: 2px 7px;
		border-radius: 999px;
		flex-shrink: 0;
	}

	.set-badge.dlc {
		background: color-mix(in srgb, var(--accent, #c8a96e) 16%, transparent);
		border: 1px solid color-mix(in srgb, var(--accent, #c8a96e) 35%, transparent);
		color: var(--accent, #c8a96e);
	}

	.set-badge.archived {
		background: var(--surface-2);
		border: 1px solid var(--border-soft);
		color: var(--text-muted);
	}

	.ach-set-progress {
		display: flex;
		align-items: center;
		gap: 12px;
		flex-shrink: 0;
	}

	.ach-set-stats {
		font-size: 12px;
		font-weight: 700;
		color: var(--text-secondary);
	}

	.ach-set-bar-track {
		width: 110px;
		height: 6px;
		border-radius: 999px;
		background: var(--surface-2);
		overflow: hidden;
	}

	.ach-set-bar-fill {
		height: 100%;
		border-radius: 999px;
		background: var(--accent, #c8a96e);
		transition: width 0.5s cubic-bezier(0.19, 1, 0.22, 1);
	}

	.ach-set-bar-fill.complete {
		background: var(--completed, #4caf6e);
	}
</style>
