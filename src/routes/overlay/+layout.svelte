<script lang="ts">
    // src/routes/overlay/+layout.svelte
    let { children } = $props();
</script>

{@render children()}

<style>
    :global(html),
    :global(body),
    :global(body > div) {
        margin: 0;
        padding: 0;
        background: transparent !important;
        width: 100%;
        height: 100%;
        overflow: visible;
    }
</style>
