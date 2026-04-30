<script lang="ts">
    import { get } from 'svelte/store';
    import { open } from '@tauri-apps/plugin-dialog';
    import { settings, saveSettings, applyTheme, type AppSettings } from '$lib/stores/settings.js';
    import { settingsOpen } from '$lib/stores/ui.js';
    import { syncSteamMetadata } from '$lib/stores/Games.js';
    import { refreshSteamUser } from '$lib/stores/user.js';

    let draft = $state<AppSettings>({ ...get(settings) });

    let isSaving = $state(false);
    let saveError = $state('');
    let apiKeyVisible = $state(false);
    let sgdbKeyVisible = $state(false);
    let previewAudio = $state<HTMLAudioElement | null>(null);
    let previewingSound = $state('');

    // Preset accent colors
    const accentPresets = [
        { label: 'Or',       value: '#c8a96e' },
        { label: 'Bleu',     value: '#4ac8ff' },
        { label: 'Vert',     value: '#3ddc84' },
        { label: 'Violet',   value: '#a78bfa' },
        { label: 'Rouge',    value: '#ff6b6b' },
        { label: 'Orange',   value: '#fb923c' },
    ];

    const soundOptions = [
        { value: 'none',              label: 'Aucun son' },
        { value: 'PS4.mp3',           label: 'PS4' },
        { value: 'PS5.mp3',           label: 'PS5' },
        { value: 'PS5 Platinum.mp3',  label: 'PS5 Platinum' },
        { value: 'Steam.mp3',         label: 'Steam' },
        { value: 'Steamdeck.mp3',     label: 'Steam Deck' },
        { value: 'Windows 8.mp3',     label: 'Windows 8' },
        { value: 'Windows 10.mp3',    label: 'Windows 10' },
        { value: 'Windows 11.mp3',    label: 'Windows 11' },
        { value: 'Xbox.mp3',          label: 'Xbox' },
        { value: 'Xbox Rare.mp3',     label: 'Xbox Rare' },
    ] as const;

    const positionOptions = [
        { value: 'top-left',     label: 'Haut gauche',  style: 'grid-area: 1 / 1 / 2 / 2;' },
        { value: 'top-center',   label: 'Haut centre',  style: 'grid-area: 1 / 2 / 2 / 3;' },
        { value: 'top-right',    label: 'Haut droite',  style: 'grid-area: 1 / 3 / 2 / 4;' },
        { value: 'bottom-left',  label: 'Bas gauche',   style: 'grid-area: 3 / 1 / 4 / 2;' },
        { value: 'bottom-center', label: 'Bas centre',  style: 'grid-area: 3 / 2 / 4 / 3;' },
        { value: 'bottom-right', label: 'Bas droite',   style: 'grid-area: 3 / 3 / 4 / 4;' },
    ] as const;


    $effect(() => {
        applyTheme({ ...draft });
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
        audio.addEventListener('ended', () => { previewingSound = ''; previewAudio = null; });
    }

    async function testNotification(): Promise<void> {
        try {
            // Ecriture immediate des settings de notif dans le store pour le test en temps reel
            const { Store } = await import('@tauri-apps/plugin-store');
            const store = await Store.load('settings.json');
            await store.set('windowPosition', draft.windowPosition);
            await store.set('notificationSound', draft.notificationSound);
            await store.save();

            const { invoke } = await import('@tauri-apps/api/core');
            await invoke('test_achievement_notif');
        } catch (error) {
            console.error('Test notification failed:', error);
            saveError = `Erreur lors du test: ${error instanceof Error ? error.message : String(error)}`;
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
                windowPosition: draft.windowPosition,
                notificationSound: draft.notificationSound,
                theme: draft.theme,
                accentColor: draft.accentColor,
            };
            await saveSettings(next);
            await refreshSteamUser();
            await syncSteamMetadata(next.steamApiKey, next.steamGridDbApiKey);
            settingsOpen.set(false);
        } catch (error) {
            const details = error instanceof Error ? error.message : String(error);
            saveError = `Echec de l'enregistrement: ${details}`;
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

    <!-- Header -->
    <div class="settings-header">
        <h1 id="settings-title" class="settings-title">Paramètres</h1>
        <div class="header-actions">
            <button class="icon-close-btn" onclick={close} title="Fermer" disabled={isSaving}>
                <svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2.5">
                    <line x1="18" y1="6" x2="6" y2="18"/>
                    <line x1="6" y1="6" x2="18" y2="18"/>
                </svg>
            </button>
            <button class="save-btn" onclick={save} disabled={isSaving}>
                {isSaving ? 'Enregistrement...' : 'Enregistrer'}
            </button>
        </div>
    </div>

    {#if saveError}
        <div class="save-error" role="alert">{saveError}</div>
    {/if}

    <div class="settings-body">

        <!-- ── Section Steam ── -->
        <section class="settings-section">
            <div class="section-label">Steam</div>

            <div class="setting-row">
                <div class="setting-info">
                    <div class="setting-name">Steam ID (64 bits)</div>
                    <div class="setting-desc">Pour afficher votre profil et vos jeux possédés</div>
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
                    <div class="setting-name">Clé API Steam</div>
                    <div class="setting-desc">Nécessaire pour récupérer les succès et les infos profil</div>
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
                                    title={apiKeyVisible ? 'Masquer' : 'Afficher'}
                            >
                                {#if apiKeyVisible}
                                    <svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
                                        <path d="M17.94 17.94A10.07 10.07 0 0 1 12 20c-7 0-11-8-11-8a18.45 18.45 0 0 1 5.06-5.94"/>
                                        <path d="M9.9 4.24A9.12 9.12 0 0 1 12 4c7 0 11 8 11 8a18.5 18.5 0 0 1-2.16 3.19"/>
                                        <line x1="1" y1="1" x2="23" y2="23"/>
                                    </svg>
                                {:else}
                                    <svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
                                        <path d="M1 12s4-8 11-8 11 8 11 8-4 8-11 8-11-8-11-8z"/>
                                        <circle cx="12" cy="12" r="3"/>
                                    </svg>
                                {/if}
                            </button>
                        </div>
                    </div>

                    <div class="setting-row" style="margin-top: 14px;">
                        <div class="setting-info">
                            <div class="setting-name">Clé API SteamGridDB</div>
                            <div class="setting-desc">Optionnel — pour les icones de jeux manquantes</div>
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
                                    title={sgdbKeyVisible ? 'Masquer' : 'Afficher'}
                            >
                                {#if sgdbKeyVisible}
                                    <svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
                                        <path d="M17.94 17.94A10.07 10.07 0 0 1 12 20c-7 0-11-8-11-8a18.45 18.45 0 0 1 5.06-5.94"/>
                                        <path d="M9.9 4.24A9.12 9.12 0 0 1 12 4c7 0 11 8 11 8a18.5 18.5 0 0 1-2.16 3.19"/>
                                        <line x1="1" y1="1" x2="23" y2="23"/>
                                    </svg>
                                {:else}
                                    <svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
                                        <path d="M1 12s4-8 11-8 11 8 11 8-4 8-11 8-11-8-11-8z"/>
                                        <circle cx="12" cy="12" r="3"/>
                                    </svg>
                                {/if}
                            </button>
                        </div>
                    </div>
                </section>

        <div class="separator"></div>

        <!-- ── Section Chemins ── -->
        <section class="settings-section">
            <div class="section-label">Chemins de recherche</div>
            <div class="setting-desc-top">Dossiers supplémentaires où Accolade cherche les fichiers de succès</div>

            <div class="paths-list">
                {#each draft.searchPaths as path (path)}
                    <div class="path-row">
                        <svg width="13" height="13" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" class="path-icon">
                            <path d="M22 19a2 2 0 0 1-2 2H4a2 2 0 0 1-2-2V5a2 2 0 0 1 2-2h5l2 3h9a2 2 0 0 1 2 2z"/>
                        </svg>
                        <span class="path-text">{path}</span>
                        <button class="remove-btn" onclick={() => removePath(path)} title="Supprimer">
                            <svg width="12" height="12" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2.5">
                                <line x1="18" y1="6" x2="6" y2="18"/><line x1="6" y1="6" x2="18" y2="18"/>
                            </svg>
                        </button>
                    </div>
                {/each}

                {#if draft.searchPaths.length === 0}
                    <div class="paths-empty">Aucun chemin ajouté — seuls les emplacements par défaut sont scannés</div>
                {/if}
            </div>

            <button class="add-path-btn" onclick={addPath}>
                <svg width="13" height="13" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2.5">
                    <line x1="12" y1="5" x2="12" y2="19"/><line x1="5" y1="12" x2="19" y2="12"/>
                </svg>
                Ajouter un dossier
            </button>
        </section>

        <div class="separator"></div>

        <!-- ── Section Position et Test ── -->
        <section class="settings-section">
            <div class="section-label">Position de la fenêtre</div>
            <div class="setting-desc-top">Où apparaît la notification HUD lors d'un succès</div>

            <div class="position-test-row">
                <div class="position-grid">
                    <div class="grid-guide vertical"></div>
                    <div class="grid-guide horizontal"></div>
                    {#each positionOptions as pos}
                        <button
                                class="pos-btn"
                                style={pos.style}
                                class:active={draft.windowPosition === pos.value}
                                onclick={() => setWindowPosition(pos.value)}
                        >
                            {pos.label}
                        </button>
                    {/each}
                </div>

                <div class="position-side">
                    <p class="position-side-label">Son</p>
                    <div class="sound-row">
                        <select class="sound-select" bind:value={draft.notificationSound}>
                            {#each soundOptions as opt}
                                <option value={opt.value}>{opt.label}</option>
                            {/each}
                        </select>
                        <button
                            class="sound-preview-btn"
                            class:playing={previewingSound === draft.notificationSound && draft.notificationSound !== 'none'}
                            onclick={() => previewSound(draft.notificationSound)}
                            title={previewingSound === draft.notificationSound ? 'Stopper' : 'Ecouter'}
                            disabled={draft.notificationSound === 'none'}
                        >
                            {#if previewingSound === draft.notificationSound && draft.notificationSound !== 'none'}
                                <!-- Stop icon -->
                                <svg width="12" height="12" viewBox="0 0 24 24" fill="currentColor">
                                    <rect x="4" y="4" width="16" height="16" rx="2"/>
                                </svg>
                            {:else}
                                <!-- Play icon -->
                                <svg width="12" height="12" viewBox="0 0 24 24" fill="currentColor">
                                    <polygon points="5,3 19,12 5,21"/>
                                </svg>
                            {/if}
                        </button>
                    </div>

                    <p class="position-side-label" style="margin-top: 12px;">Tester</p>
                    <button class="test-btn" onclick={testNotification}>
                        <svg width="13" height="13" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
                            <path d="M22 16.92v3a2 2 0 0 1-2.18 2 19.79 19.79 0 0 1-8.63-3.07 19.5 19.5 0 0 1-6-6 19.79 19.79 0 0 1-3.07-8.67A2 2 0 0 1 4.11 2h3a2 2 0 0 1 2 1.72 12.84 12.84 0 0 0 .7 2.81 2 2 0 0 1-.45 2.11L8.09 9.91a16 16 0 0 0 6 6l1.27-1.27a2 2 0 0 1 2.11-.45 12.84 12.84 0 0 0 2.81.7A2 2 0 0 1 22 16.92z"/>
                        </svg>
                        Tester la notification
                    </button>
                </div>
            </div>
        </section>

        <div class="separator"></div>

        <!-- ── Section Thème ── -->
        <section class="settings-section">
            <div class="section-label">Apparence</div>

            <!-- Dark / Light -->
            <div class="setting-row">
                <div class="setting-info">
                    <div class="setting-name">Thème</div>
                </div>
                <div class="theme-toggle">
                    <button
                            class="theme-btn"
                            class:active={draft.theme === 'dark'}
                            onclick={() => (draft.theme = 'dark')}
                    >
                        <svg width="13" height="13" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
                            <path d="M21 12.79A9 9 0 1 1 11.21 3 7 7 0 0 0 21 12.79z"/>
                        </svg>
                        Sombre
                    </button>
                    <button
                            class="theme-btn"
                            class:active={draft.theme === 'light'}
                            onclick={() => (draft.theme = 'light')}
                    >
                        <svg width="13" height="13" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
                            <circle cx="12" cy="12" r="5"/>
                            <line x1="12" y1="1" x2="12" y2="3"/><line x1="12" y1="21" x2="12" y2="23"/>
                            <line x1="4.22" y1="4.22" x2="5.64" y2="5.64"/><line x1="18.36" y1="18.36" x2="19.78" y2="19.78"/>
                            <line x1="1" y1="12" x2="3" y2="12"/><line x1="21" y1="12" x2="23" y2="12"/>
                            <line x1="4.22" y1="19.78" x2="5.64" y2="18.36"/><line x1="18.36" y1="5.64" x2="19.78" y2="4.22"/>
                        </svg>
                        Clair
                    </button>
                </div>
            </div>

            <!-- Couleur accent -->
            <div class="setting-row" style="margin-top: 20px;">
                <div class="setting-info">
                    <div class="setting-name">Couleur accent</div>
                    <div class="setting-desc">Utilisée pour les highlights, pills et badges</div>
                </div>
                <div class="accent-controls">
                    <div class="accent-presets">
                        {#each accentPresets as preset}
                            <button
                                    class="accent-swatch"
                                    class:active={draft.accentColor === preset.value}
                                    style="background: {preset.value}"
                                    onclick={() => (draft.accentColor = preset.value)}
                                    title={preset.label}
                            ></button>
                        {/each}
                    </div>
                    <!-- Couleur custom -->
                    <div class="custom-color-wrap">
                        <input
                                type="color"
                                class="color-picker"
                                bind:value={draft.accentColor}
                                title="Couleur personnalisée"
                        />
                        <span class="color-hex">{draft.accentColor}</span>
                    </div>
                </div>
            </div>

            <!-- Preview accent -->
            <div class="accent-preview" style="--preview-accent: {draft.accentColor}">
                <div class="preview-pill"></div>
                <span class="preview-badge">Légendaire · 2.4%</span>
                <div class="preview-bar-track"><div class="preview-bar-fill"></div></div>
                <span class="preview-label">Aperçu</span>
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
    }

    /* ── Header ── */
    .settings-header {
        display: flex;
        align-items: center;
        justify-content: space-between;
        padding: 24px 32px 20px;
        border-bottom: 1px solid rgba(255,255,255,0.06);
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
        color: #fff;
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
        transition: opacity 0.15s, background 0.2s;
        font-family: inherit;
    }
    .save-btn:hover { opacity: 0.85; }

    .icon-close-btn {
        width: 34px;
        height: 34px;
        border-radius: 8px;
        border: 1px solid rgba(255,255,255,0.1);
        background: rgba(255,255,255,0.05);
        color: rgba(255,255,255,0.6);
        display: flex;
        align-items: center;
        justify-content: center;
        cursor: pointer;
        transition: background 0.12s, color 0.12s;
    }
    .icon-close-btn:hover { background: rgba(255,255,255,0.1); color: #fff; }

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
        scrollbar-color: rgba(255,255,255,0.08) transparent;
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
        color: rgba(255,255,255,0.3);
        text-transform: uppercase;
        letter-spacing: 1.2px;
        margin-bottom: 18px;
    }

    .setting-desc-top {
        font-size: 12px;
        color: #6a7080;
        margin-top: -10px;
        margin-bottom: 16px;
    }

    .separator {
        height: 1px;
        background: rgba(255,255,255,0.05);
    }

    /* ── Rows ── */
    .setting-row {
        display: flex;
        align-items: center;
        justify-content: space-between;
        gap: 24px;
    }

    .setting-info { flex: 1; min-width: 0; }

    .setting-name {
        font-size: 14px;
        font-weight: 600;
        color: #fff;
        margin-bottom: 3px;
    }

    .setting-desc { font-size: 12px; color: #6a7080; }

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
        background: rgba(255,255,255,0.05);
        border: 1px solid rgba(255,255,255,0.1);
        border-radius: 8px;
        padding: 0 12px;
        font-size: 13px;
        color: #fff;
        font-family: 'Courier New', monospace;
        outline: none;
        transition: border-color 0.15s;
    }
    .text-input:focus { border-color: var(--accent, #c8a96e); }
    .text-input::placeholder { color: rgba(255,255,255,0.2); font-family: inherit; }

    .toggle-btn {
        width: 34px;
        height: 34px;
        border-radius: 8px;
        background: rgba(255,255,255,0.05);
        border: 1px solid rgba(255,255,255,0.1);
        cursor: pointer;
        color: #6a7080;
        display: flex;
        align-items: center;
        justify-content: center;
        transition: background 0.12s, color 0.12s;
        flex-shrink: 0;
    }
    .toggle-btn:hover { background: rgba(255,255,255,0.1); color: #fff; }

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
        background: rgba(255,255,255,0.04);
        border: 1px solid rgba(255,255,255,0.07);
        border-radius: 8px;
        padding: 8px 12px;
    }

    .path-icon { color: #6a7080; flex-shrink: 0; }

    .path-text {
        flex: 1;
        font-size: 12px;
        color: #aaa;
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
        color: rgba(255,255,255,0.25);
        display: flex;
        align-items: center;
        justify-content: center;
        flex-shrink: 0;
        transition: background 0.12s, color 0.12s;
    }
    .remove-btn:hover { background: rgba(255,80,80,0.15); color: #ff6b6b; }

    .paths-empty {
        font-size: 12px;
        color: rgba(255,255,255,0.2);
        font-style: italic;
        padding: 8px 0;
    }

    .add-path-btn {
        height: 32px;
        padding: 0 14px;
        border-radius: 8px;
        background: rgba(255,255,255,0.05);
        border: 1px dashed rgba(255,255,255,0.15);
        color: #6a7080;
        font-size: 13px;
        cursor: pointer;
        display: flex;
        align-items: center;
        gap: 7px;
        font-family: inherit;
        transition: background 0.12s, color 0.12s, border-color 0.12s;
    }
    .add-path-btn:hover {
        background: rgba(255,255,255,0.08);
        color: #fff;
        border-color: rgba(255,255,255,0.25);
    }

    /* ── Position ── */
    .position-grid {
        position: relative;
        width: min(260px, 100%);
        aspect-ratio: 1;
        margin-top: 8px;
        padding: 10px;
        border-radius: 16px;
        border: 1px solid rgba(255,255,255,0.08);
        background: rgba(255,255,255,0.03);
        display: grid;
        grid-template-columns: 1fr 1fr 1fr;
        grid-template-rows: 1fr 1fr 1fr;
        gap: 4px;
    }

    .grid-guide {
        position: absolute;
        background: rgba(255,255,255,0.06);
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
        background: rgba(255,255,255,0.04);
        border: 1px solid rgba(255,255,255,0.08);
        color: #6a7080;
        font-size: 12px;
        cursor: pointer;
        font-family: inherit;
        transition: background 0.12s, color 0.12s, border-color 0.12s;
    }
    .pos-btn:hover { background: rgba(255,255,255,0.08); color: #fff; }
    .pos-btn.active {
        background: color-mix(in srgb, var(--accent, #c8a96e) 16%, transparent);
        border-color: var(--accent, #c8a96e);
        color: var(--accent, #c8a96e);
        font-weight: 600;
    }


    /* ── Thème ── */
    .theme-toggle {
        display: flex;
        background: rgba(255,255,255,0.04);
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
        color: #6a7080;
        font-size: 13px;
        cursor: pointer;
        display: flex;
        align-items: center;
        gap: 7px;
        font-family: inherit;
        transition: background 0.12s, color 0.12s;
    }
    .theme-btn:hover { color: #fff; }
    .theme-btn.active { background: rgba(255,255,255,0.1); color: #fff; }

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
        transition: transform 0.15s, border-color 0.15s;
        flex-shrink: 0;
    }
    .accent-swatch:hover { transform: scale(1.15); }
    .accent-swatch.active {
        border-color: #fff;
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
        border: 1px solid rgba(255,255,255,0.1);
        background: transparent;
        cursor: pointer;
        padding: 2px;
    }

    .color-hex {
        font-size: 12px;
        color: #6a7080;
        font-family: 'Courier New', monospace;
        min-width: 60px;
    }

    /* ── Preview ── */
    .accent-preview {
        margin-top: 20px;
        background: rgba(255,255,255,0.03);
        border: 1px solid rgba(255,255,255,0.06);
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
        background: rgba(255,255,255,0.06);
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
        color: rgba(255,255,255,0.2);
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
        transition: background 0.12s, color 0.12s, border-color 0.12s;
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
        color: #6a7080;
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
        background: rgba(255,255,255,0.05);
        border: 1px solid rgba(255,255,255,0.1);
        color: #d0d4de;
        font-size: 13px;
        font-family: inherit;
        cursor: pointer;
        outline: none;
        appearance: none;
        -webkit-appearance: none;
        background-image: url("data:image/svg+xml,%3Csvg xmlns='http://www.w3.org/2000/svg' width='12' height='12' viewBox='0 0 24 24' fill='none' stroke='%236a7080' stroke-width='2'%3E%3Cpolyline points='6 9 12 15 18 9'/%3E%3C/svg%3E");
        background-repeat: no-repeat;
        background-position: right 10px center;
        padding-right: 28px;
        transition: border-color 0.12s, background-color 0.12s;
    }
    .sound-select:focus {
        border-color: var(--accent, #c8a96e);
        background-color: rgba(255,255,255,0.07);
    }
    .sound-select option {
        background: #1a1c24;
        color: #d0d4de;
    }

    .sound-preview-btn {
        width: 34px;
        height: 34px;
        flex-shrink: 0;
        border-radius: 8px;
        background: rgba(255,255,255,0.05);
        border: 1px solid rgba(255,255,255,0.1);
        color: #6a7080;
        display: flex;
        align-items: center;
        justify-content: center;
        cursor: pointer;
        transition: background 0.12s, color 0.12s, border-color 0.12s;
    }
    .sound-preview-btn:hover:not(:disabled) {
        background: rgba(255,255,255,0.1);
        color: #fff;
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

</style>
