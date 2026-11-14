<script lang="ts">
	// src/lib/AchievementNotif.svelte
	import { i18n } from '$lib/stores/i18n.js';
	import type { NotifAchievement, NotifStyle } from '$lib/stores/settings.js';

	type Position =
		| 'top-left'
		| 'top-center'
		| 'top-right'
		| 'bottom-left'
		| 'bottom-center'
		| 'bottom-right';

	interface Props {
		achievement: NotifAchievement | null;
		visible: boolean;
		position?: Position;
		notifStyle?: NotifStyle;
		preview?: boolean;
	}

	let {
		achievement,
		visible,
		position = 'top-right',
		notifStyle = 'accolade',
		preview = false
	}: Props = $props();

	const PLATINUM_COLOR = '#ffd700';
	const PLATINUM_GRADIENT = 'linear-gradient(90deg, #b8860b, #ffd700, #daa520, #ffd700, #b8860b)';
	const DEFAULT_COLOR = '#6a7080';

	const isProgress = $derived(Boolean(achievement?.is_progress));
	const isPlatinum = $derived(Boolean(achievement?.is_platinum));
	const progressCurrent = $derived(achievement?.progress_current ?? 0);
	const progressMax = $derived(achievement?.progress_max ?? 0);

	const milestonePct = $derived(
		achievement?.progress_milestone ??
			(progressMax > 0 ? Math.min(100, Math.round((progressCurrent / progressMax) * 100)) : 0)
	);

	const barPct = $derived(
		progressMax > 0
			? Math.max(0, Math.min(100, (progressCurrent / progressMax) * 100))
			: Math.max(0, Math.min(100, milestonePct))
	);

	const rarityColor = $derived(() => {
		if (isPlatinum) return PLATINUM_COLOR;
		if (achievement?.completionpercentage == null) return DEFAULT_COLOR;

		const n = achievement.completionpercentage;
		if (n <= 0.1) return '#ff3b5c';
		if (n <= 1.0) return '#ffd85a';
		if (n <= 3.0) return '#a855f7';
		if (n <= 7.0) return '#f4b860';
		if (n <= 15.0) return '#4ac8ff';
		if (n <= 35.0) return '#3ddc84';
		return DEFAULT_COLOR;
	});

	const ringDash = $derived(
		isProgress && achievement?.progress_current != null && achievement?.progress_max
			? (achievement.progress_current / achievement.progress_max) * 94.25
			: achievement?.unlocked != null && achievement?.total
				? (achievement.unlocked / achievement.total) * 94.25
				: 0
	);

	function fmtProg(n: number | undefined): string {
		if (n == null) return '0';
		return Number.isInteger(n) ? String(n) : n.toFixed(1);
	}

	const headerLabel = $derived(
		isPlatinum
			? $i18n.t('notif.platinum')
			: isProgress
				? $i18n.t('notif.progress')
				: $i18n.t('notif.unlocked')
	);

	const formattedDate = $derived(() => {
		if (!achievement?.unlocked_time) return null;
		const d = new Date(achievement.unlocked_time * 1000);
		return (
			d.toLocaleDateString($i18n.locale, { day: '2-digit', month: '2-digit', year: 'numeric' }) +
			' · ' +
			d.toLocaleTimeString($i18n.locale, { hour: '2-digit', minute: '2-digit' })
		);
	});

	const animationClass = $derived(() => {
		if (preview) return '';
		switch (position) {
			case 'top-left':
			case 'bottom-left':
				return 'slide-right';
			case 'top-right':
			case 'bottom-right':
				return 'slide-left';
			case 'top-center':
				return 'slide-down';
			case 'bottom-center':
				return 'slide-up';
			default:
				return 'slide-left';
		}
	});
</script>

