<script lang="ts">
	// src/lib/HudTracker.svelte
	import { i18n } from '$lib/stores/i18n.js';

	interface HudAchievement {
		key: string;
		name: string;
		desc: string;
		icon: string;
		icon_gray: string;
		unlocked: boolean;
		unlocked_time: number | null;
		progress?: number | null;
		max_progress?: number | null;
	}

	let {
		visible = false,
		gameName = '',
		achievements = []
	} = $props<{
		visible: boolean;
		gameName: string;
		achievements: HudAchievement[];
	}>();

	function fmt(n: number): string {
		return Number.isInteger(n) ? String(n) : n.toFixed(1);
	}
</script>

{#if visible}
	<div class="hud-card" role="status" aria-live="polite">
		<div class="hud-header">
			<span class="hud-dot"></span>
			<span class="hud-title">{gameName || $i18n.t('hud.title')}</span>
			<span class="hud-count">{achievements.length}</span>
		</div>

		{#if achievements.length === 0}
			<div class="hud-empty">{$i18n.t('hud.empty')}</div>
		{:else}
			<div class="hud-list">
				{#each achievements as ach (ach.key)}
					{@const unlocked = ach.unlocked || Boolean(ach.unlocked_time)}
					{@const hasProg = Boolean(ach.max_progress && ach.max_progress > 0)}
					{@const cur = ach.progress ?? 0}
					{@const max = ach.max_progress ?? 0}
					{@const pct = hasProg ? Math.min(Math.max((cur / max) * 100, 0), 100) : unlocked ? 100 : 0}
					{@const icon = !unlocked && ach.icon_gray ? ach.icon_gray : ach.icon}
					<div class="hud-item" class:done={unlocked}>
						{#if icon}
							<img class="hud-icon" src={icon} alt="" />
						{:else}
							<div class="hud-icon placeholder">🏆</div>
						{/if}
						<div class="hud-info">
							<div class="hud-row">
								<span class="hud-name">{ach.name || ach.key}</span>
								{#if unlocked}
									<span class="hud-tag done">✓</span>
								{:else if hasProg}
									<span class="hud-tag">{fmt(cur)}/{fmt(max)}</span>
								{/if}
							</div>
							{#if ach.desc}
								<div class="hud-desc">{ach.desc}</div>
							{/if}
							{#if hasProg && !unlocked}
								<div class="hud-bar">
									<div class="hud-bar-fill" style:width="{pct}%"></div>
								</div>
							{/if}
						</div>
					</div>
				{/each}
			</div>
		{/if}
	</div>
{/if}

<style>
	.hud-card {
		font-family:
			'Segoe UI',
			system-ui,
			-apple-system,
			sans-serif;
		width: 340px;
		max-width: calc(100% - 16px);
		margin: 0;
		background: rgba(16, 18, 24, 0.88);
		backdrop-filter: blur(10px);
		border: 1px solid rgba(200, 169, 110, 0.32);
		border-radius: 14px;
		padding: 10px 12px;
		color: #f3f4f6;
		box-shadow: 0 10px 28px rgba(0, 0, 0, 0.55);
		pointer-events: none;
	}

	.hud-header {
		display: flex;
		align-items: center;
		gap: 7px;
		padding-bottom: 7px;
		margin-bottom: 7px;
		border-bottom: 1px solid rgba(255, 255, 255, 0.08);
		font-size: 11px;
		font-weight: 700;
		letter-spacing: 0.06em;
		text-transform: uppercase;
		color: #c8a96e;
	}

	.hud-dot {
		width: 6px;
		height: 6px;
		border-radius: 50%;
		background: #c8a96e;
		flex-shrink: 0;
	}

	.hud-title {
		flex: 1;
		white-space: nowrap;
		overflow: hidden;
		text-overflow: ellipsis;
	}

	.hud-count {
		background: rgba(200, 169, 110, 0.18);
		padding: 1px 6px;
		border-radius: 6px;
		font-size: 10px;
	}

	.hud-empty {
		font-size: 11px;
		color: #9ca3af;
		padding: 4px 2px;
	}

	.hud-list {
		display: flex;
		flex-direction: column;
		gap: 7px;
	}

	.hud-item {
		display: flex;
		align-items: center;
		gap: 9px;
	}

	.hud-item.done {
		opacity: 0.55;
	}

	.hud-icon {
		width: 32px;
		height: 32px;
		border-radius: 6px;
		object-fit: cover;
		background: #23262e;
		flex-shrink: 0;
		display: flex;
		align-items: center;
		justify-content: center;
		font-size: 14px;
	}

	.hud-info {
		flex: 1;
		min-width: 0;
		display: flex;
		flex-direction: column;
		gap: 2px;
	}

	.hud-row {
		display: flex;
		align-items: center;
		justify-content: space-between;
		gap: 6px;
	}

	.hud-name {
		font-size: 12px;
		font-weight: 700;
		white-space: nowrap;
		overflow: hidden;
		text-overflow: ellipsis;
	}

	.hud-tag {
		font-size: 10px;
		font-weight: 700;
		color: #c8a96e;
		background: rgba(200, 169, 110, 0.16);
		padding: 1px 5px;
		border-radius: 4px;
		flex-shrink: 0;
	}

	.hud-tag.done {
		color: #3ddc84;
		background: rgba(61, 220, 132, 0.16);
	}

	.hud-desc {
		font-size: 10.5px;
		color: #9ca3af;
		white-space: nowrap;
		overflow: hidden;
		text-overflow: ellipsis;
	}

	.hud-bar {
		height: 3px;
		background: rgba(255, 255, 255, 0.08);
		border-radius: 2px;
		overflow: hidden;
		margin-top: 2px;
	}

	.hud-bar-fill {
		height: 100%;
		background: #c8a96e;
		border-radius: 2px;
	}
</style>
