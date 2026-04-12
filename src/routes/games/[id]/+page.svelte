<script>
    import Sidebar from '$lib/Sidebar.svelte';
    import Topbar from '$lib/Topbar.svelte';
    import GamePage from '$lib/GamePage.svelte';
    import { page } from '$app/state';
    import { games } from '$lib/stores/Games.js';

    // page depuis $app/state = rune Svelte 5, pas un store → pas de $ devant
    // $games = store Svelte classique → $ obligatoire
    let game = $derived($games.find(g => String(g.steam_id) === page.params.id) ?? null);
</script>

<div class="app-container">
    <Sidebar />
    <Topbar />
    <main class="main-slot">
        <GamePage {game} />
    </main>
</div>

<style>
    .app-container {
        display: grid;
        grid-template-columns: 72px 1fr;
        grid-template-rows: 56px 1fr;
        height: 100vh;
        gap: 8px;
        padding: 8px;
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