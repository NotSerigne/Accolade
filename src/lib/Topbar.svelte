<script>
    let searching = false;

    function openSearch() {
        searching = true;
    }
</script>

<header class="topbar">
    <button class="topbar-icon-btn" title="Accueil">
        <svg width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
            <path d="M3 9l9-7 9 7v11a2 2 0 0 1-2 2H5a2 2 0 0 1-2-2z"/>
            <polyline points="9 22 9 12 15 12 15 22"/>
        </svg>
    </button>

    <button class="topbar-search" onclick={openSearch}>
        <svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
            <circle cx="11" cy="11" r="8"/><line x1="21" y1="21" x2="16.65" y2="16.65"/>
        </svg>
        <span class="search-placeholder">What are you searching for?</span>
        <kbd class="search-shortcut">⌘K</kbd>
    </button>

    <div class="topbar-right">
        <span class="watching-badge">● Watching</span>
        <button class="topbar-icon-btn" title="Bibliothèque">
            <svg width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
                <path d="M4 19.5A2.5 2.5 0 0 1 6.5 17H20"/><path d="M6.5 2H20v20H6.5A2.5 2.5 0 0 1 4 19.5v-15A2.5 2.5 0 0 1 6.5 2z"/>
            </svg>
        </button>
    </div>
</header>

<style>
    .topbar {
        grid-column: 2 / -1;
        grid-row: 1 / 2;
        background: #161616;
        border-radius: 12px;
        padding: 0 16px;
        display: flex;
        align-items: center;
        gap: 10px;
    }

    .topbar-icon-btn {
        width: 32px;
        height: 32px;
        border-radius: 8px;
        border: 0.5px solid rgba(255, 255, 255, 0.1);
        background: rgba(255, 255, 255, 0.05);
        display: flex;
        align-items: center;
        justify-content: center;
        cursor: pointer;
        color: rgba(255, 255, 255, 0.55);
        flex-shrink: 0;
        transition: background 0.15s;
    }

    .topbar-icon-btn:hover { background: rgba(255, 255, 255, 0.1); }

    .topbar-search {
        flex: 1;
        max-width: 480px;
        height: 32px;
        margin: 0 auto;
        background: rgba(255, 255, 255, 0.06);
        border: 0.5px solid rgba(255, 255, 255, 0.1);
        border-radius: 8px;
        display: flex;
        align-items: center;
        gap: 8px;
        padding: 0 12px;
        cursor: text;
        text-align: left;
    }

    .search-placeholder {
        font-size: 13px;
        color: rgba(255, 255, 255, 0.25);
        flex: 1;
    }

    .search-shortcut {
        font-size: 10px;
        color: rgba(255, 255, 255, 0.2);
        border: 0.5px solid rgba(255, 255, 255, 0.12);
        border-radius: 3px;
        padding: 1px 5px;
        font-family: inherit;
    }

    .topbar-right {
        display: flex;
        align-items: center;
        gap: 10px;
        flex-shrink: 0;
    }

    .watching-badge {
        color: #3ddc84;
        font-size: 13px;
    }
</style>