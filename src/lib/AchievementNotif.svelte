<script lang="ts">
	// src/lib/AchievementNotif.svelte
	import { i18n } from '$lib/stores/i18n.js';
	interface Achievement {
		name?: string;
		desc?: string;
		icon?: string;
		rarity?: string;
		is_platinum?: boolean;
		completionpercentage?: number;
		unlocked?: number;
		total?: number;
		unlocked_time?: number;
	}

	type Position =
		| 'top-left'
		| 'top-center'
		| 'top-right'
		| 'bottom-left'
		| 'bottom-center'
		| 'bottom-right';

	let {
		achievement,
		visible,
		position = 'top-right'
	} = $props<{
		achievement: Achievement | null;
		visible: boolean;
		position?: Position;
	}>();

	const PLATINUM_COLORS = { accent: '#ffd700', glow: 'rgba(255, 215, 0, 0.38)' };
	const PLATINUM_GRADIENT = 'linear-gradient(90deg, #b8860b, #ffd700, #daa520, #ffd700, #b8860b)';
	const DEFAULT_COLORS = { accent: '#6a7080', glow: 'rgba(106, 112, 128, 0.35)' };

	const rarityStyle = $derived(() => {
		if (achievement?.is_platinum) return PLATINUM_COLORS;
		if (achievement?.completionpercentage == null) return DEFAULT_COLORS;

		const n = achievement.completionpercentage;
		if (n <= 0.1) return { accent: '#ff3b5c', glow: 'rgba(255, 59, 92, 0.55)' };
		if (n <= 1.0) return { accent: '#ffd85a', glow: 'rgba(255, 216, 90, 0.55)' };
		if (n <= 3.0) return { accent: '#a855f7', glow: 'rgba(168, 85, 247, 0.55)' };
		if (n <= 7.0) return { accent: '#f4b860', glow: 'rgba(244, 184, 96, 0.55)' };
		if (n <= 15.0) return { accent: '#4ac8ff', glow: 'rgba(74, 200, 255, 0.55)' };
		if (n <= 35.0) return { accent: '#3ddc84', glow: 'rgba(61, 220, 132, 0.55)' };
		return DEFAULT_COLORS;
	});

	const ringDash = $derived(
		achievement?.unlocked != null && achievement?.total
			? (achievement.unlocked / achievement.total) * 94.25
			: 0
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
	<div class="notif-wrapper" class:visible>
		<div class="badge-wrap">
			<span
				class="badge"
				class:badge-platinum={achievement.is_platinum}
				style="background: {achievement.is_platinum
					? PLATINUM_GRADIENT
					: rarityStyle().accent}; box-shadow: 0 0 12px 2px {rarityStyle().glow};"
			>
				{achievement.is_platinum ? $i18n.t('notif.platinum') : $i18n.t('notif.unlocked')}
			</span>
		</div>
		{#key achievement}
			<div class="card {animationClass()}" class:card-platinum={achievement.is_platinum}>
				<div class="body">
					<div class="icon-wrap">
						{#if achievement.icon}
							<img src={achievement.icon} alt="Achievement icon" class="icon" />
						{:else}
							<div class="icon-placeholder"></div>
						{/if}
					</div>
					<div class="info">
						<div class="name" style="color: {rarityStyle().accent};">{achievement.name}</div>
						<div class="desc">{achievement.desc}</div>
						<div class="meta">
							<span class="rarity" style="color: {rarityStyle().accent};">{achievement.rarity}</span
							>
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
									cx="18"
									cy="18"
									r="15"
									stroke-dasharray={ringDash + ' 94.25'}
									style="stroke: {rarityStyle().accent};"
								/>
							</svg>
						</div>
					{/if}
				</div>
			</div>
		{/key}
	</div>
{/if}

<style>
	.notif-wrapper {
		font-family:
			'Segoe UI',
			system-ui,
			-apple-system,
			sans-serif;
		width: 100%;
		max-width: 460px;
		display: flex;
		flex-direction: column;
		align-items: center;
		padding: 28px 36px 26px;
		box-sizing: border-box;
		opacity: 0;
		pointer-events: none;
		transition: opacity 0.5s cubic-bezier(0.4, 0, 0.2, 1);
		overflow: visible;
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
		font-family: 'Segoe UI Semibold', 'Segoe UI', system-ui, sans-serif;
		font-size: 20px;
		font-weight: 700;
		letter-spacing: 0.18em;
		color: #fff;
		border-radius: 16px;
		padding: 8px 28px;
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
			0 8px 32px rgba(0, 0, 0, 0.6);
		overflow: hidden;
	}

	.card-platinum {
		box-shadow:
			0 0 0 1px rgba(255, 215, 0, 0.28),
			0 6px 18px rgba(255, 215, 0, 0.16),
			0 0 20px rgba(255, 215, 0, 0.05);
	}

	.body {
		display: flex;
		align-items: center;
		gap: 14px;
		padding: 22px 16px 16px;
	}

	.icon,
	.icon-placeholder {
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
		font-family: 'Segoe UI Semibold', 'Segoe UI', system-ui, sans-serif;
		font-size: 18px;
		font-weight: 700;
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
		font-family: 'Segoe UI Semibold', 'Segoe UI', system-ui, sans-serif;
		font-size: 13px;
		font-weight: 700;
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
		font-family: 'Segoe UI Semibold', 'Segoe UI', system-ui, sans-serif;
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
