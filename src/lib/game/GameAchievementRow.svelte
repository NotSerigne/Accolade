<script lang="ts">
	// src/lib/game/GameAchievementRow.svelte
	import { untrack } from 'svelte';
	import {
		ACHIEVEMENT_TAG_KEYS,
		getAchievementPoints,
		getRarityCssVar,
		getRarityTier7,
		getRarityTierIndex,
		normalizeAchievementTags,
		type Achievement,
		type AchievementTagKey
	} from '$lib/stores/Games.js';
	import { i18n, rarityLabelByIndex } from '$lib/stores/i18n.js';

	interface Props {
		ach: Achievement;
		isArchived: boolean;
		revealed: boolean;
		pinned: boolean;
		copied: boolean;
		menuOpen: boolean;
		editingAch: boolean;
		editingMeta: boolean;
		savingAchEdit: boolean;
		savingMetaNote: boolean;
		globalHighlight: string;
		isFlashing: boolean;
		tagLabels: Record<AchievementTagKey, string>;
		trackRow: (node: HTMLDivElement, key: string) => { destroy: () => void };
		onTogglePin: (key: string) => void;
		onToggleMenu: (key: string) => void;
		onCloseMenu: () => void;
		onOpenAchEditor: (ach: Achievement) => void;
		onCloseAchEditor: () => void;
		onSaveAchEdit: (key: string, unlocked: boolean, dateTime: string) => void;
		onOpenMetaEditor: (ach: Achievement) => void;
		onCloseMetaEditor: () => void;
		onSaveNote: (key: string, note: string) => void;
		onClearNote: (key: string) => void;
		onToggleTag: (ach: Achievement, tag: AchievementTagKey) => void;
		onCopyName: (name: string, key: string) => void;
		onFindGuide: (name: string) => void;
	}

	let {
		ach,
		isArchived,
		revealed,
		pinned,
		copied,
		menuOpen,
		editingAch,
		editingMeta,
		savingAchEdit,
		savingMetaNote,
		globalHighlight,
		isFlashing,
		tagLabels,
		trackRow,
		onTogglePin,
		onToggleMenu,
		onCloseMenu,
		onOpenAchEditor,
		onCloseAchEditor,
		onSaveAchEdit,
		onOpenMetaEditor,
		onCloseMetaEditor,
		onSaveNote,
		onClearNote,
		onToggleTag,
		onCopyName,
		onFindGuide
	}: Props = $props();

	let iconFailed = $state(false);
	let editAchUnlocked = $state(true);
	let editAchDateTime = $state('');
	let editNoteText = $state('');

	let isUnlocked = $derived(ach.unlocked || Boolean(ach.unlocked_time));
	let isLocked = $derived(!isUnlocked);
	let isSecret = $derived(Boolean(ach.hidden));
	let isMasked = $derived(isLocked && isSecret && !revealed);

	let descText = $derived((ach.desc || '').trim());
	let noteText = $derived((ach.note ?? '').trim());
	let achTags = $derived(normalizeAchievementTags(ach.tags));
	let displayIcon = $derived(isLocked && ach.icon_gray ? ach.icon_gray : ach.icon);

	function unixToLocalDatetimeInput(unixSecs: number | null | undefined): string {
		const d = unixSecs && unixSecs > 0 ? new Date(unixSecs * 1000) : new Date();
		const pad = (n: number) => String(n).padStart(2, '0');
		return `${d.getFullYear()}-${pad(d.getMonth() + 1)}-${pad(d.getDate())}T${pad(d.getHours())}:${pad(d.getMinutes())}`;
	}

	$effect(() => {
		void displayIcon;
		iconFailed = false;
	});

	$effect(() => {
		if (editingAch) {
			untrack(() => {
				editAchUnlocked = ach.unlocked || Boolean(ach.unlocked_time);
				editAchDateTime = unixToLocalDatetimeInput(ach.unlocked_time);
			});
		}
	});

	$effect(() => {
		if (editingMeta) {
			untrack(() => {
				editNoteText = ach.note ?? '';
			});
		}
	});

	let inlineMeta = $derived.by(() => {
		if (isMasked) return '';
		const parts: string[] = [];
		if (isArchived) parts.push($i18n.t('game.archived'));
		for (const tag of achTags) parts.push(tagLabels[tag]);
		return parts.join(' · ');
	});

	let pctNum = $derived(parseFloat(ach.completionpercentage || ''));
	let hasRarityPct = $derived(Boolean(ach.completionpercentage) && Number.isFinite(pctNum));
	let rarityTier = $derived(getRarityTier7(ach.completionpercentage));
	let rarityColor = $derived(getRarityCssVar(rarityTier));
	let rarityTooltip = $derived(
		hasRarityPct
			? `${rarityLabelByIndex($i18n.language, getRarityTierIndex(rarityTier))} · +${getAchievementPoints(ach)} G`
			: ''
	);

	let isTopbarMatch = $derived(
		!isMasked &&
			Boolean(globalHighlight) &&
			(ach.name.toLowerCase().includes(globalHighlight) ||
				(ach.desc ?? '').toLowerCase().includes(globalHighlight) ||
				noteText.toLowerCase().includes(globalHighlight))
	);

	let hasProgress = $derived(
		!isMasked && isLocked && typeof ach.max_progress === 'number' && ach.max_progress > 0
	);
	let progressCur = $derived(ach.progress ?? 0);
	let progressPct = $derived(
		hasProgress && ach.max_progress
			? Math.min(Math.max((progressCur / ach.max_progress) * 100, 0), 100)
			: 0
	);

	function formatProgressVal(n: number): string {
		return Number.isInteger(n) ? String(n) : n.toFixed(1);
	}

	function formatDate(ts: number | null): string {
		if (!ts) return '';
		return new Date(ts * 1000).toLocaleDateString($i18n.locale, {
			day: '2-digit',
			month: '2-digit',
			year: 'numeric',
			hour: '2-digit',
			minute: '2-digit'
		});
	}