{#if achievement}
	<div
		class="notif-wrapper"
		class:visible
		class:is-progress={isProgress}
		class:is-platinum={isPlatinum}
		class:is-preview={preview}
		data-skin={notifStyle}
	>
		{#if isProgress}
			{#key achievement}
				<div class="card progress-card {animationClass()}">
					<div class="progress-head">
						<div class="progress-icon-wrap">
							{#if notifStyle === 'xbox360'}
								<div class="xbox-orb xbox-orb-sm" aria-hidden="true">
									<svg width="16" height="16" viewBox="0 0 24 24" fill="none">
										<path
											d="M7 5h10v4a5 5 0 0 1-10 0V5zm5 9v5m-3 0h6"
											stroke="currentColor"
											stroke-width="2"
											stroke-linecap="round"
											stroke-linejoin="round"
										/>
									</svg>
								</div>
							{:else if achievement.icon}
								<img src={achievement.icon} alt={$i18n.t('screenshots.achievementIconAlt')} class="progress-icon" />
							{:else}
								<div class="progress-icon-fallback" aria-hidden="true">
									<svg width="16" height="16" viewBox="0 0 24 24" fill="none">
										<path
											d="M6 9H4.5a2.5 2.5 0 0 1 0-5H6m12 5h1.5a2.5 2.5 0 0 0 0-5H18M4 22h16M10 14.66V17c0 .55-.47.98-.97 1.21C7.85 18.75 7 20.24 7 22m7-7.34V17c0 .55.47.98.97 1.21C16.15 18.75 17 20.24 17 22M18 2H6v7a6 6 0 0 0 12 0V2Z"
											stroke="currentColor"
											stroke-width="1.8"
											stroke-linecap="round"
											stroke-linejoin="round"
										/>
									</svg>
								</div>
							{/if}
						</div>

						<div class="progress-title-col">
							<div class="progress-kicker">{$i18n.t('notif.progress')}</div>
							<div class="progress-title">{achievement.name}</div>
						</div>

						<div class="progress-metrics">
							<span class="progress-counter">
								{fmtProg(progressCurrent)}/{fmtProg(progressMax)}
							</span>
							<span class="milestone-badge">{milestonePct}%</span>
						</div>
					</div>

					<div class="progress-track" role="progressbar" aria-valuenow={milestonePct} aria-valuemin={0} aria-valuemax={100}>
						<div class="progress-bar" style="--bar-width: {barPct}%;"></div>
					</div>
				</div>
			{/key}
		{:else}
			{#if notifStyle === 'accolade'}
				<div class="badge-wrap">
					<span
						class="badge"
						class:badge-platinum={isPlatinum}
						style="background: {isPlatinum ? PLATINUM_GRADIENT : rarityColor()};"
					>
						{headerLabel}
					</span>
				</div>
			{/if}

			{#key achievement}
				<div class="card {animationClass()}" class:card-platinum={isPlatinum}>
					<div class="body">
						<div class="icon-wrap">
							{#if notifStyle === 'xbox360'}
								<div class="xbox-orb" class:xbox-orb-platinum={isPlatinum} aria-hidden="true">
									<svg width="26" height="26" viewBox="0 0 24 24" fill="none">
										<path
											d="M6 9H4.5a2.5 2.5 0 0 1 0-5H6m12 5h1.5a2.5 2.5 0 0 0 0-5H18M4 22h16M10 14.66V17c0 .55-.47.98-.97 1.21C7.85 18.75 7 20.24 7 22m7-7.34V17c0 .55.47.98.97 1.21C16.15 18.75 17 20.24 17 22M18 2H6v7a6 6 0 0 0 12 0V2Z"
											stroke="currentColor"
											stroke-width="2"
											stroke-linecap="round"
											stroke-linejoin="round"
										/>
									</svg>
								</div>
							{:else if achievement.icon}
								<img src={achievement.icon} alt={$i18n.t('screenshots.achievementIconAlt')} class="icon" />
							{:else}
								<div class="icon-placeholder" aria-hidden="true">
									<svg width="26" height="26" viewBox="0 0 24 24" fill="none">
										<path
											d="M6 9H4.5a2.5 2.5 0 0 1 0-5H6m12 5h1.5a2.5 2.5 0 0 0 0-5H18M4 22h16M10 14.66V17c0 .55-.47.98-.97 1.21C7.85 18.75 7 20.24 7 22m7-7.34V17c0 .55.47.98.97 1.21C16.15 18.75 17 20.24 17 22M18 2H6v7a6 6 0 0 0 12 0V2Z"
											stroke="currentColor"
											stroke-width="1.6"
											stroke-linecap="round"
											stroke-linejoin="round"
										/>
									</svg>
								</div>
							{/if}
						</div>

						<div class="info">
							{#if notifStyle !== 'accolade'}
								<div class="skin-kicker" class:skin-kicker-platinum={isPlatinum}>
									{headerLabel}
								</div>
							{/if}
							<div
								class="name"
								style={notifStyle === 'accolade' ? `color: ${rarityColor()};` : undefined}
							>
								{achievement.name}
							</div>
							{#if achievement.desc}
								<div class="desc">{achievement.desc}</div>
							{/if}
							<div class="meta">
								{#if achievement.rarity}
									<span
										class="rarity"
										style={notifStyle === 'accolade' ? `color: ${rarityColor()};` : undefined}
									>
										{achievement.rarity}
									</span>
								{/if}
								{#if achievement.completionpercentage != null}
									<span class="pct">— {achievement.completionpercentage}%</span>
								{/if}
								{#if formattedDate() && notifStyle === 'accolade'}
									<span class="date">· {formattedDate()}</span>
								{/if}
							</div>
						</div>

						{#if notifStyle === 'ps5'}
							<div class="ps5-trophy" class:ps5-trophy-platinum={isPlatinum} aria-hidden="true">
								<svg width="24" height="24" viewBox="0 0 24 24" fill="currentColor">
									<path
										d="M19 4h-2V3a1 1 0 0 0-1-1H8a1 1 0 0 0-1 1v1H5a1 1 0 0 0-1 1v2a4 4 0 0 0 3.23 3.93A5.02 5.02 0 0 0 11 13.9V17H8a1 1 0 0 0-1 1v2H6a1 1 0 0 0 0 2h12a1 1 0 0 0 0-2h-1v-2a1 1 0 0 0-1-1h-3v-3.1a5.02 5.02 0 0 0 3.77-2.97A4 4 0 0 0 20 7V5a1 1 0 0 0-1-1ZM6 7V6h1v2.83A2 2 0 0 1 6 7Zm12 0a2 2 0 0 1-1 1.83V6h1v1Z"
									/>
								</svg>
							</div>
						{:else if notifStyle === 'retro'}
							<div class="retro-emblem" class:retro-emblem-platinum={isPlatinum} aria-hidden="true">
								{#if achievement.unlocked != null && achievement.total != null}
									<span class="retro-count">{achievement.unlocked}/{achievement.total}</span>
								{:else}
									<span>★</span>
								{/if}
							</div>
						{:else if achievement.unlocked != null && achievement.total != null}
							{#if notifStyle === 'accolade'}
								<div class="progress-wrap">
									<div class="fraction">{achievement.unlocked}/{achievement.total}</div>
									<svg class="ring" viewBox="0 0 36 36">
										<circle class="ring-bg" cx="18" cy="18" r="15" />
										<circle
											class="ring-fill"
											cx="18"
											cy="18"
											r="15"
											stroke-dasharray={ringDash + ' 94.25'}
											style="stroke: {rarityColor()};"
										/>
									</svg>
								</div>
							{:else}
								<div class="skin-counter" class:skin-counter-platinum={isPlatinum}>
									{achievement.unlocked}/{achievement.total}
								</div>
							{/if}
						{/if}
					</div>
				</div>
			{/key}
		{/if}
	</div>
{/if}

<style>
	.notif-wrapper {
		font-family:
			'Inter',
			-apple-system,
			BlinkMacSystemFont,
			'SF Pro Display',
			system-ui,
			sans-serif;
		-webkit-font-smoothing: antialiased;
		width: 100%;
		max-width: 420px;
		display: flex;
		flex-direction: column;
		align-items: center;
		padding: 20px 18px;
		box-sizing: border-box;
		opacity: 0;
		pointer-events: none;
		transition: opacity 0.45s cubic-bezier(0.4, 0, 0.2, 1);
		overflow: visible;
	}

	.notif-wrapper.visible {
		opacity: 1;
		pointer-events: auto;
	}

	.notif-wrapper.is-preview {
		padding: 10px 8px;
		pointer-events: none;
	}

	.notif-wrapper.is-progress {
		max-width: 400px;
		padding: 14px 16px;
	}

	.notif-wrapper.is-progress.is-preview {
		padding: 6px 8px;
	}

	/* =========================================================================
	   Default / Accolade Skin (Normal & Platinum)
	   ========================================================================= */
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
		font-size: 18px;
		font-weight: 700;
		letter-spacing: 0.16em;
		color: #fff;
		border-radius: 16px;
		padding: 7px 24px;
		box-shadow: 0 4px 12px rgba(0, 0, 0, 0.35);
	}

	.badge-platinum {
		background-size: 200% 100% !important;
		animation: badge-shimmer 1.5s ease-in-out infinite;
	}

	@keyframes badge-shimmer {
		0% {
			background-position: -100% 0;
		}
		50% {
			background-position: 100% 0;
		}
		100% {
			background-position: -100% 0;
		}
	}

	.card {
		width: 100%;
		background: #1c1e22;
		border-radius: 16px;
		box-shadow:
			0 0 0 1px rgba(255, 255, 255, 0.06),
			0 8px 28px rgba(0, 0, 0, 0.6);
		overflow: hidden;
		position: relative;
		box-sizing: border-box;
	}

	.card-platinum {
		box-shadow:
			0 0 0 1px rgba(255, 215, 0, 0.38),
			0 8px 28px rgba(0, 0, 0, 0.6);
	}

	.body {
		display: flex;
		align-items: center;
		gap: 13px;
		padding: 22px 15px 15px;
	}

	.icon-wrap {
		flex-shrink: 0;
		display: flex;
		align-items: center;
		justify-content: center;
	}

	.icon,
	.icon-placeholder {
		width: 68px;
		height: 68px;
		border-radius: 10px;
		background: #2a2d33;
		object-fit: cover;
		flex-shrink: 0;
	}

	.icon-placeholder {
		display: flex;
		align-items: center;
		justify-content: center;
		color: rgba(255, 255, 255, 0.25);
	}

	.info {
		flex: 1;
		min-width: 0;
		display: flex;
		flex-direction: column;
		gap: 3px;
	}

	.skin-kicker {
		font-size: 10px;
		font-weight: 700;
		letter-spacing: 0.12em;
		text-transform: uppercase;
		color: rgba(255, 255, 255, 0.58);
		white-space: nowrap;
		overflow: hidden;
		text-overflow: ellipsis;
	}

	.skin-kicker-platinum {
		color: #ffd700;
	}

	.name {
		font-size: 17px;
		font-weight: 700;
		letter-spacing: 0.03em;
		color: #f4f4f6;
		white-space: nowrap;
		overflow: hidden;
		text-overflow: ellipsis;
	}

	.desc {
		font-size: 12px;
		font-weight: 500;
		color: #d6d9df;
		white-space: nowrap;
		overflow: hidden;
		text-overflow: ellipsis;
	}

	.meta {
		display: flex;
		align-items: center;
		gap: 4px;
		margin-top: 4px;
		flex-wrap: wrap;
	}

	.rarity {
		font-size: 12px;
		font-weight: 700;
		letter-spacing: 0.06em;
		color: #9ca3af;
		font-variant-numeric: tabular-nums;
	}

	.pct {
		font-size: 12px;
		font-weight: 600;
		color: #9ca3af;
		font-variant-numeric: tabular-nums;
	}

	.date {
		font-size: 11px;
		color: #6b7280;
		font-variant-numeric: tabular-nums;
	}

	.progress-wrap {
		flex-shrink: 0;
		position: relative;
		width: 50px;
		height: 50px;
	}

	.fraction {
		font-size: 13px;
		font-weight: 700;
		color: #fff;
		font-variant-numeric: tabular-nums;
		position: absolute;
		inset: 0;
		display: flex;
		align-items: center;
		justify-content: center;
	}

	.ring {
		width: 50px;
		height: 50px;
		transform: rotate(-90deg);
	}

	.ring-bg {
		fill: none;
		stroke: #2a2d33;
		stroke-width: 3.5;
	}

	.ring-fill {
		fill: none;
		stroke-width: 3.5;
		stroke-linecap: round;
	}

	/* =========================================================================
	   Discreet / Compact Progress Toast Base (Accolade + shared layout)
	   ========================================================================= */
	.progress-card {
		padding: 11px 14px 12px;
		display: flex;
		flex-direction: column;
		gap: 9px;
		background: rgba(24, 26, 32, 0.94);
		border: 1px solid rgba(255, 255, 255, 0.08);
		box-shadow: 0 8px 24px rgba(0, 0, 0, 0.52);
	}

	.progress-head {
		display: flex;
		align-items: center;
		gap: 10px;
		min-width: 0;
	}

	.progress-icon-wrap {
		flex-shrink: 0;
		display: flex;
		align-items: center;
		justify-content: center;
	}

	.progress-icon,
	.progress-icon-fallback {
		width: 34px;
		height: 34px;
		border-radius: 8px;
		object-fit: cover;
		background: #272a33;
		color: #9aa2b2;
		display: flex;
		align-items: center;
		justify-content: center;
	}

	.progress-title-col {
		flex: 1;
		min-width: 0;
		display: flex;
		flex-direction: column;
		gap: 1px;
	}

	.progress-kicker {
		font-size: 9px;
		font-weight: 700;
		letter-spacing: 0.12em;
		text-transform: uppercase;
		color: rgba(255, 255, 255, 0.48);
	}

	.progress-title {
		font-size: 13px;
		font-weight: 700;
		color: #f3f4f6;
		white-space: nowrap;
		overflow: hidden;
		text-overflow: ellipsis;
	}

	.progress-metrics {
		display: flex;
		align-items: center;
		gap: 7px;
		flex-shrink: 0;
	}

	.progress-counter {
		font-size: 12px;
		font-weight: 600;
		color: #aeb4c0;
		font-variant-numeric: tabular-nums;
	}

	.milestone-badge {
		display: inline-flex;
		align-items: center;
		justify-content: center;
		padding: 2px 7px;
		border-radius: 999px;
		font-size: 11px;
		font-weight: 800;
		font-variant-numeric: tabular-nums;
		color: #111318;
		background: var(--accent, #c8a96e);
	}

	.progress-track {
		width: 100%;
		height: 5px;
		border-radius: 999px;
		background: rgba(255, 255, 255, 0.1);
		overflow: hidden;
	}

	.progress-bar {
		height: 100%;
		border-radius: inherit;
		width: var(--bar-width, 0%);
		background: linear-gradient(90deg, var(--accent, #c8a96e), #f3d28e);
		animation: progressGrow 0.65s cubic-bezier(0.22, 1, 0.36, 1) both;
	}

	@keyframes progressGrow {
		from {
			width: 0%;
		}
		to {
			width: var(--bar-width, 0%);
		}
	}

	/* =========================================================================
	   Skin: PS5
	   ========================================================================= */
	.notif-wrapper[data-skin='ps5'] .card {
		background: linear-gradient(135deg, rgba(22, 27, 40, 0.96), rgba(14, 17, 26, 0.96));
		border-radius: 22px;
		border: 1px solid rgba(173, 206, 255, 0.22);
		box-shadow:
			inset 0 1px 0 rgba(255, 255, 255, 0.16),
			0 10px 28px rgba(0, 0, 0, 0.58);
	}

	.notif-wrapper[data-skin='ps5'] .card-platinum {
		border-color: rgba(147, 197, 253, 0.65);
		box-shadow:
			inset 0 1px 0 rgba(224, 242, 254, 0.35),
			0 0 22px rgba(96, 165, 250, 0.28),
			0 10px 28px rgba(0, 0, 0, 0.62);
	}

	.notif-wrapper[data-skin='ps5'] .body {
		padding: 13px 16px;
		gap: 12px;
	}

	.notif-wrapper[data-skin='ps5'] .icon,
	.notif-wrapper[data-skin='ps5'] .icon-placeholder {
		width: 54px;
		height: 54px;
		border-radius: 12px;
		background: rgba(255, 255, 255, 0.06);
		border: 1px solid rgba(255, 255, 255, 0.12);
	}

	.notif-wrapper[data-skin='ps5'] .skin-kicker {
		color: #93c5fd;
		letter-spacing: 0.09em;
	}

	.notif-wrapper[data-skin='ps5'] .skin-kicker-platinum {
		color: #e0f2fe;
	}

	.notif-wrapper[data-skin='ps5'] .name {
		font-size: 15px;
		color: #f8fafc;
	}

	.notif-wrapper[data-skin='ps5'] .desc {
		font-size: 11.5px;
		color: #cbd5e1;
	}

	.ps5-trophy {
		width: 40px;
		height: 40px;
		border-radius: 50%;
		flex-shrink: 0;
		display: flex;
		align-items: center;
		justify-content: center;
		background: rgba(148, 163, 184, 0.14);
		border: 1px solid rgba(203, 213, 225, 0.28);
		color: #e2e8f0;
	}

	.ps5-trophy-platinum {
		background: radial-gradient(circle at 35% 30%, #e0f2fe, #60a5fa 65%, #1d4ed8);
		border-color: rgba(255, 255, 255, 0.6);
		color: #ffffff;
		box-shadow: 0 0 14px rgba(96, 165, 250, 0.45);
	}

	.notif-wrapper[data-skin='ps5'] .progress-card {
		border-radius: 20px;
		padding: 10px 14px 11px;
	}

	.notif-wrapper[data-skin='ps5'] .milestone-badge {
		background: rgba(96, 165, 250, 0.22);
		color: #bfdbfe;
		border: 1px solid rgba(147, 197, 253, 0.35);
	}

	.notif-wrapper[data-skin='ps5'] .progress-bar {
		background: linear-gradient(90deg, #3b82f6, #93c5fd);
	}

	/* =========================================================================
	   Skin: Xbox 360
	   ========================================================================= */
	.notif-wrapper[data-skin='xbox360'] .card {
		border-radius: 999px;
		background: linear-gradient(180deg, #3d4349 0%, #272b30 48%, #1a1d21 100%);
		border: 1.5px solid rgba(255, 255, 255, 0.22);
		box-shadow:
			inset 0 1px 0 rgba(255, 255, 255, 0.28),
			0 8px 24px rgba(0, 0, 0, 0.62);
	}

	.notif-wrapper[data-skin='xbox360'] .card-platinum {
		border-color: rgba(250, 204, 21, 0.72);
		box-shadow:
			inset 0 1px 0 rgba(254, 240, 138, 0.38),
			0 0 18px rgba(234, 179, 8, 0.32),
			0 8px 24px rgba(0, 0, 0, 0.62);
	}

	.notif-wrapper[data-skin='xbox360'] .body {
		padding: 10px 18px 10px 12px;
		gap: 12px;
	}

	.xbox-orb {
		width: 48px;
		height: 48px;
		border-radius: 50%;
		background: radial-gradient(circle at 32% 28%, #86efac 0%, #22c55e 42%, #107c10 82%, #064e06 100%);
		border: 2px solid rgba(255, 255, 255, 0.55);
		box-shadow:
			0 0 14px rgba(34, 197, 94, 0.55),
			inset 0 2px 4px rgba(255, 255, 255, 0.45);
		color: #ffffff;
		display: flex;
		align-items: center;
		justify-content: center;
	}

	.xbox-orb-platinum {
		background: radial-gradient(circle at 32% 28%, #fef9c3 0%, #facc15 45%, #ca8a04 82%, #854d0e 100%);
		box-shadow:
			0 0 16px rgba(250, 204, 21, 0.65),
			inset 0 2px 4px rgba(255, 255, 255, 0.55);
	}

	.xbox-orb-sm {
		width: 32px;
		height: 32px;
		border-width: 1.5px;
	}

	.notif-wrapper[data-skin='xbox360'] .skin-kicker {
		color: #4ade80;
		font-weight: 800;
	}

	.notif-wrapper[data-skin='xbox360'] .skin-kicker-platinum {
		color: #fde047;
	}

	.notif-wrapper[data-skin='xbox360'] .name {
		font-size: 15px;
		color: #ffffff;
	}

	.notif-wrapper[data-skin='xbox360'] .desc {
		font-size: 11px;
		color: #d1d5db;
	}

	.notif-wrapper[data-skin='xbox360'] .progress-card {
		border-radius: 999px;
		padding: 9px 18px 10px 12px;
		gap: 6px;
	}

	.notif-wrapper[data-skin='xbox360'] .progress-track {
		margin-left: 42px;
		width: calc(100% - 42px);
		height: 4px;
		background: rgba(0, 0, 0, 0.45);
	}

	.notif-wrapper[data-skin='xbox360'] .milestone-badge {
		background: #107c10;
		color: #ecfdf5;
		border: 1px solid #4ade80;
	}

	.notif-wrapper[data-skin='xbox360'] .progress-bar {
		background: linear-gradient(90deg, #107c10, #4ade80);
	}

	/* =========================================================================
	   Skin: Steam Deck
	   ========================================================================= */
	.notif-wrapper[data-skin='steamdeck'] .card {
		border-radius: 4px;
		background: #1a1d24;
		border: 1px solid rgba(255, 255, 255, 0.08);
		border-bottom: 3px solid #1a9fff;
		box-shadow: 0 10px 26px rgba(0, 0, 0, 0.65);
	}

	.notif-wrapper[data-skin='steamdeck'] .card-platinum {
		border-bottom-color: #ffd700;
		box-shadow:
			0 0 0 1px rgba(255, 215, 0, 0.25),
			0 10px 26px rgba(0, 0, 0, 0.65);
	}

	.notif-wrapper[data-skin='steamdeck'] .body {
		padding: 12px 14px;
		gap: 12px;
	}

	.notif-wrapper[data-skin='steamdeck'] .icon,
	.notif-wrapper[data-skin='steamdeck'] .icon-placeholder {
		width: 52px;
		height: 52px;
		border-radius: 3px;
		background: #232833;
	}

	.notif-wrapper[data-skin='steamdeck'] .skin-kicker {
		color: #1a9fff;
		letter-spacing: 0.08em;
	}

	.notif-wrapper[data-skin='steamdeck'] .skin-kicker-platinum {
		color: #ffd700;
	}

	.notif-wrapper[data-skin='steamdeck'] .name {
		font-size: 15px;
		font-weight: 700;
		color: #f1f5f9;
	}

	.notif-wrapper[data-skin='steamdeck'] .desc {
		font-size: 11.5px;
		color: #94a3b8;
	}

	.notif-wrapper[data-skin='steamdeck'] .progress-card {
		border-radius: 4px;
		border-bottom: 1px solid rgba(255, 255, 255, 0.08);
		padding: 10px 13px;
	}

	.notif-wrapper[data-skin='steamdeck'] .progress-icon,
	.notif-wrapper[data-skin='steamdeck'] .progress-icon-fallback {
		border-radius: 3px;
	}

	.notif-wrapper[data-skin='steamdeck'] .milestone-badge {
		border-radius: 3px;
		background: #1a9fff;
		color: #0b1220;
	}

	.notif-wrapper[data-skin='steamdeck'] .progress-track {
		border-radius: 2px;
		height: 4px;
		background: #282e3b;
	}

	.notif-wrapper[data-skin='steamdeck'] .progress-bar {
		background: #1a9fff;
	}

	/* =========================================================================
	   Skin: Retro 8-bit
	   ========================================================================= */
	.notif-wrapper[data-skin='retro'] {
		font-family: 'Cascadia Mono', 'JetBrains Mono', 'Consolas', monospace;
	}

	.notif-wrapper[data-skin='retro'] .card {
		border-radius: 0;
		background: #0a0b12;
		border: 2px solid #00f0ff;
		box-shadow:
			0 0 0 2px #0a0b12,
			0 0 0 4px #ffd700,
			4px 4px 0 4px rgba(0, 0, 0, 0.85);
	}

	.notif-wrapper[data-skin='retro'] .card-platinum {
		border-color: #ffd700;
		box-shadow:
			0 0 0 2px #0a0b12,
			0 0 0 4px #00f0ff,
			4px 4px 0 4px rgba(0, 0, 0, 0.85);
	}

	.notif-wrapper[data-skin='retro'] .body {
		padding: 12px 14px;
		gap: 12px;
	}

	.notif-wrapper[data-skin='retro'] .icon,
	.notif-wrapper[data-skin='retro'] .icon-placeholder {
		width: 50px;
		height: 50px;
		border-radius: 0;
		border: 2px solid #ffd700;
		background: #151726;
		color: #ffd700;
	}

	.notif-wrapper[data-skin='retro'] .skin-kicker {
		color: #00f0ff;
		font-size: 10px;
	}

	.notif-wrapper[data-skin='retro'] .skin-kicker-platinum {
		color: #ffd700;
	}

	.notif-wrapper[data-skin='retro'] .name {
		font-size: 14px;
		color: #ffd700;
		letter-spacing: 0.02em;
	}

	.notif-wrapper[data-skin='retro'] .desc {
		font-size: 11px;
		color: #e2e8f0;
	}

	.notif-wrapper[data-skin='retro'] .rarity,
	.notif-wrapper[data-skin='retro'] .pct {
		font-size: 11px;
		color: #00f0ff;
	}

	.retro-emblem {
		flex-shrink: 0;
		padding: 4px 7px;
		border: 2px solid #00f0ff;
		background: #14172b;
		color: #ffd700;
		font-size: 12px;
		font-weight: 800;
	}

	.retro-emblem-platinum {
		border-color: #ffd700;
		color: #00f0ff;
	}

	.notif-wrapper[data-skin='retro'] .progress-card {
		border-radius: 0;
		padding: 10px 12px;
	}

	.notif-wrapper[data-skin='retro'] .progress-icon,
	.notif-wrapper[data-skin='retro'] .progress-icon-fallback {
		border-radius: 0;
		border: 2px solid #ffd700;
		color: #ffd700;
	}

	.notif-wrapper[data-skin='retro'] .progress-kicker {
		color: #00f0ff;
	}

	.notif-wrapper[data-skin='retro'] .progress-title {
		font-size: 12.5px;
		color: #ffd700;
	}

	.notif-wrapper[data-skin='retro'] .milestone-badge {
		border-radius: 0;
		background: #ffd700;
		color: #0a0b12;
		border: 1px solid #00f0ff;
	}

	.notif-wrapper[data-skin='retro'] .progress-track {
		border-radius: 0;
		height: 7px;
		border: 1px solid #00f0ff;
		background: #14172b;
	}

	.notif-wrapper[data-skin='retro'] .progress-bar {
		border-radius: 0;
		background: repeating-linear-gradient(
			90deg,
			#00f0ff 0px,
			#00f0ff 8px,
			#ffd700 8px,
			#ffd700 12px
		);
	}

	/* =========================================================================
	   Shared Right Counter Pill (PS5 / Xbox 360 / Steam Deck)
	   ========================================================================= */
	.skin-counter {
		flex-shrink: 0;
		padding: 4px 9px;
		border-radius: 999px;
		font-size: 11.5px;
		font-weight: 700;
		font-variant-numeric: tabular-nums;
		color: #e2e8f0;
		background: rgba(255, 255, 255, 0.08);
		border: 1px solid rgba(255, 255, 255, 0.14);
	}

	.skin-counter-platinum {
		color: #fef08a;
		background: rgba(234, 179, 8, 0.18);
		border-color: rgba(250, 204, 21, 0.45);
	}

	.notif-wrapper[data-skin='steamdeck'] .skin-counter {
		border-radius: 3px;
		background: rgba(26, 159, 255, 0.14);
		border-color: rgba(26, 159, 255, 0.35);
		color: #bae6fd;
	}

	/* =========================================================================
	   Entrance Slide Animations
	   ========================================================================= */
	.slide-right {
		animation: slideRight 0.45s cubic-bezier(0.22, 1, 0.36, 1) forwards;
	}
	.slide-left {
		animation: slideLeft 0.45s cubic-bezier(0.22, 1, 0.36, 1) forwards;
	}
	.slide-down {
		animation: slideDown 0.45s cubic-bezier(0.22, 1, 0.36, 1) forwards;
	}
	.slide-up {
		animation: slideUp 0.45s cubic-bezier(0.22, 1, 0.36, 1) forwards;
	}

	@keyframes slideRight {
		from {
			transform: translateX(-24px);
			opacity: 0;
		}
		to {
			transform: translateX(0);
			opacity: 1;
		}
	}

	@keyframes slideLeft {
		from {
			transform: translateX(24px);
			opacity: 0;
		}
		to {
			transform: translateX(0);
			opacity: 1;
		}
	}

	@keyframes slideDown {
		from {
			transform: translateY(-24px);
			opacity: 0;
		}
		to {
			transform: translateY(0);
			opacity: 1;
		}
	}

	@keyframes slideUp {
		from {
			transform: translateY(24px);
			opacity: 0;
		}
		to {
			transform: translateY(0);
			opacity: 1;
		}
	}
</style>
