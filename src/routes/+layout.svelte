<script lang="ts">
    import './layout.css';
    import favicon from '$lib/assets/favicon.svg';
    import Settings from '$lib/Settings.svelte';
    import { loadSettings, settings } from '$lib/stores/settings.js';
    import { settingsOpen } from '$lib/stores/ui.js';
    import { syncSteamMetadata } from '$lib/stores/Games.js';
    import { onMount } from 'svelte';
    import { get } from 'svelte/store';

    let { children } = $props();

    onMount(() => {
        void (async () => {
            await loadSettings();
            await syncSteamMetadata(get(settings).steamApiKey);
        })();
    });

    function closeSettings(): void {
        settingsOpen.set(false);
    }
</script>

<svelte:head>
    <link rel="icon" href={favicon} />
</svelte:head>

{@render children()}

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
        background: #161616;
        border: 1px solid rgba(255, 255, 255, 0.08);
        border-radius: 18px;
        overflow: hidden;
        box-shadow: 0 24px 80px rgba(0, 0, 0, 0.55);
        z-index: 1;
    }
</style>
