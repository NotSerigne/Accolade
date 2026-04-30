<script lang="ts">
    import { onMount } from 'svelte';
    import { goto } from '$app/navigation';
    import { games, selectedGameId, syncSteamMetadata, type Game } from '$lib/stores/Games.js';
    import { searchQuery, searchDraft, achievementJumpIntent, watcherActive, isSyncing } from '$lib/stores/ui.js';

    let inputEl: HTMLInputElement | null = null;
    let searchWrapEl: HTMLDivElement | null = null;
    let suggestionsOpen = $state(false);

    type Suggestion = {
        kind: 'game' | 'achievement';
        gameId: number;
        title: string;
        subtitle: string;
        query: string;
        iconUrl: string;
        achievementKey?: string;
    };

    type AchievementSearchCandidate = {
        gameId: number;
        title: string;
        subtitle: string;
        query: string;
        iconUrl: string;
        achievementKey: string;
        haystack: string;
    };

    function gameSuggestionIcon(game: Game): string {
        // Priority 1: Steam Grid DB icon (SGDB)
        if (game.steamgrid_icon_url && game.steamgrid_icon_url.startsWith('http')) {
            return game.steamgrid_icon_url;
        }

        // Priority 2: game_icon if it's an HTTP URL (SGDB fallback)
        if (game.game_icon && game.game_icon.startsWith('http')) {
            return game.game_icon;
        }

        // Priority 3: header image
        if (game.header_image_url) return game.header_image_url;

        // Priority 4: game_icon if it's a valid hash (Steam client icon)
        if (game.game_icon && !game.game_icon.includes('/') && !game.game_icon.includes('\\')) {
            return `https://media.steampowered.com/steamcommunity/public/images/apps/${game.steam_id}/${game.game_icon}.ico`;
        }

        // Final Fallback: Steam API header image
        return `https://cdn.akamai.steamstatic.com/steam/apps/${game.steam_id}/header.jpg`;
    }

    let searchPlaceholder = $derived('Rechercher un jeu ou un succes');

    let normalizedDraft = $derived($searchDraft.trim().toLowerCase());

    let achievementSearchCandidates = $derived.by((): AchievementSearchCandidate[] => {
        return $games.flatMap((g) =>
            (g.achievements ?? []).map((a) => {
                const title = a.name || a.key;
                const subtitle = `${g.name || `AppID ${g.steam_id}`} · ${(a.desc || '').slice(0, 60)}`;
                return {
                    gameId: g.steam_id,
                    title,
                    subtitle,
                    query: title,
                    iconUrl: a.icon || '',
                    achievementKey: a.key,
                    haystack: `${a.name || ''} ${a.desc || ''}`.toLowerCase(),
                };
            })
        );
    });

    let suggestions = $derived.by((): Suggestion[] => {
        if (!normalizedDraft) return [];

        const results: Suggestion[] = $games
            .filter((g) => `${g.name || ''} ${g.steam_id}`.toLowerCase().includes(normalizedDraft))
            .slice(0, 4)
            .map((g) => ({
                kind: 'game' as const,
                gameId: g.steam_id,
                title: g.name || `AppID ${g.steam_id}`,
                subtitle: `Jeu · AppID ${g.steam_id}`,
                query: g.name || String(g.steam_id),
                iconUrl: gameSuggestionIcon(g),
                achievementKey: undefined,
            }));

        if (results.length < 8) {
            for (const candidate of achievementSearchCandidates) {
                if (!candidate.haystack.includes(normalizedDraft)) continue;
                results.push({
                    kind: 'achievement',
                    gameId: candidate.gameId,
                    title: candidate.title,
                    subtitle: candidate.subtitle,
                    query: candidate.query,
                    iconUrl: candidate.iconUrl,
                    achievementKey: candidate.achievementKey,
                });
                if (results.length >= 8) break;
            }
        }

        return results;
    });

    function focusSearch() {
        inputEl?.focus();
        suggestionsOpen = true;
    }

    function applySearch(value?: string) {
        const query = (value ?? $searchDraft).trim();
        searchQuery.set(query);
    }

    function onSearchInput(event: Event) {
        const value = (event.currentTarget as HTMLInputElement).value;
        if (!value.trim()) {
            searchQuery.set('');
            suggestionsOpen = false;
        } else {
            suggestionsOpen = true;
        }
    }

    function selectSuggestion(item: Suggestion) {
        if (item.kind === 'game') {
            searchQuery.set('');
            achievementJumpIntent.set(null);
            selectedGameId.set(item.gameId);
            goto(`/games/${item.gameId}`);
        } else {
            searchQuery.set(item.query);
            achievementJumpIntent.set({
                gameId: item.gameId,
                achievementKey: item.achievementKey ?? item.query,
                token: Date.now(),
            });
            selectedGameId.set(item.gameId);
            goto(`/games/${item.gameId}`);
        }

        searchDraft.set('');
        suggestionsOpen = false;
    }

    function onSearchKeydown(event: KeyboardEvent) {
        if (event.key === 'Enter') {
            event.preventDefault();
            if (suggestions.length > 0 && suggestionsOpen) {
                void selectSuggestion(suggestions[0]);
            } else {
                applySearch();
                if (!$searchDraft.trim()) searchQuery.set('');
                suggestionsOpen = false;
            }
        }
        if (event.key === 'Escape') {
            suggestionsOpen = false;
            inputEl?.blur();
        }
    }

    async function handleSync() {
        if ($isSyncing) return;
        await syncSteamMetadata();
    }

    onMount(() => {
        const onKeyDown = (event: KeyboardEvent) => {
            if ((event.ctrlKey || event.metaKey) && event.key.toLowerCase() === 'k') {
                event.preventDefault();
                focusSearch();
            }
        };

        const onPointerDown = (event: MouseEvent) => {
            const target = event.target as Node;
            if (searchWrapEl && !searchWrapEl.contains(target)) {
                suggestionsOpen = false;
            }
        };

        window.addEventListener('keydown', onKeyDown);
        window.addEventListener('mousedown', onPointerDown);
        return () => {
            window.removeEventListener('keydown', onKeyDown);
            window.removeEventListener('mousedown', onPointerDown);
        };
    });
