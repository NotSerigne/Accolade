<script lang="ts">
	// src/lib/Topbar.svelte
	import { onMount } from 'svelte';
	import { goto } from '$app/navigation';
	import { resolve } from '$app/paths';
	import { games, getGameIconUrl, syncSteamMetadata } from '$lib/stores/Games.js';
	import { selectedGameId } from '$lib/stores/selectedGame.js';
	import {
		searchQuery,
		searchDraft,
		achievementJumpIntent,
		watcherActive,
		isSyncing
	} from '$lib/stores/ui.js';
	import { i18n } from '$lib/stores/i18n.js';
	import { runningGame } from '$lib/stores/runningGame.js';

	let inputEl: HTMLInputElement | null = null;
	let searchWrapEl: HTMLDivElement | null = null;
	let suggestionsOpen = $state(false);
	let activeIndex = $state(-1);

	type Suggestion = {
		kind: 'game' | 'achievement';
		gameId: string;
		title: string;
		subtitle: string;
		query: string;
		iconUrl: string;
		achievementKey?: string;
	};

	type AchievementSearchCandidate = {
		gameId: string;
		title: string;
		subtitle: string;
		query: string;
		iconUrl: string;
		achievementKey: string;
		haystack: string;
	};

	let searchPlaceholder = $derived($i18n.t('topbar.searchPlaceholder'));

	let normalizedDraft = $derived($searchDraft.trim().toLowerCase());

	let achievementSearchCandidates = $derived.by((): AchievementSearchCandidate[] => {
		return $games.flatMap((g) =>
			(g.achievements ?? []).map((a) => {
				const title = a.name || a.key;
				const subtitle = `${g.name || `ID ${g.id}`} · ${(a.desc || '').slice(0, 60)}`;
				return {
					gameId: g.id,
					title,
					subtitle,
					query: title,
					iconUrl: a.icon || '',
					achievementKey: a.key,
					haystack: `${a.name || ''} ${a.desc || ''}`.toLowerCase()
				};
			})
		);
	});

	let suggestions = $derived.by((): Suggestion[] => {
		if (!normalizedDraft) return [];

		const results: Suggestion[] = $games
			.filter((g) => `${g.name || ''} ${g.id}`.toLowerCase().includes(normalizedDraft))
			.slice(0, 4)
			.map((g) => ({
				kind: 'game' as const,
				gameId: g.id,
				title: g.name || `ID ${g.id}`,
				subtitle: $i18n.t('topbar.gameSubtitle', { id: g.id }),
				query: g.name || g.id,
				iconUrl: getGameIconUrl(g),
				achievementKey: undefined
			}));

		if (results.length < 8) {
			for (const candidate of achievementSearchCandidates) {
				if (!candidate.haystack.includes(normalizedDraft)) continue;
				results.push({
					kind: 'achievement',
					gameId: candidate.gameId,
					title: candidate.title,
					subtitle: candidate.subtitle,
					query: candidate.query,
					iconUrl: candidate.iconUrl,
					achievementKey: candidate.achievementKey
				});
				if (results.length >= 8) break;
			}
		}

		return results;
	});

	let activeDescendantId = $derived(
		suggestionsOpen && normalizedDraft && activeIndex >= 0 && activeIndex < suggestions.length
			? `search-suggestion-${activeIndex}`
			: undefined
	);

	$effect(() => {
		void normalizedDraft;
		void suggestions.length;
		activeIndex = -1;
	});

	function focusSearch() {
		inputEl?.focus();
		suggestionsOpen = true;
	}

	function applySearch(value?: string) {
		const query = (value ?? $searchDraft).trim();
		searchQuery.set(query);
	}

	function onSearchInput(event: Event) {
		const value = (event.currentTarget as HTMLInputElement).value;
		activeIndex = -1;
		if (!value.trim()) {
			searchQuery.set('');
			suggestionsOpen = false;
		} else {
			suggestionsOpen = true;
		}
	}

	function selectSuggestion(item: Suggestion) {
		if (item.kind === 'game') {
			searchQuery.set('');
			achievementJumpIntent.set(null);
			selectedGameId.set(item.gameId);
			void goto(resolve('/(app)/games/[id]', { id: item.gameId }));
		} else {
			searchQuery.set(item.query);
			achievementJumpIntent.set({
				gameId: item.gameId,
				achievementKey: item.achievementKey ?? item.query,
				token: Date.now()
			});
			selectedGameId.set(item.gameId);
			void goto(resolve('/(app)/games/[id]', { id: item.gameId }));
		}

		searchDraft.set('');
		suggestionsOpen = false;
		activeIndex = -1;
	}

	function scrollActiveSuggestionIntoView(index: number) {
		if (typeof document === 'undefined' || index < 0) return;
		requestAnimationFrame(() => {
			document.getElementById(`search-suggestion-${index}`)?.scrollIntoView({
				block: 'nearest'
			});
		});
	}

	function onSearchKeydown(event: KeyboardEvent) {
		if (event.key === 'ArrowDown') {
			if (!normalizedDraft || suggestions.length === 0) return;
			event.preventDefault();
			if (!suggestionsOpen) {
				suggestionsOpen = true;
				activeIndex = 0;
			} else {
				activeIndex = activeIndex < suggestions.length - 1 ? activeIndex + 1 : 0;
			}
			scrollActiveSuggestionIntoView(activeIndex);
			return;
		}

		if (event.key === 'ArrowUp') {
			if (!normalizedDraft || suggestions.length === 0) return;
			event.preventDefault();
			if (!suggestionsOpen) {
				suggestionsOpen = true;
				activeIndex = suggestions.length - 1;
			} else {
				activeIndex = activeIndex > 0 ? activeIndex - 1 : suggestions.length - 1;
			}
			scrollActiveSuggestionIntoView(activeIndex);
			return;
		}

		if (event.key === 'Enter') {
			event.preventDefault();
			if (suggestions.length > 0 && suggestionsOpen) {
				const target =
					activeIndex >= 0 && activeIndex < suggestions.length
						? suggestions[activeIndex]
						: suggestions[0];
				selectSuggestion(target);
			} else {
				applySearch();
				if (!$searchDraft.trim()) searchQuery.set('');
				suggestionsOpen = false;
				activeIndex = -1;
			}
			return;
		}

		if (event.key === 'Escape') {
			suggestionsOpen = false;
			activeIndex = -1;
			inputEl?.blur();
		}
	}

	function openRunningGame() {
		const gameId = $runningGame?.game_id;
		if (!gameId) return;
		selectedGameId.set(gameId);
		void goto(resolve('/(app)/games/[id]', { id: gameId }));
	}

	async function handleSync() {
		if ($isSyncing) return;
		await syncSteamMetadata();
	}

	onMount(() => {
		const onKeyDown = (event: KeyboardEvent) => {
			if ((event.ctrlKey || event.metaKey) && event.key.toLowerCase() === 'k') {
				event.preventDefault();
				focusSearch();
			}
		};

		const onPointerDown = (event: MouseEvent) => {
			const target = event.target as Node;
			if (searchWrapEl && !searchWrapEl.contains(target)) {
				suggestionsOpen = false;
				activeIndex = -1;
			}
		};

		window.addEventListener('keydown', onKeyDown);
		window.addEventListener('mousedown', onPointerDown);
		return () => {
			window.removeEventListener('keydown', onKeyDown);
			window.removeEventListener('mousedown', onPointerDown);
		};
	});
