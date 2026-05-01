<script lang="ts">
    import { get } from 'svelte/store';
    import { onDestroy } from 'svelte';
    import { goto } from '$app/navigation';
    import { resolve } from '$app/paths';
    import { i18n, languageOptions } from '$lib/stores/i18n.js';
    import { applyTheme, saveSettings, settings, type AppSettings } from '$lib/stores/settings.js';

    let draft = $state<AppSettings>({ ...get(settings) });
    let isSaving = $state(false);
    let saveError = $state('');
    let saveSuccess = $state('');
    let previewAudio = $state<HTMLAudioElement | null>(null);
    let previewingSound = $state('');

    const accentPresets = ['#c8a96e', '#4ac8ff', '#3ddc84', '#a78bfa', '#ff6b6b', '#fb923c'] as const;
    const soundValues = ['none', 'PS4.mp3', 'PS5.mp3', 'PS5 Platinum.mp3', 'Steam.mp3', 'Steamdeck.mp3', 'Windows 8.mp3', 'Windows 10.mp3', 'Windows 11.mp3', 'Xbox.mp3', 'Xbox Rare.mp3'] as const;
    const positionValues = ['top-left', 'top-center', 'top-right', 'bottom-left', 'bottom-center', 'bottom-right'] as const;
    const positionLabelKeys = {
        'top-left': 'setup.position.top-left',
        'top-center': 'setup.position.top-center',
        'top-right': 'setup.position.top-right',
        'bottom-left': 'setup.position.bottom-left',
        'bottom-center': 'setup.position.bottom-center',
        'bottom-right': 'setup.position.bottom-right'
    } as const;
    const steamAccountUrl = 'https://store.steampowered.com/account/';
    const steamApiKeyUrl = 'https://steamcommunity.com/dev/apikey';
    const steamGridDbApiUrl = 'https://www.steamgriddb.com/profile/preferences/api';

    let soundOptions = $derived.by(() => {
        return soundValues.map((value) => ({
            value,
            label: value === 'none' ? $i18n.t('setup.sound.none') : value.replace('.mp3', '')
        }));
    });

    let positionOptions = $derived.by(() => {
        return positionValues.map((value) => ({
            value,
            label: $i18n.t(positionLabelKeys[value]),
            style:
                value === 'top-left'
                    ? 'grid-area: 1 / 1 / 2 / 2;'
                    : value === 'top-center'
                        ? 'grid-area: 1 / 2 / 2 / 3;'
                        : value === 'top-right'
                            ? 'grid-area: 1 / 3 / 2 / 4;'
                            : value === 'bottom-left'
                                ? 'grid-area: 3 / 1 / 4 / 2;'
                                : value === 'bottom-center'
                                    ? 'grid-area: 3 / 2 / 4 / 3;'
                                    : 'grid-area: 3 / 3 / 4 / 4;'
        }));
    });

    $effect(() => {
        applyTheme({ ...draft });
    });

    onDestroy(() => {
        applyTheme({ ...get(settings) });
    });

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
            saveError = $i18n.t('setup.errorTest', { details: error instanceof Error ? error.message : String(error) });
        }
    }

    async function save(): Promise<void> {
        if (isSaving) return;

        isSaving = true;
        saveError = '';
        saveSuccess = '';

        try {
            const next: AppSettings = {
                ...draft,
                setupCompleted: true,
                searchPaths: [...draft.searchPaths]
            };
            await saveSettings(next);
            saveSuccess = $i18n.t('setup.saved');
            await goto(resolve('/'));
        } catch (error) {
            saveError = $i18n.t('setup.errorSave', { details: error instanceof Error ? error.message : String(error) });
        } finally {
            isSaving = false;
        }
    }
</script>

