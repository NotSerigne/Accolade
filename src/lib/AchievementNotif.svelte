<script lang="ts">
    interface Achievement {
        name?: string;
        desc?: string;
        icon?: string;
        rarity?: string;
        completionpercentage?: number;
        unlocked?: number;
        total?: number;
        unlockedAt?: string; // ISO string ou timestamp
    }

    type Position = 'top-left' | 'top-center' | 'top-right' | 'bottom-left' | 'bottom-center' | 'bottom-right';

    let { achievement, visible, position = 'top-right' } = $props<{
        achievement: Achievement | null,
        visible: boolean,
        position?: Position
    }>();

    const ringDash = $derived(
        achievement?.unlocked != null && achievement?.total
            ? (achievement.unlocked / achievement.total) * 94.25
            : 0
    );

    const formattedDate = $derived(() => {
        if (!achievement?.unlockedAt) return null;
        const d = new Date(achievement.unlockedAt);
        return d.toLocaleDateString('en-GB', { day: '2-digit', month: 'short', year: 'numeric' })
            + ' · '
            + d.toLocaleTimeString('en-GB', { hour: '2-digit', minute: '2-digit' });
    });

    const animationClass = $derived(() => {
        switch (position) {
            case 'top-left':
            case 'bottom-left':    return 'slide-right';
            case 'top-right':
            case 'bottom-right':   return 'slide-left';
            case 'top-center':
            case 'bottom-center':  return 'curtain';
            default:               return 'slide-left';
        }
    });
</script>

