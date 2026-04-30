<script lang="ts">
    import { games, totalUnlockedAchievements, type Game } from '$lib/stores/Games.js';

    type RarityBreakdown = {
        mythic: number;
        legendary: number;
        epic: number;
        veryRare: number;
        rare: number;
        uncommon: number;
        common: number;
    };

    function getGameIcon(game: Game): string {
        // Priority 1: Steam Grid DB icon (SGDB)
        if (game.steamgrid_icon_url && game.steamgrid_icon_url.startsWith('http')) {
            return game.steamgrid_icon_url;
        }

        // Priority 2: game_icon if it's an HTTP URL (SGDB fallback)
        if (game.game_icon && game.game_icon.startsWith('http')) {
            return game.game_icon;
        }

        // Priority 3: header image
        if (game.header_image_url) return game.header_image_url;

        // Final Fallback: Steam API header image (Cloudflare)
        return `https://cdn.cloudflare.steamstatic.com/steam/apps/${game.steam_id}/header.jpg`;
    }

    let totalAchievements = $derived(
        $games.reduce((acc, g) => acc + (g.achievements_total || g.achievements?.length || 0), 0)
    );

    let avgProgression = $derived.by(() => {
        if ($games.length === 0) return 0;
        const sum = $games.reduce((acc, g) => {
            const total = g.achievements_total || g.achievements?.length || 0;
            if (total === 0) return acc;
            const unlocked = g.achievements?.filter(a => a.unlocked).length ?? 0;
            return acc + Math.round((unlocked / total) * 100);
        }, 0);
        return Math.round(sum / $games.length);
    });

    let rarityBreakdown = $derived.by(() => {
        const all = $games.flatMap(g => g.achievements ?? []).filter(a => a.unlocked);
        const breakdown: RarityBreakdown = {
            mythic: 0,
            legendary: 0,
            epic: 0,
            veryRare: 0,
            rare: 0,
            uncommon: 0,
            common: 0,
        };

        for (const a of all) {
            const n = parseFloat(a.completionpercentage);
            if (isNaN(n)) {
                breakdown.common += 1;
            } else if (n <= 0.1) {
                breakdown.mythic += 1;
            } else if (n <= 1) {
                breakdown.legendary += 1;
            } else if (n <= 3) {
                breakdown.epic += 1;
            } else if (n <= 7) {
                breakdown.veryRare += 1;
            } else if (n <= 15) {
                breakdown.rare += 1;
            } else if (n <= 35) {
                breakdown.uncommon += 1;
            } else {
                breakdown.common += 1;
            }
        }

        return breakdown;
    });

    let highRarityPct = $derived.by(() => {
        if ($totalUnlockedAchievements === 0) return 0;
        const highRarity =
            rarityBreakdown.mythic +
            rarityBreakdown.legendary +
            rarityBreakdown.epic +
            rarityBreakdown.veryRare;
        return Math.round((highRarity / $totalUnlockedAchievements) * 100);
    });

    let recentlyCompleted = $derived.by(() => {
        return $games
            .filter(g => {
                const total = g.achievements_total || g.achievements?.length || 0;
                const unlocked = g.achievements?.filter(a => a.unlocked).length ?? 0;
                return total > 0 && unlocked === total;
            })
            .sort((a, b) => {
                const latestA = Math.max(...(a.achievements?.map(a => a.unlocked_time ?? 0) ?? [0]));
                const latestB = Math.max(...(b.achievements?.map(a => a.unlocked_time ?? 0) ?? [0]));
                return latestB - latestA;
            })
            .slice(0, 3);
    });
</script>

