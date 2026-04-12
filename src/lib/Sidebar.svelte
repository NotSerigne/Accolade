<script lang="ts">
    import { games, selectedGameId, loadGames, totalUnlockedAchievements, type Game } from '$lib/stores/Games.js';
    import { settingsOpen } from '$lib/stores/ui.js';
    import { onMount } from 'svelte';
    import { goto } from '$app/navigation';
    import { page } from '$app/state';

    onMount(() => loadGames());

    function selectGame(id: number): void {
        selectedGameId.set(id);
        settingsOpen.set(false);
        goto(`/games/${id}`);
    }

    function goHome(): void {
        selectedGameId.set(null);
        settingsOpen.set(false);
        goto('/');
    }

    function goSettings(): void {
        settingsOpen.set(true);
    }

    function initials(name: string): string {
        return (name || '?')
            .split(' ')
            .slice(0, 2)
            .map((w: string) => w[0])
            .join('')
            .toUpperCase();
    }

    function progress(game: Game): string {
        const total = game.achievements_total || game.achievements?.length || 0;
        if (total === 0) return '';
        const unlocked = game.achievements?.filter((a) => a.unlocked).length ?? 0;
        return `${unlocked}/${total}`;
    }

    function handleImgError(e: Event): void {
        const target = e.target as HTMLImageElement;
        target.style.display = 'none';
        const fallback = target.nextElementSibling as HTMLElement | null;
        if (fallback) fallback.style.display = 'flex';
    }

    function iconUrl(game: Game): string {
        if (game.game_icon_url) return game.game_icon_url;
        if (game.game_icon) {
            return `https://media.steampowered.com/steamcommunity/public/images/apps/${game.steam_id}/${game.game_icon}.jpg`;
        }
        return '';
    }

    let pathname = $derived(String(page.url.pathname));
    let isHomeActive = $derived(pathname === '/');
    let isSettingsActive = $derived($settingsOpen);

</script>

