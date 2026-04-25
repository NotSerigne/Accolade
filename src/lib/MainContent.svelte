<script lang="ts">
    import { games } from '$lib/stores/Games.js';
    import { goto } from '$app/navigation';

    function rarityAccent(pct: string): string {
        const n = parseFloat(pct);
        if (isNaN(n)) return '#6a7080';
        if (n <= 0.1) return '#ff3b5c';
        if (n <= 1) return '#ffd85a';
        if (n <= 3) return '#a855f7';
        if (n <= 7) return '#f4b860';
        if (n <= 15) return '#4ac8ff';
        if (n <= 35) return '#3ddc84';
        return '#6a7080';
    }

    function progressPct(game: any): number {
        const total = game.achievements_total || game.achievements?.length || 0;
        if (total === 0) return 0;
        const unlocked = game.achievements?.filter((a: any) => a.unlocked).length ?? 0;
        return Math.round((unlocked / total) * 100);
    }

    let recentActivity = $derived.by(() => {
        return $games
            .flatMap((g: any) =>
                (g.achievements ?? [])
                    .filter((a: any) => a.unlocked && a.unlocked_time)
                    .map((a: any) => ({ ...a, gameName: g.name }))
            )
            .sort((a: any, b: any) => (b.unlocked_time ?? 0) - (a.unlocked_time ?? 0))
            .slice(0, 6);
    });

    let recentGames = $derived($games.slice(0, 4));
</script>

<main class="main-content">
    <section class="activity-section">
        <h2 class="section-title">Activité récente</h2>
        {#each recentActivity as item}
            {@const accent = rarityAccent(item.completionpercentage)}
            <div class="activity-item" style:border-left-color={accent}>
                <div class="activity-achievement">{item.name}</div>
                <div class="activity-game">{item.gameName}</div>
                <div class="activity-meta">
                    <span class="activity-rarity" style:color={accent}>{parseFloat(item.completionpercentage).toFixed(1)}%</span>
                    <span class="activity-time">{new Date((item.unlocked_time ?? 0) * 1000).toLocaleDateString('fr-FR')}</span>
                </div>
            </div>
        {/each}

        {#if recentActivity.length === 0}
            <div class="empty-hint">Aucune activité récente</div>
        {/if}
    </section>

    <section class="activity-section" style="margin-top: 28px;">
        <h2 class="section-title">Bibliothèque récente</h2>
        <div class="game-grid">
            {#each recentGames as game}
                <button class="game-card" onclick={() => goto(`/games/${game.steam_id}`)}>
                    <div class="game-card-bg">
                        {#if game.header_image_url}
                            <img src={game.header_image_url} alt={game.name} />
                        {:else}
                            <span class="game-placeholder">🎮</span>
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
        grid-column: 2 / 3;
        grid-row: 2 / 3;
        background: #161616;
        border-radius: 12px;
        padding: 32px;
        overflow-y: auto;
    }

    .activity-section {
        display: flex;
        flex-direction: column;
    }

    .section-title {
        font-size: 22px;
        font-weight: 700;
        margin-bottom: 24px;
    }

    .activity-item {
        padding: 24px 0 24px 20px;
        border-bottom: 1px solid #1e1e1e;
        border-left: 3px solid transparent;
        display: flex;
        flex-direction: column;
        gap: 6px;
    }

    .activity-item:last-child { border-bottom: none; }

    .activity-achievement {
        font-size: 16px;
        font-weight: 600;
        color: #ffffff;
    }

    .activity-game {
        font-size: 13px;
        color: #6a7080;
    }

    .activity-meta {
        margin-top: 8px;
        display: flex;
        justify-content: space-between;
        font-size: 12px;
        color: #6a7080;
    }

    .activity-rarity { color: var(--accent, #c8a96e); }


    .empty-hint {
        font-size: 13px;
        color: #444;
        padding: 16px 0;
    }

    .game-grid {
        display: grid;
        grid-template-columns: repeat(auto-fill, minmax(150px, 1fr));
        gap: 8px;
    }

    .game-card {
        position: relative;
        height: 110px;
        border-radius: 8px;
        overflow: hidden;
        border: 0.5px solid #1e1e1e;
        cursor: pointer;
        background: none;
        padding: 0;
        transition: border-color 0.15s;
    }
    .game-card:hover { border-color: #c8a96e44; }

    .game-card-bg {
        position: absolute;
        inset: 0;
        display: flex;
        align-items: center;
        justify-content: center;
        background: #1a1a1a;
    }
    .game-card-bg img { width: 100%; height: 100%; object-fit: cover; }
    .game-placeholder { font-size: 28px; }

    .game-card-overlay {
        position: absolute;
        inset: 0;
        background: linear-gradient(to top, #0d0d0d 0%, #0d0d0d55 60%, transparent 100%);
    }

    .game-card-info {
        position: absolute;
        bottom: 0; left: 0; right: 0;
        padding: 8px 10px;
    }

    .game-card-title {
        font-size: 11px;
        font-weight: 500;
        color: #e0e0e0;
        white-space: nowrap;
        overflow: hidden;
        text-overflow: ellipsis;
        margin-bottom: 5px;
    }

    .game-card-bar {
        height: 2px;
        background: #2a2a2a;
        border-radius: 1px;
        overflow: hidden;
        margin-bottom: 3px;
    }

    .game-card-bar-fill {
        height: 100%;
        background: #4caf6e;
        border-radius: 1px;
    }

    .game-card-pct { font-size: 10px; color: #666; }
    .game-card-pct.complete { color: #4caf6e; }

    ::-webkit-scrollbar { width: 4px; }
    ::-webkit-scrollbar-track { background: transparent; }
    ::-webkit-scrollbar-thumb { background: #2a2a2a; border-radius: 2px; }
    ::-webkit-scrollbar-thumb:hover { background: #3a3a3a; }
</style>