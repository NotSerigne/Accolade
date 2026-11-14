<script lang="ts">
	// src/routes/(app)/journal/+page.svelte
	import { onMount, onDestroy } from 'svelte';
	import { invoke } from '@tauri-apps/api/core';
	import { listen } from '@tauri-apps/api/event';
	import { goto } from '$app/navigation';
	import { resolve } from '$app/paths';
	import { games, type Game, type Achievement } from '$lib/stores/Games.js';
	import { selectedGameId } from '$lib/stores/selectedGame.js';
	import { achievementJumpIntent } from '$lib/stores/ui.js';
	import { i18n } from '$lib/stores/i18n.js';

	let selectedFilter = $state('all');
	let pinnedMap = $state<Record<string, string[]>>({});
	let playSessions = $state<PlaySession[]>([]);

	let disposed = false;
	let unlistenPins: (() => void) | null = null;
	let unlistenSessions: (() => void) | null = null;

	interface PlaySession {
		appid: number;
		game_id: string;
		started_at: number;
		duration_seconds: number;
		unlocked_achievements: string[];
	}

	type PinnedItem = {
		game: Game;
		achievement: Achievement;
	};

	type AchievementEvent = {
		type: 'achievement';
		date: Date;
		achievement: Achievement;
		game: Game;
	};

	type CompletionEvent = {
		type: 'completion';
		date: Date;
		game: Game;
	};

	type SessionEvent = {
		type: 'session';
		date: Date;
		session: PlaySession;
		game: Game | null;
		gameName: string;
		unlockedAchievements: Achievement[];
		unlockedCount: number;
	};

	type JournalEvent = AchievementEvent | CompletionEvent | SessionEvent;

	type ResolvedPlaySession = {
		session: PlaySession;
		game: Game | null;
		gameName: string;
		unlockedAchievements: Achievement[];
		unlockedCount: number;
	};

	type ChartDay = {
		key: string;
		weekday: string;
		fullDate: string;
		durationSeconds: number;
		heightPx: number;
		isToday: boolean;
	};

	function mergePlaySessions(existing: PlaySession[], incoming: PlaySession[]): PlaySession[] {
		const byKey: Record<string, PlaySession> = {};
		for (const s of existing) {
			byKey[`${s.started_at}:${s.appid}:${s.game_id}`] = s;
		}
		for (const s of incoming) {
			byKey[`${s.started_at}:${s.appid}:${s.game_id}`] = s;
		}
		return Object.values(byKey).sort((a, b) => b.started_at - a.started_at);
	}

	async function loadAllPins() {
		try {
			const res = await invoke<Record<string, string[]>>('get_all_pinned_achievements');
			if (!disposed) {
				pinnedMap = res ?? {};
			}
		} catch {
			if (!disposed) {
				pinnedMap = {};
			}
		}
	}

	async function loadPlaySessions() {
		try {
			const res = await invoke<PlaySession[]>('get_play_sessions');
			if (!disposed && Array.isArray(res)) {
				playSessions = mergePlaySessions(playSessions, res);
			}
		} catch {
			if (!disposed) {
				playSessions = [];
			}
		}
	}

	async function unpinAchievement(gameId: string, achKey: string, event: MouseEvent) {
		event.stopPropagation();
		try {
			const updated = await invoke<string[]>('toggle_pinned_achievement', {
				gameId,
				achKey
			});
			pinnedMap = { ...pinnedMap, [gameId]: updated };
		} catch (e) {
			console.error(e);
		}
	}

	function openGame(gameId: string) {
		selectedGameId.set(gameId);
		void goto(resolve('/(app)/games/[id]', { id: gameId }));
	}

	function jumpToAchievement(gameId: string, ach: Achievement) {
		achievementJumpIntent.set({
			gameId,
			achievementKey: ach.key,
			token: Date.now()
		});
		selectedGameId.set(gameId);
		void goto(resolve('/(app)/games/[id]', { id: gameId }));
	}

	function formatProgressVal(v: number): string {
		return Number.isInteger(v) ? String(v) : v.toFixed(1);
	}

	function formatSessionDuration(seconds: number): string {
		const total = Math.max(0, Math.floor(Number.isFinite(seconds) ? seconds : 0));
		if (total === 0) return '0m';
		if (total < 60) return `${total}s`;
		if (total < 3600) {
			return `${Math.floor(total / 60)}m`;
		}
		const hours = Math.floor(total / 3600);
		const mins = Math.floor((total % 3600) / 60);
		return `${hours}h${String(mins).padStart(2, '0')}`;
	}

	function toLocalDayKey(date: Date): string {
		const y = date.getFullYear();
		const m = String(date.getMonth() + 1).padStart(2, '0');
		const d = String(date.getDate()).padStart(2, '0');
		return `${y}-${m}-${d}`;
	}

	function resolveSessionGame(session: PlaySession, gamesList: Game[]): Game | null {
		if (session.game_id) {
			const byId = gamesList.find((g) => g.id === session.game_id);
			if (byId) return byId;
		}
		if (session.appid > 0) {
			const steamMatch = gamesList.find(
				(g) =>
					g.steam_id === session.appid &&
					g.source.type === 'Emulator' &&
					g.source.value.toLowerCase() === 'steam'
			);
			if (steamMatch) return steamMatch;
			return gamesList.find((g) => g.steam_id === session.appid) ?? null;
		}
		return null;
	}

	function resolveSessionUnlockedAchievements(
		session: PlaySession,
		game: Game | null
	): Achievement[] {
		const rawUnlocked = session.unlocked_achievements ?? [];
		if (!game || rawUnlocked.length === 0) return [];
		const keySet = new Set(rawUnlocked.map((k) => k.trim().toLowerCase()).filter(Boolean));
		if (keySet.size === 0) return [];
		return (game.achievements ?? []).filter((a) => {
			const key = a.key?.trim().toLowerCase() ?? '';
			const name = a.name?.trim().toLowerCase() ?? '';
			return (key.length > 0 && keySet.has(key)) || (name.length > 0 && keySet.has(name));
		});
	}

	function getSessionGameName(session: PlaySession, game: Game | null): string {
		if (game?.name?.trim()) return game.name;
		if (session.game_id?.trim()) return session.game_id;
		if (session.appid > 0) return `AppID ${session.appid}`;
		return 'Unknown';
	}

	function getSessionGameIcon(session: PlaySession, game: Game | null): string {
		if (game) {
			const icon = getGameIcon(game);
			if (icon) return icon;
		}
		if (session.appid > 0) {
			return `https://cdn.cloudflare.steamstatic.com/steam/apps/${session.appid}/header.jpg`;
		}
		return '';
	}

	function getEventKey(event: JournalEvent): string {
		switch (event.type) {
			case 'achievement':
				return `ach:${event.game.id}:${event.achievement.key}`;
			case 'completion':
				return `completion:${event.game.id}`;
			case 'session':
				return `session:${event.session.started_at}:${event.session.appid}:${event.session.game_id}`;
		}
	}

	onMount(() => {
		void loadAllPins();
		void loadPlaySessions();

		void listen<{ game_id: string; pins: string[] }>('pinned-achievements-updated', (ev) => {
			if (ev.payload?.game_id) {
				pinnedMap = { ...pinnedMap, [ev.payload.game_id]: ev.payload.pins ?? [] };
			}
		}).then((fn) => {
			if (disposed) fn();
			else unlistenPins = fn;
		});

		void listen<PlaySession>('play-session-recorded', (ev) => {
			if (ev.payload && typeof ev.payload.started_at === 'number') {
				playSessions = mergePlaySessions(playSessions, [ev.payload]);
			}
		}).then((fn) => {
			if (disposed) fn();
			else unlistenSessions = fn;
		});
	});

	onDestroy(() => {
		disposed = true;
		unlistenPins?.();
		unlistenSessions?.();
		unlistenPins = null;
		unlistenSessions = null;
	});

	let pinnedAchievements = $derived.by((): PinnedItem[] => {
		const list: PinnedItem[] = [];
		for (const g of $games) {
			if (selectedFilter !== 'all' && g.id !== selectedFilter) continue;
			const keys = pinnedMap[g.id];
			if (!keys || keys.length === 0) continue;
			const wantedSet = new Set(keys.map((k) => k.trim().toLowerCase()));
			for (const a of g.achievements ?? []) {
				if (wantedSet.has(a.key.trim().toLowerCase())) {
					list.push({ game: g, achievement: a });
				}
			}
		}
		return list;
	});

	let resolvedSessions = $derived.by((): ResolvedPlaySession[] => {
		const gamesList = $games;
		return playSessions.map((session) => {
			const game = resolveSessionGame(session, gamesList);
			const unlockedAchievements = resolveSessionUnlockedAchievements(session, game);
			const rawCount = session.unlocked_achievements?.length ?? 0;
			return {
				session,
				game,
				gameName: getSessionGameName(session, game),
				unlockedAchievements,
				unlockedCount: rawCount > 0 ? rawCount : unlockedAchievements.length
			};
		});
	});

	let filteredSessions = $derived.by((): ResolvedPlaySession[] => {
		if (selectedFilter === 'all') return resolvedSessions;
		return resolvedSessions.filter(
			(item) => item.game?.id === selectedFilter || item.session.game_id === selectedFilter
		);
	});

	let chart7d = $derived.by(() => {
		const now = new Date();
		const todayKey = toLocalDayKey(now);
		const totalsByDay: Record<string, number> = {};

		for (const { session } of filteredSessions) {
			if (!Number.isFinite(session.started_at) || session.started_at <= 0) continue;
			const d = new Date(session.started_at * 1000);
			if (Number.isNaN(d.getTime())) continue;
			const key = toLocalDayKey(d);
			totalsByDay[key] = (totalsByDay[key] ?? 0) + Math.max(0, session.duration_seconds || 0);
		}

		const weekdayFmt = new Intl.DateTimeFormat($i18n.locale, { weekday: 'short' });
		const fullDateFmt = new Intl.DateTimeFormat($i18n.locale, {
			weekday: 'long',
			day: 'numeric',
			month: 'long',
			year: 'numeric'
		});

		const rawDays: Omit<ChartDay, 'heightPx'>[] = [];
		for (let offset = 6; offset >= 0; offset--) {
			const d = new Date(now.getFullYear(), now.getMonth(), now.getDate() - offset);
			const key = toLocalDayKey(d);
			rawDays.push({
				key,
				weekday: weekdayFmt.format(d),
				fullDate: fullDateFmt.format(d),
				durationSeconds: totalsByDay[key] ?? 0,
				isToday: key === todayKey
			});
		}

		const totalSeconds = rawDays.reduce((acc, day) => acc + day.durationSeconds, 0);
		const maxSeconds = Math.max(1, ...rawDays.map((day) => day.durationSeconds));

		const days: ChartDay[] = rawDays.map((day) => ({
			...day,
			heightPx:
				day.durationSeconds > 0
					? Math.max(6, Math.round((day.durationSeconds / maxSeconds) * 82))
					: 0
		}));

		return {
			days,
			totalSeconds
		};
	});

	let allEvents = $derived.by((): JournalEvent[] => {
		const events: JournalEvent[] = [];

		$games.forEach((g) => {
			if (selectedFilter !== 'all' && g.id !== selectedFilter) return;

			(g.achievements ?? [])
				.filter((a) => a.unlocked)
				.forEach((a) => {
					events.push({
						type: 'achievement',
						date: new Date((a.unlocked_time ?? 0) * 1000),
						achievement: a,
						game: g
					});
				});

			const total = g.achievements_total || g.achievements?.length || 0;
			const unlocked = g.achievements?.filter((a) => a.unlocked) ?? [];
			if (total > 0 && unlocked.length === total) {
				const lastUnlockedTime = Math.max(...unlocked.map((a) => a.unlocked_time ?? 0));
				events.push({
					type: 'completion',
					date: new Date(lastUnlockedTime * 1000),
					game: g
				});
			}
		});

		for (const item of filteredSessions) {
			const endTimestamp =
				item.session.started_at + Math.max(0, item.session.duration_seconds || 0);
			events.push({
				type: 'session',
				date: new Date(endTimestamp * 1000),
				session: item.session,
				game: item.game,
				gameName: item.gameName,
				unlockedAchievements: item.unlockedAchievements,
				unlockedCount: item.unlockedCount
			});
		}

		return events.sort((a, b) => b.date.getTime() - a.date.getTime());
	});

	type EventGroup = {
		dateKey: string;
		dateLabel: string;
		achievementCount: number;
		sessionSeconds: number;
		events: JournalEvent[];
	};

	let groupedEvents = $derived.by((): EventGroup[] => {
		const groups: EventGroup[] = [];
		const byKey: Record<string, EventGroup> = {};
		const dateFmt = new Intl.DateTimeFormat($i18n.locale, {
			weekday: 'long',
			day: 'numeric',
			month: 'long',
			year: 'numeric'
		});

		for (const e of allEvents) {
			const dateKey = toLocalDayKey(e.date);
			let group = byKey[dateKey];
			if (!group) {
				group = {
					dateKey,
					dateLabel: dateFmt.format(e.date).toUpperCase(),
					achievementCount: 0,
					sessionSeconds: 0,
					events: []
				};
				byKey[dateKey] = group;
				groups.push(group);
			}
			if (e.type === 'achievement') {
				group.achievementCount += 1;
			} else if (e.type === 'session') {
				group.sessionSeconds += Math.max(0, e.session.duration_seconds || 0);
			}
			group.events.push(e);
		}
		return groups;
	});

	function formatTime(date: Date) {
		return date.toLocaleTimeString($i18n.locale, { hour: '2-digit', minute: '2-digit' });
	}

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
</script>