<aside class="sidebar">
    <!-- Home -->
    <div class="nav-wrap">
        <div class="pill" class:visible={isHomeActive}></div>
        <button
                class="game-slot home-slot"
                class:active={isHomeActive}
                onclick={goHome}
                title="Dashboard"
        >
            <svg width="20" height="20" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2.2">
                <path d="M3 9l9-7 9 7v11a2 2 0 0 1-2 2H5a2 2 0 0 1-2-2z"/>
                <polyline points="9 22 9 12 15 12 15 22"/>
            </svg>
        </button>
    </div>

    <div class="divider"></div>

    <!-- Icônes des jeux détectés -->
    <div class="games-list">
        {#each $games as game (game.steam_id)}
            {@const isActive = $selectedGameId === game.steam_id}
            <div class="nav-wrap">
                <div class="pill" class:visible={isActive}></div>
                <button
                        class="game-slot"
                        class:active={isActive}
                        onclick={() => selectGame(game.steam_id)}
                        title="{game.name || game.steam_id}{progress(game) ? ' · ' + progress(game) : ''}"
                >
                    {#if iconUrl(game)}
                        <img
                                src={iconUrl(game)}
                                alt={game.name}
                                class="game-icon-img"
                                onerror={handleImgError}
                        />
                        <span class="game-icon-fallback" style="display:none">
                            {initials(game.name || String(game.steam_id))}
                        </span>
                    {:else}
                        <span class="game-icon-fallback">
                            {initials(game.name || String(game.steam_id))}
                        </span>
                    {/if}
                </button>
            </div>
        {/each}

        {#if $games.length === 0}
            <div class="empty-hint">Aucun<br/>jeu</div>
        {/if}
    </div>

    <!-- User panel flottant style Discord -->
    <div class="user-panel">
        <div class="avatar">SG</div>
        <div class="user-info">
            <div class="user-name">Serigne</div>
            <div class="user-meta">{$totalUnlockedAchievements} succès · 0 platines</div>
        </div>
        <div class="user-actions">
            <div class="watcher-dot" title="Watcher actif"></div>
            <button
                    class="icon-btn"
                    class:active={isSettingsActive}
                    onclick={goSettings}
                    title="Paramètres"
            >
                <svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
                    <circle cx="12" cy="12" r="3"/>
                    <path d="M19.4 15a1.65 1.65 0 0 0 .33 1.82l.06.06a2 2 0 0 1-2.83 2.83l-.06-.06a1.65 1.65 0 0 0-1.82-.33 1.65 1.65 0 0 0-1 1.51V21a2 2 0 0 1-4 0v-.09A1.65 1.65 0 0 0 9 19.4a1.65 1.65 0 0 0-1.82.33l-.06.06a2 2 0 0 1-2.83-2.83l.06-.06A1.65 1.65 0 0 0 4.68 15a1.65 1.65 0 0 0-1.51-1H3a2 2 0 0 1 0-4h.09A1.65 1.65 0 0 0 4.6 9a1.65 1.65 0 0 0-.33-1.82l-.06-.06a2 2 0 0 1 2.83-2.83l.06.06A1.65 1.65 0 0 0 9 4.68a1.65 1.65 0 0 0 1-1.51V3a2 2 0 0 1 4 0v.09a1.65 1.65 0 0 0 1 1.51 1.65 1.65 0 0 0 1.82-.33l.06-.06a2 2 0 0 1 2.83 2.83l-.06.06A1.65 1.65 0 0 0 19.4 9a1.65 1.65 0 0 0 1.51 1H21a2 2 0 0 1 0 4h-.09a1.65 1.65 0 0 0-1.51 1z"/>
                </svg>
            </button>
        </div>
    </div>
</aside>

<style>
    .sidebar {
        grid-column: 1 / 2;
        grid-row: 1 / 3;
        background: #0e0e0e;
        border-radius: 12px;
        display: flex;
        flex-direction: column;
        align-items: center;
        padding: 12px 0 0;
        position: relative;
        overflow: visible;
    }

    .nav-wrap {
        position: relative;
        width: 100%;
        display: flex;
        align-items: center;
        justify-content: center;
        flex-shrink: 0;
    }

    .pill {
        position: absolute;
        left: 0;
        width: 3px;
        height: 0;
        background: var(--accent, #c8a96e);
        border-radius: 0 3px 3px 0;
        transition: height 0.18s cubic-bezier(.4,0,.2,1);
        pointer-events: none;
    }
    .pill.visible { height: 22px; }

    .game-slot {
        width: 48px;
        height: 48px;
        border-radius: 50%;
        background: #1e1e1e;
        border: none;
        cursor: pointer;
        display: flex;
        align-items: center;
        justify-content: center;
        overflow: hidden;
        transition:
                border-radius 0.2s cubic-bezier(.4,0,.2,1),
                background 0.15s,
                box-shadow 0.15s;
        flex-shrink: 0;
        margin: 3px 0;
        color: rgba(255,255,255,0.5);
    }

    .game-slot:hover {
        border-radius: 30%;
        background: #2a2a2a;
        box-shadow: 0 0 0 2px color-mix(in srgb, var(--accent, #c8a96e) 30%, transparent);
    }

    .game-slot.active {
        border-radius: 30%;
        box-shadow: 0 0 0 2px var(--accent, #c8a96e);
    }

    .home-slot { background: #1a1a1a; }
    .home-slot.active,
    .home-slot:hover { color: var(--accent, #c8a96e); }

    .game-icon-img { width: 100%; height: 100%; object-fit: cover; }

    .game-icon-fallback {
        font-size: 12px;
        font-weight: 700;
        color: var(--accent, #c8a96e);
        display: flex;
        align-items: center;
        justify-content: center;
        width: 100%;
        height: 100%;
    }

    .divider {
        width: 32px;
        height: 1px;
        background: rgba(255,255,255,0.08);
        margin: 6px 0;
        flex-shrink: 0;
    }

    .games-list {
        flex: 1;
        width: 100%;
        overflow-y: auto;
        overflow-x: visible;
        display: flex;
        flex-direction: column;
        align-items: center;
        scrollbar-width: none;
        padding-bottom: 72px;
    }
    .games-list::-webkit-scrollbar { display: none; }

    .empty-hint {
        font-size: 10px;
        color: rgba(255,255,255,0.2);
        text-align: center;
        margin-top: 12px;
        line-height: 1.5;
    }

    /* ── User panel flottant ── */
    .user-panel {
        position: absolute;
        bottom: 8px;
        left: 8px;
        width: 260px;
        background: #1a1a1a;
        border: 1px solid rgba(255,255,255,0.08);
        border-radius: 10px;
        padding: 10px 12px;
        display: flex;
        align-items: center;
        gap: 10px;
        z-index: 100;
        box-shadow: 0 4px 24px rgba(0,0,0,0.5);
    }

    .avatar {
        width: 36px;
        height: 36px;
        border-radius: 50%;
        background: var(--accent, #c8a96e);
        flex-shrink: 0;
        display: flex;
        align-items: center;
        justify-content: center;
        font-size: 12px;
        font-weight: 700;
        color: #1a1400;
    }

    .user-info { flex: 1; min-width: 0; }

    .user-name {
        font-size: 13px;
        font-weight: 700;
        color: #fff;
        white-space: nowrap;
        overflow: hidden;
        text-overflow: ellipsis;
    }

    .user-meta {
        font-size: 11px;
        color: #6a7080;
        white-space: nowrap;
        margin-top: 1px;
    }

    .user-actions { display: flex; align-items: center; gap: 6px; flex-shrink: 0; }

    .watcher-dot { width: 8px; height: 8px; border-radius: 50%; background: #3ddc84; }

    .icon-btn {
        width: 24px;
        height: 24px;
        border-radius: 5px;
        background: transparent;
        border: none;
        cursor: pointer;
        color: rgba(255,255,255,0.35);
        display: flex;
        align-items: center;
        justify-content: center;
        transition: background 0.12s, color 0.12s;
    }
    .icon-btn:hover { background: rgba(255,255,255,0.08); color: #fff; }
    .icon-btn.active { color: var(--accent, #c8a96e); }
</style>