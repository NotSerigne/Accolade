<script lang="ts">
	// src/lib/game/GameSidebar.svelte
	import { tick } from 'svelte';
	import { invoke } from '@tauri-apps/api/core';
	import { SvelteSet } from 'svelte/reactivity';
	import {
		computeRemainingCompletionistHours,
		formatPlaytime,
		formatRemainingTimeHours,
		games,
		getGameHeaderUrl,
		type Game,
		type GameProgressSummary,
		type HltbData
	} from '$lib/stores/Games.js';
	import { addGameTag, removeGameTag, toggleGameFavorite } from '$lib/stores/selectedGame.js';
	import { i18n } from '$lib/stores/i18n.js';

	interface Props {
		game: Game;
		gameTitle: string;
		currentIndex: number;
		totalGames: number;
		prevGame: Game | null;
		nextGame: Game | null;
		progress: GameProgressSummary;
		isPlatinum: boolean;
		platinumPctDisplay: number;
		globalPctDisplay: number;
		missableRemainingCount: number;
		missableFilterActive: boolean;
		onGoToGame: (id: string) => void;
		onToggleMissableFilter: () => void;
		onExportJson: () => void;
		onExportPdf: () => void;
		onOpenFolder: () => void;
	}

	let {
		game,
		gameTitle,
		currentIndex,
		totalGames,
		prevGame,
		nextGame,
		progress,
		isPlatinum,
		platinumPctDisplay,
		globalPctDisplay,
		missableRemainingCount,
		missableFilterActive,
		onGoToGame,
		onToggleMissableFilter,
		onExportJson,
		onExportPdf,
		onOpenFolder
	}: Props = $props();

	let coverStage = $state(0);
	let addingTag = $state(false);
	let newTag = $state('');
	let tagInputEl = $state<HTMLInputElement | null>(null);

	let editingPlaytime = $state(false);
	let editPlaytimeHours = $state(0);
	let editPlaytimeMins = $state(0);

	let hltbData = $state<HltbData | null>(null);
	let hltbLoading = $state(false);
	let editingHltb = $state(false);
	let customHltbMain = $state('');
	let customHltbExtra = $state('');
	let customHltb100 = $state('');

	let gamePlaytimeSeconds = $derived(game.playtime_seconds ?? 0);

	let allTags = $derived.by(() => {
		const tags = new SvelteSet<string>();
		$games.forEach((g) => g.tags?.forEach((t) => tags.add(t)));
		return Array.from(tags).sort();
	});

	let remainingHours = $derived(
		computeRemainingCompletionistHours(hltbData?.completionist_hours, gamePlaytimeSeconds)
	);

	let prevHltbGameId: string | null = null;
	$effect(() => {
		const gid = game.id;
		if (gid === prevHltbGameId) return;
		prevHltbGameId = gid ?? null;
		const gname = game.name;
		coverStage = 0;
		addingTag = false;
		editingPlaytime = false;
		editingHltb = false;
		hltbData = null;
		if (!gid || !gname) {
			hltbLoading = false;
			return;
		}
		hltbLoading = true;
		void invoke<HltbData>('get_hltb_data', {
			gameId: gid,
			gameName: gname,
			forceRefresh: false
		})
			.then((res) => {
				if (game.id === gid) hltbData = res;
			})
			.catch((e) => console.error('HLTB error:', e))
			.finally(() => {
				if (game.id === gid) hltbLoading = false;
			});
	});

	let coverCandidates = $derived.by(() => {
		const raw = [
			getGameHeaderUrl(game),
			game.background_image_url?.trim() ?? '',
			game.steamgrid_icon_url?.trim() ?? ''
		].filter(Boolean);
		return raw.filter((url, idx, arr) => arr.indexOf(url) === idx);
	});
	let coverUrl = $derived(coverCandidates[coverStage] ?? '');

	let sourceLabel = $derived(
		typeof game.source === 'object' && game.source.type === 'Emulator'
			? game.source.value
			: game.source.type
	);
	let metaLine = $derived(
		[sourceLabel, ...(game.genres ?? []).slice(0, 3)].filter(Boolean).join(' · ')
	);

	let hltbSummaryText = $derived.by(() => {
		if (hltbLoading) return '...';
		if (
			!hltbData ||
			(!hltbData.main_hours && !hltbData.main_extra_hours && !hltbData.completionist_hours)
		) {
			return '—';
		}
		const main = hltbData.main_hours ? `${hltbData.main_hours}h` : '—';
		const extra = hltbData.main_extra_hours ? `${hltbData.main_extra_hours}h` : '—';
		const comp = hltbData.completionist_hours ? `${hltbData.completionist_hours}h` : '—';
		return `${main} · ${extra} · ${comp}`;
	});

	async function toggleTagInput() {
		addingTag = !addingTag;
		if (addingTag) {
			await tick();
			tagInputEl?.focus();
		} else {
			newTag = '';
		}
	}

	function submitTag() {
		const trimmed = newTag.trim();
		if (trimmed) {
			void addGameTag(game.id, trimmed);
			newTag = '';
			addingTag = false;
		}
	}

	function openPlaytimeEditor() {
		const secs = gamePlaytimeSeconds;
		editPlaytimeHours = Math.floor(secs / 3600);
		editPlaytimeMins = Math.floor((secs % 3600) / 60);
		editingPlaytime = true;
	}

	async function saveManualPlaytime() {
		const gid = game.id;
		const totalSecs =
			Math.max(0, Math.floor(Number(editPlaytimeHours) || 0)) * 3600 +
			Math.max(0, Math.floor(Number(editPlaytimeMins) || 0)) * 60;
		try {
			await invoke('set_manual_playtime', { gameId: gid, playtimeSeconds: totalSecs });
			games.update((list) =>
				list.map((g) => (g.id === gid ? { ...g, playtime_seconds: totalSecs } : g))
			);
			editingPlaytime = false;
		} catch (e) {
			console.error('Failed to save playtime:', e);
		}
	}

	async function refreshHltb() {
		const gid = game.id;
		hltbLoading = true;
		try {
			const res = await invoke<HltbData>('get_hltb_data', {
				gameId: gid,
				gameName: game.name,
				forceRefresh: true
			});
			if (game.id === gid) hltbData = res;
		} catch (e) {
			console.error('HLTB refresh error:', e);
		} finally {
			if (game.id === gid) hltbLoading = false;
		}
	}

	function openHltbEditor() {
		customHltbMain = hltbData?.main_hours ? String(hltbData.main_hours) : '';
		customHltbExtra = hltbData?.main_extra_hours ? String(hltbData.main_extra_hours) : '';
		customHltb100 = hltbData?.completionist_hours ? String(hltbData.completionist_hours) : '';
		editingHltb = true;
	}

	async function saveCustomHltb() {
		const gid = game.id;
		const parseOpt = (val: string): number | null => {
			const n = parseFloat(String(val).replace(',', '.'));
			return Number.isFinite(n) && n > 0 ? Math.round(n * 10) / 10 : null;
		};
		try {
			const updated = await invoke<HltbData>('set_custom_hltb_data', {
				gameId: gid,
				gameName: game.name,
				mainHours: parseOpt(customHltbMain),
				mainExtraHours: parseOpt(customHltbExtra),
				completionistHours: parseOpt(customHltb100)
			});
			if (game.id === gid) {
				hltbData = updated;
				editingHltb = false;
			}
		} catch (e) {
			console.error('Failed to save custom HLTB:', e);
		}
	}
