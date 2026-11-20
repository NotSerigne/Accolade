<script lang="ts">
	// src/routes/(app)/games/[id]/+page.svelte
	import GamePage from '$lib/GamePage.svelte';
	import { page } from '$app/state';
	import { games } from '$lib/stores/Games.js';

	let game = $derived($games.find((g) => g.id === page.params.id) ?? null);
</script>

<main class="game-main">
	<GamePage {game} />
</main>

<style>
	.game-main {
		background: var(--bg-panel);
		border: 1px solid var(--border-soft);
		border-radius: 14px;
		box-shadow: var(--shadow-card);
		overflow: hidden;
		display: flex;
		flex-direction: column;
		height: 100%;
		min-width: 0;
	}
</style>
