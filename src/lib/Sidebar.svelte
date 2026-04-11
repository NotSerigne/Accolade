<script lang="ts">
    import { games, selectedGameId, loadGames, type Game } from '$lib/stores/Games.js';
    import { onMount } from 'svelte';
    import { goto } from '$app/navigation';
    import { page } from '$app/state';

    onMount(() => loadGames());

    function selectGame(id: number): void {
        selectedGameId.set(id);
        goto(`/games/${id}`);
    }

    function goHome(): void {
        selectedGameId.set(null);
        goto('/');
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
</script>

<aside class="sidebar">
    <!-- Home -->
    <div class="nav-wrap">
        <div class="pill" class:visible={page.url.pathname === '/'}></div>
        <button
                class="game-slot home-slot"
                class:active={page.url.pathname === '/'}
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

    <!-- Jeux -->
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
                    {#if game.game_icon}
                        <img
                                src="https://media.steampowered.com/steamcommunity/public/images/apps/{game.steam_id}/{game.game_icon}.jpg"
                                alt={game.name}
                                class="game-icon-img"
                                onerror={(e) => {
                                const target = e.target as HTMLImageElement;
                                target.style.display = 'none';
                                const fallback = target.nextElementSibling as HTMLElement | null;
                                if (fallback) fallback.style.display = 'flex';
                            }}
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

    <!-- User panel -->
    <div class="user-panel">
        <div class="avatar">SG</div>
        <div class="user-actions">
            <div class="watcher-dot" title="Watcher actif"></div>
            <button class="icon-btn" title="Paramètres">
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
        background: #0e0e0e;
        display: flex;
        flex-direction: column;
        align-items: center;
        padding: 12px 0 0;
        gap: 4px;
        overflow: hidden;
        border-radius: 12px;
    }

    .nav-wrap {
        position: relative;
        width: 100%;
        display: flex;
        align-items: center;
        justify-content: center;
    }

    .pill {
        position: absolute;
        left: 0;
        width: 3px;
        height: 0;
        background: #c8a96e;
        border-radius: 0 3px 3px 0;
        transition: height 0.18s cubic-bezier(.4,0,.2,1);
    }
    .pill.visible { height: 22px; }

    .game-slot {
        width: 52px;
        height: 52px;
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
        box-shadow: 0 0 0 2px rgba(200,169,110,0.3);
    }

    .game-slot.active {
        border-radius: 30%;
        box-shadow: 0 0 0 2px #c8a96e;
    }

    .home-slot { background: #1a1a1a; }
    .home-slot.active,
    .home-slot:hover { color: #c8a96e; }

    .game-icon-img { width: 100%; height: 100%; object-fit: cover; }

    .game-icon-fallback {
        font-size: 13px;
        font-weight: 700;
        color: #c8a96e;
        display: flex;
        align-items: center;
        justify-content: center;
    }

    .divider {
        width: 36px;
        height: 1px;
        background: rgba(255,255,255,0.08);
        margin: 4px 0;
        flex-shrink: 0;
    }

    .games-list {
        flex: 1;
        width: 100%;
        overflow-y: auto;
        overflow-x: hidden;
        display: flex;
        flex-direction: column;
        align-items: center;
        scrollbar-width: none;
        padding-bottom: 8px;
    }
    .games-list::-webkit-scrollbar { display: none; }

    .empty-hint {
        font-size: 10px;
        color: rgba(255,255,255,0.2);
        text-align: center;
        margin-top: 12px;
        line-height: 1.5;
    }

    .user-panel {
        width: 100%;
        background: #111;
        border-top: 1px solid rgba(255,255,255,0.06);
        padding: 10px;
        display: flex;
        align-items: center;
        justify-content: space-between;
        flex-shrink: 0;
    }

    .avatar {
        width: 34px;
        height: 34px;
        border-radius: 50%;
        background: #c8a96e;
        flex-shrink: 0;
        display: flex;
        align-items: center;
        justify-content: center;
        font-size: 11px;
        font-weight: 700;
        color: #1a1400;
    }

    .user-actions { display: flex; align-items: center; gap: 6px; }

    .watcher-dot { width: 8px; height: 8px; border-radius: 50%; background: #3ddc84; }

    .icon-btn {
        width: 24px;
        height: 24px;
        border-radius: 4px;
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
</style>