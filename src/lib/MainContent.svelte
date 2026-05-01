<script lang="ts">
    // src/lib/MainContent.svelte
    import { games, type Game, type Achievement } from '$lib/stores/Games.js';
    import { goto } from '$app/navigation';
    import { resolve } from '$app/paths';

    function rarityLabel(pct: string): string {
        const n = parseFloat(pct);
        if (isNaN(n)) return 'Commun';
        if (n <= 0.1) return 'Mythic';
        if (n <= 1) return 'Légendaire';
        if (n <= 3) return 'Épique';
        if (n <= 7) return 'Très rare';
        if (n <= 15) return 'Rare';
        if (n <= 35) return 'Peu commun';
        return 'Commun';
    }

    function rarityPalette(pct: string) {
        const n = parseFloat(pct);
        if (isNaN(n)) return { accent: '#6a7080', bg: 'rgba(106, 112, 128, 0.1)' };
        if (n <= 0.1) return { accent: '#ff3b5c', bg: 'rgba(255, 59, 92, 0.15)' };
        if (n <= 1) return { accent: '#ffd85a', bg: 'rgba(255, 216, 90, 0.15)' };
        if (n <= 3) return { accent: '#a855f7', bg: 'rgba(168, 85, 247, 0.15)' };
        if (n <= 7) return { accent: '#f4b860', bg: 'rgba(244, 184, 96, 0.15)' };
        if (n <= 15) return { accent: '#4ac8ff', bg: 'rgba(74, 200, 255, 0.15)' };
        if (n <= 35) return { accent: '#3ddc84', bg: 'rgba(61, 220, 132, 0.15)' };
        return { accent: '#6a7080', bg: 'rgba(106, 112, 128, 0.1)' };
    }

    function progressPct(game: Game): number {
        const total = game.achievements_total || game.achievements?.length || 0;
        if (total === 0) return 0;
        const unlocked = game.achievements?.filter((a) => a.unlocked).length ?? 0;
        return Math.round((unlocked / total) * 100);
    }

    function formatDateTime(ts: number | null | undefined): string {
        if (!ts) return '';
        return new Date(ts * 1000).toLocaleString('fr-FR', {
            day: '2-digit',
            month: '2-digit',
            year: 'numeric',
            hour: '2-digit',
            minute: '2-digit',
            hour12: false,
        });
    }

    type RecentAchievement = Achievement & { gameName: string; gameId: number };

    let recentActivity = $derived.by((): RecentAchievement[] => {
        return $games
            .flatMap((g) =>
                (g.achievements ?? [])
                    .filter((a) => a.unlocked && a.unlocked_time)
                    .map((a) => ({ ...a, gameName: g.name, gameId: g.steam_id }))
            )
            .sort((a, b) => (b.unlocked_time ?? 0) - (a.unlocked_time ?? 0))
            .slice(0, 6);
    });

    let recentGames = $derived.by((): Game[] => {
        return [...$games]
            .filter((g) => (g.achievements ?? []).some((a) => a.unlocked || a.unlocked_time))
            .sort((a, b) => {
                const lastUnlockA = Math.max(...(a.achievements ?? []).map((x) => x.unlocked_time ?? 0), 0);
                const lastUnlockB = Math.max(...(b.achievements ?? []).map((x) => x.unlocked_time ?? 0), 0);
                return lastUnlockB - lastUnlockA;
            })
            .slice(0, 12);
    });
</script>

