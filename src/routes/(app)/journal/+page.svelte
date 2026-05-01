<script lang="ts">
    // src/routes/(app)/journal/+page.svelte
    import { games, type Game, type Achievement } from '$lib/stores/Games.js';

    let selectedFilter = $state('all');

    type AchievementEvent = {
        type: 'achievement';
        date: Date;
        achievement: Achievement;
        game: Game;
    };

    type CompletionEvent = {
        type: 'completion';
        date: Date;
        game: Game;
    };

    type JournalEvent = AchievementEvent | CompletionEvent;

    let allEvents = $derived.by((): JournalEvent[] => {
        const events: JournalEvent[] = [];

        $games.forEach(g => {
            if (selectedFilter !== 'all' && String(g.steam_id) !== selectedFilter) return;

            (g.achievements ?? []).filter(a => a.unlocked).forEach(a => {
                events.push({
                    type: 'achievement',
                    date: new Date((a.unlocked_time ?? 0) * 1000),
                    achievement: a,
                    game: g
                });
            });

            const total = g.achievements_total || g.achievements?.length || 0;
            const unlocked = g.achievements?.filter(a => a.unlocked) ?? [];
            if (total > 0 && unlocked.length === total) {
                const lastUnlockedTime = Math.max(...unlocked.map(a => a.unlocked_time ?? 0));
                events.push({
                    type: 'completion',
                    date: new Date(lastUnlockedTime * 1000),
                    game: g
                });
            }
        });

        return events.sort((a, b) => b.date.getTime() - a.date.getTime());
    });

    type EventGroup = { dateLabel: string; events: JournalEvent[] };

    let groupedEvents = $derived.by((): EventGroup[] => {
        const groups: EventGroup[] = [];
        allEvents.forEach(e => {
            const label = e.date.toLocaleDateString('fr-FR', { weekday: 'long', day: 'numeric', month: 'long', year: 'numeric' }).toUpperCase();
            let group = groups.find(g => g.dateLabel === label);
            if (!group) {
                group = { dateLabel: label, events: [] };
                groups.push(group);
            }
            group.events.push(e);
        });
        return groups;
    });

    function formatTime(date: Date) {
        return date.toLocaleTimeString('fr-FR', { hour: '2-digit', minute: '2-digit' });
    }

    function getGameIcon(game: Game): string {

        if (game.steamgrid_icon_url && game.steamgrid_icon_url.startsWith('http')) {
            return game.steamgrid_icon_url;
        }

        if (game.game_icon && game.game_icon.startsWith('http')) {
            return game.game_icon;
        }

        if (game.header_image_url) return game.header_image_url;

        return `https://cdn.cloudflare.steamstatic.com/steam/apps/${game.steam_id}/header.jpg`;
    }

</script>

