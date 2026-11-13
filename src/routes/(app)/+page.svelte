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
		gap: 10px;
		overflow: hidden;
		height: 100%;
	}

	.main-slot {
		background: var(--bg-panel);
		border: 1px solid var(--border-soft);
		border-radius: 14px;
		overflow: hidden;
		display: flex;
		flex-direction: column;
		height: 100%;
		min-width: 0;
		box-shadow: var(--shadow-card);
	}
</style>
