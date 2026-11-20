<script lang="ts">
	// src/lib/game/GameProgressMap.svelte
	import { SvelteSet } from 'svelte/reactivity';
	import {
		getRarityCssVar,
		getRarityTier7,
		getRarityTierIndex,
		type Achievement
	} from '$lib/stores/Games.js';
	import { i18n, rarityLabelByIndex } from '$lib/stores/i18n.js';

	interface Props {
		mapAchievements: Achievement[];
		unlockedCount: number;
		totalCount: number;
		revealed: boolean;
		onSelectAchievement: (key: string) => void;
	}

	let { mapAchievements, unlockedCount, totalCount, revealed, onSelectAchievement }: Props =
		$props();

	let failedIcons = new SvelteSet<string>();

	function handleIconError(key: string) {
		failedIcons.add(key);
	}
</script>

<div class="progress-map">
	<div class="map-intro">
		<div class="map-intro-copy">
			<h2>{$i18n.t('game.shareCard')}</h2>
			<p>{$i18n.t('game.timelineStepHint')}</p>
		</div>
		<strong class="map-overall-progress tabular-nums">{unlockedCount}/{totalCount}</strong>
	</div>

	<div class="map-track">
		{#each mapAchievements as ach, index (ach.key)}
			{@const isUnlocked = ach.unlocked || Boolean(ach.unlocked_time)}
			{@const isMasked = !revealed && Boolean(ach.hidden) && !isUnlocked}
			{@const displayName = isMasked ? $i18n.t('game.secretAchievement') : ach.name || ach.key}
			{@const displayDesc = isMasked
				? $i18n.t('game.descHidden')
				: ach.desc || $i18n.t('game.achievementToUnlock')}
			{@const tier = getRarityTier7(ach.completionpercentage)}
			{@const rarityColor = getRarityCssVar(tier)}
			{@const pctNum = parseFloat(ach.completionpercentage || '')}
			{@const hasPct = Number.isFinite(pctNum)}
			{@const tierLabel = hasPct
				? rarityLabelByIndex($i18n.language, getRarityTierIndex(tier))
				: $i18n.t('game.unknown')}
			{@const pctBar = hasPct ? Math.min(Math.max(pctNum, 0), 100) : 0}
			{@const iconSrc = isUnlocked ? ach.icon || ach.icon_gray : ach.icon_gray || ach.icon}

			<button
				type="button"
				class="map-card"
				class:unlocked={isUnlocked}
				onclick={() => onSelectAchievement(ach.key)}
				aria-label={$i18n.t('game.viewDescriptionAria', { name: displayName })}
			>
				<div
					class="map-index tabular-nums"
					class:complete={isUnlocked}
					aria-label={$i18n.t('game.stepIndexAria', {
						index: index + 1,
						total: mapAchievements.length
					})}
				>
					<span>{index + 1}</span>
					<small>/{mapAchievements.length}</small>
				</div>

				<div class="map-icon">
					{#if isMasked}
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
							<rect x="3" y="11" width="18" height="11" rx="2" ry="2" />
							<path d="M7 11V7a5 5 0 0 1 10 0v4" />
						</svg>
					{:else if iconSrc && !failedIcons.has(ach.key)}
						<img
							src={iconSrc}
							alt=""
							class:muted={!isUnlocked}
							loading="lazy"
							decoding="async"
							onerror={() => handleIconError(ach.key)}
						/>
					{:else}
						<svg
							width="18"
							height="18"
							viewBox="0 0 24 24"
							fill="none"
							stroke="currentColor"
							stroke-width="1.8"
							stroke-linecap="round"
							stroke-linejoin="round"
							aria-hidden="true"
						>
							<path d="M6 9H4.5a2.5 2.5 0 0 1 0-5H6" />
							<path d="M18 9h1.5a2.5 2.5 0 0 0 0-5H18" />
							<path d="M4 22h16" />
							<path d="M10 14.66V17c0 .55-.47.98-.97 1.21C7.85 18.75 7 20.24 7 22" />
							<path d="M14 14.66V17c0 .55.47.98.97 1.21C16.15 18.75 17 20.24 17 22" />
							<path d="M18 2H6v7a6 6 0 0 0 12 0V2Z" />
						</svg>
					{/if}
				</div>

				<div class="map-content">
					<div class="map-title" class:is-masked={isMasked}>{displayName}</div>
					<div class="map-desc" class:is-masked={isMasked}>{displayDesc}</div>
					<div class="map-meta">
						<span class="map-rarity tabular-nums" style:color={rarityColor}>
							{tierLabel} · {hasPct ? pctNum.toFixed(1) : '0.0'}%
						</span>
						<div class="map-rarity-track" aria-hidden="true">
							<div
								class="map-rarity-fill"
								style:width="{pctBar}%"
								style:background={rarityColor}
							></div>
						</div>
					</div>
				</div>

				<div class="map-status" class:complete={isUnlocked} aria-hidden="true">
					{#if isUnlocked}
						<svg
							width="12"
							height="12"
							viewBox="0 0 24 24"
							fill="none"
							stroke="currentColor"
							stroke-width="2.8"
							stroke-linecap="round"
							stroke-linejoin="round"
						>
							<polyline points="20 6 9 17 4 12" />
						</svg>
					{:else}
						<svg width="6" height="6" viewBox="0 0 8 8" fill="currentColor">
							<circle cx="4" cy="4" r="3" />
						</svg>
					{/if}
				</div>
			</button>
		{/each}
	</div>
</div>

<style>
	.progress-map {
		padding: 4px 0 24px;
	}

	.map-intro {
		display: flex;
		align-items: center;
		justify-content: space-between;
		gap: 16px;
		margin-bottom: 18px;
		padding-bottom: 14px;
		border-bottom: 1px solid var(--border-soft);
	}

	.map-intro-copy {
		min-width: 0;
	}

	.map-intro h2 {
		margin: 0 0 4px;
		font-size: 16px;
		font-weight: 700;
		color: var(--text-primary);
	}

	.map-intro p {
		margin: 0;
		font-size: 12.5px;
		color: var(--text-muted);
	}

	.map-overall-progress {
		color: var(--accent);
		font-size: 15px;
		font-weight: 700;
		white-space: nowrap;
	}

	.map-track {
		display: grid;
		grid-template-columns: repeat(auto-fill, minmax(260px, 1fr));
		gap: 10px;
	}

	.map-card {
		width: 100%;
		min-width: 0;
		min-height: 76px;
		display: flex;
		align-items: center;
		gap: 11px;
		padding: 10px 12px;
		background: var(--surface-1);
		border: 1px solid var(--border-soft);
		border-radius: 10px;
		color: inherit;
		font-family: inherit;
		text-align: left;
		cursor: pointer;
		transition:
			border-color 0.16s ease,
			background 0.16s ease,
			transform 0.18s var(--spring-out);
	}

	.map-card:hover {
		transform: translateY(-1px);
		background: var(--surface-hover);
		border-color: var(--border-specular);
	}

	.map-card.unlocked {
		border-color: color-mix(in srgb, var(--accent) 32%, var(--border-soft));
	}

	.map-index {
		width: 38px;
		height: 38px;
		border: 1.5px solid var(--border-soft);
		border-radius: 50%;
		flex-shrink: 0;
		display: flex;
		align-items: center;
		justify-content: center;
		gap: 1px;
		color: var(--text-secondary);
		background: var(--surface-2);
		font-size: 12px;
		font-weight: 700;
	}

	.map-index small {
		font-size: 9px;
		font-weight: 600;
		color: var(--text-muted);
	}

	.map-index.complete {
		border-color: var(--completed);
		color: var(--completed);
	}

	.map-icon {
		width: 40px;
		height: 40px;
		border-radius: 8px;
		overflow: hidden;
		flex-shrink: 0;
		background: var(--surface-2);
		border: 1px solid var(--border-soft);
		color: var(--text-muted);
		display: flex;
		align-items: center;
		justify-content: center;
	}

	.map-icon img {
		width: 100%;
		height: 100%;
		object-fit: cover;
		display: block;
	}

	.map-icon img.muted {
		opacity: 0.42;
		filter: grayscale(0.45);
	}

	.map-content {
		flex: 1;
		min-width: 0;
	}

	.map-title {
		color: var(--text-primary);
		font-size: 13px;
		font-weight: 600;
		white-space: nowrap;
		overflow: hidden;
		text-overflow: ellipsis;
	}

	.map-title.is-masked {
		color: var(--text-muted);
	}

	.map-desc {
		color: var(--text-muted);
		font-size: 11.5px;
		white-space: nowrap;
		overflow: hidden;
		text-overflow: ellipsis;
		margin-top: 2px;
	}

	.map-desc.is-masked {
		font-style: italic;
	}

	.map-meta {
		display: flex;
		align-items: center;
		gap: 8px;
		margin-top: 6px;
	}

	.map-rarity {
		font-size: 10px;
		font-weight: 700;
		white-space: nowrap;
		text-transform: uppercase;
		letter-spacing: 0.03em;
	}

	.map-rarity-track {
		height: 3px;
		flex: 1;
		min-width: 24px;
		border-radius: 999px;
		background: var(--surface-2);
		overflow: hidden;
	}

	.map-rarity-fill {
		height: 100%;
		border-radius: inherit;
	}

	.map-status {
		width: 22px;
		height: 22px;
		border-radius: 50%;
		flex-shrink: 0;
		display: flex;
		align-items: center;
		justify-content: center;
		color: var(--text-muted);
		border: 1px solid var(--border-soft);
	}

	.map-status.complete {
		background: var(--completed);
		color: var(--completed-text);
		border-color: var(--completed);
	}
</style>