</script>

<header class="topbar">
    <div class="topbar-search-wrap" bind:this={searchWrapEl}>
        <div class="topbar-search" role="search">
            <svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
                <circle cx="11" cy="11" r="8"/><line x1="21" y1="21" x2="16.65" y2="16.65"/>
            </svg>
            <input
                class="search-input"
                type="text"
                bind:this={inputEl}
                bind:value={$searchDraft}
                placeholder={searchPlaceholder}
                onfocus={focusSearch}
                oninput={onSearchInput}
                onkeydown={onSearchKeydown}
            />
            <kbd class="search-shortcut">Ctrl+K</kbd>
        </div>

        {#if suggestionsOpen && normalizedDraft}
            <div class="search-suggestions" role="listbox" aria-label="Suggestions de recherche">
                {#if suggestions.length === 0}
                    <div class="suggestion-empty">Aucune suggestion</div>
                {:else}
                    {#each suggestions as item (item.kind + ':' + item.gameId + ':' + item.title)}
                        <button
                            class="suggestion-item"
                            class:game={item.kind === 'game'}
                            class:achievement={item.kind === 'achievement'}
                            type="button"
                            onclick={() => selectSuggestion(item)}
                        >
                            <div class="suggestion-thumb" class:game={item.kind === 'game'} class:achievement={item.kind === 'achievement'} class:with-image={Boolean(item.iconUrl)}>
                                {#if item.iconUrl}
                                    <img
                                        src={item.iconUrl}
                                        alt={item.title}
                                        onerror={(event) => {
                                            const target = event.currentTarget as HTMLImageElement;
                                            target.style.display = 'none';
                                            target.parentElement?.classList.add('no-image');
                                        }}
                                    />
                                {/if}
                                <span class="suggestion-fallback">{item.kind === 'game' ? '🎮' : '🏆'}</span>
                            </div>
                            <span class="suggestion-content">
                                <span class="suggestion-kind">{item.kind === 'game' ? 'Jeu' : 'Succes'}</span>
                                <span class="suggestion-title">{item.title}</span>
                                <span class="suggestion-subtitle">{item.subtitle}</span>
                            </span>
                        </button>
                    {/each}
                {/if}
            </div>
        {/if}
    </div>

    <div class="topbar-right">
        <button
            class="sync-btn"
            class:syncing={$isSyncing}
            onclick={handleSync}
            title="Synchroniser avec Steam"
            disabled={$isSyncing}
        >
            <svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2.5">
                <path d="M21 2v6h-6"/><path d="M3 12a9 9 0 0 1 15-6.7L21 8"/><path d="M3 22v-6h6"/><path d="M21 12a9 9 0 0 1-15 6.7L3 16"/>
            </svg>
            {#if $isSyncing}
                <span>Synchronisation...</span>
            {:else}
                <span>Actualiser</span>
            {/if}
        </button>

        <span class="watching-badge" class:active={$watcherActive}>● Scan</span>
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
        height: 100%;
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
        width: 100%;
        height: 32px;
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
    .topbar-search:focus-within {
        border-color: rgba(255, 255, 255, 0.42);
        box-shadow: 0 0 0 1px rgba(255, 255, 255, 0.2);
    }

    .topbar-search-wrap {
        flex: 1;
        max-width: 620px;
        margin: 0 auto;
        position: relative;
    }

    .search-input {
        flex: 1;
        border: none;
        background: transparent;
        color: #fff;
        font-size: 13px;
        outline: none;
        box-shadow: none;
        font-family: inherit;
    }

    .search-input::placeholder {
        color: rgba(255, 255, 255, 0.25);
    }


    .search-shortcut {
        font-size: 10px;
        color: rgba(255, 255, 255, 0.2);
        border: 0.5px solid rgba(255, 255, 255, 0.12);
        border-radius: 3px;
        padding: 1px 5px;
        font-family: inherit;
    }

    .search-suggestions {
        position: absolute;
        top: calc(100% + 6px);
        left: 0;
        right: 0;
        background: #161616;
        border: 1px solid rgba(255, 255, 255, 0.08);
        border-radius: 10px;
        padding: 6px;
        display: flex;
        flex-direction: column;
        gap: 4px;
        z-index: 120;
        box-shadow: 0 10px 32px rgba(0, 0, 0, 0.45);
    }

    .suggestion-item {
        width: 100%;
        border: 1px solid transparent;
        background: rgba(255, 255, 255, 0.02);
        border-radius: 8px;
        color: #fff;
        display: flex;
        align-items: center;
        gap: 8px;
        padding: 8px;
        text-align: left;
        cursor: pointer;
    }

    .suggestion-item:hover {
        background: rgba(255, 255, 255, 0.06);
        border-color: rgba(255, 255, 255, 0.12);
    }

    .suggestion-item.game {
        min-height: 56px;
    }

    .suggestion-item.achievement {
        min-height: 52px;
    }

    .suggestion-thumb {
        flex-shrink: 0;
        overflow: hidden;
        display: flex;
        align-items: center;
        justify-content: center;
        background: #1a1a1a;
    }

    .suggestion-thumb.game {
        width: 76px;
        height: 42px;
        border-radius: 6px;
        border: 1px solid rgba(255, 255, 255, 0.1);
    }

    .suggestion-thumb.achievement {
        width: 42px;
        height: 42px;
        border-radius: 8px;
        border: 1px solid rgba(255, 255, 255, 0.12);
    }

    .suggestion-thumb img {
        width: 100%;
        height: 100%;
        object-fit: cover;
    }

    .suggestion-fallback {
        font-size: 18px;
        opacity: 0.75;
    }

    .suggestion-thumb.with-image:not(.no-image) .suggestion-fallback {
        display: none;
    }

    .suggestion-kind {
        font-size: 9px;
        text-transform: uppercase;
        letter-spacing: 0.06em;
        color: #6a7080;
        border: 1px solid rgba(255, 255, 255, 0.1);
        border-radius: 4px;
        padding: 1px 5px;
        width: fit-content;
    }

    .suggestion-content {
        min-width: 0;
        display: flex;
        flex-direction: column;
        gap: 2px;
    }

    .suggestion-title {
        font-size: 13px;
        color: #e9edf8;
        white-space: nowrap;
        overflow: hidden;
        text-overflow: ellipsis;
    }

    .suggestion-subtitle {
        font-size: 11px;
        color: #7f8698;
        white-space: nowrap;
        overflow: hidden;
        text-overflow: ellipsis;
    }

    .suggestion-empty {
        font-size: 12px;
        color: #6a7080;
        padding: 8px;
    }

    .topbar-right {
        display: flex;
        align-items: center;
        gap: 12px;
        flex-shrink: 0;
    }

    .sync-btn {
        height: 32px;
        padding: 0 12px;
        border-radius: 8px;
        border: 1px solid rgba(255, 255, 255, 0.1);
        background: rgba(255, 255, 255, 0.05);
        color: rgba(255, 255, 255, 0.6);
        font-size: 12px;
        font-weight: 500;
        display: flex;
        align-items: center;
        gap: 8px;
        cursor: pointer;
        transition: all 0.2s;
        font-family: inherit;
    }

    .sync-btn:hover:not(:disabled) {
        background: rgba(255, 255, 255, 0.1);
        border-color: rgba(255, 255, 255, 0.2);
        color: #fff;
    }

    .sync-btn.syncing {
        color: var(--accent, #c8a96e);
        border-color: rgba(200, 169, 110, 0.3);
    }

    .sync-btn.syncing svg {
        animation: spin 1.2s linear infinite;
    }

    .sync-btn:disabled {
        cursor: wait;
        opacity: 0.8;
    }

    @keyframes spin {
        from { transform: rotate(0deg); }
        to { transform: rotate(360deg); }
    }

    .watching-badge {
        color: #6a7080;
        font-size: 13px;
        transition: color 0.3s;
    }
    .watching-badge.active {
        color: #3ddc84;
    }
</style>
