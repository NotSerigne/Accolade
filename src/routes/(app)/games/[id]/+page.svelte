<script>
	// src/routes/(app)/games/[id]/+page.svelte
	import GamePage from '$lib/GamePage.svelte';
	import { page } from '$app/state';
	import { games } from '$lib/stores/Games.js';

	let game = $derived($games.find((g) => String(g.steam_id) === page.params.id) ?? null);
</script>

<main class="game-main">
	<GamePage {game} />
</main>

<style>
	.game-main {
		background: var(--bg-panel);
		border-radius: 12px;
		overflow: hidden;
		display: flex;
		flex-direction: column;
		height: 100%;
	}
</style>
