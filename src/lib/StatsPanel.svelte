<script>
    const stats = [
        { label: 'Succès débloqués', value: '247' },
        { label: 'Progression moyenne', value: '62%' },
        { label: 'Rareté moyenne', value: '24%' },
    ];
</script>

<aside class="stats-panel">
    <section class="stats-section">
        <h2 class="section-title">Stats</h2>
        {#each stats as stat}
            <div class="stat-card">
                <div class="stat-label">{stat.label}</div>
                <div class="stat-value">{stat.value}</div>
            </div>
        {/each}
    </section>
</aside>

<style>
    .stats-panel {
        grid-column: 3 / 4;
        grid-row: 2 / 3;
        background: #161616;
        border-radius: 12px;
        padding: 32px 20px;
        overflow-y: auto;
    }

    .stats-section {
        display: flex;
        flex-direction: column;
        gap: 12px;
    }

    .section-title {
        font-size: 22px;
        font-weight: 700;
        margin-bottom: 24px;
    }

    .stat-card {
        background: #1e1e1e;
        padding: 20px;
        border-radius: 8px;
        display: flex;
        flex-direction: column;
        gap: 6px;
    }

    .stat-label {
        font-size: 11px;
        color: #6a7080;
        text-transform: uppercase;
        letter-spacing: 1px;
    }

    .stat-value {
        font-size: 28px;
        font-weight: 700;
        color: #c8a96e;
    }
</style>