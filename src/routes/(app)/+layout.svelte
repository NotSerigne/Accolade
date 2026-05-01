<script lang="ts">
    // src/routes/(app)/+layout.svelte
    import './layout.css';
    import favicon from '$lib/assets/favicon.svg';
    import Settings from '$lib/Settings.svelte';
    import { loadSettings, settings } from '$lib/stores/settings.js';
    import { settingsOpen, watcherActive } from '$lib/stores/ui.js';
    import { setupAchievementsRealtimeSync, syncSteamMetadata } from '$lib/stores/Games.js';
    import { refreshSteamUser } from '$lib/stores/user.js';
    import { onMount } from 'svelte';
    import { get } from 'svelte/store';
    import { listen } from '@tauri-apps/api/event';
    import { goto } from '$app/navigation';
    import { resolve } from '$app/paths';
    import { page } from '$app/state';
    import Sidebar from '$lib/Sidebar.svelte';
    import Topbar from '$lib/Topbar.svelte';

    let { children } = $props();
    let isSetupRoute = $derived(page.url.pathname.startsWith('/setup'));

    function hasEmptyApiKeys(): boolean {
        const s = get(settings);
        return !s.steamApiKey.trim() && !s.steamGridDbApiKey.trim();
    }

    onMount(() => {
        setupAchievementsRealtimeSync();

        const unlisten = listen<boolean>('watcher-status', (event) => {
            watcherActive.set(event.payload);
        });

        void (async () => {
            await loadSettings();
            const s = get(settings);
            const needsSetup = !s.setupCompleted || hasEmptyApiKeys();

            if (needsSetup && !isSetupRoute) {
                await goto(resolve('/setup/'));
                return;
            }

            if (!needsSetup && isSetupRoute) {
                await goto(resolve('/'));
                return;
            }

            await refreshSteamUser();
            await syncSteamMetadata(s.steamApiKey, s.steamGridDbApiKey);
        })();

        return () => {
            unlisten.then(u => u());
        };
    });

    function closeSettings(): void {
        settingsOpen.set(false);
    }
</script>

<svelte:head>
    <link rel="icon" href={favicon} />
</svelte:head>

<div class="app-container" class:setup={isSetupRoute}>
    {#if !isSetupRoute}
        <div class="sidebar-slot">
            <Sidebar />
        </div>
        <div class="topbar-slot">
            <Topbar />
        </div>
    {/if}
    <div class="content-slot">
        {@render children()}
    </div>
</div>

{#if $settingsOpen}
    <div class="settings-overlay">
        <button
                type="button"
                class="settings-backdrop"
                aria-label="Fermer les paramètres"
                onclick={closeSettings}
        ></button>
        <div class="settings-modal" role="dialog" aria-modal="true" aria-labelledby="settings-title">
            <Settings />
        </div>
    </div>
{/if}

<style>
    .app-container {
        display: grid;
        grid-template-columns: 72px 1fr;
        grid-template-rows: 64px 1fr;
        height: 100vh;
        gap: 0;
        padding: 0;
        overflow: hidden;
        background: var(--bg-app);
    }

    .sidebar-slot {
        grid-column: 1 / 2;
        grid-row: 1 / 3;
    }

    .topbar-slot {
        grid-column: 2 / 3;
        grid-row: 1 / 2;
        padding: 8px 8px 4px 0;
    }

    .content-slot {
        grid-column: 2 / 3;
        grid-row: 2 / 3;
        padding: 4px 8px 8px 0;
        overflow: hidden;
        display: flex;
        flex-direction: column;
    }

    .app-container.setup {
        grid-template-columns: 1fr;
        grid-template-rows: 1fr;
        padding: 8px;
    }

    .app-container.setup .content-slot {
        grid-column: 1 / 2;
        grid-row: 1 / 2;
        padding: 0;
    }

    .settings-overlay {
        position: fixed;
        inset: 0;
        background: rgba(0, 0, 0, 0.58);
        backdrop-filter: blur(8px);
        display: flex;
        align-items: center;
        justify-content: center;
        padding: 24px;
        z-index: 1000;
    }

    .settings-backdrop {
        position: absolute;
        inset: 0;
        border: 0;
        padding: 0;
        margin: 0;
        background: transparent;
        cursor: default;
    }

    .settings-modal {
        position: relative;
        width: min(1100px, 100%);
        height: min(88vh, 860px);
        background: var(--bg-panel);
        border: 1px solid var(--border-soft);
        border-radius: 18px;
        overflow: hidden;
        box-shadow: 0 24px 80px rgba(0, 0, 0, 0.55);
        z-index: 1;
    }
</style>
