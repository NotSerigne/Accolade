<script>
    import Sidebar from '$lib/Sidebar.svelte';
    import Topbar from '$lib/Topbar.svelte';
    import MainContent from '$lib/MainContent.svelte';
    import StatsPanel from '$lib/StatsPanel.svelte';
</script>

<div class="app-container">
    <Sidebar />
    <Topbar />
    <MainContent />
    <StatsPanel />
</div>

<style>
    .app-container {
        display: grid;
        grid-template-columns: 72px 1fr 280px;
        grid-template-rows: 56px 1fr;
        height: 100vh;
        gap: 8px;
        padding: 8px;
    }
</style>