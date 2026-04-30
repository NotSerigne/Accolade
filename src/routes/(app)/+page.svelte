<script lang="ts">
    import Sidebar from '$lib/Sidebar.svelte';
    import Topbar from '$lib/Topbar.svelte';
    import MainContent from '$lib/MainContent.svelte';
    import StatsPanel from '$lib/StatsPanel.svelte';
</script>

<div class="app-container">
    <Sidebar />
    <Topbar />
    <main class="main-slot">
        <MainContent />
    </main>
    <StatsPanel />
</div>

<style>
    .app-container {
        display: grid;
        grid-template-columns: 72px minmax(0, 1fr) 320px;
        grid-template-rows: 56px 1fr;
        height: 100vh;
        gap: 8px;
        padding: 8px;
        overflow: hidden;
    }

    .main-slot {
        grid-column: 2 / 3;
        grid-row: 2 / 3;
        background: #161616;
        border-radius: 12px;
        overflow: hidden;
        display: flex;
        flex-direction: column;
    }
</style>
