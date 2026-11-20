<script lang="ts">
	// src/lib/GamePage.svelte
	import { onMount, tick } from 'svelte';
	import { get } from 'svelte/store';
	import { listen } from '@tauri-apps/api/event';
	import { invoke } from '@tauri-apps/api/core';
	import { goto } from '$app/navigation';
	import { resolve } from '$app/paths';
	import { open } from '@tauri-apps/plugin-shell';
	import { SvelteMap, SvelteSet } from 'svelte/reactivity';
	import {
		loadAchievements,
		computeGameProgress,
		ACHIEVEMENT_TAG_KEYS,
		normalizeAchievementTags,
		type AchievementTagKey,
		type Game,
		type Achievement,
		games
	} from '$lib/stores/Games.js';
	import { searchQuery, achievementJumpIntent, settingsOpen } from '$lib/stores/ui.js';
	import { i18n } from '$lib/stores/i18n.js';
	import {
		exportGame,
		openProfilesFolder,
		exportSummaryPdf,
		selectedGameId
	} from '$lib/stores/selectedGame.js';
	import GameSidebar from '$lib/game/GameSidebar.svelte';
	import GameAchievementRow from '$lib/game/GameAchievementRow.svelte';
	import GameProgressMap from '$lib/game/GameProgressMap.svelte';

	let { game }: { game: Game | null } = $props();

	const TAG_I18N_KEYS: Record<
		AchievementTagKey,
		'game.tag.missable' | 'game.tag.multi' | 'game.tag.ngplus' | 'game.tag.buggy'
	> = {
		missable: 'game.tag.missable',
		multi: 'game.tag.multi',
		ngplus: 'game.tag.ngplus',
		buggy: 'game.tag.buggy'
	};

	let tagLabels = $derived<Record<AchievementTagKey, string>>({
		missable: $i18n.t(TAG_I18N_KEYS.missable),
		multi: $i18n.t(TAG_I18N_KEYS.multi),
		ngplus: $i18n.t(TAG_I18N_KEYS.ngplus),
		buggy: $i18n.t(TAG_I18N_KEYS.buggy)
	});

	type AchievementsUpdatedPayload = { game_id: string; achievements: Achievement[] };
	type ExportNotice = { message: string; type: 'success' | 'error' };

	let achievements = $state<Achievement[]>([]);
	let activeView = $state<'achievements' | 'map'>('achievements');
	let activeSetId = $state<string>('all');
	let filter = $state<'all' | 'unlocked' | 'locked'>('all');
	let search = $state('');
	let sort = $state<'date' | 'rarity' | 'name'>('date');
	let sortOrder = $state<'asc' | 'desc'>('desc');
	let revealed = $state(false);
	let loading = $state(true);
	let loadedGameId = $state<string | null>(null);
	let loadRequestToken = 0;

	let editingAchKey = $state<string | null>(null);
	let savingAchEdit = $state(false);
	let editingMetaAchKey = $state<string | null>(null);
	let savingMetaNote = $state(false);
	let activeAchTagFilter = $state<'all' | AchievementTagKey>('all');
	let markersMenuOpen = $state(false);
	let openRowMenuKey = $state<string | null>(null);

	let copiedKey = $state<string | null>(null);
	let copiedTimer: ReturnType<typeof setTimeout> | null = null;
	let pinnedKeys = $state<string[]>([]);
	let exportNotice = $state<ExportNotice | null>(null);
	let exportNoticeTimer: ReturnType<typeof setTimeout> | null = null;
	let jumpedToken = $state<number | null>(null);
	let flashAchievementKey = $state('');
	let flashTimer: ReturnType<typeof setTimeout> | null = null;
	const rowRefs = new SvelteMap<string, HTMLDivElement>();

	$effect(() => {
		if (game?.id) selectedGameId.set(game.id);
	});

	let lastInitializedGameId: string | null = null;
	$effect(() => {
		const gid = game?.id ?? null;
		if (gid === lastInitializedGameId) return;
		lastInitializedGameId = gid;
		editingAchKey = null;
		editingMetaAchKey = null;
		activeAchTagFilter = 'all';
		markersMenuOpen = false;
		openRowMenuKey = null;
		revealed = false;
		if (!gid) {
			pinnedKeys = [];
			return;
		}
		void invoke<string[]>('get_pinned_achievements', { gameId: gid })
			.then((pins) => {
				if (game?.id === gid) pinnedKeys = pins ?? [];
			})
			.catch(() => {});
	});

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

	let currentIndex = $derived($games.findIndex((g) => g.id === game?.id));
	let prevGame = $derived(currentIndex > 0 ? $games[currentIndex - 1] : null);
	let nextGame = $derived(
		currentIndex >= 0 && currentIndex < $games.length - 1 ? $games[currentIndex + 1] : null
	);
	let pinnedSet = $derived(new SvelteSet(pinnedKeys.map((k) => k.trim().toLowerCase())));

	function goToGame(id: string) {
		void goto(resolve('/(app)/games/[id]', { id }));
	}

	function isAchievementUnlocked(ach: Achievement): boolean {
		return ach.unlocked || Boolean(ach.unlocked_time);
	}

	function isAchievementMasked(ach: Achievement, isRevealed: boolean): boolean {
		return (ach.hidden ?? false) && !isAchievementUnlocked(ach) && !isRevealed;
	}

	function toggleRevealed() {
		revealed = !revealed;
		if (!revealed && editingMetaAchKey) {
			const edited = merged.find((a) => a.key === editingMetaAchKey);
			if (!edited || isAchievementMasked(edited, false)) editingMetaAchKey = null;
		}
	}

	onMount(() => {
		let destroyed = false;
		let unlisten: (() => void) | undefined;

		const handleKeydown = (e: KeyboardEvent) => {
			if (get(settingsOpen)) return;
			if (e.key === 'Escape' && (markersMenuOpen || openRowMenuKey)) {
				markersMenuOpen = false;
				openRowMenuKey = null;
				return;
			}
			if (e.altKey || e.ctrlKey || e.metaKey || e.shiftKey) return;
			if (
				e.target instanceof HTMLElement &&
				(e.target.tagName === 'INPUT' ||
					e.target.tagName === 'TEXTAREA' ||
					e.target.tagName === 'SELECT' ||
					e.target.isContentEditable)
			) {
				return;
			}
			if (e.key === 'ArrowLeft' && prevGame) goToGame(prevGame.id);
			else if (e.key === 'ArrowRight' && nextGame) goToGame(nextGame.id);
		};

		const handlePointerDown = (e: PointerEvent) => {
			const target = e.target instanceof Element ? e.target : null;
			if (markersMenuOpen && !target?.closest('[data-markers-popover]')) markersMenuOpen = false;
			if (openRowMenuKey && !target?.closest('[data-row-menu-container]')) openRowMenuKey = null;
		};

		window.addEventListener('keydown', handleKeydown);
		window.addEventListener('pointerdown', handlePointerDown);

		void listen<AchievementsUpdatedPayload>('achievements-updated', (event) => {
			const payload = event.payload;
			if (game && payload?.game_id === game.id && Array.isArray(payload.achievements)) {
				achievements = payload.achievements;
			}
		}).then((fn) => {
			if (destroyed) fn();
			else unlisten = fn;
		});

		return () => {
			destroyed = true;
			unlisten?.();
			window.removeEventListener('keydown', handleKeydown);
			window.removeEventListener('pointerdown', handlePointerDown);
			if (flashTimer) clearTimeout(flashTimer);
			if (exportNoticeTimer) clearTimeout(exportNoticeTimer);
			if (copiedTimer) clearTimeout(copiedTimer);
		};
	});

	async function saveAchievementEdit(achKey: string, unlocked: boolean, dateTime: string) {
		if (!game || savingAchEdit) return;
		const gid = game.id;
		savingAchEdit = true;
		try {
			let unlockedTime: number | null = null;
			if (unlocked) {
				const parsedMs = dateTime ? new Date(dateTime).getTime() : Date.now();
				unlockedTime = Number.isFinite(parsedMs)
					? Math.floor(parsedMs / 1000)
					: Math.floor(Date.now() / 1000);
			}
			const updated = await invoke<Achievement[]>('set_achievement_state', {
				gameId: gid,
				achKey,
				unlocked,
				unlockedTime
			});
			if (updated && updated.length > 0) {
				if (game?.id === gid) achievements = updated;
				games.update((list) =>
					list.map((g) => {
						if (g.id !== gid) return g;
						const prevByKey = new SvelteMap<string, Achievement>(
							(g.achievements ?? []).map((prev) => [prev.key.trim().toLowerCase(), prev])
						);
						return {
							...g,
							achievements: updated.map((u) => {
								const prev = prevByKey.get(u.key.trim().toLowerCase());
								return prev
									? {
											...u,
											groupid: u.groupid ?? prev.groupid,
											archived: u.archived ?? prev.archived,
											note: u.note !== undefined ? u.note : prev.note,
											tags: u.tags !== undefined ? u.tags : prev.tags
										}
									: u;
							})
						};
					})
				);
			} else if (game?.id === gid) {
				const norm = achKey.trim().toLowerCase();
				achievements = achievements.map((a) =>
					a.key.trim().toLowerCase() === norm ? { ...a, unlocked, unlocked_time: unlockedTime } : a
				);
			}
			editingAchKey = null;
		} catch (e) {
			console.error('Failed to update achievement state:', e);
		} finally {
			savingAchEdit = false;
		}
	}

	function updateAchievementMetaLocal(
		gameId: string,
		achKey: string,
		patch: { note?: string | null; tags?: string[] }
	) {
		const norm = achKey.trim().toLowerCase();
		const patchItem = (item: Achievement): Achievement =>
			item.key.trim().toLowerCase() === norm ? { ...item, ...patch } : item;
		if (game?.id === gameId) achievements = achievements.map(patchItem);
		games.update((list) =>
			list.map((g) =>
				g.id === gameId ? { ...g, achievements: (g.achievements ?? []).map(patchItem) } : g
			)
		);
	}

	async function saveAchievementNote(achKey: string, rawNote: string) {
		if (!game || savingMetaNote) return;
		const gid = game.id;
		const trimmed = rawNote.trim();
		savingMetaNote = true;
		try {
			const saved = await invoke<string | null>('set_achievement_note', {
				gameId: gid,
				achKey,
				note: trimmed.length > 0 ? trimmed : null
			});
			updateAchievementMetaLocal(gid, achKey, { note: saved });
			if (editingMetaAchKey === achKey) editingMetaAchKey = null;
		} catch (e) {
			console.error('Failed to save achievement note:', e);
		} finally {
			savingMetaNote = false;
		}
	}

	async function clearAchievementNote(achKey: string) {
		if (!game || savingMetaNote) return;
		const gid = game.id;
		savingMetaNote = true;
		try {
			const saved = await invoke<string | null>('set_achievement_note', {
				gameId: gid,
				achKey,
				note: null
			});
			updateAchievementMetaLocal(gid, achKey, { note: saved });
		} catch (e) {
			console.error('Failed to clear achievement note:', e);
		} finally {
			savingMetaNote = false;
		}
	}

	async function toggleAchievementTag(ach: Achievement, tagKey: AchievementTagKey) {
		if (!game) return;
		const gid = game.id;
		const prevTags = normalizeAchievementTags(ach.tags);
		const nextTags = prevTags.includes(tagKey)
			? prevTags.filter((t) => t !== tagKey)
			: [...prevTags, tagKey];
		updateAchievementMetaLocal(gid, ach.key, { tags: nextTags });
		try {
			const saved = await invoke<string[]>('set_achievement_tags', {
				gameId: gid,
				achKey: ach.key,
				tags: nextTags
			});
			updateAchievementMetaLocal(gid, ach.key, { tags: saved });
		} catch (e) {
			console.error('Failed to update achievement tags:', e);
			updateAchievementMetaLocal(gid, ach.key, { tags: prevTags });
		}
	}

	async function togglePin(key: string) {
		if (!game) return;
		const gid = game.id;
		try {
			const pins = await invoke<string[]>('toggle_pinned_achievement', {
				gameId: gid,
				achKey: key
			});
			if (game?.id === gid) pinnedKeys = pins;
		} catch (e) {
			console.error('Failed to toggle pin:', e);
		}
	}

	function copyToClipboard(text: string, key: string) {
		void navigator.clipboard.writeText(text);
		copiedKey = key;
		if (copiedTimer) clearTimeout(copiedTimer);
		copiedTimer = setTimeout(() => {
			if (copiedKey === key) copiedKey = null;
			copiedTimer = null;
		}, 2000);
	}

	function showExportNotice(message: string, type: 'success' | 'error' = 'success') {
		exportNotice = { message, type };
		if (exportNoticeTimer) clearTimeout(exportNoticeTimer);
		exportNoticeTimer = setTimeout(() => {
			exportNotice = null;
			exportNoticeTimer = null;
		}, 4000);
	}

	async function handleExportGame() {
		if (!game) return;
		try {
			showExportNotice(await exportGame(game), 'success');
		} catch {
			showExportNotice(get(i18n).t('game.exportFailed'), 'error');
		}
	}

	async function handleExportGamePdf() {
		if (!game) return;
		try {
			const t = get(i18n).t;
			const unlocked = progress.unlockedAll;
			const total = progress.totalAll;
			let content = `${t('game.pdf.game')}: ${game.name}\n${t('game.pdf.id')}: ${game.id}\n`;
			content += `${t('game.pdf.source')}: ${typeof game.source === 'object' ? game.source.type : game.source}\n`;
			content += `${t('game.progressLabel')}: ${unlocked}/${total} (${total > 0 ? globalPctDisplay : 0}%)\n\n`;
			content += `${t('game.pdf.unlockedAchievements')}\n--------------------------\n`;
			publicAchievements
				.filter((a) => isAchievementUnlocked(a))
				.forEach((a) => {
					content += `[X] ${a.name}\n`;
				});
			showExportNotice(await exportSummaryPdf(game.name, content), 'success');
		} catch (err) {
			console.error('PDF Export Error:', err);
			showExportNotice(String(err), 'error');
		}
	}

	let merged = $derived.by((): Achievement[] => {
		if (!game) return [];
		const stateMap = new SvelteMap<string, Achievement>(
			achievements.map((a) => [a.key.trim().toLowerCase(), a])
		);
		const seenKeys = new SvelteSet<string>();
		const list: Achievement[] = (game.achievements ?? []).map((s: Achievement) => {
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
				archived: Boolean(s.archived || live?.archived),
				note: live ? (live.note !== undefined ? live.note : (s.note ?? null)) : (s.note ?? null),
				tags: live ? (live.tags !== undefined ? live.tags : (s.tags ?? [])) : (s.tags ?? [])
			};
		});
		for (const live of achievements) {
			const normKey = live.key.trim().toLowerCase();
			if (!seenKeys.has(normKey)) {
				seenKeys.add(normKey);
				list.push({
					...live,
					groupid: live.groupid ?? 0,
					archived: live.archived ?? false,
					note: live.note ?? null,
					tags: live.tags ?? []
				});
			}
		}
		return list;
	});

	let achTagStats = $derived.by(() => {
		let missableRemaining = 0;
		const total: Record<AchievementTagKey, number> = {
			missable: 0,
			multi: 0,
			ngplus: 0,
			buggy: 0
		};
		for (const ach of merged) {
			if (isAchievementMasked(ach, revealed)) continue;
			const tags = normalizeAchievementTags(ach.tags);
			if (tags.length === 0) continue;
			const unlocked = isAchievementUnlocked(ach);
			for (const tag of tags) {
				total[tag] += 1;
				if (tag === 'missable' && !unlocked) missableRemaining += 1;
			}
		}
		return { missableRemaining, total };
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

	const formatPct = (unlocked: number, total: number, raw: number) =>
		total <= 0 ? 0 : unlocked === total ? 100 : Math.min(99, Math.max(0, Math.round(raw)));

	let platinumPctDisplay = $derived(
		formatPct(progress.unlockedBase, progress.totalBase, progress.platinumPct)
	);
	let globalPctDisplay = $derived(
		formatPct(progress.unlockedAll, progress.totalAll, progress.globalPct)
	);
	let globalAchievementHighlight = $derived($searchQuery.trim().toLowerCase());

	function filterAndSortAchievements(list: Achievement[]): Achievement[] {
		const secretTitle = $i18n.t('game.secretAchievement');
		let result = list;
		if (filter === 'unlocked') result = result.filter((a) => isAchievementUnlocked(a));
		else if (filter === 'locked') result = result.filter((a) => !isAchievementUnlocked(a));

		const tagFilter = activeAchTagFilter;
		if (tagFilter !== 'all') {
			result = result.filter(
				(a) =>
					!isAchievementMasked(a, revealed) && normalizeAchievementTags(a.tags).includes(tagFilter)
			);
		}

		const localQuery = search.trim().toLowerCase();
		if (localQuery) {
			const secretLower = secretTitle.toLowerCase();
			result = result.filter((a) =>
				isAchievementMasked(a, revealed)
					? secretLower.includes(localQuery)
					: a.name.toLowerCase().includes(localQuery) ||
						(a.desc ?? '').toLowerCase().includes(localQuery) ||
						(a.note ?? '').toLowerCase().includes(localQuery)
			);
		}

		const sorted = [...result];
		const mult = sortOrder === 'asc' ? 1 : -1;
		if (sort === 'date') {
			sorted.sort((a, b) => {
				const aU = isAchievementUnlocked(a);
				const bU = isAchievementUnlocked(b);
				if (aU && bU) return ((a.unlocked_time ?? 0) - (b.unlocked_time ?? 0)) * mult;
				return aU ? -1 : bU ? 1 : 0;
			});
		} else if (sort === 'rarity') {
			sorted.sort(
				(a, b) =>
					((parseFloat(a.completionpercentage) || 100) -
						(parseFloat(b.completionpercentage) || 100)) *
					mult
			);
		} else {
			sorted.sort((a, b) => {
				const nA = isAchievementMasked(a, revealed) ? secretTitle : a.name;
				const nB = isAchievementMasked(b, revealed) ? secretTitle : b.name;
				return nA.localeCompare(nB) * mult;
			});
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
			displayPct: formatPct(section.unlocked, section.total, section.pct),
			filteredAchievements: filterAndSortAchievements(section.achievements)
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
	let hasActiveFilter = $derived(
		filter !== 'all' ||
			search.trim().length > 0 ||
			activeAchTagFilter !== 'all' ||
			activeSetId !== 'all'
	);

	let mapAchievements = $derived.by(() => {
		const secretLabel = $i18n.t('game.secretAchievement');
		const unlocked = publicAchievements
			.filter((a) => isAchievementUnlocked(a))
			.sort((a, b) => (a.unlocked_time ?? 0) - (b.unlocked_time ?? 0));
		const locked = publicAchievements
			.filter((a) => !isAchievementUnlocked(a))
			.sort((a, b) => {
				const rA = parseFloat(a.completionpercentage);
				const rB = parseFloat(b.completionpercentage);
				const nA = Number.isFinite(rA) ? rA : Number.POSITIVE_INFINITY;
				const nB = Number.isFinite(rB) ? rB : Number.POSITIVE_INFINITY;
				const tA = isAchievementMasked(a, revealed) ? secretLabel : a.name;
				const tB = isAchievementMasked(b, revealed) ? secretLabel : b.name;
				return nB - nA || tA.localeCompare(tB);
			});
		return [...unlocked, ...locked];
	});

	const normKey = (key: string) => (key || '').trim().toLowerCase();

	function trackAchievementRow(node: HTMLDivElement, key: string) {
		const normalized = normKey(key);
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

	const delay = (ms: number) => new Promise<void>((r) => setTimeout(r, ms));

	async function scrollToAchievementKey(rawKey: string) {
		clearAllFilters();
		activeView = 'achievements';
		const normalized = normKey(rawKey);
		let target: HTMLDivElement | undefined;
		for (let i = 0; i < 20; i += 1) {
			await tick();
			target = rowRefs.get(normalized);
			if (target) break;
			await delay(50);
		}
		if (target) {
			target.scrollIntoView({ behavior: 'smooth', block: 'center' });
			triggerAchievementFlash(normalized);
		}
	}

	$effect(() => {
		const intent = $achievementJumpIntent;
		if (!intent || !game || intent.gameId !== game.id || jumpedToken === intent.token) return;
		jumpedToken = intent.token;
		void scrollToAchievementKey(intent.achievementKey).finally(() =>
			achievementJumpIntent.set(null)
		);
	});

	function clearAllFilters() {
		activeSetId = 'all';
		filter = 'all';
		activeAchTagFilter = 'all';
		search = '';
	}
</script>

{#snippet achRow(ach: Achievement, isArchived: boolean)}
	<GameAchievementRow
		{ach}
		{isArchived}
		{revealed}
		pinned={pinnedSet.has(normKey(ach.key))}
		copied={copiedKey === ach.key}
		menuOpen={openRowMenuKey === ach.key}
		editingAch={editingAchKey === ach.key}
		editingMeta={editingMetaAchKey === ach.key}
		{savingAchEdit}
		{savingMetaNote}
		globalHighlight={globalAchievementHighlight}
		isFlashing={flashAchievementKey === normKey(ach.key)}
		{tagLabels}
		trackRow={trackAchievementRow}
		onTogglePin={(k) => void togglePin(k)}
		onToggleMenu={(k) => (openRowMenuKey = openRowMenuKey === k ? null : k)}
		onCloseMenu={() => (openRowMenuKey = null)}
		onOpenAchEditor={(a) => {
			editingMetaAchKey = null;
			editingAchKey = editingAchKey === a.key ? null : a.key;
		}}
		onCloseAchEditor={() => (editingAchKey = null)}
		onSaveAchEdit={(k, unlocked, dt) => void saveAchievementEdit(k, unlocked, dt)}
		onOpenMetaEditor={(a) => {
			editingAchKey = null;
			editingMetaAchKey = editingMetaAchKey === a.key ? null : a.key;
		}}
		onCloseMetaEditor={() => (editingMetaAchKey = null)}
		onSaveNote={(k, note) => void saveAchievementNote(k, note)}
		onClearNote={(k) => void clearAchievementNote(k)}
		onToggleTag={(a, t) => void toggleAchievementTag(a, t)}
		onCopyName={copyToClipboard}
		onFindGuide={(name) =>
			game &&
			void open(
				`https://www.google.com/search?q=${encodeURIComponent(`${game.name} ${name} achievement guide`)}`
			)}
	/>
{/snippet}

{#if !game}
	<div class="not-found">{$i18n.t('game.notFound')}</div>
{:else}
	<div class="game-page">
		<div class="game-split">
			<GameSidebar
				{game}
				gameTitle={game.name?.trim() || `ID ${game.id}`}
				{currentIndex}
				totalGames={$games.length}
				{prevGame}
				{nextGame}
				{progress}
				{isPlatinum}
				{platinumPctDisplay}
				{globalPctDisplay}
				missableRemainingCount={achTagStats.missableRemaining}
				missableFilterActive={activeAchTagFilter === 'missable'}
				onGoToGame={goToGame}
				onToggleMissableFilter={() => {
					activeView = 'achievements';
					activeAchTagFilter = activeAchTagFilter === 'missable' ? 'all' : 'missable';
				}}
				onExportJson={() => void handleExportGame()}
				onExportPdf={() => void handleExportGamePdf()}
				onOpenFolder={() => void openProfilesFolder()}
			/>

			<section class="right-pane">
				<header class="pane-header">
					<div class="pane-header-top">
						<div class="tabs-cluster">
							<div class="view-tabs" role="tablist" aria-label={$i18n.t('game.achievementsTitle')}>
								<button
									id="view-tab-achievements"
									type="button"
									role="tab"
									aria-selected={activeView === 'achievements'}
									aria-controls="game-view-panel"
									class="underline-tab"
									class:active={activeView === 'achievements'}
									onclick={() => (activeView = 'achievements')}
								>
									{$i18n.t('game.achievementsTitle')}
								</button>
								<button
									id="view-tab-map"
									type="button"
									role="tab"
									aria-selected={activeView === 'map'}
									aria-controls="game-view-panel"
									class="underline-tab"
									class:active={activeView === 'map'}
									onclick={() => (activeView = 'map')}
								>
									{$i18n.t('game.shareCard')}
								</button>
							</div>

							{#if hasSets && activeView === 'achievements'}
								<span class="tabs-sep" aria-hidden="true"></span>
								<div class="set-tabs" role="tablist" aria-label={$i18n.t('game.allSets')}>
									<button
										type="button"
										role="tab"
										aria-selected={activeSetId === 'all'}
										class="underline-tab set-tab"
										class:active={activeSetId === 'all'}
										onclick={() => (activeSetId = 'all')}
									>
										<span>{$i18n.t('game.allSets')}</span>
										<span class="set-count tabular-nums"
											>{progress.unlockedAll}/{progress.totalAll}</span
										>
									</button>
									{#each progress.sections as section (section.id)}
										<button
											type="button"
											role="tab"
											aria-selected={activeSetId === section.id}
											class="underline-tab set-tab"
											class:active={activeSetId === section.id}
											onclick={() => (activeSetId = section.id)}
										>
											{#if section.isDlc}
												<span class="set-dlc-label">{$i18n.t('game.dlcBadge')}</span>
											{/if}
											<span class="set-name">{section.title}</span>
											<span class="set-count tabular-nums">{section.unlocked}/{section.total}</span>
										</button>
									{/each}
								</div>
							{/if}
						</div>

						<button
							type="button"
							class="secrets-btn"
							class:active={revealed}
							aria-pressed={revealed}
							onclick={toggleRevealed}
						>
							{#if revealed}
								<svg
									width="13"
									height="13"
									viewBox="0 0 24 24"
									fill="none"
									stroke="currentColor"
									stroke-width="2"
									stroke-linecap="round"
									stroke-linejoin="round"
									aria-hidden="true"
								>
									<path d="M1 12s4-8 11-8 11 8 11 8-4 8-11 8-11-8-11-8z" />
									<circle cx="12" cy="12" r="3" />
								</svg>
							{:else}
								<svg
									width="13"
									height="13"
									viewBox="0 0 24 24"
									fill="none"
									stroke="currentColor"
									stroke-width="2"
									stroke-linecap="round"
									stroke-linejoin="round"
									aria-hidden="true"
								>
									<path
										d="M17.94 17.94A10.07 10.07 0 0 1 12 20c-7 0-11-8-11-8a18.45 18.45 0 0 1 5.06-5.94M9.9 4.24A9.12 9.12 0 0 1 12 4c7 0 11 8 11 8a18.5 18.5 0 0 1-2.16 3.19m-6.72-1.07a3 3 0 1 1-4.24-4.24"
									/>
									<line x1="1" y1="1" x2="23" y2="23" />
								</svg>
							{/if}
							<span>{$i18n.t(revealed ? 'game.secretsVisible' : 'game.secrets')}</span>
						</button>
					</div>

					{#if activeView === 'achievements'}
						<div class="pane-controls">
							<div class="filter-tabs" role="group" aria-label={$i18n.t('games.filter.status')}>
								<button
									type="button"
									class="filter-btn"
									class:active={filter === 'all'}
									aria-pressed={filter === 'all'}
									onclick={() => (filter = 'all')}>{$i18n.t('game.filterAll')}</button
								>
								<button
									type="button"
									class="filter-btn"
									class:active={filter === 'unlocked'}
									aria-pressed={filter === 'unlocked'}
									onclick={() => (filter = 'unlocked')}>{$i18n.t('game.unlocked')}</button
								>
								<button
									type="button"
									class="filter-btn"
									class:active={filter === 'locked'}
									aria-pressed={filter === 'locked'}
									onclick={() => (filter = 'locked')}>{$i18n.t('game.locked')}</button
								>
							</div>

							<div class="search-wrap">
								<svg
									width="13"
									height="13"
									viewBox="0 0 24 24"
									fill="none"
									stroke="currentColor"
									stroke-width="2.2"
									aria-hidden="true"
								>
									<circle cx="11" cy="11" r="8" />
									<line x1="21" y1="21" x2="16.65" y2="16.65" />
								</svg>
								<input
									class="search-input"
									type="search"
									placeholder={$i18n.t('game.searchAchievement')}
									aria-label={$i18n.t('game.searchAchievement')}
									bind:value={search}
								/>
								{#if search}
									<button
										type="button"
										class="search-clear"
										aria-label={$i18n.t('button.close')}
										onclick={() => (search = '')}>×</button
									>
								{/if}
								<span class="search-count tabular-nums">{filteredCount} / {visibleTotalCount}</span>
							</div>

							<div class="sort-group">
								<select
									class="sort-select"
									aria-label={$i18n.t('games.sort.label')}
									bind:value={sort}
								>
									<option value="date">{$i18n.t('game.sort.date')}</option>
									<option value="rarity">{$i18n.t('game.sort.rarity')}</option>
									<option value="name">{$i18n.t('game.sort.name')}</option>
								</select>

								<button
									type="button"
									class="order-btn"
									onclick={() => (sortOrder = sortOrder === 'asc' ? 'desc' : 'asc')}
									aria-label={sortOrder === 'asc'
										? $i18n.t('game.sort.asc')
										: $i18n.t('game.sort.desc')}
									title={sortOrder === 'asc' ? $i18n.t('game.sort.asc') : $i18n.t('game.sort.desc')}
								>
									{#if sortOrder === 'asc'}
										<svg
											width="13"
											height="13"
											viewBox="0 0 24 24"
											fill="none"
											stroke="currentColor"
											stroke-width="2.3"
											aria-hidden="true"
										>
											<path d="M12 19V5M5 12l7-7 7 7" />
										</svg>
									{:else}
										<svg
											width="13"
											height="13"
											viewBox="0 0 24 24"
											fill="none"
											stroke="currentColor"
											stroke-width="2.3"
											aria-hidden="true"
										>
											<path d="M12 5v14M5 12l7 7 7-7" />
										</svg>
									{/if}
								</button>
							</div>

							<div class="markers-wrap" data-markers-popover>
								<button
									type="button"
									class="markers-btn"
									class:active={activeAchTagFilter !== 'all'}
									aria-haspopup="true"
									aria-expanded={markersMenuOpen}
									onclick={() => (markersMenuOpen = !markersMenuOpen)}
								>
									<span
										>{activeAchTagFilter === 'all'
											? $i18n.t('game.tag.markers')
											: tagLabels[activeAchTagFilter]}</span
									>
									<svg
										width="11"
										height="11"
										viewBox="0 0 24 24"
										fill="none"
										stroke="currentColor"
										stroke-width="2.4"
										aria-hidden="true"
									>
										<polyline points="6 9 12 15 18 9" />
									</svg>
								</button>

								{#if markersMenuOpen}
									<div class="markers-popover" role="menu">
										{#each ACHIEVEMENT_TAG_KEYS as tagKey (tagKey)}
											{@const active = activeAchTagFilter === tagKey}
											<button
												type="button"
												role="menuitemradio"
												aria-checked={active}
												class="markers-item"
												class:active
												onclick={() => {
													activeAchTagFilter = active ? 'all' : tagKey;
													markersMenuOpen = false;
												}}
											>
												<span>{tagLabels[tagKey]}</span>
												<span class="markers-count tabular-nums">{achTagStats.total[tagKey]}</span>
											</button>
										{/each}
										{#if activeAchTagFilter !== 'all'}
											<div class="markers-sep" role="separator"></div>
											<button
												type="button"
												role="menuitem"
												class="markers-item clear"
												onclick={() => {
													activeAchTagFilter = 'all';
													markersMenuOpen = false;
												}}
											>
												<span>{$i18n.t('game.clearFilters')}</span>
											</button>
										{/if}
									</div>
								{/if}
							</div>
						</div>
					{/if}
				</header>

				<div
					id="game-view-panel"
					class="right-scroll"
					role="tabpanel"
					aria-labelledby={activeView === 'achievements' ? 'view-tab-achievements' : 'view-tab-map'}
				>
					{#if activeView === 'achievements'}
						{#if loading}
							<div class="state-msg">{$i18n.t('game.loading')}</div>
						{:else if filteredCount === 0}
							<div class="empty-state">
								<p>{$i18n.t('game.noAchievementFound')}</p>
								{#if hasActiveFilter}
									<button type="button" class="clear-filters-btn" onclick={clearAllFilters}>
										{$i18n.t('game.clearFilters')}
									</button>
								{/if}
							</div>
						{:else if hasSets}
							<div class="ach-sections">
								{#each visibleSections as section (section.id)}
									{#if section.filteredAchievements.length > 0}
										<section class="ach-set-section">
											<header class="ach-set-header">
												<div class="ach-set-title-group">
													<h2 class="ach-set-title">{section.title}</h2>
													{#if section.isDlc}
														<span class="set-meta-tag">{$i18n.t('game.dlcBadge')}</span>
													{/if}
													{#if section.archived}
														<span class="set-meta-tag">{$i18n.t('game.archived')}</span>
													{/if}
												</div>
												<div class="ach-set-progress">
													<span class="ach-set-stats tabular-nums"
														>{section.unlocked} / {section.total} ({section.displayPct}%)</span
													>
													<div class="ach-set-bar-track" aria-hidden="true">
														<div
															class="ach-set-bar-fill"
															class:complete={section.total > 0 &&
																section.unlocked === section.total}
															style:width="{section.pct}%"
														></div>
													</div>
												</div>
											</header>
											<div class="ach-list">
												{#each section.filteredAchievements as ach (ach.key)}
													{@render achRow(ach, Boolean(ach.archived || section.archived))}
												{/each}
											</div>
										</section>
									{/if}
								{/each}
							</div>
						{:else}
							<div class="ach-list">
								{#each flatFiltered as item (item.ach.key)}
									{@render achRow(item.ach, item.isArchived)}
								{/each}
							</div>
						{/if}
					{:else}
						<GameProgressMap
							{mapAchievements}
							{unlockedCount}
							{totalCount}
							{revealed}
							onSelectAchievement={(key) => void scrollToAchievementKey(key)}
						/>
					{/if}
				</div>
			</section>
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
						aria-hidden="true"
					>
						<circle cx="12" cy="12" r="10" />
						<line x1="12" y1="8" x2="12" y2="12" />
						<line x1="12" y1="16" x2="12.01" y2="16" />
					</svg>
				{:else}
					<svg
						width="15"
						height="15"
						viewBox="0 0 24 24"
						fill="none"
						stroke="currentColor"
						stroke-width="2.4"
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
						stroke-width="2.4"
						aria-hidden="true"
					>
						<line x1="18" y1="6" x2="6" y2="18" />
						<line x1="6" y1="6" x2="18" y2="18" />
					</svg>
				</button>
			</div>
		{/if}
	</div>
{/if}

<style>
	.not-found,
	.state-msg,
	.empty-state {
		display: flex;
		flex-direction: column;
		align-items: center;
		justify-content: center;
		gap: 12px;
		color: var(--text-muted);
		font-size: 13.5px;
		padding: 48px 20px;
	}
	.not-found {
		height: 100%;
	}
	.game-page {
		height: 100%;
		min-height: 0;
		min-width: 0;
		display: flex;
		flex-direction: column;
		overflow: hidden;
		position: relative;
	}
	.game-split {
		display: grid;
		grid-template-columns: 288px minmax(0, 1fr);
		height: 100%;
		min-height: 0;
		overflow: hidden;
	}
	.right-pane {
		display: flex;
		flex-direction: column;
		min-width: 0;
		min-height: 0;
		overflow: hidden;
	}
	.pane-header {
		position: sticky;
		top: 0;
		z-index: 20;
		background: var(--bg-panel);
		border-bottom: 1px solid var(--border-soft);
		padding: 0 24px;
		flex-shrink: 0;
	}
	.pane-header-top {
		display: flex;
		align-items: center;
		justify-content: space-between;
		gap: 12px;
		min-height: 46px;
	}
	.tabs-cluster {
		display: flex;
		align-items: center;
		gap: 12px;
		min-width: 0;
		flex: 1;
		overflow-x: auto;
		scrollbar-width: none;
	}
	.tabs-cluster::-webkit-scrollbar {
		display: none;
	}
	.view-tabs,
	.set-tabs {
		display: flex;
		align-items: center;
		gap: 14px;
		flex-shrink: 0;
	}
	.tabs-sep {
		width: 1px;
		height: 14px;
		background: var(--border-soft);
		flex-shrink: 0;
	}
	.underline-tab {
		display: inline-flex;
		align-items: center;
		gap: 6px;
		height: 46px;
		padding: 0 2px;
		background: transparent;
		border: none;
		border-bottom: 2px solid transparent;
		color: var(--text-muted);
		font-family: inherit;
		font-size: 13px;
		font-weight: 600;
		white-space: nowrap;
		cursor: pointer;
	}
	.underline-tab:hover {
		color: var(--text-primary);
	}
	.underline-tab.active {
		color: var(--text-primary);
		border-bottom-color: var(--accent);
	}
	.set-tab {
		font-size: 12px;
		max-width: 220px;
	}
	.set-name {
		overflow: hidden;
		text-overflow: ellipsis;
		white-space: nowrap;
	}
	.set-dlc-label {
		font-size: 9.5px;
		font-weight: 700;
		text-transform: uppercase;
		letter-spacing: 0.04em;
		color: var(--accent);
	}
	.set-count {
		font-size: 11px;
		color: var(--text-muted);
	}
	.secrets-btn {
		display: inline-flex;
		align-items: center;
		gap: 6px;
		padding: 5px 8px;
		background: transparent;
		border: none;
		border-radius: 6px;
		color: var(--text-muted);
		font-family: inherit;
		font-size: 12px;
		font-weight: 500;
		white-space: nowrap;
		cursor: pointer;
		flex-shrink: 0;
	}
	.secrets-btn:hover {
		color: var(--text-primary);
		background: var(--surface-hover);
	}
	.secrets-btn.active {
		color: var(--accent);
	}
	.pane-controls {
		display: flex;
		align-items: center;
		flex-wrap: wrap;
		gap: 10px;
		padding: 10px 0;
		border-top: 1px solid var(--border-soft);
	}
	.filter-tabs {
		display: inline-flex;
		align-items: center;
		gap: 2px;
		background: var(--surface-2);
		border-radius: 8px;
		padding: 2px;
	}
	.filter-btn {
		padding: 5px 10px;
		border-radius: 6px;
		border: none;
		background: transparent;
		color: var(--text-muted);
		font-family: inherit;
		font-size: 12px;
		font-weight: 500;
		white-space: nowrap;
		cursor: pointer;
	}
	.filter-btn:hover {
		color: var(--text-primary);
	}
	.filter-btn.active {
		background: var(--surface-1);
		color: var(--text-primary);
		box-shadow: var(--shadow-card);
	}
	.search-wrap {
		flex: 1 1 200px;
		min-width: 160px;
		height: 32px;
		display: flex;
		align-items: center;
		gap: 8px;
		padding: 0 10px;
		background: var(--surface-2);
		border: 1px solid var(--border-soft);
		border-radius: 8px;
		color: var(--text-muted);
	}
	.search-wrap:focus-within {
		border-color: var(--accent);
	}
	.search-input {
		flex: 1;
		min-width: 0;
		background: transparent;
		border: none;
		outline: none;
		color: var(--text-primary);
		font-family: inherit;
		font-size: 12.5px;
	}
	.search-input::placeholder {
		color: var(--text-muted);
	}
	.search-input::-webkit-search-cancel-button {
		display: none;
	}
	.search-clear {
		background: transparent;
		border: none;
		color: var(--text-muted);
		font-size: 14px;
		line-height: 1;
		cursor: pointer;
		padding: 0 2px;
	}
	.search-clear:hover {
		color: var(--text-primary);
	}
	.search-count {
		font-size: 11px;
		font-weight: 600;
		color: var(--text-muted);
		white-space: nowrap;
	}
	.sort-group {
		display: inline-flex;
		align-items: center;
		gap: 4px;
	}
	.sort-select,
	.order-btn,
	.markers-btn {
		height: 32px;
		background: var(--surface-2);
		border: 1px solid var(--border-soft);
		border-radius: 8px;
		font-family: inherit;
		font-size: 12px;
		cursor: pointer;
	}
	.sort-select {
		padding: 0 10px;
		color: var(--text-primary);
	}
	.sort-select option {
		background: var(--surface-1);
		color: var(--text-primary);
	}
	.order-btn,
	.markers-btn {
		padding: 0 10px;
		display: inline-flex;
		align-items: center;
		justify-content: center;
		gap: 6px;
		color: var(--text-secondary);
		font-weight: 500;
		white-space: nowrap;
	}
	.order-btn {
		width: 32px;
		padding: 0;
	}
	.order-btn:hover,
	.markers-btn:hover {
		background: var(--surface-hover);
		color: var(--text-primary);
	}
	.markers-btn.active {
		border-color: var(--accent);
		color: var(--accent);
	}
	.markers-wrap {
		position: relative;
	}
	.markers-popover {
		position: absolute;
		top: calc(100% + 6px);
		right: 0;
		min-width: 184px;
		padding: 4px;
		border-radius: 10px;
		background: var(--surface-1);
		border: 1px solid var(--border-specular);
		box-shadow: var(--shadow-elevated);
		display: flex;
		flex-direction: column;
		z-index: 40;
	}
	.markers-item {
		display: flex;
		align-items: center;
		justify-content: space-between;
		gap: 10px;
		width: 100%;
		padding: 6px 10px;
		border: none;
		border-radius: 6px;
		background: transparent;
		color: var(--text-secondary);
		font-family: inherit;
		font-size: 12px;
		font-weight: 500;
		cursor: pointer;
		text-align: left;
	}
	.markers-item:hover {
		background: var(--surface-hover);
		color: var(--text-primary);
	}
	.markers-item.active {
		color: var(--accent);
		font-weight: 600;
	}
	.markers-item.clear {
		color: var(--text-muted);
	}
	.markers-count {
		font-size: 11px;
		color: var(--text-muted);
	}
	.markers-sep {
		height: 1px;
		background: var(--border-soft);
		margin: 4px 6px;
	}
	.right-scroll {
		flex: 1;
		min-height: 0;
		overflow-y: auto;
		padding: 12px 24px 28px;
	}
	.ach-sections {
		display: flex;
		flex-direction: column;
		gap: 24px;
	}
	.ach-set-section,
	.ach-list {
		display: flex;
		flex-direction: column;
	}
	.ach-set-header {
		display: flex;
		align-items: center;
		justify-content: space-between;
		gap: 12px;
		flex-wrap: wrap;
		padding: 8px 0;
		border-bottom: 1px solid var(--border-soft);
	}
	.ach-set-title-group {
		display: flex;
		align-items: baseline;
		gap: 8px;
		min-width: 0;
	}
	.ach-set-title {
		margin: 0;
		font-size: 13px;
		font-weight: 700;
		color: var(--text-primary);
	}
	.set-meta-tag {
		font-size: 10px;
		font-weight: 700;
		text-transform: uppercase;
		letter-spacing: 0.04em;
		color: var(--text-muted);
	}
	.ach-set-progress {
		display: flex;
		align-items: center;
		gap: 10px;
		flex-shrink: 0;
	}
	.ach-set-stats {
		font-size: 11.5px;
		font-weight: 600;
		color: var(--text-muted);
	}
	.ach-set-bar-track {
		width: 84px;
		height: 3px;
		border-radius: 999px;
		background: var(--surface-2);
		overflow: hidden;
	}
	.ach-set-bar-fill {
		height: 100%;
		border-radius: inherit;
		background: var(--accent);
	}
	.ach-set-bar-fill.complete {
		background: var(--completed);
	}
	.clear-filters-btn {
		padding: 6px 12px;
		border-radius: 8px;
		border: 1px solid var(--border-soft);
		background: var(--surface-2);
		color: var(--text-primary);
		font-family: inherit;
		font-size: 12px;
		font-weight: 600;
		cursor: pointer;
	}
	.clear-filters-btn:hover {
		background: var(--surface-hover);
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
		padding: 10px 14px;
		border-radius: 10px;
		background: var(--surface-1);
		border: 1px solid color-mix(in srgb, var(--ok) 45%, var(--border-soft));
		color: var(--text-primary);
		box-shadow: var(--shadow-elevated);
		font-size: 12px;
		font-weight: 600;
	}
	.export-toast svg:first-child {
		color: var(--ok);
		flex-shrink: 0;
	}
	.export-toast.is-error {
		border-color: color-mix(in srgb, var(--err) 45%, var(--border-soft));
	}
	.export-toast.is-error svg:first-child {
		color: var(--err);
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
	}
	.export-toast-close:hover {
		background: var(--surface-hover);
		color: var(--text-primary);
	}
	@media (max-width: 920px) {
		.game-split {
			display: flex;
			flex-direction: column;
			overflow-y: auto;
			overflow-x: hidden;
		}
		.right-pane,
		.right-scroll {
			overflow: visible;
			flex: 0 0 auto;
		}
	}
</style>