{#if achievement}
    <div class="notif-wrapper" class:visible={visible}>
        <div class="badge-wrap">
            <span class="badge">ACHIEVEMENT UNLOCKED</span>
        </div>
        <div class="card {animationClass()}">
            <div class="body">
                <div class="icon-wrap">
                    {#if achievement.icon}
                        <img src={achievement.icon} alt="Achievement icon" class="icon" />
                    {:else}
                        <div class="icon-placeholder"></div>
                    {/if}
                </div>
                <div class="info">
                    <div class="name">{achievement.name}</div>
                    <div class="desc">{achievement.desc}</div>
                    <div class="meta">
                        <span class="rarity">{achievement.rarity}</span>
                        {#if achievement.completionpercentage != null}
                            <span class="pct">— {achievement.completionpercentage}%</span>
                        {/if}
                        {#if formattedDate()}
                            <span class="date">· {formattedDate()}</span>
                        {/if}
                    </div>
                </div>
                {#if achievement.unlocked != null && achievement.total != null}
                    <div class="progress-wrap">
                        <div class="fraction">{achievement.unlocked}/{achievement.total}</div>
                        <svg class="ring" viewBox="0 0 36 36">
                            <circle class="ring-bg" cx="18" cy="18" r="15" />
                            <circle
                                    class="ring-fill"
                                    cx="18" cy="18" r="15"
                                    stroke-dasharray={ringDash + ' 94.25'}
                                    stroke-dashoffset="23.5"
                            />
                        </svg>
                    </div>
                {/if}
            </div>
        </div>
    </div>
{/if}

<style>
    @import url('https://fonts.googleapis.com/css2?family=Rajdhani:wght@700&family=Inter:wght@400;600&display=swap');

    :global(body) {
        margin: 0;
        background: transparent;
        overflow: hidden;
    }

    .notif-wrapper {
        font-family: 'Inter', sans-serif;
        width: 420px;
        display: flex;
        flex-direction: column;
        align-items: center;
        opacity: 0;
        pointer-events: none;
    }

    .notif-wrapper.visible {
        opacity: 1;
        pointer-events: auto;
    }

    .badge-wrap {
        display: flex;
        justify-content: center;
        width: 100%;
        padding: 0 16px;
        box-sizing: border-box;
        z-index: 1;
        margin-bottom: -14px;
    }

    .badge {
        display: block;
        width: calc(100% - 32px);
        text-align: center;
        font-family: 'Rajdhani', sans-serif;
        font-size: 20px;
        font-weight: 700;
        letter-spacing: 0.18em;
        color: #fff;
        background: #1d6bf3;
        border-radius: 16px;
        padding: 8px 28px;
        box-shadow: 0 0 22px 6px rgba(29, 107, 243, 0.5);
    }

    .card {
        width: 100%;
        background: #1c1e22;
        border-radius: 16px;
        box-shadow:
                0 0 0 1px rgba(255,255,255,0.06),
                0 8px 32px rgba(0,0,0,0.6);
        overflow: hidden;
    }

    .body {
        display: flex;
        align-items: center;
        gap: 14px;
        padding: 22px 16px 16px; /* 22px top pour laisser place au badge */
    }

    .icon, .icon-placeholder {
        width: 72px;
        height: 72px;
        border-radius: 10px;
        background: #2a2d33;
        object-fit: cover;
        flex-shrink: 0;
    }

    .info {
        flex: 1;
        min-width: 0;
        display: flex;
        flex-direction: column;
        gap: 3px;
    }

    .name {
        font-family: 'Rajdhani', sans-serif;
        font-size: 18px;
        font-weight: 700;
        color: #1d6bf3;
        letter-spacing: 0.04em;
        white-space: nowrap;
        overflow: hidden;
        text-overflow: ellipsis;
    }

    .desc {
        font-size: 12px;
        font-weight: 600;
        color: #e0e0e0;
        white-space: nowrap;
        overflow: hidden;
        text-overflow: ellipsis;
    }

    .meta {
        display: flex;
        align-items: center;
        gap: 4px;
        margin-top: 6px;
        flex-wrap: wrap;
    }

    .rarity {
        font-family: 'Rajdhani', sans-serif;
        font-size: 13px;
        font-weight: 700;
        color: #1d6bf3;
        letter-spacing: 0.08em;
    }

    .pct {
        font-size: 13px;
        font-weight: 600;
        color: #9ca3af;
    }

    .date {
        font-size: 12px;
        color: #6b7280;
    }

    .progress-wrap {
        flex-shrink: 0;
        position: relative;
        width: 52px;
        height: 52px;
    }

    .fraction {
        font-family: 'Rajdhani', sans-serif;
        font-size: 14px;
        font-weight: 700;
        color: #fff;
        position: absolute;
        inset: 0;
        display: flex;
        align-items: center;
        justify-content: center;
    }

    .ring {
        width: 52px;
        height: 52px;
        transform: rotate(-90deg);
    }

    .ring-bg   { fill: none; stroke: #2a2d33; stroke-width: 3.5; }
    .ring-fill { fill: none; stroke: #1d6bf3; stroke-width: 3.5; stroke-linecap: round; }

    :global(html),
    :global(body) {
        background: transparent !important;
    }
    :global(body > div) {
        background: transparent !important;
        margin: 0;
        overflow: hidden;
    }
    
    /* ── Animations ── */

    .slide-right {
        animation: slideRight 0.5s cubic-bezier(0.22, 1, 0.36, 1) forwards;
    }
    .slide-left {
        animation: slideLeft 0.5s cubic-bezier(0.22, 1, 0.36, 1) forwards;
    }
    .curtain {
        animation: curtain 0.55s cubic-bezier(0.22, 1, 0.36, 1) forwards;
    }

    @keyframes slideRight {
        from { transform: translateX(-18px); opacity: 0; }
        to   { transform: translateX(0);     opacity: 1; }
    }
    @keyframes slideLeft {
        from { transform: translateX(18px);  opacity: 0; }
        to   { transform: translateX(0);     opacity: 1; }
    }
    @keyframes curtain {
        from { transform: scaleY(0.85) translateY(-8px); opacity: 0; transform-origin: top center; }
        to   { transform: scaleY(1)    translateY(0);    opacity: 1; transform-origin: top center; }
    }
</style>