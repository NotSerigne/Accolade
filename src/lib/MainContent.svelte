<script>
    const activities = [
        {
            achievement: 'Guerrier légendaire',
            game: 'Elden Ring',
            rarity: 'Légendaire',
            percent: 2,
            time: 'Il y a 2h',
            tier: 'epic',
        },
        {
            achievement: 'Vainqueur de boss',
            game: 'Hades',
            rarity: 'Rare',
            percent: 12,
            time: 'Il y a 5h',
            tier: 'rare',
        },
        {
            achievement: 'Premier pas',
            game: 'Stardew Valley',
            rarity: 'Commun',
            percent: 45,
            time: 'Il y a 1 jour',
            tier: 'common',
        },
    ];
</script>

<main class="main-content">
    <section class="activity-section">
        <h2 class="section-title">Activité récente</h2>
        {#each activities as item}
            <div class="activity-item rarity-{item.tier}">
                <div class="activity-achievement">{item.achievement}</div>
                <div class="activity-game">{item.game}</div>
                <div class="activity-meta">
                    <span class="activity-rarity">{item.rarity} ({item.percent}%)</span>
                    <span class="activity-time">{item.time}</span>
                </div>
            </div>
        {/each}
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

    .rarity-epic   { border-left: 3px solid var(--accent, #c8a96e); }
    .rarity-rare   { border-left: 3px solid #3ddc84; }
    .rarity-common { border-left: 3px solid #2e3240; }
</style>