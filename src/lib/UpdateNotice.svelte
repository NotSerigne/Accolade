<script lang="ts">
	// src/lib/UpdateNotice.svelte
	import { updateState, installUpdate, dismissUpdate } from '$lib/stores/update.js';
	import { i18n } from '$lib/stores/i18n.js';

	let visible = $derived(
		!$updateState.dismissed &&
			($updateState.status === 'available' ||
				$updateState.status === 'downloading' ||
				$updateState.status === 'ready' ||
				$updateState.status === 'error')
	);
</script>

{#if visible}
	<div class="update-toast" role="status">
		<div class="update-icon">
			<svg width="18" height="18" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
				<path d="M21 12a9 9 0 1 1-3-6.7" />
				<polyline points="21 3 21 9 15 9" />
			</svg>
		</div>
		<div class="update-body">
			{#if $updateState.status === 'available'}
				<p class="update-title">
					{$i18n.t('update.available', { version: $updateState.version ?? '' })}
				</p>
				{#if $updateState.notes}
					<p class="update-notes">{$updateState.notes}</p>
				{/if}
				<div class="update-actions">
					<button class="update-btn primary" onclick={installUpdate}>
						{$i18n.t('update.updateNow')}
					</button>
					<button class="update-btn ghost" onclick={dismissUpdate}>
						{$i18n.t('update.later')}
					</button>
				</div>
			{:else if $updateState.status === 'downloading'}
				<p class="update-title">{$i18n.t('update.downloading')}</p>
				<div class="update-progress-track">
					<div
						class="update-progress-fill"
						style:width="{$updateState.progress ?? 0}%"
					></div>
				</div>
			{:else if $updateState.status === 'ready'}
				<p class="update-title">{$i18n.t('update.ready')}</p>
			{:else if $updateState.status === 'error'}
				<p class="update-title error">{$i18n.t('update.error')}</p>
				<div class="update-actions">
					<button class="update-btn ghost" onclick={dismissUpdate}>
						{$i18n.t('button.close')}
					</button>
				</div>
			{/if}
		</div>
	</div>
{/if}

<style>
	.update-toast {
		position: fixed;
		right: 20px;
		bottom: 20px;
		z-index: 2000;
		display: flex;
		gap: 12px;
		align-items: flex-start;
		max-width: 340px;
		padding: 16px;
		background: var(--bg-panel);
		border: 1px solid var(--border-soft);
		border-radius: 14px;
		box-shadow: 0 16px 48px rgba(0, 0, 0, 0.4);
		animation: update-slide-in 0.3s ease;
	}

	@keyframes update-slide-in {
		from {
			opacity: 0;
			transform: translateY(12px);
		}
		to {
			opacity: 1;
			transform: translateY(0);
		}
	}

	.update-icon {
		flex-shrink: 0;
		display: flex;
		align-items: center;
		justify-content: center;
		width: 32px;
		height: 32px;
		border-radius: 50%;
		background: color-mix(in srgb, var(--accent) 18%, transparent);
		color: var(--accent);
	}

	.update-body {
		flex: 1;
		min-width: 0;
	}

	.update-title {
		margin: 0;
		font-size: 13.5px;
		font-weight: 600;
		color: var(--text-primary);
	}

	.update-title.error {
		color: #e5484d;
	}

	.update-notes {
		margin: 6px 0 0;
		font-size: 12px;
		color: var(--text-secondary);
		max-height: 80px;
		overflow-y: auto;
		white-space: pre-line;
	}

	.update-actions {
		display: flex;
		gap: 8px;
		margin-top: 12px;
	}

	.update-btn {
		border: none;
		border-radius: 8px;
		padding: 7px 14px;
		font-size: 12.5px;
		font-weight: 600;
		cursor: pointer;
		transition:
			background 0.15s ease,
			opacity 0.15s ease;
	}

	.update-btn.primary {
		background: var(--accent);
		color: var(--accent-text, #111);
	}

	.update-btn.primary:hover {
		opacity: 0.88;
	}

	.update-btn.ghost {
		background: transparent;
		color: var(--text-secondary);
	}

	.update-btn.ghost:hover {
		color: var(--text-primary);
	}

	.update-progress-track {
		margin-top: 10px;
		height: 6px;
		border-radius: 999px;
		background: var(--border-soft);
		overflow: hidden;
	}

	.update-progress-fill {
		height: 100%;
		background: var(--accent);
		transition: width 0.2s ease;
	}
</style>
