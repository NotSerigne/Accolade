<script lang="ts">
	import { onMount } from 'svelte';
	import { invoke } from '@tauri-apps/api/core';
	import { getCurrentWindow } from '@tauri-apps/api/window';
	import type { Game } from '$lib/stores/Games.js';
	import type { SteamUser } from '$lib/stores/user.js';
	import { calculateAdvancedStats } from '$lib/stores/selectedGame.js';
	import { i18n } from '$lib/stores/i18n.js';

	interface ProfileData {
		user?: SteamUser;
		games: Game[];
		stats?: {
			rarestAchievement: { name: string; pct: string } | null;
			maxAchievementsInDay: number;
			maxAchievementsInWeek: number;
			totalRemaining: number;
			totalUnlocked: number;
		};
	}

	let profile1 = $state<ProfileData | null>(null);
	let profile2 = $state<ProfileData | null>(null);
	let isDraggingOver = $state(false);
	let dragHoverSlot = $state<1 | 2 | null>(null);
	let slot1El: HTMLElement | null = null;
	let slot2El: HTMLElement | null = null;

	function parseJsonText(rawText: string): ProfileData | null {
		try {
			const data = JSON.parse(rawText);
			let profile: ProfileData;

			if (Array.isArray(data)) {
				profile = { games: data };
			} else if (data && Array.isArray(data.games)) {
				profile = data;
			} else {
				return null;
			}

			if (!profile.stats) {
				profile.stats = calculateAdvancedStats(profile.games);
			}

			return profile;
		} catch {
			return null;
		}
	}

	async function parseFile(file: File): Promise<ProfileData | null> {
		if (!file.name.toLowerCase().endsWith('.json')) return null;
		return new Promise((resolve) => {
			const reader = new FileReader();
			reader.onload = (e) => {
				resolve(parseJsonText((e.target?.result as string) || ''));
			};
			reader.readAsText(file);
		});
	}

	function assignProfile(data: ProfileData, targetSlot?: 1 | 2 | null) {
		if (targetSlot === 1) {
			profile1 = data;
		} else if (targetSlot === 2) {
			profile2 = data;
		} else if (!profile1) {
			profile1 = data;
		} else if (!profile2) {
			profile2 = data;
		} else {
			profile1 = profile2;
			profile2 = data;
		}
	}

	async function handleFiles(files: FileList | File[], targetSlot?: 1 | 2 | null) {
		const validFiles = Array.from(files).filter((f) => f.name.toLowerCase().endsWith('.json'));
		if (validFiles.length === 0) return;

		for (let i = 0; i < validFiles.length; i++) {
			const data = await parseFile(validFiles[i]);
			if (data) {
				assignProfile(data, validFiles.length === 1 ? targetSlot : null);
			}
		}
	}

	async function handleDroppedPaths(paths: string[], targetSlot?: 1 | 2 | null) {
		const jsonPaths = paths.filter((p) => p.toLowerCase().endsWith('.json'));
		if (jsonPaths.length === 0) return;

		for (let i = 0; i < jsonPaths.length; i++) {
			try {
				const raw = await invoke<string>('read_json_file', { path: jsonPaths[i] });
				const data = parseJsonText(raw);
				if (data) {
					assignProfile(data, jsonPaths.length === 1 ? targetSlot : null);
				}
			} catch (err) {
				console.error('[compare] Failed to read dropped JSON file:', err);
			}
		}
	}

	function detectSlotAtPosition(x: number, y: number): 1 | 2 | null {
		const scale = window.devicePixelRatio || 1;
		const candidates = [
			{ cx: x, cy: y },
			{ cx: x / scale, cy: y / scale }
		];

		for (const { cx, cy } of candidates) {
			if (slot1El) {
				const r1 = slot1El.getBoundingClientRect();
				if (cx >= r1.left && cx <= r1.right && cy >= r1.top && cy <= r1.bottom) return 1;
			}
			if (slot2El) {
				const r2 = slot2El.getBoundingClientRect();
				if (cx >= r2.left && cx <= r2.right && cy >= r2.top && cy <= r2.bottom) return 2;
			}
		}
		return null;
	}

	onMount(() => {
		let unlisten: (() => void) | null = null;
		const win = getCurrentWindow();
		void win
			.onDragDropEvent((event) => {
				const payload = event.payload;
				if (payload.type === 'enter' || payload.type === 'over') {
					isDraggingOver = true;
					if ('position' in payload && payload.position) {
						dragHoverSlot = detectSlotAtPosition(payload.position.x, payload.position.y);
					}
				} else if (payload.type === 'leave') {
					isDraggingOver = false;
					dragHoverSlot = null;
				} else if (payload.type === 'drop') {
					const slot =
						'position' in payload && payload.position
							? detectSlotAtPosition(payload.position.x, payload.position.y)
							: dragHoverSlot;
					isDraggingOver = false;
					dragHoverSlot = null;
					if ('paths' in payload && payload.paths?.length) {
						void handleDroppedPaths(payload.paths, slot);
					}
				}
			})
			.then((fn) => {
				unlisten = fn;
			})
			.catch(() => {});

		return () => {
			unlisten?.();
		};
	});

	const statsSummary = $derived.by(() => {
		if (!profile1 || !profile2) return null;

		const g1 = profile1.games;
		const g2 = profile2.games;

		const ids2 = new Set(g2.map((g) => g.id));
		const common = g1.filter((g) => ids2.has(g.id));

		const getCompPct = (games: Game[]) => {
			const total = games.reduce((acc, g) => acc + (g.achievements_total || 0), 0);
			const unlocked = games.reduce(
				(acc, g) => acc + (g.achievements?.filter((a) => a.unlocked).length || 0),
				0
			);
			return total > 0 ? Math.round((unlocked / total) * 100) : 0;
		};

		return {
			comp1: getCompPct(g1),
			comp2: getCompPct(g2),
			commonCount: common.length
		};
	});

	function getGameStats(p: ProfileData, gameId: string) {
		const g = p.games.find((x) => x.id === gameId);
		if (!g) return { pct: 0, unlocked: 0, total: 0 };
		const unlocked = g.achievements?.filter((a) => a.unlocked).length || 0;
		const total = g.achievements_total || g.achievements?.length || 0;
		return {
			pct: Math.round((unlocked / (total || 1)) * 100),
			unlocked,
			total
		};
	}

	const commonGames = $derived(
		profile1 && profile2
			? profile1.games.filter((g) => profile2!.games.some((x) => x.id === g.id))
			: []
	);

	function reset() {
		profile1 = null;
		profile2 = null;
	}
