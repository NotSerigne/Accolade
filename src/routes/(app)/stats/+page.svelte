<script lang="ts">
    import { games, totalUnlockedAchievements, type Game, type Achievement } from '$lib/stores/Games.js';

    let activeTab = $state('overview');

    // Stats calculations
    let totalPossibleAchievements = $derived(
        $games.reduce((acc, g) => acc + (g.achievements_total || g.achievements?.length || 0), 0)
    );

    let globalCompletionRate = $derived(
        totalPossibleAchievements > 0
            ? Math.round(($totalUnlockedAchievements / totalPossibleAchievements) * 10000) / 100
            : 0
    );

    let remainingAchievements = $derived(totalPossibleAchievements - $totalUnlockedAchievements);

    let sortedByProgression = $derived.by(() => {
        return [...$games]
            .map(g => {
                const total = g.achievements_total || g.achievements?.length || 0;
                const unlocked = g.achievements?.filter(a => a.unlocked).length ?? 0;
                const pct = total > 0 ? (unlocked / total) * 100 : 0;
                return { ...g, unlockedCount: unlocked, totalCount: total, progression: pct };
            })
            .sort((a, b) => {
                if (Math.round(b.progression) !== Math.round(a.progression)) return b.progression - a.progression;
                return b.unlockedCount - a.unlockedCount;
            });
    });

    let topGames = $derived(sortedByProgression.slice(0, 10));
    let podium = $derived(sortedByProgression.slice(0, 3));

    let allUnlockedAchievements = $derived.by(() => {
        return $games.flatMap(g =>
            (g.achievements ?? [])
                .filter(a => a.unlocked)
                .map(a => ({ ...a, gameName: g.name }))
        ).sort((a, b) => (b.unlocked_time ?? 0) - (a.unlocked_time ?? 0));
    });

    let rarestAchievement = $derived.by(() => {
        const unlocked = allUnlockedAchievements;
        if (unlocked.length === 0) return null;
        return [...unlocked].sort((a, b) => parseFloat(a.completionpercentage) - parseFloat(b.completionpercentage))[0];
    });

    // Activity calculations
    let today = new Date();
    let oneYearAgo = new Date();
    oneYearAgo.setFullYear(today.getFullYear() - 1);

    function formatDate(date: Date) {
        return date.toLocaleDateString('fr-FR', { day: 'numeric', month: 'short', year: 'numeric' });
    }

    // --- Heatmap Data ---
    let heatmapData = $derived.by(() => {
        const unlocked = allUnlockedAchievements;
        const grid = [];
        const now = new Date();
        now.setHours(23, 59, 59, 999);

        // Find the most recent Sunday to align the grid
        const lastSunday = new Date(now);
        lastSunday.setDate(now.getDate() - now.getDay());

        // Start 51 weeks before that Sunday (total 52 weeks)
        const startDate = new Date(lastSunday);
        startDate.setDate(lastSunday.getDate() - 51 * 7);

        // Group achievements by day
        const dayMap = new Map<string, { count: number; achievements: any[] }>();
        unlocked.forEach(a => {
            if (!a.unlocked_time) return;
            const d = new Date(a.unlocked_time * 1000);
            const key = d.toISOString().split('T')[0];
            if (!dayMap.has(key)) {
                dayMap.set(key, { count: 0, achievements: [] });
            }
            const data = dayMap.get(key)!;
            data.count++;
            data.achievements.push(a);
        });

        // Build 52 columns
        for (let w = 0; w < 52; w++) {
            const col = [];
            for (let d = 0; d < 7; d++) {
                const date = new Date(startDate);
                date.setDate(startDate.getDate() + w * 7 + d);
                const key = date.toISOString().split('T')[0];
                const dayData = dayMap.get(key) || { count: 0, achievements: [] };

                // Don't show future dates
                const isFuture = date > now;

                col.push({
                    date,
                    count: dayData.count,
                    achievements: dayData.achievements,
                    isFuture
                });
            }
            grid.push(col);
        }
        return grid;
    });

    const monthLabels = ["Jan", "Fév", "Mar", "Avr", "Mai", "Juin", "Juil", "Août", "Sep", "Oct", "Nov", "Déc"];
    const dayLabels = ["Dim", "Lun", "Mar", "Mer", "Jeu", "Ven", "Sam"];

    let heatmapMonths = $derived.by(() => {
        const labels = [];
        let currentMonth = -1;
        heatmapData.forEach((col, i) => {
            const month = col[0].date.getMonth();
            if (month !== currentMonth) {
                labels.push({ label: monthLabels[month], index: i });
                currentMonth = month;
            }
        });
        return labels;
    });

    let cumulativeTimeframe = $state('all');
    let currentWindowOffset = $state(0);
    let chartContainerWidth = $state(0);
    let chartPhysicalWidth = $derived(cumulativeTimeframe === 'all' ? chartContainerWidth : chartContainerWidth * 3);

    let timeframeLabel = $derived.by(() => {
        switch (cumulativeTimeframe) {
            case '1y': return "Dernière année";
            case '6m': return "6 derniers mois";
            case '1m': return "Dernier mois";
            case '1w': return "Dernière semaine";
            default: return "Historique complet";
        }
    });

    // --- Cumulative Progress Data ---
    let cumulativeData = $derived.by(() => {
        const allUnlocked = [...allUnlockedAchievements].sort((a, b) => (a.unlocked_time ?? 0) - (b.unlocked_time ?? 0));
        if (allUnlocked.length === 0) return [];

        let startTime = 0;
        const now = new Date();

        if (cumulativeTimeframe === '1y') {
            const d = new Date(now);
            d.setFullYear(d.getFullYear() - 1);
            startTime = Math.floor(d.getTime() / 1000);
        } else if (cumulativeTimeframe === '6m') {
            const d = new Date(now);
            d.setMonth(d.getMonth() - 6);
            startTime = Math.floor(d.getTime() / 1000);
        } else if (cumulativeTimeframe === '1m') {
            const d = new Date(now);
            d.setMonth(d.getMonth() - 1);
            startTime = Math.floor(d.getTime() / 1000);
        } else if (cumulativeTimeframe === '1w') {
            const d = new Date(now);
            d.setDate(d.getDate() - 7);
            startTime = Math.floor(d.getTime() / 1000);
        }

        let runningCount = 0;
        const fullHistory: { x: number; y: number; date: Date; count: number }[] = [];

        allUnlocked.forEach(a => {
            runningCount++;
            fullHistory.push({
                x: a.unlocked_time ?? 0,
                y: runningCount,
                date: new Date((a.unlocked_time ?? 0) * 1000),
                count: runningCount
            });
        });

        if (cumulativeTimeframe === 'all') {
            const nowTime = Math.floor(Date.now() / 1000);
            fullHistory.push({
                x: nowTime,
                y: runningCount,
                date: new Date(nowTime * 1000),
                count: runningCount
            });
            return fullHistory;
        }

        // Filter to selected timeframe
        const recentHistory = fullHistory.filter(p => p.x >= startTime);

        // We need a starting point at exactly the start of the timeframe
        const countAtStart = fullHistory.filter(p => p.x < startTime).length;
        const startPoint = {
            x: startTime,
            y: countAtStart,
            date: new Date(startTime * 1000),
            count: countAtStart
        };

        const result = [startPoint, ...recentHistory];

        // Add a point for today
        const nowTime = Math.floor(Date.now() / 1000);
        result.push({
            x: nowTime,
            y: runningCount,
            date: new Date(nowTime * 1000),
            count: runningCount
        });

        return result;
    });

    let cumulativeMeta = $derived.by(() => {
        const width = chartPhysicalWidth || 800;
        const height = 180;

        if (cumulativeData.length === 0) return { minX: 0, maxX: 1, minY: 0, maxY: 1, path: '', points: [] as any[], width, height };
        const minX = cumulativeData[0].x;
        const maxX = cumulativeData[cumulativeData.length - 1].x;
        const minY = Math.min(...cumulativeData.map(p => p.y));
        const maxY = Math.max(...cumulativeData.map(p => p.y), 1);

        const range = maxY - minY;
        const displayMinY = Math.max(0, minY - range * 0.1);
        const displayMaxY = maxY + (range * 0.1 || 10);

        const points = cumulativeData.map(p => ({
            ...p,
            px: ((p.x - minX) / (maxX - minX)) * width,
            py: height - ((p.y - displayMinY) / (displayMaxY - displayMinY)) * height
        }));

        const path = points.map(p => `${p.px},${p.py}`).join(' ');

        return { minX, maxX, minY: displayMinY, maxY: displayMaxY, path, points, width, height };
    });
    // --- Weekly Rhythm Data ---
    let weeklyRhythm = $derived.by(() => {
        const unlocked = allUnlockedAchievements;
        const weeks = [];
        const now = new Date();

        // Last 26 weeks
        for (let i = 25; i >= 0; i--) {
            const d = new Date(now);
            d.setDate(now.getDate() - i * 7);
            const startOfWeek = new Date(d);
            startOfWeek.setDate(d.getDate() - d.getDay());
            startOfWeek.setHours(0,0,0,0);

            const endOfWeek = new Date(startOfWeek);
            endOfWeek.setDate(startOfWeek.getDate() + 7);

            const count = unlocked.filter(a => {
                const ut = (a.unlocked_time ?? 0) * 1000;
                return ut >= startOfWeek.getTime() && ut < endOfWeek.getTime();
            }).length;

            weeks.push({
                start: startOfWeek,
                count
            });
        }

        const max = Math.max(...weeks.map(w => w.count), 1);
        return weeks.map(w => ({ ...w, height: (w.count / max) * 100, isMax: w.count === max && max > 0 }));
    });

    let bestWeek = $derived.by(() => {
        const unlocked = allUnlockedAchievements;
        if (unlocked.length === 0) return { count: 0, date: '' };

        const weekMap = new Map<string, number>();
        unlocked.forEach(a => {
            if (!a.unlocked_time) return;
            const d = new Date(a.unlocked_time * 1000);
            const startOfWeek = new Date(d);
            startOfWeek.setDate(d.getDate() - d.getDay()); // Sunday
            const key = startOfWeek.toISOString().split('T')[0];
            weekMap.set(key, (weekMap.get(key) || 0) + 1);
        });

        let maxCount = 0;
        let bestKey = '';
        for (const [key, count] of weekMap.entries()) {
            if (count > maxCount) {
                maxCount = count;
                bestKey = key;
            }
        }
        return { count: maxCount, date: bestKey };
    });

    function getGameIcon(game: Game | any): string {
        if (game.steamgrid_icon_url && game.steamgrid_icon_url.startsWith('http')) return game.steamgrid_icon_url;
        if (game.game_icon && game.game_icon.startsWith('http')) return game.game_icon;
        if (game.game_icon && !game.game_icon.includes('/') && !game.game_icon.includes('\\')) {
            return `https://media.steampowered.com/steamcommunity/public/images/apps/${game.steam_id}/${game.game_icon}.ico`;
        }
        if (game.header_image_url) return game.header_image_url;
        return '';
    }

    // Tooltip state
    let tooltip = $state({ show: false, x: 0, y: 0, title: '', value: '', sub: '', list: [] as string[] });

    function showTooltip(e: MouseEvent, title: string, value: string, sub: string = '', list: string[] = []) {
        tooltip = {
            show: true,
            x: e.clientX,
            y: e.clientY,
            title,
            value,
            sub,
            list
        };
    }

    function hideTooltip() {
        tooltip.show = false;
    }

