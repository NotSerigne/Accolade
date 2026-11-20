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
		grid-template-columns: minmax(0, 1fr) 310px;
		gap: 10px;
		height: 100%;
		min-height: 0;
		overflow: hidden;
	}

	.main-slot {
		background: var(--bg-panel);
		border: 1px solid var(--border-soft);
		border-radius: 14px;
		box-shadow: var(--shadow-card);
		overflow: hidden;
		display: flex;
		flex-direction: column;
		min-width: 0;
		min-height: 0;
		height: 100%;
	}

	@media (max-width: 1020px) {
		.home-main {
			grid-template-columns: 1fr;
			grid-auto-rows: max-content;
			overflow-y: auto;
		}

		.main-slot {
			height: auto;
			overflow: visible;
		}

		.home-main :global(.main-content),
		.home-main :global(.stats-panel) {
			height: auto;
			overflow-y: visible;
		}
	}
</style>
