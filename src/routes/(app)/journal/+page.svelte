<script lang="ts">
	// src/routes/(app)/journal/+page.svelte
	import { onMount } from 'svelte';
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

	type JournalEvent = AchievementEvent | CompletionEvent;

	async function loadAllPins() {
		try {
			const res = await invoke<Record<string, string[]>>('get_all_pinned_achievements');
			pinnedMap = res ?? {};
		} catch {
			pinnedMap = {};
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

	onMount(() => {
		void loadAllPins();
		let unlisten: (() => void) | null = null;
		void listen<{ game_id: string; pins: string[] }>('pinned-achievements-updated', (ev) => {
			if (ev.payload?.game_id) {
				pinnedMap = { ...pinnedMap, [ev.payload.game_id]: ev.payload.pins ?? [] };
			}
		}).then((fn) => {
			unlisten = fn;
		});

		return () => {
			unlisten?.();
		};
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

		return events.sort((a, b) => b.date.getTime() - a.date.getTime());
	});

	type EventGroup = { dateLabel: string; events: JournalEvent[] };

	let groupedEvents = $derived.by((): EventGroup[] => {
		const groups: EventGroup[] = [];
		allEvents.forEach((e) => {
			const label = e.date
				.toLocaleDateString($i18n.locale, {
					weekday: 'long',
					day: 'numeric',
					month: 'long',
					year: 'numeric'
				})
				.toUpperCase();
			let group = groups.find((g) => g.dateLabel === label);
			if (!group) {
				group = { dateLabel: label, events: [] };
				groups.push(group);
			}
			group.events.push(e);
		});
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
											onerror={(e) => {
												const t = e.target as HTMLImageElement;
												t.style.display = 'none';
											}}
										/>
									{:else}
										<div class="achievement-icon achievement-icon-placeholder">🏆</div>
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

		{#each groupedEvents as group (group.dateLabel)}
			<div class="day-group">
				<div class="day-header">
					<span class="date-label">{group.dateLabel}</span>
					<span class="count"
						>{$i18n.t('journal.countAchievements', { count: group.events.length })}</span
					>
				</div>
				<div class="events-list">
					{#each group.events as event (event.game.id + ':' + (event.type === 'achievement' ? event.achievement.key : 'completion'))}
						<div class="event-card" class:completion={event.type === 'completion'}>
							<img
								src={getGameIcon(event.game)}
								alt={event.game.name}
								class="game-icon"
								onerror={(e) => {
									const t = e.target as HTMLImageElement;
									if (event.game.steam_id && !t.src.includes('header.jpg')) {
										t.src = `https://cdn.cloudflare.steamstatic.com/steam/apps/${event.game.steam_id}/header.jpg`;
									}
								}}
							/>
							<div class="event-info">
								{#if event.type === 'achievement'}
									<div class="achievement-row">
										{#if event.achievement.icon}
											<img
												src={event.achievement.icon}
												alt={event.achievement.name}
												class="achievement-icon"
												onerror={(e) => {
													const t = e.target as HTMLImageElement;
													t.style.display = 'none';
												}}
											/>
										{:else}
											<div class="achievement-icon achievement-icon-placeholder">🏆</div>
										{/if}
										<div class="text">
											<span class="name">{event.achievement.name}</span>
											<span class="game">{event.game.name}</span>
										</div>
									</div>
								{:else}
									<div class="completion-row">
										<div class="trophy-icon">🏆</div>
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
		background: rgba(200, 169, 110, 0.1);
		display: flex;
		align-items: center;
		justify-content: center;
		font-size: 18px;
		flex-shrink: 0;
	}
	.trophy-icon {
		width: 40px;
		height: 40px;
		border-radius: 8px;
		background: rgba(200, 169, 110, 0.1);
		display: flex;
		align-items: center;
		justify-content: center;
		font-size: 20px;
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
		background: rgba(255, 255, 255, 0.08);
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
	}
	.rarity {
		font-size: 11px;
		font-weight: 700;
		color: var(--accent, #c8a96e);
		background: rgba(200, 169, 110, 0.05);
		padding: 1px 6px;
		border-radius: 4px;
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
		background: rgba(255, 255, 255, 0.1);
		border-radius: 3px;
	}
</style>
