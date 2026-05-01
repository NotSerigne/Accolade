<script lang="ts">
	// src/lib/Settings.svelte
	import { get } from 'svelte/store';
	import { open } from '@tauri-apps/plugin-dialog';
	import { settings, saveSettings, applyTheme, type AppSettings } from '$lib/stores/settings.js';
	import { settingsOpen } from '$lib/stores/ui.js';
	import { syncSteamMetadata } from '$lib/stores/Games.js';
	import { refreshSteamUser } from '$lib/stores/user.js';
	import {
		i18n,
		languageOptions,
		rarityLabelByIndex,
		type TranslationKey
	} from '$lib/stores/i18n.js';

	let draft = $state<AppSettings>({ ...get(settings) });

	let isSaving = $state(false);
	let saveError = $state('');
	let apiKeyVisible = $state(false);
	let sgdbKeyVisible = $state(false);
	let previewAudio = $state<HTMLAudioElement | null>(null);
	let previewingSound = $state('');

	const accentPresets = [
		{ labelKey: 'settings.accent.gold', value: '#c8a96e' },
		{ labelKey: 'settings.accent.blue', value: '#4ac8ff' },
		{ labelKey: 'settings.accent.green', value: '#3ddc84' },
		{ labelKey: 'settings.accent.purple', value: '#a78bfa' },
		{ labelKey: 'settings.accent.red', value: '#ff6b6b' },
		{ labelKey: 'settings.accent.orange', value: '#fb923c' }
	] as const;

	const soundOptions: Array<{ value: string; label?: string; labelKey?: TranslationKey }> = [
		{ value: 'none', labelKey: 'setup.sound.none' },
		{ value: 'PS4.mp3', label: 'PS4' },
		{ value: 'PS5.mp3', label: 'PS5' },
		{ value: 'PS5 Platinum.mp3', label: 'PS5 Platinum' },
		{ value: 'Steam.mp3', label: 'Steam' },
		{ value: 'Steamdeck.mp3', label: 'Steam Deck' },
		{ value: 'Windows 8.mp3', label: 'Windows 8' },
		{ value: 'Windows 10.mp3', label: 'Windows 10' },
		{ value: 'Windows 11.mp3', label: 'Windows 11' },
		{ value: 'Xbox.mp3', label: 'Xbox' },
		{ value: 'Xbox Rare.mp3', label: 'Xbox Rare' }
	];

	const positionOptions = [
		{ value: 'top-left', labelKey: 'setup.position.top-left', style: 'grid-area: 1 / 1 / 2 / 2;' },
		{
			value: 'top-center',
			labelKey: 'setup.position.top-center',
			style: 'grid-area: 1 / 2 / 2 / 3;'
		},
		{
			value: 'top-right',
			labelKey: 'setup.position.top-right',
			style: 'grid-area: 1 / 3 / 2 / 4;'
		},
		{
			value: 'bottom-left',
			labelKey: 'setup.position.bottom-left',
			style: 'grid-area: 3 / 1 / 4 / 2;'
		},
		{
			value: 'bottom-center',
			labelKey: 'setup.position.bottom-center',
			style: 'grid-area: 3 / 2 / 4 / 3;'
		},
		{
			value: 'bottom-right',
			labelKey: 'setup.position.bottom-right',
			style: 'grid-area: 3 / 3 / 4 / 4;'
		}
	] as const;

	$effect(() => {
		applyTheme({ ...draft });
	});

	$effect.pre(() => {
		// Update settings store when language changes in draft
		if (draft.language) {
			settings.set({ ...get(settings), language: draft.language });
		}
	});

	async function addPath(): Promise<void> {
		const selected = await open({ directory: true, multiple: false });
		if (typeof selected === 'string' && !draft.searchPaths.includes(selected)) {
			draft.searchPaths = [...draft.searchPaths, selected];
		}
	}

	function removePath(path: string): void {
		draft.searchPaths = draft.searchPaths.filter((p) => p !== path);
	}

	function setWindowPosition(value: AppSettings['windowPosition']): void {
		draft.windowPosition = value;
	}

	function previewSound(filename: string): void {
		if (previewAudio) {
			previewAudio.pause();
			previewAudio = null;
		}
		if (!filename || filename === 'none' || previewingSound === filename) {
			previewingSound = '';
			return;
		}
		const audio = new Audio(`/sounds/${encodeURIComponent(filename)}`);
		audio.volume = 0.7;
		previewAudio = audio;
		previewingSound = filename;
		audio.play().catch(() => {});
		audio.addEventListener('ended', () => {
			previewingSound = '';
			previewAudio = null;
		});
	}

	async function testNotification(): Promise<void> {
		try {
			const { Store } = await import('@tauri-apps/plugin-store');
			const store = await Store.load('settings.json');
			await store.set('windowPosition', draft.windowPosition);
			await store.set('notificationSound', draft.notificationSound);
			await store.save();

			const { invoke } = await import('@tauri-apps/api/core');
			await invoke('test_achievement_notif');
		} catch (error) {
			console.error('Test notification failed:', error);
			saveError = $i18n.t('setup.errorTest', {
				details: error instanceof Error ? error.message : String(error)
			});
		}
	}

	async function save(): Promise<void> {
		if (isSaving) return;

		isSaving = true;
		saveError = '';

		try {
			const next: AppSettings = {
				steamId: draft.steamId,
				steamApiKey: draft.steamApiKey,
				steamGridDbApiKey: draft.steamGridDbApiKey,
				searchPaths: [...draft.searchPaths],
				language: draft.language,
				setupCompleted: draft.setupCompleted,
				windowPosition: draft.windowPosition,
				notificationSound: draft.notificationSound,
				theme: draft.theme,
				accentColor: draft.accentColor,
				launchOnStartup: draft.launchOnStartup,
				startMinimized: draft.startMinimized,
				minimizeToTray: draft.minimizeToTray
			};
			await saveSettings(next);

			try {
				const { enable, disable } = await import('@tauri-apps/plugin-autostart');
				if (next.launchOnStartup) {
					await enable();
				} else {
					await disable();
				}
			} catch (e) {
				console.warn('Failed to configure autostart:', e);
			}

			await refreshSteamUser();
			await syncSteamMetadata(next.steamApiKey, next.steamGridDbApiKey);
			settingsOpen.set(false);
		} catch (error) {
			const details = error instanceof Error ? error.message : String(error);
			saveError = $i18n.t('setup.errorSave', { details });
			console.error('Failed to save settings:', error);
		} finally {
			isSaving = false;
		}
	}

	function close(): void {
		applyTheme({ ...get(settings) });
		settingsOpen.set(false);
	}
