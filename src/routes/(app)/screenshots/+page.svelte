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

		return () => {
			unlisten.then((f) => f());
		};
	});

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

	function getRarityClass(rarity: string | null) {
		if (!rarity) return '';
		const normalized = rarity.toLowerCase().trim();
		if (normalized.includes('commun')) return 'commune';
		if (normalized.includes('peu') || normalized.includes('uncommon')) return 'peu-commune';
		if (normalized.includes('très rare') || normalized.includes('very rare')) return 'tres-rare';
		if (normalized.includes('rare')) return 'rare';
		if (normalized.includes('épique') || normalized.includes('epic')) return 'epique';
		if (normalized.includes('légendaire') || normalized.includes('legendary')) return 'legendaire';
		if (normalized.includes('mythique') || normalized.includes('mythic')) return 'mythique';
		return 'commune';
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
			<img src={convertFileSrc(selectedScreenshot.path)} alt={selectedScreenshot.filename} />
			<div class="lightbox-footer">
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
								<div class="ach-brand">
									<img class="ach-icon" src={ach.icon} alt="Achievement icon" />
									<div class="ach-details">
										<div class="ach-header">
											<h4>{ach.name || ach.key}</h4>
											{#if achIndex}
												<span class="ach-badge">Succès n°{achIndex}</span>
											{/if}
											{#if ach.rarity}
												<span class="ach-rarity {getRarityClass(ach.rarity)}">{ach.rarity}</span>
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
		background: rgba(0, 0, 0, 0.9);
		z-index: 2000;
		display: flex;
		align-items: center;
		justify-content: center;
		padding: 40px;
		backdrop-filter: blur(8px);
	}

	.lightbox-content {
		max-width: 100%;
		max-height: 100%;
		background: var(--bg-main);
		border-radius: 16px;
		overflow: hidden;
		box-shadow: 0 30px 60px rgba(0, 0, 0, 0.8);
		display: flex;
		flex-direction: column;
		border: 1px solid var(--border-soft);
	}

	.lightbox-content img {
		max-width: 100%;
		min-height: 0;
		flex: 1 1 auto;
		object-fit: contain;
	}

	.lightbox-footer {
		padding: 24px 32px;
		display: flex;
		justify-content: space-between;
		align-items: center;
		background: var(--surface-1);
		flex-shrink: 0;
	}

	.rich-info-container {
		display: flex;
		flex-direction: column;
		gap: 16px;
	}

	.game-brand {
		display: flex;
		align-items: center;
		gap: 12px;
	}

	.game-icon {
		width: 40px;
		height: 40px;
		border-radius: 8px;
		object-fit: cover;
		background: var(--surface-2);
	}

	.brand-text h3 {
		font-size: 18px;
		font-weight: 800;
		color: var(--text-primary);
		margin-bottom: 2px;
	}

	.date-text {
		color: var(--text-muted);
		font-size: 13px;
	}

	.ach-brand {
		display: flex;
		align-items: flex-start;
		gap: 16px;
		background: var(--surface-2);
		padding: 16px;
		border-radius: 12px;
		border: 1px solid var(--border-soft);
	}

	.ach-icon {
		width: 64px;
		height: 64px;
		border-radius: 12px;
		object-fit: cover;
		box-shadow: 0 4px 12px rgba(0, 0, 0, 0.3);
	}

	.ach-details {
		display: flex;
		flex-direction: column;
		gap: 6px;
	}

	.ach-header {
		display: flex;
		align-items: center;
		flex-wrap: wrap;
		gap: 8px;
	}

	.ach-header h4 {
		font-size: 16px;
		font-weight: 700;
		color: var(--text-primary);
	}

	.ach-badge {
		background: var(--accent);
		color: var(--accent-text);
		padding: 2px 8px;
		border-radius: 12px;
		font-size: 11px;
		font-weight: 800;
		text-transform: uppercase;
		letter-spacing: 0.5px;
	}

	.ach-rarity {
		font-size: 11px;
		font-weight: 700;
		padding: 2px 8px;
		border-radius: 12px;
		text-transform: uppercase;
		letter-spacing: 0.5px;
	}

	.ach-rarity.commune {
		background: rgba(156, 163, 175, 0.15);
		color: #9ca3af;
	}

	.ach-rarity.peu-commune {
		background: rgba(59, 130, 246, 0.15);
		color: #60a5fa;
	}

	.ach-rarity.tres-rare {
		background: rgba(139, 92, 246, 0.15);
		color: #a78bfa;
	}

	.ach-rarity.rare {
		background: rgba(168, 85, 247, 0.15);
		color: #c084fc;
	}

	.ach-rarity.epique {
		background: rgba(236, 72, 153, 0.15);
		color: #f472b6;
	}

	.ach-rarity.legendaire {
		background: rgba(234, 179, 8, 0.15);
		color: #facc15;
	}

	.ach-rarity.mythique {
		background: rgba(239, 68, 68, 0.15);
		color: #f87171;
	}

	.ach-pct {
		font-size: 12px;
		color: var(--text-muted);
		font-variant-numeric: tabular-nums;
	}

	.ach-desc {
		font-size: 14px;
		color: var(--text-secondary);
		line-height: 1.4;
		max-width: 500px;
	}

	.ach-key {
		margin-top: 8px;
		font-size: 12px;
		color: var(--text-muted);
	}

	.ach-key code {
		background: var(--surface-2);
		padding: 2px 6px;
		border-radius: 4px;
		color: var(--accent);
	}

	.lightbox-actions {
		display: flex;
		gap: 12px;
	}

	.secondary-btn {
		background: var(--surface-2);
		border: 1px solid var(--border-soft);
		color: var(--text-primary);
		padding: 10px 20px;
		border-radius: 8px;
		font-weight: 600;
		cursor: pointer;
		transition: all 0.2s;
	}

	.secondary-btn:hover {
		background: var(--surface-hover);
	}

	.danger-btn {
		background: rgba(220, 38, 38, 0.1);
		border: 1px solid rgba(220, 38, 38, 0.2);
		color: #ef4444;
		padding: 10px 20px;
		border-radius: 8px;
		font-weight: 600;
		cursor: pointer;
		transition: all 0.2s;
	}

	.danger-btn:hover {
		background: #ef4444;
		color: white;
	}
</style>
