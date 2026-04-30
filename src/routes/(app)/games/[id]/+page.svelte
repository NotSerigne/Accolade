<script>
    import GamePage from '$lib/GamePage.svelte';
    import { page } from '$app/state';
    import { games } from '$lib/stores/Games.js';

    // page depuis $app/state = rune Svelte 5, pas un store → pas de $ devant
    // $games = store Svelte classique → $ obligatoire
    let game = $derived($games.find(g => String(g.steam_id) === page.params.id) ?? null);
</script>

<main class="game-main">
    <GamePage {game} />
</main>

<style>
    .game-main {
        background: #161616;
        border-radius: 12px;
        overflow: hidden;
        display: flex;
        flex-direction: column;
        height: 100%;
    }
</style>