</script>

<div class="settings-page">
	<div class="settings-header">
		<h1 id="settings-title" class="settings-title">{$i18n.t('settings.title')}</h1>
		<div class="header-actions">
			<button
				class="icon-close-btn"
				onclick={close}
				title={$i18n.t('button.close')}
				disabled={isSaving}
			>
				<svg
					width="14"
					height="14"
					viewBox="0 0 24 24"
					fill="none"
					stroke="currentColor"
					stroke-width="2.5"
				>
					<line x1="18" y1="6" x2="6" y2="18" />
					<line x1="6" y1="6" x2="18" y2="18" />
				</svg>
			</button>
			<button class="save-btn" onclick={save} disabled={isSaving}>
				{isSaving ? $i18n.t('setup.saving') : $i18n.t('setup.save')}
			</button>
		</div>
	</div>

	{#if saveError}
		<div class="save-error" role="alert">{saveError}</div>
	{/if}

	<div class="settings-body">
		<section class="settings-section">
			<div class="section-label">{$i18n.t('setup.steam')}</div>

			<div class="setting-row">
				<div class="setting-info">
					<div class="setting-name">{$i18n.t('setup.steamId')}</div>
					<div class="setting-desc">{$i18n.t('settings.profile')}</div>
				</div>
				<input
					class="text-input"
					type="text"
					placeholder="7656119XXXXXXXXXX"
					bind:value={draft.steamId}
				/>
			</div>

			<div class="setting-row" style="margin-top: 14px;">
				<div class="setting-info">
					<div class="setting-name">{$i18n.t('setup.steamApiKey')}</div>
					<div class="setting-desc">{$i18n.t('settings.apiKeyNeeded')}</div>
				</div>
				<div class="api-key-wrap">
					<input
						class="text-input"
						type={apiKeyVisible ? 'text' : 'password'}
						placeholder="XXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXX"
						bind:value={draft.steamApiKey}
					/>
					<button
						class="toggle-btn"
						onclick={() => (apiKeyVisible = !apiKeyVisible)}
						title={apiKeyVisible ? $i18n.t('settings.apiKeyHide') : $i18n.t('settings.apiKeyShow')}
					>
						{#if apiKeyVisible}
							<svg
								width="14"
								height="14"
								viewBox="0 0 24 24"
								fill="none"
								stroke="currentColor"
								stroke-width="2"
							>
								<path
									d="M17.94 17.94A10.07 10.07 0 0 1 12 20c-7 0-11-8-11-8a18.45 18.45 0 0 1 5.06-5.94"
								/>
								<path d="M9.9 4.24A9.12 9.12 0 0 1 12 4c7 0 11 8 11 8a18.5 18.5 0 0 1-2.16 3.19" />
								<line x1="1" y1="1" x2="23" y2="23" />
							</svg>
						{:else}
							<svg
								width="14"
								height="14"
								viewBox="0 0 24 24"
								fill="none"
								stroke="currentColor"
								stroke-width="2"
							>
								<path d="M1 12s4-8 11-8 11 8 11 8-4 8-11 8-11-8-11-8z" />
								<circle cx="12" cy="12" r="3" />
							</svg>
						{/if}
					</button>
				</div>
			</div>

			<div class="setting-row" style="margin-top: 14px;">
				<div class="setting-info">
					<div class="setting-name">{$i18n.t('setup.sgdbApiKey')}</div>
					<div class="setting-desc">{$i18n.t('settings.apiKeyOptional')}</div>
				</div>
				<div class="api-key-wrap">
					<input
						class="text-input"
						type={sgdbKeyVisible ? 'text' : 'password'}
						placeholder="XXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXX"
						bind:value={draft.steamGridDbApiKey}
					/>
					<button
						class="toggle-btn"
						onclick={() => (sgdbKeyVisible = !sgdbKeyVisible)}
						title={sgdbKeyVisible ? $i18n.t('settings.apiKeyHide') : $i18n.t('settings.apiKeyShow')}
					>
						{#if sgdbKeyVisible}
							<svg
								width="14"
								height="14"
								viewBox="0 0 24 24"
								fill="none"
								stroke="currentColor"
								stroke-width="2"
							>
								<path
									d="M17.94 17.94A10.07 10.07 0 0 1 12 20c-7 0-11-8-11-8a18.45 18.45 0 0 1 5.06-5.94"
								/>
								<path d="M9.9 4.24A9.12 9.12 0 0 1 12 4c7 0 11 8 11 8a18.5 18.5 0 0 1-2.16 3.19" />
								<line x1="1" y1="1" x2="23" y2="23" />
							</svg>
						{:else}
							<svg
								width="14"
								height="14"
								viewBox="0 0 24 24"
								fill="none"
								stroke="currentColor"
								stroke-width="2"
							>
								<path d="M1 12s4-8 11-8 11 8 11 8-4 8-11 8-11-8-11-8z" />
								<circle cx="12" cy="12" r="3" />
							</svg>
						{/if}
					</button>
				</div>
			</div>
		</section>

		<div class="separator"></div>

		<section class="settings-section">
			<div class="section-label">{$i18n.t('settings.searchPaths')}</div>
			<div class="setting-desc-top">{$i18n.t('settings.searchPathsDesc')}</div>

			<div class="paths-list">
				{#each draft.searchPaths as path (path)}
					<div class="path-row">
						<svg
							width="13"
							height="13"
							viewBox="0 0 24 24"
							fill="none"
							stroke="currentColor"
							stroke-width="2"
							class="path-icon"
						>
							<path
								d="M22 19a2 2 0 0 1-2 2H4a2 2 0 0 1-2-2V5a2 2 0 0 1 2-2h5l2 3h9a2 2 0 0 1 2 2z"
							/>
						</svg>
						<span class="path-text">{path}</span>
						<button
							class="remove-btn"
							onclick={() => removePath(path)}
							title={$i18n.t('button.remove')}
						>
							<svg
								width="12"
								height="12"
								viewBox="0 0 24 24"
								fill="none"
								stroke="currentColor"
								stroke-width="2.5"
							>
								<line x1="18" y1="6" x2="6" y2="18" /><line x1="6" y1="6" x2="18" y2="18" />
							</svg>
						</button>
					</div>
				{/each}

				{#if draft.searchPaths.length === 0}
					<div class="paths-empty">{$i18n.t('settings.searchPathsEmpty')}</div>
				{/if}
			</div>

			<button class="add-path-btn" onclick={addPath}>
				<svg
					width="13"
					height="13"
					viewBox="0 0 24 24"
					fill="none"
					stroke="currentColor"
					stroke-width="2.5"
				>
					<line x1="12" y1="5" x2="12" y2="19" /><line x1="5" y1="12" x2="19" y2="12" />
				</svg>
				{$i18n.t('settings.addFolder')}
			</button>
		</section>

		<div class="separator"></div>

		<section class="settings-section">
			<div class="section-label">{$i18n.t('settings.language')}</div>
			<div class="setting-row">
				<div class="setting-info">
					<div class="setting-name">{$i18n.t('settings.languageInterface')}</div>
					<div class="setting-desc">{$i18n.t('settings.languageDesc')}</div>
				</div>
				<select class="sound-select" bind:value={draft.language}>
					{#each languageOptions as option (option.value)}
						<option value={option.value}>{option.label}</option>
					{/each}
				</select>
			</div>
		</section>

		<div class="separator"></div>

		<section class="settings-section">
			<div class="section-label">{$i18n.t('settings.position')}</div>
			<div class="setting-desc-top">{$i18n.t('settings.positionDesc')}</div>

			<div class="position-test-row">
				<div class="position-grid">
					<div class="grid-guide vertical"></div>
					<div class="grid-guide horizontal"></div>
					{#each positionOptions as pos (pos.value)}
						<button
							class="pos-btn"
							style={pos.style}
							class:active={draft.windowPosition === pos.value}
							onclick={() => setWindowPosition(pos.value)}
						>
							{$i18n.t(pos.labelKey)}
						</button>
					{/each}
				</div>

				<div class="position-side">
					<p class="position-side-label">{$i18n.t('settings.sound')}</p>
					<div class="sound-row">
						<select class="sound-select" bind:value={draft.notificationSound}>
							{#each soundOptions as opt (opt.value)}
								<option value={opt.value}>{opt.labelKey ? $i18n.t(opt.labelKey) : opt.label}</option
								>
							{/each}
						</select>
						<button
							class="sound-preview-btn"
							class:playing={previewingSound === draft.notificationSound &&
								draft.notificationSound !== 'none'}
							onclick={() => previewSound(draft.notificationSound)}
							title={previewingSound === draft.notificationSound
								? $i18n.t('settings.soundStop')
								: $i18n.t('settings.soundPlay')}
							disabled={draft.notificationSound === 'none'}
						>
							{#if previewingSound === draft.notificationSound && draft.notificationSound !== 'none'}
								<!-- Stop icon -->
								<svg width="12" height="12" viewBox="0 0 24 24" fill="currentColor">
									<rect x="4" y="4" width="16" height="16" rx="2" />
								</svg>
							{:else}
								<!-- Play icon -->
								<svg width="12" height="12" viewBox="0 0 24 24" fill="currentColor">
									<polygon points="5,3 19,12 5,21" />
								</svg>
							{/if}
						</button>
					</div>

					<p class="position-side-label" style="margin-top: 12px;">{$i18n.t('settings.test')}</p>
					<button class="test-btn" onclick={testNotification}>
						<svg
							width="13"
							height="13"
							viewBox="0 0 24 24"
							fill="none"
							stroke="currentColor"
							stroke-width="2"
						>
							<path
								d="M22 16.92v3a2 2 0 0 1-2.18 2 19.79 19.79 0 0 1-8.63-3.07 19.5 19.5 0 0 1-6-6 19.79 19.79 0 0 1-3.07-8.67A2 2 0 0 1 4.11 2h3a2 2 0 0 1 2 1.72 12.84 12.84 0 0 0 .7 2.81 2 2 0 0 1-.45 2.11L8.09 9.91a16 16 0 0 0 6 6l1.27-1.27a2 2 0 0 1 2.11-.45 12.84 12.84 0 0 0 2.81.7A2 2 0 0 1 22 16.92z"
							/>
						</svg>
						{$i18n.t('settings.testNotif')}
					</button>
				</div>
			</div>
		</section>

		<div class="separator"></div>

		<!-- ── Section Comportement ── -->
		<section class="settings-section">
			<div class="section-label">{$i18n.t('settings.behavior') || 'Comportement'}</div>

			<div class="setting-row">
				<div class="setting-info">
					<div class="setting-name">
						{$i18n.t('settings.launchOnStartup') || 'Lancer au démarrage de Windows'}
					</div>
				</div>
				<label class="toggle-switch">
					<input type="checkbox" bind:checked={draft.launchOnStartup} />
					<span class="slider"></span>
				</label>
			</div>

			<div class="setting-row" style="margin-top: 14px;">
				<div class="setting-info">
					<div class="setting-name">
						{$i18n.t('settings.startMinimized') || 'Démarrer en minimisé'}
					</div>
				</div>
				<label class="toggle-switch">
					<input type="checkbox" bind:checked={draft.startMinimized} />
					<span class="slider"></span>
				</label>
			</div>

			<div class="setting-row" style="margin-top: 14px;">
				<div class="setting-info">
					<div class="setting-name">
						{$i18n.t('settings.minimizeToTray') || "Minimiser l'app à la place de fermer"}
					</div>
				</div>
				<label class="toggle-switch">
					<input type="checkbox" bind:checked={draft.minimizeToTray} />
					<span class="slider"></span>
				</label>
			</div>
		</section>

		<div class="separator"></div>

		<!-- ── Section Thème ── -->
		<section class="settings-section">
			<div class="section-label">{$i18n.t('settings.appearance')}</div>

			<!-- Dark / Light -->
			<div class="setting-row">
				<div class="setting-info">
					<div class="setting-name">{$i18n.t('setup.theme')}</div>
				</div>
				<div class="theme-toggle">
					<button
						class="theme-btn"
						class:active={draft.theme === 'dark'}
						onclick={() => (draft.theme = 'dark')}
					>
						<svg
							width="13"
							height="13"
							viewBox="0 0 24 24"
							fill="none"
							stroke="currentColor"
							stroke-width="2"
						>
							<path d="M21 12.79A9 9 0 1 1 11.21 3 7 7 0 0 0 21 12.79z" />
						</svg>
						{$i18n.t('setup.theme.dark')}
					</button>
					<button
						class="theme-btn"
						class:active={draft.theme === 'light'}
						onclick={() => (draft.theme = 'light')}
					>
						<svg
							width="13"
							height="13"
							viewBox="0 0 24 24"
							fill="none"
							stroke="currentColor"
							stroke-width="2"
						>
							<circle cx="12" cy="12" r="5" />
							<line x1="12" y1="1" x2="12" y2="3" /><line x1="12" y1="21" x2="12" y2="23" />
							<line x1="4.22" y1="4.22" x2="5.64" y2="5.64" /><line
								x1="18.36"
								y1="18.36"
								x2="19.78"
								y2="19.78"
							/>
							<line x1="1" y1="12" x2="3" y2="12" /><line x1="21" y1="12" x2="23" y2="12" />
							<line x1="4.22" y1="19.78" x2="5.64" y2="18.36" /><line
								x1="18.36"
								y1="5.64"
								x2="19.78"
								y2="4.22"
							/>
						</svg>
						{$i18n.t('setup.theme.light')}
					</button>
					<button
						class="theme-btn"
						class:active={draft.theme === 'system'}
						onclick={() => (draft.theme = 'system')}
					>
						<svg
							width="13"
							height="13"
							viewBox="0 0 24 24"
							fill="none"
							stroke="currentColor"
							stroke-width="2"
						>
							<rect x="3" y="4" width="18" height="12" rx="2" />
							<path d="M8 20h8" />
							<path d="M12 16v4" />
						</svg>
						{$i18n.t('setup.theme.system')}
					</button>
				</div>
			</div>

			<!-- Couleur accent -->
			<div class="setting-row" style="margin-top: 20px;">
				<div class="setting-info">
					<div class="setting-name">{$i18n.t('setup.accent')}</div>
					<div class="setting-desc">{$i18n.t('settings.accentDesc')}</div>
				</div>
				<div class="accent-controls">
					<div class="accent-presets">
						{#each accentPresets as preset (preset.value)}
							<button
								class="accent-swatch"
								class:active={draft.accentColor === preset.value}
								style="background: {preset.value}"
								onclick={() => (draft.accentColor = preset.value)}
								title={$i18n.t(preset.labelKey)}
							></button>
						{/each}
					</div>
					<!-- Couleur custom -->
					<div class="custom-color-wrap">
						<input
							type="color"
							class="color-picker"
							bind:value={draft.accentColor}
							title={$i18n.t('settings.customColor')}
						/>
						<span class="color-hex">{draft.accentColor}</span>
					</div>
				</div>
			</div>

			<!-- Preview accent -->
			<div class="accent-preview" style="--preview-accent: {draft.accentColor}">
				<div class="preview-pill"></div>
				<span class="preview-badge">{rarityLabelByIndex($i18n.language, 1)} · 2.4%</span>
				<div class="preview-bar-track"><div class="preview-bar-fill"></div></div>
				<span class="preview-label">{$i18n.t('settings.preview')}</span>
			</div>
		</section>
	</div>
</div>

<style>
	.settings-page {
		height: 100%;
		display: flex;
		flex-direction: column;
		overflow: hidden;
		background: var(--bg-panel);
		color: var(--text-primary);
	}

	/* ── Header ── */
	.settings-header {
		display: flex;
		align-items: center;
		justify-content: space-between;
		padding: 24px 32px 20px;
		border-bottom: 1px solid var(--border-soft);
		flex-shrink: 0;
	}

	.header-actions {
		display: flex;
		align-items: center;
		gap: 10px;
	}

	.settings-title {
		font-size: 22px;
		font-weight: 800;
		color: var(--text-primary);
		letter-spacing: -0.4px;
	}

	.save-btn {
		height: 34px;
		padding: 0 18px;
		border-radius: 8px;
		border: none;
		background: var(--accent, #c8a96e);
		color: #111;
		font-size: 13px;
		font-weight: 700;
		cursor: pointer;
		display: flex;
		align-items: center;
		gap: 7px;
		transition:
			opacity 0.15s,
			background 0.2s;
		font-family: inherit;
	}
	.save-btn:hover {
		opacity: 0.85;
	}

	.icon-close-btn {
		width: 34px;
		height: 34px;
		border-radius: 8px;
		border: 1px solid var(--border-soft);
		background: var(--surface-2);
		color: var(--text-secondary);
		display: flex;
		align-items: center;
		justify-content: center;
		cursor: pointer;
		transition:
			background 0.12s,
			color 0.12s;
	}
	.icon-close-btn:hover {
		background: var(--surface-hover);
		color: var(--text-primary);
	}

	.icon-close-btn:disabled,
	.save-btn:disabled {
		opacity: 0.6;
		cursor: not-allowed;
	}

	.save-error {
		margin: 12px 32px 0;
		padding: 10px 12px;
		border: 1px solid rgba(255, 107, 107, 0.45);
		background: rgba(255, 107, 107, 0.12);
		color: #ffb4b4;
		border-radius: 8px;
		font-size: 12px;
	}

	/* ── Body scrollable ── */
	.settings-body {
		flex: 1;
		overflow-y: auto;
		padding: 28px 32px 40px;
		scrollbar-width: thin;
		scrollbar-color: rgba(255, 255, 255, 0.08) transparent;
		display: flex;
		flex-direction: column;
		gap: 0;
	}

	/* ── Sections ── */
	.settings-section {
		padding: 24px 0;
	}

	.section-label {
		font-size: 11px;
		font-weight: 700;
		color: var(--text-secondary);
		text-transform: uppercase;
		letter-spacing: 1.2px;
		margin-bottom: 18px;
	}

	.setting-desc-top {
		font-size: 12px;
		color: var(--text-muted);
		margin-top: -10px;
		margin-bottom: 16px;
	}

	.separator {
		height: 1px;
		background: var(--border-soft);
	}

	/* ── Rows ── */
	.setting-row {
		display: flex;
		align-items: center;
		justify-content: space-between;
		gap: 24px;
	}

	.setting-info {
		flex: 1;
		min-width: 0;
	}

	.setting-name {
		font-size: 14px;
		font-weight: 600;
		color: var(--text-primary);
		margin-bottom: 3px;
	}

	.setting-desc {
		font-size: 12px;
		color: var(--text-muted);
	}

	/* ── API Key ── */
	.api-key-wrap {
		display: flex;
		align-items: center;
		gap: 6px;
		flex-shrink: 0;
	}

	.text-input {
		width: 280px;
		height: 34px;
		background: var(--surface-2);
		border: 1px solid var(--border-soft);
		border-radius: 8px;
		padding: 0 12px;
		font-size: 13px;
		color: var(--text-primary);
		font-family: 'Courier New', monospace;
		outline: none;
		transition: border-color 0.15s;
	}
	.text-input:focus {
		border-color: var(--accent, #c8a96e);
	}
	.text-input::placeholder {
		color: var(--text-muted);
		font-family: inherit;
	}

	.toggle-btn {
		width: 34px;
		height: 34px;
		border-radius: 8px;
		background: var(--surface-2);
		border: 1px solid var(--border-soft);
		cursor: pointer;
		color: var(--text-muted);
		display: flex;
		align-items: center;
		justify-content: center;
		transition:
			background 0.12s,
			color 0.12s;
		flex-shrink: 0;
	}
	.toggle-btn:hover {
		background: var(--surface-hover);
		color: var(--text-primary);
	}

	/* ── Chemins ── */
	.paths-list {
		display: flex;
		flex-direction: column;
		gap: 6px;
		margin-bottom: 12px;
	}

	.path-row {
		display: flex;
		align-items: center;
		gap: 10px;
		background: var(--surface-2);
		border: 1px solid var(--border-soft);
		border-radius: 8px;
		padding: 8px 12px;
	}

	.path-icon {
		color: var(--text-muted);
		flex-shrink: 0;
	}

	.path-text {
		flex: 1;
		font-size: 12px;
		color: var(--text-secondary);
		white-space: nowrap;
		overflow: hidden;
		text-overflow: ellipsis;
		font-family: 'Courier New', monospace;
	}

	.remove-btn {
		width: 22px;
		height: 22px;
		border-radius: 4px;
		background: transparent;
		border: none;
		cursor: pointer;
		color: var(--text-muted);
		display: flex;
		align-items: center;
		justify-content: center;
		flex-shrink: 0;
		transition:
			background 0.12s,
			color 0.12s;
	}
	.remove-btn:hover {
		background: rgba(255, 80, 80, 0.15);
		color: #ff6b6b;
	}

	.paths-empty {
		font-size: 12px;
		color: var(--text-muted);
		font-style: italic;
		padding: 8px 0;
	}

	.add-path-btn {
		height: 32px;
		padding: 0 14px;
		border-radius: 8px;
		background: var(--surface-2);
		border: 1px dashed var(--border-soft);
		color: var(--text-muted);
		font-size: 13px;
		cursor: pointer;
		display: flex;
		align-items: center;
		gap: 7px;
		font-family: inherit;
		transition:
			background 0.12s,
			color 0.12s,
			border-color 0.12s;
	}
	.add-path-btn:hover {
		background: var(--surface-hover);
		color: var(--text-primary);
		border-color: color-mix(in srgb, var(--accent) 40%, var(--border-soft));
	}

	/* ── Position ── */
	.position-grid {
		position: relative;
		width: min(260px, 100%);
		aspect-ratio: 1;
		margin-top: 8px;
		padding: 10px;
		border-radius: 16px;
		border: 1px solid var(--border-soft);
		background: var(--surface-2);
		display: grid;
		grid-template-columns: 1fr 1fr 1fr;
		grid-template-rows: 1fr 1fr 1fr;
		gap: 4px;
	}

	.grid-guide {
		position: absolute;
		background: var(--border-soft);
		pointer-events: none;
	}

	.grid-guide.vertical {
		top: 10px;
		bottom: 10px;
		left: 50%;
		width: 1px;
		transform: translateX(-50%);
	}

	.grid-guide.horizontal {
		left: 10px;
		right: 10px;
		top: 50%;
		height: 1px;
		transform: translateY(-50%);
	}

	.pos-btn {
		width: 100%;
		height: 100%;
		min-height: 44px;
		border-radius: 8px;
		background: var(--surface-2);
		border: 1px solid var(--border-soft);
		color: var(--text-muted);
		font-size: 12px;
		cursor: pointer;
		font-family: inherit;
		transition:
			background 0.12s,
			color 0.12s,
			border-color 0.12s;
	}
	.pos-btn:hover {
		background: var(--surface-hover);
		color: var(--text-primary);
	}
	.pos-btn.active {
		background: color-mix(in srgb, var(--accent, #c8a96e) 16%, transparent);
		border-color: var(--accent, #c8a96e);
		color: var(--accent, #c8a96e);
		font-weight: 600;
	}

	/* ── Thème ── */
	.theme-toggle {
		display: flex;
		background: var(--surface-2);
		border-radius: 8px;
		padding: 3px;
		gap: 2px;
	}

	.theme-btn {
		height: 32px;
		padding: 0 16px;
		border-radius: 6px;
		border: none;
		background: transparent;
		color: var(--text-muted);
		font-size: 13px;
		cursor: pointer;
		display: flex;
		align-items: center;
		gap: 7px;
		font-family: inherit;
		transition:
			background 0.12s,
			color 0.12s;
	}
	.theme-btn:hover {
		color: var(--text-primary);
	}
	.theme-btn.active {
		background: var(--surface-hover);
		color: var(--text-primary);
	}

	/* ── Accent ── */
	.accent-controls {
		display: flex;
		flex-direction: column;
		align-items: flex-end;
		gap: 10px;
	}

	.accent-presets {
		display: flex;
		gap: 8px;
	}

	.accent-swatch {
		width: 26px;
		height: 26px;
		border-radius: 50%;
		border: 2px solid transparent;
		cursor: pointer;
		transition:
			transform 0.15s,
			border-color 0.15s;
		flex-shrink: 0;
	}
	.accent-swatch:hover {
		transform: scale(1.15);
	}
	.accent-swatch.active {
		border-color: var(--text-primary);
		transform: scale(1.1);
	}

	.custom-color-wrap {
		display: flex;
		align-items: center;
		gap: 8px;
	}

	.color-picker {
		width: 32px;
		height: 32px;
		border-radius: 6px;
		border: 1px solid var(--border-soft);
		background: transparent;
		cursor: pointer;
		padding: 2px;
	}

	.color-hex {
		font-size: 12px;
		color: var(--text-muted);
		font-family: 'Courier New', monospace;
		min-width: 60px;
	}

	/* ── Preview ── */
	.accent-preview {
		margin-top: 20px;
		background: var(--surface-2);
		border: 1px solid var(--border-soft);
		border-radius: 10px;
		padding: 14px 18px;
		display: flex;
		align-items: center;
		gap: 14px;
	}

	.preview-pill {
		width: 3px;
		height: 24px;
		border-radius: 2px;
		background: var(--preview-accent, #c8a96e);
		flex-shrink: 0;
	}

	.preview-badge {
		font-size: 10px;
		font-weight: 600;
		padding: 2px 8px;
		border-radius: 4px;
		background: color-mix(in srgb, var(--preview-accent, #c8a96e) 15%, transparent);
		color: var(--preview-accent, #c8a96e);
		text-transform: uppercase;
		letter-spacing: 0.5px;
		flex-shrink: 0;
	}

	.preview-bar-track {
		flex: 1;
		height: 4px;
		background: var(--border-soft);
		border-radius: 2px;
		overflow: hidden;
	}

	.preview-bar-fill {
		width: 35%;
		height: 100%;
		background: var(--preview-accent, #c8a96e);
		border-radius: 2px;
	}

	.preview-label {
		font-size: 11px;
		color: var(--text-muted);
		flex-shrink: 0;
	}

	.test-btn {
		height: 36px;
		padding: 0 16px;
		border-radius: 8px;
		background: color-mix(in srgb, #4ac8ff 12%, transparent);
		border: 1px solid #4ac8ff;
		color: #4ac8ff;
		font-size: 13px;
		cursor: pointer;
		display: flex;
		align-items: center;
		gap: 7px;
		font-family: inherit;
		font-weight: 500;
		transition:
			background 0.12s,
			color 0.12s,
			border-color 0.12s;
	}

	.test-btn:hover {
		background: color-mix(in srgb, #4ac8ff 18%, transparent);
		border-color: #4ac8ff;
	}

	.test-btn:active {
		transform: scale(0.98);
	}

	.position-test-row {
		display: flex;
		align-items: flex-start;
		gap: 24px;
		margin-top: 8px;
	}

	.position-side {
		display: flex;
		flex-direction: column;
		gap: 10px;
		padding-top: 4px;
	}

	.position-side-label {
		font-size: 12px;
		color: var(--text-muted);
		margin: 0;
		text-transform: uppercase;
		letter-spacing: 0.08em;
	}

	/* ── Son ── */
	.sound-row {
		display: flex;
		align-items: center;
		gap: 6px;
	}

	.sound-select {
		flex: 1;
		height: 34px;
		padding: 0 10px;
		border-radius: 8px;
		background: var(--surface-2);
		border: 1px solid var(--border-soft);
		color: var(--text-primary);
		font-size: 13px;
		font-family: inherit;
		cursor: pointer;
		outline: none;
		appearance: none;
		-webkit-appearance: none;
		background-image: url("data:image/svg+xml,%3Csvg xmlns='http://www.w3.org/2000/svg' width='12' height='12' fill='none' stroke='%236a7080' stroke-width='2' stroke-linecap='round' stroke-linejoin='round'%3E%3Cpath d='m3 4.5 3 3 3-3'/%3E%3C/svg%3E");
		background-repeat: no-repeat;
		background-position: right 10px center;
		padding-right: 28px;
		transition:
			border-color 0.12s,
			background-color 0.12s;
	}
	.sound-select:focus {
		border-color: var(--accent, #c8a96e);
		background-color: var(--surface-hover);
	}
	.sound-select option {
		background: #ffffff;
		color: #111111;
	}

	.sound-preview-btn {
		width: 34px;
		height: 34px;
		flex-shrink: 0;
		border-radius: 8px;
		background: var(--surface-2);
		border: 1px solid var(--border-soft);
		color: var(--text-muted);
		display: flex;
		align-items: center;
		justify-content: center;
		cursor: pointer;
		transition:
			background 0.12s,
			color 0.12s,
			border-color 0.12s;
	}
	.sound-preview-btn:hover:not(:disabled) {
		background: var(--surface-hover);
		color: var(--text-primary);
	}
	.sound-preview-btn.playing {
		background: color-mix(in srgb, var(--accent, #c8a96e) 16%, transparent);
		border-color: var(--accent, #c8a96e);
		color: var(--accent, #c8a96e);
	}
	.sound-preview-btn:disabled {
		opacity: 0.3;
		cursor: not-allowed;
	}

	/* ── Toggles ── */
	.toggle-switch {
		position: relative;
		display: inline-block;
		width: 40px;
		height: 22px;
		flex-shrink: 0;
	}
	.toggle-switch input {
		opacity: 0;
		width: 0;
		height: 0;
	}
	.slider {
		position: absolute;
		cursor: pointer;
		top: 0;
		left: 0;
		right: 0;
		bottom: 0;
		background-color: var(--surface-2);
		border: 1px solid var(--border-soft);
		transition: 0.2s;
		border-radius: 22px;
	}
	.slider:before {
		position: absolute;
		content: '';
		height: 14px;
		width: 14px;
		left: 3px;
		bottom: 3px;
		background-color: var(--text-muted);
		transition: 0.2s;
		border-radius: 50%;
	}
	input:checked + .slider {
		background-color: color-mix(in srgb, var(--accent, #c8a96e) 20%, var(--surface-2));
		border-color: var(--accent, #c8a96e);
	}
	input:checked + .slider:before {
		transform: translateX(18px);
		background-color: var(--accent, #c8a96e);
	}
</style>