</script>

<div
	class="ach-row"
	class:is-unlocked={isUnlocked}
	class:is-archived={isArchived}
	class:is-masked={isMasked}
	class:highlighted={isTopbarMatch}
	class:jump-flash={isFlashing}
	class:menu-open={menuOpen}
	class:actions-active={menuOpen || editingAch || editingMeta}
	use:trackRow={ach.key}
>
	<div class="ach-row-main">
		<div class="ach-icon">
			{#if isMasked}
				<span class="ach-icon-placeholder" aria-hidden="true">
					<svg
						width="16"
						height="16"
						viewBox="0 0 24 24"
						fill="none"
						stroke="currentColor"
						stroke-width="2"
						stroke-linecap="round"
						stroke-linejoin="round"
					>
						<rect x="3" y="11" width="18" height="11" rx="2" ry="2" />
						<path d="M7 11V7a5 5 0 0 1 10 0v4" />
					</svg>
				</span>
			{:else if displayIcon && !iconFailed}
				<img
					src={displayIcon}
					alt={ach.name || ach.key}
					class:muted={isLocked}
					loading="lazy"
					decoding="async"
					onerror={() => (iconFailed = true)}
				/>
			{:else}
				<span class="ach-icon-placeholder" aria-hidden="true">
					<svg
						width="17"
						height="17"
						viewBox="0 0 24 24"
						fill="none"
						stroke="currentColor"
						stroke-width="1.8"
						stroke-linecap="round"
						stroke-linejoin="round"
					>
						<path d="M6 9H4.5a2.5 2.5 0 0 1 0-5H6" />
						<path d="M18 9h1.5a2.5 2.5 0 0 0 0-5H18" />
						<path d="M4 22h16" />
						<path d="M10 14.66V17c0 .55-.47.98-.97 1.21C7.85 18.75 7 20.24 7 22" />
						<path d="M14 14.66V17c0 .55.47.98.97 1.21C16.15 18.75 17 20.24 17 22" />
						<path d="M18 2H6v7a6 6 0 0 0 12 0V2Z" />
					</svg>
				</span>
			{/if}
		</div>

		<div class="ach-body">
			<div class="ach-title-line">
				<span class="ach-name" class:is-locked={isLocked} class:is-masked={isMasked}>
					{isMasked ? $i18n.t('game.secretAchievement') : ach.name || ach.key}
				</span>

				{#if pinned}
					<svg
						class="ach-pin-indicator"
						width="12"
						height="12"
						viewBox="0 0 24 24"
						fill="currentColor"
						aria-hidden="true"
					>
						<line x1="12" y1="17" x2="12" y2="22" stroke="currentColor" stroke-width="2.2" />
						<path
							d="M5 17h14v-1.76a2 2 0 0 0-1.11-1.79l-1.78-.9A2 2 0 0 1 15 10.76V6h1a2 2 0 0 0 0-4H8a2 2 0 0 0 0 4h1v4.76a2 2 0 0 1-1.11 1.79l-1.78.9A2 2 0 0 0 5 15.24Z"
						/>
					</svg>
				{/if}

				{#if inlineMeta}
					<span class="ach-inline-meta">{inlineMeta}</span>
				{/if}
			</div>

			{#if isMasked}
				<p class="ach-desc is-masked">{$i18n.t('game.descHidden')}</p>
			{:else if descText}
				<p class="ach-desc">{descText}</p>
			{/if}

			{#if hasProgress && ach.max_progress}
				<div class="ach-progress-row">
					<div class="ach-progress-track">
						<div class="ach-progress-fill" style:width="{progressPct}%"></div>
					</div>
					<span class="ach-progress-text tabular-nums">
						{formatProgressVal(progressCur)} / {formatProgressVal(ach.max_progress)}
					</span>
				</div>
			{/if}

			{#if !isMasked && noteText}
				<button
					type="button"
					class="ach-note-btn"
					title={$i18n.t('game.note.edit')}
					onclick={() => onOpenMetaEditor(ach)}
				>
					{noteText}
				</button>
			{/if}
		</div>

		<div class="ach-telemetry">
			{#if hasRarityPct}
				<span class="ach-rarity tabular-nums" title={rarityTooltip}>
					<span class="rarity-dot" style:background={rarityColor} aria-hidden="true"></span>
					<span>{pctNum.toFixed(1)}%</span>
				</span>
			{/if}
			{#if isUnlocked && ach.unlocked_time}
				<span class="ach-date tabular-nums">{formatDate(ach.unlocked_time)}</span>
			{/if}
		</div>

		<div class="ach-actions-slot" data-row-menu-container>
			<button
				type="button"
				class="row-icon-btn"
				class:is-pinned={pinned}
				aria-pressed={pinned}
				aria-label={pinned ? $i18n.t('hud.unpin') : $i18n.t('hud.pin')}
				title={pinned ? $i18n.t('hud.unpin') : $i18n.t('hud.pin')}
				onclick={() => onTogglePin(ach.key)}
			>
				<svg
					width="13"
					height="13"
					viewBox="0 0 24 24"
					fill={pinned ? 'currentColor' : 'none'}
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
			</button>

			{#if !isMasked}
				<button
					type="button"
					class="row-icon-btn"
					class:is-active={editingMeta || Boolean(noteText) || achTags.length > 0}
					aria-pressed={editingMeta}
					aria-label={$i18n.t('game.note.edit')}
					title={$i18n.t('game.note.edit')}
					onclick={() => onOpenMetaEditor(ach)}
				>
					<svg
						width="13"
						height="13"
						viewBox="0 0 16 16"
						fill="none"
						stroke="currentColor"
						stroke-width="1.5"
						stroke-linecap="round"
						stroke-linejoin="round"
						aria-hidden="true"
					>
						<path
							d="M10.5 2.5H4a1.5 1.5 0 0 0-1.5 1.5v8A1.5 1.5 0 0 0 4 13.5h8a1.5 1.5 0 0 0 1.5-1.5V5.5Z"
						/>
						<path d="M10 2.5V6h3.5" />
						<path d="M5.5 8.5h5M5.5 11h3.5" />
					</svg>
				</button>
			{/if}

			<button
				type="button"
				class="row-icon-btn"
				class:is-active={menuOpen}
				aria-haspopup="menu"
				aria-expanded={menuOpen}
				aria-label={$i18n.t('game.editAchievementDate')}
				onclick={(e) => {
					e.stopPropagation();
					onToggleMenu(ach.key);
				}}
			>
				<svg width="14" height="14" viewBox="0 0 24 24" fill="currentColor" aria-hidden="true">
					<circle cx="5" cy="12" r="1.75" />
					<circle cx="12" cy="12" r="1.75" />
					<circle cx="19" cy="12" r="1.75" />
				</svg>
			</button>

			{#if menuOpen}
				<div class="row-menu" role="menu">
					<button
						type="button"
						role="menuitem"
						class="row-menu-item"
						onclick={() => {
							onOpenAchEditor(ach);
							onCloseMenu();
						}}
					>
						<span>{$i18n.t('game.edit')}</span>
					</button>

					{#if !isMasked}
						<button
							type="button"
							role="menuitem"
							class="row-menu-item"
							onclick={() => onCopyName(ach.name || ach.key, ach.key)}
						>
							<span>{copied ? $i18n.t('game.copied') : $i18n.t('game.copyName')}</span>
						</button>
						<button
							type="button"
							role="menuitem"
							class="row-menu-item"
							onclick={() => {
								onFindGuide(ach.name || ach.key);
								onCloseMenu();
							}}
						>
							<span>{$i18n.t('game.findGuide')}</span>
						</button>
						<div class="row-menu-sep" role="separator"></div>
						{#each ACHIEVEMENT_TAG_KEYS as tagKey (tagKey)}
							{@const active = achTags.includes(tagKey)}
							<button
								type="button"
								role="menuitemcheckbox"
								aria-checked={active}
								class="row-menu-item"
								class:is-checked={active}
								onclick={() => onToggleTag(ach, tagKey)}
							>
								<span>{tagLabels[tagKey]}</span>
								{#if active}
									<svg
										width="12"
										height="12"
										viewBox="0 0 24 24"
										fill="none"
										stroke="currentColor"
										stroke-width="2.5"
										stroke-linecap="round"
										stroke-linejoin="round"
										aria-hidden="true"
									>
										<polyline points="20 6 9 17 4 12" />
									</svg>
								{/if}
							</button>
						{/each}

						{#if noteText}
							<div class="row-menu-sep" role="separator"></div>
							<button
								type="button"
								role="menuitem"
								class="row-menu-item is-danger"
								onclick={() => {
									onClearNote(ach.key);
									onCloseMenu();
								}}
							>
								<span>{$i18n.t('game.note.clear')}</span>
							</button>
						{/if}
					{/if}
				</div>
			{/if}
		</div>
	</div>

	{#if editingAch}
		<div class="inline-editor">
			<div class="editor-left">
				<button
					type="button"
					class="state-toggle"
					class:unlocked={editAchUnlocked}
					aria-pressed={editAchUnlocked}
					onclick={() => (editAchUnlocked = !editAchUnlocked)}
				>
					{#if editAchUnlocked}
						<svg
							width="12"
							height="12"
							viewBox="0 0 24 24"
							fill="none"
							stroke="currentColor"
							stroke-width="2.6"
							stroke-linecap="round"
							stroke-linejoin="round"
							aria-hidden="true"
						>
							<polyline points="20 6 9 17 4 12" />
						</svg>
						<span>{$i18n.t('game.stateUnlocked')}</span>
					{:else}
						<svg
							width="12"
							height="12"
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
						<span>{$i18n.t('game.stateLocked')}</span>
					{/if}
				</button>

				{#if editAchUnlocked}
					<div class="dt-group">
						<span class="dt-label">{$i18n.t('game.dateTimeLabel')}</span>
						<input
							type="datetime-local"
							class="dt-input tabular-nums"
							aria-label={$i18n.t('game.dateTimeLabel')}
							bind:value={editAchDateTime}
						/>
						<button
							type="button"
							class="editor-btn"
							onclick={() =>
								(editAchDateTime = unixToLocalDatetimeInput(Math.floor(Date.now() / 1000)))}
						>
							{$i18n.t('game.now')}
						</button>
					</div>
				{/if}
			</div>

			<div class="editor-right">
				<button
					type="button"
					class="editor-btn is-primary"
					disabled={savingAchEdit}
					onclick={() => onSaveAchEdit(ach.key, editAchUnlocked, editAchDateTime)}
				>
					{savingAchEdit ? '...' : $i18n.t('game.save')}
				</button>
				<button type="button" class="editor-btn" onclick={onCloseAchEditor}>
					{$i18n.t('game.cancel')}
				</button>
			</div>
		</div>
	{/if}

	{#if !isMasked && editingMeta}
		<div class="inline-editor meta-editor">
			<div class="tag-toggles">
				<span class="meta-label">{$i18n.t('game.tag.markers')}</span>
				{#each ACHIEVEMENT_TAG_KEYS as tagKey (tagKey)}
					{@const active = achTags.includes(tagKey)}
					<button
						type="button"
						class="tag-toggle"
						data-tag={tagKey}
						class:active
						aria-pressed={active}
						onclick={() => onToggleTag(ach, tagKey)}
					>
						{tagLabels[tagKey]}
					</button>
				{/each}
			</div>

			<div class="note-row">
				<input
					type="text"
					class="note-input"
					placeholder={$i18n.t('game.note.placeholder')}
					aria-label={$i18n.t('game.note.badgeTitle')}
					bind:value={editNoteText}
					onkeydown={(event) => {
						if (event.key === 'Enter') {
							event.preventDefault();
							onSaveNote(ach.key, editNoteText);
						} else if (event.key === 'Escape') {
							onCloseMetaEditor();
						}
					}}
				/>
				<button
					type="button"
					class="editor-btn is-primary"
					disabled={savingMetaNote}
					onclick={() => onSaveNote(ach.key, editNoteText)}
				>
					{savingMetaNote ? '...' : $i18n.t('game.note.save')}
				</button>
				{#if noteText || editNoteText.trim()}
					<button
						type="button"
						class="editor-btn is-danger"
						disabled={savingMetaNote}
						onclick={() => {
							editNoteText = '';
							onClearNote(ach.key);
						}}
					>
						{$i18n.t('game.note.clear')}
					</button>
				{/if}
				<button type="button" class="editor-btn" onclick={onCloseMetaEditor}>
					{$i18n.t('game.note.cancel')}
				</button>
			</div>

			<span class="hud-hint">{$i18n.t('game.note.hudHint')}</span>
		</div>
	{/if}
</div>

<style>
	.ach-row {
		position: relative;
		border-bottom: 1px solid var(--border-soft);
		transition: background 0.14s ease;
	}
	.ach-row:not(.menu-open):not(.actions-active) {
		content-visibility: auto;
		contain-intrinsic-size: 0 64px;
	}
	.ach-row.menu-open {
		z-index: 30;
	}
	.ach-row:hover {
		background: var(--surface-2);
	}
	.ach-row.is-archived:not(.is-unlocked) .ach-icon {
		opacity: 0.55;
	}
	.ach-row.highlighted {
		background: color-mix(in srgb, var(--accent) 9%, transparent);
		box-shadow: inset 2px 0 0 var(--accent);
	}
	.ach-row.jump-flash {
		animation: achJumpFlash 1.8s ease;
	}
	@keyframes achJumpFlash {
		0% {
			background: color-mix(in srgb, var(--accent) 22%, transparent);
			box-shadow: inset 3px 0 0 var(--accent);
		}
		100% {
			background: transparent;
			box-shadow: none;
		}
	}
	.ach-row-main {
		display: flex;
		align-items: center;
		gap: 12px;
		padding: 11px 4px;
		min-height: 62px;
	}
	.ach-icon {
		width: 40px;
		height: 40px;
		border-radius: 8px;
		overflow: hidden;
		flex-shrink: 0;
		background: var(--surface-2);
		border: 1px solid var(--border-soft);
		display: flex;
		align-items: center;
		justify-content: center;
	}
	.ach-icon img {
		width: 100%;
		height: 100%;
		object-fit: cover;
		display: block;
	}
	.ach-icon img.muted {
		opacity: 0.42;
		filter: grayscale(0.45);
	}
	.ach-icon-placeholder {
		color: var(--text-muted);
		display: flex;
		align-items: center;
		justify-content: center;
	}
	.ach-body {
		flex: 1;
		min-width: 0;
		display: flex;
		flex-direction: column;
		gap: 3px;
	}
	.ach-title-line {
		display: flex;
		align-items: baseline;
		flex-wrap: wrap;
		gap: 6px;
		min-width: 0;
	}
	.ach-name {
		font-size: 13.5px;
		font-weight: 600;
		color: var(--text-primary);
		line-height: 1.3;
	}
	.ach-name.is-locked {
		color: var(--text-secondary);
	}
	.ach-name.is-masked {
		color: var(--text-muted);
	}
	.ach-pin-indicator {
		color: var(--accent);
		flex-shrink: 0;
		align-self: center;
	}
	.ach-inline-meta {
		font-size: 11px;
		font-weight: 500;
		color: var(--text-muted);
	}
	.ach-desc {
		margin: 0;
		font-size: 12.5px;
		line-height: 1.38;
		color: var(--text-muted);
		display: -webkit-box;
		-webkit-line-clamp: 2;
		line-clamp: 2;
		-webkit-box-orient: vertical;
		overflow: hidden;
	}
	.ach-desc.is-masked {
		font-style: italic;
	}
	.ach-progress-row {
		display: flex;
		align-items: center;
		gap: 8px;
		margin-top: 3px;
	}
	.ach-progress-track {
		width: 120px;
		height: 3px;
		border-radius: 999px;
		background: var(--surface-2);
		overflow: hidden;
	}
	.ach-progress-fill {
		height: 100%;
		border-radius: inherit;
		background: var(--accent);
	}
	.ach-progress-text {
		font-size: 11px;
		font-weight: 600;
		color: var(--text-muted);
	}
	.ach-note-btn {
		align-self: flex-start;
		margin-top: 3px;
		padding: 1px 0 1px 8px;
		border: none;
		border-left: 2px solid var(--border-specular);
		background: transparent;
		color: var(--text-secondary);
		font-family: inherit;
		font-size: 12px;
		line-height: 1.35;
		text-align: left;
		cursor: pointer;
		max-width: 100%;
		word-break: break-word;
	}
	.ach-note-btn:hover {
		color: var(--text-primary);
	}
	.ach-telemetry {
		flex-shrink: 0;
		display: flex;
		flex-direction: column;
		align-items: flex-end;
		gap: 2px;
		min-width: 68px;
	}
	.ach-rarity {
		display: inline-flex;
		align-items: center;
		gap: 6px;
		font-size: 11.5px;
		font-weight: 600;
		color: var(--text-secondary);
	}
	.rarity-dot {
		width: 6px;
		height: 6px;
		border-radius: 50%;
		flex-shrink: 0;
	}
	.ach-date {
		font-size: 10.5px;
		color: var(--text-muted);
		white-space: nowrap;
	}
	.ach-actions-slot {
		width: 88px;
		flex-shrink: 0;
		display: flex;
		align-items: center;
		justify-content: flex-end;
		gap: 2px;
		position: relative;
	}
	.row-icon-btn {
		width: 26px;
		height: 26px;
		border-radius: 6px;
		border: none;
		background: transparent;
		color: var(--text-muted);
		display: inline-flex;
		align-items: center;
		justify-content: center;
		cursor: pointer;
		opacity: 0;
		transition:
			opacity 0.14s ease,
			color 0.14s ease,
			background 0.14s ease;
	}
	.ach-row:hover .row-icon-btn,
	.ach-row:focus-within .row-icon-btn,
	.ach-row.actions-active .row-icon-btn {
		opacity: 1;
	}
	.row-icon-btn.is-pinned {
		opacity: 0.72;
		color: var(--accent);
	}
	.ach-row:hover .row-icon-btn.is-pinned,
	.ach-row:focus-within .row-icon-btn.is-pinned {
		opacity: 1;
	}
	.row-icon-btn:hover,
	.row-icon-btn.is-active {
		color: var(--text-primary);
		background: var(--surface-hover);
	}
	.row-menu {
		position: absolute;
		top: calc(100% + 4px);
		right: 0;
		min-width: 176px;
		padding: 4px;
		border-radius: 10px;
		background: var(--surface-1);
		border: 1px solid var(--border-specular);
		box-shadow: var(--shadow-elevated);
		display: flex;
		flex-direction: column;
		z-index: 40;
	}
	.row-menu-item {
		display: flex;
		align-items: center;
		justify-content: space-between;
		gap: 8px;
		width: 100%;
		padding: 6px 10px;
		border: none;
		border-radius: 6px;
		background: transparent;
		color: var(--text-secondary);
		font-family: inherit;
		font-size: 12px;
		font-weight: 500;
		text-align: left;
		cursor: pointer;
	}
	.row-menu-item:hover {
		background: var(--surface-hover);
		color: var(--text-primary);
	}
	.row-menu-item.is-checked {
		color: var(--accent);
	}
	.row-menu-item.is-danger {
		color: var(--err);
	}
	.row-menu-sep {
		height: 1px;
		background: var(--border-soft);
		margin: 4px 6px;
	}
	.inline-editor {
		margin: 0 0 12px 52px;
		padding: 10px 12px;
		border: 1px solid var(--border-soft);
		border-radius: 8px;
		background: var(--surface-1);
		display: flex;
		align-items: center;
		justify-content: space-between;
		gap: 10px;
		flex-wrap: wrap;
	}
	.inline-editor.meta-editor {
		flex-direction: column;
		align-items: stretch;
		gap: 8px;
	}
	.editor-left,
	.editor-right {
		display: flex;
		align-items: center;
		gap: 8px;
		flex-wrap: wrap;
	}
	.state-toggle {
		display: inline-flex;
		align-items: center;
		gap: 6px;
		padding: 4px 10px;
		border-radius: 6px;
		border: 1px solid var(--border-soft);
		background: var(--surface-1);
		color: var(--text-secondary);
		font-family: inherit;
		font-size: 11.5px;
		font-weight: 600;
		cursor: pointer;
	}
	.state-toggle.unlocked {
		border-color: color-mix(in srgb, var(--ok) 45%, transparent);
		background: color-mix(in srgb, var(--ok) 8%, transparent);
		color: var(--ok);
	}
	.dt-group,
	.tag-toggles,
	.note-row {
		display: flex;
		align-items: center;
		gap: 6px;
		flex-wrap: wrap;
	}
	.dt-label,
	.meta-label {
		font-size: 11.5px;
		font-weight: 600;
		color: var(--text-secondary);
	}
	.dt-input,
	.note-input {
		background: var(--surface-1);
		border: 1px solid var(--border-soft);
		border-radius: 6px;
		padding: 4px 8px;
		color: var(--text-primary);
		font-family: inherit;
		font-size: 12px;
	}
	.dt-input:focus,
	.note-input:focus {
		border-color: var(--accent);
	}
	.tag-toggle {
		padding: 3px 9px;
		border-radius: 999px;
		border: 1px solid var(--border-soft);
		background: var(--surface-1);
		color: var(--text-secondary);
		font-family: inherit;
		font-size: 11px;
		font-weight: 600;
		cursor: pointer;
	}
	.tag-toggle:hover {
		background: var(--surface-hover);
		color: var(--text-primary);
	}
	.tag-toggle[data-tag='missable'].active {
		border-color: color-mix(in srgb, var(--warn) 55%, transparent);
		background: color-mix(in srgb, var(--warn) 8%, transparent);
		color: var(--warn);
	}
	.tag-toggle[data-tag='multi'].active {
		border-color: color-mix(in srgb, var(--rarity-rare) 55%, transparent);
		background: color-mix(in srgb, var(--rarity-rare) 8%, transparent);
		color: var(--rarity-rare);
	}
	.tag-toggle[data-tag='ngplus'].active {
		border-color: color-mix(in srgb, var(--rarity-epic) 55%, transparent);
		background: color-mix(in srgb, var(--rarity-epic) 8%, transparent);
		color: var(--rarity-epic);
	}
	.tag-toggle[data-tag='buggy'].active {
		border-color: color-mix(in srgb, var(--err) 55%, transparent);
		background: color-mix(in srgb, var(--err) 8%, transparent);
		color: var(--err);
	}
	.note-input {
		flex: 1 1 220px;
		min-width: 0;
		padding: 5px 9px;
	}
	.editor-btn {
		padding: 4px 10px;
		border-radius: 6px;
		border: 1px solid var(--border-soft);
		background: var(--surface-1);
		color: var(--text-secondary);
		font-family: inherit;
		font-size: 11.5px;
		font-weight: 600;
		cursor: pointer;
	}
	.editor-btn:hover:not(:disabled) {
		background: var(--surface-hover);
		color: var(--text-primary);
	}
	.editor-btn:disabled {
		opacity: 0.5;
		cursor: default;
	}
	.editor-btn.is-primary {
		background: var(--accent);
		border-color: var(--accent);
		color: var(--accent-text);
	}
	.editor-btn.is-danger {
		color: var(--err);
		border-color: color-mix(in srgb, var(--err) 40%, transparent);
		background: color-mix(in srgb, var(--err) 10%, transparent);
	}
	.hud-hint {
		font-size: 10.5px;
		color: var(--text-muted);
	}
	@media (max-width: 640px) {
		.ach-row-main {
			flex-wrap: wrap;
		}
		.ach-telemetry {
			margin-left: 52px;
			flex-direction: row;
			align-items: center;
			gap: 8px;
		}
		.ach-actions-slot {
			margin-left: auto;
		}
		.inline-editor {
			margin-left: 0;
		}
	}
</style>