<div class="stats-panel">

    <h2 class="section-title">Stats</h2>

    <div class="stat-card">
        <span class="stat-label">Succès débloqués</span>
        <span class="stat-value">{$totalUnlockedAchievements}</span>
        <span class="stat-sub">sur {totalAchievements} total</span>
    </div>

    <div class="stat-card">
        <span class="stat-label">Progression moy.</span>
        <span class="stat-value">{avgProgression}%</span>
    </div>

    <div class="stat-card">
        <span class="stat-label">Rareté moyenne</span>
        <span class="stat-value">{highRarityPct}%</span>
    </div>

    <h2 class="section-title">Répartition Rareté</h2>

    <ul class="rarity-list">
        <li><span class="dot mythic"></span><span class="rarity-label">Mythique</span><span class="rarity-count">{rarityBreakdown.mythic}</span></li>
        <li><span class="dot legendary"></span><span class="rarity-label">Légendaire</span><span class="rarity-count">{rarityBreakdown.legendary}</span></li>
        <li><span class="dot epic"></span><span class="rarity-label">Épique</span><span class="rarity-count">{rarityBreakdown.epic}</span></li>
        <li><span class="dot very-rare"></span><span class="rarity-label">Très rare</span><span class="rarity-count">{rarityBreakdown.veryRare}</span></li>
        <li><span class="dot rare"></span><span class="rarity-label">Rare</span><span class="rarity-count">{rarityBreakdown.rare}</span></li>
        <li><span class="dot uncommon"></span><span class="rarity-label">Peu commun</span><span class="rarity-count">{rarityBreakdown.uncommon}</span></li>
        <li><span class="dot common"></span><span class="rarity-label">Commun</span><span class="rarity-count">{rarityBreakdown.common}</span></li>
    </ul>

    <h2 class="section-title">Complétés récents</h2>

    <ul class="completed-list">
        {#each recentlyCompleted as game}
            <li class="completed-item">
                <img
                    src={getGameIcon(game)}
                    alt={game.name}
                    class="game-icon"
                    onerror={(e) => {
                        const t = e.target as HTMLImageElement;
                        if (!t.src.includes('header.jpg')) {
                            t.src = `https://cdn.cloudflare.steamstatic.com/steam/apps/${game.steam_id}/header.jpg`;
                        }
                    }}
                />
                <div class="completed-info">
                    <span class="completed-name">{game.name}</span>
                    <span class="completed-sub">
                    100% · {game.achievements?.length}/{game.achievements?.length}
                </span>
                </div>
            </li>
        {/each}
    </ul>

</div>

<style>
    .stats-panel {
        background: #161616;
        border-radius: 12px;
        padding: 24px;
        overflow-y: auto;
        display: flex;
        flex-direction: column;
        gap: 8px;
        height: 100%;
    }

    .section-title {
        font-size: 11px;
        font-weight: 600;
        letter-spacing: 0.08em;
        text-transform: uppercase;
        color: #6a7080;
        margin-top: 16px;
        margin-bottom: 4px;
    }

    .stat-card {
        background: #1a1a1a;
        border-radius: 8px;
        border: 0.5px solid #1e1e1e;
        padding: 16px;
        display: flex;
        flex-direction: column;
        gap: 4px;
    }

    .stat-label {
        font-size: 11px;
        font-weight: 600;
        letter-spacing: 0.08em;
        text-transform: uppercase;
        color: #6a7080;
    }

    .stat-value {
        font-size: 32px;
        font-weight: 700;
        color: var(--accent, #c8a96e);
        line-height: 1;
    }

    .stat-sub {
        font-size: 12px;
        color: #6a7080;
        margin-top: 2px;
    }

    /* Rarity breakdown */
    .rarity-list {
        list-style: none;
        padding: 0;
        margin: 0;
        display: flex;
        flex-direction: column;
        gap: 10px;
    }

    .rarity-list li {
        display: flex;
        align-items: center;
        gap: 10px;
        font-size: 13px;
    }

    .dot {
        width: 8px;
        height: 8px;
        border-radius: 50%;
        flex-shrink: 0;
    }

    .dot.mythic    { background: #ff3b5c; }
    .dot.legendary { background: #ffd85a; }
    .dot.epic      { background: #a855f7; }
    .dot.very-rare { background: #f4b860; }
    .dot.rare      { background: #4ac8ff; }
    .dot.uncommon  { background: #3ddc84; }
    .dot.common    { background: #6a7080; }

    .rarity-label { flex: 1; color: #b0b8c8; }
    .rarity-count { color: #ffffff; font-weight: 600; }

    /* Completed list */
    .completed-list {
        list-style: none;
        padding: 0;
        margin: 0;
        display: flex;
        flex-direction: column;
        gap: 2px;
    }

    .completed-item {
        display: flex;
        align-items: center;
        gap: 12px;
        padding: 10px 0;
        border-bottom: 1px solid #1e1e1e;
    }

    .completed-item:last-child { border-bottom: none; }

    .game-icon {
        width: 36px;
        height: 36px;
        border-radius: 6px;
        object-fit: cover;
        background: #1a1a1a;
        flex-shrink: 0;
    }

    .completed-info {
        display: flex;
        flex-direction: column;
        gap: 2px;
        overflow: hidden;
    }

    .completed-name {
        font-size: 13px;
        font-weight: 600;
        color: #e0e0e0;
        white-space: nowrap;
        overflow: hidden;
        text-overflow: ellipsis;
    }

    .completed-sub {
        font-size: 11px;
        color: var(--accent, #4caf6e);
    }

    ::-webkit-scrollbar { width: 4px; }
    ::-webkit-scrollbar-track { background: transparent; }
    ::-webkit-scrollbar-thumb { background: #2a2a2a; border-radius: 2px; }
    ::-webkit-scrollbar-thumb:hover { background: #3a3a3a; }

</style>