<main class="main-content">
    <section class="activity-section">
        <h2 class="section-title">Activité récente</h2>
        <div class="activity-grid">
            {#each recentActivity as item (item.gameId + ':' + item.key)}
                {@const palette = rarityPalette(item.completionpercentage)}
                <button class="activity-card" onclick={() => void goto(resolve('/(app)/games/[id]', { id: String(item.gameId) }))}>
                    <div class="activity-icon-wrap">
                        {#if item.icon}
                            <img
                                src={item.icon}
                                alt={item.name}
                                class="activity-icon"
                                onerror={(e) => {
                                    const t = e.target as HTMLImageElement;
                                    if (!t.src.includes('header.jpg')) {
                                        t.src = `https://cdn.cloudflare.steamstatic.com/steam/apps/${item.gameId}/header.jpg`;
                                    }
                                }}
                            />
                        {:else}
                            <img src="https://cdn.cloudflare.steamstatic.com/steam/apps/{item.gameId}/header.jpg" alt={item.name} class="activity-icon" />
                        {/if}
                    </div>
                    <div class="activity-info">
                        <div class="activity-header">
                            <span class="activity-name">{item.name}</span>
                        </div>
                        <div class="activity-game">{item.gameName}</div>
                        <div class="activity-footer">
                            <span class="activity-rarity-badge" style:color={palette.accent} style:background={palette.bg}>
                                {rarityLabel(item.completionpercentage)} · {parseFloat(item.completionpercentage).toFixed(1)}%
                            </span>
                            <span class="activity-time">{formatDateTime(item.unlocked_time)}</span>
                        </div>
                    </div>
                </button>
            {/each}
        </div>

        {#if recentActivity.length === 0}
            <div class="empty-hint">Aucune activité récente</div>
        {/if}
    </section>

    <section class="activity-section" style="margin-top: 32px;">
        <h2 class="section-title">Bibliothèque récente</h2>
        <div class="game-grid">
            {#each recentGames as game (game.steam_id)}
                <button class="game-card" onclick={() => void goto(resolve('/(app)/games/[id]', { id: String(game.steam_id) }))}>
                    <div class="game-card-bg">
                        {#if game.header_image_url}
                            <img
                                src={game.header_image_url}
                                alt={game.name}
                                onerror={(e) => {
                                    const t = e.target as HTMLImageElement;
                                    if (!t.src.includes('header.jpg')) {
                                        t.src = `https://cdn.cloudflare.steamstatic.com/steam/apps/${game.steam_id}/header.jpg`;
                                    }
                                }}
                            />
                        {:else}
                            <img src="https://cdn.cloudflare.steamstatic.com/steam/apps/{game.steam_id}/header.jpg" alt={game.name} />
                        {/if}
                    </div>
                    <div class="game-card-overlay"></div>
                    <div class="game-card-info">
                        <div class="game-card-title">{game.name || game.steam_id}</div>
                        <div class="game-card-bar">
                            <div class="game-card-bar-fill" style="width: {progressPct(game)}%"></div>
                        </div>
                        <div class="game-card-pct" class:complete={progressPct(game) === 100}>
                            {progressPct(game)}%
                        </div>
                    </div>
                </button>
            {/each}
        </div>
    </section>
</main>

<style>
    .main-content {
        background: #161616;
        border-radius: 12px;
        padding: 32px;
        overflow-y: auto;
        height: 100%;
        display: flex;
        flex-direction: column;
    }

    .activity-section {
        display: flex;
        flex-direction: column;
    }

    .activity-section:last-child {
        flex: 1;
    }

    .section-title {
        font-size: 20px;
        font-weight: 800;
        margin-bottom: 20px;
        letter-spacing: -0.5px;
        color: #fff;
    }

    .activity-grid {
        display: grid;
        grid-template-columns: repeat(auto-fill, minmax(340px, 1fr));
        gap: 12px;
    }

    .activity-card {
        display: flex;
        gap: 14px;
        padding: 12px;
        background: rgba(255, 255, 255, 0.03);
        border: 1px solid rgba(255, 255, 255, 0.05);
        border-radius: 12px;
        cursor: pointer;
        transition: all 0.2s cubic-bezier(0.4, 0, 0.2, 1);
        text-align: left;
        width: 100%;
        font-family: inherit;
    }

    .activity-card:hover {
        background: rgba(255, 255, 255, 0.06);
        border-color: rgba(255, 255, 255, 0.1);
        transform: translateY(-2px);
    }

    .activity-icon-wrap {
        flex-shrink: 0;
    }

    .activity-icon {
        width: 56px;
        height: 56px;
        border-radius: 8px;
        object-fit: cover;
        background: #1a1a1a;
    }

    .activity-info {
        flex: 1;
        min-width: 0;
        display: flex;
        flex-direction: column;
        justify-content: space-between;
    }

    .activity-header {
        display: flex;
        justify-content: space-between;
        align-items: flex-start;
        margin-bottom: 2px;
    }

    .activity-name {
        font-size: 14px;
        font-weight: 700;
        color: #fff;
        white-space: nowrap;
        overflow: hidden;
        text-overflow: ellipsis;
    }

    .activity-game {
        font-size: 12px;
        font-weight: 500;
        color: #6a7080;
        white-space: nowrap;
        overflow: hidden;
        text-overflow: ellipsis;
        margin-bottom: 6px;
    }

    .activity-footer {
        display: flex;
        justify-content: space-between;
        align-items: center;
    }

    .activity-rarity-badge {
        font-size: 10px;
        font-weight: 800;
        padding: 2px 8px;
        border-radius: 4px;
        text-transform: uppercase;
        letter-spacing: 0.5px;
    }

    .activity-time {
        font-size: 10px;
        font-weight: 600;
        color: #6a7080;
    }

    .empty-hint {
        font-size: 13px;
        color: #444;
        padding: 16px 0;
    }

    .game-grid {
        display: grid;
        grid-template-columns: repeat(auto-fill, minmax(200px, 1fr));
        gap: 12px;
    }

    .game-card {
        position: relative;
        height: 120px;
        border-radius: 10px;
        overflow: hidden;
        border: 1px solid rgba(255, 255, 255, 0.05);
        cursor: pointer;
        background: #1a1a1a;
        padding: 0;
        transition: all 0.2s;
    }

    .game-card:hover {
        border-color: var(--accent, #c8a96e);
        transform: scale(1.02);
    }

    .game-card-bg {
        position: absolute;
        inset: 0;
        display: flex;
        align-items: center;
        justify-content: center;
    }
    .game-card-bg img { width: 100%; height: 100%; object-fit: cover; }

    .game-card-overlay {
        position: absolute;
        inset: 0;
        background: linear-gradient(to top, #0d0d0d 0%, #0d0d0d55 60%, transparent 100%);
    }

    .game-card-info {
        position: absolute;
        bottom: 0; left: 0; right: 0;
        padding: 10px;
    }

    .game-card-title {
        font-size: 11px;
        font-weight: 700;
        color: #fff;
        white-space: nowrap;
        overflow: hidden;
        text-overflow: ellipsis;
        margin-bottom: 6px;
    }

    .game-card-bar {
        height: 3px;
        background: rgba(255, 255, 255, 0.08);
        border-radius: 2px;
        overflow: hidden;
        margin-bottom: 4px;
    }

    .game-card-bar-fill {
        height: 100%;
        background: var(--accent, #c8a96e);
        border-radius: 2px;
    }

    .game-card-pct { font-size: 10px; font-weight: 800; color: #6a7080; }
    .game-card-pct.complete { color: var(--accent, #c8a96e); }

    ::-webkit-scrollbar { width: 4px; }
    ::-webkit-scrollbar-track { background: transparent; }
    ::-webkit-scrollbar-thumb { background: #2a2a2a; border-radius: 2px; }
    ::-webkit-scrollbar-thumb:hover { background: #3a3a3a; }
</style>
