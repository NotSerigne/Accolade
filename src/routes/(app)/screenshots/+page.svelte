<script lang="ts">
	import { invoke } from '@tauri-apps/api/core';
	import { convertFileSrc } from '@tauri-apps/api/core';
	import { games } from '$lib/stores/Games.js';
	import { i18n } from '$lib/stores/i18n.js';
	import { settings } from '$lib/stores/settings.js';
	import { onMount } from 'svelte';
	import { listen } from '@tauri-apps/api/event';
	import { fade } from 'svelte/transition';

	interface Screenshot {
		filename: string;
		path: string;
		timestamp: number;
		game_id: string | null;
		achievement_key: string | null;
	}

	let screenshots = $state<Screenshot[]>([]);
	let loading = $state(true);
	let selectedScreenshot = $state<Screenshot | null>(null);

	async function loadScreenshots() {
		try {
			loading = true;
			screenshots = await invoke('get_screenshots');
		} catch (e) {
			console.error('Failed to load screenshots:', e);
		} finally {
			loading = false;
		}
	}

	onMount(() => {
		loadScreenshots();

		const unlisten = listen('screenshot-taken', () => {
			loadScreenshots();
		});

		const handleKeydown = (e: KeyboardEvent) => {
			if (!selectedScreenshot) return;
			if (e.key === 'ArrowLeft') {
				navigateScreenshot('prev');
			} else if (e.key === 'ArrowRight') {
				navigateScreenshot('next');
			} else if (e.key === 'Escape') {
				closeLightbox();
			}
		};
		window.addEventListener('keydown', handleKeydown);

		return () => {
			unlisten.then((f) => f());
			window.removeEventListener('keydown', handleKeydown);
		};
	});

	let currentIndex = $derived(
		selectedScreenshot
			? screenshots.findIndex((s) => s.filename === selectedScreenshot!.filename)
			: -1
	);

	function navigateScreenshot(direction: 'prev' | 'next') {
		if (!selectedScreenshot) return;
		const index = screenshots.findIndex((s) => s.filename === selectedScreenshot!.filename);
		if (index === -1) return;

		if (direction === 'prev' && index > 0) {
			selectedScreenshot = screenshots[index - 1];
		} else if (direction === 'next' && index < screenshots.length - 1) {
			selectedScreenshot = screenshots[index + 1];
		}
	}

	function getGameData(id: string | null) {
		if (!id) return null;
		return $games.find((g) => g.id === id) || null;
	}

	function getAchData(gameId: string | null, achKey: string | null) {
		if (!gameId || !achKey) return null;
		const game = getGameData(gameId);
		if (!game) return null;
		return game.achievements.find((a) => a.key === achKey) || null;
	}

	function getAchIndex(gameId: string | null, achKey: string | null) {
		if (!gameId || !achKey) return null;
		const game = getGameData(gameId);
		if (!game) return null;
		// Sort unlocked achievements chronologically to find the N°X
		const unlocked = game.achievements
			.filter((a) => a.unlocked && a.unlocked_time)
			.sort((a, b) => a.unlocked_time! - b.unlocked_time!);
		const index = unlocked.findIndex((a) => a.key === achKey);
		return index >= 0 ? index + 1 : null;
	}

	function getRarityClass(rarity: string | null, pct: string | number | null) {
		const n = pct != null ? (typeof pct === 'string' ? parseFloat(pct) : pct) : null;

		if (n !== null && !isNaN(n)) {
			if (n <= 0.1) return 'mythique';
			if (n <= 1.0) return 'legendaire';
			if (n <= 3.0) return 'epique';
			if (n <= 7.0) return 'rare';
			if (n <= 15.0) return 'tres-rare';
			if (n <= 35.0) return 'peu-commune';
			return 'commune';
		}

		if (!rarity) return 'commune';
		const normalized = rarity.toLowerCase().trim();
		if (normalized.includes('mythique') || normalized.includes('mythic')) return 'mythique';
		if (normalized.includes('légendaire') || normalized.includes('legendary')) return 'legendaire';
		if (normalized.includes('épique') || normalized.includes('epic')) return 'epique';
		if (normalized.includes('très rare') || normalized.includes('very rare')) return 'tres-rare';
		if (normalized.includes('rare')) return 'rare';
		if (normalized.includes('peu') || normalized.includes('uncommon')) return 'peu-commune';
		return 'commune';
	}

	function getGameProgress(gameId: string | null) {
		const game = getGameData(gameId);
		if (!game) return { unlocked: 0, total: 0 };
		const unlocked = game.achievements.filter((a) => a.unlocked).length;
		const total = game.achievements_total || game.achievements.length;
		return { unlocked, total };
	}

	async function deleteScreenshot(screenshot: Screenshot, event: Event) {
		event.stopPropagation();
		if (confirm($i18n.t('screenshots.deleteConfirm') || "Supprimer cette capture d'écran ?")) {
			try {
				await invoke('delete_screenshot', { filename: screenshot.filename });
				if (selectedScreenshot?.filename === screenshot.filename) {
					selectedScreenshot = null;
				}
				await loadScreenshots();
			} catch (e) {
				console.error('Failed to delete screenshot:', e);
			}
		}
	}

	async function openFolder() {
		try {
			await invoke('open_screenshots_dir');
		} catch (e) {
			console.error('Failed to open folder:', e);
		}
	}

	function formatDate(timestamp: number | null | undefined) {
		if (!timestamp) return 'Date inconnue';
		return new Date(timestamp * 1000).toLocaleString($i18n.locale || 'fr-FR', {
			dateStyle: 'medium',
			timeStyle: 'short'
		});
	}

	function selectScreenshot(screenshot: Screenshot) {
		selectedScreenshot = screenshot;
	}

	function closeLightbox() {
		selectedScreenshot = null;
	}