</script>

<aside class="game-sidebar">
	<nav class="sb-nav" aria-label="Game navigation">
		<button
			type="button"
			class="sb-nav-btn"
			disabled={!prevGame}
			aria-label={$i18n.t('game.prevGame')}
			title={prevGame ? prevGame.name || prevGame.id : $i18n.t('game.prevGame')}
			onclick={() => prevGame && onGoToGame(prevGame.id)}
		>
			<svg
				width="12"
				height="12"
				viewBox="0 0 24 24"
				fill="none"
				stroke="currentColor"
				stroke-width="2.4"
				stroke-linecap="round"
				stroke-linejoin="round"
				aria-hidden="true"
			>
				<path d="M15 18l-6-6 6-6" />
			</svg>
			<span class="sb-nav-label">{prevGame ? prevGame.name || prevGame.id : '—'}</span>
		</button>

		<span class="sb-nav-index tabular-nums">{currentIndex + 1} / {totalGames}</span>

		<button
			type="button"
			class="sb-nav-btn align-right"
			disabled={!nextGame}
			aria-label={$i18n.t('game.nextGame')}
			title={nextGame ? nextGame.name || nextGame.id : $i18n.t('game.nextGame')}
			onclick={() => nextGame && onGoToGame(nextGame.id)}
		>
			<span class="sb-nav-label">{nextGame ? nextGame.name || nextGame.id : '—'}</span>
			<svg
				width="12"
				height="12"
				viewBox="0 0 24 24"
				fill="none"
				stroke="currentColor"
				stroke-width="2.4"
				stroke-linecap="round"
				stroke-linejoin="round"
				aria-hidden="true"
			>
				<path d="M9 18l6-6-6-6" />
			</svg>
		</button>
	</nav>

	<div class="sb-section sb-identity">
		<div class="sb-cover">
			{#if coverUrl}
				<img src={coverUrl} alt={gameTitle} decoding="async" onerror={() => (coverStage += 1)} />
			{:else}
				<div class="sb-cover-fallback" aria-hidden="true">
					<svg
						width="26"
						height="26"
						viewBox="0 0 24 24"
						fill="none"
						stroke="currentColor"
						stroke-width="1.6"
						stroke-linecap="round"
						stroke-linejoin="round"
					>
						<path d="M6 9H4.5a2.5 2.5 0 0 1 0-5H6" />
						<path d="M18 9h1.5a2.5 2.5 0 0 0 0-5H18" />
						<path d="M4 22h16" />
						<path d="M10 14.66V17c0 .55-.47.98-.97 1.21C7.85 18.75 7 20.24 7 22" />
						<path d="M14 14.66V17c0 .55.47.98.97 1.21C16.15 18.75 17 20.24 17 22" />
						<path d="M18 2H6v7a6 6 0 0 0 12 0V2Z" />
					</svg>
				</div>
			{/if}
		</div>

		<div class="sb-title-row">
			<h1 class="sb-title">{gameTitle}</h1>
			<button
				type="button"
				class="sb-fav-btn"
				class:is-favorited={game.is_favorite}
				aria-pressed={game.is_favorite}
				aria-label={$i18n.t(
					game.is_favorite ? 'games.action.removeFavorite' : 'games.action.addFavorite'
				)}
				title={$i18n.t(
					game.is_favorite ? 'games.action.removeFavorite' : 'games.action.addFavorite'
				)}
				onclick={() => void toggleGameFavorite(game.id)}
			>
				<svg
					width="17"
					height="17"
					viewBox="0 0 24 24"
					fill={game.is_favorite ? 'currentColor' : 'none'}
					stroke="currentColor"
					stroke-width="1.9"
					stroke-linecap="round"
					stroke-linejoin="round"
					aria-hidden="true"
				>
					<path
						d="M20.84 4.61a5.5 5.5 0 0 0-7.78 0L12 5.67l-1.06-1.06a5.5 5.5 0 0 0-7.78 7.78l1.06 1.06L12 21.23l7.78-7.78 1.06-1.06a5.5 5.5 0 0 0 0-7.78z"
					/>
				</svg>
			</button>
		</div>

		{#if metaLine}
			<div class="sb-meta-line" title={metaLine}>{metaLine}</div>
		{/if}

		<div class="sb-tags-row">
			{#each game.tags || [] as tag (tag)}
				<span class="sb-tag">
					<span>{tag}</span>
					<button
						type="button"
						class="sb-tag-remove"
						onclick={() => void removeGameTag(game.id, tag)}
						title={$i18n.t('game.removeTag')}
						aria-label={`${$i18n.t('game.removeTag')} ${tag}`}
					>
						×
					</button>
				</span>
			{/each}
			<button
				type="button"
				class="sb-tag-add-toggle"
				aria-expanded={addingTag}
				aria-label={$i18n.t('game.showTagInput')}
				title={$i18n.t('game.showTagInput')}
				onclick={() => void toggleTagInput()}
			>
				{addingTag ? '×' : '+'}
			</button>
		</div>

		{#if addingTag}
			<div class="sb-tag-editor">
				<input
					bind:this={tagInputEl}
					type="text"
					class="sb-inline-input"
					placeholder={$i18n.t('game.addTag')}
					aria-label={$i18n.t('game.addTag')}
					bind:value={newTag}
					list="existing-tags"
					onkeydown={(e) => {
						if (e.key === 'Enter') {
							e.preventDefault();
							submitTag();
						} else if (e.key === 'Escape') {
							newTag = '';
							addingTag = false;
						}
					}}
				/>
				<datalist id="existing-tags">
					{#each allTags as tag (tag)}
						<option value={tag}></option>
					{/each}
				</datalist>
				<button
					type="button"
					class="sb-inline-confirm"
					aria-label={$i18n.t('game.showTagInput')}
					onclick={submitTag}
				>
					+
				</button>
			</div>
		{/if}
	</div>

	<div class="sb-section sb-gauges">
		{#if progress.hasDlc}
			<div class="sb-gauge">
				<div class="sb-gauge-head">
					<span class="sb-gauge-label">{$i18n.t('game.platinumProgress')}</span>
					<span class="sb-gauge-val tabular-nums" class:is-complete={isPlatinum}>
						{progress.unlockedBase}/{progress.totalBase} · {platinumPctDisplay}%
					</span>
				</div>
				<div class="sb-bar-track">
					<div
						class="sb-bar-fill"
						style:width="{progress.platinumPct}%"
						style:background="var(--completed)"
					></div>
				</div>
			</div>

			<div class="sb-gauge">
				<div class="sb-gauge-head">
					<span class="sb-gauge-label">{$i18n.t('game.totalProgressWithDlc')}</span>
					<span class="sb-gauge-val tabular-nums" class:is-complete={globalPctDisplay === 100}>
						{progress.unlockedAll}/{progress.totalAll} · {globalPctDisplay}%
					</span>
				</div>
				<div class="sb-bar-track">
					<div
						class="sb-bar-fill"
						style:width="{progress.globalPct}%"
						style:background={globalPctDisplay === 100 ? 'var(--completed)' : 'var(--accent)'}
					></div>
				</div>
			</div>
		{:else}
			<div class="sb-gauge">
				<div class="sb-gauge-head">
					<span class="sb-gauge-label">{progress.unlockedBase} / {progress.totalBase}</span>
					<span class="sb-gauge-val tabular-nums" class:is-complete={isPlatinum}
						>{platinumPctDisplay}%</span
					>
				</div>
				<div class="sb-bar-track">
					<div
						class="sb-bar-fill"
						style:width="{progress.platinumPct}%"
						style:background={isPlatinum ? 'var(--completed)' : 'var(--accent)'}
					></div>
				</div>
			</div>
		{/if}
	</div>

	<div class="sb-section sb-telemetry">
		<div class="sb-stat-row">
			<span class="sb-stat-label">{$i18n.t('game.playtime')}</span>
			<div class="sb-stat-right">
				{#if !editingPlaytime}
					<span class="sb-stat-val tabular-nums">{formatPlaytime(gamePlaytimeSeconds)}</span>
				{/if}
				<button
					type="button"
					class="sb-icon-btn"
					aria-label={$i18n.t('game.editPlaytime')}
					title={$i18n.t('game.editPlaytime')}
					aria-expanded={editingPlaytime}
					onclick={() => (editingPlaytime ? (editingPlaytime = false) : openPlaytimeEditor())}
				>
					<svg
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
						<path d="M12 20h9" />
						<path d="M16.5 3.5a2.121 2.121 0 0 1 3 3L7 19l-4 1 1-4L16.5 3.5z" />
					</svg>
				</button>
			</div>
		</div>

		{#if editingPlaytime}
			<div class="sb-inline-editor">
				<div class="sb-pt-fields">
					<input
						type="number"
						min="0"
						bind:value={editPlaytimeHours}
						class="sb-num-input tabular-nums"
						aria-label={$i18n.t('game.hours')}
						title={$i18n.t('game.hours')}
						onkeydown={(e) => {
							if (e.key === 'Enter') void saveManualPlaytime();
							else if (e.key === 'Escape') editingPlaytime = false;
						}}
					/>
					<span class="sb-unit">h</span>
					<input
						type="number"
						min="0"
						max="59"
						bind:value={editPlaytimeMins}
						class="sb-num-input tabular-nums"
						aria-label={$i18n.t('game.minutes')}
						title={$i18n.t('game.minutes')}
						onkeydown={(e) => {
							if (e.key === 'Enter') void saveManualPlaytime();
							else if (e.key === 'Escape') editingPlaytime = false;
						}}
					/>
					<span class="sb-unit">m</span>
				</div>
				<div class="sb-editor-actions">
					<button
						type="button"
						class="sb-text-btn is-primary"
						onclick={() => void saveManualPlaytime()}
					>
						{$i18n.t('game.save')}
					</button>
					<button type="button" class="sb-text-btn" onclick={() => (editingPlaytime = false)}>
						{$i18n.t('game.cancel')}
					</button>
				</div>
			</div>
		{/if}

		<div class="sb-stat-row">
			<span class="sb-stat-label">HLTB</span>
			<div class="sb-stat-right">
				{#if !editingHltb}
					<span
						class="sb-stat-val tabular-nums"
						title={`${$i18n.t('game.hltb.main')} · ${$i18n.t('game.hltb.extra')} · ${$i18n.t('game.hltb.completionist')}`}
					>
						{hltbSummaryText}
					</span>
				{/if}
				<button
					type="button"
					class="sb-icon-btn"
					aria-label={$i18n.t('game.hltb.editManual')}
					title={$i18n.t('game.hltb.editManual')}
					aria-expanded={editingHltb}
					onclick={() => (editingHltb ? (editingHltb = false) : openHltbEditor())}
				>
					<svg
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
						<path d="M12 20h9" />
						<path d="M16.5 3.5a2.121 2.121 0 0 1 3 3L7 19l-4 1 1-4L16.5 3.5z" />
					</svg>
				</button>
				<button
					type="button"
					class="sb-icon-btn"
					disabled={hltbLoading}
					aria-label={$i18n.t('game.hltb.refresh')}
					title={$i18n.t('game.hltb.refresh')}
					onclick={() => void refreshHltb()}
				>
					<svg
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
						<polyline points="23 4 23 10 17 10" />
						<path d="M20.49 15a9 9 0 1 1-2.12-9.36L23 10" />
					</svg>
				</button>
			</div>
		</div>

		{#if editingHltb}
			<div class="sb-inline-editor">
				<div class="sb-hltb-grid">
					<label class="sb-hltb-field">
						<span>{$i18n.t('game.hltb.mainShort')}</span>
						<input
							type="number"
							step="0.5"
							min="0"
							bind:value={customHltbMain}
							placeholder="15"
							class="sb-num-input tabular-nums"
							onkeydown={(e) => {
								if (e.key === 'Enter') void saveCustomHltb();
								else if (e.key === 'Escape') editingHltb = false;
							}}
						/>
					</label>
					<label class="sb-hltb-field">
						<span>{$i18n.t('game.hltb.extraShort')}</span>
						<input
							type="number"
							step="0.5"
							min="0"
							bind:value={customHltbExtra}
							placeholder="28"
							class="sb-num-input tabular-nums"
							onkeydown={(e) => {
								if (e.key === 'Enter') void saveCustomHltb();
								else if (e.key === 'Escape') editingHltb = false;
							}}
						/>
					</label>
					<label class="sb-hltb-field">
						<span>{$i18n.t('game.hltb.compShort')}</span>
						<input
							type="number"
							step="0.5"
							min="0"
							bind:value={customHltb100}
							placeholder="45"
							class="sb-num-input tabular-nums"
							onkeydown={(e) => {
								if (e.key === 'Enter') void saveCustomHltb();
								else if (e.key === 'Escape') editingHltb = false;
							}}
						/>
					</label>
				</div>
				<div class="sb-editor-actions">
					<button
						type="button"
						class="sb-text-btn is-primary"
						onclick={() => void saveCustomHltb()}
					>
						{$i18n.t('game.save')}
					</button>
					<button type="button" class="sb-text-btn" onclick={() => (editingHltb = false)}>
						{$i18n.t('game.cancel')}
					</button>
				</div>
			</div>
		{/if}

		{#if remainingHours !== null || isPlatinum}
			<div class="sb-stat-row">
				<span class="sb-stat-label">{$i18n.t('game.estimatedRemaining')}</span>
				<span class="sb-stat-val tabular-nums" class:is-complete={isPlatinum}>
					{isPlatinum
						? $i18n.t('game.completedLabel')
						: remainingHours !== null
							? `~${formatRemainingTimeHours(remainingHours)}`
							: ''}
				</span>
			</div>
		{/if}
	</div>

	{#if missableRemainingCount > 0}
		<div class="sb-section">
			<button
				type="button"
				class="sb-missable-link"
				class:is-active={missableFilterActive}
				aria-pressed={missableFilterActive}
				onclick={onToggleMissableFilter}
			>
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
					<path
						d="M10.29 3.86 1.82 18a2 2 0 0 0 1.71 3h16.94a2 2 0 0 0 1.71-3L13.71 3.86a2 2 0 0 0-3.42 0Z"
					/>
					<line x1="12" y1="9" x2="12" y2="13" />
					<line x1="12" y1="17" x2="12.01" y2="17" />
				</svg>
				<span>
					{missableRemainingCount === 1
						? $i18n.t('game.tag.missableRemainingOne', { count: 1 })
						: $i18n.t('game.tag.missableRemainingMany', { count: missableRemainingCount })}
				</span>
			</button>
		</div>
	{/if}

	<footer class="sb-footer">
		<button
			type="button"
			class="sb-export-link"
			aria-label={$i18n.t('game.exportJson')}
			title={$i18n.t('game.exportJson')}
			onclick={onExportJson}
		>
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
				<path d="M21 15v4a2 2 0 0 1-2 2H5a2 2 0 0 1-2-2v-4" />
				<polyline points="7 10 12 15 17 10" />
				<line x1="12" y1="15" x2="12" y2="3" />
			</svg>
			<span>JSON</span>
		</button>

		<button
			type="button"
			class="sb-export-link"
			aria-label={$i18n.t('game.exportPdf')}
			title={$i18n.t('game.exportPdf')}
			onclick={onExportPdf}
		>
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
				<path d="M14 2H6a2 2 0 0 0-2 2v16a2 2 0 0 0 2 2h12a2 2 0 0 0 2-2V8z" />
				<polyline points="14 2 14 8 20 8" />
				<line x1="16" y1="13" x2="8" y2="13" />
				<line x1="16" y1="17" x2="8" y2="17" />
			</svg>
			<span>PDF</span>
		</button>

		<button
			type="button"
			class="sb-export-link"
			aria-label={$i18n.t('game.exportFolder')}
			title={$i18n.t('game.exportFolder')}
			onclick={onOpenFolder}
		>
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
				<path d="M22 19a2 2 0 0 1-2 2H4a2 2 0 0 1-2-2V5a2 2 0 0 1 2-2h5l2 3h9a2 2 0 0 1 2 2z" />
			</svg>
			<span>{$i18n.t('game.exportFolder')}</span>
		</button>
	</footer>
</aside>

<style>
	.game-sidebar {
		display: flex;
		flex-direction: column;
		height: 100%;
		min-height: 0;
		overflow-y: auto;
		border-right: 1px solid var(--border-soft);
	}
	@media (max-width: 920px) {
		.game-sidebar {
			height: auto;
			overflow: visible;
			border-right: none;
			border-bottom: 1px solid var(--border-soft);
		}
	}
	.sb-nav {
		display: grid;
		grid-template-columns: minmax(0, 1fr) auto minmax(0, 1fr);
		align-items: center;
		gap: 8px;
		padding: 12px 16px;
		border-bottom: 1px solid var(--border-soft);
		flex-shrink: 0;
	}
	.sb-nav-btn {
		display: inline-flex;
		align-items: center;
		gap: 5px;
		min-width: 0;
		padding: 4px 2px;
		background: transparent;
		border: none;
		border-radius: 6px;
		color: var(--text-secondary);
		font-family: inherit;
		font-size: 11.5px;
		font-weight: 500;
		cursor: pointer;
	}
	.sb-nav-btn.align-right {
		justify-content: flex-end;
	}
	.sb-nav-btn:hover:not(:disabled) {
		color: var(--text-primary);
	}
	.sb-nav-btn:disabled {
		opacity: 0.3;
		cursor: default;
	}
	.sb-nav-label {
		min-width: 0;
		overflow: hidden;
		text-overflow: ellipsis;
		white-space: nowrap;
	}
	.sb-nav-index {
		font-size: 11px;
		font-weight: 600;
		color: var(--text-muted);
		white-space: nowrap;
	}
	.sb-section {
		padding: 14px 16px;
		border-bottom: 1px solid var(--border-soft);
		display: flex;
		flex-direction: column;
		gap: 10px;
	}
	.sb-cover {
		width: 100%;
		aspect-ratio: 16 / 9;
		border-radius: 8px;
		border: 1px solid var(--border-soft);
		overflow: hidden;
		background: var(--surface-2);
	}
	.sb-cover img {
		width: 100%;
		height: 100%;
		object-fit: cover;
		display: block;
	}
	.sb-cover-fallback {
		width: 100%;
		height: 100%;
		display: flex;
		align-items: center;
		justify-content: center;
		color: var(--text-muted);
		opacity: 0.65;
	}
	.sb-title-row {
		display: flex;
		align-items: flex-start;
		justify-content: space-between;
		gap: 8px;
	}
	.sb-title {
		margin: 0;
		font-size: 17px;
		font-weight: 700;
		line-height: 1.25;
		letter-spacing: -0.02em;
		color: var(--text-primary);
		min-width: 0;
		word-break: break-word;
	}
	.sb-fav-btn {
		flex-shrink: 0;
		display: inline-flex;
		align-items: center;
		justify-content: center;
		width: 28px;
		height: 28px;
		margin-top: -2px;
		background: transparent;
		border: none;
		border-radius: 6px;
		color: var(--text-muted);
		cursor: pointer;
		transition:
			color 0.15s ease,
			transform 0.18s var(--spring-out);
	}
	.sb-fav-btn:hover {
		color: var(--text-primary);
		transform: scale(1.08);
	}
	.sb-fav-btn.is-favorited {
		color: var(--err);
	}
	.sb-meta-line {
		font-size: 11.5px;
		font-weight: 500;
		color: var(--text-muted);
		white-space: nowrap;
		overflow: hidden;
		text-overflow: ellipsis;
	}
	.sb-tags-row {
		display: flex;
		align-items: center;
		flex-wrap: wrap;
		gap: 6px;
	}
	.sb-tag {
		display: inline-flex;
		align-items: center;
		gap: 4px;
		font-size: 11px;
		font-weight: 500;
		color: var(--text-secondary);
		background: var(--surface-2);
		padding: 2px 7px;
		border-radius: 5px;
	}
	.sb-tag-remove {
		background: transparent;
		border: none;
		color: var(--text-muted);
		cursor: pointer;
		font-size: 13px;
		line-height: 1;
		padding: 0 1px;
	}
	.sb-tag-remove:hover {
		color: var(--text-primary);
	}
	.sb-tag-add-toggle {
		display: inline-flex;
		align-items: center;
		justify-content: center;
		width: 22px;
		height: 22px;
		background: transparent;
		border: 1px dashed var(--border-soft);
		border-radius: 5px;
		color: var(--text-muted);
		font-size: 13px;
		line-height: 1;
		cursor: pointer;
	}
	.sb-tag-add-toggle:hover {
		color: var(--text-primary);
		border-color: var(--border-specular);
	}
	.sb-tag-editor {
		display: flex;
		align-items: center;
		gap: 6px;
	}
	.sb-inline-input,
	.sb-num-input {
		background: var(--surface-2);
		border: 1px solid var(--border-soft);
		border-radius: 6px;
		font-family: inherit;
		color: var(--text-primary);
	}
	.sb-inline-input {
		flex: 1;
		min-width: 0;
		padding: 4px 8px;
		font-size: 11.5px;
	}
	.sb-inline-input:focus,
	.sb-num-input:focus {
		border-color: var(--accent);
	}
	.sb-inline-confirm {
		flex-shrink: 0;
		width: 24px;
		height: 24px;
		border-radius: 6px;
		border: none;
		background: var(--accent);
		color: var(--accent-text);
		font-weight: 700;
		cursor: pointer;
		display: inline-flex;
		align-items: center;
		justify-content: center;
	}
	.sb-gauges {
		gap: 12px;
	}
	.sb-gauge {
		display: flex;
		flex-direction: column;
		gap: 6px;
	}
	.sb-gauge-head {
		display: flex;
		align-items: baseline;
		justify-content: space-between;
		gap: 8px;
	}
	.sb-gauge-label {
		font-size: 11.5px;
		font-weight: 500;
		color: var(--text-secondary);
		min-width: 0;
		overflow: hidden;
		text-overflow: ellipsis;
		white-space: nowrap;
	}
	.sb-gauge-val {
		font-size: 12px;
		font-weight: 700;
		color: var(--text-primary);
		flex-shrink: 0;
	}
	.sb-gauge-val.is-complete,
	.sb-stat-val.is-complete {
		color: var(--completed);
	}
	.sb-bar-track {
		height: 4px;
		border-radius: 999px;
		background: var(--surface-2);
		overflow: hidden;
	}
	.sb-bar-fill {
		height: 100%;
		border-radius: inherit;
		transition: width 0.5s var(--spring-out);
	}
	.sb-telemetry {
		gap: 9px;
	}
	.sb-stat-row {
		display: flex;
		align-items: center;
		justify-content: space-between;
		gap: 8px;
		min-height: 22px;
	}
	.sb-stat-label {
		font-size: 11px;
		font-weight: 600;
		letter-spacing: 0.04em;
		text-transform: uppercase;
		color: var(--text-muted);
	}
	.sb-stat-right {
		display: inline-flex;
		align-items: center;
		gap: 5px;
		min-width: 0;
	}
	.sb-stat-val {
		font-size: 12px;
		font-weight: 600;
		color: var(--text-primary);
		white-space: nowrap;
	}
	.sb-icon-btn {
		display: inline-flex;
		align-items: center;
		justify-content: center;
		width: 22px;
		height: 22px;
		background: transparent;
		border: none;
		border-radius: 5px;
		color: var(--text-muted);
		cursor: pointer;
	}
	.sb-icon-btn:hover:not(:disabled) {
		color: var(--text-primary);
		background: var(--surface-hover);
	}
	.sb-icon-btn:disabled {
		opacity: 0.4;
		cursor: default;
	}
	.sb-inline-editor {
		display: flex;
		flex-direction: column;
		gap: 8px;
		padding: 6px 0 2px;
	}
	.sb-pt-fields {
		display: flex;
		align-items: center;
		gap: 5px;
	}
	.sb-num-input {
		width: 56px;
		padding: 4px 6px;
		font-size: 12px;
		font-weight: 600;
	}
	.sb-unit {
		font-size: 11.5px;
		color: var(--text-muted);
		margin-right: 4px;
	}
	.sb-hltb-grid {
		display: grid;
		grid-template-columns: repeat(3, minmax(0, 1fr));
		gap: 6px;
	}
	.sb-hltb-field {
		display: flex;
		flex-direction: column;
		gap: 4px;
		font-size: 10.5px;
		color: var(--text-muted);
	}
	.sb-hltb-field .sb-num-input {
		width: 100%;
	}
	.sb-editor-actions {
		display: flex;
		align-items: center;
		gap: 6px;
	}
	.sb-text-btn {
		padding: 4px 10px;
		border-radius: 6px;
		border: 1px solid var(--border-soft);
		background: transparent;
		color: var(--text-secondary);
		font-family: inherit;
		font-size: 11.5px;
		font-weight: 600;
		cursor: pointer;
	}
	.sb-text-btn:hover {
		color: var(--text-primary);
		background: var(--surface-hover);
	}
	.sb-text-btn.is-primary {
		background: var(--accent);
		border-color: var(--accent);
		color: var(--accent-text);
	}
	.sb-missable-link {
		display: inline-flex;
		align-items: center;
		gap: 7px;
		padding: 2px 0;
		background: transparent;
		border: none;
		color: var(--warn);
		font-family: inherit;
		font-size: 12px;
		font-weight: 600;
		cursor: pointer;
		text-align: left;
	}
	.sb-missable-link:hover {
		opacity: 0.85;
	}
	.sb-missable-link.is-active {
		text-decoration: underline;
		text-underline-offset: 3px;
	}
	.sb-footer {
		margin-top: auto;
		padding: 12px 16px;
		display: flex;
		align-items: center;
		flex-wrap: wrap;
		gap: 8px 14px;
	}
	.sb-export-link {
		display: inline-flex;
		align-items: center;
		gap: 5px;
		padding: 4px 2px;
		background: transparent;
		border: none;
		color: var(--text-muted);
		font-family: inherit;
		font-size: 11.5px;
		font-weight: 500;
		cursor: pointer;
	}
	.sb-export-link:hover {
		color: var(--text-primary);
	}
</style>