<main class="journal-page">
    <header class="page-header">
        <div class="header-left">
            <div class="title-row">
                <div class="icon-circle"><svg width="20" height="20" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2.2"><path d="M4 19.5A2.5 2.5 0 0 1 6.5 17H20"/><path d="M6.5 2H20v20H6.5A2.5 2.5 0 0 1 4 19.5v-15A2.5 2.5 0 0 1 6.5 2z"/></svg></div>
                <h1>Journal</h1>
            </div>
            <p class="subtitle">Historique de vos succès débloqués</p>
        </div>
        <div class="header-right">
            <select bind:value={selectedFilter} class="game-select">
                <option value="all">Tous les jeux</option>
                {#each $games as game (game.steam_id)}
                    <option value={String(game.steam_id)}>{game.name}</option>
                {/each}
            </select>
        </div>
    </header>

    <div class="content scrollable">
        {#each groupedEvents as group (group.dateLabel)}
            <div class="day-group">
                <div class="day-header">
                    <span class="date-label">{group.dateLabel}</span>
                    <span class="count">{group.events.length} succès</span>
                </div>
                <div class="events-list">
                    {#each group.events as event (event.game.steam_id + ':' + (event.type === 'achievement' ? event.achievement.key : 'completion'))}
                        <div class="event-card" class:completion={event.type === 'completion'}>
                            <img
                                src={getGameIcon(event.game)}
                                alt={event.game.name}
                                class="game-icon"
                                onerror={(e) => {
                                    const t = e.target as HTMLImageElement;
                                    if (!t.src.includes('header.jpg')) {
                                        t.src = `https://cdn.cloudflare.steamstatic.com/steam/apps/${event.game.steam_id}/header.jpg`;
                                    }
                                }}
                            />
                            <div class="event-info">
                                {#if event.type === 'achievement'}
                                    <div class="achievement-row">
                                        <img src={event.achievement.icon} alt={event.achievement.name} class="achievement-icon" />
                                        <div class="text">
                                            <span class="name">{event.achievement.name}</span>
                                            <span class="game">{event.game.name}</span>
                                        </div>
                                    </div>
                                {:else}
                                    <div class="completion-row">
                                        <div class="trophy-icon">🏆</div>
                                        <div class="text">
                                            <span class="name">Jeu complété à 100% !</span>
                                            <span class="game">{event.game.name}</span>
                                        </div>
                                    </div>
                                {/if}
                            </div>
                            <div class="event-meta">
                                <span class="time">{formatTime(event.date)}</span>
                                {#if event.type === 'achievement'}
                                    <span class="rarity">{event.achievement.completionpercentage}%</span>
                                {/if}
                            </div>
                        </div>
                    {/each}
                </div>
            </div>
        {/each}

        {#if groupedEvents.length === 0}
            <div class="empty-state">
                <p>Aucun événement trouvé.</p>
            </div>
        {/if}
    </div>
</main>

<style>
    .journal-page {
        grid-column: 2 / -1;
        grid-row: 2 / -1;
        display: flex;
        flex-direction: column;
        background: #161616;
        color: #fff;
        height: 100%;
        overflow: hidden;
    }

    .page-header {
        padding: 32px;
        display: flex;
        justify-content: space-between;
        align-items: center;
    }

    .title-row { display: flex; align-items: center; gap: 12px; margin-bottom: 8px; }
    .icon-circle { width: 36px; height: 36px; border-radius: 50%; background: rgba(200, 169, 110, 0.1); color: var(--accent, #c8a96e); display: flex; align-items: center; justify-content: center; }
    .page-header h1 { font-size: 28px; font-weight: 700; margin: 0; color: #fff; }
    .subtitle { font-size: 14px; color: rgba(255,255,255,0.4); margin: 0; }

    .game-select { background: #1a1a1a; border: 1px solid rgba(255,255,255,0.1); color: #fff; padding: 8px 16px; border-radius: 8px; outline: none; font-size: 13px; }

    .content { flex: 1; overflow-y: auto; padding: 0 32px 32px; }

    .day-group { margin-bottom: 40px; }
    .day-header { display: flex; justify-content: space-between; align-items: center; padding-bottom: 12px; border-bottom: 1px solid rgba(255,255,255,0.05); margin-bottom: 16px; }
    .date-label { font-size: 12px; font-weight: 700; color: rgba(255,255,255,0.3); letter-spacing: 0.05em; }
    .day-header .count { font-size: 11px; color: rgba(255,255,255,0.2); }

    .events-list { display: flex; flex-direction: column; gap: 8px; }
    .event-card { background: #1a1a1a; border: 1px solid rgba(255,255,255,0.05); border-radius: 12px; padding: 12px 20px; display: flex; align-items: center; gap: 20px; }
    .event-card.completion { background: rgba(200, 169, 110, 0.05); border-color: rgba(200, 169, 110, 0.1); }

    .game-icon { width: 32px; height: 32px; border-radius: 6px; object-fit: cover; }
    .event-info { flex: 1; min-width: 0; }

    .achievement-row, .completion-row { display: flex; align-items: center; gap: 16px; }
    .achievement-icon { width: 40px; height: 40px; border-radius: 8px; }
    .trophy-icon { width: 40px; height: 40px; border-radius: 8px; background: rgba(200, 169, 110, 0.1); display: flex; align-items: center; justify-content: center; font-size: 20px; }

    .text { display: flex; flex-direction: column; min-width: 0; }
    .text .name { font-size: 15px; font-weight: 600; color: #fff; white-space: nowrap; overflow: hidden; text-overflow: ellipsis; }
    .text .game { font-size: 12px; color: rgba(255,255,255,0.3); white-space: nowrap; overflow: hidden; text-overflow: ellipsis; }

    .event-meta { display: flex; flex-direction: column; align-items: flex-end; gap: 4px; }
    .time { font-size: 12px; color: rgba(255,255,255,0.2); }
    .rarity { font-size: 11px; font-weight: 700; color: var(--accent, #c8a96e); background: rgba(200, 169, 110, 0.05); padding: 1px 6px; border-radius: 4px; }

    .empty-state { text-align: center; padding: 60px; color: rgba(255,255,255,0.2); }

    .scrollable::-webkit-scrollbar { width: 6px; }
    .scrollable::-webkit-scrollbar-track { background: transparent; }
    .scrollable::-webkit-scrollbar-thumb { background: rgba(255,255,255,0.1); border-radius: 3px; }
</style>
