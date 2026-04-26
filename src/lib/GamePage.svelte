<script lang="ts">
    import { onMount, tick } from 'svelte';
    import { listen } from '@tauri-apps/api/event';
    import { loadAchievements, type Game, type Achievement } from '$lib/stores/Games.js';
    import { searchQuery, achievementJumpIntent } from '$lib/stores/ui.js';

    let { game }: { game: Game | null } = $props();

    let achievements = $state<Achievement[]>([]);
    let filter = $state<'all' | 'unlocked' | 'locked'>('all');
    let search = $state('');
    let sort = $state<'date' | 'rarity' | 'name'>('date');
    let revealed = $state(true);
    let loading = $state(true);
    let showHeaderLogo = $state(true);
    let jumpedToken = $state<number | null>(null);
    let flashAchievementKey = $state('');

    const rowRefs = new Map<string, HTMLDivElement>();
    let flashTimer: ReturnType<typeof setTimeout> | null = null;

    $effect(() => {
        if (game) {
            loading = true;
            // Pas d'annotation de type dans le callback → évite le conflit void/PromiseLike
            loadAchievements(game).then((result) => {
                achievements = result;
                loading = false;
            });
        }
    });

    onMount(() => {
        let unlisten: (() => void) | undefined;

        listen<Achievement[]>('achievement-unlocked', (event) => {
            if (game && Array.isArray(event.payload)) {
                achievements = event.payload;
            }
        }).then((fn) => {
            unlisten = fn;
        });

        return () => { unlisten?.(); };
    });

    let merged = $derived.by((): Achievement[] => {
        if (!game) return [];
        const schema = game.achievements ?? [];
        const stateMap = new Map<string, Achievement>(
            achievements.map((a) => [a.key.trim().toLowerCase(), a])
        );

        return schema.map((s: Achievement) => {
            const live = stateMap.get(s.key.trim().toLowerCase());
            return {
                ...s,
                unlocked: live?.unlocked ?? s.unlocked ?? false,
                unlocked_time: live?.unlocked_time ?? s.unlocked_time ?? null,
                rarity: live?.rarity || s.rarity || '',
                completionpercentage: live?.completionpercentage || s.completionpercentage || '',
                desc: live?.desc || s.desc || '',
            };
        });
    });

    let globalAchievementHighlight = $derived($searchQuery.trim().toLowerCase());

    let filtered = $derived.by((): Achievement[] => {
        let list: Achievement[] = merged;

        if (filter === 'unlocked') list = list.filter((a) => a.unlocked);
        if (filter === 'locked')   list = list.filter((a) => !a.unlocked);

        const localQuery = search.trim().toLowerCase();
        if (localQuery) {
            list = list.filter((a) =>
                a.name.toLowerCase().includes(localQuery) ||
                (a.desc ?? '').toLowerCase().includes(localQuery)
            );
        }

        list = [...list];

        if (sort === 'date') {
            list.sort((a: Achievement, b: Achievement) => {
                if (a.unlocked && b.unlocked) return (b.unlocked_time ?? 0) - (a.unlocked_time ?? 0);
                if (a.unlocked) return -1;
                if (b.unlocked) return 1;
                return 0;
            });
        } else if (sort === 'rarity') {
            list.sort((a: Achievement, b: Achievement) =>
                (parseFloat(a.completionpercentage) || 100) - (parseFloat(b.completionpercentage) || 100)
            );
        } else if (sort === 'name') {
            list.sort((a: Achievement, b: Achievement) => a.name.localeCompare(b.name));
        }

        return list;
    });

    let unlockedCount = $derived(merged.filter((a) => a.unlocked).length);
    let totalCount    = $derived(merged.length);
    let progressPct   = $derived(totalCount > 0 ? Math.round((unlockedCount / totalCount) * 100) : 0);

    function normalizeAchievementKey(key: string): string {
        return (key || '').trim().toLowerCase();
    }

    function trackAchievementRow(node: HTMLDivElement, key: string) {
        const normalized = normalizeAchievementKey(key);
        rowRefs.set(normalized, node);

        return {
            destroy() {
                rowRefs.delete(normalized);
            },
        };
    }

    function triggerAchievementFlash(key: string) {
        flashAchievementKey = key;
        if (flashTimer) clearTimeout(flashTimer);
        flashTimer = setTimeout(() => {
            flashAchievementKey = '';
            flashTimer = null;
        }, 850);
    }

    function delay(ms: number): Promise<void> {
        return new Promise((resolve) => setTimeout(resolve, ms));
    }

    $effect(() => {
        const intent = $achievementJumpIntent;
        if (!intent || !game) return;
        if (intent.gameId !== game.steam_id) return;
        if (jumpedToken === intent.token) return;

        jumpedToken = intent.token;
        const normalizedKey = normalizeAchievementKey(intent.achievementKey);

        // Evite qu'un filtre local masque la ligne cible au moment du jump.
        filter = 'all';
        search = '';

        void (async () => {
            let target: HTMLDivElement | undefined;

            for (let i = 0; i < 20; i += 1) {
                await tick();
                target = rowRefs.get(normalizedKey);
                if (target) break;
                await delay(50);
            }

            if (target) {
                target.scrollIntoView({ behavior: 'smooth', block: 'center' });
                triggerAchievementFlash(normalizedKey);
            }

            achievementJumpIntent.set(null);
        })();
    });

    function formatDate(ts: number | null): string {
        if (!ts) return '';
        return new Date(ts * 1000).toLocaleString('fr-FR', {
            day: '2-digit',
            month: '2-digit',
            year: 'numeric',
            hour: '2-digit',
            minute: '2-digit',
            second: '2-digit',
            hour12: false,
        });
    }

    function rarityLabel(pct: string): string {
        const n = parseFloat(pct);
        if (isNaN(n)) return '';
        if (n <= 0.1) return 'Mythic';
        if (n <= 1) return 'Légendaire'
        if (n <= 3)  return 'Épique';
        if (n <= 7)  return 'Très rare';
        if (n <= 15) return 'Rare';
        if (n <= 35) return 'Peu commun';
        return 'Commun';
    }

    type RarityPalette = {
        border: string;
        shadow: string;
        badgeBg: string;
        badgeColor: string;
        fill: string;
    };

    function rarityPalette(pct: string): RarityPalette {
        const n = parseFloat(pct);
        if (isNaN(n)) {
            return {
                border: 'rgba(106,112,128,0.4)',
                shadow: '0 0 0 1px rgba(106,112,128,0.12)',
                badgeBg: 'rgba(255,255,255,0.05)',
                badgeColor: '#6a7080',
                fill: '#4a5060',
            };
        }

        if (n <= 0.1) {
            return {
                border: '#ff3b5c',
                shadow: '0 0 0 1px rgba(255,59,92,0.16), 0 0 10px rgba(255,59,92,0.25)',
                badgeBg: 'rgba(255,59,92,0.14)',
                badgeColor: '#ff3b5c',
                fill: '#ff3b5c',
            };
        }
        if (n <= 1) {
            return {
                border: '#ffd85a',
                shadow: '0 0 0 1px rgba(255,216,90,0.14), 0 0 10px rgba(255,216,90,0.18)',
                badgeBg: 'rgba(255,216,90,0.14)',
                badgeColor: '#ffd85a',
                fill: '#ffd85a',
            };
        }
        if (n <= 3) {
            return {
                border: '#a855f7',
                shadow: '0 0 0 1px rgba(168,85,247,0.14), 0 0 10px rgba(168,85,247,0.2)',
                badgeBg: 'rgba(168,85,247,0.14)',
                badgeColor: '#a855f7',
                fill: '#a855f7',
            };
        }
        if (n <= 7) {
            return {
                border: '#f4b860',
                shadow: '0 0 0 1px rgba(244,184,96,0.12), 0 0 10px rgba(244,184,96,0.18)',
                badgeBg: 'rgba(244,184,96,0.14)',
                badgeColor: '#f4b860',
                fill: '#f4b860',
            };
        }
        if (n <= 15) {
            return {
                border: '#4ac8ff',
                shadow: '0 0 0 1px rgba(74,200,255,0.14), 0 0 10px rgba(74,200,255,0.18)',
                badgeBg: 'rgba(74,200,255,0.1)',
                badgeColor: '#4ac8ff',
                fill: '#4ac8ff',
            };
        }
        if (n <= 35) {
            return {
                border: '#3ddc84',
                shadow: '0 0 0 1px rgba(61,220,132,0.12), 0 0 10px rgba(61,220,132,0.16)',
                badgeBg: 'rgba(61,220,132,0.12)',
                badgeColor: '#3ddc84',
                fill: '#3ddc84',
            };
        }

        return {
            border: 'rgba(106,112,128,0.4)',
            shadow: '0 0 0 1px rgba(106,112,128,0.12)',
            badgeBg: 'rgba(255,255,255,0.05)',
            badgeColor: '#6a7080',
            fill: '#4a5060',
        };
    }

    function gameTitle(current: Game): string {
        return current.name?.trim() || `AppID ${current.steam_id}`;
    }

    function heroBackground(current: Game): string {
        return current.background_image_url || current.header_image_url || '';
    }