</script>

<div class="page-container" in:fade={{ duration: 200 }}>
	<header class="header">
		<div class="header-content">
			<h1>{$i18n.t('screenshots.title') || "Captures d'écran"}</h1>
			<p class="subtitle">
				{screenshots.length}
				{screenshots.length > 1 ? 'captures enregistrées' : 'capture enregistrée'}
			</p>
		</div>
		<button class="action-btn" onclick={openFolder}>
			<svg
				width="18"
				height="18"
				viewBox="0 0 24 24"
				fill="none"
				stroke="currentColor"
				stroke-width="2"
			>
				<path d="M22 19a2 2 0 0 1-2 2H4a2 2 0 0 1-2-2V5a2 2 0 0 1 2-2h5l2 3h9a2 2 0 0 1 2 2z" />
			</svg>
			{$i18n.t('screenshots.openFolder') || 'Ouvrir le dossier'}
		</button>
	</header>

	{#if loading}
		<div class="loading-state">
			<div class="spinner"></div>
		</div>
	{:else if screenshots.length === 0}
		<div class="empty-state" in:fade>
			<div class="empty-icon">
				<svg
					width="64"
					height="64"
					viewBox="0 0 24 24"
					fill="none"
					stroke="currentColor"
					stroke-width="1.5"
				>
					<rect x="3" y="3" width="18" height="18" rx="2" ry="2" />
					<circle cx="8.5" cy="8.5" r="1.5" />
					<polyline points="21 15 16 10 5 21" />
				</svg>
			</div>
			<h2>{$i18n.t('screenshots.emptyTitle') || 'Aucune capture'}</h2>
			<p>
				{$i18n.t('screenshots.emptyDesc') ||
					'Les captures prises lors de vos succès apparaîtront ici.'}
			</p>
			<p class="hint">
				{$i18n.t('screenshots.hintPrefix') || 'Appuyez sur'}
				<kbd>{$settings.screenshotShortcut || 'F12'}</kbd>
				{$i18n.t('screenshots.hintSuffix') || 'pour prendre une capture manuelle.'}
			</p>
		</div>
	{:else}
		<div class="screenshot-grid">
			{#each screenshots as screenshot (screenshot.filename)}
				<div
					class="screenshot-card"
					role="button"
					tabindex="0"
					onclick={() => selectScreenshot(screenshot)}
					onkeydown={(e) => e.key === 'Enter' && selectScreenshot(screenshot)}
					in:fade={{ duration: 200 }}
				>
					<div class="image-wrapper">
						<img src={convertFileSrc(screenshot.path)} alt={screenshot.filename} loading="lazy" />
						<div class="overlay">
							<button
								class="delete-btn"
								onclick={(e) => deleteScreenshot(screenshot, e)}
								title="Supprimer"
							>
								<svg
									width="16"
									height="16"
									viewBox="0 0 24 24"
									fill="none"
									stroke="currentColor"
									stroke-width="2"
								>
									<path
										d="M3 6h18M19 6v14a2 2 0 0 1-2 2H7a2 2 0 0 1-2-2V6m3 0V4a2 2 0 0 1 2-2h4a2 2 0 0 1 2 2v2"
									/>
								</svg>
							</button>
						</div>
					</div>
					<div class="card-info">
						<span class="game-name"
							>{getGameData(screenshot.game_id)?.name || screenshot.game_id || 'Manuel'}</span
						>
						<span class="date"
							>{formatDate(
								getAchData(screenshot.game_id, screenshot.achievement_key)?.unlocked_time ||
									screenshot.timestamp
							)}</span
						>
					</div>
				</div>
			{/each}
		</div>
	{/if}
</div>

{#if selectedScreenshot}
	<!-- svelte-ignore a11y_click_events_have_key_events -->
	<!-- svelte-ignore a11y_no_static_element_interactions -->
	<div
		class="lightbox"
		onclick={closeLightbox}
		in:fade={{ duration: 150 }}
		out:fade={{ duration: 150 }}
	>
		<!-- svelte-ignore a11y_click_events_have_key_events -->
		<!-- svelte-ignore a11y_no_static_element_interactions -->
		<div class="lightbox-content" onclick={(e) => e.stopPropagation()}>
			<img
				class="lightbox-main-img"
				src={convertFileSrc(selectedScreenshot.path)}
				alt={selectedScreenshot.filename}
			/>
			<div class="lightbox-footer">
				<div class="lightbox-navigation">
					<button
						class="nav-btn-mini"
						disabled={currentIndex <= 0}
						onclick={() => navigateScreenshot('prev')}
						aria-label="Précédent"
					>
						<svg
							width="18"
							height="18"
							viewBox="0 0 24 24"
							fill="none"
							stroke="currentColor"
							stroke-width="3"
						>
							<path d="M15 18l-6-6 6-6" />
						</svg>
					</button>
					<span class="nav-count">{currentIndex + 1} / {screenshots.length}</span>
					<button
						class="nav-btn-mini"
						disabled={currentIndex >= screenshots.length - 1}
						onclick={() => navigateScreenshot('next')}
						aria-label="Suivant"
					>
						<svg
							width="18"
							height="18"
							viewBox="0 0 24 24"
							fill="none"
							stroke="currentColor"
							stroke-width="3"
						>
							<path d="M9 18l6-6-6-6" />
						</svg>
					</button>
				</div>
				<div class="lightbox-info-container">
					<div class="lightbox-info">
						{#if selectedScreenshot.game_id}
							{@const game = getGameData(selectedScreenshot.game_id)}
							{@const ach = getAchData(
								selectedScreenshot.game_id,
								selectedScreenshot.achievement_key
							)}
							{@const achIndex = getAchIndex(
								selectedScreenshot.game_id,
								selectedScreenshot.achievement_key
							)}

							{@const progress = getGameProgress(selectedScreenshot.game_id)}
							{@const ringDash = progress.total ? (progress.unlocked / progress.total) * 94.25 : 0}
							<div class="rich-info-container">
								<div class="game-brand">
									{#if game?.steamgrid_icon_url || game?.game_icon}
										<img
											class="game-icon"
											src={game.steamgrid_icon_url || game.game_icon}
											alt="Game icon"
										/>
									{/if}
									<div class="brand-text">
										<h3>{game?.name || selectedScreenshot.game_id}</h3>
										<p class="date-text">
											{formatDate(ach?.unlocked_time || selectedScreenshot.timestamp)}
										</p>
									</div>
								</div>

								{#if ach}
									<div class="ach-brand {getRarityClass(ach.rarity, ach.completionpercentage)}">
										<img class="ach-icon" src={ach.icon} alt="Achievement icon" />
										<div class="ach-details">
											<div class="ach-header">
												<h4>{ach.name || ach.key}</h4>
												{#if achIndex}
													<span class="ach-badge">Succès n°{achIndex}</span>
												{/if}
												{#if ach.rarity}
													<span
														class="ach-rarity {getRarityClass(
															ach.rarity,
															ach.completionpercentage
														)}">{ach.rarity}</span
													>
												{/if}
												{#if ach.completionpercentage}
													<span class="ach-pct"
														>{parseFloat(ach.completionpercentage).toFixed(1)}%</span
													>
												{/if}
											</div>
											{#if ach.desc}
												<p class="ach-desc">{ach.desc}</p>
											{/if}
										</div>

										{#if progress.total > 0}
											<div class="ach-progress-circle">
												<div class="progress-fraction">{progress.unlocked}/{progress.total}</div>
												<svg class="progress-ring" viewBox="0 0 36 36">
													<circle class="ring-bg" cx="18" cy="18" r="15" />
													<circle
														class="ring-fill"
														cx="18"
														cy="18"
														r="15"
														stroke-dasharray="{ringDash} 94.25"
													/>
												</svg>
											</div>
										{/if}
									</div>
								{:else if selectedScreenshot.achievement_key}
									<div class="ach-key">
										Succès : <code>{selectedScreenshot.achievement_key}</code>
									</div>
								{/if}
							</div>
						{:else}
							<div class="brand-text">
								<h3>Manuel</h3>
								<p class="date-text">{formatDate(selectedScreenshot.timestamp)}</p>
							</div>
						{/if}
					</div>
					<div class="lightbox-actions">
						<button class="secondary-btn" onclick={closeLightbox}>Fermer</button>
						<button
							class="danger-btn"
							onclick={(e) => {
								deleteScreenshot(selectedScreenshot!, e);
								closeLightbox();
							}}>Supprimer</button
						>
					</div>
				</div>
			</div>
		</div>
	</div>
{/if}

<style>
	.page-container {
		padding: 40px;
		height: 100%;
		overflow-y: auto;
		display: flex;
		flex-direction: column;
		gap: 32px;
	}

	.header {
		display: flex;
		justify-content: space-between;
		align-items: flex-end;
	}

	.header h1 {
		font-size: 32px;
		font-weight: 800;
		color: var(--text-primary);
		margin-bottom: 4px;
	}

	.subtitle {
		color: var(--text-muted);
		font-size: 14px;
	}

	.action-btn {
		display: flex;
		align-items: center;
		gap: 8px;
		background: var(--surface-1);
		border: 1px solid var(--border-soft);
		padding: 8px 16px;
		border-radius: 8px;
		color: var(--text-primary);
		font-size: 14px;
		font-weight: 600;
		cursor: pointer;
		transition: all 0.2s;
	}

	.action-btn:hover {
		background: var(--surface-hover);
		border-color: var(--accent);
	}

	.loading-state {
		flex: 1;
		display: flex;
		align-items: center;
		justify-content: center;
	}

	.spinner {
		width: 40px;
		height: 40px;
		border: 3px solid var(--border-soft);
		border-top-color: var(--accent);
		border-radius: 50%;
		animation: spin 1s linear infinite;
	}

	@keyframes spin {
		to {
			transform: rotate(360deg);
		}
	}

	.empty-state {
		flex: 1;
		display: flex;
		flex-direction: column;
		align-items: center;
		justify-content: center;
		text-align: center;
		gap: 16px;
		color: var(--text-muted);
		padding-bottom: 80px;
	}

	.empty-icon {
		opacity: 0.3;
		margin-bottom: 8px;
	}

	.empty-state h2 {
		font-size: 24px;
		color: var(--text-primary);
	}

	.hint {
		font-size: 13px;
		background: var(--surface-1);
		padding: 6px 12px;
		border-radius: 6px;
		margin-top: 12px;
	}

	kbd {
		background: var(--surface-2);
		border: 1px solid var(--border-soft);
		border-radius: 4px;
		padding: 2px 6px;
		font-family: inherit;
		font-weight: 700;
		color: var(--accent);
	}

	.screenshot-grid {
		display: grid;
		grid-template-columns: repeat(auto-fill, minmax(280px, 1fr));
		gap: 24px;
	}

	.screenshot-card {
		background: var(--surface-1);
		border: 1px solid var(--border-soft);
		border-radius: 12px;
		overflow: hidden;
		display: flex;
		flex-direction: column;
		cursor: pointer;
		transition: all 0.2s cubic-bezier(0.4, 0, 0.2, 1);
		text-align: left;
		padding: 0;
	}

	.screenshot-card:hover {
		transform: translateY(-4px);
		border-color: var(--accent);
		box-shadow: 0 12px 30px rgba(0, 0, 0, 0.4);
	}

	.image-wrapper {
		position: relative;
		aspect-ratio: 16 / 9;
		overflow: hidden;
		background: #000;
	}

	.image-wrapper img {
		width: 100%;
		height: 100%;
		object-fit: cover;
		transition: transform 0.5s;
	}

	.screenshot-card:hover .image-wrapper img {
		transform: scale(1.05);
	}

	.overlay {
		position: absolute;
		top: 0;
		left: 0;
		right: 0;
		bottom: 0;
		background: linear-gradient(
			to bottom,
			rgba(0, 0, 0, 0.4) 0%,
			transparent 40%,
			rgba(0, 0, 0, 0.6) 100%
		);
		opacity: 0;
		transition: opacity 0.2s;
		display: flex;
		justify-content: flex-end;
		padding: 12px;
	}

	.screenshot-card:hover .overlay {
		opacity: 1;
	}

	.delete-btn {
		background: rgba(220, 38, 38, 0.8);
		color: white;
		border: none;
		width: 32px;
		height: 32px;
		border-radius: 8px;
		display: flex;
		align-items: center;
		justify-content: center;
		cursor: pointer;
		transition: all 0.2s;
		backdrop-filter: blur(4px);
	}

	.delete-btn:hover {
		background: rgb(220, 38, 38);
		transform: scale(1.1);
	}

	.card-info {
		padding: 16px;
		display: flex;
		flex-direction: column;
		gap: 4px;
	}

	.game-name {
		font-weight: 700;
		color: var(--text-primary);
		font-size: 15px;
	}

	.date {
		font-size: 12px;
		color: var(--text-muted);
	}

	/* Lightbox */
	.lightbox {
		position: fixed;
		top: 0;
		left: 0;
		right: 0;
		bottom: 0;
		background: rgba(0, 0, 0, 0.96);
		z-index: 2000;
		display: flex;
		align-items: center;
		justify-content: center;
		padding: 10px;
		backdrop-filter: blur(12px);
	}

	.lightbox-content {
		max-width: 98vw;
		max-height: 98vh;
		background: var(--bg-main);
		border-radius: 10px;
		overflow: hidden;
		box-shadow: 0 40px 100px rgba(0, 0, 0, 0.9);
		display: flex;
		flex-direction: column;
		border: 1px solid var(--border-soft);
		width: fit-content;
		min-width: 400px;
	}

	.lightbox-main-img {
		display: block;
		margin: 0 auto;
		width: auto;
		height: auto;
		max-width: 100%;
		max-height: calc(100vh - 160px);
		object-fit: contain;
		background: #000;
	}

	.lightbox-footer {
		padding: 8px 16px;
		display: flex;
		flex-direction: column;
		gap: 8px;
		background: var(--surface-1);
		flex-shrink: 0;
		border-top: 1px solid var(--border-soft);
	}

	.lightbox-navigation {
		display: flex;
		align-items: center;
		justify-content: center;
		gap: 12px;
		padding-bottom: 6px;
		border-bottom: 1px solid var(--border-soft);
	}

	.nav-btn-mini {
		background: var(--surface-2);
		border: 1px solid var(--border-soft);
		color: var(--text-primary);
		width: 28px;
		height: 28px;
		border-radius: 6px;
		display: flex;
		align-items: center;
		justify-content: center;
		cursor: pointer;
		transition: all 0.2s;
	}

	.nav-btn-mini:hover:not(:disabled) {
		background: var(--accent);
		color: var(--accent-text);
		border-color: var(--accent);
	}

	.nav-btn-mini:disabled {
		opacity: 0.3;
		cursor: not-allowed;
	}

	.nav-count {
		font-size: 12px;
		font-weight: 700;
		color: var(--text-secondary);
		min-width: 45px;
		text-align: center;
	}

	.lightbox-info-container {
		display: flex;
		justify-content: space-between;
		align-items: center;
		gap: 20px;
	}

	.lightbox-info {
		flex: 1;
		min-width: 0;
	}

	.rich-info-container {
		display: flex;
		flex-direction: column;
		gap: 6px;
	}

	.game-brand {
		display: flex;
		align-items: center;
		gap: 8px;
	}

	.game-icon {
		width: 24px;
		height: 24px;
		border-radius: 4px;
		object-fit: cover;
		background: var(--surface-2);
	}

	.brand-text {
		display: flex;
		align-items: baseline;
		gap: 8px;
	}

	.brand-text h3 {
		font-size: 14px;
		font-weight: 800;
		color: var(--text-primary);
	}

	.date-text {
		color: var(--text-muted);
		font-size: 11px;
	}

	.ach-brand {
		display: flex;
		align-items: center;
		gap: 12px;
		background: var(--surface-2);
		padding: 6px 12px;
		border-radius: 8px;
		border: 1px solid var(--border-soft);
		position: relative;
		min-height: 48px;
		width: fit-content;
		max-width: 100%;
	}

	.ach-icon {
		width: 36px;
		height: 36px;
		border-radius: 6px;
		object-fit: cover;
		box-shadow: 0 4px 10px rgba(0, 0, 0, 0.3);
		flex-shrink: 0;
	}

	.ach-details {
		display: flex;
		flex-direction: column;
		gap: 2px;
		flex: 1;
		min-width: 0;
		padding-right: 8px;
	}

	.ach-header {
		display: flex;
		align-items: center;
		flex-wrap: nowrap;
		gap: 6px;
		min-width: 0;
	}

	.ach-header h4 {
		font-size: 13px;
		font-weight: 700;
		color: var(--text-primary);
		white-space: nowrap;
		overflow: hidden;
		text-overflow: ellipsis;
	}

	.ach-badge {
		background: var(--accent);
		color: var(--accent-text);
		padding: 0px 5px;
		border-radius: 8px;
		font-size: 9px;
		font-weight: 800;
		text-transform: uppercase;
		letter-spacing: 0.5px;
		white-space: nowrap;
		flex-shrink: 0;
	}

	.ach-rarity {
		font-size: 9px;
		font-weight: 800;
		padding: 0px 5px;
		border-radius: 8px;
		text-transform: uppercase;
		letter-spacing: 0.5px;
		white-space: nowrap;
		flex-shrink: 0;
	}

	/* Rarity Colors for Brand Cards and Badges */
	.ach-brand.commune {
		border-color: rgba(156, 163, 175, 0.2);
		background: rgba(156, 163, 175, 0.03);
	}
	.ach-brand.peu-commune {
		border-color: rgba(61, 220, 132, 0.3);
		background: rgba(61, 220, 132, 0.03);
	}
	.ach-brand.tres-rare {
		border-color: rgba(74, 200, 255, 0.3);
		background: rgba(74, 200, 255, 0.03);
	}
	.ach-brand.rare {
		border-color: rgba(244, 184, 96, 0.3);
		background: rgba(244, 184, 96, 0.03);
	}
	.ach-brand.epique {
		border-color: rgba(168, 85, 247, 0.3);
		background: rgba(168, 85, 247, 0.03);
	}
	.ach-brand.legendaire {
		border-color: rgba(255, 216, 90, 0.3);
		background: rgba(255, 216, 90, 0.03);
	}
	.ach-brand.mythique {
		border-color: rgba(255, 59, 92, 0.3);
		background: rgba(255, 59, 92, 0.03);
	}

	.ach-rarity.commune {
		background: rgba(156, 163, 175, 0.1);
		color: #9ca3af;
	}
	.ach-rarity.peu-commune {
		background: rgba(61, 220, 132, 0.1);
		color: #3ddc84;
	}
	.ach-rarity.tres-rare {
		background: rgba(74, 200, 255, 0.1);
		color: #4ac8ff;
	}
	.ach-rarity.rare {
		background: rgba(244, 184, 96, 0.1);
		color: #f4b860;
	}
	.ach-rarity.epique {
		background: rgba(168, 85, 247, 0.1);
		color: #a855f7;
	}
	.ach-rarity.legendaire {
		background: rgba(255, 216, 90, 0.1);
		color: #ffd85a;
	}
	.ach-rarity.mythique {
		background: rgba(255, 59, 92, 0.1);
		color: #ff3b5c;
	}

	.ach-brand.commune .ring-fill {
		stroke: #9ca3af;
	}
	.ach-brand.peu-commune .ring-fill {
		stroke: #3ddc84;
	}
	.ach-brand.tres-rare .ring-fill {
		stroke: #4ac8ff;
	}
	.ach-brand.rare .ring-fill {
		stroke: #f4b860;
	}
	.ach-brand.epique .ring-fill {
		stroke: #a855f7;
	}
	.ach-brand.legendaire .ring-fill {
		stroke: #ffd85a;
	}
	.ach-brand.mythique .ring-fill {
		stroke: #ff3b5c;
	}

	.ach-pct {
		font-size: 10px;
		color: var(--text-muted);
		font-variant-numeric: tabular-nums;
		font-weight: 600;
		flex-shrink: 0;
	}

	.ach-desc {
		font-size: 11px;
		color: var(--text-secondary);
		line-height: 1.3;
		white-space: nowrap;
		overflow: hidden;
		text-overflow: ellipsis;
		display: block;
	}

	.ach-progress-circle {
		flex-shrink: 0;
		position: relative;
		width: 36px;
		height: 36px;
	}

	.progress-fraction {
		font-family: inherit;
		font-size: 9px;
		font-weight: 800;
		color: var(--text-primary);
		position: absolute;
		inset: 0;
		display: flex;
		align-items: center;
		justify-content: center;
	}

	.progress-ring {
		width: 36px;
		height: 36px;
		transform: rotate(-90deg);
	}

	.ring-bg {
		fill: none;
		stroke: var(--surface-3);
		stroke-width: 3.5;
	}
	.ring-fill {
		fill: none;
		stroke-width: 3.5;
		stroke-linecap: round;
		transition: stroke-dasharray 0.6s ease;
	}

	.ach-key {
		margin-top: 2px;
		font-size: 10px;
		color: var(--text-muted);
	}

	.ach-key code {
		background: var(--surface-2);
		padding: 1px 4px;
		border-radius: 4px;
		color: var(--accent);
	}

	.lightbox-actions {
		display: flex;
		gap: 8px;
	}

	.secondary-btn {
		background: var(--surface-2);
		border: 1px solid var(--border-soft);
		color: var(--text-primary);
		padding: 6px 12px;
		border-radius: 6px;
		font-weight: 600;
		cursor: pointer;
		font-size: 12px;
		transition: all 0.2s;
	}

	.secondary-btn:hover {
		background: var(--surface-hover);
	}

	.danger-btn {
		background: rgba(220, 38, 38, 0.1);
		border: 1px solid rgba(220, 38, 38, 0.2);
		color: #ef4444;
		padding: 6px 12px;
		border-radius: 6px;
		font-weight: 600;
		cursor: pointer;
		font-size: 12px;
		transition: all 0.2s;
	}

	.danger-btn:hover {
		background: #ef4444;
		color: white;
	}
</style>