</script>

<main class="stats-page">
    {#if tooltip.show}
        <div class="custom-tooltip" style="left: {tooltip.x + 10}px; top: {tooltip.y + 10}px">
            <div class="tooltip-title">{tooltip.title}</div>
            <div class="tooltip-value">{tooltip.value}</div>
            {#if tooltip.sub}
                <div class="tooltip-sub">{tooltip.sub}</div>
            {/if}
            {#if tooltip.list.length > 0}
                <ul class="tooltip-list">
                    {#each tooltip.list.slice(0, 5) as item}
                        <li>• {item}</li>
                    {/each}
                    {#if tooltip.list.length > 5}
                        <li class="more">+{tooltip.list.length - 5} autres</li>
                    {/if}
                </ul>
            {/if}
        </div>
    {/if}

    <header class="stats-header">
        <div class="header-left">
            <h1>Statistiques</h1>
            <div class="tabs">
                <button type="button" class:active={activeTab === 'overview'} onclick={() => activeTab = 'overview'}>Vue d'ensemble</button>
                <button type="button" class:active={activeTab === 'activity'} onclick={() => activeTab = 'activity'}>Activité</button>
            </div>
        </div>
        <div class="header-right">
            <span class="date-now">{formatDate(today)}</span>
        </div>
    </header>

    <div class="stats-content scrollable">
        {#if activeTab === 'overview'}
            <section class="grid-overview">
                <!-- Stat Cards -->
                <div class="card stat-summary">
                    <div class="icon-circle games"><svg width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2"><path d="M6 12L3 17V19H21V17L18 12M6 12L12 3L18 12M6 12H18"/></svg></div>
                    <div class="card-info">
                        <span class="label">JEUX DÉTECTÉS</span>
                        <span class="value">{$games.length}</span>
                    </div>
                </div>

                <div class="card stat-summary">
                    <div class="icon-circle trophy"><svg width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2"><path d="M6 9H4.5a2.5 2.5 0 0 1 0-5H6M18 9h1.5a2.5 2.5 0 0 0 0-5H18M4 22h16M10 14.66V17c0 .55.47.98.97 1.21C11.47 18.44 12 19 12 19s.53-.56 1.03-.79c.5-.23.97-.66.97-1.21v-2.34M7 2h10a1 1 0 0 1 1 1v12a1 1 0 0 1-1 1H7a1 1 0 0 1-1-1V3a1 1 0 0 1 1-1Z"/></svg></div>
                    <div class="card-info">
                        <span class="label">SUCCÈS DÉBLOQUÉS</span>
                        <span class="value">{$totalUnlockedAchievements}</span>
                        <span class="sub">sur {totalPossibleAchievements}</span>
                    </div>
                </div>

                <div class="card stat-summary">
                    <div class="icon-circle rate"><svg width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2"><circle cx="12" cy="12" r="10"/><circle cx="12" cy="12" r="3"/></svg></div>
                    <div class="card-info">
                        <span class="label">TAUX DE COMPLÉTION</span>
                        <span class="value">{globalCompletionRate}%</span>
                        <span class="sub">moyenne globale</span>
                    </div>
                </div>

                <div class="card stat-summary">
                    <div class="icon-circle remaining"><svg width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2"><path d="M12 2v4M12 18v4M4.93 4.93l2.83 2.83M16.24 16.24l2.83 2.83M2 12h4M18 12h4M4.93 19.07l2.83-2.83M16.24 7.76l2.83-2.83"/></svg></div>
                    <div class="card-info">
                        <span class="label">SUCCÈS RESTANTS</span>
                        <span class="value">{remainingAchievements}</span>
                        <span class="sub">à débloquer</span>
                    </div>
                </div>

                <!-- Main Grid Row -->
                <div class="card top-games">
                    <div class="card-header">
                        <h2>Top jeux par progression</h2>
                        <span class="count">{$games.length} jeux</span>
                    </div>
                    <div class="games-progress-list">
                        {#each topGames as game}
                            <div class="game-progress-row">
                                <div class="game-info">
                                    <span class="name">{game.name}</span>
                                    {#if game.progression === 100}
                                        <span class="completed-tag">Remastered</span>
                                    {/if}
                                </div>
                                <div class="progress-bar-wrap">
                                    <div class="progress-bar">
                                        <div class="progress-fill" style="width: {game.progression}%"></div>
                                    </div>
                                    <span class="progress-pct">{Math.round(game.progression)}%</span>
                                </div>
                            </div>
                        {/each}
                    </div>
                </div>

                <div class="side-column">
                    <div class="card global-progression">
                        <h2>PROGRESSION GLOBALE</h2>
                        <div class="donut-wrap">
                            <div class="donut" style="--pct: {globalCompletionRate}">
                                <div class="donut-inner">
                                    <span class="donut-val">{globalCompletionRate}%</span>
                                    <span class="donut-label">complétion</span>
                                </div>
                            </div>
                        </div>
                        <div class="unlocked-count">
                            <span class="val">{$totalUnlockedAchievements} / {totalPossibleAchievements}</span>
                            <span class="lbl">succès débloqués</span>
                        </div>
                    </div>

                    <div class="card podium">
                        <h2>PODIUM</h2>
                        <div class="podium-list">
                            {#each podium as game, i}
                                <div class="podium-item">
                                    <div class="rank">{i + 1}</div>
                                    <span class="name">{game.name}</span>
                                    <span class="pct">{Math.round(game.progression)}%</span>
                                </div>
                            {/each}
                        </div>
                    </div>

                    <div class="card rarest">
                        <span class="card-subtitle"><svg width="12" height="12" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2"><circle cx="12" cy="12" r="10"/><path d="M12 8v8M8 12h8"/></svg> SUCCÈS LE PLUS RARE</span>
                        {#if rarestAchievement}
                            <div class="rarity-item">
                                <img src={rarestAchievement.icon} alt={rarestAchievement.name} />
                                <div class="info">
                                    <span class="name">{rarestAchievement.name}</span>
                                    <span class="game">{rarestAchievement.gameName}</span>
                                </div>
                                <span class="rarity-badge">{rarestAchievement.completionpercentage}%</span>
                            </div>
                        {:else}
                            <p class="empty">Aucun succès</p>
                        {/if}
                    </div>

                    <div class="card best-week">
                        <span class="card-subtitle"><svg width="12" height="12" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2"><path d="M13 2L3 14h9l-1 8 10-12h-9l1-8z"/></svg> MEILLEURE SEMAINE</span>
                        <div class="best-week-val">
                            <span class="val">{bestWeek.count}</span>
                            <span class="lbl">succès en une semaine</span>
                        </div>
                    </div>
                </div>
            </section>
        {:else}
            <section class="activity-view">
                <div class="card heatmap-card">
                    <div class="card-header">
                        <h2>Activité</h2>
                        <span class="sub">{$totalUnlockedAchievements} succès sur 52 semaines</span>
                    </div>
                    <div class="heatmap-container">
                        <div class="heatmap-wrapper">
                            <div class="heatmap-y-labels">
                                <span class="day-label">Lun</span>
                                <span class="day-label">Mer</span>
                                <span class="day-label">Ven</span>
                                <span class="day-label">Dim</span>
                            </div>
                            <div class="heatmap-main">
                                <div class="heatmap-months">
                                    {#each heatmapMonths as { label, index }}
                                        <span class="month-label" style="left: {index * 13}px">{label}</span>
                                    {/each}
                                </div>
                                <div class="heatmap-grid">
                                    {#each heatmapData as col}
                                        <div class="heatmap-col">
                                            {#each col as cell}
                                                <div
                                                    class="heatmap-cell"
                                                    class:lvl1={cell.count > 0 && cell.count <= 1}
                                                    class:lvl2={cell.count > 1 && cell.count <= 3}
                                                    class:lvl3={cell.count > 3 && cell.count <= 6}
                                                    class:lvl4={cell.count > 6 && cell.count <= 10}
                                                    class:lvl5={cell.count > 10}
                                                    class:is-future={cell.isFuture}
                                                    onmouseenter={(e) => showTooltip(
                                                        e,
                                                        cell.date.toLocaleDateString('fr-FR', { day: 'numeric', month: 'long', year: 'numeric' }),
                                                        `${cell.count} succès`,
                                                        '',
                                                        cell.achievements.map(a => a.name)
                                                    )}
                                                    onmouseleave={hideTooltip}
                                                ></div>
                                            {/each}
                                        </div>
                                    {/each}
                                </div>
                            </div>
                        </div>
                        <div class="heatmap-legend">
                            <span>Moins</span>
                            <div class="heatmap-cell"></div>
                            <div class="heatmap-cell lvl1"></div>
                            <div class="heatmap-cell lvl2"></div>
                            <div class="heatmap-cell lvl3"></div>
                            <div class="heatmap-cell lvl4"></div>
                            <div class="heatmap-cell lvl5"></div>
                            <span>Plus</span>
                        </div>
                    </div>
                </div>

                <div class="charts-row">
                    <div class="card chart-card">
                        <div class="chart-header-row">
                            <div class="title-group">
                                <h2>Progression cumulée</h2>
                                <p class="sub">{timeframeLabel}</p>
                            </div>
                            <div class="chart-nav-group">
                                {#if cumulativeTimeframe !== 'all'}
                                    <div class="window-nav">
                                        <button onclick={() => currentWindowOffset++}>
                                            <svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2.5"><path d="M15 18l-6-6 6-6"/></svg>
                                        </button>
                                        <button onclick={() => currentWindowOffset = Math.max(0, currentWindowOffset - 1)} disabled={currentWindowOffset === 0}>
                                            <svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2.5"><path d="M9 18l6-6-6-6"/></svg>
                                        </button>
                                    </div>
                                {/if}
                                <div class="timeframe-selector">
                                    <button class:active={cumulativeTimeframe === 'all'} onclick={() => { cumulativeTimeframe = 'all'; currentWindowOffset = 0; }}>Tout</button>
                                    <button class:active={cumulativeTimeframe === '1y'} onclick={() => { cumulativeTimeframe = '1y'; currentWindowOffset = 0; }}>An</button>
                                    <button class:active={cumulativeTimeframe === '1m'} onclick={() => { cumulativeTimeframe = '1m'; currentWindowOffset = 0; }}>Mois</button>
                                    <button class:active={cumulativeTimeframe === '1w'} onclick={() => { cumulativeTimeframe = '1w'; currentWindowOffset = 0; }}>Sem</button>
                                </div>
                            </div>
                        </div>
                        <div class="chart-container" bind:clientWidth={chartContainerWidth}>
                            {#if cumulativeData.length > 0}
                                <svg width="100%" height="100%" viewBox="0 0 {cumulativeMeta.width} {cumulativeMeta.height}" preserveAspectRatio="none">
                                    <defs>
                                        <linearGradient id="line-gradient" x1="0%" y1="0%" x2="0%" y2="100%">
                                            <stop offset="0%" stop-color="var(--accent, #c8a96e)" stop-opacity="0.2" />
                                            <stop offset="100%" stop-color="var(--accent, #c8a96e)" stop-opacity="0" />
                                        </linearGradient>
                                    </defs>

                                    <path
                                        d="M 0 {cumulativeMeta.height} L {cumulativeMeta.path} L {cumulativeMeta.width} {cumulativeMeta.height} Z"
                                        fill="url(#line-gradient)"
                                    />
                                    <polyline
                                        fill="none"
                                        stroke="var(--accent, #c8a96e)"
                                        stroke-width="2"
                                        vector-effect="non-scaling-stroke"
                                        points={cumulativeMeta.path}
                                    />

                                    {#each cumulativeMeta.points as p}
                                        <circle
                                            cx={p.px}
                                            cy={p.py}
                                            r="2.5"
                                            fill="var(--accent, #c8a96e)"
                                            class="chart-point"
                                            onmouseenter={(e) => showTooltip(
                                                e,
                                                p.date.toLocaleDateString('fr-FR', { day: 'numeric', month: 'short', year: 'numeric', hour: '2-digit', minute: '2-digit' }),
                                                `${p.count} succès cumulés`
                                            )}
                                            onmouseleave={hideTooltip}
                                        />
                                    {/each}
                                </svg>
                            {/if}
                        </div>
                    </div>
                    <div class="card chart-card">
                        <h2>Rythme de déblocage</h2>
                        <p class="sub">Succès par semaine (26 dernières semaines)</p>
                        <div class="chart-container">
                            <div class="bar-chart">
                                {#each weeklyRhythm as week}
                                    <div
                                        class="bar"
                                        class:max={week.isMax}
                                        class:empty={week.count === 0}
                                        style="height: {Math.max(week.height, 2)}%"
                                        onmouseenter={(e) => showTooltip(
                                            e,
                                            `Semaine du ${week.start.toLocaleDateString('fr-FR', { day: 'numeric', month: 'short' })}`,
                                            `${week.count} succès débloqués`
                                        )}
                                        onmouseleave={hideTooltip}
                                    ></div>
                                {/each}
                            </div>
                        </div>
                    </div>
                </div>

                <div class="card recent-list">
                    <div class="card-header">
                        <h2>Succès récents</h2>
                        <span class="count">50 derniers succès débloqués</span>
                    </div>
                    <div class="recent-scroll">
                        {#each allUnlockedAchievements.slice(0, 50) as a}
                            <div class="recent-item">
                                <img src={a.icon} alt={a.name} />
                                <div class="info">
                                    <div class="name-row">
                                        <span class="name">{a.name}</span>
                                        <span class="rarity-badge small">{a.completionpercentage}%</span>
                                    </div>
                                    <span class="game">{a.gameName}</span>
                                    <p class="desc">{a.desc}</p>
                                </div>
                                <span class="time">🕒 {new Date((a.unlocked_time ?? 0) * 1000).toLocaleDateString('fr-FR', { day: 'numeric', month: 'short' })}</span>
                            </div>
                        {/each}
                    </div>
                </div>
            </section>
        {/if}
    </div>
</main>

<style>
    .stats-page {
        grid-column: 2 / -1;
        grid-row: 2 / -1;
        display: flex;
        flex-direction: column;
        background: #161616;
        color: #fff;
        height: 100%;
        overflow: hidden;
    }

    .stats-header {
        padding: 24px 32px;
        display: flex;
        justify-content: space-between;
        align-items: flex-start;
    }

    .stats-header h1 {
        font-size: 28px;
        font-weight: 700;
        margin: 0 0 16px 0;
    }

    .tabs {
        display: flex;
        gap: 8px;
        background: rgba(255,255,255,0.05);
        padding: 4px;
        border-radius: 8px;
        width: fit-content;
    }

    .tabs button {
        background: transparent;
        border: none;
        color: rgba(255,255,255,0.5);
        padding: 6px 16px;
        border-radius: 6px;
        font-size: 13px;
        cursor: pointer;
        transition: all 0.2s;
    }

    .tabs button.active {
        background: rgba(200, 169, 110, 0.1);
        color: var(--accent, #c8a96e);
    }

    .date-now {
        color: rgba(255,255,255,0.3);
        font-size: 12px;
    }

    .scrollable {
        flex: 1;
        overflow-y: auto;
        padding: 0 32px 32px;
    }

    .grid-overview {
        display: grid;
        grid-template-columns: repeat(4, 1fr);
        grid-template-rows: auto 1fr;
        gap: 16px;
    }

    .card {
        background: #1a1a1a;
        border: 1px solid rgba(255,255,255,0.05);
        border-radius: 12px;
        padding: 20px;
    }

    .stat-summary {
        display: flex;
        align-items: center;
        gap: 16px;
    }

    .icon-circle {
        width: 40px;
        height: 40px;
        border-radius: 10px;
        display: flex;
        align-items: center;
        justify-content: center;
        background: rgba(255,255,255,0.05);
    }

    .icon-circle.games { color: var(--accent, #c8a96e); background: rgba(200, 169, 110, 0.1); }
    .icon-circle.trophy { color: #3ddc84; background: rgba(61, 220, 132, 0.1); }
    .icon-circle.rate { color: #a855f7; background: rgba(168, 85, 247, 0.1); }
    .icon-circle.remaining { color: #f4b860; background: rgba(244, 184, 96, 0.1); }

    .card-info { display: flex; flex-direction: column; }
    .card-info .label { font-size: 10px; font-weight: 600; color: rgba(255,255,255,0.3); letter-spacing: 0.05em; }
    .card-info .value { font-size: 24px; font-weight: 700; margin: 2px 0; color: #fff; }
    .card-info .sub { font-size: 11px; color: rgba(255,255,255,0.2); }

    .top-games { grid-column: 1 / 4; }
    .card-header { display: flex; justify-content: space-between; align-items: baseline; margin-bottom: 20px; }
    .card-header h2 { font-size: 16px; font-weight: 600; color: #fff; }
    .card-header .count { font-size: 11px; color: rgba(255,255,255,0.2); }

    .game-progress-row { margin-bottom: 16px; }
    .game-progress-row .game-info { display: flex; align-items: center; gap: 8px; margin-bottom: 8px; }
    .game-progress-row .name { font-size: 14px; font-weight: 500; color: #e0e0e0; }
    .game-progress-row .completed-tag { font-size: 10px; color: rgba(255,255,255,0.4); }

    .progress-bar-wrap { display: flex; align-items: center; gap: 12px; }
    .progress-bar { flex: 1; height: 12px; background: rgba(255,255,255,0.05); border-radius: 6px; overflow: hidden; }
    .progress-fill { height: 100%; background: var(--accent, #c8a96e); border-radius: 6px; }
    .progress-pct { font-size: 12px; color: var(--accent, #c8a96e); font-weight: 600; width: 40px; text-align: right; }

    .side-column { display: flex; flex-direction: column; gap: 16px; }

    .global-progression { display: flex; flex-direction: column; align-items: center; text-align: center; }
    .global-progression h2 { font-size: 11px; font-weight: 600; color: rgba(255,255,255,0.3); margin-bottom: 20px; align-self: flex-start; }

    .donut-wrap { position: relative; width: 140px; height: 140px; margin-bottom: 16px; }
    .donut {
        width: 100%; height: 100%;
        border-radius: 50%;
        background: conic-gradient(var(--accent, #c8a96e) calc(var(--pct) * 1%), rgba(255,255,255,0.05) 0);
        display: flex; align-items: center; justify-content: center;
    }
    .donut-inner {
        width: 80%; height: 80%;
        background: #1a1a1a;
        border-radius: 50%;
        display: flex; flex-direction: column; align-items: center; justify-content: center;
    }
    .donut-val { font-size: 24px; font-weight: 700; color: #fff; }
    .donut-label { font-size: 10px; color: rgba(255,255,255,0.3); }

    .unlocked-count { text-align: center; }
    .unlocked-count .val { font-size: 18px; font-weight: 600; display: block; color: #fff; }
    .unlocked-count .lbl { font-size: 10px; color: rgba(255,255,255,0.3); }

    .podium h2 { font-size: 11px; font-weight: 600; color: rgba(255,255,255,0.3); margin-bottom: 16px; }
    .podium-list { display: flex; flex-direction: column; gap: 12px; }
    .podium-item { display: flex; align-items: center; gap: 12px; }
    .podium-item .rank { width: 24px; height: 24px; border-radius: 12px; display: flex; align-items: center; justify-content: center; font-size: 10px; font-weight: 700; background: rgba(200, 169, 110, 0.1); color: var(--accent, #c8a96e); }
    .podium-item .name { flex: 1; font-size: 13px; color: #e0e0e0; white-space: nowrap; overflow: hidden; text-overflow: ellipsis; }
    .podium-item .pct { font-size: 12px; font-weight: 600; color: #fff; border-bottom: 2px solid var(--accent, #c8a96e); }

    .card-subtitle { font-size: 11px; font-weight: 600; color: rgba(255,255,255,0.3); display: flex; align-items: center; gap: 6px; margin-bottom: 12px; }
    .rarity-item { display: flex; align-items: center; gap: 12px; }
    .rarity-item img { width: 40px; height: 40px; border-radius: 8px; }
    .rarity-item .info { flex: 1; display: flex; flex-direction: column; min-width: 0; }
    .rarity-item .name { font-size: 14px; font-weight: 600; color: #fff; white-space: nowrap; overflow: hidden; text-overflow: ellipsis; }
    .rarity-item .game { font-size: 11px; color: rgba(255,255,255,0.3); white-space: nowrap; overflow: hidden; text-overflow: ellipsis; }
    .rarity-badge { font-size: 11px; font-weight: 700; color: #f4b860; background: rgba(244, 184, 96, 0.1); padding: 2px 6px; border-radius: 4px; }

    .best-week-val .val { font-size: 32px; font-weight: 700; color: #fff; display: block; }
    .best-week-val .lbl { font-size: 11px; color: rgba(255,255,255,0.3); }

    /* Activity View Styles */
    .activity-view { display: flex; flex-direction: column; gap: 16px; }
    .heatmap-card .sub { font-size: 12px; color: rgba(255,255,255,0.3); }
    .heatmap-container { margin-top: 20px; }

    .heatmap-wrapper {
        display: flex;
        gap: 8px;
        position: relative;
    }

    .heatmap-y-labels {
        display: flex;
        flex-direction: column;
        justify-content: space-between;
        padding: 18px 0 2px;
        font-size: 9px;
        color: rgba(255,255,255,0.2);
        height: 77px;
        margin-top: 18px;
    }

    .heatmap-main {
        flex: 1;
        display: flex;
        flex-direction: column;
        gap: 4px;
        overflow-x: auto;
        padding-bottom: 4px;
    }

    .heatmap-months {
        position: relative;
        height: 18px;
        font-size: 10px;
        color: rgba(255,255,255,0.2);
    }

    .month-label {
        position: absolute;
        white-space: nowrap;
    }

    .heatmap-grid { display: flex; gap: 3px; }
    .heatmap-col { display: flex; flex-direction: column; gap: 3px; }
    .heatmap-cell {
        width: 10px;
        height: 10px;
        background: rgba(255,255,255,0.05);
        border-radius: 2px;
        transition: transform 0.1s;
    }
    .heatmap-cell:hover:not(.is-future) {
        transform: scale(1.2);
        z-index: 10;
        outline: 1px solid rgba(255,255,255,0.2);
    }
    .heatmap-cell.lvl1 { background: color-mix(in srgb, var(--accent, #c8a96e) 20%, transparent); }
    .heatmap-cell.lvl2 { background: color-mix(in srgb, var(--accent, #c8a96e) 40%, transparent); }
    .heatmap-cell.lvl3 { background: color-mix(in srgb, var(--accent, #c8a96e) 60%, transparent); }
    .heatmap-cell.lvl4 { background: color-mix(in srgb, var(--accent, #c8a96e) 80%, transparent); }
    .heatmap-cell.lvl5 { background: var(--accent, #c8a96e); }
    .heatmap-cell.is-future { opacity: 0.1; cursor: default; }

    .heatmap-legend { display: flex; align-items: center; gap: 6px; font-size: 10px; color: rgba(255,255,255,0.3); margin-top: 12px; justify-content: flex-end; }

    .charts-row { display: grid; grid-template-columns: 1fr 1fr; gap: 16px; }
    .chart-card .sub { font-size: 11px; color: rgba(255,255,255,0.3); margin: 4px 0 20px; }

    .chart-header-row {
        display: flex;
        justify-content: space-between;
        align-items: flex-start;
        margin-bottom: 20px;
    }

    .chart-nav-group {
        display: flex;
        align-items: center;
        gap: 12px;
    }

    .window-nav {
        display: flex;
        gap: 2px;
        background: rgba(255,255,255,0.03);
        padding: 2px;
        border-radius: 6px;
    }

    .window-nav button {
        background: transparent;
        border: none;
        color: rgba(255,255,255,0.3);
        width: 24px;
        height: 24px;
        display: flex;
        align-items: center;
        justify-content: center;
        border-radius: 4px;
        cursor: pointer;
        transition: all 0.2s;
    }

    .window-nav button:hover:not(:disabled) {
        background: rgba(255,255,255,0.05);
        color: #fff;
    }

    .window-nav button:disabled {
        opacity: 0.1;
        cursor: default;
    }

    .timeframe-selector {
        display: flex;
        gap: 4px;
        background: rgba(255,255,255,0.03);
        padding: 2px;
        border-radius: 6px;
    }

    .timeframe-selector button {
        background: transparent;
        border: none;
        color: rgba(255,255,255,0.3);
        padding: 4px 8px;
        border-radius: 4px;
        font-size: 10px;
        font-weight: 600;
        cursor: pointer;
        transition: all 0.2s;
        text-transform: uppercase;
    }

    .timeframe-selector button:hover {
        color: rgba(255,255,255,0.6);
    }

    .timeframe-selector button.active {
        background: rgba(255,255,255,0.07);
        color: var(--accent, #c8a96e);
    }

    .chart-container {
        height: 180px;
        position: relative;
        padding-bottom: 20px;
        border-bottom: 1px solid rgba(255,255,255,0.05);
    }

    .chart-point {
        cursor: pointer;
        transition: r 0.2s, opacity 0.2s;
        opacity: 0.4;
    }
    .chart-point:hover {
        r: 4;
        opacity: 1;
    }
    .bar-chart { display: flex; align-items: flex-end; gap: 4px; width: 100%; height: 100%; justify-content: space-between; }
    .bar {
        flex: 1;
        border-radius: 2px 2px 0 0;
        background: color-mix(in srgb, var(--accent, #c8a96e) 25%, transparent);
        transition: all 0.2s;
        cursor: pointer;
    }
    .bar:hover {
        background: color-mix(in srgb, var(--accent, #c8a96e) 50%, transparent);
    }
    .bar.empty {
        background: rgba(255, 255, 255, 0.05);
    }
    .bar.max {
        background: #3ddc84;
    }    .bar.max:hover {
        background: #4ef095;
    }

    .recent-list { flex: 1; }
    .recent-scroll { display: flex; flex-direction: column; gap: 16px; margin-top: 12px; }
    .recent-item { display: flex; align-items: flex-start; gap: 16px; padding-bottom: 16px; border-bottom: 1px solid rgba(255,255,255,0.03); }
    .recent-item img { width: 48px; height: 48px; border-radius: 8px; flex-shrink: 0; }
    .recent-item .info { flex: 1; display: flex; flex-direction: column; min-width: 0; }

    .name-row { display: flex; align-items: center; gap: 8px; margin-bottom: 2px; }
    .recent-item .name { font-size: 15px; font-weight: 600; color: #fff; }
    .rarity-badge.small { font-size: 9px; padding: 1px 4px; }

    .recent-item .game { font-size: 12px; color: var(--accent, #c8a96e); font-weight: 500; margin-bottom: 4px; }
    .recent-item .desc { font-size: 13px; color: rgba(255,255,255,0.4); line-height: 1.4; display: -webkit-box; -webkit-line-clamp: 2; -webkit-box-orient: vertical; overflow: hidden; }
    .recent-item .time { font-size: 11px; color: rgba(255,255,255,0.2); white-space: nowrap; margin-top: 4px; }

    /* Custom Tooltip */
    .custom-tooltip {
        position: fixed;
        background: #1e2329;
        border: 1px solid rgba(255,255,255,0.1);
        padding: 12px;
        border-radius: 8px;
        z-index: 1000;
        pointer-events: none;
        box-shadow: 0 8px 24px rgba(0,0,0,0.5);
        min-width: 150px;
        max-width: 250px;
    }

    .tooltip-title { font-size: 11px; color: rgba(255,255,255,0.4); margin-bottom: 4px; }
    .tooltip-value { font-size: 16px; font-weight: 700; color: #fff; margin-bottom: 8px; }
    .tooltip-sub { font-size: 12px; color: rgba(255,255,255,0.6); margin-bottom: 8px; }

    .tooltip-list {
        list-style: none;
        padding: 0;
        margin: 0;
        border-top: 1px solid rgba(255,255,255,0.05);
        padding-top: 8px;
    }

    .tooltip-list li {
        font-size: 12px;
        color: rgba(255,255,255,0.7);
        margin-bottom: 4px;
        white-space: nowrap;
        overflow: hidden;
        text-overflow: ellipsis;
    }

    .tooltip-list li.more {
        color: rgba(255,255,255,0.3);
        font-style: italic;
    }

    .empty { font-size: 12px; color: rgba(255,255,255,0.2); }

    .scrollable::-webkit-scrollbar { width: 6px; }
    .scrollable::-webkit-scrollbar-track { background: transparent; }
    .scrollable::-webkit-scrollbar-thumb { background: rgba(255,255,255,0.1); border-radius: 3px; }
</style>