<main class="journal-page">
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
						stroke-width="2.2"
						><path d="M4 19.5A2.5 2.5 0 0 1 6.5 17H20" /><path
							d="M6.5 2H20v20H6.5A2.5 2.5 0 0 1 4 19.5v-15A2.5 2.5 0 0 1 6.5 2z"
						/></svg
					>
				</div>
				<h1>{$i18n.t('journal.title')}</h1>
			</div>
			<p class="subtitle">{$i18n.t('journal.subtitle')}</p>
		</div>
		<div class="header-right">
			<select bind:value={selectedFilter} class="game-select">
				<option value="all">{$i18n.t('journal.allGames')}</option>
				{#each $games as game (game.id)}
					<option value={game.id}>{game.name}</option>
				{/each}
			</select>
		</div>
	</header>

	<div class="content scrollable">
		<section class="playtime-chart-card" aria-label={$i18n.t('journal.session.chartTitle')}>
			<div class="chart-header">
				<div class="chart-title-wrap">
					<div class="chart-icon-badge" aria-hidden="true">
						<svg
							width="16"
							height="16"
							viewBox="0 0 24 24"
							fill="none"
							stroke="currentColor"
							stroke-width="2"
							stroke-linecap="round"
							stroke-linejoin="round"
						>
							<circle cx="12" cy="12" r="10" />
							<polyline points="12 6 12 12 16 14" />
						</svg>
					</div>
					<h2 class="chart-title">{$i18n.t('journal.session.chartTitle')}</h2>
				</div>
				<span class="chart-total-pill tabular-nums">
					{$i18n.t('journal.session.chartTotal', {
						duration: formatSessionDuration(chart7d.totalSeconds)
					})}
				</span>
			</div>

			<div class="chart-plot">
				<div class="chart-gridlines" aria-hidden="true">
					<div class="chart-gridline"></div>
					<div class="chart-gridline"></div>
					<div class="chart-gridline"></div>
				</div>
				<div class="chart-bars" role="list">
					{#each chart7d.days as day (day.key)}
						{@const formattedDuration = formatSessionDuration(day.durationSeconds)}
						<div
							class="chart-col"
							class:is-today={day.isToday}
							class:has-playtime={day.durationSeconds > 0}
							role="listitem"
							title={`${day.fullDate} — ${formattedDuration}`}
						>
							<div class="chart-bar-area">
								{#if day.durationSeconds > 0}
									<span class="chart-value tabular-nums">{formattedDuration}</span>
									<div class="chart-bar-fill" style:height="{day.heightPx}px"></div>
								{:else}
									<div class="chart-bar-stub"></div>
								{/if}
							</div>
							<span class="chart-day-label">{day.weekday}</span>
						</div>
					{/each}
				</div>
			</div>

			{#if chart7d.totalSeconds === 0}
				<p class="chart-empty-hint">{$i18n.t('journal.session.noSessions7d')}</p>
			{/if}
		</section>

		{#if pinnedAchievements.length > 0}
			<div class="day-group pinned-section">
				<div class="day-header pinned-header">
					<span class="date-label pinned-label">
						<svg
							width="13"
							height="13"
							viewBox="0 0 24 24"
							fill="currentColor"
							stroke="currentColor"
							stroke-width="2"
							stroke-linecap="round"
							stroke-linejoin="round"
						>
							<line x1="12" y1="17" x2="12" y2="22" />
							<path
								d="M5 17h14v-1.76a2 2 0 0 0-1.11-1.79l-1.78-.9A2 2 0 0 1 15 10.76V6h1a2 2 0 0 0 0-4H8a2 2 0 0 0 0 4h1v4.76a2 2 0 0 1-1.11 1.79l-1.78.9A2 2 0 0 0 5 15.24Z"
							/>
						</svg>
						{$i18n.t('journal.pinnedSection')}
					</span>
					<span class="count">
						{$i18n.t('journal.pinnedCount', { count: pinnedAchievements.length })}
					</span>
				</div>
				<div class="events-list">
					{#each pinnedAchievements as item (item.game.id + ':pin:' + item.achievement.key)}
						{@const ach = item.achievement}
						<!-- svelte-ignore a11y_click_events_have_key_events -->
						<!-- svelte-ignore a11y_no_static_element_interactions -->
						<div
							class="event-card pinned-card"
							title={$i18n.t('journal.openGame')}
							onclick={() => jumpToAchievement(item.game.id, ach)}
						>
							<img
								src={getGameIcon(item.game)}
								alt={item.game.name}
								class="game-icon"
								loading="lazy"
								decoding="async"
								onerror={(e) => {
									const t = e.target as HTMLImageElement;
									if (item.game.steam_id && !t.src.includes('header.jpg')) {
										t.src = `https://cdn.cloudflare.steamstatic.com/steam/apps/${item.game.steam_id}/header.jpg`;
									}
								}}
							/>
							<div class="event-info">
								<div class="achievement-row">
									{#if ach.icon || ach.icon_gray}
										<img
											src={ach.unlocked ? ach.icon || ach.icon_gray : ach.icon_gray || ach.icon}
											alt={ach.name}
											class="achievement-icon"
											class:is-locked={!ach.unlocked}
											loading="lazy"
											decoding="async"
											onerror={(e) => {
												const t = e.target as HTMLImageElement;
												t.style.display = 'none';
											}}
										/>
									{:else}
										<div class="achievement-icon achievement-icon-placeholder">
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
									<div class="text">
										<span class="name">{ach.name || ach.key}</span>
										<span class="game">
											{item.game.name}
											{#if ach.desc}
												· <span class="pin-desc">{ach.desc}</span>
											{/if}
										</span>
										{#if !ach.unlocked && ach.max_progress && ach.max_progress > 0}
											{@const cur = ach.progress ?? 0}
											{@const pct = Math.min(Math.max((cur / ach.max_progress) * 100, 0), 100)}
											<div class="pin-progress-wrap">
												<div class="pin-progress-track">
													<div class="pin-progress-fill" style:width="{pct}%"></div>
												</div>
												<span class="pin-progress-txt">
													{formatProgressVal(cur)} / {formatProgressVal(ach.max_progress)}
												</span>
											</div>
										{/if}
									</div>
								</div>
							</div>
							<div class="event-meta pinned-meta">
								{#if ach.completionpercentage}
									<span class="rarity">
										{parseFloat(ach.completionpercentage).toFixed(1)}%
									</span>
								{/if}
								<button
									class="unpin-btn"
									onclick={(e) => unpinAchievement(item.game.id, ach.key, e)}
								>
									<svg
										width="11"
										height="11"
										viewBox="0 0 24 24"
										fill="currentColor"
										stroke="currentColor"
										stroke-width="2"
										stroke-linecap="round"
										stroke-linejoin="round"
									>
										<line x1="12" y1="17" x2="12" y2="22" />
										<path
											d="M5 17h14v-1.76a2 2 0 0 0-1.11-1.79l-1.78-.9A2 2 0 0 1 15 10.76V6h1a2 2 0 0 0 0-4H8a2 2 0 0 0 0 4h1v4.76a2 2 0 0 1-1.11 1.79l-1.78.9A2 2 0 0 0 5 15.24Z"
										/>
									</svg>
									{$i18n.t('hud.unpin')}
								</button>
							</div>
						</div>
					{/each}
				</div>
			</div>
		{/if}

		{#each groupedEvents as group (group.dateKey)}
			<div class="day-group">
				<div class="day-header">
					<span class="date-label">{group.dateLabel}</span>
					<span class="count">
						{#if group.achievementCount > 0 && group.sessionSeconds > 0}
							{$i18n.t('journal.countAchievements', { count: group.achievementCount })} · {formatSessionDuration(
								group.sessionSeconds
							)}
						{:else if group.achievementCount > 0}
							{$i18n.t('journal.countAchievements', { count: group.achievementCount })}
						{:else if group.sessionSeconds > 0}
							{formatSessionDuration(group.sessionSeconds)}
						{:else}
							{$i18n.t('journal.countAchievements', { count: group.events.length })}
						{/if}
					</span>
				</div>
				<div class="events-list">
					{#each group.events as event (getEventKey(event))}
						<div
							class="event-card"
							class:completion={event.type === 'completion'}
							class:session={event.type === 'session'}
						>
							{#if event.type === 'session'}
								{@const sessionIconUrl = getSessionGameIcon(event.session, event.game)}
								{#if sessionIconUrl}
									<img
										src={sessionIconUrl}
										alt={event.gameName}
										class="game-icon"
										loading="lazy"
										decoding="async"
										onerror={(e) => {
											const t = e.target as HTMLImageElement;
											const fallbackAppId =
												event.game?.steam_id ||
												(event.session.appid > 0 ? event.session.appid : null);
											if (fallbackAppId && !t.src.includes('header.jpg')) {
												t.src = `https://cdn.cloudflare.steamstatic.com/steam/apps/${fallbackAppId}/header.jpg`;
											} else {
												t.style.display = 'none';
											}
										}}
									/>
								{:else}
									<div class="game-icon game-icon-placeholder" aria-hidden="true">
										<svg
											width="16"
											height="16"
											viewBox="0 0 24 24"
											fill="none"
											stroke="currentColor"
											stroke-width="2"
											stroke-linecap="round"
											stroke-linejoin="round"
										>
											<line x1="6" y1="12" x2="10" y2="12" />
											<line x1="8" y1="10" x2="8" y2="14" />
											<line x1="15" y1="13" x2="15.01" y2="13" />
											<line x1="18" y1="11" x2="18.01" y2="11" />
											<rect x="2" y="6" width="20" height="12" rx="4" />
										</svg>
									</div>
								{/if}
							{:else}
								<img
									src={getGameIcon(event.game)}
									alt={event.game.name}
									class="game-icon"
									loading="lazy"
									decoding="async"
									onerror={(e) => {
										const t = e.target as HTMLImageElement;
										if (event.game.steam_id && !t.src.includes('header.jpg')) {
											t.src = `https://cdn.cloudflare.steamstatic.com/steam/apps/${event.game.steam_id}/header.jpg`;
										}
									}}
								/>
							{/if}

							<div class="event-info">
								{#if event.type === 'achievement'}
									<div class="achievement-row">
										{#if event.achievement.icon}
											<img
												src={event.achievement.icon}
												alt={event.achievement.name}
												class="achievement-icon"
												loading="lazy"
												decoding="async"
												onerror={(e) => {
													const t = e.target as HTMLImageElement;
													t.style.display = 'none';
												}}
											/>
										{:else}
											<div class="achievement-icon achievement-icon-placeholder">
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
										<div class="text">
											<span class="name">{event.achievement.name}</span>
											<span class="game">{event.game.name}</span>
										</div>
									</div>
								{:else if event.type === 'session'}
									<div class="session-block">
										<div class="session-row">
											<div class="session-icon" aria-hidden="true">
												<svg
													width="20"
													height="20"
													viewBox="0 0 24 24"
													fill="none"
													stroke="currentColor"
													stroke-width="2"
													stroke-linecap="round"
													stroke-linejoin="round"
												>
													<line x1="6" y1="12" x2="10" y2="12" />
													<line x1="8" y1="10" x2="8" y2="14" />
													<line x1="15" y1="13" x2="15.01" y2="13" />
													<line x1="18" y1="11" x2="18.01" y2="11" />
													<rect x="2" y="6" width="20" height="12" rx="4" />
												</svg>
											</div>
											<div class="text">
												<div class="session-headline">
													<span class="session-badge">
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
															<circle cx="12" cy="12" r="10" />
															<polyline points="12 6 12 12 16 14" />
														</svg>
														{$i18n.t('journal.session.badge')}
													</span>
													{#if event.game}
														{@const targetGame = event.game}
														<button
															type="button"
															class="session-title-btn"
															title={$i18n.t('journal.openGame')}
															onclick={() => openGame(targetGame.id)}
														>
															{$i18n.t('journal.session.entryTitle', {
																duration: formatSessionDuration(event.session.duration_seconds),
																game: event.gameName
															})}
														</button>
													{:else}
														<span class="name">
															{$i18n.t('journal.session.entryTitle', {
																duration: formatSessionDuration(event.session.duration_seconds),
																game: event.gameName
															})}
														</span>
													{/if}
												</div>
												<div class="session-subline">
													{#if event.unlockedCount > 0}
														<span class="session-unlocks-pill tabular-nums">
															{$i18n.t('journal.session.unlockedCount', {
																count: event.unlockedCount
															})}
														</span>
													{:else}
														<span class="session-no-unlocks">
															{$i18n.t('journal.session.noUnlocks')}
														</span>
													{/if}
												</div>
											</div>
										</div>

										{#if event.unlockedAchievements.length > 0 && event.game}
											{@const targetGame = event.game}
											<div class="session-chips">
												{#each event.unlockedAchievements as ach, idx (ach.key || `${ach.name}:${idx}`)}
													<button
														type="button"
														class="session-ach-chip"
														title={$i18n.t('journal.openGame')}
														onclick={() => jumpToAchievement(targetGame.id, ach)}
													>
														{#if ach.icon || ach.icon_gray}
															<img
																src={ach.icon || ach.icon_gray}
																alt={ach.name || ach.key}
																class="session-chip-icon"
																loading="lazy"
																decoding="async"
																onerror={(e) => {
																	const t = e.target as HTMLImageElement;
																	t.style.display = 'none';
																}}
															/>
														{/if}
														<span class="session-chip-name">{ach.name || ach.key}</span>
													</button>
												{/each}
											</div>
										{/if}
									</div>
								{:else}
									<div class="completion-row">
										<div class="trophy-icon">
											<svg
												width="20"
												height="20"
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
										<div class="text">
											<span class="name">{$i18n.t('journal.completed')}</span>
											<span class="game">{event.game.name}</span>
										</div>
									</div>
								{/if}
							</div>

							<div class="event-meta">
								<span class="time">{formatTime(event.date)}</span>
								{#if event.type === 'achievement'}
									<span class="rarity">{event.achievement.completionpercentage}%</span>
								{:else if event.type === 'session'}
									<span class="session-duration-meta tabular-nums">
										{formatSessionDuration(event.session.duration_seconds)}
									</span>
								{/if}
							</div>
						</div>
					{/each}
				</div>
			</div>
		{/each}

		{#if groupedEvents.length === 0 && pinnedAchievements.length === 0}
			<div class="empty-state">
				<p>{$i18n.t('journal.empty')}</p>
			</div>
		{/if}
	</div>
</main>

<style>
	.journal-page {
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

	.game-select {
		background: var(--surface-1);
		border: 1px solid var(--border-soft);
		color: var(--text-primary);
		padding: 8px 16px;
		border-radius: 8px;
		outline: none;
		font-size: 13px;
	}

	.content {
		flex: 1;
		overflow-y: auto;
		padding: 0 32px 32px;
	}

	.playtime-chart-card {
		background: linear-gradient(
			145deg,
			color-mix(in srgb, var(--accent, #c8a96e) 7%, var(--glass-bg, var(--surface-1))),
			var(--surface-1)
		);
		backdrop-filter: blur(18px) saturate(160%);
		-webkit-backdrop-filter: blur(18px) saturate(160%);
		border: 1px solid var(--border-specular, var(--border-soft));
		border-radius: 16px;
		padding: 18px 22px 16px;
		margin-bottom: 32px;
		box-shadow: var(--shadow-card);
	}

	.chart-header {
		display: flex;
		align-items: center;
		justify-content: space-between;
		gap: 12px;
		flex-wrap: wrap;
		margin-bottom: 16px;
	}
	.chart-title-wrap {
		display: flex;
		align-items: center;
		gap: 10px;
		min-width: 0;
	}
	.chart-icon-badge {
		width: 30px;
		height: 30px;
		border-radius: 9px;
		background: color-mix(in srgb, var(--accent, #c8a96e) 14%, transparent);
		border: 1px solid color-mix(in srgb, var(--accent, #c8a96e) 28%, transparent);
		color: var(--accent, #c8a96e);
		display: flex;
		align-items: center;
		justify-content: center;
		flex-shrink: 0;
	}
	.chart-title {
		font-size: 15px;
		font-weight: 700;
		color: var(--text-primary);
		margin: 0;
	}
	.chart-total-pill {
		font-size: 12px;
		font-weight: 700;
		color: var(--accent, #c8a96e);
		background: color-mix(in srgb, var(--accent, #c8a96e) 12%, transparent);
		border: 1px solid color-mix(in srgb, var(--accent, #c8a96e) 28%, transparent);
		padding: 4px 11px;
		border-radius: 999px;
		white-space: nowrap;
	}

	.chart-plot {
		position: relative;
		padding-top: 4px;
	}
	.chart-gridlines {
		position: absolute;
		inset: 4px 0 24px 0;
		display: flex;
		flex-direction: column;
		justify-content: space-between;
		pointer-events: none;
	}
	.chart-gridline {
		width: 100%;
		border-top: 1px dashed var(--border-soft);
		opacity: 0.55;
	}
	.chart-bars {
		position: relative;
		z-index: 1;
		display: grid;
		grid-template-columns: repeat(7, minmax(0, 1fr));
		gap: 10px;
		align-items: end;
	}
	.chart-col {
		display: flex;
		flex-direction: column;
		align-items: center;
		gap: 8px;
		padding: 4px 2px 2px;
		border-radius: 10px;
		transition: background 0.16s ease;
		cursor: default;
	}
	.chart-col:hover {
		background: var(--surface-2);
	}
	.chart-bar-area {
		height: 108px;
		width: 100%;
		display: flex;
		flex-direction: column;
		align-items: center;
		justify-content: flex-end;
		gap: 5px;
	}
	.chart-value {
		font-size: 11px;
		font-weight: 600;
		color: var(--text-secondary);
		line-height: 1;
		white-space: nowrap;
	}
	.chart-col.has-playtime:hover .chart-value,
	.chart-col.is-today.has-playtime .chart-value {
		color: var(--accent, #c8a96e);
	}
	.chart-bar-fill {
		width: min(38px, 68%);
		border-radius: 7px 7px 3px 3px;
		background: linear-gradient(
			180deg,
			var(--accent, #c8a96e),
			color-mix(in srgb, var(--accent, #c8a96e) 58%, transparent)
		);
		border: 1px solid color-mix(in srgb, var(--accent, #c8a96e) 55%, transparent);
		box-shadow: 0 4px 14px color-mix(in srgb, var(--accent, #c8a96e) 22%, transparent);
		transition:
			height 0.35s var(--spring-out, cubic-bezier(0.22, 1, 0.36, 1)),
			filter 0.16s ease,
			transform 0.16s ease;
	}
	.chart-col.has-playtime:hover .chart-bar-fill {
		filter: brightness(1.14);
		transform: translateY(-1px);
	}
	.chart-bar-stub {
		width: min(32px, 56%);
		height: 3px;
		border-radius: 999px;
		background: var(--border-soft);
		opacity: 0.7;
	}
	.chart-day-label {
		font-size: 11px;
		font-weight: 600;
		color: var(--text-muted);
		text-transform: capitalize;
		line-height: 1.2;
	}
	.chart-col.is-today .chart-day-label {
		color: var(--accent, #c8a96e);
		font-weight: 700;
	}
	.chart-empty-hint {
		margin: 12px 0 0;
		font-size: 12px;
		color: var(--text-muted);
		text-align: center;
	}

	.day-group {
		margin-bottom: 40px;
	}
	.day-header {
		display: flex;
		justify-content: space-between;
		align-items: center;
		padding-bottom: 12px;
		border-bottom: 1px solid var(--border-soft);
		margin-bottom: 16px;
	}
	.pinned-header {
		border-bottom-color: rgba(200, 169, 110, 0.28);
	}
	.date-label {
		font-size: 12px;
		font-weight: 700;
		color: var(--text-secondary);
		letter-spacing: 0.05em;
	}
	.pinned-label {
		display: inline-flex;
		align-items: center;
		gap: 7px;
		color: var(--accent, #c8a96e);
	}
	.day-header .count {
		font-size: 11px;
		color: var(--text-muted);
		font-variant-numeric: tabular-nums;
	}

	.events-list {
		display: flex;
		flex-direction: column;
		gap: 8px;
	}
	.event-card {
		background: var(--surface-1);
		border: 1px solid var(--border-soft);
		border-radius: 12px;
		padding: 12px 20px;
		display: flex;
		align-items: center;
		gap: 20px;
	}
	.event-card.completion {
		background: rgba(200, 169, 110, 0.05);
		border-color: rgba(200, 169, 110, 0.1);
	}
	.event-card.pinned-card {
		border-color: rgba(200, 169, 110, 0.22);
		background: linear-gradient(135deg, rgba(200, 169, 110, 0.07), var(--surface-1));
		cursor: pointer;
		transition:
			transform 0.15s ease,
			border-color 0.15s ease;
	}
	.event-card.pinned-card:hover {
		border-color: var(--accent, #c8a96e);
		transform: translateY(-1px);
	}

	.game-icon {
		width: 32px;
		height: 32px;
		border-radius: 6px;
		object-fit: cover;
	}
	.event-info {
		flex: 1;
		min-width: 0;
	}

	.achievement-row,
	.completion-row {
		display: flex;
		align-items: center;
		gap: 16px;
	}
	.achievement-icon {
		width: 40px;
		height: 40px;
		border-radius: 8px;
		flex-shrink: 0;
	}
	.achievement-icon.is-locked {
		filter: grayscale(0.6);
		opacity: 0.85;
	}
	.achievement-icon-placeholder {
		background: color-mix(in srgb, var(--accent, #c8a96e) 12%, transparent);
		color: var(--accent, #c8a96e);
		display: flex;
		align-items: center;
		justify-content: center;
		flex-shrink: 0;
	}
	.trophy-icon {
		width: 40px;
		height: 40px;
		border-radius: 8px;
		background: color-mix(in srgb, var(--accent, #c8a96e) 12%, transparent);
		color: var(--accent, #c8a96e);
		display: flex;
		align-items: center;
		justify-content: center;
		flex-shrink: 0;
	}

	.text {
		display: flex;
		flex-direction: column;
		min-width: 0;
		flex: 1;
	}
	.text .name {
		font-size: 15px;
		font-weight: 600;
		color: var(--text-primary);
		white-space: nowrap;
		overflow: hidden;
		text-overflow: ellipsis;
	}
	.text .game {
		font-size: 12px;
		color: var(--text-secondary);
		white-space: nowrap;
		overflow: hidden;
		text-overflow: ellipsis;
	}
	.pin-desc {
		color: var(--text-muted);
	}

	.pin-progress-wrap {
		display: flex;
		align-items: center;
		gap: 10px;
		margin-top: 6px;
		max-width: 280px;
	}
	.pin-progress-track {
		flex: 1;
		height: 4px;
		background: var(--surface-3);
		border-radius: 99px;
		overflow: hidden;
	}
	.pin-progress-fill {
		height: 100%;
		background: var(--accent, #c8a96e);
		border-radius: 99px;
	}
	.pin-progress-txt {
		font-size: 11px;
		font-weight: 700;
		color: var(--accent, #c8a96e);
		white-space: nowrap;
		font-variant-numeric: tabular-nums;
	}

	.event-meta {
		display: flex;
		flex-direction: column;
		align-items: flex-end;
		gap: 4px;
	}
	.pinned-meta {
		flex-direction: row;
		align-items: center;
		gap: 10px;
	}
	.time {
		font-size: 12px;
		color: var(--text-muted);
		font-variant-numeric: tabular-nums;
	}
	.rarity {
		font-size: 11px;
		font-weight: 700;
		color: var(--accent, #c8a96e);
		background: color-mix(in srgb, var(--accent, #c8a96e) 10%, transparent);
		padding: 1px 6px;
		border-radius: 4px;
		font-variant-numeric: tabular-nums;
	}

	.unpin-btn {
		display: inline-flex;
		align-items: center;
		gap: 6px;
		padding: 5px 10px;
		border-radius: 7px;
		border: 1px solid rgba(200, 169, 110, 0.35);
		background: rgba(200, 169, 110, 0.12);
		color: var(--accent, #c8a96e);
		font-size: 11px;
		font-weight: 700;
		cursor: pointer;
		transition:
			background 0.15s ease,
			border-color 0.15s ease;
	}
	.unpin-btn:hover {
		background: rgba(239, 68, 68, 0.15);
		border-color: rgba(239, 68, 68, 0.45);
		color: #f87171;
	}

	.event-card.session {
		align-items: flex-start;
		background: linear-gradient(
			135deg,
			color-mix(in srgb, var(--accent, #c8a96e) 5%, var(--surface-1)),
			var(--surface-1)
		);
		border-color: color-mix(in srgb, var(--accent, #c8a96e) 20%, var(--border-soft));
	}
	.event-card.session .game-icon {
		margin-top: 4px;
		flex-shrink: 0;
	}
	.event-card.session .event-meta {
		margin-top: 2px;
		flex-shrink: 0;
	}
	.game-icon-placeholder {
		background: var(--surface-2);
		border: 1px solid var(--border-soft);
		color: var(--text-muted);
		display: flex;
		align-items: center;
		justify-content: center;
	}

	.session-block {
		display: flex;
		flex-direction: column;
		gap: 10px;
		min-width: 0;
	}
	.session-row {
		display: flex;
		align-items: center;
		gap: 16px;
		min-width: 0;
	}
	.session-icon {
		width: 40px;
		height: 40px;
		border-radius: 8px;
		background: color-mix(in srgb, var(--accent, #c8a96e) 12%, transparent);
		border: 1px solid color-mix(in srgb, var(--accent, #c8a96e) 26%, transparent);
		color: var(--accent, #c8a96e);
		display: flex;
		align-items: center;
		justify-content: center;
		flex-shrink: 0;
	}
	.session-headline {
		display: flex;
		align-items: center;
		gap: 8px;
		flex-wrap: wrap;
		min-width: 0;
	}
	.session-badge {
		display: inline-flex;
		align-items: center;
		gap: 5px;
		padding: 2px 8px;
		border-radius: 999px;
		background: color-mix(in srgb, var(--accent, #c8a96e) 14%, transparent);
		border: 1px solid color-mix(in srgb, var(--accent, #c8a96e) 32%, transparent);
		color: var(--accent, #c8a96e);
		font-size: 11px;
		font-weight: 700;
		letter-spacing: 0.02em;
		flex-shrink: 0;
	}
	.session-title-btn {
		appearance: none;
		border: none;
		background: transparent;
		padding: 0;
		margin: 0;
		font-family: inherit;
		font-size: 15px;
		font-weight: 600;
		color: var(--text-primary);
		text-align: left;
		cursor: pointer;
		min-width: 0;
		transition: color 0.15s ease;
	}
	.session-title-btn:hover {
		color: var(--accent, #c8a96e);
	}
	.session-subline {
		display: flex;
		align-items: center;
		gap: 8px;
		margin-top: 4px;
	}
	.session-unlocks-pill {
		display: inline-flex;
		align-items: center;
		padding: 1px 8px;
		border-radius: 999px;
		font-size: 11px;
		font-weight: 700;
		color: #34d399;
		background: rgba(52, 211, 153, 0.12);
		border: 1px solid rgba(52, 211, 153, 0.3);
	}
	:global([data-theme='light']) .session-unlocks-pill {
		color: #047857;
		background: rgba(16, 185, 129, 0.12);
		border-color: rgba(16, 185, 129, 0.3);
	}
	.session-no-unlocks {
		font-size: 12px;
		color: var(--text-muted);
	}
	.session-chips {
		display: flex;
		flex-wrap: wrap;
		gap: 6px;
		padding-left: 56px;
	}
	.session-ach-chip {
		display: inline-flex;
		align-items: center;
		gap: 6px;
		padding: 4px 9px 4px 5px;
		border-radius: 8px;
		background: var(--surface-2);
		border: 1px solid var(--border-soft);
		color: var(--text-primary);
		font-family: inherit;
		font-size: 12px;
		font-weight: 500;
		max-width: 240px;
		cursor: pointer;
		transition:
			border-color 0.15s ease,
			background 0.15s ease,
			transform 0.15s ease;
	}
	.session-ach-chip:hover {
		border-color: var(--accent, #c8a96e);
		background: color-mix(in srgb, var(--accent, #c8a96e) 12%, var(--surface-2));
		transform: translateY(-1px);
	}
	.session-chip-icon {
		width: 18px;
		height: 18px;
		border-radius: 4px;
		object-fit: cover;
		flex-shrink: 0;
	}
	.session-chip-name {
		white-space: nowrap;
		overflow: hidden;
		text-overflow: ellipsis;
	}
	.session-duration-meta {
		font-size: 11px;
		font-weight: 700;
		color: var(--text-secondary);
		background: var(--surface-2);
		border: 1px solid var(--border-soft);
		padding: 1px 6px;
		border-radius: 4px;
	}

	.empty-state {
		text-align: center;
		padding: 60px;
		color: var(--text-muted);
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
</style>