<main class="setup-page">
    <header class="page-header">
        <h1>{$i18n.t('setup.title')}</h1>
        <p>{$i18n.t('setup.subtitle')}</p>
    </header>

    {#if saveError}
        <div class="feedback error">{saveError}</div>
    {/if}
    {#if saveSuccess}
        <div class="feedback success">{saveSuccess}</div>
    {/if}

    <section class="section">
        <h2>{$i18n.t('setup.steam')}</h2>
        <div class="field">
            <label for="steamId">{$i18n.t('setup.steamId')}</label>
            <input id="steamId" class="control mono" type="text" bind:value={draft.steamId} placeholder="7656119XXXXXXXXXX" />
            <a class="setup-link" href={steamAccountUrl} target="_blank" rel="noreferrer">
                {$i18n.t('setup.link.steamId')}
            </a>
        </div>
        <div class="field" style="margin-top: 12px;">
            <label for="steamApiKey">{$i18n.t('setup.steamApiKey')}</label>
            <input id="steamApiKey" class="control mono" type="password" bind:value={draft.steamApiKey} placeholder="XXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXX" />
            <a class="setup-link" href={steamApiKeyUrl} target="_blank" rel="noreferrer">
                {$i18n.t('setup.link.steamApiKey')}
            </a>
        </div>
        <div class="field" style="margin-top: 12px;">
            <label for="sgdbApiKey">{$i18n.t('setup.sgdbApiKey')}</label>
            <input id="sgdbApiKey" class="control mono" type="password" bind:value={draft.steamGridDbApiKey} placeholder="XXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXX" />
            <a class="setup-link" href={steamGridDbApiUrl} target="_blank" rel="noreferrer">
                {$i18n.t('setup.link.sgdbApiKey')}
            </a>
        </div>
    </section>

    <section class="section">
        <h2>{$i18n.t('setup.language')}</h2>
        <select class="control" bind:value={draft.language}>
            {#each languageOptions as option (option.value)}
                <option value={option.value}>{option.label}</option>
            {/each}
        </select>
    </section>

    <section class="section">
        <h2>{$i18n.t('setup.notifications')}</h2>
        <div class="row">
            <fieldset class="field">
                <legend>{$i18n.t('setup.position')}</legend>
                <div class="position-grid">
                    {#each positionOptions as pos (pos.value)}
                        <button
                            type="button"
                            class="pos-btn"
                            style={pos.style}
                            class:active={draft.windowPosition === pos.value}
                            onclick={() => (draft.windowPosition = pos.value as AppSettings['windowPosition'])}
                            aria-label={pos.label}
                        >
                            {pos.label}
                        </button>
                    {/each}
                </div>
            </fieldset>

            <fieldset class="field">
                <legend>{$i18n.t('setup.sound')}</legend>
                <div class="sound-row">
                    <select class="control" bind:value={draft.notificationSound}>
                        {#each soundOptions as option (option.value)}
                            <option value={option.value}>{option.label}</option>
                        {/each}
                    </select>
                    <button
                        type="button"
                        class="icon-btn"
                        class:active={previewingSound === draft.notificationSound && draft.notificationSound !== 'none'}
                        onclick={() => previewSound(draft.notificationSound)}
                        disabled={draft.notificationSound === 'none'}
                        aria-label={previewingSound === draft.notificationSound && draft.notificationSound !== 'none' ? 'Stop preview' : 'Play preview'}
                    >
                        {#if previewingSound === draft.notificationSound && draft.notificationSound !== 'none'}
                            ■
                        {:else}
                            ▶
                        {/if}
                    </button>
                </div>
                <button type="button" class="secondary-btn" onclick={testNotification}>
                    {$i18n.t('setup.test')}
                </button>
            </fieldset>
        </div>
    </section>

    <section class="section">
        <h2>{$i18n.t('setup.appearance')}</h2>
        <div class="row">
            <fieldset class="field">
                <legend>{$i18n.t('setup.theme')}</legend>
                <div class="theme-toggle">
                    <button type="button" class:active={draft.theme === 'dark'} onclick={() => (draft.theme = 'dark')}>
                        {$i18n.t('setup.theme.dark')}
                    </button>
                    <button type="button" class:active={draft.theme === 'light'} onclick={() => (draft.theme = 'light')}>
                        {$i18n.t('setup.theme.light')}
                    </button>
                    <button type="button" class:active={draft.theme === 'system'} onclick={() => (draft.theme = 'system')}>
                        {$i18n.t('setup.theme.system')}
                    </button>
                </div>
            </fieldset>

            <fieldset class="field">
                <legend>{$i18n.t('setup.accent')}</legend>
                <div class="accent-row">
                    {#each accentPresets as preset (preset)}
                        <button
                            type="button"
                            class="swatch"
                            class:active={draft.accentColor === preset}
                            style="background: {preset};"
                            onclick={() => (draft.accentColor = preset)}
                            aria-label={preset === '#c8a96e' ? 'Gold' : preset === '#4ac8ff' ? 'Cyan' : preset === '#3ddc84' ? 'Green' : preset === '#a78bfa' ? 'Purple' : preset === '#ff6b6b' ? 'Red' : preset === '#fb923c' ? 'Orange' : preset}
                        ></button>
                    {/each}
                    <input id="accentColorPicker" type="color" bind:value={draft.accentColor} aria-label="Custom accent color" />
                </div>
            </fieldset>
        </div>
    </section>

    <footer class="footer">
        <button type="button" class="save-btn" onclick={save} disabled={isSaving}>
            {isSaving ? $i18n.t('setup.saving') : $i18n.t('setup.save')}
        </button>
    </footer>
</main>

<style>
    .setup-page {
        height: 100%;
        overflow-y: auto;
        padding: 28px;
        background: var(--bg-panel);
        color: var(--text-primary);
        border-radius: 12px;
    }

    .page-header h1 {
        margin: 0;
        font-size: 28px;
    }

    .page-header p {
        margin: 8px 0 20px;
        color: var(--text-muted);
        font-size: 13px;
    }

    .feedback {
        padding: 10px 12px;
        border-radius: 8px;
        margin-bottom: 12px;
        font-size: 12px;
    }

    .feedback.error {
        border: 1px solid rgba(255, 107, 107, 0.4);
        background: rgba(255, 107, 107, 0.1);
        color: #ffb4b4;
    }

    .feedback.success {
        border: 1px solid rgba(61, 220, 132, 0.4);
        background: rgba(61, 220, 132, 0.1);
        color: #78f0ae;
    }

    .section {
        background: var(--surface-1);
        border: 1px solid var(--border-soft);
        border-radius: 12px;
        padding: 16px;
        margin-bottom: 14px;
    }

    .section h2 {
        margin: 0 0 14px;
        font-size: 14px;
        color: var(--text-secondary);
        text-transform: uppercase;
        letter-spacing: 0.06em;
    }

    .row {
        display: grid;
        grid-template-columns: 1fr 1fr;
        gap: 16px;
    }

    .field {
        display: flex;
        flex-direction: column;
        gap: 8px;
    }

    .field label {
        font-size: 12px;
        color: var(--text-muted);
    }

    .field,
    fieldset.field {
        display: flex;
        flex-direction: column;
        gap: 8px;
        border: none;
        padding: 0;
        margin: 0;
    }

    .field label,
    fieldset.field legend {
        font-size: 12px;
        color: var(--text-muted);
    }

    .control {
        height: 36px;
        border-radius: 8px;
        border: 1px solid var(--border-soft);
        background: var(--surface-2);
        color: var(--text-primary);
        padding: 0 10px;
        font-family: inherit;
    }

    .control option {
        background: #ffffff;
        color: #111111;
    }

    .mono {
        font-family: 'Courier New', monospace;
    }

    .setup-link {
        font-size: 12px;
        color: var(--accent);
        text-decoration: underline;
        text-underline-offset: 2px;
    }

    .position-grid {
        display: grid;
        grid-template-columns: repeat(3, 1fr);
        grid-template-rows: repeat(3, 1fr);
        gap: 6px;
        height: 170px;
        background: var(--surface-2);
        border: 1px solid var(--border-soft);
        border-radius: 10px;
        padding: 8px;
    }

    .pos-btn {
        border: 1px solid var(--border-soft);
        border-radius: 8px;
        background: var(--surface-3);
        color: var(--text-muted);
        font-size: 11px;
        cursor: pointer;
    }

    .pos-btn.active {
        border-color: var(--accent);
        color: var(--accent);
        background: color-mix(in srgb, var(--accent) 16%, transparent);
    }

    .sound-row,
    .accent-row {
        display: flex;
        gap: 8px;
        align-items: center;
    }

    .sound-row .control {
        flex: 1;
    }

    .icon-btn {
        width: 36px;
        height: 36px;
        border-radius: 8px;
        border: 1px solid var(--border-soft);
        background: var(--surface-2);
        color: var(--text-secondary);
        cursor: pointer;
    }

    .icon-btn.active {
        color: var(--accent);
        border-color: var(--accent);
    }

    .secondary-btn {
        align-self: flex-start;
        height: 34px;
        border-radius: 8px;
        border: 1px solid rgba(74, 200, 255, 0.7);
        color: #4ac8ff;
        background: color-mix(in srgb, #4ac8ff 12%, transparent);
        padding: 0 12px;
        font-family: inherit;
        cursor: pointer;
    }

    .theme-toggle {
        display: flex;
        gap: 6px;
    }

    .theme-toggle button {
        height: 34px;
        border-radius: 8px;
        border: 1px solid var(--border-soft);
        background: var(--surface-2);
        color: var(--text-muted);
        padding: 0 14px;
        font-family: inherit;
        cursor: pointer;
    }

    .theme-toggle button.active {
        border-color: var(--accent);
        color: var(--accent);
        background: color-mix(in srgb, var(--accent) 12%, transparent);
    }

    .swatch {
        width: 24px;
        height: 24px;
        border-radius: 50%;
        border: 2px solid transparent;
        cursor: pointer;
    }

    .swatch.active {
        border-color: var(--text-primary);
    }

    .accent-row input[type='color'] {
        width: 34px;
        height: 34px;
        border: 1px solid var(--border-soft);
        border-radius: 8px;
        background: transparent;
        padding: 2px;
    }

    .footer {
        display: flex;
        justify-content: flex-end;
        margin-top: 16px;
    }

    .save-btn {
        height: 36px;
        padding: 0 16px;
        border: 0;
        border-radius: 8px;
        background: var(--accent);
        color: #111;
        font-weight: 700;
        cursor: pointer;
    }

    .save-btn:disabled {
        opacity: 0.6;
        cursor: not-allowed;
    }
</style>
