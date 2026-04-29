<script lang="ts">
    interface Achievement {
        name?: string;
        desc?: string;
        icon?: string;
        rarity?: string;
        completionpercentage?: number;
        unlocked?: number;
        total?: number;
    }

    let { achievement, visible } = $props<{ achievement: Achievement | null, visible: boolean }>();

    const ringDash = $derived(
        achievement?.unlocked != null && achievement?.total
            ? (achievement.unlocked / achievement.total) * 94.25
            : 0
    );
</script>

{#if achievement}
    <div class="notif" class:visible={visible}>
        <div class="header">
            <span class="header-text">ACHIEVEMENT UNLOCKED</span>
        </div>
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
{/if}

<style>
    @import url('https://fonts.googleapis.com/css2?family=Rajdhani:wght@700&family=Inter:wght@400;600&display=swap');

    :global(body) {
        margin: 0;
        background: transparent;
        overflow: hidden;
    }

    .notif {
        font-family: 'Inter', sans-serif;
        background: #111214;
        border-radius: 16px;
        overflow: visible;
        width: 420px;
        box-shadow:
                0 0 0 1px rgba(255,255,255,0.06),
                0 8px 32px rgba(0,0,0,0.6);

        animation: slideIn 0.55s cubic-bezier(0.34, 1.42, 0.64, 1) forwards;
    }

    @keyframes slideIn {
        from { transform: translateX(-115%); opacity: 0; }
        to   { transform: translateX(0);    opacity: 1; }
    }

    .header {
        display: flex;
        justify-content: center;
        padding: 10px 20px 0;
    }

    .header-text {
        font-family: 'Rajdhani', sans-serif;
        font-size: 13px;
        font-weight: 700;
        letter-spacing: 0.12em;
        color: #fff;
        background: #1d6bf3;
        border-radius: 999px;
        padding: 5px 20px;
        box-shadow: 0 0 18px 4px rgba(29, 107, 243, 0.55);
    }

    .body {
        display: flex;
        align-items: center;
        gap: 14px;
        padding: 14px 16px 16px;
        background: #1c1e22;
        margin: 10px 0 0;
        border-radius: 0 0 16px 16px;
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

    .ring-bg  { fill: none; stroke: #2a2d33; stroke-width: 3.5; }
    .ring-fill { fill: none; stroke: #1d6bf3; stroke-width: 3.5; stroke-linecap: round; }

    .notif.visible {
        transform: translateX(0);
        opacity: 1;
    }
</style>