</script>

<header class="topbar">
	<div class="topbar-search-wrap" bind:this={searchWrapEl}>
		<div class="topbar-search" role="search">
			<svg
				width="14"
				height="14"
				viewBox="0 0 24 24"
				fill="none"
				stroke="currentColor"
				stroke-width="2"
				aria-hidden="true"
			>
				<circle cx="11" cy="11" r="8" /><line x1="21" y1="21" x2="16.65" y2="16.65" />
			</svg>
			<input
				class="search-input"
				type="text"
				role="combobox"
				aria-autocomplete="list"
				aria-expanded={suggestionsOpen && Boolean(normalizedDraft)}
				aria-controls="topbar-search-listbox"
				aria-activedescendant={activeDescendantId}
				bind:this={inputEl}
				bind:value={$searchDraft}
				placeholder={searchPlaceholder}
				onfocus={focusSearch}
				oninput={onSearchInput}
				onkeydown={onSearchKeydown}
			/>
			<kbd class="search-shortcut tabular-nums">Ctrl+K</kbd>
		</div>

		{#if suggestionsOpen && normalizedDraft}
			<div
				id="topbar-search-listbox"
				class="search-suggestions"
				role="listbox"
				aria-label={$i18n.t('topbar.searchSuggestions')}
			>
				{#if suggestions.length === 0}
					<div class="suggestion-empty">{$i18n.t('topbar.noSuggestion')}</div>
				{:else}
					{#each suggestions as item, index (`${item.kind}:${item.gameId}:${item.achievementKey ?? item.title}:${index}`)}
						<button
							id="search-suggestion-{index}"
							class="suggestion-item"
							class:game={item.kind === 'game'}
							class:achievement={item.kind === 'achievement'}
							class:active={activeIndex === index}
							role="option"
							aria-selected={activeIndex === index}
							type="button"
							onmouseenter={() => (activeIndex = index)}
							onclick={() => selectSuggestion(item)}
						>
							<div
								class="suggestion-thumb"
								class:game={item.kind === 'game'}
								class:achievement={item.kind === 'achievement'}
								class:with-image={Boolean(item.iconUrl)}
							>
								{#if item.iconUrl}
									<img
										src={item.iconUrl}
										alt={item.title}
										onerror={(event) => {
											const target = event.currentTarget as HTMLImageElement;
											target.style.display = 'none';
											target.parentElement?.classList.add('no-image');
										}}
									/>
								{/if}
								<span class="suggestion-fallback" aria-hidden="true">
									{#if item.kind === 'game'}
										<svg
											width="18"
											height="18"
											viewBox="0 0 24 24"
											fill="none"
											stroke="currentColor"
											stroke-width="1.8"
											stroke-linecap="round"
											stroke-linejoin="round"
											aria-hidden="true"
										>
											<line x1="6" y1="11" x2="10" y2="11" />
											<line x1="8" y1="9" x2="8" y2="13" />
											<line x1="15" y1="12" x2="15.01" y2="12" />
											<line x1="18" y1="10" x2="18.01" y2="10" />
											<path
												d="M17.32 5H6.68a4 4 0 0 0-3.978 3.59c-.006.052-.01.101-.017.152C2.604 9.416 2 14.456 2 16a3 3 0 0 0 3 3c1 0 1.5-.5 2-1l1.414-1.414A2 2 0 0 1 9.828 16h4.344a2 2 0 0 1 1.414.586L17 18c.5.5 1 1 2 1a3 3 0 0 0 3-3c0-1.545-.604-6.584-.685-7.258-.007-.05-.011-.1-.017-.151A4 4 0 0 0 17.32 5z"
											/>
										</svg>
									{:else}
										<svg
											width="18"
											height="18"
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
								</span>
							</div>
							<span class="suggestion-content">
								<span class="suggestion-kind">
									{item.kind === 'game'
										? $i18n.t('topbar.kind.game')
										: $i18n.t('topbar.kind.achievement')}
								</span>
								<span class="suggestion-title">{item.title}</span>
								<span class="suggestion-subtitle">{item.subtitle}</span>
							</span>
						</button>
					{/each}
				{/if}
			</div>
		{/if}
	</div>

	<div class="topbar-right">
		{#if $runningGame}
			<button
				class="playing-pill"
				type="button"
				onclick={openRunningGame}
				disabled={!$runningGame.game_id}
				title={$i18n.t('topbar.playingTitle', { name: $runningGame.name })}
			>
				<span class="playing-dot" aria-hidden="true"></span>
				<span class="playing-label">{$i18n.t('topbar.playing')}</span>
				<span class="playing-name">{$runningGame.name}</span>
			</button>
		{/if}

		<button
			class="sync-btn"
			class:syncing={$isSyncing}
			type="button"
			onclick={handleSync}
			title={$i18n.t('topbar.syncTitle')}
			disabled={$isSyncing}
		>
			<svg
				width="14"
				height="14"
				viewBox="0 0 24 24"
				fill="none"
				stroke="currentColor"
				stroke-width="2.4"
				aria-hidden="true"
			>
				<path d="M21 2v6h-6" /><path d="M3 12a9 9 0 0 1 15-6.7L21 8" /><path d="M3 22v-6h6" /><path
					d="M21 12a9 9 0 0 1-15 6.7L3 16"
				/>
			</svg>
			{#if $isSyncing}
				<span>{$i18n.t('topbar.syncing')}</span>
			{:else}
				<span>{$i18n.t('topbar.sync')}</span>
			{/if}
		</button>

		<span class="watching-badge tabular-nums" class:active={$watcherActive}>
			<span class="watching-dot" aria-hidden="true"></span>{$i18n.t('topbar.scan')}
		</span>
	</div>
</header>

<style>
	.topbar {
		grid-column: 2 / -1;
		grid-row: 1 / 2;
		background: var(--bg-panel);
		backdrop-filter: blur(20px) saturate(180%);
		-webkit-backdrop-filter: blur(20px) saturate(180%);
		border: 1px solid var(--border-specular);
		border-radius: 16px;
		box-shadow: var(--shadow-card);
		padding: 0 16px;
		height: 100%;
		display: flex;
		align-items: center;
		gap: 12px;
	}

	.topbar-search {
		width: 100%;
		height: 36px;
		background: var(--surface-2);
		border: 1px solid var(--border-soft);
		border-radius: 10px;
		display: flex;
		align-items: center;
		gap: 8px;
		padding: 0 12px;
		cursor: text;
		text-align: left;
		color: var(--text-muted);
		transition:
			border-color 0.2s ease,
			background 0.2s ease,
			box-shadow 0.2s ease;
	}

	.topbar-search:focus-within {
		background: var(--surface-1);
		border-color: color-mix(in srgb, var(--accent) 45%, var(--border-soft));
		box-shadow: 0 0 0 2px color-mix(in srgb, var(--accent) 22%, transparent);
	}

	.topbar-search-wrap {
		flex: 1;
		max-width: 620px;
		margin: 0 auto;
		position: relative;
	}

	.search-input {
		flex: 1;
		border: none;
		background: transparent;
		color: var(--text-primary);
		font-size: 13px;
		outline: none;
		box-shadow: none;
		font-family: inherit;
	}

	.search-input::placeholder {
		color: var(--text-muted);
	}

	.search-shortcut {
		font-size: 10px;
		font-weight: 600;
		color: var(--text-muted);
		background: var(--surface-1);
		border: 1px solid var(--border-soft);
		border-radius: 5px;
		padding: 2px 6px;
		font-family: inherit;
	}

	.search-suggestions {
		position: absolute;
		top: calc(100% + 8px);
		left: 0;
		right: 0;
		background: var(--glass-bg, var(--surface-1));
		backdrop-filter: blur(20px) saturate(180%);
		-webkit-backdrop-filter: blur(20px) saturate(180%);
		border: 1px solid var(--border-specular);
		border-radius: 14px;
		padding: 6px;
		display: flex;
		flex-direction: column;
		gap: 4px;
		max-height: 440px;
		overflow-y: auto;
		z-index: 120;
		box-shadow: var(--shadow-elevated);
	}

	.suggestion-item {
		width: 100%;
		border: 1px solid transparent;
		background: transparent;
		border-radius: 10px;
		color: var(--text-primary);
		display: flex;
		align-items: center;
		gap: 10px;
		padding: 8px;
		text-align: left;
		cursor: pointer;
		transition:
			background 0.14s ease,
			border-color 0.14s ease;
	}

	.suggestion-item:hover {
		background: var(--surface-hover);
		border-color: color-mix(in srgb, var(--accent) 35%, var(--border-soft));
	}

	.suggestion-item.active {
		background: color-mix(in srgb, var(--accent) 14%, var(--surface-2));
		border-color: color-mix(in srgb, var(--accent) 35%, var(--border-soft));
	}

	.suggestion-item.game {
		min-height: 56px;
	}

	.suggestion-item.achievement {
		min-height: 52px;
	}

	.suggestion-thumb {
		flex-shrink: 0;
		overflow: hidden;
		display: flex;
		align-items: center;
		justify-content: center;
		background: var(--surface-3);
	}

	.suggestion-thumb.game {
		width: 76px;
		height: 42px;
		border-radius: 8px;
		border: 1px solid var(--border-soft);
	}

	.suggestion-thumb.achievement {
		width: 42px;
		height: 42px;
		border-radius: 10px;
		border: 1px solid var(--border-soft);
	}

	.suggestion-thumb img {
		width: 100%;
		height: 100%;
		object-fit: cover;
	}

	.suggestion-fallback {
		display: flex;
		align-items: center;
		justify-content: center;
		color: var(--text-muted);
	}

	.suggestion-thumb.with-image:not(.no-image) .suggestion-fallback {
		display: none;
	}

	.suggestion-kind {
		font-size: 9px;
		font-weight: 600;
		text-transform: uppercase;
		letter-spacing: 0.06em;
		color: var(--text-muted);
		border: 1px solid var(--border-soft);
		background: var(--surface-2);
		border-radius: 5px;
		padding: 1px 6px;
		width: fit-content;
	}

	.suggestion-content {
		min-width: 0;
		display: flex;
		flex-direction: column;
		gap: 2px;
	}

	.suggestion-title {
		font-size: 13px;
		font-weight: 600;
		color: var(--text-primary);
		white-space: nowrap;
		overflow: hidden;
		text-overflow: ellipsis;
	}

	.suggestion-subtitle {
		font-size: 11px;
		color: var(--text-secondary);
		white-space: nowrap;
		overflow: hidden;
		text-overflow: ellipsis;
	}

	.suggestion-empty {
		font-size: 12px;
		color: var(--text-muted);
		padding: 10px 12px;
	}

	.topbar-right {
		display: flex;
		align-items: center;
		gap: 10px;
		flex-shrink: 0;
	}

	.sync-btn {
		height: 34px;
		padding: 0 13px;
		border-radius: 999px;
		border: 1px solid var(--border-soft);
		background: var(--surface-2);
		color: var(--text-secondary);
		font-size: 12px;
		font-weight: 600;
		font-variant-numeric: tabular-nums;
		display: flex;
		align-items: center;
		gap: 7px;
		cursor: pointer;
		transition:
			background 0.18s ease,
			border-color 0.18s ease,
			color 0.18s ease;
		font-family: inherit;
	}

	.sync-btn:hover:not(:disabled) {
		background: var(--surface-hover);
		border-color: color-mix(in srgb, var(--accent) 35%, var(--border-soft));
		color: var(--text-primary);
	}

	.sync-btn.syncing {
		color: var(--accent, #c8a96e);
		border-color: color-mix(in srgb, var(--accent) 35%, var(--border-soft));
		background: color-mix(in srgb, var(--accent) 10%, var(--surface-2));
	}

	.sync-btn.syncing svg {
		animation: spin 1.2s linear infinite;
	}

	.sync-btn:disabled {
		cursor: wait;
		opacity: 0.85;
	}

	@keyframes spin {
		from {
			transform: rotate(0deg);
		}
		to {
			transform: rotate(360deg);
		}
	}

	.playing-pill {
		height: 34px;
		max-width: 240px;
		padding: 0 13px;
		border-radius: 999px;
		border: 1px solid color-mix(in srgb, var(--accent) 38%, var(--border-soft));
		background: color-mix(in srgb, var(--accent) 12%, var(--surface-2));
		color: var(--text-primary);
		font-size: 12px;
		font-variant-numeric: tabular-nums;
		font-family: inherit;
		display: flex;
		align-items: center;
		gap: 7px;
		cursor: pointer;
		transition:
			background 0.18s ease,
			border-color 0.18s ease;
	}

	.playing-pill:hover:not(:disabled) {
		background: color-mix(in srgb, var(--accent) 20%, var(--surface-2));
	}

	.playing-pill:disabled {
		cursor: default;
	}

	.playing-dot {
		width: 7px;
		height: 7px;
		border-radius: 50%;
		background: var(--accent);
		flex-shrink: 0;
		animation: playing-pulse 1.6s ease-in-out infinite;
	}

	.playing-label {
		color: var(--text-muted);
		flex-shrink: 0;
	}

	.playing-name {
		font-weight: 600;
		white-space: nowrap;
		overflow: hidden;
		text-overflow: ellipsis;
	}

	@keyframes playing-pulse {
		0%,
		100% {
			opacity: 1;
		}
		50% {
			opacity: 0.35;
		}
	}

	.watching-badge {
		height: 34px;
		padding: 0 12px;
		border-radius: 999px;
		border: 1px solid var(--border-soft);
		background: var(--surface-2);
		display: flex;
		align-items: center;
		gap: 7px;
		white-space: nowrap;
		flex-shrink: 0;
		color: var(--text-muted);
		font-size: 12px;
		font-weight: 600;
		transition:
			color 0.25s ease,
			background 0.25s ease,
			border-color 0.25s ease;
	}

	.watching-dot {
		width: 7px;
		height: 7px;
		border-radius: 50%;
		background: currentColor;
		flex-shrink: 0;
		transition: background 0.3s ease;
	}

	.watching-badge.active {
		color: #3ddc84;
		background: color-mix(in srgb, #3ddc84 12%, var(--surface-2));
		border-color: color-mix(in srgb, #3ddc84 32%, var(--border-soft));
	}

	:global([data-theme='light']) .watching-badge.active {
		color: #15803d;
		background: color-mix(in srgb, #16a34a 12%, var(--surface-2));
		border-color: color-mix(in srgb, #16a34a 32%, var(--border-soft));
	}
</style>