</script>

<!-- svelte-ignore a11y_no_static_element_interactions -->
<div
	class="compare-page"
	class:is-dragging={isDraggingOver}
	ondragover={(e) => {
		e.preventDefault();
		isDraggingOver = true;
	}}
	ondragleave={(e) => {
		if (e.currentTarget === e.target) {
			isDraggingOver = false;
			dragHoverSlot = null;
		}
	}}
	ondrop={(e) => {
		e.preventDefault();
		const slot = dragHoverSlot;
		isDraggingOver = false;
		dragHoverSlot = null;
		if (e.dataTransfer?.files?.length) {
			void handleFiles(e.dataTransfer.files, slot);
		}
	}}
>
	{#if isDraggingOver}
		<div class="drop-overlay">
			<div class="drop-overlay-box">
				<svg
					width="44"
					height="44"
					viewBox="0 0 24 24"
					fill="none"
					stroke="currentColor"
					stroke-width="2"
					stroke-linecap="round"
					stroke-linejoin="round"
				>
					<path d="M21 15v4a2 2 0 0 1-2 2H5a2 2 0 0 1-2-2v-4" />
					<polyline points="7 10 12 15 17 10" />
					<line x1="12" y1="15" x2="12" y2="3" />
				</svg>
				<span>{$i18n.t('compare.dropOverlay')}</span>
			</div>
		</div>
	{/if}

	<header class="page-header">
		<h1>{$i18n.t('compare.title')}</h1>
		<p class="subtitle">{$i18n.t('compare.subtitle')}</p>

		<div class="header-actions">
			<label class="upload-btn">
				<svg
					width="18"
					height="18"
					viewBox="0 0 24 24"
					fill="none"
					stroke="currentColor"
					stroke-width="2"
					stroke-linecap="round"
					stroke-linejoin="round"
					><path d="M21 15v4a2 2 0 0 1-2 2H5a2 2 0 0 1-2-2v-4" /><polyline
						points="17 8 12 3 7 8"
					/><line x1="12" y1="3" x2="12" y2="15" /></svg
				>
				{$i18n.t('compare.selectProfiles')}
				<input
					type="file"
					accept=".json"
					multiple
					onchange={(e) => {
						const input = e.target as HTMLInputElement;
						if (input.files) void handleFiles(input.files);
					}}
				/>
			</label>
		</div>
		<p class="drop-hint-label">{$i18n.t('compare.dropHint')}</p>
	</header>

	<div class="profiles-overview">
		<label
			class="profile-slot"
			class:active={profile1}
			class:drag-target={dragHoverSlot === 1}
			bind:this={slot1El}
			ondragover={(e) => {
				e.preventDefault();
				e.stopPropagation();
				isDraggingOver = true;
				dragHoverSlot = 1;
			}}
			ondrop={(e) => {
				e.preventDefault();
				e.stopPropagation();
				isDraggingOver = false;
				dragHoverSlot = null;
				if (e.dataTransfer?.files?.length) {
					void handleFiles(e.dataTransfer.files, 1);
				}
			}}
		>
			<input
				type="file"
				accept=".json"
				class="slot-file-input"
				onchange={(e) => {
					const input = e.target as HTMLInputElement;
					if (input.files) void handleFiles(input.files, 1);
					input.value = '';
				}}
			/>
			{#if profile1}
				<div class="user-info-card">
					<img
						src={profile1.user?.avatarfull ||
							'https://avatars.steamstatic.com/fef49e7fa7e1997310d705b2a6158ff8dc1cdfeb_full.jpg'}
						alt="Avatar"
						class="user-avatar"
					/>
					<div class="user-meta">
						<span class="user-name"
							>{profile1.user?.personaname || $i18n.t('compare.defaultProfile1')}</span
						>
						<span class="user-games">{profile1.games.length} jeux</span>
					</div>
				</div>
			{:else}
				<div class="placeholder-card">
					<div class="icon">📁</div>
					<span>{$i18n.t('compare.profile1NotLoaded')}</span>
				</div>
			{/if}
		</label>

		<div class="vs-badge">VS</div>

		<label
			class="profile-slot"
			class:active={profile2}
			class:drag-target={dragHoverSlot === 2}
			bind:this={slot2El}
			ondragover={(e) => {
				e.preventDefault();
				e.stopPropagation();
				isDraggingOver = true;
				dragHoverSlot = 2;
			}}
			ondrop={(e) => {
				e.preventDefault();
				e.stopPropagation();
				isDraggingOver = false;
				dragHoverSlot = null;
				if (e.dataTransfer?.files?.length) {
					void handleFiles(e.dataTransfer.files, 2);
				}
			}}
		>
			<input
				type="file"
				accept=".json"
				class="slot-file-input"
				onchange={(e) => {
					const input = e.target as HTMLInputElement;
					if (input.files) void handleFiles(input.files, 2);
					input.value = '';
				}}
			/>
			{#if profile2}
				<div class="user-info-card">
					<img
						src={profile2.user?.avatarfull ||
							'https://avatars.steamstatic.com/fef49e7fa7e1997310d705b2a6158ff8dc1cdfeb_full.jpg'}
						alt="Avatar"
						class="user-avatar"
					/>
					<div class="user-meta">
						<span class="user-name"
							>{profile2.user?.personaname || $i18n.t('compare.defaultProfile2')}</span
						>
						<span class="user-games">{profile2.games.length} jeux</span>
					</div>
				</div>
			{:else}
				<div class="placeholder-card">
					<div class="icon">📁</div>
					<span>{$i18n.t('compare.profile2NotLoaded')}</span>
				</div>
			{/if}
		</label>
	</div>

	{#if profile1 && profile2}
		{@const s1 = profile1.stats}
		{@const s2 = profile2.stats}

		<div class="stats-ribbon">
			<div class="ribbon-item">
				<div class="val">{statsSummary?.comp1}% vs {statsSummary?.comp2}%</div>
				<div class="lbl">{$i18n.t('compare.avgCompletion')}</div>
			</div>
			<div class="ribbon-item">
				<div class="val">{statsSummary?.commonCount}</div>
				<div class="lbl">{$i18n.t('compare.commonGames')}</div>
			</div>
			<button class="reset-btn" onclick={reset}>{$i18n.t('compare.reset')}</button>
		</div>

		<section class="fun-stats">
			<h2>{$i18n.t('compare.statDuel')}</h2>
			<div class="fun-stats-grid">
				<div class="fun-card">
					<div class="fun-label">{$i18n.t('compare.totalUnlocked')}</div>
					<div class="fun-values">
						<span class="v1">{s1?.totalUnlocked ?? '?'}</span>
						<span class="v-vs">vs</span>
						<span class="v2">{s2?.totalUnlocked ?? '?'}</span>
					</div>
				</div>
				<div class="fun-card">
					<div class="fun-label">{$i18n.t('compare.recordDay')}</div>
					<div class="fun-values">
						<span class="v1">{s1?.maxAchievementsInDay ?? '?'}</span>
						<span class="v-vs">vs</span>
						<span class="v2">{s2?.maxAchievementsInDay ?? '?'}</span>
					</div>
				</div>
				<div class="fun-card">
					<div class="fun-label">{$i18n.t('compare.recordWeek')}</div>
					<div class="fun-values">
						<span class="v1">{s1?.maxAchievementsInWeek ?? '?'}</span>
						<span class="v-vs">vs</span>
						<span class="v2">{s2?.maxAchievementsInWeek ?? '?'}</span>
					</div>
				</div>
				<div class="fun-card">
					<div class="fun-label">{$i18n.t('compare.remaining')}</div>
					<div class="fun-values">
						<span class="v1">{s1?.totalRemaining ?? '?'}</span>
						<span class="v-vs">vs</span>
						<span class="v2">{s2?.totalRemaining ?? '?'}</span>
					</div>
				</div>
				<div class="fun-card rarest">
					<div class="fun-label">{$i18n.t('compare.rarestUnlocked')}</div>
					<div class="fun-desc">
						<div class="d1">
							{s1?.rarestAchievement
								? `${s1.rarestAchievement.name} (${s1.rarestAchievement.pct}%)`
								: 'N/A'}
						</div>
						<div class="d-vs">{$i18n.t('compare.against')}</div>
						<div class="d2">
							{s2?.rarestAchievement
								? `${s2.rarestAchievement.name} (${s2.rarestAchievement.pct}%)`
								: 'N/A'}
						</div>
					</div>
				</div>
			</div>
		</section>

		{#if commonGames.length > 0}
			<section class="comparison-details">
				<h2>{$i18n.t('compare.commonGamesComparison')}</h2>
				<div class="compare-grid">
					{#each commonGames as game (game.id)}
						{@const g1 = getGameStats(profile1!, game.id)}
						{@const g2 = getGameStats(profile2!, game.id)}
						<div class="compare-card">
							<div class="game-header-mini">
								<img src={game.game_icon} alt="" class="mini-icon" />
								<span class="mini-title">{game.name}</span>
							</div>
							<div class="compare-bars">
								<div class="bar-row">
									<div class="bar-fill-track">
										<div class="bar-fill" style:width="{g1.pct}%"></div>
									</div>
									<div class="bar-info">
										<span class="bar-count">{g1.unlocked}/{g1.total}</span>
										<span class="bar-pct">{g1.pct}%</span>
									</div>
								</div>
								<div class="bar-row">
									<div class="bar-fill-track">
										<div class="bar-fill alt" style:width="{g2.pct}%"></div>
									</div>
									<div class="bar-info">
										<span class="bar-count">{g2.unlocked}/{g2.total}</span>
										<span class="bar-pct">{g2.pct}%</span>
									</div>
								</div>
							</div>
						</div>
					{/each}
				</div>
			</section>
		{:else}
			<div class="empty-state">
				<p>{$i18n.t('compare.noCommonGames')}</p>
			</div>
		{/if}
	{:else}
		<div class="empty-state">
			<svg
				width="64"
				height="64"
				viewBox="0 0 24 24"
				fill="none"
				stroke="currentColor"
				stroke-width="1.5"
				><circle cx="12" cy="12" r="10" /><path
					d="M12 2a14.5 14.5 0 0 0 0 20 14.5 14.5 0 0 0 0-20"
				/><path d="M2 12h20" /></svg
			>
			<p>{$i18n.t('compare.importTwoFiles')}</p>
		</div>
	{/if}
</div>

<style>
	.compare-page {
		height: 100%;
		display: flex;
		flex-direction: column;
		padding: 40px;
		overflow-y: auto;
		color: var(--text-primary);
		transition: background 0.3s;
		position: relative;
	}

	.drop-overlay {
		position: fixed;
		inset: 0;
		z-index: 50;
		background: rgba(10, 12, 18, 0.72);
		backdrop-filter: blur(6px);
		display: flex;
		align-items: center;
		justify-content: center;
		pointer-events: none;
	}

	.drop-overlay-box {
		display: flex;
		flex-direction: column;
		align-items: center;
		gap: 14px;
		padding: 36px 56px;
		border-radius: 20px;
		border: 2px dashed var(--accent, #c8a96e);
		background: rgba(200, 169, 110, 0.1);
		color: var(--accent, #c8a96e);
		font-size: 1.1rem;
		font-weight: 700;
		box-shadow: 0 18px 50px rgba(0, 0, 0, 0.45);
	}

	/* Subtile Floating Scrollbar */
	.compare-page::-webkit-scrollbar {
		width: 14px;
	}

	.compare-page::-webkit-scrollbar-track {
		background: transparent;
	}

	.compare-page::-webkit-scrollbar-thumb {
		background: rgba(255, 255, 255, 0.1);
		border: 4px solid transparent;
		background-clip: padding-box;
		border-radius: 10px;
		transition: background 0.2s;
	}

	.compare-page::-webkit-scrollbar-thumb:hover {
		background: rgba(255, 255, 255, 0.2);
	}

	.page-header {
		text-align: center;
		margin-bottom: 40px;
	}
	h1 {
		font-size: 2.2rem;
		margin-bottom: 8px;
	}
	.subtitle {
		color: var(--text-secondary);
		opacity: 0.8;
		margin-bottom: 24px;
	}

	.header-actions {
		display: flex;
		justify-content: center;
		margin-top: 10px;
	}

	.drop-hint-label {
		margin-top: 12px;
		font-size: 0.82rem;
		color: var(--text-muted);
	}

	.upload-btn {
		display: flex;
		align-items: center;
		gap: 10px;
		background: var(--accent);
		color: var(--accent-text);
		padding: 12px 24px;
		border-radius: 12px;
		font-weight: 700;
		cursor: pointer;
		transition: all 0.2s cubic-bezier(0.4, 0, 0.2, 1);
		box-shadow: 0 4px 15px color-mix(in srgb, var(--accent) 30%, transparent);
	}

	.upload-btn:hover {
		transform: translateY(-2px);
		filter: brightness(1.1);
		box-shadow: 0 6px 20px color-mix(in srgb, var(--accent) 40%, transparent);
	}

	.upload-btn input,
	.slot-file-input {
		display: none;
	}

	.profiles-overview {
		display: flex;
		align-items: center;
		justify-content: center;
		gap: 40px;
		margin-bottom: 50px;
	}

	.profile-slot {
		width: 340px;
		height: 100px;
		background: var(--surface-1);
		border: 1px dashed var(--border-soft);
		border-radius: 16px;
		display: flex;
		align-items: center;
		justify-content: center;
		cursor: pointer;
		transition: all 0.25s ease;
	}

	.profile-slot:hover,
	.profile-slot.drag-target {
		border-color: var(--accent, #c8a96e);
		background: color-mix(in srgb, var(--accent, #c8a96e) 8%, var(--surface-1));
		transform: translateY(-2px);
	}

	.profile-slot.active {
		border-style: solid;
		border-color: var(--accent);
		background: var(--surface-2);
	}

	.user-info-card {
		display: flex;
		align-items: center;
		gap: 20px;
		width: 100%;
		padding: 0 20px;
	}

	.user-avatar {
		width: 56px;
		height: 56px;
		border-radius: 12px;
		border: 2px solid var(--accent);
	}
	.user-meta {
		display: flex;
		flex-direction: column;
	}
	.user-name {
		font-weight: 700;
		font-size: 1.1rem;
	}
	.user-games {
		font-size: 0.85rem;
		color: var(--text-secondary);
	}

	.placeholder-card {
		display: flex;
		flex-direction: column;
		align-items: center;
		gap: 8px;
		opacity: 0.5;
	}
	.placeholder-card .icon {
		font-size: 1.5rem;
	}

	.vs-badge {
		font-size: 1.5rem;
		font-weight: 900;
		color: var(--accent);
		opacity: 0.3;
	}

	.stats-ribbon {
		background: var(--surface-1);
		border: 1px solid var(--border-soft);
		border-radius: 12px;
		padding: 20px 40px;
		display: flex;
		align-items: center;
		justify-content: space-around;
		margin-bottom: 50px;
	}

	.ribbon-item {
		text-align: center;
	}
	.ribbon-item .val {
		font-size: 1.4rem;
		font-weight: 800;
		color: var(--accent);
	}
	.ribbon-item .lbl {
		font-size: 0.75rem;
		text-transform: uppercase;
		letter-spacing: 1px;
		color: var(--text-secondary);
	}

	.reset-btn {
		padding: 8px 16px;
		border-radius: 8px;
		border: 1px solid var(--border-soft);
		background: transparent;
		color: var(--text-secondary);
		cursor: pointer;
		font-size: 0.9rem;
	}
	.reset-btn:hover {
		background: var(--surface-hover);
		color: var(--text-primary);
	}

	.fun-stats {
		margin-bottom: 50px;
	}
	.fun-stats h2 {
		margin-bottom: 20px;
		font-size: 1.3rem;
	}

	.fun-stats-grid {
		display: grid;
		grid-template-columns: repeat(auto-fit, minmax(250px, 1fr));
		gap: 20px;
	}

	.fun-card {
		background: var(--surface-1);
		border: 1px solid var(--border-soft);
		border-radius: 14px;
		padding: 20px;
		text-align: center;
		display: flex;
		flex-direction: column;
		justify-content: center;
	}

	.fun-label {
		font-size: 0.75rem;
		text-transform: uppercase;
		color: var(--text-secondary);
		margin-bottom: 15px;
		letter-spacing: 0.5px;
	}
	.fun-values {
		display: flex;
		align-items: center;
		justify-content: center;
		gap: 20px;
		font-size: 2rem;
		font-weight: 800;
	}
	.v-vs {
		font-size: 1rem;
		opacity: 0.3;
	}
	.v1 {
		color: var(--accent);
	}
	.v2 {
		color: color-mix(in srgb, var(--accent) 60%, white);
	}

	.fun-desc {
		display: flex;
		flex-direction: column;
		gap: 5px;
		font-size: 0.9rem;
	}
	.d1 {
		color: var(--accent);
		font-weight: 700;
	}
	.d2 {
		color: color-mix(in srgb, var(--accent) 60%, white);
		font-weight: 700;
	}
	.d-vs {
		font-size: 0.7rem;
		opacity: 0.4;
		text-transform: uppercase;
	}

	.comparison-details h2 {
		margin-bottom: 25px;
		font-size: 1.3rem;
	}

	.compare-grid {
		display: grid;
		grid-template-columns: repeat(auto-fill, minmax(350px, 1fr));
		gap: 20px;
		padding-bottom: 40px;
	}

	.compare-card {
		background: var(--surface-1);
		border: 1px solid var(--border-soft);
		border-radius: 14px;
		padding: 16px;
	}

	.game-header-mini {
		display: flex;
		align-items: center;
		gap: 12px;
		margin-bottom: 16px;
	}
	.mini-icon {
		width: 32px;
		height: 32px;
		border-radius: 6px;
	}
	.mini-title {
		font-weight: 600;
		font-size: 0.95rem;
		white-space: nowrap;
		overflow: hidden;
		text-overflow: ellipsis;
	}

	.compare-bars {
		display: flex;
		flex-direction: column;
		gap: 10px;
	}
	.bar-row {
		display: flex;
		align-items: center;
		gap: 12px;
	}
	.bar-fill-track {
		flex: 1;
		height: 6px;
		background: var(--surface-2);
		border-radius: 3px;
		overflow: hidden;
	}
	.bar-fill {
		height: 100%;
		background: var(--accent);
		border-radius: 3px;
	}
	.bar-fill.alt {
		background: color-mix(in srgb, var(--accent) 60%, white);
	}
	.bar-info {
		display: flex;
		align-items: center;
		gap: 10px;
		min-width: 85px;
		justify-content: flex-end;
	}
	.bar-count {
		font-size: 0.7rem;
		color: var(--text-muted);
		font-family: monospace;
	}
	.bar-pct {
		font-size: 0.75rem;
		font-family: monospace;
		font-weight: 700;
		width: 35px;
		text-align: right;
	}

	.empty-state {
		text-align: center;
		padding: 60px 0;
		color: var(--text-muted);
		opacity: 0.8;
	}
	.empty-state svg {
		margin-bottom: 16px;
		opacity: 0.2;
	}
</style>
