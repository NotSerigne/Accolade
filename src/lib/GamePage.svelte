<script lang="ts">
	// src/lib/GamePage.svelte
	import { onMount, tick } from 'svelte';
	import { listen } from '@tauri-apps/api/event';
	import { loadAchievements, type Game, type Achievement, games } from '$lib/stores/Games.js';
	import { searchQuery, achievementJumpIntent } from '$lib/stores/ui.js';
	import { goto } from '$app/navigation';
	import { resolve } from '$app/paths';
	import { open } from '@tauri-apps/plugin-shell';
	import { SvelteMap } from 'svelte/reactivity';
	import { i18n, rarityLabelByIndex } from '$lib/stores/i18n.js';

	let { game }: { game: Game | null } = $props();

	let currentIndex = $derived($games.findIndex((g) => g.steam_id === game?.steam_id));
	let prevGame = $derived(currentIndex > 0 ? $games[currentIndex - 1] : null);
	let nextGame = $derived(currentIndex < $games.length - 1 ? $games[currentIndex + 1] : null);

	function goToGame(id: number) {
		void goto(resolve('/(app)/games/[id]', { id: String(id) }));
	}

	let copiedKey = $state<string | null>(null);

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

	type AchievementsUpdatedPayload = {
		steam_id: number;
		achievements: Achievement[];
	};

	let achievements = $state<Achievement[]>([]);
	let filter = $state<'all' | 'unlocked' | 'locked'>('all');
	let search = $state('');
	let sort = $state<'date' | 'rarity' | 'name'>('date');
	let sortOrder = $state<'asc' | 'desc'>('desc');
	let revealed = $state(true);
	let loading = $state(true);
	let loadedGameId = $state<number | null>(null);
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

		if (loadedGameId === game.steam_id) return;

		loadedGameId = game.steam_id;
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

		listen<AchievementsUpdatedPayload>('achievements-updated', (event) => {
			const payload = event.payload;
			if (game && payload?.steam_id === game.steam_id && Array.isArray(payload.achievements)) {
				achievements = payload.achievements;
			}
		}).then((fn) => {
			unlisten = fn;
		});

		return () => {
			unlisten?.();
		};
	});

	let merged = $derived.by((): Achievement[] => {
		if (!game) return [];
		const schema = game.achievements ?? [];
		const stateMap = new Map<string, Achievement>(
			achievements.map((a) => [a.key.trim().toLowerCase(), a])
		);

		return schema.map((s: Achievement) => {
			const live = stateMap.get(s.key.trim().toLowerCase());
			return {
				...s,
				unlocked: live?.unlocked ?? s.unlocked ?? false,
				unlocked_time: live?.unlocked_time ?? s.unlocked_time ?? null,
				rarity: live?.rarity || s.rarity || '',
				completionpercentage: live?.completionpercentage || s.completionpercentage || '',
				desc: live?.desc || s.desc || ''
			};
		});
	});

	let globalAchievementHighlight = $derived($searchQuery.trim().toLowerCase());

	let filtered = $derived.by((): Achievement[] => {
		let list: Achievement[] = merged;

		if (filter === 'unlocked') list = list.filter((a) => a.unlocked);
		if (filter === 'locked') list = list.filter((a) => !a.unlocked);

		const localQuery = search.trim().toLowerCase();
		if (localQuery) {
			list = list.filter(
				(a) =>
					a.name.toLowerCase().includes(localQuery) ||
					(a.desc ?? '').toLowerCase().includes(localQuery)
			);
		}

		list = [...list];

		const orderMultiplier = sortOrder === 'asc' ? 1 : -1;

		if (sort === 'date') {
			list.sort((a: Achievement, b: Achievement) => {
				if (a.unlocked && b.unlocked) {
					return ((a.unlocked_time ?? 0) - (b.unlocked_time ?? 0)) * orderMultiplier;
				}
				if (a.unlocked) return -1;
				if (b.unlocked) return 1;
				return 0;
			});
		} else if (sort === 'rarity') {
			list.sort(
				(a: Achievement, b: Achievement) =>
					((parseFloat(a.completionpercentage) || 100) -
						(parseFloat(b.completionpercentage) || 100)) *
					orderMultiplier
			);
		} else if (sort === 'name') {
			list.sort((a: Achievement, b: Achievement) => a.name.localeCompare(b.name) * orderMultiplier);
		}

		return list;
	});

	let unlockedCount = $derived(merged.filter((a) => a.unlocked).length);
	let totalCount = $derived(merged.length);
	let progressPct = $derived(totalCount > 0 ? Math.round((unlockedCount / totalCount) * 100) : 0);

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
		}, 850);
	}

	function delay(ms: number): Promise<void> {
		return new Promise((resolve) => setTimeout(resolve, ms));
	}

	$effect(() => {
		const intent = $achievementJumpIntent;
		if (!intent || !game) return;
		if (intent.gameId !== game.steam_id) return;
		if (jumpedToken === intent.token) return;

		jumpedToken = intent.token;
		const normalizedKey = normalizeAchievementKey(intent.achievementKey);

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

	function formatDate(ts: number | null): string {
		if (!ts) return '';
		return new Date(ts * 1000).toLocaleDateString('fr-FR', {
			day: '2-digit',
			month: '2-digit',
			year: 'numeric'
		});
	}

	function rarityLabel(pct: string): string {
		const n = parseFloat(pct);
		if (isNaN(n)) return '';
		if (n <= 0.1) return rarityLabelByIndex($i18n.language, 0);
		if (n <= 1) return rarityLabelByIndex($i18n.language, 1);
		if (n <= 3) return rarityLabelByIndex($i18n.language, 2);
		if (n <= 7) return rarityLabelByIndex($i18n.language, 3);
		if (n <= 15) return rarityLabelByIndex($i18n.language, 4);
		if (n <= 35) return rarityLabelByIndex($i18n.language, 5);
		return rarityLabelByIndex($i18n.language, 6);
	}

	type RarityPalette = {
		border: string;
		shadow: string;
		badgeBg: string;
		badgeColor: string;
		fill: string;
	};

	function rarityPalette(pct: string): RarityPalette {
		const n = parseFloat(pct);
		if (isNaN(n)) {
			return {
				border: 'rgba(255,255,255,0.06)',
				shadow: 'none',
				badgeBg: 'rgba(255,255,255,0.05)',
				badgeColor: '#6a7080',
				fill: '#4a5060'
			};
		}

		if (n <= 0.1) {
			return {
				border: '#ff3b5c',
				shadow: '0 0 10px rgba(255,59,92,0.15)',
				badgeBg: 'rgba(255,59,92,0.14)',
				badgeColor: '#ff3b5c',
				fill: '#ff3b5c'
			};
		}
		if (n <= 1) {
			return {
				border: '#ffd85a',
				shadow: '0 0 10px rgba(255,216,90,0.12)',
				badgeBg: 'rgba(255,216,90,0.14)',
				badgeColor: '#ffd85a',
				fill: '#ffd85a'
			};
		}
		if (n <= 3) {
			return {
				border: '#a855f7',
				shadow: '0 0 10px rgba(168,85,247,0.15)',
				badgeBg: 'rgba(168,85,247,0.14)',
				badgeColor: '#a855f7',
				fill: '#a855f7'
			};
		}
		if (n <= 7) {
			return {
				border: '#f4b860',
				shadow: '0 0 10px rgba(244,184,96,0.12)',
				badgeBg: 'rgba(244,184,96,0.14)',
				badgeColor: '#f4b860',
				fill: '#f4b860'
			};
		}
		if (n <= 15) {
			return {
				border: '#4ac8ff',
				shadow: '0 0 10px rgba(74,200,255,0.15)',
				badgeBg: 'rgba(74,200,255,0.1)',
				badgeColor: '#4ac8ff',
				fill: '#4ac8ff'
			};
		}
		if (n <= 35) {
			return {
				border: '#3ddc84',
				shadow: '0 0 10px rgba(61,220,132,0.12)',
				badgeBg: 'rgba(61,220,132,0.12)',
				badgeColor: '#3ddc84',
				fill: '#3ddc84'
			};
		}

		return {
			border: 'rgba(255,255,255,0.06)',
			shadow: 'none',
			badgeBg: 'rgba(255,255,255,0.05)',
			badgeColor: '#6a7080',
			fill: '#4a5060'
		};
	}

	function gameTitle(current: Game): string {
		return current.name?.trim() || `AppID ${current.steam_id}`;
	}

	function heroBackground(current: Game): string {
		return `https://cdn.cloudflare.steamstatic.com/steam/apps/${current.steam_id}/library_hero.jpg`;
	}
</script>

{#if !game}
	<div class="not-found">Jeu introuvable.</div>
{:else}
	<div class="game-page" style:--game-bg={`url('${heroBackground(game)}')`}>
		<div class="bg-overlay"></div>

		<div class="game-nav">
			<button
				class="nav-btn"
				disabled={!prevGame}
				onclick={() => prevGame && goToGame(prevGame.steam_id)}
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
				<span class="nav-label">{prevGame ? prevGame.name || prevGame.steam_id : ''}</span>
			</button>

			<span class="nav-index">{currentIndex + 1} / {$games.length}</span>

			<button
				class="nav-btn"
				disabled={!nextGame}
				onclick={() => nextGame && goToGame(nextGame.steam_id)}
			>
				<span class="nav-label">{nextGame ? nextGame.name || nextGame.steam_id : ''}</span>
				<svg
					width="12"
					height="12"
					viewBox="0 0 24 24"
					fill="none"
					stroke="currentColor"
					stroke-width="3"
				>
					<path d="M9 18l6-6 6-6" />
				</svg>
			</button>
		</div>

		<div class="game-scroll-area">
			<div class="content-wrapper">
				<div class="game-header">
					<div class="game-title-row">
						<h1 class="game-title">{gameTitle(game)}</h1>
						<span class="emulator-badge">{game.emulator}</span>
					</div>

					<div class="progress-card">
						<div class="progress-left">
							<div class="progress-stats">
								<span class="main-count"
									><strong>{unlockedCount}</strong> <span class="slash">/ {totalCount}</span></span
								>
								<span class="main-pct">{progressPct}%</span>
							</div>
							<div class="progress-bar-track">
								<div class="progress-bar-fill" style="width: {progressPct}%"></div>
							</div>
						</div>
						<div class="progress-right">
							<div class="stat-item">
								<span class="stat-label">TOTAL</span>
								<span class="stat-value">{totalCount}</span>
							</div>
							<div class="stat-item">
								<span class="stat-label">DÉBLOQUÉS</span>
								<span class="stat-value highlight">{unlockedCount}</span>
							</div>
						</div>
					</div>
				</div>

				<div class="main-section">
					<div class="controls-bar">
						<div class="filter-tabs">
							<button class="tab" class:active={filter === 'all'} onclick={() => (filter = 'all')}
								>Tout</button
							>
							<button
								class="tab"
								class:active={filter === 'unlocked'}
								onclick={() => (filter = 'unlocked')}>Débloqués</button
							>
							<button
								class="tab"
								class:active={filter === 'locked'}
								onclick={() => (filter = 'locked')}>Verrouillés</button
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
								placeholder="Chercher un  fa..."
								bind:value={search}
							/>
							<span class="search-count">{filtered.length} / {totalCount}</span>
						</div>

						<select class="sort-select" bind:value={sort}>
							<option value="date">Date d'obtention</option>
							<option value="rarity">Rareté</option>
							<option value="name">Nom</option>
						</select>

						<button
							class="order-btn"
							onclick={() => (sortOrder = sortOrder === 'asc' ? 'desc' : 'asc')}
							title={sortOrder === 'asc' ? 'Croissant' : 'Décroissant'}
						>
							{#if sortOrder === 'asc'}
								<svg
									width="14"
									height="14"
									viewBox="0 0 24 24"
									fill="none"
									stroke="currentColor"
									stroke-width="2.5"
								>
									<path d="M12 19V5M5 12l7-7 7 7" />
								</svg>
							{:else}
								<svg
									width="14"
									height="14"
									viewBox="0 0 24 24"
									fill="none"
									stroke="currentColor"
									stroke-width="2.5"
								>
									<path d="M12 5v14M5 12l7 7 7-7" />
								</svg>
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
								<span>Révélés</span>
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
								<span>Cachés</span>
							{/if}
						</button>
					</div>

					<!-- Liste -->
					<div class="achievements-list">
						{#if loading}
							<div class="loading-state">Chargement…</div>
						{:else}
							<div class="ach-grid">
								{#each filtered as ach (ach.key)}
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
									{@const isTopbarMatch =
										globalAchievementHighlight &&
										(ach.name.toLowerCase().includes(globalAchievementHighlight) ||
											(ach.desc ?? '').toLowerCase().includes(globalAchievementHighlight))}

									<div
										class="ach-card"
										class:is-unlocked={isUnlocked}
										class:is-secret-hidden={showMaskedDescription}
										class:highlighted={Boolean(isTopbarMatch)}
										class:jump-flash={flashAchievementKey === achNormalizedKey}
										style:border-color={palette.border}
										style:box-shadow={palette.shadow}
										use:trackAchievementRow={ach.key}
									>
										<div class="ach-card-inner">
											<div class="ach-icon-box">
												{#if displayIcon}
													<img src={displayIcon} alt={ach.name} class:muted-icon={isLocked} />
												{:else}
													<span class="ach-placeholder">🏆</span>
												{/if}
											</div>

											<div class="ach-details">
												<div class="ach-header-row">
													<div class="ach-name" class:muted={isLocked}>
														{isLocked && isSecret && !revealed
															? 'Succès caché'
															: ach.name || ach.key}
													</div>
													{#if isUnlocked && ach.unlocked_time}
														<div class="ach-date">{formatDate(ach.unlocked_time)}</div>
													{/if}
												</div>

												<div class="ach-desc-row">
													{#if showDescription}
														<div class="ach-desc">{descText}</div>
													{:else if showMaskedDescription}
														<div class="ach-desc italic">Description masquée</div>
													{/if}
												</div>

												<div class="ach-footer-row">
													{#if ach.completionpercentage}
														<div class="rarity-info">
															<span
																class="rarity-tag"
																style:background={palette.badgeBg}
																style:color={palette.badgeColor}
															>
																{rarityLabel(ach.completionpercentage)} | {parseFloat(
																	ach.completionpercentage
																).toFixed(1)}%
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
																Copié
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
																Copier le nom
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
															Chercher un guide
														</button>
													</div>
												</div>
											</div>
										</div>
									</div>
								{/each}
							</div>

							{#if filtered.length === 0}
								<div class="empty-state">Aucun succès trouvé.</div>
							{/if}
						{/if}
					</div>
				</div>
			</div>
		</div>
	</div>
{/if}

<style>
	.game-page {
		height: 100%;
		display: flex;
		flex-direction: column;
		overflow: hidden;
		background-color: #080a0f;
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
		scrollbar-color: rgba(255, 255, 255, 0.15) transparent;
	}

	.game-scroll-area::-webkit-scrollbar {
		width: 6px;
	}
	.game-scroll-area::-webkit-scrollbar-thumb {
		background: rgba(255, 255, 255, 0.15);
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
			rgba(8, 10, 15, 0.2) 0%,
			rgba(8, 10, 15, 0.85) 35%,
			rgba(8, 10, 15, 0.98) 100%
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
		color: #6a7080;
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
		background: rgba(8, 10, 15, 0.5);
		backdrop-filter: blur(16px);
		border-bottom: 1px solid rgba(255, 255, 255, 0.03);
	}

	.nav-btn {
		display: flex;
		align-items: center;
		gap: 8px;
		background: rgba(255, 255, 255, 0.04);
		border: 1px solid rgba(255, 255, 255, 0.06);
		border-radius: 8px;
		padding: 6px 14px;
		color: #888;
		font-size: 11px;
		font-weight: 600;
		cursor: pointer;
		transition: all 0.2s;
		max-width: 180px;
	}
	.nav-btn:hover:not(:disabled) {
		background: rgba(255, 255, 255, 0.08);
		color: #fff;
		border-color: rgba(255, 255, 255, 0.12);
	}
	.nav-btn:disabled {
		opacity: 0.2;
		cursor: default;
	}
	.nav-label {
		white-space: nowrap;
		overflow: hidden;
		text-overflow: ellipsis;
	}
	.nav-index {
		font-size: 12px;
		color: #444;
		font-weight: 700;
		letter-spacing: 0.5px;
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
		letter-spacing: -1px;
	}
	.emulator-badge {
		font-size: 10px;
		font-weight: 700;
		color: #fff;
		background: rgba(255, 255, 255, 0.1);
		border: 1px solid rgba(255, 255, 255, 0.1);
		border-radius: 20px;
		padding: 4px 14px;
		text-transform: uppercase;
		letter-spacing: 1.2px;
	}

	.progress-card {
		background: rgba(13, 20, 28, 0.85);
		border: 1px solid rgba(255, 255, 255, 0.04);
		border-radius: 16px;
		padding: 24px 32px;
		display: flex;
		gap: 48px;
		backdrop-filter: blur(12px);
		box-shadow: 0 8px 32px rgba(0, 0, 0, 0.4);
	}
	.progress-left {
		flex: 1;
		display: flex;
		flex-direction: column;
		gap: 16px;
	}
	.progress-stats {
		display: flex;
		align-items: baseline;
		justify-content: space-between;
	}
	.main-count {
		font-size: 32px;
		font-weight: 800;
		color: #fff;
	}
	.slash {
		color: #444;
		font-size: 20px;
		font-weight: 600;
		margin-left: 6px;
	}
	.main-pct {
		font-size: 32px;
		font-weight: 900;
		color: var(--accent, #00e5ff);
	}
	.progress-bar-track {
		height: 10px;
		background: rgba(255, 255, 255, 0.05);
		border-radius: 5px;
		overflow: hidden;
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
		border-left: 1px solid rgba(255, 255, 255, 0.08);
		padding-left: 48px;
	}
	.stat-item {
		display: flex;
		flex-direction: column;
		gap: 6px;
	}
	.stat-label {
		font-size: 11px;
		font-weight: 700;
		color: #6a7080;
		letter-spacing: 1.5px;
	}
	.stat-value {
		font-size: 24px;
		font-weight: 800;
		color: #fff;
	}
	.stat-value.highlight {
		color: #3ddc84;
	}

	.main-section {
		flex: 1;
		display: flex;
		flex-direction: column;
		overflow: hidden;
		background: rgba(13, 20, 28, 0.45);
		border: 1px solid rgba(255, 255, 255, 0.03);
		border-radius: 20px 20px 0 0;
		backdrop-filter: blur(8px);
		min-width: 0;
	}

	.controls-bar {
		display: flex;
		align-items: center;
		gap: 14px;
		padding: 20px 28px;
		border-bottom: 1px solid rgba(255, 255, 255, 0.04);
		flex-shrink: 0;
		overflow-x: auto;
		scrollbar-width: none;
	}
	.controls-bar::-webkit-scrollbar {
		display: none;
	}

	.filter-tabs {
		display: flex;
		background: rgba(0, 0, 0, 0.3);
		border-radius: 10px;
		padding: 4px;
		gap: 4px;
	}
	.tab {
		padding: 8px 18px;
		border-radius: 8px;
		border: none;
		background: transparent;
		cursor: pointer;
		font-size: 13px;
		color: #6a7080;
		font-weight: 600;
		transition: all 0.25s;
	}
	.tab.active {
		background: var(--accent, #0066cc);
		color: #fff;
		box-shadow: 0 4px 12px color-mix(in srgb, var(--accent, #0066cc) 40%, transparent);
	}
	.tab:hover:not(.active) {
		color: #fff;
		background: rgba(255, 255, 255, 0.05);
	}

	.search-wrap {
		flex: 1;
		max-width: 440px;
		height: 38px;
		background: rgba(0, 0, 0, 0.3);
		border: 1px solid rgba(255, 255, 255, 0.05);
		border-radius: 10px;
		display: flex;
		align-items: center;
		gap: 12px;
		padding: 0 14px;
	}
	.search-input {
		flex: 1;
		background: transparent;
		border: none;
		outline: none;
		font-size: 13px;
		color: #fff;
	}
	.search-input::placeholder {
		color: #333;
	}
	.search-count {
		font-size: 12px;
		color: #333;
		font-weight: 700;
	}

	.sort-select {
		height: 38px;
		background: rgba(255, 255, 255, 0.05);
		border: 1px solid rgba(255, 255, 255, 0.1);
		border-radius: 10px;
		padding: 0 14px;
		font-size: 13px;
		color: #fff;
		outline: none;
		cursor: pointer;
		color-scheme: dark;
		transition:
			border-color 0.2s,
			background 0.2s;
	}
	.sort-select:hover {
		background: rgba(255, 255, 255, 0.08);
	}
	.sort-select:focus {
		border-color: var(--accent, #0066cc);
	}
	.sort-select option {
		background: #1a1a1a;
		color: #fff;
	}

	.order-btn {
		height: 38px;
		width: 38px;
		display: flex;
		align-items: center;
		justify-content: center;
		background: rgba(255, 255, 255, 0.05);
		border: 1px solid rgba(255, 255, 255, 0.1);
		border-radius: 10px;
		color: #6a7080;
		cursor: pointer;
		transition: all 0.2s;
	}
	.order-btn:hover {
		background: rgba(255, 255, 255, 0.08);
		color: #fff;
	}

	.reveal-btn {
		height: 38px;
		padding: 0 16px;
		background: rgba(255, 255, 255, 0.03);
		border: 1px solid rgba(255, 255, 255, 0.08);
		border-radius: 10px;
		font-size: 13px;
		color: #6a7080;
		cursor: pointer;
		display: flex;
		align-items: center;
		gap: 8px;
		transition: all 0.2s;
	}
	.reveal-btn.active {
		background: color-mix(in srgb, var(--accent, #0066cc) 12%, transparent);
		border-color: color-mix(in srgb, var(--accent, #0066cc) 30%, transparent);
		color: var(--accent, #00e5ff);
	}
	.reveal-btn span {
		font-weight: 600;
	}

	.achievements-list {
		flex: 1;
		padding: 24px 28px;
	}
	.ach-grid {
		display: flex;
		flex-direction: column;
		gap: 14px;
	}

	.ach-card {
		background: rgba(8, 12, 18, 0.6);
		border: 1px solid rgba(255, 255, 255, 0.05);
		border-radius: 14px;
		transition: all 0.25s;
		position: relative;
	}
	.ach-card:hover {
		transform: translateY(-1px);
		background: rgba(12, 18, 26, 0.8);
		border-color: rgba(255, 255, 255, 0.1);
	}
	.ach-card.is-unlocked {
		border-color: rgba(255, 255, 255, 0.08);
	}
	.ach-card-inner {
		display: flex;
		gap: 16px;
		padding: 10px 16px;
		height: 88px;
		box-sizing: border-box;
	}

	.ach-icon-box {
		width: 68px;
		height: 68px;
		border-radius: 10px;
		overflow: hidden;
		flex-shrink: 0;
		background: #10141b;
		border: 1px solid rgba(255, 255, 255, 0.05);
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
		font-size: 28px;
		display: flex;
		align-items: center;
		justify-content: center;
		height: 100%;
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
		line-height: 1;
	}
	.ach-name {
		font-size: 16px;
		font-weight: 800;
		color: #fff;
		letter-spacing: -0.2px;
		white-space: nowrap;
		overflow: hidden;
		text-overflow: ellipsis;
	}
	.ach-name.muted {
		color: #444;
	}
	.ach-date {
		font-size: 10px;
		color: #333;
		font-weight: 700;
		letter-spacing: 0.5px;
	}

	.ach-desc-row {
		overflow: hidden;
	}
	.ach-desc {
		font-size: 12px;
		color: #9ca3af;
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
		opacity: 0.5;
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
	}
	.rarity-progress-track {
		width: 100px;
		height: 4px;
		background: rgba(255, 255, 255, 0.05);
		border-radius: 3px;
		overflow: hidden;
	}
	.rarity-progress-fill {
		height: 100%;
		border-radius: 3px;
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
	.ach-card:hover .ach-actions {
		opacity: 1;
	}
	.action-btn {
		background: rgba(0, 0, 0, 0.6);
		border: 1px solid rgba(255, 255, 255, 0.1);
		border-radius: 6px;
		padding: 4px 10px;
		font-size: 10px;
		color: #ccc;
		font-weight: 700;
		cursor: pointer;
		display: flex;
		align-items: center;
		gap: 6px;
		transition: all 0.2s;
		backdrop-filter: blur(4px);
	}
	.action-btn:hover {
		background: rgba(255, 255, 255, 0.1);
		color: #fff;
		border-color: rgba(255, 255, 255, 0.2);
	}
	.action-btn.is-copied {
		background: rgba(61, 220, 132, 0.2);
		color: #3ddc84;
		border-color: rgba(61, 220, 132, 0.4);
	}

	.ach-card.jump-flash {
		animation: achJumpFlash 0.85s ease;
	}
	@keyframes achJumpFlash {
		0% {
			transform: scale(1.02);
			background: rgba(0, 102, 204, 0.15);
			border-color: rgba(0, 102, 204, 0.4);
		}
		100% {
			transform: scale(1);
			background: rgba(8, 12, 18, 0.6);
		}
	}

	.is-secret-hidden {
		opacity: 0.3;
		filter: blur(2px);
	}
</style>