</script>

{#if !game}
    <div class="not-found">Jeu introuvable.</div>
{:else}
    <div class="game-page">

        <!-- Header -->
        <div class="game-header" style:--game-bg={heroBackground(game) ? `url('${heroBackground(game)}')` : 'none'}>
            {#if showHeaderLogo}
                <div class="game-logo-bg" aria-hidden="true">
                    <img
                        src="/logo.png"
                        alt=""
                        onerror={() => {
                            showHeaderLogo = false;
                        }}
                    />
                </div>
            {/if}
            <div class="game-header-overlay"></div>
            <div class="game-title-row">
                <h1 class="game-title">{gameTitle(game)}</h1>
                <span class="emulator-badge">{game.emulator}</span>
            </div>

            <div class="progress-block">
                <div class="progress-top">
                    <span class="progress-fraction">
                        <strong>{unlockedCount}</strong>
                        <span class="slash"> / {totalCount}</span>
                    </span>
                    <span class="progress-pct">{progressPct}%</span>
                    <div class="progress-meta">
                        <span>TOTAL <strong>{totalCount}</strong></span>
                        <span>DEBLOQUES <strong class="gold">{unlockedCount}</strong></span>
                    </div>
                </div>
                <div class="progress-bar-track">
                    <div class="progress-bar-fill" style="width: {progressPct}%"></div>
                </div>
            </div>
        </div>

        <!-- Contrôles -->
        <div class="controls">
            <div class="filter-tabs">
                <button class="tab" class:active={filter === 'all'}      onclick={() => filter = 'all'}>Tout</button>
                <button class="tab" class:active={filter === 'unlocked'} onclick={() => filter = 'unlocked'}>Débloqués</button>
                <button class="tab" class:active={filter === 'locked'}   onclick={() => filter = 'locked'}>Verrouillés</button>
            </div>

            <div class="search-wrap">
                <svg width="13" height="13" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2.5">
                    <circle cx="11" cy="11" r="8"/><line x1="21" y1="21" x2="16.65" y2="16.65"/>
                </svg>
                <input
                        class="search-input"
                        type="text"
                        placeholder="Chercher un succès..."
                        bind:value={search}
                />
                <span class="search-count">{filtered.length} / {totalCount}</span>
            </div>

            <select class="sort-select" bind:value={sort}>
                <option value="date">Date d'obtention</option>
                <option value="rarity">Rareté</option>
                <option value="name">Nom</option>
            </select>

            <button
                    class="reveal-btn"
                    class:active={revealed}
                    onclick={() => (revealed = !revealed)}
            >
                <svg width="13" height="13" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
                    {#if revealed}
                        <path d="M1 12s4-8 11-8 11 8 11 8-4 8-11 8-11-8-11-8z"/>
                        <circle cx="12" cy="12" r="3"/>
                    {:else}
                        <path d="M17.94 17.94A10.07 10.07 0 0 1 12 20c-7 0-11-8-11-8a18.45 18.45 0 0 1 5.06-5.94"/>
                        <path d="M9.9 4.24A9.12 9.12 0 0 1 12 4c7 0 11 8 11 8a18.5 18.5 0 0 1-2.16 3.19"/>
                        <line x1="1" y1="1" x2="23" y2="23"/>
                    {/if}
                </svg>
                Succès secrets
            </button>
        </div>

        <!-- Liste -->
        <div class="achievements-list">
            {#if loading}
                <div class="loading-state">Chargement…</div>
            {:else}
                {#each filtered as ach (ach.key)}
                    {@const isUnlocked = ach.unlocked || Boolean(ach.unlocked_time)}
                    {@const isLocked = !isUnlocked}
                    {@const isSecret = ach.hidden ?? false}
                    {@const descText = (ach.desc || '').trim()}
                    {@const showDescription = Boolean(descText) && (isUnlocked || !isSecret || revealed)}
                    {@const showMaskedDescription = isLocked && isSecret && !revealed}
                    {@const palette = rarityPalette(ach.completionpercentage)}
                    {@const achNormalizedKey = normalizeAchievementKey(ach.key)}
                    {@const isTopbarMatch = globalAchievementHighlight && (
                        ach.name.toLowerCase().includes(globalAchievementHighlight) ||
                        (ach.desc ?? '').toLowerCase().includes(globalAchievementHighlight)
                    )}

                    <div
                        class="ach-row"
                        class:is-secret-hidden={showMaskedDescription}
                        class:highlighted={Boolean(isTopbarMatch)}
                        class:jump-flash={flashAchievementKey === achNormalizedKey}
                        use:trackAchievementRow={ach.key}
                    >
                        <div class="ach-icon-wrap" class:grayscale={isLocked} style:border-color={palette.border} style:box-shadow={palette.shadow}>
                            {#if ach.icon}
                                <img src={ach.icon} alt={ach.name} />
                            {:else}
                                <span class="ach-placeholder">🏆</span>
                            {/if}
                        </div>

                        <div class="ach-info">
                            <div class="ach-name" class:muted={isLocked}>{ach.name || ach.key}</div>
                            {#if showDescription}
                                <div class="ach-desc">{descText}</div>
                            {:else if showMaskedDescription}
                                <div class="ach-desc" class:italic={!descText}>{descText || 'Description masquee'}</div>
                            {/if}
                            {#if ach.completionpercentage}
                                <div class="rarity-row">
                                    <span class="rarity-badge" style:background={palette.badgeBg} style:color={palette.badgeColor}>
                                        {rarityLabel(ach.completionpercentage)} · {parseFloat(ach.completionpercentage).toFixed(1)}%
                                    </span>
                                    <div class="rarity-track">
                                        <div class="rarity-fill" style:background={palette.fill} style:width={`${Math.min(parseFloat(ach.completionpercentage), 100)}%`}></div>
                                    </div>
                                </div>
                            {/if}
                        </div>

                        {#if isUnlocked && ach.unlocked_time}
                            <div class="ach-date">{formatDate(ach.unlocked_time)}</div>
                        {/if}
                    </div>
                {/each}

                {#if filtered.length === 0}
                    <div class="empty-state">Aucun succès trouvé.</div>
                {/if}
            {/if}
        </div>
    </div>
{/if}

<style>
    .game-page { height: 100%; display: flex; flex-direction: column; overflow: hidden; }

    .not-found, .loading-state, .empty-state {
        display: flex; align-items: center; justify-content: center;
        color: #6a7080; font-size: 14px;
    }
    .not-found { height: 100%; }
    .loading-state, .empty-state { height: 120px; }

    .game-header {
        position: relative;
        padding: 24px 28px 20px;
        border-bottom: 1px solid rgba(255,255,255,0.06);
        flex-shrink: 0;
        background-image: var(--game-bg);
        background-size: cover;
        background-position: center;
        overflow: hidden;
    }
    .game-logo-bg {
        position: absolute;
        inset: 0;
        display: flex;
        align-items: center;
        justify-content: center;
        pointer-events: none;
        z-index: 0;
    }
    .game-logo-bg img {
        width: min(92%, 760px);
        height: min(92%, 240px);
        object-fit: contain;
        opacity: 0.26;
        filter: drop-shadow(0 14px 28px rgba(0,0,0,0.4));
    }
    .game-header-overlay {
        position: absolute;
        inset: 0;
        background: linear-gradient(180deg, rgba(8, 10, 16, 0.35) 0%, rgba(8, 10, 16, 0.92) 100%);
        pointer-events: none;
        z-index: 1;
    }
    .game-title-row, .progress-block { position: relative; z-index: 2; }
    .game-title-row { display: flex; align-items: center; gap: 12px; margin-bottom: 18px; }
    .game-title { font-size: 26px; font-weight: 800; color: #fff; letter-spacing: -0.5px; }
    .emulator-badge {
        font-size: 10px; font-weight: 600; color: #d2d8e8;
        border: 1px solid rgba(255,255,255,0.2); border-radius: 4px;
        padding: 2px 8px; text-transform: uppercase; letter-spacing: 1px;
        background: rgba(0,0,0,0.35);
    }

    .progress-block { display: flex; flex-direction: column; gap: 10px; }
    .progress-top { display: flex; align-items: baseline; gap: 14px; }
    .progress-fraction { font-size: 22px; font-weight: 700; color: #fff; }
    .slash { font-size: 16px; color: #6a7080; }
    .progress-pct { font-size: 22px; font-weight: 700; color: #4ac8ff; margin-left: auto; }
    .progress-meta { display: flex; gap: 20px; font-size: 11px; color: #6a7080; text-transform: uppercase; letter-spacing: 0.8px; }
    .progress-meta strong { color: #fff; }
    .gold { color: var(--accent, #c8a96e) !important; }
    .progress-bar-track { height: 5px; background: rgba(255,255,255,0.08); border-radius: 3px; overflow: hidden; }
    .progress-bar-fill { height: 100%; background: #4ac8ff; border-radius: 3px; transition: width 0.5s ease; }

    .controls {
        display: flex; align-items: center; gap: 10px;
        padding: 12px 28px; border-bottom: 1px solid rgba(255,255,255,0.06);
        flex-shrink: 0; flex-wrap: wrap;
    }
    .filter-tabs { display: flex; background: rgba(255,255,255,0.04); border-radius: 8px; padding: 3px; gap: 2px; }
    .tab {
        padding: 5px 14px; border-radius: 6px; border: none; background: transparent;
        cursor: pointer; font-size: 13px; color: #6a7080;
        transition: background 0.12s, color 0.12s; font-family: inherit;
    }
    .tab.active { background: rgba(255,255,255,0.1); color: #fff; }
    .tab:hover:not(.active) { color: #aaa; }

    .search-wrap {
        flex: 1; min-width: 160px; max-width: 320px; height: 32px;
        background: rgba(255,255,255,0.05); border: 1px solid rgba(255,255,255,0.08);
        border-radius: 7px; display: flex; align-items: center;
        gap: 8px; padding: 0 10px; color: rgba(255,255,255,0.35);
    }
    .search-input { flex: 1; background: transparent; border: none; outline: none; font-size: 13px; color: #fff; font-family: inherit; }
    .search-input::placeholder { color: rgba(255,255,255,0.25); }
    .search-count { font-size: 11px; color: #6a7080; white-space: nowrap; }

    .sort-select {
        height: 32px; background: rgba(255,255,255,0.05);
        border: 1px solid rgba(255,255,255,0.08); border-radius: 7px;
        padding: 0 10px; font-size: 13px; color: #fff; cursor: pointer;
        font-family: inherit; outline: none;
        appearance: none;
        -webkit-appearance: none;
        color-scheme: dark;
    }
    .sort-select:focus {
        border-color: rgba(255,255,255,0.28);
        box-shadow: 0 0 0 1px rgba(255,255,255,0.16);
    }
    .sort-select option {
        background: #171a22;
        color: #f1f5ff;
    }
    .sort-select option:checked {
        background: #232938;
        color: #ffffff;
    }

    .reveal-btn {
        height: 32px; padding: 0 12px; background: rgba(74,200,255,0.08);
        border: 1px solid rgba(74,200,255,0.2); border-radius: 7px;
        font-size: 13px; color: #4ac8ff; cursor: pointer;
        display: flex; align-items: center; gap: 6px;
        font-family: inherit; transition: background 0.12s;
    }
    .reveal-btn.active { background: rgba(74,200,255,0.15); }
    .reveal-btn:hover  { background: rgba(74,200,255,0.2); }

    .achievements-list {
        flex: 1; overflow-y: auto; padding: 4px 28px 20px;
        scrollbar-width: thin; scrollbar-color: rgba(255,255,255,0.08) transparent;
    }

    .ach-row {
        display: flex; align-items: center; gap: 16px;
        padding: 14px 0; border-bottom: 1px solid rgba(255,255,255,0.05);
        transition: opacity 0.15s;
    }
    .ach-row.highlighted {
        border-bottom-color: transparent;
        border-radius: 8px;
        background: color-mix(in srgb, var(--accent, #c8a96e) 10%, transparent);
        box-shadow: inset 0 0 0 1px color-mix(in srgb, var(--accent, #c8a96e) 42%, transparent);
        padding-left: 10px;
        padding-right: 10px;
    }
    .ach-row.jump-flash {
        animation: achJumpFlash 0.85s ease;
        border-bottom-color: transparent;
        border-radius: 8px;
        box-shadow:
            inset 0 0 0 1px color-mix(in srgb, var(--accent, #c8a96e) 85%, transparent),
            0 0 18px color-mix(in srgb, var(--accent, #c8a96e) 30%, transparent);
        background: color-mix(in srgb, var(--accent, #c8a96e) 22%, transparent);
        padding-left: 10px;
        padding-right: 10px;
    }
    @keyframes achJumpFlash {
        0% {
            box-shadow:
                inset 0 0 0 1px color-mix(in srgb, var(--accent, #c8a96e) 95%, transparent),
                0 0 24px color-mix(in srgb, var(--accent, #c8a96e) 45%, transparent);
            background: color-mix(in srgb, var(--accent, #c8a96e) 30%, transparent);
        }
        100% {
            box-shadow:
                inset 0 0 0 1px color-mix(in srgb, var(--accent, #c8a96e) 42%, transparent),
                0 0 0 transparent;
            background: color-mix(in srgb, var(--accent, #c8a96e) 10%, transparent);
        }
    }
    .ach-row.is-secret-hidden { opacity: 0.35; filter: blur(2px); pointer-events: none; }
    .ach-row:last-child { border-bottom: none; }

    .ach-icon-wrap {
        width: 52px; height: 52px; border-radius: 10px; overflow: hidden;
        flex-shrink: 0; display: flex; align-items: center; justify-content: center;
        background: #1e1e1e; border: 2px solid transparent;
    }
    .ach-icon-wrap img { width: 100%; height: 100%; object-fit: cover; }
    .ach-icon-wrap.grayscale { filter: grayscale(100%) brightness(0.5); }
    .ach-placeholder { font-size: 22px; }

    .ach-info { flex: 1; min-width: 0; display: flex; flex-direction: column; gap: 4px; }
    .ach-name { font-size: 14px; font-weight: 600; color: #fff; white-space: nowrap; overflow: hidden; text-overflow: ellipsis; }
    .ach-name.muted { color: #6a7080; }
    .ach-desc { font-size: 12px; color: #6a7080; }
    .ach-desc.italic { font-style: italic; }

    .rarity-row { display: flex; align-items: center; gap: 8px; margin-top: 2px; }
    .rarity-badge {
        font-size: 10px; font-weight: 600; padding: 2px 6px; border-radius: 4px;
        text-transform: uppercase; letter-spacing: 0.4px; flex-shrink: 0;
    }

    .rarity-track { flex: 1; max-width: 140px; height: 3px; background: rgba(255,255,255,0.06); border-radius: 2px; overflow: hidden; }
    .rarity-fill { height: 100%; border-radius: 2px; }

    .ach-date { font-size: 12px; color: #6a7080; white-space: nowrap; flex-shrink: 0; align-self: flex-start; padding-top: 4px; }
</style>