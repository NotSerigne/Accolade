<script lang="ts">
    import { games, selectedGameId, loadGames, totalUnlockedAchievements, type Game } from '$lib/stores/Games.js';
    import { settingsOpen, watcherActive } from '$lib/stores/ui.js';
    import { steamUser } from '$lib/stores/user.js';
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

    function goStats(): void {
        selectedGameId.set(null);
        settingsOpen.set(false);
        goto('/stats');
    }

    function goObjectives(): void {
        selectedGameId.set(null);
        settingsOpen.set(false);
        goto('/objectives');
    }

    function goJournal(): void {
        selectedGameId.set(null);
        settingsOpen.set(false);
        goto('/journal');
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

    function iconUrl(game: Game): string {
        // Priority 1: Steam Grid DB icon (SGDB)
        if (game.steamgrid_icon_url && game.steamgrid_icon_url.startsWith('http')) {
            return game.steamgrid_icon_url;
        }

        // Priority 2: game_icon if it's an HTTP URL (SGDB fallback)
        if (game.game_icon && game.game_icon.startsWith('http')) {
            return game.game_icon;
        }

        // Priority 3: game_icon if it's a valid hash (Steam client icon)
        if (game.game_icon && !game.game_icon.includes('/') && !game.game_icon.includes('\\')) {
            return `https://media.steampowered.com/steamcommunity/public/images/apps/${game.steam_id}/${game.game_icon}.ico`;
        }

        // Priority 4: header image as fallback
        if (game.header_image_url) return game.header_image_url;

        // Final Fallback: Steam API header image (Cloudflare)
        return `https://cdn.cloudflare.steamstatic.com/steam/apps/${game.steam_id}/header.jpg`;
    }

    function onIconError(name: string, steamId: number) {
        return (e: Event) => {
            const t = e.target as HTMLImageElement;
            const currentSrc = t.src;

            // Si on a déjà essayé le header et que ça a échoué, on passe aux initiales
            if (currentSrc.includes('header.jpg')) {
                t.onerror = null;
                t.src = `https://ui-avatars.com/api/?name=${encodeURIComponent(name || String(steamId))}&background=1e1e1e&color=c8a96e&size=52&bold=true&length=2`;
            } else {
                // Sinon on tente le header
                t.src = `https://cdn.cloudflare.steamstatic.com/steam/apps/${steamId}/header.jpg`;
            }
        };
    }

    let pathname = $derived(String(page.url.pathname));
    let isHomeActive = $derived(pathname === '/');
    let isStatsActive = $derived(pathname === '/stats');
    let isObjectivesActive = $derived(pathname === '/objectives');
    let isJournalActive = $derived(pathname === '/journal');

    let isSettingsActive = $derived($settingsOpen);
    let sortedGames = $derived.by(() => {
        return [...$games].sort((a, b) =>
            (a.name || String(a.steam_id)).localeCompare(
                b.name || String(b.steam_id),
                'fr',
                { sensitivity: 'base', numeric: true }
            )
        );
    });

</script>

<aside class="sidebar">
    <!-- Home -->
    <div class="nav-wrap">
        <div class="pill" class:visible={isHomeActive}></div>
        <button
                class="game-slot home-slot"
                class:active={isHomeActive}
                onclick={goHome}
                title="Accueil"
        >
            <svg width="20" height="20" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2.2">
                <path d="M3 9l9-7 9 7v11a2 2 0 0 1-2 2H5a2 2 0 0 1-2-2z"/>
                <polyline points="9 22 9 12 15 12 15 22"/>
            </svg>
        </button>
    </div>

    <div class="divider"></div>

    <div class="nav-wrap">
        <div class="pill" class:visible={isStatsActive}></div>
        <button
                class="game-slot nav-btn"
                class:active={isStatsActive}
                onclick={goStats}
                title="Statistiques"
        >
            <svg width="20" height="20" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2.2">
                <path d="M21.21 15.89A10 10 0 1 1 8 2.83" />
                <path d="M22 12A10 10 0 0 0 12 2v10z" />
            </svg>
        </button>
    </div>

    <div class="nav-wrap">
        <div class="pill" class:visible={isObjectivesActive}></div>
        <button
                class="game-slot nav-btn"
                class:active={isObjectivesActive}
                onclick={goObjectives}
                title="Objectifs"
        >
            <svg width="20" height="20" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2.2">
                <circle cx="12" cy="12" r="10" />
                <path d="M12 8v8M8 12h8" />
            </svg>
        </button>
    </div>

    <div class="nav-wrap">
        <div class="pill" class:visible={isJournalActive}></div>
        <button
                class="game-slot nav-btn"
                class:active={isJournalActive}
                onclick={goJournal}
                title="Journal"
        >
            <svg width="20" height="20" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2.2">
                <path d="M4 19.5A2.5 2.5 0 0 1 6.5 17H20" />
                <path d="M6.5 2H20v20H6.5A2.5 2.5 0 0 1 4 19.5v-15A2.5 2.5 0 0 1 6.5 2z" />
            </svg>
        </button>
    </div>

    <div class="divider"></div>

    <!-- Icônes des jeux détectés -->
    <div class="games-list">

        {#each sortedGames as game (game.steam_id)}
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
                                onerror={onIconError(game.name, game.steam_id)}
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

        {#if sortedGames.length === 0}
            <div class="empty-hint">Aucun<br/>jeu</div>
        {/if}
    </div>

    <!-- User panel flottant style Discord -->
    <div class="user-panel">
        {#if $steamUser}
            <img class="avatar" src={$steamUser.avatarfull} alt={$steamUser.personaname} />
            <div class="user-info">
                <div class="user-name">{$steamUser.personaname}</div>
                <div class="user-meta">{$totalUnlockedAchievements} succès · 0 platines</div>
            </div>
        {:else}
            <div class="avatar">?</div>
            <div class="user-info">
                <div class="user-name">Non connecté</div>
                <div class="user-meta">Configurez votre Steam ID</div>
            </div>
        {/if}
        <div class="user-actions">
            <div class="watcher-dot" class:active={$watcherActive} title={$watcherActive ? "Watcher actif" : "Watcher inactif"}></div>
            <button
                    class="icon-btn"
                    class:active={isSettingsActive}
                    onclick={goSettings}
                    title="Paramètres"
            >
                <svg width="15.75" height="15.75" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
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
        height: 100%;
        background: #0e0e0e;
        border-radius: 0;
        display: flex;
        flex-direction: column;
        align-items: center;
        padding: 12px 0 0;
        position: relative;
        overflow: visible;
        z-index: 1000;
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
        border-radius: 12px;
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

    .nav-btn { background: #1a1a1a; }
    .nav-btn.active,
    .nav-btn:hover { color: var(--accent, #c8a96e); }
    .nav-btn[title="Journal"].active,
    .nav-btn[title="Journal"]:hover { color: var(--accent, #c8a96e); }

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
        bottom: 20px;
        left: 16px;
        width: 320px;
        background: #1a1a1a;
        border: 1px solid rgba(255,255,255,0.08);
        border-radius: 16px;
        padding: 16px 20px;
        display: flex;
        align-items: center;
        gap: 16px;
        z-index: 100;
        box-shadow: 0 20px 50px rgba(0,0,0,0.7);
        backdrop-filter: blur(12px);
    }

    .avatar {
        width: 52px;
        height: 52px;
        border-radius: 50%;
        background: var(--accent, #c8a96e);
        flex-shrink: 0;
        display: flex;
        align-items: center;
        justify-content: center;
        font-size: 16px;
        font-weight: 700;
        color: #1a1400;
        object-fit: cover;
    }

    .user-info { flex: 1; min-width: 0; }

    .user-name {
        font-size: 16px;
        font-weight: 700;
        color: #fff;
        white-space: nowrap;
        overflow: hidden;
        text-overflow: ellipsis;
    }

    .user-meta {
        font-size: 13px;
        color: #6a7080;
        white-space: nowrap;
        margin-top: 2px;
    }

    .user-actions { display: flex; align-items: center; gap: 10px; flex-shrink: 0; }

    .watcher-dot { width: 10px; height: 10px; border-radius: 50%; background: #6a7080; transition: background 0.3s; }
    .watcher-dot.active { background: #3ddc84; }

    .icon-btn {
        width: 32px;
        height: 32px;
        border-radius: 8px;
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
