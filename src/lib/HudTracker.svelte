<script lang="ts">
	// src/lib/HudTracker.svelte
	import { i18n } from '$lib/stores/i18n.js';
	import { getRarityPalette } from '$lib/stores/Games.js';

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
		rarity?: number | string | null;
		completionpercentage?: string | number | null;
		pinned?: boolean;
	}

	let {
		visible = false,
		gameName = '',
		achievements = [],
		totalCount = undefined,
		unlockedCount = undefined,
		pinnedKeys = []
	} = $props<{
		visible: boolean;
		gameName: string;
		achievements: HudAchievement[];
		totalCount?: number;
		unlockedCount?: number;
		pinnedKeys?: string[];
	}>();

	let pinnedSet = $derived(new Set(pinnedKeys.map((k: string) => k.trim().toLowerCase())));

	let effectiveTotal = $derived(totalCount ?? achievements.length);
	let effectiveUnlocked = $derived(
		unlockedCount ??
			achievements.filter((a: HudAchievement) => a.unlocked || Boolean(a.unlocked_time)).length
	);
	let overallPct = $derived(
		effectiveTotal > 0 ? Math.min(100, Math.round((effectiveUnlocked / effectiveTotal) * 100)) : 0
	);

	function fmt(n: number): string {
		return Number.isInteger(n) ? String(n) : n.toFixed(1);
	}

	function getRarityPct(ach: HudAchievement): number | null {
		const raw =
			ach.completionpercentage !== undefined &&
			ach.completionpercentage !== null &&
			ach.completionpercentage !== ''
				? ach.completionpercentage
				: ach.rarity;
		if (raw === undefined || raw === null || raw === '') return null;
		const n = typeof raw === 'number' ? raw : parseFloat(String(raw));
		return Number.isFinite(n) ? n : null;
	}
</script>

