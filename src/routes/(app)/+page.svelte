<script lang="ts">
    // src/routes/(app)/+page.svelte
    import MainContent from '$lib/MainContent.svelte';
    import StatsPanel from '$lib/StatsPanel.svelte';
</script>

<main class="home-main">
    <div class="main-slot">
        <MainContent />
    </div>
    <StatsPanel />
</main>

<style>
    .home-main {
        display: grid;
        grid-template-columns: minmax(0, 1fr) 320px;
        gap: 8px;
        overflow: hidden;
        height: 100%;
    }

    .main-slot {
        background: var(--bg-panel);
        border-radius: 12px;
        overflow: hidden;
        display: flex;
        flex-direction: column;
        height: 100%;
    }
</style>