{#if visible}
	<div class="hud-tracker" role="status" aria-live="polite">
		<div class="hud-header">
			<span class="hud-dot" aria-hidden="true"></span>
			<div class="hud-title-group">
				<span class="hud-subtitle">{$i18n.t('hud.trackerTitle')}</span>
				<span class="hud-title">{gameName || $i18n.t('hud.title')}</span>
			</div>
			<span class="hud-count tabular-nums">{effectiveUnlocked} / {effectiveTotal}</span>
		</div>

		<div class="hud-progress-section">
			<div class="hud-progress-track">
				<div class="hud-progress-fill" style:width="{overallPct}%"></div>
			</div>
			<span class="hud-progress-pct tabular-nums">{overallPct}%</span>
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
					{@const isPinned = Boolean(ach.pinned) || pinnedSet.has(ach.key.trim().toLowerCase())}
					{@const rarityPct = getRarityPct(ach)}
					{@const palette = rarityPct !== null ? getRarityPalette(rarityPct) : null}
					<div class="ach-row" class:done={unlocked}>
						{#if icon}
							<img class="hud-icon" src={icon} alt="" loading="lazy" decoding="async" />
						{:else}
							<div class="hud-icon placeholder">
								<svg
									width="16"
									height="16"
									viewBox="0 0 24 24"
									fill="none"
									stroke="currentColor"
									stroke-width="2"
									stroke-linecap="round"
									stroke-linejoin="round"
									aria-hidden="true"
								>
									<path
										d="M6 9H4.5a2.5 2.5 0 0 1 0-5H6M18 9h1.5a2.5 2.5 0 0 0 0-5H18M4 22h16M10 14.66V17c0 .55.47.98.97 1.21C11.47 18.44 12 19 12 19s.53-.56 1.03-.79c.5-.23.97-.66.97-1.21v-2.34M7 2h10a1 1 0 0 1 1 1v12a1 1 0 0 1-1 1H7a1 1 0 0 1-1-1V3a1 1 0 0 1 1-1Z"
									/>
								</svg>
							</div>
						{/if}
						<div class="hud-info">
							<div class="hud-row">
								<span class="hud-name">{ach.name || ach.key}</span>
								<div class="hud-badges">
									{#if isPinned}
										<span
											class="hud-pin-badge"
											title={$i18n.t('hud.pinned')}
											aria-label={$i18n.t('hud.pinned')}
										>
											<svg
												width="10"
												height="10"
												viewBox="0 0 24 24"
												fill="currentColor"
												stroke="currentColor"
												stroke-width="2"
												stroke-linecap="round"
												stroke-linejoin="round"
												aria-hidden="true"
											>
												<line x1="12" y1="17" x2="12" y2="22" />
												<path
													d="M5 17h14v-1.76a2 2 0 0 0-1.11-1.79l-1.78-.9A2 2 0 0 1 15 10.76V6h1a2 2 0 0 0 0-4H8a2 2 0 0 0 0 4h1v4.76a2 2 0 0 1-1.11 1.79l-1.78.9A2 2 0 0 0 5 15.24Z"
												/>
											</svg>
										</span>
									{/if}
									{#if rarityPct !== null && rarityPct < 100 && palette}
										<span
											class="hud-rarity-pill tabular-nums"
											style:color={palette.color}
											style:background={palette.bg}
											style:border-color={palette.border}
										>
											{rarityPct.toFixed(1)}%
										</span>
									{/if}
									{#if unlocked}
										<span
											class="hud-tag done"
											title={$i18n.t('hud.completed')}
											aria-label={$i18n.t('hud.completed')}
										>
											<svg
												width="10"
												height="10"
												viewBox="0 0 24 24"
												fill="none"
												stroke="currentColor"
												stroke-width="3"
												stroke-linecap="round"
												stroke-linejoin="round"
												aria-hidden="true"
											>
												<polyline points="20 6 9 17 4 12" />
											</svg>
										</span>
									{:else if hasProg}
										<span class="hud-tag tabular-nums" title={$i18n.t('hud.inProgress')}>
											{fmt(cur)}/{fmt(max)}
										</span>
									{/if}
								</div>
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
	.hud-tracker {
		font-family:
			'Inter',
			-apple-system,
			BlinkMacSystemFont,
			'SF Pro Display',
			'Segoe UI',
			system-ui,
			sans-serif;
		-webkit-font-smoothing: antialiased;
		width: 340px;
		max-width: calc(100% - 16px);
		margin: 8px;
		background: linear-gradient(165deg, rgba(18, 22, 32, 0.88) 0%, rgba(10, 13, 20, 0.94) 100%);
		backdrop-filter: blur(24px) saturate(180%);
		-webkit-backdrop-filter: blur(24px) saturate(180%);
		border: 1px solid rgba(255, 255, 255, 0.12);
		border-radius: 18px;
		padding: 12px 14px;
		color: #f5f5f7;
		box-shadow:
			inset 0 1px 0 rgba(255, 255, 255, 0.14),
			0 20px 48px rgba(0, 0, 0, 0.56);
		pointer-events: none;
		animation: hudSpringIn 0.32s cubic-bezier(0.22, 1, 0.36, 1) forwards;
	}

	:global([data-theme='light']) .hud-tracker {
		background: linear-gradient(
			165deg,
			rgba(255, 255, 255, 0.88) 0%,
			rgba(241, 245, 249, 0.94) 100%
		);
		border: 1px solid rgba(15, 23, 42, 0.12);
		color: #101828;
		box-shadow:
			inset 0 1px 0 rgba(255, 255, 255, 0.9),
			0 18px 40px rgba(15, 23, 42, 0.18);
	}

	@keyframes hudSpringIn {
		0% {
			opacity: 0;
			transform: translateY(-8px) scale(0.96);
		}
		100% {
			opacity: 1;
			transform: translateY(0) scale(1);
		}
	}

	.tabular-nums {
		font-variant-numeric: tabular-nums;
	}

	.hud-header {
		display: flex;
		align-items: center;
		gap: 9px;
		padding-bottom: 10px;
		border-bottom: 1px solid rgba(255, 255, 255, 0.08);
	}

	:global([data-theme='light']) .hud-header {
		border-bottom-color: rgba(15, 23, 42, 0.08);
	}

	.hud-dot {
		width: 8px;
		height: 8px;
		border-radius: 50%;
		background: var(--accent, #c8a96e);
		flex-shrink: 0;
	}

	.hud-title-group {
		flex: 1;
		min-width: 0;
		display: flex;
		flex-direction: column;
		gap: 1px;
	}

	.hud-subtitle {
		font-size: 9.5px;
		font-weight: 700;
		letter-spacing: 0.08em;
		text-transform: uppercase;
		color: var(--accent, #c8a96e);
	}

	.hud-title {
		font-size: 13px;
		font-weight: 700;
		letter-spacing: -0.01em;
		color: inherit;
		white-space: nowrap;
		overflow: hidden;
		text-overflow: ellipsis;
	}

	.hud-count {
		background: color-mix(in srgb, var(--accent, #c8a96e) 18%, transparent);
		border: 1px solid color-mix(in srgb, var(--accent, #c8a96e) 35%, transparent);
		color: var(--accent, #c8a96e);
		padding: 3px 8px;
		border-radius: 999px;
		font-size: 11px;
		font-weight: 700;
		flex-shrink: 0;
	}

	.hud-progress-section {
		display: flex;
		align-items: center;
		gap: 10px;
		margin: 10px 0 12px;
	}

	.hud-progress-track {
		flex: 1;
		height: 6px;
		border-radius: 999px;
		background: rgba(255, 255, 255, 0.1);
		overflow: hidden;
		box-shadow: inset 0 1px 2px rgba(0, 0, 0, 0.25);
	}

	:global([data-theme='light']) .hud-progress-track {
		background: rgba(15, 23, 42, 0.1);
		box-shadow: inset 0 1px 2px rgba(15, 23, 42, 0.08);
	}

	.hud-progress-fill {
		height: 100%;
		border-radius: 999px;
		background: linear-gradient(
			90deg,
			var(--accent, #c8a96e),
			color-mix(in srgb, var(--accent, #c8a96e) 70%, #fff)
		);
		transition: width 0.4s cubic-bezier(0.22, 1, 0.36, 1);
	}

	.hud-progress-pct {
		font-size: 11px;
		font-weight: 700;
		color: var(--accent, #c8a96e);
		min-width: 34px;
		text-align: right;
	}

	.hud-empty {
		font-size: 12px;
		color: rgba(255, 255, 255, 0.6);
		padding: 6px 2px;
	}

	:global([data-theme='light']) .hud-empty {
		color: #64748b;
	}

	.hud-list {
		display: flex;
		flex-direction: column;
		gap: 7px;
	}

	.ach-row {
		display: flex;
		align-items: center;
		gap: 10px;
		padding: 7px 9px;
		border-radius: 12px;
		background: rgba(255, 255, 255, 0.04);
		border: 1px solid rgba(255, 255, 255, 0.06);
	}

	:global([data-theme='light']) .ach-row {
		background: rgba(15, 23, 42, 0.04);
		border-color: rgba(15, 23, 42, 0.06);
	}

	.ach-row.done {
		opacity: 0.62;
	}

	.hud-icon {
		width: 34px;
		height: 34px;
		border-radius: 10px;
		object-fit: cover;
		background: rgba(255, 255, 255, 0.08);
		color: var(--accent, #c8a96e);
		flex-shrink: 0;
		display: flex;
		align-items: center;
		justify-content: center;
		border: 1px solid rgba(255, 255, 255, 0.1);
	}

	:global([data-theme='light']) .hud-icon {
		background: rgba(15, 23, 42, 0.06);
		border-color: rgba(15, 23, 42, 0.08);
	}

	.hud-info {
		flex: 1;
		min-width: 0;
		display: flex;
		flex-direction: column;
		gap: 3px;
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

	.hud-badges {
		display: flex;
		align-items: center;
		gap: 4px;
		flex-shrink: 0;
	}

	.hud-pin-badge {
		display: inline-flex;
		align-items: center;
		justify-content: center;
		width: 18px;
		height: 18px;
		border-radius: 6px;
		color: var(--accent, #c8a96e);
		background: color-mix(in srgb, var(--accent, #c8a96e) 16%, transparent);
		border: 1px solid color-mix(in srgb, var(--accent, #c8a96e) 30%, transparent);
	}

	.hud-rarity-pill {
		font-size: 10px;
		font-weight: 700;
		padding: 1px 6px;
		border-radius: 999px;
		border: 1px solid transparent;
		line-height: 1.4;
	}

	.hud-tag {
		font-size: 10px;
		font-weight: 700;
		color: var(--accent, #c8a96e);
		background: color-mix(in srgb, var(--accent, #c8a96e) 16%, transparent);
		padding: 1px 6px;
		border-radius: 6px;
		flex-shrink: 0;
	}

	.hud-tag.done {
		color: #3ddc84;
		background: rgba(61, 220, 132, 0.16);
		display: inline-flex;
		align-items: center;
		justify-content: center;
		padding: 3px 5px;
	}

	.hud-desc {
		font-size: 10.5px;
		color: rgba(255, 255, 255, 0.65);
		white-space: nowrap;
		overflow: hidden;
		text-overflow: ellipsis;
	}

	:global([data-theme='light']) .hud-desc {
		color: #475569;
	}

	.hud-bar {
		height: 3px;
		background: rgba(255, 255, 255, 0.1);
		border-radius: 999px;
		overflow: hidden;
		margin-top: 2px;
	}

	:global([data-theme='light']) .hud-bar {
		background: rgba(15, 23, 42, 0.1);
	}

	.hud-bar-fill {
		height: 100%;
		background: linear-gradient(
			90deg,
			var(--accent, #c8a96e),
			color-mix(in srgb, var(--accent, #c8a96e) 70%, #fff)
		);
		border-radius: 999px;
	}
</style>
